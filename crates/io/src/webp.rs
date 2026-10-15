//! WebP export: lossless VP8L through the `image` crate's pure-Rust
//! encoder (`image-webp`). Lossy VP8 needs libwebp and stays out of
//! scope — the export is the honest lossless round-trip: pixels in,
//! the same pixels out (verified by the round-trip test against the
//! import decoder in [`crate::raster`]).

use crate::{ExportError, Plane};

/// Encodes a plane as a lossless WebP (VP8L), alpha preserved.
///
/// # Errors
/// [`ExportError::Encode`] when the encoder refuses the plane.
pub fn encode(plane: &Plane<'_>) -> Result<Vec<u8>, ExportError> {
    let image = image::RgbaImage::from_raw(plane.width, plane.height, plane.rgba.to_vec())
        .ok_or_else(|| ExportError::Encode {
            format: "webp",
            reason: format!(
                "plane {}x{} does not hold its pixels",
                plane.width, plane.height
            ),
        })?;
    let mut out = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut out),
            image::ImageFormat::WebP,
        )
        .map_err(|error| ExportError::Encode {
            format: "webp",
            reason: error.to_string(),
        })?;
    Ok(out)
}
