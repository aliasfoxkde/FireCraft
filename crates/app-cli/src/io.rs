//! The full `.fw.png` read/write path plus the desktop
//! [`engine::platform::FileSystem`] implementation.
//!
//! Open: PNG chunks → `mkTS` node tree → typed document + unknown bag,
//! with the file's `mkBT` tiles joined into the document's raster bank
//! (`TIDi` → tile, ARGB converted). Save: document + bag → canonical
//! tree → zlib → `mkTS` re-embedded into the original chunk stream
//! (everything else rides along verbatim — `prVW`, the legacy `mkBS`
//! copy — the same preservation posture as the model's unknown bag);
//! bank tiles the original stream does not carry (minted at runtime)
//! are appended before `IEND`.

use doc::{Document, fwmap};
use fwpng::chunk::{PNG_SIGNATURE, RawChunk, parse_chunks, write_chunk};
use fwpng::document::parse_mkts;
use fwpng::nodetree;
use std::collections::BTreeSet;
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

/// Joins a file's `mkBT` tiles into `document`'s raster bank (`TIDi`
/// → tile, ARGB → RGBA8). A 256×256 grayscale tile has no 128×128
/// RGBA bank slot — skipped here; a `TILv` that references one fails
/// loudly at render ([`compose::ComposeError::TileBankMiss`]) rather
/// than at load, so unreferenced strays stay harmless.
fn join_tiles(document: &mut Document, chunks: &[RawChunk]) -> Result<(), CliError> {
    if !chunks.iter().any(|chunk| &chunk.ctype == b"mkBT") {
        return Ok(());
    }
    let images =
        fwpng::parse_mkbt_chunks(chunks).map_err(|error| CliError(format!("mkBT: {error}")))?;
    for image in images {
        if image.grayscale {
            continue;
        }
        let plane: [u8; raster::TILE_BYTES] = image
            .to_rgba8()
            .try_into()
            .map_err(|_| CliError(format!("mkBT tile {} is not 128×128", image.index)))?;
        document
            .tiles
            .insert(image.index, raster::Tile::from_rgba8(plane));
    }
    Ok(())
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
    let (mut document, unknown) =
        fwmap::read(&nodes).map_err(|error| CliError(format!("model: {error}")))?;
    join_tiles(&mut document, &chunks)?;
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
        write_new_tiles(&mut out, document, &BTreeSet::new())?;
        write_chunk(&mut out, b"IEND", &[]);
        return Ok(out);
    }
    // The original stream's tile ids: bank tiles already on disk stay
    // verbatim (byte-exact round-trips); only runtime-minted ids —
    // ones flatten/retouch/place-image added since open — serialize.
    // A malformed tile stream refuses: the id set cannot be known, so
    // neither a verbatim nor a merged re-emit is honest.
    let original_ids = original_tile_ids(original)?;
    let fresh: BTreeSet<u32> = document
        .tiles
        .ids()
        .filter(|id| !original_ids.contains(id))
        .collect();
    let mut tree_written = false;
    for chunk in original {
        if &chunk.ctype == b"IEND" {
            write_new_tiles(&mut out, document, &fresh)?;
        }
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

/// The tile ids one original chunk stream carries. A malformed tile
/// stream refuses — the id set cannot be known, so neither a verbatim
/// nor a merged re-emit is honest.
fn original_tile_ids(original: &[RawChunk]) -> Result<BTreeSet<u32>, CliError> {
    if !original.iter().any(|chunk| &chunk.ctype == b"mkBT") {
        return Ok(BTreeSet::new());
    }
    Ok(fwpng::parse_mkbt_chunks(original)
        .map_err(|error| CliError(format!("mkBT: {error}")))?
        .into_iter()
        .map(|tile| tile.index)
        .collect())
}

/// Serializes the document bank's `fresh` tile ids into `mkBT` chunks
/// (bank order, lowest id first). A high-entropy tile's compressed
/// stream exceeds one 64 KiB chunk; [`fwpng::encode_mkbt`] splits it
/// into the header-carrying first payload plus raw continuations.
fn write_new_tiles(
    out: &mut Vec<u8>,
    document: &Document,
    fresh: &BTreeSet<u32>,
) -> Result<(), CliError> {
    for id in fresh {
        let Some(tile) = document.tiles.get(*id) else {
            continue;
        };
        for payload in fwpng::encode_mkbt(*id, tile.pixels())
            .map_err(|error| CliError(format!("mkBT encode: {error}")))?
        {
            write_chunk(out, b"mkBT", &payload);
        }
    }
    Ok(())
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

    fn read_dir(&self, path: &str) -> Result<Vec<String>, engine::EngineError> {
        let mut names = Vec::new();
        let entries = std::fs::read_dir(path).map_err(|error| engine::EngineError::Io {
            path: path.to_owned(),
            detail: error.to_string(),
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| engine::EngineError::Io {
                path: path.to_owned(),
                detail: error.to_string(),
            })?;
            if entry
                .file_type()
                .map_err(|error| engine::EngineError::Io {
                    path: path.to_owned(),
                    detail: error.to_string(),
                })?
                .is_file()
            {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        names.sort();
        Ok(names)
    }
}
