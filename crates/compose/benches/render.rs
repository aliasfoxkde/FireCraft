//! M4 baseline: compose throughput over the corpus golden fixture.
//!
//! Two scenarios: the full pipeline (PNG chunks → node tree → document
//! → pixels) and compose alone (document → pixels), so later milestones
//! can tell renderer regressions from parser regressions. The fixture
//! is 16×16 — small by design; these numbers are the baseline the
//! devlog records, not a performance claim.

use compose::render;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::io::Read as _;
use std::path::PathBuf;

/// The committed golden fixture, resolved from this crate's directory.
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/fixtures/composed.fw.png")
}

/// Parses the fixture into a document, or returns `None` when the
/// corpus is missing (benches skip instead of panicking).
fn load_document() -> Option<doc::Document> {
    let bytes = std::fs::read(fixture()).ok()?;
    let chunks = fwpng::chunk::parse_chunks(&bytes).ok()?;
    let chunk = chunks.iter().find(|c| &c.ctype == b"mkTS")?;
    let mut inflated = Vec::new();
    flate2::read::ZlibDecoder::new(&chunk.data[..])
        .read_to_end(&mut inflated)
        .ok()?;
    let nodes = fwpng::nodetree::parse(&inflated).ok()?;
    let (document, _bag) = doc::fwmap::read(&nodes).ok()?;
    Some(document)
}

fn bench_render(c: &mut Criterion) {
    let Some(document) = load_document() else {
        eprintln!(
            "[bench] composed.fw.png not found — run `cargo run -p xtask -- corpus-gen`; skipping"
        );
        return;
    };

    c.bench_function("compose/render/16x16", |b| {
        b.iter(|| render(black_box(&document), 0, 0).ok())
    });

    // Full pipeline for comparison: parse + read + render.
    let bytes = match std::fs::read(fixture()) {
        Ok(bytes) => bytes,
        Err(_) => return,
    };
    c.bench_function("pipeline/parse-read-render/16x16", |b| {
        b.iter(|| {
            let rendered = fwpng::chunk::parse_chunks(&bytes).ok().and_then(|chunks| {
                chunks
                    .iter()
                    .find(|c| &c.ctype == b"mkTS")
                    .and_then(|chunk| {
                        let mut inflated = Vec::new();
                        if flate2::read::ZlibDecoder::new(&chunk.data[..])
                            .read_to_end(&mut inflated)
                            .is_err()
                        {
                            return None;
                        }
                        fwpng::nodetree::parse(&inflated)
                            .ok()
                            .and_then(|nodes| doc::fwmap::read(&nodes).ok())
                    })
            });
            match rendered {
                Some((document, _)) => render(black_box(&document), 0, 0).ok(),
                None => None,
            }
        })
    });
}

criterion_group!(benches, bench_render);
criterion_main!(benches);
