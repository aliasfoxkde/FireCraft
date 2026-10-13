//! Point operations: per-pixel color remaps.
//!
//! All of them funnel through [`Surface::map_straight`] (or its LUT
//! form), so the premultiply order is decided once in `lib.rs`
//! (decision 0004). LUT-able ops build a 256-entry table and apply it
//! per channel — that keeps the integer-exact proofs to one loop.

use crate::{Surface, clamp255, div_half};
use color::BlendMode;
use color::Rgba8;
use color::blend as blend_mode;
use color::pixel::unpremultiply;

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

/// Knot list of one tone curve, `(x, y)` pairs in 0..=255 (CS6 stores
/// `GPLv` grab points as floats; anything outside the range is
/// clamped when the table is built).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurvesKnots {
    /// The master (`rgb`) curve, applied to all three channels first.
    pub rgb: Vec<(f64, f64)>,
    /// The red curve.
    pub red: Vec<(f64, f64)>,
    /// The green curve.
    pub green: Vec<(f64, f64)>,
    /// The blue curve.
    pub blue: Vec<(f64, f64)>,
}

impl CurvesKnots {
    /// True when every channel's list is empty (an effect that would
    /// change nothing — callers skip it).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rgb.is_empty() && self.red.is_empty() && self.green.is_empty() && self.blue.is_empty()
    }
}

/// Tone curves: piecewise-linear remaps built from grab-point knot
/// lists, the master (`rgb`) curve composed under each channel's own.
///
/// CS6's Curves dialog draws straight segments between grab points
/// (`GPLv`/`GPTv` carry the points; no spline tension is stored), so
/// linear interpolation between sorted knots is the format's own
/// semantics, not a substitution. All table arithmetic is integer:
/// knots scale to thousandths and every interpolation divides through
/// [`crate::div_half`]. Fewer than two knots on a channel is that
/// channel's identity; a fully empty [`CurvesKnots`] yields an exact
/// copy.
#[must_use]
pub fn curves(surface: &Surface, knots: &CurvesKnots) -> Surface {
    if knots.is_empty() {
        return surface.clone();
    }
    let master = curve_lut(&knots.rgb);
    let r = channel_lut(master.as_ref(), &knots.red);
    let g = channel_lut(master.as_ref(), &knots.green);
    let b = channel_lut(master.as_ref(), &knots.blue);
    surface
        .map_straight(|px| Rgba8::new(r[px.r as usize], g[px.g as usize], b[px.b as usize], px.a))
}

/// Builds one channel's final table: the channel's own curve if it
/// has knots, else the master, else identity.
fn channel_lut(master: Option<&[u8; 256]>, knots: &[(f64, f64)]) -> [u8; 256] {
    match curve_lut(knots) {
        Some(own) => match master {
            Some(m) => std::array::from_fn(|v| m[own[v] as usize]),
            None => own,
        },
        None => match master {
            Some(m) => *m,
            None => std::array::from_fn(|v| v as u8),
        },
    }
}

/// Piecewise-linear 256-entry table from sorted knots; `None` when
/// fewer than two knots (identity). Interpolation is thousandths-
/// fixed-point with house rounding, so a pinned knot list produces
/// byte-stable tables.
fn curve_lut(knots: &[(f64, f64)]) -> Option<[u8; 256]> {
    if knots.len() < 2 {
        return None;
    }
    let mut pts: Vec<(i64, i64)> = knots
        .iter()
        .map(|(x, y)| {
            (
                (x.clamp(0.0, 255.0) * 1000.0).round() as i64,
                (y.clamp(0.0, 255.0) * 1000.0).round() as i64,
            )
        })
        .collect();
    pts.sort_by_key(|(x, y)| (*x, *y));
    pts.dedup_by_key(|(x, _)| *x);
    if pts.len() < 2 {
        return None;
    }
    let mut lut = [0u8; 256];
    for (v, out) in lut.iter_mut().enumerate() {
        let x = v as i64 * 1000;
        let y = if x <= pts[0].0 {
            pts[0].1
        } else if x >= pts[pts.len() - 1].0 {
            pts[pts.len() - 1].1
        } else {
            // First segment whose upper edge reaches x — one always
            // does for x strictly inside the sorted span (the fallback
            // below keeps the lookup total regardless).
            let at = pts
                .windows(2)
                .position(|w| x <= w[1].0)
                .unwrap_or(pts.len() - 2);
            let seg = &pts[at..at + 2];
            let span = seg[1].0 - seg[0].0;
            if span == 0 {
                seg[1].1
            } else {
                seg[0].1 + div_half((seg[1].1 - seg[0].1) * (x - seg[0].0), span)
            }
        };
        *out = clamp255(div_half(y, 1000));
    }
    Some(lut)
}

/// Auto Levels: stretches each channel's histogram to the full
/// 0..=255 range — the lowest non-zero bin to 0, the highest to 255.
///
/// Pixels with any opacity vote in the histogram; fully transparent
/// pixels carry no color information (their straight color is
/// undefined) and are excluded. A channel whose votes collapse to a
/// single bin maps to itself. Substitution recorded: CS6's
/// `StHistogramRGBProcessor` family may clip a fraction of outlier
/// bins — not observable without a render oracle, so no clipping
/// happens here.
#[must_use]
pub fn auto_levels(surface: &Surface) -> Surface {
    let mut hist = [0u64; 768];
    for px in surface.pixels().chunks_exact(4) {
        if px[3] == 0 {
            continue;
        }
        let straight = unpremultiply([
            u16::from(px[0]),
            u16::from(px[1]),
            u16::from(px[2]),
            u16::from(px[3]),
        ]);
        hist[usize::from(straight.r)] += 1;
        hist[256 + usize::from(straight.g)] += 1;
        hist[512 + usize::from(straight.b)] += 1;
    }
    let mut luts = [[0u8; 256]; 3];
    for (ch, lut) in luts.iter_mut().enumerate() {
        let bins = &hist[ch * 256..(ch + 1) * 256];
        let Some(lo) = bins.iter().position(|c| *c > 0) else {
            // No votes: the channel stays identity (all pixels were
            // transparent).
            *lut = std::array::from_fn(|v| v as u8);
            continue;
        };
        let hi = bins.iter().rposition(|c| *c > 0).unwrap_or(lo);
        if hi <= lo {
            *lut = std::array::from_fn(|v| v as u8);
        } else {
            for (v, out) in lut.iter_mut().enumerate() {
                *out = clamp255(div_half(
                    (v as i64 - lo as i64) * 255,
                    hi as i64 - lo as i64,
                ));
            }
        }
    }
    surface.map_straight(|px| {
        Rgba8::new(
            luts[0][px.r as usize],
            luts[1][px.g as usize],
            luts[2][px.b as usize],
            px.a,
        )
    })
}

/// Convert to Alpha: turns the image into a black stencil whose
/// opacity is the source's Rec.601 luma scaled by its own alpha —
/// light pixels become opaque, dark pixels transparent.
///
/// Public behavior of the CS6 filter (dark → transparent, light →
/// opaque); its exact integer arithmetic lives behind
/// `CCreateAlphaFilter` with no RTTI-visible processor, so the
/// house luma `(299r + 587g + 114b) / 1000` with add-half rounding
/// is the pinned substitution (same discipline as the noise
/// generator).
#[must_use]
pub fn convert_to_alpha(surface: &Surface) -> Surface {
    surface.map_straight(|px| {
        let luma = div_half(
            299 * i64::from(px.r) + 587 * i64::from(px.g) + 114 * i64::from(px.b),
            1000,
        );
        Rgba8::new(0, 0, 0, clamp255(div_half(luma * i64::from(px.a), 255)))
    })
}

/// Color Fill: blends a solid color over every pixel at
/// `opacity_percent` (0..=100) in the given blend mode, keeping each
/// pixel's own alpha — the fill recolors the object's silhouette and
/// cannot paint outside it.
///
/// The mode blends two fully opaque colors (the fill over the
/// pixel's color) and the result is lerped toward the pixel's own
/// color by the inverse opacity. Transparent pixels pass through
/// untouched (a fill must not produce a rectangle).
#[must_use]
pub fn solid_fill(
    surface: &Surface,
    fill: Rgba8,
    opacity_percent: u32,
    mode: BlendMode,
) -> Surface {
    let opacity = i64::from(opacity_percent.clamp(0, 100));
    if opacity == 0 {
        return surface.clone();
    }
    let rest = 100 - opacity;
    surface.map_straight(move |px| {
        if px.a == 0 {
            return px;
        }
        let mixed = blend_mode(
            mode,
            Rgba8::new(fill.r, fill.g, fill.b, 255),
            Rgba8::new(px.r, px.g, px.b, 255),
        );
        let channel =
            |m: u8, c: u8| clamp255(div_half(i64::from(m) * opacity + i64::from(c) * rest, 100));
        Rgba8::new(
            channel(mixed.r, px.r),
            channel(mixed.g, px.g),
            channel(mixed.b, px.b),
            px.a,
        )
    })
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

    // ---- curves ----

    #[test]
    fn curves_interpolates_linearly_between_knots() {
        // Knots (0,0), (128,64), (255,255): strip pixel 0 sits on the
        // first knot, pixel 3 on the last; the 128-gray pixel sits on
        // the middle knot and must come out exactly 64.
        let knots = CurvesKnots {
            rgb: vec![(0.0, 0.0), (128.0, 64.0), (255.0, 255.0)],
            ..CurvesKnots::default()
        };
        let out = curves(&strip(), &knots);
        assert_eq!(px(&out, 0), [0, 0, 0, 255], "knot 0 exact");
        assert_eq!(px(&out, 3), [255, 255, 255, 255], "knot end exact");
        assert_eq!(px(&out, 1), [64, 64, 64, 255], "mid knot exact");
        // Halfway along the (0,0)→(128,64) segment: x=64 → y=32.
        let quarter = Surface::from_straight(1, 1, &[64, 64, 64, 255]).unwrap();
        assert_eq!(px(&curves(&quarter, &knots), 0), [32, 32, 32, 255]);
    }

    #[test]
    fn curves_identity_degenerates_and_channel_composition() {
        let s = strip();
        // Empty lists are an exact copy.
        assert_eq!(curves(&s, &CurvesKnots::default()), s);
        // Fewer than two knots on every list: copy.
        let lone = CurvesKnots {
            rgb: vec![(64.0, 200.0)],
            ..CurvesKnots::default()
        };
        assert_eq!(curves(&s, &lone), s);
        // Channel curve first, then the master on the result (the
        // composite-pass order): red lifts 128 → 255, master then
        // halves 255 → 128; green/blue take only the master halving.
        let knots = CurvesKnots {
            rgb: vec![(0.0, 0.0), (255.0, 128.0)],
            red: vec![(0.0, 0.0), (128.0, 255.0), (255.0, 255.0)],
            ..CurvesKnots::default()
        };
        let out = curves(&s, &knots);
        let p = px(&out, 1); // 128-gray pixel
        assert_eq!(p[0], 128, "red 128→255, then master halves");
        assert_eq!(p[1], 64, "green halves only");
        assert_eq!(p[2], 64, "blue halves only");
        // Channel-only knot list leaves other channels to the master
        // (absent → identity).
        let red_only = CurvesKnots {
            red: vec![(0.0, 0.0), (255.0, 0.0)],
            ..CurvesKnots::default()
        };
        let out = curves(&s, &red_only);
        assert_eq!(px(&out, 2), [0, 0, 0, 255], "red collapsed, g/b kept");
        assert_eq!(curves(&s, &knots), curves(&s, &knots), "deterministic");
    }

    // ---- auto levels ----

    #[test]
    fn auto_levels_stretches_each_channel_full_range() {
        // Red occupies 64..192 only; green is flat; blue touches both
        // ends. After the stretch: red 64→0 and 192→255; green stays
        // (single bin → identity); blue is already full-range.
        let mut s = Surface::new(3, 1).unwrap();
        s.set_straight(0, 0, Rgba8::new(64, 100, 0, 255));
        s.set_straight(1, 0, Rgba8::new(128, 100, 255, 255));
        s.set_straight(2, 0, Rgba8::new(192, 100, 255, 255));
        let out = auto_levels(&s);
        assert_eq!(px(&out, 0)[0], 0);
        assert_eq!(px(&out, 1)[0], 128, "(128−64)·255/128 = 127.5 → 128");
        assert_eq!(px(&out, 2)[0], 255);
        assert_eq!(px(&out, 0)[1], 100, "single-bin channel is identity");
        assert_eq!(px(&out, 1)[1], 100);
        assert_eq!(px(&out, 0)[2], 0, "full-range channel unchanged");
        assert_eq!(px(&out, 1)[2], 255);
    }

    #[test]
    fn auto_levels_ignores_fully_transparent_pixels() {
        // Only the opaque pixel votes: its lone bin is identity, and
        // the transparent-zero never drags the floor down.
        let s = Surface::from_straight(2, 1, &[200, 200, 200, 255, 0, 0, 0, 0]).unwrap();
        let out = auto_levels(&s);
        assert_eq!(px(&out, 0), [200, 200, 200, 255]);
        assert_eq!(px(&out, 1), [0, 0, 0, 0], "transparent stays transparent");
        assert_eq!(auto_levels(&s), auto_levels(&s), "deterministic");
    }

    // ---- convert to alpha ----

    #[test]
    fn convert_to_alpha_maps_luma_to_opacity_over_black() {
        let s = strip();
        let out = convert_to_alpha(&s);
        assert_eq!(px(&out, 0), [0, 0, 0, 0], "black → transparent");
        assert_eq!(px(&out, 3), [0, 0, 0, 255], "white → opaque black");
        // 128-gray: luma 128 (Rec.601 of a gray is itself).
        assert_eq!(px(&out, 1), [0, 0, 0, 128]);
        // Pure red: luma 299·255/1000 = 76.245 → 76; color gone.
        assert_eq!(px(&out, 2), [0, 0, 0, 76]);
        // Existing alpha scales the result: half-alpha white → 128.
        let half = Surface::from_straight(1, 1, &[255, 255, 255, 128]).unwrap();
        let out = convert_to_alpha(&half);
        assert_eq!(px(&out, 0), [0, 0, 0, 128], "255·128/255 = 128");
        assert_eq!(convert_to_alpha(&s), convert_to_alpha(&s), "deterministic");
    }

    // ---- color fill ----

    #[test]
    fn solid_fill_recolors_the_silhouette_at_opacity() {
        // Opaque strip + one transparent pixel: the fill must recolor
        // opaque pixels and never paint the transparent one.
        let s =
            Surface::from_straight(3, 1, &[10, 20, 30, 255, 10, 20, 30, 128, 0, 0, 0, 0]).unwrap();
        let fill = Rgba8::new(255, 0, 0, 255);
        let full = solid_fill(&s, fill, 100, BlendMode::Normal);
        assert_eq!(px(&full, 0), [255, 0, 0, 255], "100% replaces color");
        // `get` reads premultiplied bytes: red 255 at alpha 128 stores
        // as 128. The straight color under it is the fill's.
        assert_eq!(px(&full, 1), [128, 0, 0, 128], "alpha preserved");
        assert_eq!(px(&full, 2), [0, 0, 0, 0], "transparent untouched");
        // 0% is an exact copy.
        assert_eq!(solid_fill(&s, fill, 0, BlendMode::Normal), s);
        // 50% normal: (10,20,30) vs (255,0,0) → 133, 10, 15 (half-up).
        let half = solid_fill(&s, fill, 50, BlendMode::Normal);
        assert_eq!(px(&half, 0), [133, 10, 15, 255]);
        assert_eq!(solid_fill(&s, fill, 50, BlendMode::Normal), half);
    }

    #[test]
    fn solid_fill_blend_modes_match_color_semantics() {
        // Multiply red × cyan-gray: per-channel product semantics.
        let s = Surface::from_straight(1, 1, &[128, 255, 0, 255]).unwrap();
        let fill = Rgba8::new(255, 0, 0, 255);
        let out = solid_fill(&s, fill, 100, BlendMode::Multiply);
        // multiply(128,255)=128, multiply(255,0)=0, multiply(0,0)=0.
        assert_eq!(px(&out, 0), [128, 0, 0, 255]);
        // Screen: white under any fill stays white.
        let white = Surface::from_straight(1, 1, &[255, 255, 255, 255]).unwrap();
        let out = solid_fill(&white, fill, 100, BlendMode::Screen);
        assert_eq!(px(&out, 0), [255, 255, 255, 255]);
        assert_eq!(
            solid_fill(&s, fill, 100, BlendMode::Multiply),
            solid_fill(&s, fill, 100, BlendMode::Multiply),
            "deterministic"
        );
    }
}
