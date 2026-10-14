//! The `mkBT` bank across the file boundary: tiles a document's bank
//! carries that the original chunk stream does not (runtime-minted by
//! flatten / place-image / retouch) serialize on save and rejoin the
//! bank on open; untouched files are stable — a tile-free file saves
//! byte-exactly, and a foreign tiled file keeps every non-`mkTS`
//! chunk and its bank verbatim (the tree itself re-canonicalizes).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app_cli::CliError;
use app_cli::io;
use engine::CommandRegistry;
use engine::catalog;
use engine::fuzzdrive::{FuzzRng, random_document};

/// Bytes of a tile-free `.fw.png`, canonically written through the
/// model.
fn fwpng_bytes(document: &doc::Document) -> Vec<u8> {
    let (tree, _) =
        doc::fwmap::write(document, &doc::fwmap::UnknownBag::default()).expect("writes");
    let inflated = fwpng::nodetree::serialize(&tree);
    let mut compressed = Vec::new();
    let mut encoder =
        flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &inflated).expect("deflate");
    drop(encoder);
    let mut file = Vec::new();
    file.extend_from_slice(&fwpng::chunk::PNG_SIGNATURE);
    fwpng::chunk::write_chunk(&mut file, b"mkTS", &compressed);
    fwpng::chunk::write_chunk(&mut file, b"IEND", &[]);
    file
}

fn registry() -> CommandRegistry {
    let mut reg = CommandRegistry::new();
    let _ = catalog::install(&mut reg);
    reg
}

#[test]
fn minted_tiles_reach_the_file_and_rejoin_the_bank() {
    let mut rng = FuzzRng::new(11);
    let mut document = random_document(&mut rng);
    // A canvas the placement can intersect, and a cell the fuzz spine
    // always carries (page 0 / layer 0 / cell 0).
    document.pages[0].info.canvas.width = 64.0;
    document.pages[0].info.canvas.height = 48.0;
    let file = fwpng_bytes(&document);
    let mut loaded = io::open(&file).expect("opens");
    let before_registry = loaded.document.registry.len();
    assert!(
        loaded.document.tiles.is_empty(),
        "the fuzz spine starts bank-free"
    );

    let registry = registry();
    registry
        .execute(
            &mut loaded.document,
            "bitmap.addImage",
            serde_json::json!({
                "x": 0.0, "y": 0.0, "width": 8.0, "height": 8.0,
                "page": 0, "layer": 0, "cell": 0, "name": null,
            }),
        )
        .expect("place-image mints fresh zero tiles");
    let minted: Vec<u32> = loaded.document.tiles.ids().collect();
    assert!(!minted.is_empty(), "the command minted bank tiles");

    let saved = io::save(&loaded.document, &loaded.unknown, &loaded.chunks)
        .expect("saves with the minted bank");
    let reopened = io::open(&saved).expect("reopens");
    assert_eq!(
        reopened.document.registry.len(),
        before_registry + 1,
        "the placed image survives the round trip"
    );
    for id in &minted {
        let tile = reopened.document.tiles.get(*id).expect("tile rejoins");
        assert!(
            tile.pixels().iter().all(|&byte| byte == 0),
            "tile {id} carries the placed image's fresh zeros"
        );
    }

    // The merged form is canonical: a second save is byte-identical.
    let resaved =
        io::save(&reopened.document, &reopened.unknown, &reopened.chunks).expect("resaves");
    assert_eq!(saved, resaved, "merged tiles re-emit stably");
}

#[test]
fn untouched_files_keep_their_chunks_and_bank() {
    // A tile-free file: the bank merge contributes nothing.
    let mut rng = FuzzRng::new(3);
    let document = random_document(&mut rng);
    let file = fwpng_bytes(&document);
    let loaded = io::open(&file).expect("opens");
    let saved = io::save(&loaded.document, &loaded.unknown, &loaded.chunks).expect("saves");
    assert_eq!(file, saved, "no bank, no change");

    // Tiled corpus fixtures: the bank rides verbatim — populating the
    // in-memory bank at open must not perturb a single tile byte. The
    // tree itself re-canonicalizes on save (foreign files are not
    // byte-stable by design), so the comparison is chunk-wise with
    // mkTS excluded, plus the same bank after a reopen.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    for name in ["minimal.fw.png", "tiles-split.fw.png"] {
        let path = format!("{manifest_dir}/../../corpus/fixtures/{name}");
        let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("read {path}: {error}"));
        let loaded = io::open(&bytes).expect("opens");
        let bank: Vec<(u32, Vec<u8>)> = loaded
            .document
            .tiles
            .ids()
            .map(|id| {
                (
                    id,
                    loaded
                        .document
                        .tiles
                        .get(id)
                        .expect("bank")
                        .pixels()
                        .to_vec(),
                )
            })
            .collect();
        assert!(!bank.is_empty(), "{name} carries tiles into the bank");
        let saved = io::save(&loaded.document, &loaded.unknown, &loaded.chunks).expect("saves");
        let before = fwpng::parse_chunks(&bytes).expect("original parses");
        let after = fwpng::parse_chunks(&saved).expect("saved parses");
        let non_tree = |chunks: &[fwpng::RawChunk]| -> Vec<(Vec<u8>, Vec<u8>)> {
            chunks
                .iter()
                .filter(|chunk| &chunk.ctype != b"mkTS")
                .map(|chunk| (chunk.ctype.to_vec(), chunk.data.clone()))
                .collect()
        };
        assert_eq!(non_tree(&before), non_tree(&after), "{name} chunks");
        let reopened = io::open(&saved).expect("reopens");
        let reopened_bank: Vec<(u32, Vec<u8>)> = reopened
            .document
            .tiles
            .ids()
            .map(|id| {
                (
                    id,
                    reopened
                        .document
                        .tiles
                        .get(id)
                        .expect("bank")
                        .pixels()
                        .to_vec(),
                )
            })
            .collect();
        assert_eq!(bank, reopened_bank, "{name} bank survives the round trip");
    }
}

#[test]
fn a_malformed_tile_stream_refuses_instead_of_silently_dropping() {
    // A file whose mkBT chunk cannot parse: open refuses (the bank
    // cannot be joined), so the dangling-reference file never loads.
    let mut rng = FuzzRng::new(5);
    let document = random_document(&mut rng);
    let (tree, _) =
        doc::fwmap::write(&document, &doc::fwmap::UnknownBag::default()).expect("writes");
    let mut file = Vec::new();
    file.extend_from_slice(&fwpng::chunk::PNG_SIGNATURE);
    // The tree must be embedded as mkTS for open to get past the tree
    // stage before the tile check.
    let mut compressed = Vec::new();
    {
        let mut encoder =
            flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::default());
        use std::io::Write as _;
        encoder
            .write_all(&fwpng::nodetree::serialize(&tree))
            .expect("deflates");
        encoder.finish().expect("finishes");
    }
    fwpng::chunk::write_chunk(&mut file, b"mkTS", &compressed);
    fwpng::chunk::write_chunk(&mut file, b"mkBT", &[0xFA, 0xCE]); // truncated
    fwpng::chunk::write_chunk(&mut file, b"IEND", &[]);
    let Err(CliError(error)) = io::open(&file) else {
        panic!("malformed tiles must refuse");
    };
    assert!(error.contains("mkBT"), "{error}");
}
