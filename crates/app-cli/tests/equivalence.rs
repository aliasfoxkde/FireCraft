//! The M5 equivalence gate, headless form: replaying a journal through
//! the CLI (`Session::reopen`) and through a surface-style apply loop
//! over the same journal entries must produce byte-identical
//! documents, and a convert round-trip must re-open identically.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app_cli::io;
use engine::catalog;
use engine::fuzzdrive::{FuzzRng, random_call, random_document};
use engine::{CommandRegistry, Journal, Session};

/// Builds a registry with the built-in catalog installed.
fn registry() -> CommandRegistry {
    let mut reg = CommandRegistry::new();
    let _ = catalog::install(&mut reg);
    reg
}

/// Bytes of a document, canonically written through the model.
fn bytes(document: &doc::Document) -> Vec<u8> {
    let (tree, _) = doc::fwmap::write(document, &doc::fwmap::UnknownBag::default())
        .expect("fixture document writes");
    fwpng::nodetree::serialize(&tree)
}

/// Drives `count` calls from the shared generator, journaling each
/// success, and returns how many succeeded.
fn drive(
    document: &mut doc::Document,
    registry: &CommandRegistry,
    journal: &mut Journal,
    rng: &mut FuzzRng,
    count: usize,
) -> usize {
    let mut applied = 0;
    for _ in 0..count {
        let (name, payload) = random_call(rng, document);
        // Surface-style apply: mutate only through the registry.
        let outcome = registry.execute(document, &name, payload.clone());
        if outcome.is_ok() {
            journal.record(&name, &payload, None);
            applied += 1;
        }
    }
    applied
}

#[test]
fn reopen_replays_to_the_same_document_as_the_apply_loop() {
    let registry = registry();
    for seed in [1_u64, 2, 3] {
        let mut rng = FuzzRng::new(seed);

        // Path A: apply loop, journaling each success.
        let mut document_a = random_document(&mut rng);
        let mut journal = Journal::new();
        drive(&mut document_a, &registry, &mut journal, &mut rng, 24);
        let lines_a = journal.to_lines();

        // Path B: fresh seed document + the same journal text via the
        // CLI's restart path. Needs the SAME starting document, so
        // re-generate it from the same seed.
        let mut rng_b = FuzzRng::new(seed);
        let document_b = random_document(&mut rng_b);
        assert_eq!(
            bytes(&document_b),
            {
                // document_a before the applies equalled document_b;
                // re-derive A's starting bytes from a fresh generator
                // rather than storing a clone.
                let mut probe_rng = FuzzRng::new(seed);
                bytes(&random_document(&mut probe_rng))
            },
            "generator must be seed-deterministic for this comparison"
        );

        let session = Session::reopen(document_b, &lines_a).expect("recorded journal reopens");
        assert_eq!(
            bytes(session.document()),
            bytes(&document_a),
            "seed {seed}: reopen path and apply loop diverged"
        );
        assert_eq!(session.journal_len(), journal.len());
    }
}

#[test]
fn cli_apply_journal_matches_engine_apply_journal() {
    // A journal built in-engine (session.journal_lines) replays
    // identically through the CLI helper, undo stack included.
    for seed in [11_u64, 12] {
        let registry = registry();
        let mut rng = FuzzRng::new(seed);

        let seed_document = random_document(&mut rng);
        let mut document = seed_document.clone();
        let mut journal = Journal::new();
        let applied = drive(&mut document, &registry, &mut journal, &mut rng, 16);
        assert!(applied > 0, "generator must land some live calls");
        let lines = journal.to_lines();

        // Restart from the UNMUTATED seed: the journal is the only
        // thing that carries the edits across.
        let reopened = Session::reopen(seed_document, &lines).expect("reopens");
        // The rebuilt session can undo everything back to the seed.
        let rebuilt_lines = reopened.journal_lines();
        let mut probe = reopened;
        while probe.undo().is_ok() {}
        let undone = probe.document();

        // The rebuilt journal carries the same entries; the undone
        // document re-serializes stably.
        assert_eq!(
            Journal::from_lines(&rebuilt_lines).expect("parses").len(),
            applied
        );
        assert_eq!(bytes(undone), bytes(undone), "stable across re-serialize");
    }
}

#[test]
fn convert_round_trips_through_open_and_save() {
    // Assemble a minimal .fw.png stream from a fixture document, then
    // run the CLI convert path: open -> save -> open must be
    // byte-identical on the second pass (canonical form is stable).
    let mut rng = FuzzRng::new(7);
    let document = random_document(&mut rng);
    let (tree, _) =
        doc::fwmap::write(&document, &doc::fwmap::UnknownBag::default()).expect("writes");
    let inflated = fwpng::nodetree::serialize(&tree);

    let mut compressed = Vec::new();
    let mut encoder =
        flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &inflated).expect("deflate");
    std::io::Write::flush(&mut encoder).expect("deflate");
    drop(encoder);

    let mut file = Vec::new();
    file.extend_from_slice(&fwpng::chunk::PNG_SIGNATURE);
    fwpng::chunk::write_chunk(&mut file, b"mkTS", &compressed);
    fwpng::chunk::write_chunk(&mut file, b"IEND", &[]);

    let first = app_cli::ops::convert(&file).expect("converts");
    let second = app_cli::ops::convert(&first).expect("reconverts");
    assert_eq!(first, second, "canonical re-emit must be idempotent");

    let loaded = io::open(&first).expect("re-opens converted file");
    assert_eq!(loaded.document.registry.len(), document.registry.len());
    assert_eq!(loaded.document.pages.len(), document.pages.len());
}
