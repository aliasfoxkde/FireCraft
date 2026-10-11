//! Point operations: per-pixel color remaps.
//!
//! All of them funnel through [`Surface::map_straight`] (or its LUT
//! form), so the premultiply order is decided once in `lib.rs`
//! (decision 0004). LUT-able ops build a 256-entry table and apply it
//! per channel — that keeps the integer-exact proofs to one loop.

use crate::{Surface, clamp255, div_half};
use color::Rgba8;

/// Applies a caller-supplied per-channel lookup table (the "curves"
/// surface: a curve editor emits exactly this table).
#[must_use]
pub fn apply_lut(surface: &Surface, lut: &[u8; 256]) -> Surface {
    surface.map_straight(|px| {
        Rgba8::new(
            lut[px.r as usize],
            lut[px.g as usize],
            lut[px.b as usize],
            px.a,
        )
    })
}

/// Negates each channel; alpha passes through. Self-inverse on opaque
/// pixels (property-tested).
#[must_use]
pub fn invert(surface: &Surface) -> Surface {
    let lut = invert_lut();
    apply_lut(surface, &lut)
}

fn invert_lut() -> [u8; 256] {
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        *v = 255 - i as u8;
    }
    lut
}

/// Brightness and contrast in one pinned pass, both −100..=100
/// (values outside clamp).
///
/// Pinned formula, applied contrast-first then brightness, fused into
/// a single LUT:
/// `out = round((in − 128) · (100 + c) / 100) + 128 + round(b · 128 / 100)`
/// so `b = ±100` shifts by ±128 levels and `c = −100` flattens to the
/// midpoint gray (then brightness lifts it).
#[must_use]
pub fn brightness_contrast(surface: &Surface, brightness: i32, contrast: i32) -> Surface {
    let b = brightness.clamp(-100, 100) as i64;
    let c = contrast.clamp(-100, 100) as i64;
    let delta = div_half(b * 128, 100);
    let scale = 100 + c;
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        // Round the signed contraction directly (div_half is
        // half-away-from-zero), not as a difference of two roundings —
        // splitting it drifts a level on signed inputs.
        let contracted = div_half((i as i64 - 128) * scale, 100);
        *v = clamp255(contracted + 128 + delta);
    }
    apply_lut(surface, &lut)
}

/// Levels with the Photoshop-style five controls: remap
/// `[in_black, in_white]` under `gamma` onto `[out_black, out_white]`.
///
/// `gamma == 1.0` takes the integer-exact path (the M4-compose
/// discipline: this is the case journal replay must bit-match).
/// `gamma != 1.0` computes in `f64` — deterministic IEEE math, but not
/// claimed integer-exact; decision 0004 records the split.
///
/// Degenerate inputs are pinned: `in_white <= in_black` maps
/// everything below `in_black` to `out_black` and the rest to
/// `out_white` (span clamps to 1 so the division cannot trap).
#[must_use]
pub fn levels(
    surface: &Surface,
    in_black: u8,
    in_white: u8,
    gamma: f64,
    out_black: u8,
    out_white: u8,
) -> Surface {
    let lo = in_black.min(in_white);
    let hi = in_black.max(in_white);
    let span = (hi - lo).max(1) as i64;
    let (ob, ow) = (out_black.min(out_white), out_black.max(out_white));
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        *v = if gamma == 1.0 {
            let t = (i as i64 - lo as i64).clamp(0, span);
            clamp255(div_half(t * (ow as i64 - ob as i64), span) + ob as i64)
        } else {
            let t = ((i as f64 - f64::from(lo)) / span as f64).clamp(0.0, 1.0);
            let g = t.powf(1.0 / gamma);
            clamp255((f64::from(ob) + f64::from(ow - ob) * g).round() as i64)
        };
    }
    apply_lut(surface, &lut)
}

/// Collapses each channel to `levels` evenly spaced values (2..=256,
/// clamped). `levels = 256` is the exact identity (256 values in, 256
/// levels out — quantization only merges once `levels < 256`); every
/// level count is idempotent, which the tests pin.
#[must_use]
pub fn posterize(surface: &Surface, levels: u32) -> Surface {
    let n = levels.clamp(2, 256) as i64;
    let last = n - 1;
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        let q = div_half(i as i64 * last, 255);
        *v = clamp255(div_half(q * 255, last));
    }
    apply_lut(surface, &lut)
}

/// Binary threshold on Rec.601 luma: `(299r + 587g + 114b) / 1000`
/// (add-half), `>= threshold` → white else black; alpha passes
/// through.
#[must_use]
pub fn threshold(surface: &Surface, threshold: u8) -> Surface {
    let t = i64::from(threshold);
    surface.map_straight(|px| {
        let luma = div_half(
            299 * i64::from(px.r) + 587 * i64::from(px.g) + 114 * i64::from(px.b),
            1000,
        );
        let v = if luma >= t { 255 } else { 0 };
        Rgba8::new(v, v, v, px.a)
    })
}

/// Hue/saturation/lightness shift. `hue_deg` is additive (any value;
/// taken mod 360), `saturation` and `lightness` are deltas in
/// −1..=1 (clamped; saturation scales, lightness adds).
///
/// Runs in `f64` — the HSL round trip is not representable in integer
/// math, and decision 0004 records that this op makes no
/// integer-exactness claim, only a determinism claim plus a ≤1-LSB
/// identity bound at zero adjustment.
#[must_use]
pub fn hsl(surface: &Surface, hue_deg: f64, saturation: f64, lightness: f64) -> Surface {
    let sat_scale = (1.0 + saturation.clamp(-1.0, 1.0)).clamp(0.0, 2.0);
    let light_add = lightness.clamp(-1.0, 1.0);
    surface.map_straight(|px| {
        let (h, s, l) = rgb_to_hsl(px);
        let (r, g, b) = hsl_to_rgb(
            (h + hue_deg).rem_euclid(360.0),
            (s * sat_scale).clamp(0.0, 1.0),
            (l + light_add).clamp(0.0, 1.0),
        );
        Rgba8::new(
            clamp255((r * 255.0).round() as i64),
            clamp255((g * 255.0).round() as i64),
            clamp255((b * 255.0).round() as i64),
            px.a,
        )
    })
}

/// `(h°, s, l)` from straight RGB 0..=255. Standard formulas.
fn rgb_to_hsl(px: Rgba8) -> (f64, f64, f64) {
    let r = f64::from(px.r) / 255.0;
    let g = f64::from(px.g) / 255.0;
    let b = f64::from(px.b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

/// Straight RGB 0..=255 from `(h°, s, l)`. Standard formulas.
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    if s == 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hue = |mut t: f64| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let hk = h / 360.0;
    (hue(hk + 1.0 / 3.0), hue(hk), hue(hk - 1.0 / 3.0))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::SurfaceError;

    /// A 4×1 opaque test strip covering the LUT corners and midtones.
    fn strip() -> Surface {
        Surface::from_straight(
            4,
            1,
            &[
                0, 0, 0, 255, // black
                128, 128, 128, 255, // midtone gray
                255, 0, 0, 255, // pure red
                255, 255, 255, 255, // white
            ],
        )
        .unwrap()
    }

    fn px(s: &Surface, x: u32) -> [u8; 4] {
        s.get(x, 0).unwrap()
    }

    #[test]
    fn invert_is_exact_and_self_inverse() {
        let s = strip();
        let once = invert(&s);
        assert_eq!(px(&once, 0), [255, 255, 255, 255]);
        assert_eq!(px(&once, 1), [127, 127, 127, 255]);
        assert_eq!(px(&once, 2), [0, 255, 255, 255]);
        assert_eq!(
            invert(&once),
            s,
            "invert twice must restore the input bytes"
        );
    }

    #[test]
    fn lut_identity_is_the_identity() {
        let lut = core::array::from_fn(|i| i as u8);
        let s = strip();
        assert_eq!(apply_lut(&s, &lut), s);
    }

    #[test]
    fn brightness_contrast_matches_the_pinned_formula() {
        let s = Surface::from_straight(
            3,
            1,
            &[0, 0, 0, 255, 128, 128, 128, 255, 255, 255, 255, 255],
        )
        .unwrap();

        // brightness +100 → +128 levels, pinned by the formula.
        let bright = brightness_contrast(&s, 100, 0);
        assert_eq!(px(&bright, 0), [128, 128, 128, 255]);
        assert_eq!(px(&bright, 1), [255, 255, 255, 255]); // clamped
        // brightness −100 on midtone: 128 − 128 = 0.
        assert_eq!(px(&brightness_contrast(&s, -100, 0), 1), [0, 0, 0, 255]);

        // contrast +100 doubles the distance from 128: (178−128)·2 = 228.
        let cs = Surface::from_straight(2, 1, &[128, 128, 128, 255, 178, 178, 178, 255]).unwrap();
        let contrasted = brightness_contrast(&cs, 0, 100);
        assert_eq!(px(&contrasted, 0), [128, 128, 128, 255]);
        assert_eq!(px(&contrasted, 1), [228, 228, 228, 255]);

        // contrast −100 flattens to gray 128, then brightness applies.
        let flat = brightness_contrast(&cs, 50, -100);
        assert_eq!(px(&flat, 1), [192, 192, 192, 255]); // 128 + 64
    }

    #[test]
    fn levels_gamma_one_is_integer_exact() {
        let s = Surface::from_straight(
            5,
            1,
            &[
                0, 0, 0, 255, //
                31, 31, 31, 255, //
                32, 32, 32, 255, //
                63, 63, 63, 255, //
                64, 64, 64, 255,
            ],
        )
        .unwrap();
        // Remap [32, 64] → [0, 255]: 32→0, 63→round(31·255/32)=247, 64→255.
        let out = levels(&s, 32, 64, 1.0, 0, 255);
        assert_eq!(px(&out, 0), [0, 0, 0, 255]); // below in_black clamps
        assert_eq!(px(&out, 1), [0, 0, 0, 255]); // 31 clamps to in_black
        assert_eq!(px(&out, 2), [0, 0, 0, 255]);
        assert_eq!(px(&out, 3), [247, 247, 247, 255]);
        assert_eq!(px(&out, 4), [255, 255, 255, 255]);
        // Identity when the range is already full.
        assert_eq!(levels(&s, 0, 255, 1.0, 0, 255), s);
    }

    #[test]
    fn levels_gamma_two_brightens_midtones_photoshop_convention() {
        let s = strip();
        let a = levels(&s, 0, 255, 2.0, 0, 255);
        let b = levels(&s, 0, 255, 2.0, 0, 255);
        assert_eq!(a, b, "f64 path must still be byte-deterministic");
        // Photoshop levels convention: out = t^(1/gamma), so gamma 2
        // lifts midtones — t=0.5 → 0.5^0.5 ≈ 0.7071 → 181.
        assert_eq!(px(&a, 1), [181, 181, 181, 255]);
        // Endpoints survive any gamma.
        assert_eq!(px(&a, 0), [0, 0, 0, 255]);
        assert_eq!(px(&a, 3), [255, 255, 255, 255]);
        // gamma 0.5 mirrors it: t=0.5 → 0.25 → 64.
        assert_eq!(px(&levels(&s, 0, 255, 0.5, 0, 255), 1), [64, 64, 64, 255]);
    }

    #[test]
    fn posterize_quantizes_and_identity_at_255() {
        let s = Surface::from_straight(
            4,
            1,
            &[
                0, 0, 0, 255, 127, 127, 127, 255, 128, 128, 128, 255, 255, 255, 255, 255,
            ],
        )
        .unwrap();
        let two = posterize(&s, 2);
        assert_eq!(px(&two, 0), [0, 0, 0, 255]);
        assert_eq!(px(&two, 1), [0, 0, 0, 255]);
        assert_eq!(px(&two, 2), [255, 255, 255, 255]);
        assert_eq!(px(&two, 3), [255, 255, 255, 255]);

        // levels=256 is the exact identity across all 256 values.
        let all = Surface::from_straight(
            256,
            1,
            &(0..256usize)
                .flat_map(|i| [i as u8, i as u8, i as u8, 255u8])
                .collect::<Vec<u8>>(),
        )
        .unwrap();
        assert_eq!(posterize(&all, 256), all);

        // Quantization is idempotent at every level count: re-posterizing
        // an already-quantized strip changes nothing.
        for lv in [2u32, 3, 7, 16, 64, 128, 255, 256] {
            let out = posterize(&all, lv);
            assert_eq!(posterize(&out, lv), out, "posterize({lv}) not idempotent");
        }
    }

    #[test]
    fn threshold_uses_rec601_luma() {
        // 50% gray (128): luma = round(128·(299+587+114)/1000) = 128 → white at t=128.
        let gray = Surface::from_straight(1, 1, &[128, 128, 128, 255]).unwrap();
        assert_eq!(px(&threshold(&gray, 128), 0), [255, 255, 255, 255]);
        assert_eq!(px(&threshold(&gray, 129), 0), [0, 0, 0, 255]);
        // A saturated blue (0,0,255) has luma ≈ 114 → below a t of 115.
        let blue = Surface::from_straight(1, 1, &[0, 0, 255, 255]).unwrap();
        assert_eq!(px(&threshold(&blue, 115), 0), [0, 0, 0, 255]);
    }

    #[test]
    fn hsl_zero_adjustment_is_within_one_lsb() {
        let all = Surface::from_straight(
            256,
            1,
            &(0..256usize)
                .flat_map(|i| [i as u8, (i * 7 % 256) as u8, (i * 13 % 256) as u8, 255u8])
                .collect::<Vec<u8>>(),
        )
        .unwrap();
        let out = hsl(&all, 0.0, 0.0, 0.0);
        for x in 0..256u32 {
            let (a, b) = (px(&all, x), px(&out, x));
            for ch in 0..3 {
                assert!(
                    (i64::from(a[ch]) - i64::from(b[ch])).abs() <= 1,
                    "identity HSL moved pixel {x} channel {ch}: {} -> {}",
                    a[ch],
                    b[ch]
                );
            }
        }
    }

    #[test]
    fn hsl_rotation_is_exact_on_primaries() {
        let red = Surface::from_straight(1, 1, &[255, 0, 0, 255]).unwrap();
        let cyan = hsl(&red, 180.0, 0.0, 0.0);
        assert_eq!(px(&cyan, 0), [0, 255, 255, 255]);
        // Desaturating fully gives the luma-equivalent gray (l = 0.5 for red).
        let gray = hsl(&red, 0.0, -1.0, 0.0);
        assert_eq!(px(&gray, 0), [128, 128, 128, 255]);
    }

    #[test]
    fn point_ops_preserve_dimensions_and_alpha() {
        let s = Surface::from_straight(
            3,
            2,
            &[
                255, 0, 0, 128, //
                0, 255, 0, 64, //
                0, 0, 255, 200, //
                1, 2, 3, 255, //
                4, 5, 6, 0, //
                7, 8, 9, 1,
            ],
        )
        .unwrap();
        for out in [
            invert(&s),
            brightness_contrast(&s, 20, -30),
            levels(&s, 16, 240, 1.0, 0, 255),
            posterize(&s, 4),
            threshold(&s, 128),
            hsl(&s, 30.0, 0.2, -0.1),
        ] {
            assert_eq!((out.width(), out.height()), (3, 2));
            for x in 0..3u32 {
                for y in 0..2u32 {
                    assert_eq!(
                        out.get(x, y).unwrap()[3],
                        s.get(x, y).unwrap()[3],
                        "alpha changed at ({x},{y})"
                    );
                }
            }
        }
    }

    #[test]
    fn map_straight_never_produces_invalid_premultiplied_pixels() {
        // Noisy translucent pixels must keep rgb <= a after any op
        // (the premultiply invariant compose depends on).
        let s = Surface::from_straight(2, 1, &[255, 255, 255, 128, 10, 200, 30, 3]).unwrap();
        for out in [brightness_contrast(&s, 100, 100), hsl(&s, 120.0, 1.0, 0.5)] {
            for x in 0..2u32 {
                let p = out.get(x, 0).unwrap();
                assert!(p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]);
            }
        }
    }

    #[test]
    fn surfaces_never_construct_from_bad_input() {
        assert_eq!(Surface::new(0, 1).unwrap_err(), SurfaceError::ZeroDimension);
        assert_eq!(
            Surface::from_straight(1, 1, &[]).unwrap_err(),
            SurfaceError::PixelCountMismatch {
                expected: 4,
                actual: 0
            }
        );
    }
}
