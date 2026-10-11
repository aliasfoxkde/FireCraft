//! The full `.fw.png` read/write path plus the desktop
//! [`engine::platform::FileSystem`] implementation.
//!
//! Open: PNG chunks → `mkTS` node tree → typed document + unknown bag.
//! Save: document + bag → canonical tree → zlib → `mkTS` re-embedded
//! into the original chunk stream (everything else rides along
//! verbatim — `prVW`, `mkBT` tiles, the legacy `mkBS` copy — the same
//! preservation posture as the model's unknown bag).

use doc::{Document, fwmap};
use fwpng::chunk::{PNG_SIGNATURE, RawChunk, parse_chunks, write_chunk};
use fwpng::document::parse_mkts;
use fwpng::nodetree;
use std::io::Write as _;

use crate::CliError;

/// One opened document and everything needed to write it back.
pub struct Loaded {
    /// The typed document.
    pub document: Document,
    /// Nodes the model does not interpret, preserved for re-emit.
    pub unknown: doc::fwmap::UnknownBag,
    /// The original file's chunk stream (mkTS is replaced on save).
    pub chunks: Vec<RawChunk>,
}

/// Parses a `.fw.png` byte stream into its document.
pub fn open(bytes: &[u8]) -> Result<Loaded, CliError> {
    let chunks = parse_chunks(bytes).map_err(|error| CliError(format!("parse: {error}")))?;
    let Some(nodes) = parse_mkts(&chunks).map_err(|error| CliError(format!("mkTS: {error}")))?
    else {
        return Err(CliError(
            "no mkTS node tree in file (not a Fireworks document?)".to_owned(),
        ));
    };
    let (document, unknown) =
        fwmap::read(&nodes).map_err(|error| CliError(format!("model: {error}")))?;
    Ok(Loaded {
        document,
        unknown,
        chunks,
    })
}

/// Re-emits `document` canonically into the chunk stream of
/// `original`, producing a full `.fw.png` file's bytes.
pub fn save(
    document: &Document,
    unknown: &doc::fwmap::UnknownBag,
    original: &[RawChunk],
) -> Result<Vec<u8>, CliError> {
    let (tree, _stats) =
        fwmap::write(document, unknown).map_err(|error| CliError(format!("write: {error}")))?;
    let inflated = nodetree::serialize(&tree);
    let mut compressed = Vec::new();
    let mut encoder =
        flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::default());
    encoder
        .write_all(&inflated)
        .map_err(|error| CliError(format!("deflate: {error}")))?;
    encoder
        .finish()
        .map_err(|error| CliError(format!("deflate: {error}")))?;

    let mut out = Vec::from(PNG_SIGNATURE);
    // A never-opened document has no original stream: mint a fresh one
    // around the new tree (IHDR sized from the canvas, RGBA8). The
    // refusal below stays for real files that lost their mkTS — those
    // are suspicious, a blank slate is not.
    if original.is_empty() {
        let canvas = &document.info.canvas;
        let width = u32::try_from(canvas.width.max(1.0) as u64).unwrap_or(u32::MAX);
        let height = u32::try_from(canvas.height.max(1.0) as u64).unwrap_or(u32::MAX);
        let mut ihdr = Vec::with_capacity(13);
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        write_chunk(&mut out, b"IHDR", &ihdr);
        write_chunk(&mut out, b"mkTS", &compressed);
        write_chunk(&mut out, b"IEND", &[]);
        return Ok(out);
    }
    let mut tree_written = false;
    for chunk in original {
        if &chunk.ctype == b"mkTS" {
            if tree_written {
                continue; // one canonical tree, not the split originals
            }
            write_chunk(&mut out, b"mkTS", &compressed);
            tree_written = true;
        } else {
            write_chunk(&mut out, &chunk.ctype, &chunk.data);
        }
    }
    if !tree_written {
        return Err(CliError(
            "original stream has no mkTS chunk; refusing to invent one".to_owned(),
        ));
    }
    Ok(out)
}

/// The desktop file system, injected into engine services. Lives here
/// (not in `engine`) so the engine crate stays wasm-clean by
/// construction.
pub struct StdFs;

impl engine::platform::FileSystem for StdFs {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, engine::EngineError> {
        std::fs::read(path).map_err(|error| engine::EngineError::Io {
            path: path.to_owned(),
            detail: error.to_string(),
        })
    }

    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), engine::EngineError> {
        std::fs::write(path, bytes).map_err(|error| engine::EngineError::Io {
            path: path.to_owned(),
            detail: error.to_string(),
        })
    }

    fn exists(&self, path: &str) -> bool {
        std::path::Path::new(path).exists()
    }
}
