//! The `prVW` thumbnail: a 128x128 indexed-color preview of the flattened
//! document.

use crate::error::FwPngError;

/// Magic at the start of the inflated `prVW` stream.
const MAGIC: [u8; 4] = [0xCA, 0xFE, 0xBE, 0xEF];
/// Bytes of palette indices at the end of the stream (128x128, 1 byte
/// per pixel).
pub const THUMB_BYTES: usize = 128 * 128;
/// Thumbnail edge length.
pub const THUMB_SIDE: usize = 128;

/// The decoded `prVW` preview.
///
/// Only the top-left `width x height` of the 128x128 index grid is
/// meaningful; trailing index bytes are uninitialized writer padding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thumbnail {
    /// Preview width in pixels (<= 128).
    pub width: u16,
    /// Preview height in pixels (<= 128).
    pub height: u16,
    /// RGB palette triplets, index = palette position.
    pub palette: Vec<[u8; 3]>,
    /// Exactly [`THUMB_BYTES`] palette indices, row-major from the
    /// top-left.
    pub indices: Vec<u8>,
}

/// Inflates and decodes a `prVW` chunk payload.
///
/// # Errors
/// [`FwPngError::Inflate`] when the zlib stream is bad, [`FwPngError::BadPayload`]
/// when the decoded stream is shorter than the fixed-size index block, has
/// the wrong magic, or its palette never terminates.
pub fn parse_prvw(data: &[u8]) -> Result<Thumbnail, FwPngError> {
    let inflated = crate::tile::inflate(data)?;
    if inflated.len() < 8 + THUMB_BYTES {
        return Err(FwPngError::BadPayload {
            chunk: *b"prVW",
            offset: 0,
            reason: "inflated stream is shorter than the fixed 16384-byte index block",
        });
    }
    if inflated[..4] != MAGIC {
        return Err(FwPngError::BadPayload {
            chunk: *b"prVW",
            offset: 0,
            reason: "prVW magic mismatch (expected CA FE BE EF)",
        });
    }
    let width = u16::from_be_bytes([inflated[4], inflated[5]]);
    let height = u16::from_be_bytes([inflated[6], inflated[7]]);

    // RGB triplets up to the 00 00 00 terminator, never past the start of
    // the fixed index block.
    let indices_start = inflated.len() - THUMB_BYTES;
    let mut palette = Vec::new();
    let mut i = 8usize;
    while i + 3 <= indices_start && inflated[i..i + 3] != [0, 0, 0] {
        palette.push([inflated[i], inflated[i + 1], inflated[i + 2]]);
        i += 3;
    }
    if i + 3 > indices_start {
        return Err(FwPngError::BadPayload {
            chunk: *b"prVW",
            offset: i,
            reason: "prVW palette has no 00 00 00 terminator before the index block",
        });
    }

    Ok(Thumbnail {
        width,
        height,
        palette,
        indices: inflated[indices_start..].to_vec(),
    })
}

impl Thumbnail {
    /// Expands the top-left `width x height` region to straight-alpha RGBA8
    /// (`width * height * 4` bytes). Out-of-range indices decode as opaque
    /// black — real writers emit palette colors, not holes.
    #[must_use]
    pub fn to_rgba8(&self) -> Vec<u8> {
        let (w, h) = (self.width as usize, self.height as usize);
        let mut out = vec![0u8; w * h * 4];
        for (y, row) in out.chunks_exact_mut(w * 4).enumerate() {
            for (x, px) in row.chunks_exact_mut(4).enumerate() {
                let idx = self.indices[y * THUMB_SIDE + x];
                let [r, g, b] = self.palette.get(idx as usize).copied().unwrap_or([0, 0, 0]);
                px[0] = r;
                px[1] = g;
                px[2] = b;
                px[3] = 255;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::io::Write;

    /// Builds a synthetic inflated prVW stream and zlib-compresses it.
    fn synthetic(width: u16, height: u16, palette: &[[u8; 3]]) -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(&MAGIC);
        raw.extend_from_slice(&width.to_be_bytes());
        raw.extend_from_slice(&height.to_be_bytes());
        for rgb in palette {
            raw.extend_from_slice(rgb);
        }
        raw.extend_from_slice(&[0, 0, 0]); // terminator
        while raw.len() < 8 + 8 + THUMB_BYTES {
            raw.push(0xEE); // uninitialized padding
        }
        raw.extend_from_slice(&[0u8; THUMB_BYTES]); // index block
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(&raw).expect("in-memory write");
        enc.finish().expect("in-memory finish")
    }

    #[test]
    fn round_trip_dimensions_and_palette() {
        let data = synthetic(100, 60, &[[255, 0, 0], [0, 255, 0]]);
        let t = parse_prvw(&data).expect("parses");
        assert_eq!((t.width, t.height), (100, 60));
        assert_eq!(t.palette, vec![[255, 0, 0], [0, 255, 0]]);
        assert_eq!(t.indices.len(), THUMB_BYTES);

        let rgba = t.to_rgba8();
        assert_eq!(rgba.len(), 100 * 60 * 4);
        assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
        // Second palette color at index 1.
        let t2 = Thumbnail {
            indices: {
                let mut ix = vec![0u8; THUMB_BYTES];
                ix[0] = 1;
                ix
            },
            ..t.clone()
        };
        assert_eq!(&t2.to_rgba8()[0..4], &[0, 255, 0, 255]);
    }

    #[test]
    fn bad_magic_is_rejected() {
        // Zeroed raw stream: the prVW magic check fails before anything else.
        let raw = vec![0u8; 8 + THUMB_BYTES];
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(&raw).expect("in-memory write");
        let data = enc.finish().expect("in-memory finish");
        assert!(matches!(
            parse_prvw(&data),
            Err(FwPngError::BadPayload {
                reason: "prVW magic mismatch (expected CA FE BE EF)",
                ..
            })
        ));
    }

    #[test]
    fn short_stream_is_rejected() {
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(&MAGIC).expect("in-memory write");
        let data = enc.finish().expect("in-memory finish");
        assert!(matches!(
            parse_prvw(&data),
            Err(FwPngError::BadPayload {
                reason: "inflated stream is shorter than the fixed 16384-byte index block",
                ..
            })
        ));
    }

    #[test]
    fn corrupt_zlib_is_rejected() {
        let mut data = synthetic(10, 10, &[]);
        let n = data.len();
        data[n - 1] ^= 0xFF;
        assert!(matches!(parse_prvw(&data), Err(FwPngError::Inflate { .. })));
    }
}
