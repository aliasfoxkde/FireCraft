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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn encode_produces_nonempty_webp() {
        let rgba = vec![255u8, 0, 0, 255]; // opaque red
        let plane = Plane::new(1, 1, &rgba).unwrap();
        let out = encode(&plane).unwrap();
        assert!(!out.is_empty());
        // RIFF header + WebP signature (image crate lowercases to "WEBP").
        assert_eq!(&out[0..4], b"RIFF");
        assert_eq!(&out[8..12], b"WEBP");
    }

    #[test]
    fn encode_rejects_mismatched_plane() {
        // Plane says 2×2 (16 bytes) but only 4 bytes provided → Plane::new fails.
        let result = Plane::new(2, 2, &[0u8; 4]);
        assert!(result.is_err());
    }
}
