//! BMP export, hand-rolled (the format is two fixed headers over
//! bottom-up rows; a dependency would not buy correctness).
//!
//! - 24-bit: `BITMAPFILEHEADER` + `BITMAPINFOHEADER` (`BI_RGB`), BGR
//!   rows bottom-up, rows padded to 4 bytes, alpha flattened over the
//!   matte.
//! - 32-bit: `BITMAPFILEHEADER` + `BITMAPV4HEADER` (`BI_BITFIELDS`
//!   with explicit RGBA masks and the `Win ` color space) so alpha
//!   survives; BGRA rows bottom-up (already 4-byte aligned).
//!
//! Reference: Microsoft Windows BMP format (public specification);
//! headers written field-by-field, little-endian.

use crate::{ExportError, Matte, Plane};

/// Pixel layout of the BMP.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BmpDepth {
    /// Opaque 24-bit BGR (`BI_RGB`); alpha is flattened over the matte.
    #[default]
    Bpp24,
    /// 32-bit BGRA (`BI_BITFIELDS` + V4 header) carrying alpha.
    Bpp32,
}

/// BMP encoder options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BmpOptions {
    /// Pixel layout.
    pub depth: BmpDepth,
    /// Matte for 24-bit output.
    pub matte: Matte,
}

/// Encodes a plane as BMP.
///
/// # Errors
/// [`ExportError::TooLarge`] above 2147483647 per axis (the signed
/// header field), [`ExportError::Encode`] on arithmetic overflow.
pub fn encode(plane: &Plane<'_>, options: &BmpOptions) -> Result<Vec<u8>, ExportError> {
    // i32 fields in the info header; keep a defensive ceiling.
    const LIMIT: u32 = i32::MAX as u32;
    if plane.width > LIMIT || plane.height > LIMIT {
        return Err(ExportError::TooLarge {
            format: "bmp",
            width: plane.width,
            height: plane.height,
            limit: LIMIT,
        });
    }

    let (header_len, bpp) = match options.depth {
        BmpDepth::Bpp24 => (40usize, 24u16),
        BmpDepth::Bpp32 => (108, 32),
    };
    let row_bytes = match options.depth {
        BmpDepth::Bpp24 => plane.width as usize * 3,
        BmpDepth::Bpp32 => plane.width as usize * 4,
    };
    let pad = (4 - row_bytes % 4) % 4;
    let stride = row_bytes + pad;
    let pixel_bytes = stride
        .checked_mul(plane.height as usize)
        .ok_or(ExportError::Encode {
            format: "bmp",
            reason: "pixel array overflows usize".to_owned(),
        })?;
    let data_offset = 14 + header_len;
    let file_len = data_offset + pixel_bytes;

    let mut out = Vec::with_capacity(file_len);
    // BITMAPFILEHEADER
    out.extend_from_slice(b"BM");
    push_u32(&mut out, file_len as u32);
    out.extend_from_slice(&[0; 4]); // reserved
    push_u32(&mut out, data_offset as u32);
    // Info header
    push_u32(&mut out, header_len as u32);
    push_i32(&mut out, plane.width as i32);
    push_i32(&mut out, plane.height as i32); // positive: bottom-up
    push_u16(&mut out, 1); // planes
    push_u16(&mut out, bpp);
    match options.depth {
        BmpDepth::Bpp24 => push_u32(&mut out, 0), // BI_RGB
        BmpDepth::Bpp32 => push_u32(&mut out, 3), // BI_BITFIELDS
    }
    push_u32(&mut out, pixel_bytes as u32);
    push_i32(&mut out, 2835); // 72 dpi
    push_i32(&mut out, 2835);
    push_u32(&mut out, 0); // palette entries
    push_u32(&mut out, 0); // important colors
    if options.depth == BmpDepth::Bpp32 {
        push_u32(&mut out, 0x00FF_0000); // red mask
        push_u32(&mut out, 0x0000_FF00); // green mask
        push_u32(&mut out, 0x0000_00FF); // blue mask
        push_u32(&mut out, 0xFF00_0000); // alpha mask
        push_u32(&mut out, 0x5769_6E20); // CSType 'Win '
        out.extend_from_slice(&[0; 36]); // CIEXYZTRIPLE endpoints
        out.extend_from_slice(&[0; 12]); // gamma r/g/b
    }

    // Rows bottom-up, BGRA/BGR.
    for y in (0..plane.height).rev() {
        for x in 0..plane.width {
            let px = plane.pixel(x, y);
            match options.depth {
                BmpDepth::Bpp24 => {
                    out.push(crate::flatten_channel(
                        px[2],
                        px[3],
                        options.matte.blue_channel(),
                    ));
                    out.push(crate::flatten_channel(
                        px[1],
                        px[3],
                        options.matte.green_channel(),
                    ));
                    out.push(crate::flatten_channel(
                        px[0],
                        px[3],
                        options.matte.red_channel(),
                    ));
                }
                BmpDepth::Bpp32 => {
                    out.push(px[2]);
                    out.push(px[1]);
                    out.push(px[0]);
                    out.push(px[3]);
                }
            }
        }
        out.extend(std::iter::repeat_n(0, pad));
    }
    Ok(out)
}

fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn push_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}
