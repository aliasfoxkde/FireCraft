//! Deterministic CPU image kernels (workspace layer 2).
//!
//! The catalog the M7 ticket pins, extended in M14 to the full CS6
//! live-filter surface: point ops (invert, brightness/contrast,
//! levels, curves, posterize, threshold, HSL, auto levels, color
//! fill, convert to alpha), convolutions (box/gaussian blur, unsharp
//! with threshold, sharpen presets, find edges, emboss), geometric
//! resampling blurs (motion, zoom, radial spin), and seeded noise.
//! Every kernel is a pure `&Surface -> Surface` function: same input
//! bytes, same output bytes, every run — journal replay and the
//! fuzzers depend on that.
//!
//! Buffers here are **premultiplied** RGBA8 (compose's native form), so
//! a kernel result can flow back into the compositor without a second
//! conversion. Color-order rules live in
//! `docs/decisions/0004-filter-color-order.md`; the short version:
//! convolutions run on premultiplied channels directly, point ops
//! round-trip each pixel through [`color::unpremultiply`] first.

mod convolve;
mod geometric;
mod noise;
mod point;
mod shadow;

pub use convolve::{
    box_blur, emboss, find_edges, gaussian_blur, sharpen, sharpen_more, unsharp_mask,
    unsharp_mask_thresholded,
};
pub use geometric::{motion_blur, radial_blur_spin, zoom_blur};
pub use noise::{SplitMix64, add_noise};
pub use point::{
    CurvesKnots, apply_lut, auto_levels, brightness_contrast, convert_to_alpha, curves, hsl,
    invert, levels, posterize, solid_fill, threshold,
};
pub use shadow::{drop_shadow, inner_glow, inner_shadow, outer_glow, solid_shadow};

use color::Rgba8;
use color::pixel::{premultiply, unpremultiply};

/// A whole-image RGBA8 buffer, premultiplied, row-major.
///
/// Kernels need whole-image neighborhoods (a blur reads across tile
/// boundaries), so this is deliberately *not* [`raster`]'s tiled page
/// store: `compose` bridges `Render` ↔ `Surface` at the layer-3
/// boundary. `raster` is referenced by name only — no dependency.
///
/// [`raster`]: ../raster/index.html
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

/// Why a `Surface` could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceError {
    /// A zero-sized image is never valid: the compositor cannot place it.
    ZeroDimension,
    /// `width * height * 4` does not fit the platform `usize`.
    TooLarge { width: u32, height: u32 },
    /// The pixel slice is not `width * height * 4` bytes.
    PixelCountMismatch { expected: usize, actual: usize },
}

impl std::fmt::Display for SurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SurfaceError::ZeroDimension => write!(f, "surface needs a non-zero size"),
            SurfaceError::TooLarge { width, height } => {
                write!(f, "surface {width}x{height} exceeds addressable pixels")
            }
            SurfaceError::PixelCountMismatch { expected, actual } => {
                write!(f, "pixel buffer holds {actual} bytes, expected {expected}")
            }
        }
    }
}

impl std::error::Error for SurfaceError {}

impl Surface {
    /// A transparent-black surface of the given size.
    pub fn new(width: u32, height: u32) -> Result<Self, SurfaceError> {
        let len = pixel_len(width, height)?;
        Ok(Self {
            width,
            height,
            pixels: vec![0; len],
        })
    }

    /// Adopts a premultiplied buffer (e.g. a `compose::Render`'s pixels).
    ///
    /// # Errors
    /// [`SurfaceError`] when the size is zero or the buffer length is
    /// not `width * height * 4`.
    pub fn from_premultiplied(
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Result<Self, SurfaceError> {
        let expected = pixel_len(width, height)?;
        if pixels.len() != expected {
            return Err(SurfaceError::PixelCountMismatch {
                expected,
                actual: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Converts from a straight-alpha RGBA8 buffer.
    ///
    /// # Errors
    /// [`SurfaceError`] when the size is zero or the slice length is
    /// not `width * height * 4`.
    pub fn from_straight(width: u32, height: u32, pixels: &[u8]) -> Result<Self, SurfaceError> {
        let expected = pixel_len(width, height)?;
        if pixels.len() != expected {
            return Err(SurfaceError::PixelCountMismatch {
                expected,
                actual: pixels.len(),
            });
        }
        let mut converted = vec![0u8; expected];
        for (src, dst) in pixels.chunks_exact(4).zip(converted.chunks_exact_mut(4)) {
            let px = premultiply(Rgba8::new(src[0], src[1], src[2], src[3]));
            dst.copy_from_slice(&[px[0] as u8, px[1] as u8, px[2] as u8, px[3] as u8]);
        }
        Ok(Self {
            width,
            height,
            pixels: converted,
        })
    }

    /// Straight-alpha RGBA8 copy (PNG-encode and egui shapes want this).
    #[must_use]
    pub fn to_straight(&self) -> Vec<u8> {
        let mut out = vec![0u8; self.pixels.len()];
        for (src, dst) in self.pixels.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
            let px = unpremultiply([
                u16::from(src[0]),
                u16::from(src[1]),
                u16::from(src[2]),
                u16::from(src[3]),
            ]);
            dst.copy_from_slice(&[px.r, px.g, px.b, px.a]);
        }
        out
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The premultiplied backing store, row-major RGBA8.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Consumes the surface, yielding the premultiplied backing store.
    #[must_use]
    pub fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }

    /// The premultiplied pixel at `(x, y)`, or `None` out of bounds.
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = pixel_offset(x, y, self.width);
        Some([
            self.pixels[at],
            self.pixels[at + 1],
            self.pixels[at + 2],
            self.pixels[at + 3],
        ])
    }

    /// Overwrites the pixel at `(x, y)` with a straight-alpha color.
    /// Out-of-bounds writes are ignored (kernels clamp coordinates and
    /// never need to detect the miss).
    pub fn set_straight(&mut self, x: u32, y: u32, px: Rgba8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let p = premultiply(px);
        let at = pixel_offset(x, y, self.width);
        self.pixels[at..at + 4].copy_from_slice(&[p[0] as u8, p[1] as u8, p[2] as u8, p[3] as u8]);
    }

    /// Runs `f` over every pixel's **straight** RGBA and stores the
    /// result. This is the point-op primitive: premultiply order is
    /// decided here once, so individual ops cannot get it wrong
    /// (decision 0004). Alpha may be changed by `f`; most ops pass it
    /// through.
    pub(crate) fn map_straight(&self, mut f: impl FnMut(Rgba8) -> Rgba8) -> Surface {
        let mut out = self.clone();
        for px in out.pixels.chunks_exact_mut(4) {
            let straight = unpremultiply([
                u16::from(px[0]),
                u16::from(px[1]),
                u16::from(px[2]),
                u16::from(px[3]),
            ]);
            let next = premultiply(f(straight));
            px.copy_from_slice(&[next[0] as u8, next[1] as u8, next[2] as u8, next[3] as u8]);
        }
        out
    }

    /// Channel index of `(x, y)` in a row-major RGBA8 buffer.
    pub(crate) fn pixel_offset(&self, x: u32, y: u32) -> usize {
        pixel_offset(x, y, self.width)
    }
}

/// `width * height * 4`, refusing zero sizes and overflow.
fn pixel_len(width: u32, height: u32) -> Result<usize, SurfaceError> {
    if width == 0 || height == 0 {
        return Err(SurfaceError::ZeroDimension);
    }
    // The square of u32::MAX fits u64; the ×4 does not — hence checked.
    let area = u64::from(width) * u64::from(height);
    let len = area
        .checked_mul(4)
        .ok_or(SurfaceError::TooLarge { width, height })?;
    usize::try_from(len).map_err(|_| SurfaceError::TooLarge { width, height })
}

/// Row-major RGBA8 offset of `(x, y)` at row stride `width * 4`.
fn pixel_offset(x: u32, y: u32, width: u32) -> usize {
    (y as usize * width as usize + x as usize) * 4
}

/// House rounding: add-half for non-negative numerators, symmetric
/// half-away-from-zero for signed ones. All kernels divide through
/// this so rounding is pinned in exactly one place.
pub(crate) fn div_half(numer: i64, denom: i64) -> i64 {
    debug_assert!(denom > 0);
    if numer >= 0 {
        (numer + denom / 2) / denom
    } else {
        -((-numer + denom / 2) / denom)
    }
}

/// Saturating `i64 -> u8`.
pub(crate) fn clamp255(v: i64) -> u8 {
    v.clamp(0, 255) as u8
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn surface_round_trips_straight_pixels() {
        let mut s = Surface::from_straight(2, 1, &[255, 0, 0, 255, 0, 0, 0, 128]).unwrap();
        assert_eq!(s.get(0, 0), Some([255, 0, 0, 255]));
        // Opaque premultiply is exact, so the straight copy is too.
        assert_eq!(s.to_straight(), vec![255, 0, 0, 255, 0, 0, 0, 128]);
        // A translucent write re-premultiplies on store.
        s.set_straight(1, 0, Rgba8::new(255, 255, 255, 128));
        let stored = s.get(1, 0).unwrap();
        assert!(stored[0] <= 128, "premultiplied red must not exceed alpha");
    }

    #[test]
    fn surface_rejects_bad_shapes() {
        assert_eq!(Surface::new(0, 8).unwrap_err(), SurfaceError::ZeroDimension);
        assert_eq!(
            Surface::from_straight(2, 2, &[0; 12]).unwrap_err(),
            SurfaceError::PixelCountMismatch {
                expected: 16,
                actual: 12
            }
        );
        assert!(matches!(
            Surface::new(u32::MAX, u32::MAX).unwrap_err(),
            SurfaceError::TooLarge { .. }
        ));
    }

    #[test]
    fn div_half_rounds_as_documented() {
        assert_eq!(div_half(5, 10), 1); // 0.5 rounds away from zero
        assert_eq!(div_half(-5, 10), -1); // symmetric on the negative side
        assert_eq!(div_half(4, 10), 0);
        assert_eq!(div_half(-4, 10), 0);
        // Odd denominators use the floor of half (the /1000 convention).
        assert_eq!(div_half(1500, 1000), 2);
        assert_eq!(div_half(1499, 1000), 1);
    }

    #[test]
    fn map_straight_sees_unpremultiplied_colors() {
        // A half-alpha red stores as ~(127, 0, 0, 128); the mapped color
        // must be seen as (255, 0, 0, 128), not the stored premul form.
        let s = Surface::from_straight(1, 1, &[255, 0, 0, 128]).unwrap();
        let seen = std::cell::Cell::new(Rgba8::TRANSPARENT);
        let _ = s.map_straight(|px| {
            seen.set(px);
            px
        });
        let seen = seen.into_inner();
        assert_eq!((seen.r, seen.g, seen.b, seen.a), (255, 0, 0, 128));
    }
}
