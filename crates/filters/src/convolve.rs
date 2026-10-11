//! Convolutions: separable blurs, unsharp masking, edge and relief
//! kernels.
//!
//! Everything here runs on the **premultiplied** channels directly
//! (decision 0004): convolution is a weighted average, and averaging
//! premultiplied colors is exactly what keeps halos out of transparent
//! regions. Neighborhood sampling clamps to the image edge. Sums use
//! `u64` accumulators with the house add-half divide
//! ([`crate::div_half`]) so a pass is byte-stable; a *direct sum per
//! pixel*, not a sliding window, on purpose — a future sliding-window
//! optimization must first prove byte-equality against these sums, and
//! the determinism tests below are that gate.

use crate::{Surface, clamp255, div_half};

/// Box blur of the given radius (`0` returns an exact copy).
/// Separable: one horizontal pass, then one vertical pass, each an
/// unweighted mean over `2r+1` clamp-to-edge samples per channel.
#[must_use]
pub fn box_blur(surface: &Surface, radius: u32) -> Surface {
    if radius == 0 {
        return surface.clone();
    }
    let r = radius as usize;
    let horizontal = blur_axis(surface, r, true);
    blur_axis(&horizontal, r, false)
}

/// Gaussian blur by three successive box passes (Wells' / Kutskir's
/// cascade — three boxes approximate a Gaussian well and each is
/// separable). `sigma <= 0` returns an exact copy.
///
/// Only the *radius selection* uses `f64` (the public-domain
/// `boxes_for_gauss` below, pinned in decision 0004); every pixel
/// computation stays integer.
#[must_use]
pub fn gaussian_blur(surface: &Surface, sigma: f64) -> Surface {
    if sigma <= 0.0 {
        return surface.clone();
    }
    let mut out = surface.clone();
    for r in boxes_for_gauss(sigma, 3) {
        if r == 0 {
            continue;
        }
        out = blur_axis(&out, r, true);
        out = blur_axis(&out, r, false);
    }
    out
}

/// Unsharp mask: `out = orig + amount% · (orig − blurred)`, computed
/// on the premultiplied channels with signed house rounding.
/// `amount_percent` clamps to 0..=500; `0` returns an exact copy
/// (no round trip).
#[must_use]
pub fn unsharp_mask(surface: &Surface, sigma: f64, amount_percent: u32) -> Surface {
    let amount = amount_percent.min(500) as i64;
    if amount == 0 {
        return surface.clone();
    }
    let blurred = gaussian_blur(surface, sigma);
    let mut out = surface.clone();
    for (px, soft) in out.pixels.iter_mut().zip(&blurred.pixels) {
        let orig = i64::from(*px);
        let delta = orig - i64::from(*soft);
        *px = clamp255(orig + div_half(amount * delta, 100));
    }
    out
}

/// Sobel edge magnitude, `|Gx| + |Gy|` per premultiplied channel,
/// saturating to 255. The source alpha passes through unchanged (this
/// detects color structure; an all-zero alpha output would hide the
/// edges it finds — pinned by test).
#[must_use]
pub fn find_edges(surface: &Surface) -> Surface {
    let (w, h) = (surface.width() as usize, surface.height() as usize);
    let mut out = surface.clone();
    for y in 0..h as u32 {
        for x in 0..w as u32 {
            let at = surface.pixel_offset(x, y);
            for ch in 0..3 {
                let gx = sobel_axis(surface, x, y, ch, true);
                let gy = sobel_axis(surface, x, y, ch, false);
                out.pixels[at + ch] = clamp255(gx.abs() + gy.abs());
            }
        }
    }
    out
}

/// One Sobel 3×3 gradient component. `Gx` weights rows `[1,2,1]`
/// across the x-axis stencil `[-1,0,1]`; `Gy` swaps the roles.
fn sobel_axis(surface: &Surface, x: u32, y: u32, ch: usize, horizontal: bool) -> i64 {
    let (w, h) = (surface.width(), surface.height());
    let (row_w, col_w): ([i64; 3], [i64; 3]) = if horizontal {
        ([1, 2, 1], [-1, 0, 1])
    } else {
        ([-1, 0, 1], [1, 2, 1])
    };
    let mut sum = 0i64;
    for (row, rw) in row_w.iter().enumerate() {
        let dy = row as i64 - 1;
        let sy = (i64::from(y) + dy).clamp(0, i64::from(h) - 1) as u32;
        for dx in -1i64..=1 {
            let sx = (i64::from(x) + dx).clamp(0, i64::from(w) - 1) as u32;
            let p = surface.pixels[surface.pixel_offset(sx, sy) + ch];
            sum += i64::from(p) * rw * col_w[(dx + 1) as usize];
        }
    }
    sum
}

/// Emboss: relieves the image toward a light at `azimuth_degrees`
/// (0° = from the right, increasing clockwise — image y is down).
/// `out = center − neighbor(toward light) + 128` per premultiplied
/// channel, alpha kept. The neighbor is one pixel along the rounded
/// 8-way direction, edge-clamped.
#[must_use]
pub fn emboss(surface: &Surface, azimuth_degrees: f64) -> Surface {
    const DIRECTIONS: [(i64, i64); 8] = [
        (1, 0),   //   0° east
        (1, 1),   //  45° south-east
        (0, 1),   //  90° south
        (-1, 1),  // 135° south-west
        (-1, 0),  // 180° west
        (-1, -1), // 225° north-west
        (0, -1),  // 270° north
        (1, -1),  // 315° north-east
    ];
    let octant = (azimuth_degrees / 45.0).round() as i64;
    let (dx, dy) = DIRECTIONS[octant.rem_euclid(8) as usize];
    let (w, h) = (surface.width(), surface.height());
    let mut out = surface.clone();
    for y in 0..h {
        for x in 0..w {
            let nx = (i64::from(x) + dx).clamp(0, i64::from(w - 1)) as u32;
            let ny = (i64::from(y) + dy).clamp(0, i64::from(h - 1)) as u32;
            let at = surface.pixel_offset(x, y);
            let neighbor = surface.pixel_offset(nx, ny);
            for ch in 0..3 {
                let center = i64::from(surface.pixels[at + ch]);
                let other = i64::from(surface.pixels[neighbor + ch]);
                out.pixels[at + ch] = clamp255(center - other + 128);
            }
        }
    }
    out
}

/// One separable pass: mean over `2r+1` samples per pixel per channel
/// (alpha included — the box must stay a plain average for the
/// constant-invariance proof to hold).
fn blur_axis(surface: &Surface, r: usize, horizontal: bool) -> Surface {
    let (w, h) = (surface.width(), surface.height());
    let mut out = surface.clone();
    let span = 2 * r + 1;
    let (outer, inner) = if horizontal { (h, w) } else { (w, h) };
    for o in 0..outer {
        for i in 0..inner {
            let at = if horizontal {
                surface.pixel_offset(i, o)
            } else {
                surface.pixel_offset(o, i)
            };
            for ch in 0..4 {
                let mut sum = 0u64;
                for d in -(r as i64)..=r as i64 {
                    let s = (i as i64 + d).clamp(0, inner as i64 - 1) as usize;
                    let sp = if horizontal {
                        surface.pixel_offset(s as u32, o)
                    } else {
                        surface.pixel_offset(o, s as u32)
                    };
                    sum += u64::from(surface.pixels[sp + ch]);
                }
                out.pixels[at + ch] = ((sum + span as u64 / 2) / span as u64) as u8;
            }
        }
    }
    out
}

/// Radius selection for the 3-box Gaussian cascade (public-domain
/// `boxesForGauss`, Ivan Kutskir's algorithm): ideal box width
/// `sqrt(12σ²/n + 1)` forced odd, with `m` of the `n` boxes widened by
/// 2 to absorb the remainder. `f64` appears only here; the returned
/// radii fully determine the integer passes.
fn boxes_for_gauss(sigma: f64, n: usize) -> Vec<usize> {
    let n = n as f64;
    let sigma_sq = sigma * sigma;
    let w_ideal = (12.0 * sigma_sq / n + 1.0).sqrt();
    let mut wl = w_ideal.floor();
    if wl % 2.0 == 0.0 {
        wl -= 1.0;
    }
    let wu = wl + 2.0;
    let m_ideal = (12.0 * sigma_sq - n * wl * wl - 4.0 * n * wl - 3.0 * n) / (-4.0 * wl - 4.0);
    let m = m_ideal.round();
    (0..n as usize)
        .map(|i| {
            let size = if (i as f64) < m { wu } else { wl };
            ((size - 1.0) / 2.0) as usize
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// 6×6 opaque gray ramp (byte = 40x + 4y) — enough structure to
    /// tell a blur from a copy in any direction.
    fn ramp() -> Surface {
        let mut s = Surface::new(6, 6).unwrap();
        for y in 0..6u32 {
            for x in 0..6u32 {
                let v = (40 * x + 4 * y) as u8;
                s.set_straight(x, y, color::Rgba8::new(v, v, v, 255));
            }
        }
        s
    }

    fn px(s: &Surface, x: u32, y: u32) -> [u8; 4] {
        s.get(x, y).unwrap()
    }

    #[test]
    fn box_blur_is_deterministic_and_constant_invariant() {
        let s = ramp();
        assert_eq!(box_blur(&s, 2), box_blur(&s, 2), "same bytes in, same out");
        // A uniform opaque field is a fixed point of a clamped box mean.
        let flat = Surface::from_straight(9, 7, &[77u8; 9 * 7 * 4]).unwrap();
        assert_eq!(box_blur(&flat, 3), flat);
        // Radius 0 is an exact copy.
        assert_eq!(box_blur(&s, 0), s);
    }

    #[test]
    fn box_blur_symmetry() {
        // The ramp is linear, so blurring along x only must not move the
        // center column's value (odd window, linear signal).
        let s = ramp();
        let h = blur_axis(&s, 2, true);
        assert_eq!(px(&h, 2, 3), px(&s, 2, 3));
        // Transposing the pass direction mirrors the change, not the
        // result: vertical-only leaves the center row's value alone.
        let v = blur_axis(&s, 2, false);
        assert_eq!(px(&v, 3, 2), px(&s, 3, 2));
    }

    #[test]
    fn gaussian_blur_is_deterministic_constant_invariant_and_noop_at_zero() {
        let s = ramp();
        assert_eq!(gaussian_blur(&s, 1.4), gaussian_blur(&s, 1.4));
        assert_eq!(gaussian_blur(&s, 0.0), s);
        assert_eq!(gaussian_blur(&s, -2.0), s);
        let flat = Surface::from_straight(9, 7, &[200u8; 9 * 7 * 4]).unwrap();
        assert_eq!(gaussian_blur(&flat, 2.5), flat);
    }

    #[test]
    fn boxes_for_gauss_selects_pinned_radii() {
        // Recomputed by hand from the pinned formula (decision 0004);
        // these triples ARE the gaussian's meaning of each sigma.
        assert_eq!(boxes_for_gauss(2.0, 3), vec![2, 2, 1]);
        assert_eq!(boxes_for_gauss(5.0, 3), vec![5, 5, 4]);
        assert_eq!(boxes_for_gauss(8.0, 3), vec![8, 8, 7]);
        // The cascade's minimum box width is 3, so even a tiny sigma
        // blurs by radius 1 — callers wanting a no-op pass sigma <= 0.
        assert_eq!(boxes_for_gauss(0.1, 3), vec![1, 1, 1]);
    }

    #[test]
    fn gaussian_spreads_a_single_dot() {
        // One black pixel at the center of a white field darkens the
        // center most, with non-increasing falloff along the axes (the
        // integer cascade quantizes the response tail into plateaus —
        // measured σ=1.5 row: 255,255,254,253,252,250,250,249,... — so
        // the pin is non-increasing, not strictly monotonic).
        let mut s = Surface::from_straight(15, 15, &[255u8; 15 * 15 * 4]).unwrap();
        s.set_straight(7, 7, color::Rgba8::new(0, 0, 0, 255));
        let out = gaussian_blur(&s, 1.5);
        let lum = |x: u32, y: u32| i64::from(px(&out, x, y)[0]);
        assert!(lum(7, 7) < 255, "dot must survive blurred");
        assert!(lum(7, 7) < lum(6, 7), "center strictly darkest");
        assert!(lum(6, 7) <= lum(5, 7), "falloff non-increasing");
        assert!(lum(5, 7) < 255, "blur reaches past one pixel");
        assert_eq!(lum(0, 0), 255, "far corner untouched");
    }

    #[test]
    fn unsharp_sharpens_toward_the_original_and_noops_at_zero_amount() {
        let s = ramp();
        assert_eq!(unsharp_mask(&s, 1.0, 0), s);
        let sharpened = unsharp_mask(&s, 1.0, 100);
        assert_eq!(unsharp_mask(&s, 1.0, 100), sharpened, "deterministic");
        // Unsharp(blur(x)) with amount 100 reconstructs x up to integer
        // rounding: |orig + (orig − blur(orig)) − orig| = |orig − blur| ≥
        // the blurred distance from orig in the direction of orig. On the
        // center of the ramp, sharpening must move the blurred value back
        // toward (or past) the original, never away.
        let blurred = gaussian_blur(&s, 1.0);
        for x in 1..5u32 {
            for y in 1..5u32 {
                let orig = i64::from(px(&s, x, y)[0]);
                let soft = i64::from(px(&blurred, x, y)[0]);
                let sharp = i64::from(px(&sharpened, x, y)[0]);
                let before = (orig - soft).abs();
                let after = (orig - sharp).abs();
                assert!(
                    after <= before,
                    "sharpening moved ({x},{y}) away from original: {soft} -> {sharp}, orig {orig}"
                );
            }
        }
    }

    #[test]
    fn find_edges_marks_both_sides_of_a_step() {
        // Vertical step: columns 0..4 black, 4..8 white, opaque.
        let mut s = Surface::new(8, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..8u32 {
                let v = if x < 4 { 0 } else { 255 };
                s.set_straight(x, y, color::Rgba8::new(v, v, v, 255));
            }
        }
        let out = find_edges(&s);
        for y in 0..4u32 {
            for x in 0..8u32 {
                let expect = if x == 3 || x == 4 { 255 } else { 0 };
                assert_eq!(px(&out, x, y), [expect, expect, expect, 255]);
            }
        }
        // Uniform input has no edges: RGB collapses to black, alpha kept.
        let flat = Surface::from_straight(3, 3, &[90u8; 3 * 3 * 4]).unwrap();
        let out = find_edges(&flat);
        assert_eq!(px(&out, 1, 1), [0, 0, 0, 90]);
        assert_eq!(find_edges(&s), find_edges(&s), "deterministic");
    }

    #[test]
    fn emboss_relief_direction_follows_azimuth() {
        // Horizontal ramp increasing to the right on a flat 128 base.
        let mut s = Surface::new(8, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..8u32 {
                let v = 128u8;
                s.set_straight(x, y, color::Rgba8::new(v, v, v, 255));
            }
        }
        // Flat input relieves to exact mid-gray everywhere, any azimuth.
        for az in [0.0, 90.0, 180.0, 315.0] {
            let out = emboss(&s, az);
            for x in 0..8u32 {
                assert_eq!(px(&out, x, 1), [128, 128, 128, 255]);
            }
        }

        // Ramp base: left half 96, right half 160 — the light side flips
        // the relief sign with the azimuth.
        let mut ramp = Surface::new(8, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..8u32 {
                let v = if x < 4 { 96 } else { 160 };
                ramp.set_straight(x, y, color::Rgba8::new(v, v, v, 255));
            }
        }
        let east = emboss(&ramp, 0.0); // neighbor = right (x+1)
        let west = emboss(&ramp, 180.0); // neighbor = left (x−1)
        // Boundary column 3 (96) samples right (160): 96−160+128 = 64.
        assert_eq!(px(&east, 3, 1), [64, 64, 64, 255]);
        // Same column under west light samples left (96): 96−96+128 = 128.
        assert_eq!(px(&west, 3, 1), [128, 128, 128, 255]);
        // Boundary column 4 (160) samples right (160): 128; left (96): 192.
        assert_eq!(px(&east, 4, 1), [128, 128, 128, 255]);
        assert_eq!(px(&west, 4, 1), [192, 192, 192, 255]);
        assert_eq!(emboss(&ramp, 0.0), east, "deterministic");
    }

    #[test]
    fn blur_preserves_alpha_plane_means() {
        // Half-transparent rows blur their alpha too (a plain average).
        let mut s = Surface::new(4, 3).unwrap();
        for y in 0..3u32 {
            for x in 0..4u32 {
                let a = if y == 0 { 0 } else { 255 };
                s.set_straight(x, y, color::Rgba8::new(255, 0, 0, a));
            }
        }
        let out = box_blur(&s, 1);
        // Middle row: two 255 rows above/below? y=1 sees y=0 (a=0) and
        // y=2 (a=255) → mean 128 with r=3 window; exact per-row values:
        assert_eq!(px(&out, 1, 1)[3], 170); // (0+255+255)/3 = 170
        assert_eq!(px(&out, 1, 0)[3], 85); // (0+0+255)/3 = 85
    }
}
