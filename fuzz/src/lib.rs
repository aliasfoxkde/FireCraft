//! Shared fuzz anchors: one function per hostile input surface.
//!
//! The contract for every anchor is "must not panic" — parse errors are
//! expected outcomes and are discarded. Both the stable driver
//! (`src/bin/anchors.rs`) and the libfuzzer targets (`fuzz_targets/`)
//! call these, so a crash found by either is reproducible by the other.

/// Untrusted byte stream interpreted as a PNG chunk sequence.
pub fn anchor_fwpng_chunks(data: &[u8]) {
    let _ = fwpng::parse_chunks(data);
}

/// Untrusted node-tree payload; on success the canonical serializer must
/// also round-trip (parse(serialize(nodes)) parses).
pub fn anchor_fwpng_nodetree(data: &[u8]) {
    if let Ok(nodes) = fwpng::nodetree::parse(data) {
        let round_tripped = fwpng::nodetree::serialize(&nodes);
        let _ = fwpng::nodetree::parse(&round_tripped);
    }
}

/// Untrusted node-tree payload driven through the typed document model:
/// mapping must never panic (typed errors only), and on success the
/// write -> serialize -> parse -> read cycle must hold together.
pub fn anchor_doc_read(data: &[u8]) {
    if let Ok(nodes) = fwpng::nodetree::parse(data)
        && let Ok((document, bag)) = doc::fwmap::read(&nodes)
        && let Ok((tree, _stats)) = doc::fwmap::write(&document, &bag)
    {
        let bytes = fwpng::nodetree::serialize(&tree);
        if let Ok(nodes2) = fwpng::nodetree::parse(&bytes) {
            let _ = doc::fwmap::read(&nodes2);
        }
    }
}

/// Untrusted `prVW` thumbnail payload.
pub fn anchor_fwpng_prvw(data: &[u8]) {
    let _ = fwpng::thumbnail::parse_prvw(data);
}

/// Untrusted `mkBT` tile payload.
pub fn anchor_fwpng_mkbt(data: &[u8]) {
    let _ = fwpng::tile::parse_mkbt(data);
}

/// 96+ arbitrary bytes interpreted as two affine transforms (6 f64 each,
/// little endian) and a rect; map_rect and inverse must never panic,
/// whatever bit patterns the bytes hold (NaN, infinities, subnormals).
pub fn anchor_geom_transform(data: &[u8]) {
    let mut vals = [0.0f64; 12];
    for (i, val) in vals.iter_mut().enumerate() {
        let start = i * 8;
        if start + 8 <= data.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data[start..start + 8]);
            *val = f64::from_le_bytes(bytes);
        }
    }
    let a = geom::Affine::new([vals[0], vals[1], vals[2], vals[3], vals[4], vals[5]]);
    let rect = geom::Rect::new(vals[6], vals[7], vals[8], vals[9]);
    let _ = geom::transform::map_rect(a, rect);
    let _ = a.inverse();
}

/// 256 bytes as 32 f64s: two closed 4-anchor paths (8 points). Every
/// boolean op, both offset directions, and every cap/join combination
/// must either succeed or return a typed error — never panic — and the
/// boolean pipeline must be byte-deterministic (same input, same output
/// bytes, twice).
pub fn anchor_vector_ops(data: &[u8]) {
    let mut vals = [0.0f64; 16];
    for (i, val) in vals.iter_mut().enumerate() {
        let start = i * 8;
        if start + 8 <= data.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data[start..start + 8]);
            *val = f64::from_le_bytes(bytes);
        }
    }
    let quad = |base: usize| doc::PathObject {
        contours: vec![doc::Contour {
            closed: true,
            point_count: 4,
            points: (0..4)
                .map(|k| doc::PathPoint {
                    anchor: geom::Point::new(vals[base + k * 2], vals[base + k * 2 + 1]),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                })
                .collect(),
        }],
        primitive: None,
        even_odd: false,
        paint: None,
        fill_start: None,
        fill_end: None,
        fill_third: None,
        brush_placement: None,
        fill_on_top: None,
        locked: false,
        visible: true,
        opacity: 1000,
        blend: 0,
        name: None,
        effects: Vec::new(),
    };
    let (a, b) = (quad(0), quad(8));
    for op in [
        vector::BooleanOp::Union,
        vector::BooleanOp::Intersect,
        vector::BooleanOp::Punch,
        vector::BooleanOp::Crop,
    ] {
        if let Ok(first) = vector::boolean(&a, &b, op) {
            let again = vector::boolean(&a, &b, op).unwrap_or_else(|error| {
                panic!("boolean lost determinism on retry: {error}");
            });
            let encode = |path: &doc::PathObject| -> Vec<u8> {
                let mut out = Vec::new();
                for contour in &path.contours {
                    for point in &contour.points {
                        out.extend_from_slice(&point.anchor.x.to_le_bytes());
                        out.extend_from_slice(&point.anchor.y.to_le_bytes());
                        out.push(u8::from(point.is_curve));
                    }
                }
                out
            };
            assert_eq!(
                encode(&first),
                encode(&again),
                "boolean({op:?}) is not byte-deterministic"
            );
        }
    }
    for distance in [vals[0], -vals[0].abs()] {
        let _ = vector::offset_path(&a, distance);
    }
    for cap in [vector::CapKind::Butt, vector::CapKind::Square, vector::CapKind::Round] {
        for join in [vector::JoinKind::Miter, vector::JoinKind::Bevel, vector::JoinKind::Round] {
            let _ = vector::expand_stroke(
                &a,
                &vector::StrokeStyle {
                    width: vals[0].abs().max(1.0),
                    cap,
                    join,
                    miter_limit: vector::DEFAULT_MITER_LIMIT,
                },
            );
        }
    }
}

/// Untrusted byte stream interpreted as an SVG document: the usvg
/// parse → model walk must refuse, import, or report — never panic,
/// hang, or overflow the stack (the importer's own depth ceiling and
/// canvas ceiling are part of the contract under test).
pub fn anchor_svgimport(data: &[u8]) {
    if let Ok(imported) = io::svgimport::decode(data) {
        // The imported document must render (or refuse) without
        // panicking too — compose is the first consumer of the walk.
        let _ = compose::render(&imported.document, 0, 0);
    }
}

/// Untrusted byte stream interpreted as a PSD: the header gates, the
/// flat record walk over both real-world channel layouts, and the
/// composite fallback must refuse, import, or report — never panic.
pub fn anchor_psdimport(data: &[u8]) {
    if let Ok(imported) = io::psd::decode(data) {
        // compose is the first consumer of the walk.
        let _ = compose::render(&imported.document, 0, 0);
    }
}

/// Untrusted byte stream interpreted as an animated GIF: the frame
/// loop, every composited state through compose, and the full
/// import→export→import round trip (both option paths — cropped diff
/// frames and full frames) must refuse, produce, or report — never
/// panic.
pub fn anchor_gifanim(data: &[u8]) {
    let Ok(imported) = io::gif::decode_animated(data) else {
        return;
    };
    for frame in 0..imported.report.frames {
        let _ = compose::render(&imported.document, 0, frame);
    }
    let options = [
        io::gif::GifOptions::default(),
        io::gif::GifOptions {
            crop: true,
            diff: true,
            repeat: Some(3),
        },
    ];
    for option in &options {
        if let Ok(out) = io::gif::encode_animated(&imported.document, 0, option) {
            let _ = io::gif::decode_animated(&out);
        }
    }
}

/// How many random catalog calls one seed drives.
pub const HUNT_STEPS: usize = 48;

/// panic_hunt: drives the full built-in command catalog with random
/// (sometimes corrupt) payloads against a random document. The contract
/// — violated by panic, which is the failure signal:
///
/// 1. a command either succeeds or returns an error, never panics;
/// 2. a failed call leaves the document byte-identical;
/// 3. a successful call undoes back to the prior bytes and redoes
///    forward again;
/// 4. the journal replays onto a fresh copy of the seed document, and
///    the replayed undo stack reaches the original again.
///
/// `seed` is the only input: the stable driver enumerates seeds, the
/// libfuzzer target derives one from the input bytes, so any crash
/// either finds is reproducible by both.
pub fn anchor_engine_commands(seed: u64) {
    use engine::Session;

    let mut rng = engine::fuzzdrive::FuzzRng::new(seed);
    let document = engine::fuzzdrive::random_document(&mut rng);
    let Ok(original) = serde_json::to_vec(&document) else {
        panic!("seed {seed}: the random document failed to serialize");
    };
    let mut session = Session::new(document);

    for step in 0..HUNT_STEPS {
        let (name, payload) = engine::fuzzdrive::random_call(&mut rng, session.document());
        let before = document_bytes(session.document(), seed, step, &name);
        match session.apply(&name, payload) {
            Err(_) => {
                let after = document_bytes(session.document(), seed, step, &name);
                assert_eq!(
                    after, before,
                    "seed {seed} step {step}: failed call {name} mutated the document"
                );
            }
            Ok(()) => {
                let applied = document_bytes(session.document(), seed, step, &name);
                session
                    .undo()
                    .unwrap_or_else(|error| panic!("seed {seed} step {step}: undo broke: {error}"));
                let undone = document_bytes(session.document(), seed, step, &name);
                assert_eq!(
                    undone, before,
                    "seed {seed} step {step}: undo diverged after {name}"
                );
                session
                    .redo()
                    .unwrap_or_else(|error| panic!("seed {seed} step {step}: redo broke: {error}"));
                let redone = document_bytes(session.document(), seed, step, &name);
                assert_eq!(
                    redone, applied,
                    "seed {seed} step {step}: redo diverged after {name}"
                );
            }
        }
    }

    // Replay: same journal onto a fresh copy of the seed document.
    let lines = session.journal_lines();
    let edited = document_bytes(session.document(), seed, HUNT_STEPS, "replay");
    let Ok(fresh) = serde_json::from_slice::<doc::Document>(&original) else {
        panic!("seed {seed}: the seed document failed to deserialize");
    };
    let mut restarted = Session::reopen(fresh, &lines)
        .unwrap_or_else(|error| panic!("seed {seed}: journal replay broke: {error}"));
    assert_eq!(
        document_bytes(restarted.document(), seed, HUNT_STEPS, "replay"),
        edited,
        "seed {seed}: replay diverged from the live session"
    );
    while restarted.undo_len() > 0 {
        restarted
            .undo()
            .unwrap_or_else(|error| panic!("seed {seed}: replayed undo broke: {error}"));
    }
    let undone_all = document_bytes(restarted.document(), seed, HUNT_STEPS, "replay-undo");
    assert_eq!(
        undone_all, original,
        "seed {seed}: replayed undo did not reach the original"
    );
}

/// Serializes the document for a contract check; a serialization
/// failure is itself a panic (the failure signal).
fn document_bytes(document: &doc::Document, seed: u64, step: usize, name: &str) -> Vec<u8> {
    match serde_json::to_vec(document) {
        Ok(bytes) => bytes,
        Err(error) => panic!("seed {seed} step {step} ({name}): serialize broke: {error}"),
    }
}
