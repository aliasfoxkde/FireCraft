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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    // ── helpers ───────────────────────────────────────────────────────────────

    /// White 2×2 RGBA plane with all alpha = 255.
    fn white_plane() -> Plane<'static> {
        Plane::new(2, 2, &[255u8; 16]).unwrap()
    }

    /// Constructs a WBMP byte stream manually so we can test decode on
    /// well-formed and malformed inputs without going through `encode`.
    fn wbmp_bytes(type_octet: u8, fixed: u8, width: u64, height: u64) -> Vec<u8> {
        let mut b = vec![type_octet, fixed];
        let mut w = width;
        loop {
            let octet = (w & 0x7F) as u8 | if w > 0x7F { 0x80 } else { 0 };
            b.push(octet);
            w >>= 7;
            if w == 0 {
                break;
            }
        }
        let mut h = height;
        loop {
            let octet = (h & 0x7F) as u8 | if h > 0x7F { 0x80 } else { 0 };
            b.push(octet);
            h >>= 7;
            if h == 0 {
                break;
            }
        }
        b
    }

    // ── Plane::new validation ────────────────────────────────────────────────

    #[test]
    fn plane_new_rejects_mismatched_len() {
        // width=2, height=2 → need 16 bytes, provide 8.
        let result = Plane::new(2, 2, &[0u8; 8]);
        assert!(result.is_err());
    }

    #[test]
    fn plane_new_accepts_correct_len() {
        let p = Plane::new(2, 2, &[0u8; 16]).unwrap();
        assert_eq!(p.width, 2);
        assert_eq!(p.height, 2);
    }

    #[test]
    fn plane_luma_at_returns_correct_value() {
        // Opaque white pixel → luma = 255.
        let rgba = [255u8, 255, 255, 255];
        let p = Plane::new(1, 1, &rgba).unwrap();
        assert_eq!(p.luma_at(0, 0, crate::Matte::White), 255);
    }

    #[test]
    fn plane_flatten_rgb_produces_rgb() {
        let rgba = [255u8, 0, 0, 255]; // opaque red
        let p = Plane::new(1, 1, &rgba).unwrap();
        let rgb = p.flatten_rgb(crate::Matte::White);
        assert_eq!(rgb, &[255, 0, 0]);
    }

    // ── varint helpers ────────────────────────────────────────────────────────

    #[test]
    fn varint_single_octet() {
        let input = [0x00];
        let mut c = 0;
        let v = read_varint(&input, &mut c).unwrap();
        assert_eq!(v, 0);
        assert_eq!(c, 1);
    }

    #[test]
    fn varint_two_octets() {
        // 0x81 0x01  →  (1 << 0) | (1 << 7) = 129
        let input = [0x81, 0x01];
        let mut c = 0;
        let v = read_varint(&input, &mut c).unwrap();
        assert_eq!(v, 129);
        assert_eq!(c, 2);
    }

    #[test]
    fn varint_truncated() {
        // Last octet has continuation bit set but there is no more data.
        let input = [0x81];
        let mut c = 0;
        let err = read_varint(&input, &mut c).unwrap_err();
        assert!(matches!(err, ImportError::Truncated { format: "wbmp", .. }));
    }

    // ── decode: header errors ─────────────────────────────────────────────────

    #[test]
    fn decode_rejects_empty() {
        let err = decode(&[]).unwrap_err();
        assert!(matches!(
            err,
            ImportError::Truncated {
                needed: "type + fixed header octets",
                ..
            }
        ));
    }

    #[test]
    fn decode_rejects_type_non_zero() {
        // type=1 is not WMT type 0.
        let b = wbmp_bytes(1, 0, 2, 2);
        let err = decode(&b).unwrap_err();
        assert!(matches!(err, ImportError::Unsupported { reason, .. }
                       if reason.contains("type 0")));
    }

    #[test]
    fn decode_rejects_extended_header() {
        // fixed = 0x20 signals an extension header.
        let b = wbmp_bytes(0, 0x20, 2, 2);
        let err = decode(&b).unwrap_err();
        assert!(matches!(err, ImportError::Unsupported { reason, .. }
                       if reason.contains("extended header")));
    }

    #[test]
    fn decode_rejects_nonzero_fixed_header() {
        let b = wbmp_bytes(0, 0x01, 2, 2);
        let err = decode(&b).unwrap_err();
        assert!(matches!(err, ImportError::Decode { reason, .. }
                       if reason.contains("0x00")));
    }

    #[test]
    fn decode_rejects_zero_width() {
        let b = wbmp_bytes(0, 0, 0, 2);
        let err = decode(&b).unwrap_err();
        assert!(matches!(err, ImportError::Decode { reason, .. }
                       if reason.contains("degenerate")));
    }

    #[test]
    fn decode_rejects_zero_height() {
        let b = wbmp_bytes(0, 0, 2, 0);
        let err = decode(&b).unwrap_err();
        assert!(matches!(err, ImportError::Decode { reason, .. }
                       if reason.contains("degenerate")));
    }

    #[test]
    fn decode_rejects_oversized_dimensions() {
        // width * height > MAX_PIXELS (100_000_000)
        let b = wbmp_bytes(0, 0, 200_000, 200_000);
        let err = decode(&b).unwrap_err();
        assert!(matches!(err, ImportError::Unsupported { reason, .. }
                       if reason.contains("exceeds")));
    }

    #[test]
    fn decode_rejects_truncated_pixel_array() {
        // Header OK: width=16, height=2 needs 4 pixel bytes
        // (2 bytes/row × 2 rows). Provide only 2 — still 2 short.
        let mut b = wbmp_bytes(0, 0, 16, 2);
        b.extend([0u8; 2]); // only 2 of 4 needed bytes
        let err = decode(&b).unwrap_err();
        assert!(matches!(
            err,
            ImportError::Truncated {
                needed: "complete pixel array",
                ..
            }
        ));
    }

    #[test]
    fn decode_roundtrips_through_encode() {
        let plane = white_plane();
        let encoded = encode(&plane, crate::Matte::White).unwrap();
        let (w, h, rgba) = decode(&encoded).unwrap();
        assert_eq!(w, 2);
        assert_eq!(h, 2);
        // White pixels are 255 after luma threshold, all opaque → RGBA [255,255,255,255].
        for px in rgba.chunks_exact(4) {
            assert_eq!(px, &[255, 255, 255, 255]);
        }
    }

    // ── encode: dimension limits ───────────────────────────────────────────────

    #[test]
    fn encode_produces_correct_header() {
        let plane = white_plane();
        let out = encode(&plane, crate::Matte::White).unwrap();
        // Type header 0x00 0x00, then varint width=2, height=2.
        // varint 2 = 0x02 (no continuation bit).
        assert_eq!(&out[0..2], &[0x00, 0x00]);
        assert_eq!(out[2], 2); // width varint
        assert_eq!(out[3], 2); // height varint
    }

    #[test]
    fn encode_pixel_row_thresholding() {
        // Opaque black pixel: luma=0 < 128 → bit=0 → row byte 0x00.
        // Opaque white pixel: luma=255 ≥ 128 → bit=1 → row byte 0x80.
        let black = [0u8, 0, 0, 255];
        let p_black = Plane::new(1, 1, &black).unwrap();
        let out_black = encode(&p_black, crate::Matte::White).unwrap();
        // Header (00 00 01 01) + 1 pixel byte; bit=0 → 0x00.
        assert_eq!(out_black[4], 0x00, "black pixel should set no bits");

        let white = [255u8, 255, 255, 255];
        let p_white = Plane::new(1, 1, &white).unwrap();
        let out_white = encode(&p_white, crate::Matte::White).unwrap();
        // bit=1 → 0x80 (bit 7 set = first pixel white).
        assert_eq!(out_white[4], 0x80, "white pixel should set bit 7");
    }

    #[test]
    fn encode_matte_black_affects_thresholding() {
        // Opaque white pixel over black matte → luma = (255*255 + 0*127)/255 = 255
        // → bit=1. Opaque black pixel over black matte → luma = 0 → bit=0.
        // 1×1 white pixel.
        let white = [255u8, 255, 255, 255];
        let p = Plane::new(1, 1, &white).unwrap();
        let out = encode(&p, crate::Matte::Black).unwrap();
        // Header + 1 pixel byte; bit 0 set → 0x80>>0 = 0x80.
        assert_eq!(out[4], 0x80);
    }
}
