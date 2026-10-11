//! PNG export, wrapping [`fwpng`]'s standard-image encoder.
//!
//! `fwpng` stays the single PNG implementation in the workspace (its
//! layer gate keeps it standalone); `io` only re-exposes the ordinary
//! image path for format dispatch. Bit-depth / interlace options land
//! with the `EXPv` optimization-settings task (M8 task 8).

use crate::{ExportError, Plane};

/// Encodes a straight-alpha RGBA8 plane as PNG.
///
/// # Errors
/// [`ExportError::Encode`] when the underlying encoder fails.
pub fn encode(plane: &Plane<'_>) -> Result<Vec<u8>, ExportError> {
    fwpng::image::encode_rgba8(plane.width, plane.height, plane.rgba).map_err(|error| {
        ExportError::Encode {
            format: "png",
            reason: error.to_string(),
        }
    })
}
