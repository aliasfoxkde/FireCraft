//! The `mkBT` tile: one 128x128 ARGB block of the document's raster cache.

use crate::error::FwPngError;
use std::io::Read;

/// Magic at the start of a `mkBT` chunk payload.
const MAGIC: [u8; 4] = [0xFA, 0xCE, 0xCA, 0xFE];
/// Header size: 4 magic + 4 index + 3 reserved + 1 flag + 64 reserved.
/// Offsets are provisional until pinned against the reference corpus in M2.
pub const MKBT_HEADER_LEN: usize = 76;
/// Uncompressed tile payload size, always: 128x128 ARGB, or 256x256
/// grayscale.
pub const TILE_STREAM_BYTES: usize = 65_536;

/// Inflates a zlib stream shared by the private chunks.
pub(crate) fn inflate(data: &[u8]) -> Result<Vec<u8>, FwPngError> {
    let mut out = Vec::new();
    // read_to_end; the decoder reports malformed streams as io::Error.
    flate2::read::ZlibDecoder::new(data)
        .read_to_end(&mut out)
        .map_err(|_| FwPngError::Inflate {
            offset: 0,
            reason: "zlib stream failed to inflate",
        })?;
    Ok(out)
}

/// One decoded `mkBT` tile.
///
/// Long (grayscale) streams are split across consecutive `mkBT` chunks by
/// Fireworks; callers that encounter such files concatenate the payloads
/// before calling [`parse_mkbt`] (the M2 document layer handles this).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileImage {
    /// 24-bit tile index (the `TIDi` referenced by `TILv` nodes).
    pub index: u32,
    /// `true` when the payload is 256x256 grayscale rather than 128x128
    /// ARGB.
    pub grayscale: bool,
    /// Exactly [`TILE_STREAM_BYTES`] bytes of pixel data.
    pub data: Vec<u8>,
}

/// Decodes one `mkBT` chunk payload.
///
/// # Errors
/// [`FwPngError::BadPayload`] on a magic, length, or reserved-byte
/// mismatch; [`FwPngError::Inflate`] when the zlib stream is bad.
pub fn parse_mkbt(data: &[u8]) -> Result<TileImage, FwPngError> {
    if data.len() < MKBT_HEADER_LEN {
        return Err(FwPngError::BadPayload {
            chunk: *b"mkBT",
            offset: 0,
            reason: "chunk shorter than the 76-byte tile header",
        });
    }
    if data[..4] != MAGIC {
        return Err(FwPngError::BadPayload {
            chunk: *b"mkBT",
            offset: 0,
            reason: "mkBT magic mismatch (expected FA CE CA FE)",
        });
    }
    let index = u32::from_be_bytes([data[4], data[5], data[6], 0]) >> 8;
    let grayscale = data[11] != 0;
    let inflated = inflate(&data[MKBT_HEADER_LEN..])?;
    if inflated.len() != TILE_STREAM_BYTES {
        return Err(FwPngError::BadPayload {
            chunk: *b"mkBT",
            offset: MKBT_HEADER_LEN,
            reason: "inflated tile must be exactly 65536 bytes",
        });
    }
    Ok(TileImage {
        index,
        grayscale,
        data: inflated,
    })
}

impl TileImage {
    /// Expands the tile to row-major RGBA8: 128x128x4 for ARGB tiles,
    /// 256x256x4 for grayscale.
    ///
    /// Byte order inside ARGB tiles is provisional (A,R,G,B per the
    /// png-analyser notes) until the M2 corpus pins it.
    #[must_use]
    pub fn to_rgba8(&self) -> Vec<u8> {
        if self.grayscale {
            self.data.iter().flat_map(|&g| [g, g, g, 255]).collect()
        } else {
            self.data
                .chunks_exact(4)
                .flat_map(|px| [px[1], px[2], px[3], px[0]])
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::io::Write;

    fn zlib(raw: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(raw).expect("in-memory write");
        enc.finish().expect("in-memory finish")
    }

    #[test]
    fn argb_tile_round_trips() {
        let mut raw = vec![0u8; TILE_STREAM_BYTES];
        raw[0] = 0xAA; // A
        raw[1] = 0x11; // R
        raw[2] = 0x22; // G
        raw[3] = 0x33; // B

        let mut chunk = MAGIC.to_vec();
        chunk.extend_from_slice(&[0, 0, 42, 0]); // BE u32, index in top 24 bits
        chunk.extend_from_slice(&[0, 0, 0]); // reserved
        chunk.push(0); // grayscale flag at [11]
        chunk.extend_from_slice(&[0u8; 64]);
        chunk.extend_from_slice(&zlib(&raw));

        let tile = parse_mkbt(&chunk).expect("parses");
        assert_eq!(tile.index, 42);
        assert!(!tile.grayscale);
        let rgba = tile.to_rgba8();
        assert_eq!(&rgba[0..4], &[0x11, 0x22, 0x33, 0xAA]);
    }

    #[test]
    fn grayscale_flag_changes_expansion() {
        let mut raw = vec![0u8; TILE_STREAM_BYTES];
        raw[0] = 0x80;
        let mut chunk = MAGIC.to_vec();
        chunk.extend_from_slice(&[0, 0, 7, 0]); // BE u32 index
        chunk.extend_from_slice(&[0, 0, 0]); // reserved
        chunk.push(1); // grayscale flag at [11]
        chunk.extend_from_slice(&[0u8; 64]);
        chunk.extend_from_slice(&zlib(&raw));
        let tile = parse_mkbt(&chunk).expect("parses");
        assert!(tile.grayscale);
        assert_eq!(tile.index, 7);
        assert_eq!(&tile.to_rgba8()[0..4], &[0x80, 0x80, 0x80, 255]);
    }

    #[test]
    fn wrong_magic_is_rejected() {
        let mut chunk = vec![0u8; MKBT_HEADER_LEN + 16];
        chunk[0] = 0xFB;
        assert!(matches!(
            parse_mkbt(&chunk),
            Err(FwPngError::BadPayload {
                reason: "mkBT magic mismatch (expected FA CE CA FE)",
                ..
            })
        ));
    }

    #[test]
    fn wrong_inflated_size_is_rejected() {
        let mut chunk = MAGIC.to_vec();
        chunk.extend_from_slice(&[0u8; MKBT_HEADER_LEN - 4]);
        chunk.extend_from_slice(&zlib(&[0u8; 100]));
        assert!(matches!(
            parse_mkbt(&chunk),
            Err(FwPngError::BadPayload {
                reason: "inflated tile must be exactly 65536 bytes",
                ..
            })
        ));
    }

    #[test]
    fn truncated_header_is_rejected() {
        assert!(matches!(
            parse_mkbt(&[0; 10]),
            Err(FwPngError::BadPayload {
                reason: "chunk shorter than the 76-byte tile header",
                ..
            })
        ));
    }
}
