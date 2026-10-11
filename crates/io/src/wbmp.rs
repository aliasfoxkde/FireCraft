//! WBMP (Wireless Bitmap, WMT type 0) export + import, hand-rolled —
//! the whole format is a two-byte type header, two multi-byte
//! dimensions, and 1-bit packed rows.
//!
//! Bit semantics: **0 = black, 1 = white** (WAP-190 WML spec, public).
//! Multi-byte integers carry 7 bits per octet, least-significant
//! group first, with bit 8 (0x80) as the continuation flag.
//!
//! Threshold policy: ITU-R 601 luma `(77r + 151g + 28b + 127)/255`
//! over the matte, `>= 128` → white. Import decodes to opaque
//! black/white RGBA8.

use crate::{ExportError, ImportError, Matte, Plane};

/// Header octets before the dimensions (type 0, fixed header 0x00).
const TYPE_HEADER: [u8; 2] = [0x00, 0x00];

/// Defensive ceiling on decoded planes: 100M pixels (400 MB RGBA).
/// Anything larger is refused, not allocated.
const MAX_PIXELS: u64 = 100_000_000;

/// Encodes a plane as WBMP type 0, thresholding over `matte`.
///
/// # Errors
/// [`ExportError::Encode`] on arithmetic overflow.
pub fn encode(plane: &Plane<'_>, matte: Matte) -> Result<Vec<u8>, ExportError> {
    let row_bytes = usize::try_from(plane.width.div_ceil(8))
        .map_err(|_| overflow())?
        .max(1);
    let pixel_bytes = row_bytes
        .checked_mul(plane.height as usize)
        .ok_or_else(overflow)?;

    let mut out = Vec::with_capacity(2 + 10 + pixel_bytes);
    out.extend_from_slice(&TYPE_HEADER);
    push_varint(&mut out, u64::from(plane.width));
    push_varint(&mut out, u64::from(plane.height));

    for y in 0..plane.height {
        let mut row = vec![0u8; row_bytes];
        for x in 0..plane.width {
            if plane.luma_at(x, y, matte) >= 128 {
                let byte = x as usize / 8;
                row[byte] |= 0x80 >> (x % 8);
            }
        }
        out.extend_from_slice(&row);
    }
    Ok(out)
}

/// Decodes a WBMP type 0 stream to opaque RGBA8 (white = 255, black
/// = 0; alpha stays 255 — the format has none).
///
/// # Errors
/// [`ImportError::Truncated`] on short headers/rows,
/// [`ImportError::Unsupported`] for non-zero type/fixed-header fields
/// and oversized dimensions,
/// [`ImportError::Decode`] for trailing garbage or misaligned data.
pub fn decode(input: &[u8]) -> Result<(u32, u32, Vec<u8>), ImportError> {
    const FORMAT: &str = "wbmp";
    if input.len() < 2 {
        return Err(ImportError::Truncated {
            format: FORMAT,
            needed: "type + fixed header octets",
        });
    }
    if input[0] != 0 {
        return Err(ImportError::Unsupported {
            format: FORMAT,
            reason: format!("only WMT type 0 is supported, got octet {}", input[0]),
        });
    }
    // Bit 5 (0x20) of the fixed header announces an extension header.
    if input[1] & 0x20 != 0 {
        return Err(ImportError::Unsupported {
            format: FORMAT,
            reason: "extended headers are not supported".to_owned(),
        });
    }
    if input[1] != 0 {
        return Err(ImportError::Decode {
            format: FORMAT,
            reason: format!("fixed header must be 0x00, got {:#04x}", input[1]),
        });
    }

    let mut cursor = 2usize;
    let width = read_varint(input, &mut cursor)?;
    let height = read_varint(input, &mut cursor)?;
    if width == 0 || height == 0 {
        return Err(ImportError::Decode {
            format: FORMAT,
            reason: format!("degenerate dimensions {width}x{height}"),
        });
    }
    if width
        .checked_mul(height)
        .is_none_or(|pixels| pixels > MAX_PIXELS)
    {
        return Err(ImportError::Unsupported {
            format: FORMAT,
            reason: format!(
                "{}x{} exceeds the {}-pixel ceiling",
                width, height, MAX_PIXELS
            ),
        });
    }
    let (width, height) = (
        u32::try_from(width).map_err(|_| overflow_import())?,
        height as u32,
    );

    let row_bytes = width.div_ceil(8) as usize;
    let needed = row_bytes
        .checked_mul(height as usize)
        .ok_or_else(overflow_import)?;
    if input.len() < cursor + needed {
        return Err(ImportError::Truncated {
            format: FORMAT,
            needed: "complete pixel array",
        });
    }

    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height as usize {
        let row = &input[cursor + y * row_bytes..cursor + (y + 1) * row_bytes];
        for x in 0..width as usize {
            let bit = row[x / 8] & (0x80 >> (x % 8)) != 0;
            let v = if bit { 255u8 } else { 0 };
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    }
    Ok((width, height, rgba))
}

fn push_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut octet = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            octet |= 0x80;
        }
        out.push(octet);
        if value == 0 {
            break;
        }
    }
}

fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u64, ImportError> {
    let mut value: u64 = 0;
    for shift in (0..64).step_by(7) {
        let Some(&octet) = input.get(*cursor) else {
            return Err(ImportError::Truncated {
                format: "wbmp",
                needed: "complete multi-byte integer",
            });
        };
        *cursor += 1;
        value |= u64::from(octet & 0x7F) << shift;
        if octet & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(ImportError::Decode {
        format: "wbmp",
        reason: "multi-byte integer exceeds 64 bits".to_owned(),
    })
}

fn overflow() -> ExportError {
    ExportError::Encode {
        format: "wbmp",
        reason: "pixel array overflows usize".to_owned(),
    }
}

fn overflow_import() -> ImportError {
    ImportError::Decode {
        format: "wbmp",
        reason: "dimension does not fit the platform word".to_owned(),
    }
}
