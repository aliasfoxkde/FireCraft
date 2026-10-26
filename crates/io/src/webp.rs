//! WebP export: lossless VP8L through the `image` crate's pure-Rust
//! encoder (`image-webp`). Lossy VP8 via `libwebp-sys` behind the
//! `lossy-webp` feature gate (native only; wasm32 cannot enable it).
//!
//! The export is the honest lossless round-trip: pixels in, the same
//! pixels out (verified by the round-trip test against the import decoder
//! in [`crate::raster`]).

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

/// Encodes a plane as lossy WebP (VP8) via `libwebp-sys`.
///
/// Only available when the `lossy-webp` feature is enabled (native builds).
/// WASM builds must not enable this feature.
///
/// # Errors
/// [`ExportError::Encode`] when libwebp refuses the input.
#[cfg(feature = "lossy-webp")]
pub fn encode_lossy(plane: &Plane<'_>, quality: f32) -> Result<Vec<u8>, ExportError> {
    crate::webp_lossy::encode(plane, quality)
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

    #[test]
    fn encode_rejects_dimension_length_mismatch() {
        // A plane whose buffer is shorter than width*height*4 refuses.
        let err = encode(&Plane {
            width: 4,
            height: 4,
            rgba: &[0u8; 8],
        })
        .unwrap_err();
        assert!(matches!(err, ExportError::Encode { format: "webp", .. }));
    }
}
