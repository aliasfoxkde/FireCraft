//! Geometric blurs: directional and radial resampling (motion blur,
//! zoom blur, radial spin blur).
//!
//! Like the convolutions, these run on the **premultiplied** channels
//! directly (decision 0004) and clamp samples to the image edge.
//! Unlike the axis-aligned convolutions they sample off-grid, so the
//! trigonometry is precomputed into a fixed-point table (sin in
//! thousandths over 4096 divisions of a full turn): `f64` libm results
//! are not cross-platform identical, and the journal-replay contract
//! is byte-stability on every target. The table is built once per
//! process from `f64` `sin` (the radius-selection precedent from the
//! Gaussian cascade); everything per pixel is integer.

use crate::{Surface, clamp255, div_half};

use std::sync::LazyLock;

/// Turn divisions in the trig table (divisible by 4, so 90° rotations
/// are exact table offsets).
const TURNS: i64 = 4096;

/// Sin in thousandths at `angle`/`TURNS` of a full turn.
pub(crate) fn sin_fp(angle: i64) -> i64 {
    static SIN: LazyLock<Vec<i64>> = LazyLock::new(|| {
        (0..TURNS)
            .map(|i| {
                let t = f64::from(i as i32) * std::f64::consts::TAU / TURNS as f64;
                (t.sin() * 1000.0).round() as i64
            })
            .collect()
    });
    SIN[angle.rem_euclid(TURNS) as usize]
}

/// Cos in thousandths, a quarter turn ahead of sin.
pub(crate) fn cos_fp(angle: i64) -> i64 {
    sin_fp(angle + TURNS / 4)
}

/// Degrees → trig-table units (the shared conversion for every
/// angle-taking kernel).
pub(crate) fn angle_units(degrees: f64) -> i64 {
    (degrees * TURNS as f64 / 360.0).round() as i64
}

/// Motion blur: averages `distance + 1` one-sided taps stepping in the
/// `angle_degrees` direction (0° = +x/right, increasing
/// counterclockwise as the UI shows it — image y is down, so the
/// sample step is `(cos θ, −sin θ)` in pixel space). `distance == 0`
/// is an exact copy.
///
/// Substitution recorded: whether CS6 taps one-sided `[0..=d]` or
/// centered `[−d/2..=d/2]` is not observable without a render oracle;
/// one-sided is pinned here (both are constant-invariant; neither is
/// self-inverse under angle+180°, only mirrored).
#[must_use]
pub fn motion_blur(surface: &Surface, angle_degrees: f64, distance: u32) -> Surface {
    let taps = i64::from(distance) + 1;
    let angle = (angle_degrees * TURNS as f64 / 360.0).round() as i64;
    let (cos, sin) = (cos_fp(angle), sin_fp(angle));
    let (w, h) = (surface.width(), surface.height());
    let mut out = surface.clone();
    for y in 0..h {
        for x in 0..w {
            let at = surface.pixel_offset(x, y);
            for ch in 0..4 {
                let mut sum = 0u64;
                for t in 0..taps {
                    // Nearest sample at (x, y) + t·(cos, −sin)/1000,
                    // rounding each coordinate half-up in thousandths.
                    let sx = (i64::from(x) * 1000 + cos * t + 500).div_euclid(1000);
                    let sy = (i64::from(y) * 1000 - sin * t + 500).div_euclid(1000);
                    let sx = sx.clamp(0, i64::from(w) - 1) as u32;
                    let sy = sy.clamp(0, i64::from(h) - 1) as u32;
                    sum += u64::from(surface.pixels[surface.pixel_offset(sx, sy) + ch]);
                }
                out.pixels[at + ch] = clamp255(div_half(sum as i64, taps));
            }
        }
    }
    out
}

/// Zoom blur: averages `quality` (1..=100) taps shrinking the ray from
/// the image center toward the pixel by up to `amount_percent` % —
/// the outward smear. `amount == 0` is an exact copy; `quality == 1`
/// has no second tap to average and is one too.
///
/// Substitution recorded: CS6's exact tap placement (uniform in scale,
/// as here, vs uniform in radius) is not observable.
#[must_use]
pub fn zoom_blur(surface: &Surface, amount_percent: u32, quality: u32) -> Surface {
    let amount = amount_percent.clamp(0, 100) as i64 * 10;
    let taps = i64::from(quality.clamp(1, 100));
    if amount == 0 || taps == 1 {
        return surface.clone();
    }
    let (w, h) = (surface.width(), surface.height());
    let cx = (i64::from(w) - 1) * 500;
    let cy = (i64::from(h) - 1) * 500;
    let mut out = surface.clone();
    for y in 0..h {
        for x in 0..w {
            let at = surface.pixel_offset(x, y);
            let (dx, dy) = (i64::from(x) * 1000 - cx, i64::from(y) * 1000 - cy);
            for ch in 0..4 {
                let mut sum = 0u64;
                for i in 0..taps {
                    let t = i * 1000 / (taps - 1);
                    let scale = 1000 - div_half(t * amount, 1000);
                    let sx = (cx + div_half(dx * scale, 1000) + 500).div_euclid(1000);
                    let sy = (cy + div_half(dy * scale, 1000) + 500).div_euclid(1000);
                    let sx = sx.clamp(0, i64::from(w) - 1) as u32;
                    let sy = sy.clamp(0, i64::from(h) - 1) as u32;
                    sum += u64::from(surface.pixels[surface.pixel_offset(sx, sy) + ch]);
                }
                out.pixels[at + ch] = clamp255(div_half(sum as i64, taps));
            }
        }
    }
    out
}

/// Radial (spin) blur: averages `quality` (1..=100) taps rotating the
/// pixel around the image center across a total sweep of `amount`
/// degrees (the dictionary's observed 1..=100 range read as degrees) —
/// the circular smear. `amount == 0` is an exact copy; `quality == 1`
/// is one too (no second tap).
///
/// Substitution recorded: the sweep-to-amount mapping (degrees, pinned
/// here) and the tap arc (uniform over `[0..=sweep]`, not centered)
/// are not observable; both are pinned for determinism and revisited
/// only on render-oracle evidence.
#[must_use]
pub fn radial_blur_spin(surface: &Surface, amount: u32, quality: u32) -> Surface {
    let sweep_degrees = amount.clamp(0, 360) as i64;
    let taps = i64::from(quality.clamp(1, 100));
    if sweep_degrees == 0 || taps == 1 {
        return surface.clone();
    }
    let sweep = div_half(sweep_degrees * TURNS, 360);
    let (w, h) = (surface.width(), surface.height());
    let cx = (i64::from(w) - 1) * 500;
    let cy = (i64::from(h) - 1) * 500;
    let mut out = surface.clone();
    for y in 0..h {
        for x in 0..w {
            let at = surface.pixel_offset(x, y);
            let (dx, dy) = (i64::from(x) * 1000 - cx, i64::from(y) * 1000 - cy);
            for ch in 0..4 {
                let mut sum = 0u64;
                for i in 0..taps {
                    let angle = i * sweep / (taps - 1);
                    let (cos, sin) = (cos_fp(angle), sin_fp(angle));
                    // Rotate the offset by θ (thousandths → pixels).
                    let rx = div_half(dx * cos - dy * sin, 1000);
                    let ry = div_half(dx * sin + dy * cos, 1000);
                    let sx = (cx + rx + 500).div_euclid(1000);
                    let sy = (cy + ry + 500).div_euclid(1000);
                    let sx = sx.clamp(0, i64::from(w) - 1) as u32;
                    let sy = sy.clamp(0, i64::from(h) - 1) as u32;
                    sum += u64::from(surface.pixels[surface.pixel_offset(sx, sy) + ch]);
                }
                out.pixels[at + ch] = clamp255(div_half(sum as i64, taps));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use color::Rgba8;

    fn ramp() -> Surface {
        let mut s = Surface::new(8, 8).unwrap();
        for y in 0..8u32 {
            for x in 0..8u32 {
                let v = (30 * x + 3 * y) as u8;
                s.set_straight(x, y, Rgba8::new(v, v, v, 255));
            }
        }
        s
    }

    fn px(s: &Surface, x: u32, y: u32) -> [u8; 4] {
        s.get(x, y).unwrap()
    }

    #[test]
    fn trig_table_hits_the_quadrants() {
        assert_eq!(sin_fp(0), 0);
        assert_eq!(sin_fp(TURNS / 4), 1000, "sin 90° = 1");
        assert_eq!(sin_fp(TURNS / 2), 0);
        assert_eq!(sin_fp(3 * TURNS / 4), -1000, "sin 270° = −1");
        assert_eq!(cos_fp(0), 1000);
        assert_eq!(cos_fp(TURNS / 2), -1000);
        // 45° rounds to √2/2 ≈ 707 thousandths.
        assert_eq!(sin_fp(TURNS / 8), 707);
        assert_eq!(cos_fp(TURNS / 8), 707);
    }

    #[test]
    fn motion_blur_noops_at_zero_distance_and_is_deterministic() {
        let s = ramp();
        assert_eq!(motion_blur(&s, 37.0, 0), s);
        assert_eq!(motion_blur(&s, 30.0, 3), motion_blur(&s, 30.0, 3));
    }

    #[test]
    fn motion_blur_smears_along_the_angle_only() {
        // Distance 1 at 0° averages (x, x+1): a pure vertical step
        // between columns 3|4 smears into three bands; the horizontal
        // dimension within a column stays flat.
        let mut s = Surface::new(8, 2).unwrap();
        for y in 0..2u32 {
            for x in 0..8u32 {
                let v = if x < 4 { 100 } else { 200 };
                s.set_straight(x, y, Rgba8::new(v, v, v, 255));
            }
        }
        let out = motion_blur(&s, 0.0, 1);
        // Column 3 taps (3, y) and (4, y): mean 150. Column 4 taps
        // (4, y) twice: 200. Columns ≤ 2 tap their own value.
        assert_eq!(px(&out, 3, 0)[0], 150);
        assert_eq!(px(&out, 4, 0)[0], 200);
        assert_eq!(px(&out, 2, 0)[0], 100);
        assert_eq!(px(&out, 5, 0)[0], 200);
        // Rotating the same blur 180° must mirror the smear to the
        // left: column 4 now taps (3, y), column 3 stays 100.
        let flipped = motion_blur(&s, 180.0, 1);
        assert_eq!(px(&flipped, 4, 0)[0], 150);
        assert_eq!(px(&flipped, 3, 0)[0], 100);
        // 90° (upward in UI terms) moves the smear vertically, so the
        // horizontal step's columns keep their flat values but the two
        // rows mix: row 0 taps (x, 0) and (x, 1) — identical values.
        let up = motion_blur(&s, 90.0, 1);
        assert_eq!(px(&up, 3, 0), px(&s, 3, 0), "rows agree on this input");
    }

    #[test]
    fn zoom_blur_noops_at_zero_amount_and_keeps_the_center() {
        let s = ramp();
        assert_eq!(zoom_blur(&s, 0, 8), s);
        assert_eq!(zoom_blur(&s, 50, 1), s, "single tap cannot average");
        assert_eq!(zoom_blur(&s, 40, 5), zoom_blur(&s, 40, 5));
        // The exact center pixel samples itself at every scale.
        let out = zoom_blur(&s, 60, 9);
        assert_eq!(px(&out, 3, 3), px(&s, 3, 3));
        assert_eq!(px(&out, 4, 4), px(&s, 4, 4));
    }

    #[test]
    fn zoom_blur_pulls_inward_toward_the_center() {
        // White frame, black center: zooming drags center blackness
        // outward, so the ring next to the center darkens.
        let mut s = Surface::new(9, 9).unwrap();
        for y in 0..9u32 {
            for x in 0..9u32 {
                let v = if x == 4 && y == 4 { 0 } else { 255 };
                s.set_straight(x, y, Rgba8::new(v, v, v, 255));
            }
        }
        let out = zoom_blur(&s, 100, 10);
        let ring = i64::from(px(&out, 3, 4)[0]);
        assert!(ring < 255, "center dot must smear outward, got {ring}");
        // 4-fold symmetry of the ring response: the table's quarter-turn
        // offsets and the square grid agree.
        assert_eq!(px(&out, 3, 4), px(&out, 4, 3));
        assert_eq!(px(&out, 5, 4), px(&out, 4, 5));
        assert_eq!(px(&out, 3, 4), px(&out, 5, 4));
    }

    #[test]
    fn radial_spin_noops_at_zero_amount_and_keeps_the_center() {
        let s = ramp();
        assert_eq!(radial_blur_spin(&s, 0, 8), s);
        assert_eq!(radial_blur_spin(&s, 50, 1), s);
        assert_eq!(radial_blur_spin(&s, 25, 6), radial_blur_spin(&s, 25, 6));
        // The exact center pixel (odd size) samples itself at every
        // angle: a zero-length ray cannot rotate.
        let mut s9 = Surface::new(9, 9).unwrap();
        for y in 0..9u32 {
            for x in 0..9u32 {
                let v = (28 * x + 3 * y) as u8;
                s9.set_straight(x, y, Rgba8::new(v, v, v, 255));
            }
        }
        let out = radial_blur_spin(&s9, 45, 8);
        assert_eq!(px(&out, 4, 4), px(&s9, 4, 4));
    }

    #[test]
    fn radial_spin_smears_the_dot_along_the_pinned_sweep_arc() {
        // A single dot at offset (2, 0) from the center, swept 90°:
        // the output darkens where a rotated copy of the pixel offset
        // lands back on the dot — the arc from (2, 0) to (0, −2)
        // (the sweep rotates sample positions by R(+θ), y down), and
        // nowhere else on the ring.
        let mut s = Surface::new(9, 9).unwrap();
        for y in 0..9u32 {
            for x in 0..9u32 {
                let v = if x == 6 && y == 4 { 0 } else { 255 };
                s.set_straight(x, y, Rgba8::new(v, v, v, 255));
            }
        }
        let out = radial_blur_spin(&s, 90, 90);
        assert_eq!(px(&out, 4, 4), [255, 255, 255, 255], "center untouched");
        let at = |dx: i64, dy: i64| px(&out, (4 + dx) as u32, (4 + dy) as u32);
        assert!(at(2, 0)[0] < 255, "arc start (the dot itself) darkened");
        assert!(at(0, -2)[0] < 255, "arc end darkened");
        assert_eq!(at(2, 0), at(0, -2), "endpoint response symmetric");
        assert_eq!(at(-2, 0)[0], 255, "opposite ring untouched");
        assert_eq!(at(0, 2)[0], 255, "counter-arc ring untouched");
        assert_eq!(
            radial_blur_spin(&s, 90, 90),
            radial_blur_spin(&s, 90, 90),
            "deterministic"
        );
    }

    #[test]
    fn uniform_fields_are_fixed_points_of_every_geometric_blur() {
        let flat = Surface::from_straight(7, 5, &[123u8; 7 * 5 * 4]).unwrap();
        assert_eq!(motion_blur(&flat, 33.0, 4), flat);
        assert_eq!(zoom_blur(&flat, 70, 9), flat);
        assert_eq!(radial_blur_spin(&flat, 30, 9), flat);
    }
}
