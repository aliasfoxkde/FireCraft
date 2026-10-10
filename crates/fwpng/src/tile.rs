//! The `mkBT` tile: one 128x128 ARGB block of the document's raster cache.

use crate::chunk::RawChunk;
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
/// Cap on the total compressed bytes one tile stream may occupy before
/// inflation (hostile-input rule; real tiles compress far below 1 MiB).
pub const MAX_TILE_STREAM_COMPRESSED: usize = 1024 * 1024;

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
/// Compressed streams larger than one chunk are split across consecutive
/// `mkBT` chunks with the same index; [`parse_mkbt_chunks`] reassembles
/// them.
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

/// Extracts the 24-bit tile index from a `mkBT` payload (BE u32 at
/// offset 4 with the index in the top three bytes).
fn tile_index(data: &[u8]) -> Result<u32, FwPngError> {
    data.get(4..8)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], 0]) >> 8)
        .ok_or(FwPngError::BadPayload {
            chunk: *b"mkBT",
            offset: 4,
            reason: "mkBT chunk shorter than its fixed header",
        })
}

/// Decodes one tile whose compressed stream spans consecutive `mkBT`
/// payloads. The first payload carries the 76-byte header; every payload
/// is a raw piece of the zlib stream (Fireworks splits >64 KiB streams
/// across consecutive same-index chunks; continuation payloads carry no
/// header of their own — provisional until the corpus pins it).
///
/// # Errors
/// As [`parse_mkbt`], plus [`FwPngError::BadPayload`] when the
/// concatenation exceeds [`MAX_TILE_STREAM_COMPRESSED`].
pub fn parse_mkbt_stream(payloads: &[&[u8]]) -> Result<TileImage, FwPngError> {
    let mut joined = Vec::new();
    for payload in payloads {
        if joined.len() + payload.len() > MAX_TILE_STREAM_COMPRESSED {
            return Err(FwPngError::BadPayload {
                chunk: *b"mkBT",
                offset: joined.len(),
                reason: "tile stream exceeds the compressed-size cap",
            });
        }
        joined.extend_from_slice(payload);
    }
    parse_mkbt(&joined)
}

/// Decodes every tile from the `mkBT` chunks of a parsed PNG. A chunk
/// whose payload starts with the tile magic *starts* a stream; any other
/// consecutive `mkBT` chunk is a raw continuation of the previous stream
/// (Fireworks splits >64 KiB compressed streams mid-zlib, and the
/// continuation carries no header — provisional until the corpus pins
/// it). Tiles come back in file order.
///
/// # Errors
/// As [`parse_mkbt`], plus [`FwPngError::BadPayload`] when a continuation
/// appears with no stream to continue.
pub fn parse_mkbt_chunks(chunks: &[RawChunk]) -> Result<Vec<TileImage>, FwPngError> {
    const MKBT: [u8; 4] = *b"mkBT";
    let mut tiles = Vec::new();
    let mut group: Vec<&[u8]> = Vec::new();
    for chunk in chunks {
        if chunk.ctype != MKBT {
            continue;
        }
        let starts_stream = chunk.data.len() >= 4 && chunk.data[..4] == MAGIC;
        if starts_stream {
            if !group.is_empty() {
                tiles.push(parse_mkbt_stream(&group)?);
                group.clear();
            }
            group.push(&chunk.data);
        } else if group.is_empty() {
            return Err(FwPngError::BadPayload {
                chunk: MKBT,
                offset: chunk.offset,
                reason: "mkBT continuation chunk with no stream to continue",
            });
        } else {
            group.push(&chunk.data);
        }
    }
    if !group.is_empty() {
        tiles.push(parse_mkbt_stream(&group)?);
    }
    Ok(tiles)
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
    let index = tile_index(data)?;
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
    use crate::chunk::{PNG_SIGNATURE, parse_chunks, write_chunk};
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

    /// Builds a full mkBT payload with the given tile index and raw
    /// (pre-zlib) pixel bytes.
    fn tile_payload(index: u32, grayscale: u8, raw: &[u8]) -> Vec<u8> {
        let mut chunk = MAGIC.to_vec();
        chunk.extend_from_slice(&(index << 8).to_be_bytes());
        chunk.extend_from_slice(&[0, 0, 0]); // reserved
        chunk.push(grayscale); // flag at [11]
        chunk.extend_from_slice(&[0u8; 64]);
        chunk.extend_from_slice(&zlib(raw));
        chunk
    }

    #[test]
    fn split_stream_matches_single() {
        let mut raw = vec![0u8; TILE_STREAM_BYTES];
        raw[10] = 0x5A;
        let payload = tile_payload(3, 0, &raw);

        // Split the compressed stream at an arbitrary early point: header
        // + first span in the first payload, raw remainder in the second.
        let split_at = MKBT_HEADER_LEN + (payload.len() - MKBT_HEADER_LEN) / 3;
        let first = &payload[..split_at];
        let rest = &payload[split_at..];
        let tile = parse_mkbt_stream(&[first, rest]).expect("split stream parses");
        let whole = parse_mkbt(&payload).expect("single parses");
        assert_eq!(tile, whole);
        assert_eq!(tile.index, 3);
    }

    #[test]
    fn chunk_list_groups_consecutive_same_index() {
        let mut raw = vec![0u8; TILE_STREAM_BYTES];
        raw[100] = 0x77;
        let payload0 = tile_payload(0, 0, &raw);

        let mut file = Vec::from(PNG_SIGNATURE);
        write_chunk(&mut file, b"mkTS", b"MKBv{FRCi{1}}");
        // Tile 0 split across two chunks (mid-compressed-stream)...
        let split_at = MKBT_HEADER_LEN + (payload0.len() - MKBT_HEADER_LEN).div_ceil(2);
        write_chunk(&mut file, b"mkBT", &payload0[..split_at]);
        write_chunk(&mut file, b"mkBT", &payload0[split_at..]);
        // ...then tile 1 in one chunk.
        write_chunk(
            &mut file,
            b"mkBT",
            &tile_payload(1, 0, &[0u8; TILE_STREAM_BYTES]),
        );
        write_chunk(&mut file, b"IEND", &[]);

        let chunks = parse_chunks(&file).expect("parses");
        let tiles = parse_mkbt_chunks(&chunks).expect("tiles decode");
        assert_eq!(tiles.len(), 2, "split tile must reassemble to one");
        assert_eq!(tiles[0].index, 0);
        assert_eq!(tiles[0].data[100], 0x77);
        assert_eq!(tiles[1].index, 1);
    }

    #[test]
    fn oversized_stream_is_capped_not_allocated() {
        // Two payloads whose sum exceeds the 1 MiB compressed cap: the
        // second must be rejected on the running total, not allocated.
        let big = vec![0u8; MAX_TILE_STREAM_COMPRESSED];
        let half = vec![0u8; 8];
        assert!(matches!(
            parse_mkbt_stream(&[&big, &half]),
            Err(FwPngError::BadPayload {
                reason: "tile stream exceeds the compressed-size cap",
                ..
            })
        ));
    }

    #[test]
    fn short_mkbt_chunk_in_list_is_rejected_with_offset() {
        let mut file = Vec::from(PNG_SIGNATURE);
        write_chunk(&mut file, b"mkBT", &[0, 0, 0]);
        write_chunk(&mut file, b"IEND", &[]);
        let chunks = parse_chunks(&file).expect("parses");
        assert!(matches!(
            parse_mkbt_chunks(&chunks),
            Err(FwPngError::BadPayload { offset: 8, .. })
        ));
    }

    #[test]
    fn headerless_continuation_with_no_stream_is_rejected() {
        let mut file = Vec::from(PNG_SIGNATURE);
        // Raw zlib bytes, no magic: cannot start a stream.
        write_chunk(&mut file, b"mkBT", &[0x78, 0x9c, 1, 2, 3, 4]);
        write_chunk(&mut file, b"IEND", &[]);
        let chunks = parse_chunks(&file).expect("parses");
        assert!(matches!(
            parse_mkbt_chunks(&chunks),
            Err(FwPngError::BadPayload {
                reason: "mkBT continuation chunk with no stream to continue",
                ..
            })
        ));
    }
}
