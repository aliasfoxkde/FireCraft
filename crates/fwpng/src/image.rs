//! Standard PNG image path (the flattened preview every viewer shows),
//! decoded with the [`png`] crate and normalized to RGBA8.
//!
//! The Fireworks private chunks are parsed separately; this module only
//! handles the ordinary image stream so callers can display a document
//! without understanding the node tree.

use std::io::Cursor;

use crate::error::FwPngError;

/// A decoded standard PNG image as tightly packed RGBA8.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandardImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, straight (un-premultiplied) alpha.
    pub rgba8: Vec<u8>,
}

/// Decodes the standard PNG stream into RGBA8. Palette, low-bit-depth
/// gray, and 16-bit inputs are normalized by the decoder; the remaining
/// four 8-bit layouts are expanded here.
///
/// # Errors
/// [`FwPngError::PngDecode`] when the `png` crate rejects the stream,
/// [`FwPngError::UnsupportedImage`] for layouts that survive
/// normalization but have no RGBA8 mapping here.
pub fn decode_standard_rgba8(input: &[u8]) -> Result<StandardImage, FwPngError> {
    let mut decoder = png::Decoder::new(Cursor::new(input));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(png_decode_err)?;
    let size = reader
        .output_buffer_size()
        .ok_or(FwPngError::UnsupportedImage {
            layout: "decoder reported no output size".to_string(),
        })?;
    let mut buf = vec![0u8; size];
    let info = reader.next_frame(&mut buf).map_err(png_decode_err)?;
    buf.truncate(info.buffer_size());

    let rgba8 = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => expand_rgb(&buf),
        png::ColorType::Grayscale => expand_gray(&buf),
        png::ColorType::GrayscaleAlpha => expand_gray_alpha(&buf),
        other => {
            return Err(FwPngError::UnsupportedImage {
                layout: format!("{other:?} after normalization (unexpected)"),
            });
        }
    };
    Ok(StandardImage {
        width: info.width,
        height: info.height,
        rgba8,
    })
}

fn png_decode_err(err: png::DecodingError) -> FwPngError {
    FwPngError::PngDecode {
        reason: err.to_string(),
    }
}

/// Encodes straight-alpha RGBA8 pixels as a standard PNG (the write
/// side of [`decode_standard_rgba8`]): 8-bit RGBA, no interlace — the
/// export path for rasterized pages.
///
/// # Errors
/// [`FwPngError::PngEncode`] when the `png` crate rejects the write
/// (buffer length mismatch, stream failure).
pub fn encode_rgba8(width: u32, height: u32, rgba8: &[u8]) -> Result<Vec<u8>, FwPngError> {
    let expected = width as usize * height as usize * 4;
    if rgba8.len() != expected {
        return Err(FwPngError::UnsupportedImage {
            layout: format!(
                "encode: {} bytes for {width}x{height} (want {expected})",
                rgba8.len()
            ),
        });
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|err| FwPngError::PngEncode {
                reason: err.to_string(),
            })?;
        writer
            .write_image_data(rgba8)
            .map_err(|err| FwPngError::PngEncode {
                reason: err.to_string(),
            })?;
    }
    Ok(out)
}

/// RGB (3 bytes/px) -> RGBA (4 bytes/px) with opaque alpha.
fn expand_rgb(buf: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len() / 3 * 4);
    for px in buf.chunks_exact(3) {
        out.extend_from_slice(px);
        out.push(255);
    }
    out
}

/// 8-bit gray -> RGBA with the gray value in every channel.
fn expand_gray(buf: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len() * 4);
    for &g in buf {
        out.extend_from_slice(&[g, g, g, 255]);
    }
    out
}

/// Gray+alpha (2 bytes/px) -> RGBA.
fn expand_gray_alpha(buf: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len() / 2 * 4);
    for px in buf.chunks_exact(2) {
        out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// Encodes RGBA8 pixels through the png crate (write side only used
    /// to build test inputs).
    fn encode(width: u32, height: u32, color: png::ColorType, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, width, height);
            encoder.set_color(color);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(bytes).expect("pixels");
        }
        out
    }

    #[test]
    fn decodes_rgba_round_trip() {
        let pixels = [1, 2, 3, 4, 5, 6, 7, 8];
        let img =
            decode_standard_rgba8(&encode(2, 1, png::ColorType::Rgba, &pixels)).expect("decodes");
        assert_eq!(img.width, 2);
        assert_eq!(img.height, 1);
        assert_eq!(img.rgba8, pixels);
    }

    #[test]
    fn expands_rgb_to_opaque_rgba() {
        let img =
            decode_standard_rgba8(&encode(1, 1, png::ColorType::Rgb, &[9, 8, 7])).expect("decodes");
        assert_eq!(img.rgba8, vec![9, 8, 7, 255]);
    }

    #[test]
    fn expands_grayscale() {
        let img = decode_standard_rgba8(&encode(1, 2, png::ColorType::Grayscale, &[0, 255]))
            .expect("decodes");
        assert_eq!(img.rgba8, vec![0, 0, 0, 255, 255, 255, 255, 255]);
    }

    #[test]
    fn expands_gray_alpha() {
        let img = decode_standard_rgba8(&encode(1, 1, png::ColorType::GrayscaleAlpha, &[10, 128]))
            .expect("decodes");
        assert_eq!(img.rgba8, vec![10, 10, 10, 128]);
    }

    #[test]
    fn downsamples_sixteen_bit() {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, 1, 1);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Sixteen);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(&[0x12, 0x34]).expect("pixels");
        }
        let img = decode_standard_rgba8(&out).expect("decodes");
        assert_eq!(img.rgba8, vec![0x12, 0x12, 0x12, 255]);
    }

    #[test]
    fn garbage_errors_cleanly() {
        assert!(decode_standard_rgba8(b"not a png").is_err());
    }

    #[test]
    fn encode_round_trips_through_decode() {
        let pixels = [10_u8, 20, 30, 40, 50, 60, 70, 80];
        let encoded = super::encode_rgba8(2, 1, &pixels).expect("encodes");
        let decoded = decode_standard_rgba8(&encoded).expect("decodes");
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.rgba8, pixels);
    }

    #[test]
    fn encode_rejects_a_short_buffer() {
        assert!(super::encode_rgba8(2, 1, &[1, 2, 3]).is_err());
    }
}
