//! PNG export, wrapping [`fwpng`]'s standard-image encoder.
//!
//! `fwpng` stays the single PNG implementation in the workspace (its
//! layer gate keeps it standalone); `io` only re-exposes the ordinary
//! image path for format dispatch.

use crate::{ExportError, Plane};

/// PNG encoder options (`EXPv` mapped).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PngOptions {
    /// `GFIb` — Adam7 interlaced output. The `png` crate writes the
    /// IHDR flag but cannot emit the seven passes, so requesting it is
    /// refused loudly rather than producing a corrupt stream.
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
/// [`ExportError::Unsupported`] for interlaced output (no Adam7 pass
/// encoder), [`ExportError::Encode`] when the underlying encoder
/// fails.
pub fn encode_with(plane: &Plane<'_>, options: &PngOptions) -> Result<Vec<u8>, ExportError> {
    if options.interlaced {
        return Err(ExportError::Unsupported {
            format: "png",
            reason: "interlaced output needs an Adam7 pass encoder (not implemented)".to_owned(),
        });
    }
    fwpng::image::encode_rgba8(plane.width, plane.height, plane.rgba).map_err(|error| {
        ExportError::Encode {
            format: "png",
            reason: error.to_string(),
        }
    })
}
