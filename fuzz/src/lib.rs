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
    for cap in [
        vector::CapKind::Butt,
        vector::CapKind::Square,
        vector::CapKind::Round,
    ] {
        for join in [
            vector::JoinKind::Miter,
            vector::JoinKind::Bevel,
            vector::JoinKind::Round,
        ] {
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
            repeat: io::gif::Loop::Times(3),
            dither: 0,
        },
    ];
    for option in &options {
        if let Ok(out) = io::gif::encode_animated(&imported.document, 0, option) {
            let _ = io::gif::decode_animated(&out);
        }
    }
}

/// Untrusted node-tree payload driven through the `EXPv` mapping: the
/// export-options record and frame durations must survive
/// read → write → re-read unchanged, and every codec-option derivation
/// must stay in range for whatever the bytes mapped (totality is the
/// contract; panic is the failure signal).
pub fn anchor_expv(data: &[u8]) {
    let Ok(nodes) = fwpng::nodetree::parse(data) else {
        return;
    };
    let Ok((document, bag)) = doc::fwmap::read(&nodes) else {
        return;
    };
    let doc = &document;
    let export = io::expv::for_page(&document, 0).cloned();
    let gif = io::expv::gif_options(export.as_ref());
    assert!(gif.dither <= 100, "dither out of range: {}", gif.dither);
    let jpeg = io::expv::jpeg_options(export.as_ref(), io::expv::StatedJpeg::default());
    assert!(
        (1..=100).contains(&jpeg.quality),
        "jpeg quality out of range: {}",
        jpeg.quality
    );
    let _ = io::expv::png_options(export.as_ref());

    if let Ok((tree, _stats)) = doc::fwmap::write(&document, &bag) {
        let bytes = fwpng::nodetree::serialize(&tree);
        if let Ok(nodes2) = fwpng::nodetree::parse(&bytes)
            && let Ok((doc2, _bag2)) = doc::fwmap::read(&nodes2)
        {
            assert_eq!(doc.export, doc2.export, "export options shifted on write");
            let durations = doc
                .pages
                .first()
                .map(|page| page.info.frame_durations.clone());
            let durations2 = doc2
                .pages
                .first()
                .map(|page| page.info.frame_durations.clone());
            assert_eq!(durations, durations2, "frame durations shifted on write");
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

// --- M9 task 8: script mutation ---

/// Rising-only floor: script cases (baselines + mutants) one full
/// driver sweep must clear. Raise only with a sweep proving the higher
/// count stays typed-never-panic (8192 proven: 256 seeds x 32 cases,
/// 2026-10-12, release).
pub const SCRIPT_MUTATION_FLOOR: usize = 8192;

/// Mutated runs per seed (each seed also runs every seed script
/// unmutated as its baseline).
pub const SCRIPT_MUTANTS_PER_SEED: usize = 16;

/// Fuzz limits for script runs: the sandbox contract under test is
/// "every failure is a typed report", and the loop/recursion limits
/// are part of what mutants hammer — tighter bounds keep the debug
/// build's sweep cheap while still firing every limit path.
pub const FUZZ_LIMITS: scripting::Limits = scripting::Limits {
    loop_iterations: 100_000,
    recursion: 128,
    stack_bytes: 1 << 18,
    host_calls: 2_000,
};

/// Synthetic seed scripts (written here, owned here — no Adobe
/// material): they sweep the fw/dom surface, the Files host, control
/// marks/reverts, and every failure shape (throw, syntax error, limit
/// halt). Mutations of these are the untrusted corpus.
pub const SCRIPT_SEEDS: &[&str] = &[
    "fw.getDocumentDOM().addNewRectangle(2, 4, 40, 12, 'plate');",
    "var dom = fw.getDocumentDOM(); dom.addNewText(0, 0, 30, 10, 'hi', 'label');",
    "var dom = fw.getDocumentDOM(); dom.addNewLayer(null, false); dom.deleteLayer(0);",
    "var dom = fw.getDocumentDOM(); dom.selectAllOnLayer(0); dom.pathUnion();",
    "var dom = fw.getDocumentDOM(); dom.addNewRectangle(0, 0, 8, 8, null); \
     dom.clipCopy(); dom.addNewLayer(null, false); dom.clipPaste();",
    "var t = Files.read('data.txt'); fw.getDocumentDOM().addNewText(0, 0, 40, 10, t, null);",
    "fw.getDocumentDOM().addNewRectangle(1, 1, 5, 5, 'a'); \
     dom = null; fw.getDocumentDOM().clipCut();",
    "var dom = fw.getDocumentDOM(); dom.addNewRectangle(0, 0, 3, 3, 'm'); dom.mark(); \
     dom.addNewRectangle(9, 9, 3, 3, 'n'); dom.revert();",
    "throw new Error('aborted mid-run');",
    "var dom = fw.getDocumentDOM(",
    "while (true) { fw.getDocumentDOM().addNewRectangle(1, 2, 3, 4); }",
    "function f(n) { return n === 0 ? 0 : f(n - 1); } f(9999);",
    "fw.noSuchApi();",
    "var x = ",
    "fw.getDocumentDOM().addNewRectangle('a', 'b', 'c', 'd', 'e');",
    "fw.getDocumentDOM().addNewRectangle(0, 0, 10, 10, null); // trailing",
];

/// The byte-level mutation set, shared by the script and JSON anchors.
fn mutate_bytes(rng: &mut engine::fuzzdrive::FuzzRng, bytes: &mut Vec<u8>) {
    const TOKENS: &[&[u8]] = &[
        b"{",
        b"}",
        b"(",
        b")",
        b"[",
        b"]",
        b";",
        b",",
        b"'",
        b"\"",
        b"\\",
        b"+",
        b"-",
        b"/",
        b"*",
        b"=",
        b"<",
        b">",
        b"!",
        b"&",
        b"|",
        b"%",
        b"$",
        b"#",
        b"\n",
        b"\t",
        b" ",
        b"0",
        b"9",
        b"fw",
        b"dom",
        b"null",
        b"true",
        b"function",
        b"while",
        b"return",
        b"command",
        b"args",
        b"method",
        b"params",
        b"id",
        b"jsonrpc",
        b"name",
        b"arguments",
    ];
    match rng.below(5) {
        0 => {
            // Flip one byte.
            if !bytes.is_empty() {
                let at = rng.below(bytes.len());
                bytes[at] ^= 0x5A;
            }
        }
        1 => {
            // Delete a short run.
            if bytes.len() > 8 {
                let at = rng.below(bytes.len() - 4);
                let take = 1 + rng.below(8);
                bytes.drain(at..(at + take).min(bytes.len()));
            }
        }
        2 => {
            // Duplicate a short run to the end.
            if bytes.len() > 4 {
                let at = rng.below(bytes.len() - 2);
                let take = 1 + rng.below(6);
                let end = (at + take).min(bytes.len());
                let slice = bytes[at..end].to_vec();
                bytes.extend_from_slice(&slice);
            }
        }
        3 => {
            // Splice a token in.
            let token: Vec<u8> = rng.pick(TOKENS).copied().unwrap_or(b"?").to_vec();
            let at = rng.below(bytes.len() + 1);
            bytes.splice(at..at, token);
        }
        _ => {
            // Truncate.
            if !bytes.is_empty() {
                let keep = rng.below(bytes.len());
                bytes.truncate(keep);
            }
        }
    }
}

/// One script-mutation case: run `source` through the scripting host
/// against a fresh session. The contract:
///
/// 1. the run ends in a typed [`scripting::ScriptReport`] — a panic is
///    the failure signal (the driver catches and reports);
/// 2. a limit-halted run leaves the document byte-identical — limits
///    fire during interpretation, before any intent drains;
/// 3. any other outcome (completed, threw before the drain, or threw
///    mid-drain where the host deliberately keeps the applied prefix
///    and reports the failing intent) leaves a journal that replays
///    onto a pristine copy of the base document to exactly the current
///    bytes — the universal integrity invariant.
pub fn anchor_script_mutation(seed: u64, case: usize, source: &str) {
    use engine::platform::{FileSystem, MemoryFs};

    let base =
        engine::fuzzdrive::random_document(&mut engine::fuzzdrive::FuzzRng::new(seed ^ 0x5C1_0000));
    let base_bytes = match serde_json::to_vec(&base) {
        Ok(bytes) => bytes,
        Err(error) => panic!("seed {seed} case {case}: base serialize broke: {error}"),
    };
    let mut session = engine::Session::new(base);
    let mut script = match scripting::ScriptEngine::new(FUZZ_LIMITS) {
        Ok(script) => script,
        Err(error) => panic!("seed {seed} case {case}: engine build failed: {error}"),
    };
    let fs: std::rc::Rc<dyn FileSystem> = std::rc::Rc::new(MemoryFs::new());
    let _ = fs.write_file("data.txt", b"seed data");
    let report = scripting::run_with_file_system(fs, || {
        scripting::run_in_session(&mut session, &mut script, "mutant.jsf", source)
    });
    if report.outcome == scripting::Outcome::LimitExceeded {
        let after = match serde_json::to_vec(session.document()) {
            Ok(bytes) => bytes,
            Err(error) => panic!("seed {seed} case {case}: serialize broke: {error}"),
        };
        assert_eq!(
            after, base_bytes,
            "seed {seed} case {case}: a limit-halted run mutated the document"
        );
        return;
    }
    // Completed or threw (pre- or mid-drain): the journal replays onto
    // a pristine copy of the base to exactly the current document.
    let lines = session.journal_lines();
    let Ok(replay_base) = serde_json::from_slice::<doc::Document>(&base_bytes) else {
        panic!("seed {seed} case {case}: base deserialize broke");
    };
    let replayed = match engine::Session::reopen(replay_base, &lines) {
        Ok(replayed) => replayed,
        Err(error) => panic!("seed {seed} case {case}: replay broke: {error}"),
    };
    let live = match serde_json::to_vec(session.document()) {
        Ok(bytes) => bytes,
        Err(error) => panic!("seed {seed} case {case}: serialize broke: {error}"),
    };
    let rebuilt = match serde_json::to_vec(replayed.document()) {
        Ok(bytes) => bytes,
        Err(error) => panic!("seed {seed} case {case}: serialize broke: {error}"),
    };
    assert_eq!(
        live, rebuilt,
        "seed {seed} case {case}: journal replay diverged from the live run"
    );
}

/// The mutator: `0..mutations` byte-level edits over one seed script.
/// Returns the mutated source (lossy UTF-8 — mutation may split
/// characters; the host takes `&str`).
pub fn mutated_script(seed: u64, case: usize) -> String {
    let mut rng = engine::fuzzdrive::FuzzRng::new(seed ^ ((case as u64) << 24) ^ 0x5C1);
    let seed_script = SCRIPT_SEEDS[case % SCRIPT_SEEDS.len()];
    let mut bytes = seed_script.as_bytes().to_vec();
    let edits = 1 + rng.below(4);
    for _ in 0..edits {
        mutate_bytes(&mut rng, &mut bytes);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

// --- M9 task 8: control channel ---

/// Rising-only floor: hostile requests one full channel sweep must
/// clear across both dispatchers (4096 proven: 256 seeds x 16 cases,
/// 2026-10-12, release).
pub const CONTROL_CHANNEL_FLOOR: usize = 4096;

/// Requests per seed (a deterministic fraction are valid traffic).
pub const CHANNEL_CASES_PER_SEED: usize = 16;

/// A valid control-channel request shape (the mutator's base).
const CHANNEL_TEMPLATE: &str = r#"{"command":"object.createPath","args":{"kind":"rect","x":5,"y":6,"width":20,"height":10,"page":0,"layer":0,"cell":0,"name":"c"}}"#;

/// A valid MCP tools/call envelope (the mutator's base).
const MCP_TEMPLATE: &str = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"firecraft.apply","arguments":{"command":"object.rename","args":{"id":{"page":0,"layer":0,"cell":0,"index":0},"name":"renamed"}}}}"#;

/// Random junk JSON: the structural garbage the transport may receive.
fn junk_json(rng: &mut engine::fuzzdrive::FuzzRng, depth: usize) -> serde_json::Value {
    const KEYS: &[&str] = &[
        "command",
        "args",
        "method",
        "params",
        "id",
        "jsonrpc",
        "name",
        "arguments",
        "result",
        "error",
        "code",
        "path",
        "",
        "firecraft.apply",
        "tools/call",
    ];
    if depth == 0 || rng.chance(40) {
        return match rng.below(5) {
            0 => serde_json::Value::Null,
            1 => serde_json::json!(rng.below(2) == 1),
            2 => serde_json::json!(rng.below(1000) as i64 - 500),
            3 => serde_json::json!(format!("junk-{}", rng.below(1000))),
            _ => serde_json::json!(rng.below(1 << 30) as f64 / 7.0),
        };
    }
    if rng.chance(50) {
        let len = rng.below(4);
        let mut items = Vec::new();
        for _ in 0..len {
            items.push(junk_json(rng, depth - 1));
        }
        return serde_json::Value::Array(items);
    }
    let len = rng.below(4);
    let mut map = serde_json::Map::new();
    for _ in 0..len {
        let key = rng.pick(KEYS).copied().unwrap_or("k").to_owned();
        map.insert(key, junk_json(rng, depth - 1));
    }
    serde_json::Value::Object(map)
}

/// One control-channel case against both dispatchers. The contract:
///
/// 1. `channel::handle` answers `{ok: bool, ...}` — `error` is a
///    string when `ok` is false, and an accepted request grows the
///    journal by exactly one;
/// 2. `mcp::handle` answers `None` only for `notifications/*` and
///    otherwise a well-formed JSON-RPC envelope (exactly one of
///    `error` / `result`);
/// 3. neither dispatcher ever panics (the driver catches), and the
///    journal never shrinks.
pub fn anchor_control_channel(seed: u64, case: usize) {
    use app_cli::{channel, mcp};
    use engine::platform::{FileSystem, MemoryFs};

    let mut rng = engine::fuzzdrive::FuzzRng::new(seed ^ ((case as u64) << 24) ^ 0xC4A);
    let base =
        engine::fuzzdrive::random_document(&mut engine::fuzzdrive::FuzzRng::new(seed ^ 0xC4A_0000));
    let mut session = engine::Session::new(base);
    let fs: std::rc::Rc<dyn FileSystem> = std::rc::Rc::new(MemoryFs::new());

    // Roughly one case in eight is unmuted valid traffic so the
    // accepted paths stay exercised amid the garbage.
    let valid = case.is_multiple_of(8);
    let template = if rng.chance(50) {
        MCP_TEMPLATE
    } else {
        CHANNEL_TEMPLATE
    };
    let request = if valid {
        template.to_owned()
    } else {
        let mut bytes = template.as_bytes().to_vec();
        let edits = 1 + rng.below(6);
        for _ in 0..edits {
            mutate_bytes(&mut rng, &mut bytes);
        }
        // Sometimes pure junk instead of a mutated template.
        if rng.chance(30) {
            return; // junk-JSON cases are driven as parsed values below
        }
        String::from_utf8_lossy(&bytes).into_owned()
    };

    match serde_json::from_str::<serde_json::Value>(&request) {
        Err(_) => {
            // The mcp verb's transport-parse path: always the typed
            // -32700 envelope.
            let parse = mcp::parse_error(&request);
            assert_eq!(parse["jsonrpc"], "2.0");
            assert_eq!(parse["error"]["code"], -32700);
        }
        Ok(value) => {
            // Channel dispatcher.
            let before = session.journal_len();
            let reply = channel::handle(&mut session, &value);
            let ok = reply.get("ok").and_then(serde_json::Value::as_bool);
            match ok {
                Some(false) => assert!(
                    reply.get("error").is_some_and(serde_json::Value::is_string),
                    "seed {seed} case {case}: refusal without a string error: {reply}"
                ),
                Some(true) => {
                    let grew = session.journal_len() - before;
                    assert_eq!(
                        grew, 1,
                        "seed {seed} case {case}: accepted request grew the journal by {grew}"
                    );
                }
                other => {
                    panic!("seed {seed} case {case}: untyped channel reply {other:?}: {reply}")
                }
            }
            assert!(
                session.journal_len() >= before,
                "seed {seed} case {case}: the journal shrank"
            );

            // MCP dispatcher over the same value.
            if let Some(reply) = mcp::handle(&mut session, fs.clone(), &value) {
                assert_eq!(reply["jsonrpc"], "2.0", "seed {seed} case {case}: {reply}");
                let has_error = reply.get("error").is_some_and(serde_json::Value::is_object);
                let has_result = reply
                    .get("result")
                    .is_some_and(serde_json::Value::is_object);
                assert!(
                    has_error ^ has_result,
                    "seed {seed} case {case}: envelope must carry exactly one of error/result: {reply}"
                );
            }
        }
    }
}

/// The junk-JSON arm: parsed structural garbage straight into both
/// dispatchers (no transport parse step to hide behind).
pub fn anchor_control_channel_junk(seed: u64, case: usize) {
    use app_cli::{channel, mcp};
    use engine::platform::{FileSystem, MemoryFs};

    let base =
        engine::fuzzdrive::random_document(&mut engine::fuzzdrive::FuzzRng::new(seed ^ 0x0DD_0000));
    let mut session = engine::Session::new(base);
    let fs: std::rc::Rc<dyn FileSystem> = std::rc::Rc::new(MemoryFs::new());
    let mut rng = engine::fuzzdrive::FuzzRng::new(seed ^ ((case as u64) << 24) ^ 0xDD0);
    let value = junk_json(&mut rng, 3);
    let reply = channel::handle(&mut session, &value);
    let ok = reply.get("ok").and_then(serde_json::Value::as_bool);
    assert!(
        ok == Some(true) || ok == Some(false),
        "seed {seed} case {case}: untyped channel reply: {reply}"
    );
    if let Some(reply) = mcp::handle(&mut session, fs, &value) {
        assert_eq!(reply["jsonrpc"], "2.0", "seed {seed} case {case}: {reply}");
        let has_error = reply.get("error").is_some_and(serde_json::Value::is_object);
        let has_result = reply
            .get("result")
            .is_some_and(serde_json::Value::is_object);
        assert!(
            has_error ^ has_result,
            "seed {seed} case {case}: envelope must carry exactly one of error/result: {reply}"
        );
    }
}
