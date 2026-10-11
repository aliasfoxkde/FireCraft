//! TIFF export via `tiff` 0.11 (the codec behind the `image` crate's
//! TIFF support, used directly so compression is selectable).
//!
//! Always RGBA8 (`PhotometricInterpretation` RGB with extra sample =
//! associated alpha is what every mainstream reader accepts for
//! straight RGBA). Compression: uncompressed or PackBits (the CS6
//! save dialog's two options for the format's lossless family; LZW
//! and Deflate are recorded as future options in the decision note).

use std::io::Cursor;

use tiff::encoder::colortype::RGBA8;
use tiff::encoder::{Compression, TiffEncoder};

use crate::{ExportError, Plane};

/// Lossless TIFF row compression.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TiffCompression {
    /// No compression (default — maximum reader compatibility).
    #[default]
    None,
    /// PackBits run-length coding (the classic Mac-era TIFF default).
    Packbits,
}

/// TIFF encoder options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TiffOptions {
    /// Row compression.
    pub compression: TiffCompression,
}

/// Encodes a plane as RGBA8 TIFF.
///
/// # Errors
/// [`ExportError::Encode`] when the encoder fails at any stage.
pub fn encode(plane: &Plane<'_>, options: &TiffOptions) -> Result<Vec<u8>, ExportError> {
    let mut buffer = Vec::new();
    {
        // `TiffEncoder` needs `Write + Seek`; buffer through a cursor.
        let encoder = TiffEncoder::new(Cursor::new(&mut buffer))
            .map_err(|error| encode_err("writer setup", error))?;
        let mut encoder = match options.compression {
            TiffCompression::None => encoder,
            TiffCompression::Packbits => encoder.with_compression(Compression::Packbits),
        };
        // `write_image` finishes its image directory; the encoder's
        // Drop then writes the (already complete) IFD chain.
        encoder
            .write_image::<RGBA8>(plane.width, plane.height, plane.rgba)
            .map_err(|error| encode_err("pixel data", error))?;
    }
    Ok(buffer)
}

fn encode_err(stage: &str, error: tiff::TiffError) -> ExportError {
    ExportError::Encode {
        format: "tiff",
        reason: format!("{stage}: {error}"),
    }
}
