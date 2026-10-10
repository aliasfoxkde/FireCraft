//! Reader and writer for the Adobe Fireworks PNG document format.
//!
//! A Fireworks PNG is an ordinary PNG plus up to five private big-endian
//! chunks that carry the editable document:
//!
//! | Chunk  | Role |
//! |--------|------|
//! | `prVW` | zlib stream: `CA FE BE EF` magic, thumbnail width/height, RGB palette, 128x128 palette indices |
//! | `mkBF` | legacy file header (mostly zeros) |
//! | `mkTS` | zlib stream holding the node tree — the actual document |
//! | `mkBS` | legacy simplified copy of the node tree |
//! | `mkBT` | one tile of the 128x128 ARGB raster cache, per 64 KiB |
//!
//! This crate is standalone: it has no dependencies on the rest of the
//! FireCraft workspace and never links C, so it builds for `wasm32` and can
//! be published independently.
//!
//! Chunk byte offsets are provisional until pinned against the reference
//! corpus in milestone M2 (see `docs/planning/firecraft/M2-fwpng-read.md`).

pub mod chunk;
pub mod document;
pub mod error;
pub mod image;
pub mod nodetree;
pub mod thumbnail;
pub mod tile;

pub use chunk::{RawChunk, is_fireworks, parse_chunks, write_chunk};
pub use error::FwPngError;
pub use image::{StandardImage, decode_standard_rgba8};
pub use nodetree::{Node, NodeValue};
pub use thumbnail::Thumbnail;
pub use tile::{TileImage, parse_mkbt_chunks, parse_mkbt_stream};

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::chunk::{is_fireworks, parse_chunks, write_chunk};
    use super::nodetree::{parse, serialize};

    const CANONICAL: &str = "MKBv{FRCi{1}XLCi{0}LYLv{LAYv{BKGb{0}}}}";

    /// Builds a minimal PNG-shaped byte stream: signature + the given
    /// chunks + IEND.
    fn synthetic_file(extra: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut out = crate::chunk::PNG_SIGNATURE.to_vec();
        for (ctype, data) in extra {
            write_chunk(&mut out, ctype, data);
        }
        write_chunk(&mut out, b"IEND", &[]);
        out
    }

    #[test]
    fn chunk_round_trip_and_detection() {
        let tree = parse(CANONICAL.as_bytes()).expect("canonical example parses");
        let mkts = serialize(&tree);

        let file = synthetic_file(&[(b"mkTS", mkts.clone()), (b"mkBT", vec![0u8; 32])]);
        let chunks = parse_chunks(&file).expect("synthetic file parses");
        assert_eq!(chunks.len(), 3, "mkTS + mkBT + IEND");
        assert_eq!(chunks[0].ctype, *b"mkTS");
        assert_eq!(chunks[0].data, mkts);
        assert!(is_fireworks(&chunks));

        let plain = synthetic_file(&[]);
        let chunks = parse_chunks(&plain).expect("plain PNG parses");
        assert_eq!(chunks.len(), 1, "IEND only");
        assert!(!is_fireworks(&chunks));
    }

    #[test]
    fn corrupt_crc_is_rejected() {
        let mut file = synthetic_file(&[(b"mkTS", b"payload".to_vec())]);
        // Flip a byte inside the mkTS CRC (last 4 bytes of the chunk).
        let n = file.len();
        file[n - 2] ^= 0xFF;
        assert!(matches!(
            parse_chunks(&file),
            Err(crate::FwPngError::BadCrc { .. })
        ));
    }

    #[test]
    fn truncation_and_signature_are_rejected() {
        let file = synthetic_file(&[(b"mkTS", b"payload".to_vec())]);
        assert!(matches!(
            parse_chunks(&file[..file.len() - 3]),
            Err(crate::FwPngError::UnexpectedEof { .. })
        ));
        let mut not_png = file.clone();
        not_png[1] = b'X';
        assert!(matches!(
            parse_chunks(&not_png),
            Err(crate::FwPngError::BadSignature)
        ));
    }

    #[test]
    fn canonical_node_tree_round_trips_byte_for_byte() {
        let tree = parse(CANONICAL.as_bytes()).expect("parses");
        assert_eq!(serialize(&tree), CANONICAL.as_bytes());
        let root = tree.first().expect("root node");
        assert_eq!(root.fourcc, *b"MKBv");
        let kids = root.as_children().expect("root is a container");
        assert_eq!(kids.len(), 3, "FRCi, XLCi, LYLv");
        assert_eq!(kids[0].as_int(), Some(1));
        assert_eq!(kids[1].as_int(), Some(0));
    }
}
