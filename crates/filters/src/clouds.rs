//! Clouds: the Render family's value-noise fills (Clouds
//! `ff84d00c-…`, Difference Clouds `d2e7769c-…`; dictionary
//! parameters `sky_color`, `cloud_color`).
//!
//! CS6's fractal-noise kernel is unobservable without a render
//! oracle, so the texture is a pinned substitution, recorded here per
//! the M7 discipline: one-octave value noise on a 16 px lattice. Each
//! lattice point hashes `(seed, x, y)` through the SplitMix64 final
//! cascade — position-independent, so the same world pixel carries the
//! same texture in any bounding box — and the field bilinearly
//! interpolates the four corners in integer math with add-half
//! rounding. Deterministic by construction: no clocks, no sequential
//! stream, replay-stable.
//!
//! Clouds FILLS the source's opaque bounding box: every pixel inside
//! the box becomes opaque texture (the CS6 filter fills the selection,
//! the box is the effect's reach), pixels outside clear to
//! transparent. Difference Clouds keeps the source silhouette and
//! subtracts per channel: `out = |src − texture|` on straight colors,
//! alpha untouched.

use crate::{Surface, clamp255, div_half};
use color::Rgba8;
use color::pixel::unpremultiply;

/// Lattice cell size in pixels.
const CELL: i64 = 16;

/// One lattice value: the SplitMix64 final cascade over
/// `(seed, x, y)`, folded to 0..=255.
fn lattice(seed: u64, gx: i64, gy: i64) -> i64 {
    let x = u64::from_ne_bytes(gx.to_ne_bytes());
    let y = u64::from_ne_bytes(gy.to_ne_bytes());
    let mut z =
        seed ^ x.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ y.wrapping_mul(0xC2B2_3AE3_27D4_4B4F);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((z ^ (z >> 31)) & 0xFF) as i64
}

/// The noise field at a world pixel: bilinear over the four
/// surrounding lattice corners, integer-exact.
fn value_noise(x: i64, y: i64, seed: u64) -> i64 {
    let (gx, gy) = (x.div_euclid(CELL), y.div_euclid(CELL));
    let (fx, fy) = (x.rem_euclid(CELL), y.rem_euclid(CELL));
    let a = lattice(seed, gx, gy);
    let b = lattice(seed, gx + 1, gy);
    let c = lattice(seed, gx, gy + 1);
    let d = lattice(seed, gx + 1, gy + 1);
    let top = div_half(a * (CELL - fx) + b * fx, CELL);
    let bottom = div_half(c * (CELL - fx) + d * fx, CELL);
    div_half(top * (CELL - fy) + bottom * fy, CELL)
}

/// The tightest box holding every pixel with alpha (`x0, y0, x1, y1`
/// inclusive), or `None` on an empty surface.
fn alpha_bbox(surface: &Surface) -> Option<(u32, u32, u32, u32)> {
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for y in 0..surface.height() {
        for x in 0..surface.width() {
            if surface.pixels[surface.pixel_offset(x, y) + 3] == 0 {
                continue;
            }
            bbox = Some(match bbox {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    bbox
}

/// The texture color at a world pixel: `sky` toward `cloud` as the
/// noise rises, integer-exact.
fn texture(sky: Rgba8, cloud: Rgba8, n: i64) -> Rgba8 {
    let channel =
        |s: u8, c: u8| clamp255(div_half(i64::from(s) * (255 - n) + i64::from(c) * n, 255));
    Rgba8::new(
        channel(sky.r, cloud.r),
        channel(sky.g, cloud.g),
        channel(sky.b, cloud.b),
        255,
    )
}

/// Clouds: opaque value-noise fill across the source's bounding box;
/// transparent outside it. An empty surface is an exact copy.
#[must_use]
pub fn clouds(surface: &Surface, seed: u64, sky: Rgba8, cloud: Rgba8) -> Surface {
    let Some((x0, y0, x1, y1)) = alpha_bbox(surface) else {
        return surface.clone();
    };
    let mut out = surface.clone();
    for y in 0..surface.height() {
        for x in 0..surface.width() {
            let at = surface.pixel_offset(x, y);
            if x < x0 || x > x1 || y < y0 || y > y1 {
                out.pixels[at..at + 4].fill(0);
                continue;
            }
            let n = value_noise(i64::from(x), i64::from(y), seed);
            out.set_straight(x, y, texture(sky, cloud, n));
        }
    }
    out
}

/// Difference Clouds: the same texture subtracted per channel inside
/// the source's own silhouette — `out = |src − texture|` on straight
/// colors, alpha untouched. Fully transparent pixels pass through.
#[must_use]
pub fn difference_clouds(surface: &Surface, seed: u64, sky: Rgba8, cloud: Rgba8) -> Surface {
    let mut out = surface.clone();
    for y in 0..surface.height() {
        for x in 0..surface.width() {
            let at = surface.pixel_offset(x, y);
            let p = [
                u16::from(surface.pixels[at]),
                u16::from(surface.pixels[at + 1]),
                u16::from(surface.pixels[at + 2]),
                u16::from(surface.pixels[at + 3]),
            ];
            if p[3] == 0 {
                continue;
            }
            let src = unpremultiply(p);
            let n = value_noise(i64::from(x), i64::from(y), seed);
            let channel = |v: u8, s: u8, c: u8| {
                let t = div_half(i64::from(s) * (255 - n) + i64::from(c) * n, 255);
                (i64::from(v) - t).abs().min(255) as u8
            };
            out.set_straight(
                x,
                y,
                Rgba8::new(
                    channel(src.r, sky.r, cloud.r),
                    channel(src.g, sky.g, cloud.g),
                    channel(src.b, sky.b, cloud.b),
                    src.a,
                ),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// One opaque red pixel at (3, 2) on a 12×12 field.
    fn speck() -> Surface {
        let mut s = Surface::new(12, 12).unwrap();
        s.set_straight(3, 2, Rgba8::new(255, 0, 0, 255));
        s
    }

    #[test]
    fn lattice_origin_is_zero_and_field_interpolates_to_it() {
        // seed 0 at the origin hashes to 0, and (0,0) is a lattice
        // corner, so the field must return exactly it — this pins the
        // whole hash + interpolation chain at one point.
        assert_eq!(lattice(0, 0, 0), 0);
        assert_eq!(value_noise(0, 0, 0), 0);
        // One step along the cell edge: quarter-way between corner
        // values a and b — pinned, not re-derived.
        let a = lattice(7, 0, 0);
        let b = lattice(7, 1, 0);
        assert_eq!(value_noise(4, 0, 7), div_half(a * 12 + b * 4, 16));
    }

    #[test]
    fn clouds_fills_the_bbox_opaquely_and_clears_outside() {
        let s = speck();
        let sky = Rgba8::new(0, 0, 255, 255);
        let cloud = Rgba8::new(255, 255, 255, 255);
        let out = clouds(&s, 42, sky, cloud);
        for y in 0..12u32 {
            for x in 0..12u32 {
                let p = out.get(x, y).unwrap();
                if x == 3 && y == 2 {
                    assert_eq!(p[3], 255, "fill bounds must be opaque");
                    let n = value_noise(3, 2, 42);
                    let want = texture(sky, cloud, n);
                    assert_eq!([p[0], p[1], p[2]], [want.r, want.g, want.b]);
                } else {
                    assert_eq!(p, [0, 0, 0, 0], "outside the box must clear at ({x},{y})");
                }
            }
        }
    }

    #[test]
    fn clouds_texture_stays_between_sky_and_cloud() {
        // A filled block so the bbox spans many lattice cells.
        let mut s = Surface::new(40, 40).unwrap();
        for y in 0..40u32 {
            for x in 0..40u32 {
                s.set_straight(x, y, Rgba8::new(10, 10, 10, 255));
            }
        }
        let sky = Rgba8::new(0, 0, 255, 255);
        let cloud = Rgba8::new(255, 255, 255, 255);
        let out = clouds(&s, 9, sky, cloud);
        let mut varied = false;
        for y in 0..40u32 {
            for x in 0..40u32 {
                let p = out.get(x, y).unwrap();
                assert_eq!(p[3], 255);
                assert!(p[0] >= sky.r && p[0] <= cloud.r);
                assert!(p[2] >= sky.b && p[2] <= cloud.b);
                if p[0] != out.get(0, 0).unwrap()[0] {
                    varied = true;
                }
            }
        }
        assert!(varied, "the texture must not be flat across the box");
    }

    #[test]
    fn clouds_is_deterministic_and_seed_sensitive() {
        let s = speck();
        let a = clouds(
            &s,
            1234,
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(255, 255, 255, 255),
        );
        let b = clouds(
            &s,
            1234,
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(255, 255, 255, 255),
        );
        assert_eq!(a, b, "same seed must replay identically");
        let c = clouds(
            &s,
            1235,
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(255, 255, 255, 255),
        );
        assert_ne!(a, c, "different seeds must diverge");
    }

    #[test]
    fn clouds_with_equal_colors_is_a_flat_fill() {
        let s = speck();
        let gray = Rgba8::new(77, 77, 77, 255);
        let out = clouds(&s, 5, gray, gray);
        assert_eq!(out.get(3, 2).unwrap(), [77, 77, 77, 255]);
    }

    #[test]
    fn clouds_on_an_empty_surface_is_an_exact_copy() {
        let s = Surface::new(6, 6).unwrap();
        assert_eq!(
            clouds(&s, 11, Rgba8::new(0, 0, 0, 255), Rgba8::new(9, 9, 9, 255)),
            s
        );
    }

    #[test]
    fn difference_clouds_subtracts_per_channel_and_keeps_alpha() {
        // Texture == source color → black; the alpha rides through;
        // fully transparent pixels stay untouched.
        let mut s = Surface::new(4, 2).unwrap();
        s.set_straight(0, 0, Rgba8::new(100, 100, 100, 255));
        s.set_straight(1, 0, Rgba8::new(100, 100, 100, 128));
        let gray = Rgba8::new(100, 100, 100, 255);
        let out = difference_clouds(&s, 3, gray, gray);
        assert_eq!(out.get(0, 0).unwrap(), [0, 0, 0, 255]);
        assert_eq!(out.get(1, 0).unwrap(), [0, 0, 0, 128]);
        assert_eq!(out.get(2, 0).unwrap(), [0, 0, 0, 0]);
        // Bounds: the difference of any pixel is within 0..=255 by
        // construction; a saturated case pins the channel math.
        let out2 = difference_clouds(
            &s,
            3,
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(255, 255, 255, 255),
        );
        let p = out2.get(0, 0).unwrap();
        let n = value_noise(0, 0, 3);
        assert_eq!(i64::from(p[0]), (100 - n).abs());
    }

    #[test]
    fn difference_clouds_preserves_dimensions() {
        let s = speck();
        let out = difference_clouds(&s, 8, Rgba8::new(0, 0, 0, 255), Rgba8::new(9, 9, 9, 255));
        assert_eq!((out.width(), out.height()), (12, 12));
    }
}
