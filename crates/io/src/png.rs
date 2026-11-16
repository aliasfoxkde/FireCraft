//! PNG export, wrapping [`fwpng`]'s standard-image encoder.
//!
//! `fwpng` stays the single PNG implementation in the workspace (its
//! layer gate keeps it standalone); `io` only re-exposes the ordinary
//! image path for format dispatch.

use crate::{ExportError, Plane};

/// PNG encoder options (`EXPv` mapped).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PngOptions {
    /// `GFIb` — Adam7 interlaced output. The interlace flag is written
    /// into the IHDR and the seven passes are emitted, so the file
    /// decodes to the same pixels as the non-interlaced write.
    pub interlaced: bool,
}

/// Encodes a straight-alpha RGBA8 plane as PNG with default options.
///
/// # Errors
/// [`ExportError::Encode`] when the underlying encoder fails.
pub fn encode(plane: &Plane<'_>) -> Result<Vec<u8>, ExportError> {
    encode_with(plane, &PngOptions::default())
}

/// Encodes a straight-alpha RGBA8 plane as PNG with explicit options.
///
/// # Errors
/// [`ExportError::Encode`] when the underlying encoder fails.
pub fn encode_with(plane: &Plane<'_>, options: &PngOptions) -> Result<Vec<u8>, ExportError> {
    let encoded = if options.interlaced {
        fwpng::image::encode_rgba8_interlaced(plane.width, plane.height, plane.rgba)
    } else {
        fwpng::image::encode_rgba8(plane.width, plane.height, plane.rgba)
    };
    encoded.map_err(|error| ExportError::Encode {
        format: "png",
        reason: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn png_options_default_is_not_interlaced() {
        let opts = PngOptions::default();
        assert!(!opts.interlaced);
    }

    #[test]
    fn encode_produces_nonempty_png() {
        let rgba = vec![0u8; 4]; // 1×1 black
        let plane = Plane::new(1, 1, &rgba).unwrap();
        let out = encode(&plane).unwrap();
        assert!(!out.is_empty());
        // PNG signature.
        assert_eq!(&out[0..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn encode_with_writes_the_interlaced_passes() {
        let rgba = vec![0u8; 4];
        let plane = Plane::new(1, 1, &rgba).unwrap();
        let opts = PngOptions { interlaced: true };
        let out = encode_with(&plane, &opts).expect("interlaced output encodes");
        assert_eq!(&out[0..8], b"\x89PNG\r\n\x1a\n");
        // The IHDR's interlace byte is the last of its thirteen.
        let ihdr = out
            .windows(4)
            .position(|window| window == b"IHDR")
            .expect("the file carries an IHDR");
        assert_eq!(out[ihdr + 4 + 12], 1, "the interlace method is Adam7");
    }
}
