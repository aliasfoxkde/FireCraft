//! PNG chunk-level parsing and writing, including CRC validation and the
//! Fireworks-chunk detection helper.

use crate::error::FwPngError;

/// The 8-byte PNG file signature.
pub const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Largest chunk payload accepted. Real Fireworks chunks are far smaller
/// (`mkBT` caps at 64 KiB per tile); the cap exists so a hostile length
/// field cannot drive a huge allocation before bounds checks.
pub const MAX_CHUNK_LEN: u32 = 256 * 1024 * 1024;

/// A chunk exactly as it appears in the file: type, payload, CRC, and the
/// file offset of its length field. Unknown chunks are preserved so a
/// later writer can round-trip files byte-identically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawChunk {
    /// Four-byte chunk type, always ASCII.
    pub ctype: [u8; 4],
    /// Chunk payload (data between type and CRC).
    pub data: Vec<u8>,
    /// File offset of the chunk's length field.
    pub offset: usize,
    /// CRC-32 recorded in the file (verified against the bytes before
    /// parsing returns).
    pub crc: u32,
}

impl RawChunk {
    /// The chunk type as a string (safe: types are validated ASCII).
    #[must_use]
    pub fn type_str(&self) -> &str {
        std::str::from_utf8(&self.ctype).unwrap_or_default()
    }
}

/// Parses a complete PNG stream into its chunks, verifying each CRC.
///
/// Stops at `IEND`. Bytes after `IEND` are ignored (some writers pad).
///
/// # Errors
/// [`FwPngError::BadSignature`] when the signature is wrong,
/// [`FwPngError::BadCrc`] on any CRC mismatch, [`FwPngError::BadChunk`]
/// for malformed headers or oversized lengths, [`FwPngError::UnexpectedEof`]
/// for truncated streams, including a missing `IEND`.
pub fn parse_chunks(input: &[u8]) -> Result<Vec<RawChunk>, FwPngError> {
    if !input.starts_with(&PNG_SIGNATURE) {
        return Err(FwPngError::BadSignature);
    }
    let mut pos = 8usize;
    let mut chunks = Vec::new();
    loop {
        if pos >= input.len() {
            return Err(FwPngError::UnexpectedEof {
                offset: pos,
                needed: 8,
            });
        }
        let offset = pos;
        let header = read(input, pos, 8, offset)?;
        let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let ctype = [header[4], header[5], header[6], header[7]];
        pos += 8;

        if !ctype.iter().all(u8::is_ascii) {
            return Err(FwPngError::BadChunk {
                chunk: ctype,
                offset,
                reason: "chunk type is not ASCII",
            });
        }
        if len > MAX_CHUNK_LEN {
            return Err(FwPngError::BadChunk {
                chunk: ctype,
                offset,
                reason: "chunk length exceeds safety cap",
            });
        }

        let data = read(input, pos, len as usize, offset)?.to_vec();
        pos += data.len();
        let crc_bytes = read(input, pos, 4, offset)?;
        let crc = u32::from_be_bytes([crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]]);
        pos += 4;

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&ctype);
        hasher.update(&data);
        let computed = hasher.finalize();
        if computed != crc {
            return Err(FwPngError::BadCrc {
                chunk: ctype,
                expected: crc,
                got: computed,
                offset,
            });
        }

        let is_end = &ctype == b"IEND";
        chunks.push(RawChunk {
            ctype,
            data,
            offset,
            crc,
        });
        if is_end {
            return Ok(chunks);
        }
    }
}

/// Appends one chunk (`length | type | data | crc`) to `out`.
pub fn write_chunk(out: &mut Vec<u8>, ctype: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes());
    out.extend_from_slice(ctype);
    out.extend_from_slice(data);
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(ctype);
    hasher.update(data);
    out.extend_from_slice(&hasher.finalize().to_be_bytes());
}

/// Whether any Fireworks private chunk is present.
#[must_use]
pub fn is_fireworks(chunks: &[RawChunk]) -> bool {
    chunks.iter().any(|c| is_fireworks_type(&c.ctype))
}

/// The five Fireworks private chunk types (ASCII, case-sensitive: real
/// files use `prVW`/`mkTS`/`mkBS`/`mkBT`/`mkBF`).
#[must_use]
pub fn is_fireworks_type(ctype: &[u8; 4]) -> bool {
    matches!(ctype, b"prVW" | b"mkTS" | b"mkBS" | b"mkBT" | b"mkBF")
}

/// Reads exactly `count` bytes at `pos` or fails with [`FwPngError::UnexpectedEof`].
fn read(input: &[u8], pos: usize, count: usize, chunk_offset: usize) -> Result<&[u8], FwPngError> {
    input
        .get(pos..pos.saturating_add(count))
        .ok_or(FwPngError::UnexpectedEof {
            offset: chunk_offset,
            needed: count,
        })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn build(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut out = PNG_SIGNATURE.to_vec();
        for (ctype, data) in chunks {
            write_chunk(&mut out, ctype, data);
        }
        write_chunk(&mut out, b"IEND", &[]);
        out
    }

    #[test]
    fn round_trips_types_data_and_offsets() {
        let file = build(&[(b"mkTS", b"hello"), (b"mkBT", &[1, 2, 3])]);
        let chunks = parse_chunks(&file).expect("parses");
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].type_str(), "mkTS");
        assert_eq!(chunks[0].data, b"hello");
        assert_eq!(chunks[0].offset, 8);
        assert_eq!(chunks[2].ctype, *b"IEND");
        assert!(chunks[2].data.is_empty());
    }

    #[test]
    fn empty_payload_is_legal() {
        let file = build(&[]);
        let chunks = parse_chunks(&file).expect("parses");
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn private_chunks_round_trip_byte_identically() {
        // Standard + private mix in the shape of a real .fw.png: unknown
        // ancillary chunks must survive in order, byte for byte.
        let mut png = PNG_SIGNATURE.to_vec();
        write_chunk(&mut png, b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
        write_chunk(&mut png, b"prVW", &[0xCA, 0xFE, 0xBE, 0xEF, 1, 2, 3, 4]);
        write_chunk(&mut png, b"mkTS", b"MKBv{FRCi{1}}");
        write_chunk(&mut png, b"mkBS", b"MKBv{FRCi{1}}");
        write_chunk(&mut png, b"mkBT", &[0xFA, 0xCE, 0xCA, 0xFE, 0, 0, 0, 0]);
        write_chunk(&mut png, b"gAmA", &[0, 1, 0x86, 0xA0]);
        write_chunk(&mut png, b"IEND", &[]);
        assert!(is_fireworks(&parse_chunks(&png).expect("parses")));

        let chunks = parse_chunks(&png).expect("parses");
        let types: Vec<&str> = chunks.iter().map(|c| c.type_str()).collect();
        assert_eq!(
            types,
            ["IHDR", "prVW", "mkTS", "mkBS", "mkBT", "gAmA", "IEND"]
        );

        let mut rebuilt = PNG_SIGNATURE.to_vec();
        for chunk in &chunks {
            write_chunk(&mut rebuilt, &chunk.ctype, &chunk.data);
        }
        assert_eq!(rebuilt, png, "reconstruction must be byte-identical");
    }

    #[test]
    fn oversized_length_is_rejected_before_allocation() {
        let mut file = PNG_SIGNATURE.to_vec();
        file.extend_from_slice(&u32::MAX.to_be_bytes());
        file.extend_from_slice(b"mkTS");
        assert!(matches!(
            parse_chunks(&file),
            Err(FwPngError::BadChunk {
                reason: "chunk length exceeds safety cap",
                ..
            })
        ));
    }

    #[test]
    fn non_ascii_type_is_rejected() {
        let mut file = PNG_SIGNATURE.to_vec();
        file.extend_from_slice(&0u32.to_be_bytes());
        file.extend_from_slice(&[0xFF, b'm', b'k', b'T']);
        file.extend_from_slice(&0u32.to_be_bytes());
        assert!(matches!(
            parse_chunks(&file),
            Err(FwPngError::BadChunk {
                reason: "chunk type is not ASCII",
                ..
            })
        ));
    }

    #[test]
    fn missing_iend_is_eof() {
        let mut file = build(&[(b"mkTS", b"x")]);
        // Drop the trailing IEND entirely (20 bytes: 4+4+1+4 and 4+4+0+4).
        let cut = file.len() - 20;
        file.truncate(cut);
        assert!(matches!(
            parse_chunks(&file),
            Err(FwPngError::UnexpectedEof { .. })
        ));
    }

    #[test]
    fn trailing_bytes_after_iend_are_ignored() {
        let mut file = build(&[]);
        file.extend_from_slice(b"garbage");
        let chunks = parse_chunks(&file).expect("parses");
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn fireworks_type_detection() {
        assert!(is_fireworks_type(b"prVW"));
        assert!(is_fireworks_type(b"mkBT"));
        assert!(!is_fireworks_type(b"IHDR"));
        assert!(!is_fireworks_type(b"MKTS"), "case-sensitive");
    }
}
