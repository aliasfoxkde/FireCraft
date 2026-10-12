//! Corpus gate: pins deterministic fixtures and verifies them.
//!
//! Two kinds of corpus entries live in `corpus/fixtures/`:
//!
//! - **Generated fixtures** (committed): built by [`gen`] with our own
//!   writers — clean-room bytes, no Adobe IP. They pin current parse
//!   behavior as regression anchors and exercise the round-trip floor.
//! - **Real Fireworks files** (never committed): dropped into
//!   `corpus/incoming/` by the operator; the manifest may pin their
//!   digests, the files stay out of the repo.
//!
//! `MANIFEST.tsv` lines: `<sha256> <relative-path> [# note]`. The gate
//! verifies digests, then runs the round-trip battery on every entry and
//! fails when the pass rate drops below [`ROUNDTRIP_FLOOR`].

use crate::util::{inflate, sha256_hex, workspace_root};
use fwpng::chunk::{is_fireworks, parse_chunks, write_chunk};
use std::io::Write as _;
use std::path::Path;

/// Minimum share (percent) of corpus entries that must pass the
/// round-trip battery. Rising-only: raise as fidelity grows, never lower.
pub const ROUNDTRIP_FLOOR: usize = 100;

const FIXTURE_DIR: &str = "corpus/fixtures";
const MANIFEST: &str = "corpus/MANIFEST.tsv";

/// Gate entry point: verify digests + round-trip battery.
pub fn run() -> Result<(), String> {
    verify(false)
}

/// `xtask corpus --report`: per-entry detail on stderr.
pub fn report() -> Result<(), String> {
    verify(true)
}

/// Parsed manifest: (pinned sha256, workspace-relative path) pairs,
/// comments and blanks skipped. Shared by the corpus and golden gates.
pub fn manifest_entries() -> Result<Vec<(String, String)>, String> {
    let manifest = workspace_root().join(MANIFEST);
    if !manifest.exists() {
        return Err(format!(
            "{MANIFEST} is missing; run `cargo run -p xtask -- corpus-gen`"
        ));
    }
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("read manifest: {e}"))?;
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match (
            line.split_whitespace().next(),
            line.split_whitespace().nth(1),
        ) {
            (Some(want), Some(rel)) => entries.push((want.to_owned(), rel.to_owned())),
            _ => return Err(format!("malformed manifest line: {line}")),
        }
    }
    Ok(entries)
}

fn verify(verbose: bool) -> Result<(), String> {
    let mut checked = 0usize;
    let mut passed = 0usize;
    let mut errors = Vec::new();
    for (want, rel) in manifest_entries()? {
        let bytes = match std::fs::read(workspace_root().join(&rel)) {
            Ok(b) => b,
            Err(err) => {
                errors.push(format!("{rel}: {err}"));
                continue;
            }
        };
        let got = sha256_hex(&bytes);
        if got != want {
            errors.push(format!("{rel}: sha256 {got} != pinned {want}"));
            continue;
        }
        checked += 1;
        let name = Path::new(&rel)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&rel);
        // The filename carries the detection contract: `*.fw.png` must be
        // recognized as Fireworks, plain `.png` must not.
        let checks = round_trip_checks(&bytes, rel.ends_with(".fw.png"));
        let failed: Vec<&String> = checks
            .iter()
            .filter(|(_, ok)| !ok)
            .map(|(m, _)| m)
            .collect();
        if failed.is_empty() {
            passed += 1;
            if verbose {
                eprintln!("[xtask] corpus: {name}: {} check(s) ok", checks.len());
            }
        } else {
            for msg in failed {
                errors.push(format!("{name}: {msg}"));
            }
        }
    }
    let floor_met = checked > 0 && passed * 100 >= checked * ROUNDTRIP_FLOOR;
    if errors.is_empty() && floor_met {
        eprintln!(
            "[xtask] corpus: {checked} file(s) verified, round-trip {passed}/{checked} (floor {ROUNDTRIP_FLOOR}%)"
        );
        Ok(())
    } else {
        if !floor_met {
            errors.push(format!(
                "round-trip floor not met: {passed}/{checked} below {ROUNDTRIP_FLOOR}%"
            ));
        }
        Err(errors.join("\n"))
    }
}

/// Round-trip battery for one file: what must hold for a clean
/// parse -> model -> re-emit cycle. Returns (check, passed) pairs.
fn round_trip_checks(bytes: &[u8], expect_fireworks: bool) -> Vec<(String, bool)> {
    let mut checks = Vec::new();
    let chunks = match parse_chunks(bytes) {
        Ok(c) => c,
        Err(err) => {
            checks.push((format!("parse_chunks: {err}"), false));
            return checks;
        }
    };
    let fw = is_fireworks(&chunks) == expect_fireworks;
    checks.push((format!("fireworks detection = {expect_fireworks}"), fw));

    // Every node-tree payload must canonically re-serialize to its own
    // inflated bytes, and the typed document model must round-trip
    // through it losslessly.
    for ctype in [b"mkTS", b"mkBS"] {
        for chunk in chunks.iter().filter(|c| &c.ctype == ctype) {
            let label = String::from_utf8_lossy(ctype).into_owned();
            match inflate(&chunk.data) {
                Ok(inflated) => node_tree_checks(&label, &inflated, &mut checks),
                Err(err) => checks.push((format!("{label} inflate: {err}"), false)),
            }
        }
    }
    // Tiles decode with exact stream size (only when tiles are present).
    if chunks.iter().any(|c| &c.ctype == b"mkBT") {
        match fwpng::parse_mkbt_chunks(&chunks) {
            Ok(tiles) => checks.push((format!("{} tile(s) decoded", tiles.len()), true)),
            Err(err) => checks.push((format!("mkBT decode: {err}"), false)),
        }
    }
    // Thumbnail payload parses.
    for chunk in chunks.iter().filter(|c| &c.ctype == b"prVW") {
        match fwpng::thumbnail::parse_prvw(&chunk.data) {
            Ok(_) => checks.push(("prVW parses".to_string(), true)),
            Err(err) => checks.push((format!("prVW parse: {err}"), false)),
        }
    }
    checks
}

/// Checks one inflated node-tree payload: canonical re-serialization,
/// then the doc-model floor — read -> write -> read must reproduce the
/// same [`doc::model::Document`], and the canonical write must be
/// idempotent byte-for-byte (nodes the model does not interpret ride in
/// the bag and re-bag on re-read; a byte-identical re-write proves they
/// survive the modeled edit cycle).
fn node_tree_checks(label: &str, inflated: &[u8], checks: &mut Vec<(String, bool)>) {
    let nodes = match fwpng::nodetree::parse(inflated) {
        Ok(nodes) => nodes,
        Err(err) => {
            checks.push((format!("{label} parse: {err}"), false));
            return;
        }
    };
    checks.push((
        format!("{label} canonical re-emit"),
        fwpng::nodetree::serialize(&nodes) == inflated,
    ));

    let (document, bag) = match doc::fwmap::read(&nodes) {
        Ok(pair) => pair,
        Err(err) => {
            checks.push((format!("{label} doc read: {err}"), false));
            return;
        }
    };
    let (tree2, stats) = match doc::fwmap::write(&document, &bag) {
        Ok(pair) => pair,
        Err(err) => {
            checks.push((format!("{label} doc write: {err}"), false));
            return;
        }
    };
    checks.push((
        format!("{label} doc write consumes bag"),
        stats.unconsumed_unknown == 0,
    ));
    let bytes2 = fwpng::nodetree::serialize(&tree2);
    match fwpng::nodetree::parse(&bytes2) {
        Ok(nodes2) => match doc::fwmap::read(&nodes2) {
            Ok((document2, bag2)) => {
                checks.push((
                    format!("{label} doc round-trip equal"),
                    document2 == document,
                ));
                checks.push((
                    format!("{label} doc write idempotent"),
                    match doc::fwmap::write(&document2, &bag2) {
                        Ok((tree3, _)) => fwpng::nodetree::serialize(&tree3) == bytes2,
                        Err(_) => false,
                    },
                ));
            }
            Err(err) => checks.push((format!("{label} doc re-read: {err}"), false)),
        },
        Err(err) => checks.push((format!("{label} re-parse written tree: {err}"), false)),
    }
}

/// Regenerates every fixture deterministically and rewrites the manifest
/// with fresh digests.
pub fn gen_fixtures() -> Result<(), String> {
    let dir = workspace_root().join(FIXTURE_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;

    /// One pending fixture: name, built bytes, manifest note.
    type Fixture = (&'static str, Result<Vec<u8>, String>, &'static str);
    let fixtures: [Fixture; 10] = [
        (
            "minimal.fw.png",
            build_minimal(),
            "generated: standard 8x8 RGBA + prVW + mkTS/mkBS (same tree) + mkBF + mkBT tile 0",
        ),
        (
            "tiles-split.fw.png",
            build_tiles_split(),
            "generated: mkBT tile 0 split across two chunks, tile 1 single",
        ),
        (
            "legacy-drift.fw.png",
            build_legacy_drift(),
            "generated: mkBS deliberately diverges from mkTS (comparison-helper target)",
        ),
        (
            "composed.fw.png",
            build_composed(),
            "generated: 16x16 page, red rect + 1000\u{2030} group holding a black half-canvas rect at 400\u{2030} (golden anchor)",
        ),
        (
            "scripted.fw.png",
            build_scripted(),
            "generated: 64x48 page, autoshape star + pie + spiral created through the engine registry (scripted-shape golden pin)",
        ),
        (
            "filtered.fw.png",
            build_filtered(),
            "generated: 64x48 page, autoshapes carrying live effect stacks through filter.add (M7 filter golden pin; dropShadow is the unfiltered pass-through control)",
        ),
        (
            "text.fw.png",
            build_text(),
            "generated: 64x48 page, one text object through text.createText/setAttrs/setFont (M7 text golden pin; glyphs lay out over the bundled Liberation Sans)",
        ),
        (
            "textpath.fw.png",
            build_textpath(),
            "generated: 64x48 page, text attached to an arc guide through text.attachPath (M7 text-on-path golden pin; TFTi 2 + TAPv with TAMi rotate)",
        ),
        (
            "rich.fw.png",
            build_rich(),
            "generated: 64x48 page, named rect+text become rich symbol \"btn\" — the definition carries a Widget script (JSVv under MSTv), the instance a parameter store (DTAv), and the opCode-2 re-derivation mutates the definition members through the M9 host drain (M10 rich-symbol golden pin)",
        ),
        (
            "plain.png",
            build_plain(),
            "generated: control, no private chunks (is_fireworks must be false)",
        ),
    ];

    let mut manifest = String::from(
        "# FireCraft corpus — digests pinned by `cargo run -p xtask -- corpus-gen`.\n\
         # fixtures/ are clean-room generated (committed); incoming/ holds real\n\
         # Fireworks files, which stay uncommitted.\n",
    );
    let fixture_count = fixtures.len();
    for (name, built, note) in fixtures {
        let bytes = built?;
        std::fs::write(dir.join(name), &bytes).map_err(|e| format!("write {name}: {e}"))?;
        manifest.push_str(&format!(
            "{} {FIXTURE_DIR}/{name} # {}\n",
            sha256_hex(&bytes),
            note
        ));
    }
    std::fs::write(workspace_root().join(MANIFEST), manifest)
        .map_err(|e| format!("write manifest: {e}"))?;
    eprintln!(
        "[xtask] corpus-gen: {fixture_count} fixture(s) written to {FIXTURE_DIR}/, manifest updated"
    );
    // Fresh fixtures need fresh goldens: re-render and re-pin. This also
    // re-checks the semantic anchors, so a generator that stops matching
    // its hand-computed pixels fails here, before anything gets pinned.
    crate::golden::bless()?;
    Ok(())
}

/// Standard 8x8 RGBA PNG with the full private-chunk set (single-span
/// tile).
fn build_minimal() -> Result<Vec<u8>, String> {
    let standard = encode_rgba8(8, 8, |x, y| [x as u8 * 32, y as u8 * 32, 0x40, 255])?;
    insert_before_iend(
        &standard,
        &[
            (b"prVW", zlib(&prvw_payload())?),
            (b"mkBF", vec![0u8; 28]),
            (
                b"mkTS",
                zlib(b"MKBv{FRCi{1}XLCi{0}LYLv{LAYv{BKGb{0}}LYLv{LAYv{BKGb{1}}}}}")?,
            ),
            (
                b"mkBS",
                zlib(b"MKBv{FRCi{1}XLCi{0}LYLv{LAYv{BKGb{0}}LYLv{LAYv{BKGb{1}}}}}")?,
            ),
            (b"mkBT", mkbt_payload(0, false)?),
        ],
    )
}

/// mkTS + a tile-0 stream split across two consecutive mkBT chunks +
/// tile 1 in one chunk.
fn build_tiles_split() -> Result<Vec<u8>, String> {
    let standard = encode_rgba8(4, 4, |_, _| [10, 20, 30, 255])?;
    let payload = mkbt_payload(0, false)?;
    let split = MKBT_SPLIT_AT.min(payload.len());
    insert_before_iend(
        &standard,
        &[
            (b"mkTS", zlib(b"MKBv{FRCi{1}}")?),
            (b"mkBT", payload[..split].to_vec()),
            (b"mkBT", payload[split..].to_vec()),
            (b"mkBT", mkbt_payload(1, false)?),
        ],
    )
}

/// mkTS and mkBS deliberately differ (legacy drift case).
fn build_legacy_drift() -> Result<Vec<u8>, String> {
    let standard = encode_rgba8(2, 2, |_, _| [200, 100, 50, 255])?;
    insert_before_iend(
        &standard,
        &[
            (b"mkTS", zlib(b"MKBv{FRCi{1}XLCi{0}}")?),
            (b"mkBS", zlib(b"MKBv{FRCi{2}}")?),
        ],
    )
}

/// Control: plain PNG, no private chunks.
fn build_plain() -> Result<Vec<u8>, String> {
    encode_rgba8(3, 3, |x, y| {
        [255, 255, 255, if (x + y) % 2 == 0 { 255 } else { 0 }]
    })
}

/// The golden anchor: a 16×16 page with a full-canvas red rectangle
/// (`FCLi` = ABGR red, opaque) plus a group at 1000‰ holding a black
/// left-half rectangle at 400‰. Every quantization point is
/// integer-exact: black α = 0.4 over red leaves 255 × 0.6 = 153 on the
/// red channel — left half (153, 0, 0, 255), right half (255, 0, 0,
/// 255). mkTS and mkBS carry the same tree.
///
/// Scalar spelling is the node-tree canonical form the corpus gate
/// pins: lowercase hex ints, one-decimal whole floats, 0/1 bools.
fn build_composed() -> Result<Vec<u8>, String> {
    let standard = encode_rgba8(16, 16, |_, _| [255, 0, 0, 255])?;
    let tree = b"MKBv{FRCi{1}}\
PDCv{FRCi{1}FRCi{1}XLCf{0.0}YLCf{0.0}WIDf{16.0}HITf{16.0}RESf{72.0}RSUi{0}BGCi{ffffff}\
LYLv{LAYv{BKGb{0}\
CLLv{CELv{VISb{1}ELMv{\
PTHv{PATv{BCLi{0}FCLi{ff0000ff}}PBLv{PBPv{ISCb{1}PPCi{4}\
PBTv{XLCf{0.0}YLCf{0.0}}PBTv{XLCf{16.0}YLCf{0.0}}\
PBTv{XLCf{16.0}YLCf{16.0}}PBTv{XLCf{0.0}YLCf{16.0}}}}\
VISb{1}OPAi{3e8}}\
GRPv{GRTi{0}OPAi{3e8}BLDi{0}VISb{1}ELMv{\
PTHv{PATv{BCLi{0}FCLi{ff000000}}PBLv{PBPv{ISCb{1}PPCi{4}\
PBTv{XLCf{0.0}YLCf{0.0}}PBTv{XLCf{8.0}YLCf{0.0}}\
PBTv{XLCf{8.0}YLCf{16.0}}PBTv{XLCf{0.0}YLCf{16.0}}}}\
VISb{1}OPAi{190}}}}}}}}}}";
    insert_before_iend(&standard, &[(b"mkTS", zlib(tree)?), (b"mkBS", zlib(tree)?)])
}

/// Where the tile-0 stream is cut in `tiles-split.fw.png` (inside the
/// zlib payload, well past the 76-byte header).
const MKBT_SPLIT_AT: usize = 96;

/// Builds `scripted.fw.png`: a page whose objects are created through
/// the engine registry's `autoshape.*` commands — the M6 scripted-shape
/// golden. Same pipeline the app and CLI drive (registry → doc model →
/// canonical writer), so the fixture pins the whole scripted-shape path,
/// generator geometry included.
fn build_scripted() -> Result<Vec<u8>, String> {
    let mut registry = engine::CommandRegistry::new();
    engine::catalog::install(&mut registry).map_err(|e| format!("catalog install: {e}"))?;
    let mut session = engine::Session::with_registry(doc::Document::default(), registry);
    let mut apply = |name: &str, payload: serde_json::Value| -> Result<(), String> {
        session
            .apply(name, payload)
            .map_err(|e| format!("scripted fixture: {name}: {e}"))
    };
    apply("page.add", serde_json::json!({ "index": null }))?;
    apply(
        "page.setCanvasSize",
        serde_json::json!({ "page": 0, "width": 64.0, "height": 48.0 }),
    )?;
    let placed = |mut object: serde_json::Value| -> serde_json::Value {
        let place = serde_json::json!({ "page": 0, "layer": 0, "cell": 0, "name": null });
        if let (Some(fields), Some(place)) = (object.as_object_mut(), place.as_object()) {
            for (key, value) in place {
                fields.insert(key.clone(), value.clone());
            }
        }
        object
    };
    apply(
        "autoshape.star",
        placed(serde_json::json!({
            "centerX": 16.0, "centerY": 14.0,
            "outer": 10.0, "inner": 5.0,
            "outerRound": 0.0, "innerRound": 0.0,
            "points": 5, "startAngle": -std::f64::consts::FRAC_PI_2,
        })),
    )?;
    apply(
        "autoshape.pie",
        placed(serde_json::json!({
            "centerX": 44.0, "centerY": 14.0,
            "outer": 10.0, "inner": 0.0,
            "start": -std::f64::consts::FRAC_PI_2, "end": std::f64::consts::PI,
        })),
    )?;
    apply(
        "autoshape.spiral",
        placed(serde_json::json!({
            "centerX": 16.0, "centerY": 36.0,
            "radius": 9.0, "tightnessDeg": 80.0,
            "clockwise": true, "closed": false,
        })),
    )?;
    apply(
        "autoshape.doughnut",
        placed(serde_json::json!({
            "centerX": 44.0, "centerY": 36.0,
            "outer": 9.0, "inner": 4.5,
            "start": -std::f64::consts::FRAC_PI_2,
            "end": 3.0 * std::f64::consts::FRAC_PI_2,
        })),
    )?;
    let (tree, stats) = doc::fwmap::write(session.document(), &doc::fwmap::UnknownBag::default())
        .map_err(|e| format!("scripted fixture: doc write: {e}"))?;
    if stats.unconsumed_unknown != 0 {
        return Err(format!(
            "scripted fixture: doc write left {} unknown node(s)",
            stats.unconsumed_unknown
        ));
    }
    let payload = fwpng::nodetree::serialize(&tree);
    let standard = encode_rgba8(64, 48, |_, _| [255, 255, 255, 255])?;
    insert_before_iend(
        &standard,
        &[(b"mkTS", zlib(&payload)?), (b"mkBS", zlib(&payload)?)],
    )
}

/// Builds `rich.fw.png`: a rich symbol built entirely through the
/// registry plus the M9 host — named rect + text become symbol "btn",
/// the definition gains a Widget script, the instance a parameter
/// store, and the opCode-2 re-derivation lands through the host drain
/// (scripted ≡ direct ≡ replay is the scripting crate's contract; the
/// fixture pins the wire shape that carries it).
fn build_rich() -> Result<Vec<u8>, String> {
    const WIDGET_SCRIPT: &str = r##"
var defaults = [
    { name: "label", type: "text", value: "Ready" },
    { name: "color", type: "color", value: "#3465a4" }
];
var values = Widget.elem.customData["currentValues"];
if (Widget.opCode === 1) {
    values = defaults;
    Widget.elem.customData["currentValues"] = defaults;
}
for (var i = 0; i < values.length; i++) {
    var v = values[i];
    if (v.name === "label") { Widget.GetObjectByName("label").textChars = v.value; }
    if (v.name === "color") { Widget.GetObjectByName("plate").pathAttributes.fillColor = v.value; }
}
"##;
    let mut registry = engine::CommandRegistry::new();
    engine::catalog::install(&mut registry).map_err(|e| format!("catalog install: {e}"))?;
    let mut session = engine::Session::with_registry(doc::Document::default(), registry);
    // Direct applies (not a closure) so the document reads below can
    // interleave with the mutations.
    macro_rules! apply {
        ($name:expr, $payload:expr $(,)?) => {
            session
                .apply($name, $payload)
                .map_err(|e| format!("rich fixture: {}: {e}", $name))?
        };
    }
    apply!("page.add", serde_json::json!({ "index": null }));
    apply!(
        "page.setCanvasSize",
        serde_json::json!({ "page": 0, "width": 64.0, "height": 48.0 })
    );
    let placed = |mut object: serde_json::Value| -> serde_json::Value {
        // No `name` here: the members must keep theirs (GetObjectByName
        // looks them up by name).
        let place = serde_json::json!({ "page": 0, "layer": 0, "cell": 0 });
        if let (Some(fields), Some(place)) = (object.as_object_mut(), place.as_object()) {
            for (key, value) in place {
                fields.insert(key.clone(), value.clone());
            }
        }
        object
    };
    apply!(
        "object.createPath",
        placed(serde_json::json!({
            "kind": "rect",
            "x": 8.0, "y": 8.0, "width": 32.0, "height": 14.0, "name": "plate",
        })),
    );
    apply!(
        "text.createText",
        placed(serde_json::json!({
            "x": 10.0, "y": 11.0, "width": 28.0, "height": 8.0,
            "text": "label", "name": "label",
        })),
    );
    let members = session.document().pages[0].layers.layers[0].cells[0]
        .elements
        .clone();
    apply!(
        "symbol.create",
        serde_json::json!({
            "ids": members, "name": "btn", "symbol_type": "graphic",
        }),
    );
    apply!(
        "symbol.setScript",
        serde_json::json!({ "symbol": 0, "source": WIDGET_SCRIPT }),
    );
    let instance = session.document().pages[0].layers.layers[0].cells[0].elements[0];
    apply!(
        "symbol.setParam",
        serde_json::json!({
            "id": instance, "name": "label", "kind": "text", "value": "Ready",
        }),
    );
    apply!(
        "symbol.setParam",
        serde_json::json!({
            "id": instance, "name": "color", "kind": "color", "value": "#cc0000",
        }),
    );
    // The re-derivation itself, through the M9 host — its drained
    // commands are ordinary journal rows, so the fixture needs no JS
    // to reload.
    let mut script = scripting::ScriptEngine::new(scripting::Limits::default())
        .map_err(|e| format!("rich fixture: engine: {e}"))?;
    let report = scripting::rederive_symbol(&mut session, &mut script, instance, 2);
    if !report.completed() {
        return Err(format!("rich fixture re-derive: {:?}", report.message));
    }
    let (tree, stats) = doc::fwmap::write(session.document(), &doc::fwmap::UnknownBag::default())
        .map_err(|e| format!("rich fixture: doc write: {e}"))?;
    if stats.unconsumed_unknown != 0 {
        return Err(format!(
            "rich fixture: doc write left {} unknown node(s)",
            stats.unconsumed_unknown
        ));
    }
    let payload = fwpng::nodetree::serialize(&tree);
    let standard = encode_rgba8(64, 48, |_, _| [255, 255, 255, 255])?;
    insert_before_iend(
        &standard,
        &[(b"mkTS", zlib(&payload)?), (b"mkBS", zlib(&payload)?)],
    )
}

/// Builds `text.fw.png`: one text object created and styled entirely
/// through the engine registry (`text.createText` / `text.setAttrs` /
/// `text.setFont`) — the M7 text golden. The glyph outlines the
/// compositor rasterizes come from the bundled Liberation Sans via the
/// `text` crate, so the pin covers layout + lowering + fill paint.
fn build_text() -> Result<Vec<u8>, String> {
    fn apply(
        session: &mut engine::Session,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        session
            .apply(name, payload)
            .map_err(|e| format!("text fixture: {name}: {e}"))
    }
    // The id the text command allocated: the cell's newest element.
    fn newest(session: &engine::Session) -> Result<serde_json::Value, String> {
        let elements = &session.document().pages[0].layers.layers[0].cells[0].elements;
        let id = elements
            .last()
            .ok_or_else(|| "text fixture: no object in cell".to_owned())?;
        Ok(serde_json::json!({
            "index": id.index(),
            "generation": id.generation(),
        }))
    }

    let mut registry = engine::CommandRegistry::new();
    engine::catalog::install(&mut registry).map_err(|e| format!("catalog install: {e}"))?;
    let mut session = engine::Session::with_registry(doc::Document::default(), registry);
    apply(
        &mut session,
        "page.add",
        serde_json::json!({ "index": null }),
    )?;
    apply(
        &mut session,
        "page.setCanvasSize",
        serde_json::json!({ "page": 0, "width": 64.0, "height": 48.0 }),
    )?;
    apply(
        &mut session,
        "text.createText",
        serde_json::json!({
            "page": 0, "layer": 0, "cell": 0,
            "x": 12.0, "y": 18.0, "width": 40.0, "height": 12.0,
            "text": "FireCraft",
            "name": "tagline",
        }),
    )?;
    let mut attrs = serde_json::json!({ "size": 10.0, "fill_raw": 4_278_190_080i64 });
    if let Some(fields) = attrs.as_object_mut() {
        fields.insert("id".to_owned(), newest(&session)?);
    }
    apply(&mut session, "text.setAttrs", attrs)?;
    let mut font = serde_json::json!({ "font": "Liberation Sans" });
    if let Some(fields) = font.as_object_mut() {
        fields.insert("id".to_owned(), newest(&session)?);
    }
    apply(&mut session, "text.setFont", font)?;
    let (tree, stats) = doc::fwmap::write(session.document(), &doc::fwmap::UnknownBag::default())
        .map_err(|e| format!("text fixture: doc write: {e}"))?;
    if stats.unconsumed_unknown != 0 {
        return Err(format!(
            "text fixture: doc write left {} unknown node(s)",
            stats.unconsumed_unknown
        ));
    }
    let payload = fwpng::nodetree::serialize(&tree);
    let standard = encode_rgba8(64, 48, |_, _| [255, 255, 255, 255])?;
    insert_before_iend(
        &standard,
        &[(b"mkTS", zlib(&payload)?), (b"mkBS", zlib(&payload)?)],
    )
}

/// Builds `textpath.fw.png`: the text fixture's object attached to an
/// arc guide through `text.attachPath` — the M7 text-on-path golden.
/// The rail is one cubic rising from (8,40) to (56,16); with `TAMi` 0
/// the baselines ride it, so the ink band spans far taller than any
/// linear flow would.
fn build_textpath() -> Result<Vec<u8>, String> {
    fn apply(
        session: &mut engine::Session,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        session
            .apply(name, payload)
            .map_err(|e| format!("textpath fixture: {name}: {e}"))
    }
    // The id the text command allocated: the cell's newest element.
    fn newest(session: &engine::Session) -> Result<serde_json::Value, String> {
        let elements = &session.document().pages[0].layers.layers[0].cells[0].elements;
        let id = elements
            .last()
            .ok_or_else(|| "textpath fixture: no object in cell".to_owned())?;
        Ok(serde_json::json!({
            "index": id.index(),
            "generation": id.generation(),
        }))
    }

    let mut registry = engine::CommandRegistry::new();
    engine::catalog::install(&mut registry).map_err(|e| format!("catalog install: {e}"))?;
    let mut session = engine::Session::with_registry(doc::Document::default(), registry);
    apply(
        &mut session,
        "page.add",
        serde_json::json!({ "index": null }),
    )?;
    apply(
        &mut session,
        "page.setCanvasSize",
        serde_json::json!({ "page": 0, "width": 64.0, "height": 48.0 }),
    )?;
    apply(
        &mut session,
        "text.createText",
        serde_json::json!({
            "page": 0, "layer": 0, "cell": 0,
            "x": 8.0, "y": 16.0, "width": 48.0, "height": 32.0,
            "text": "FireCraft",
            "name": "arc tagline",
        }),
    )?;
    let mut attrs = serde_json::json!({ "size": 8.0, "fill_raw": 4_278_190_080i64 });
    if let Some(fields) = attrs.as_object_mut() {
        fields.insert("id".to_owned(), newest(&session)?);
    }
    apply(&mut session, "text.setAttrs", attrs)?;
    let mut font = serde_json::json!({ "font": "Liberation Sans" });
    if let Some(fields) = font.as_object_mut() {
        fields.insert("id".to_owned(), newest(&session)?);
    }
    apply(&mut session, "text.setFont", font)?;
    let mut attach = serde_json::json!({
        "path": [{
            "closed": false,
            "points": [
                {"x": 8.0, "y": 40.0, "secondary": {"x": 24.0, "y": 40.0}, "curve": true},
                {"x": 56.0, "y": 16.0, "primary": {"x": 40.0, "y": 16.0}, "curve": true},
            ],
        }],
        "alignment": 0,
        "offset": 2.0,
    });
    if let Some(fields) = attach.as_object_mut() {
        fields.insert("id".to_owned(), newest(&session)?);
    }
    apply(&mut session, "text.attachPath", attach)?;
    let (tree, stats) = doc::fwmap::write(session.document(), &doc::fwmap::UnknownBag::default())
        .map_err(|e| format!("textpath fixture: doc write: {e}"))?;
    if stats.unconsumed_unknown != 0 {
        return Err(format!(
            "textpath fixture: doc write left {} unknown node(s)",
            stats.unconsumed_unknown
        ));
    }
    let payload = fwpng::nodetree::serialize(&tree);
    let standard = encode_rgba8(64, 48, |_, _| [255, 255, 255, 255])?;
    insert_before_iend(
        &standard,
        &[(b"mkTS", zlib(&payload)?), (b"mkBS", zlib(&payload)?)],
    )
}

/// Builds `filtered.fw.png`: four Auto Shapes, each carrying a live
/// effect stack added through `filter.add` — the M7 scripted-filter
/// golden. One object per kernel family the compositor interprets
/// (blur, point lift, invert) plus a dropShadow whose GUID is beyond
/// the catalog, pinning unfiltered pass-through in the same image.
/// Filter identity is the CS6 `EffectMoaID` GUID with `DCVs` decimal
/// strings, exactly as the reader sees them in real files.
fn build_filtered() -> Result<Vec<u8>, String> {
    fn apply(
        session: &mut engine::Session,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        session
            .apply(name, payload)
            .map_err(|e| format!("filtered fixture: {name}: {e}"))
    }
    // Placement into the fixture's first cell, like `build_scripted`.
    fn placed(mut object: serde_json::Value) -> serde_json::Value {
        let place = serde_json::json!({ "page": 0, "layer": 0, "cell": 0, "name": null });
        if let (Some(fields), Some(place)) = (object.as_object_mut(), place.as_object()) {
            for (key, value) in place {
                fields.insert(key.clone(), value.clone());
            }
        }
        object
    }
    // The id an autoshape command allocates: the cell's newest element.
    fn newest(session: &engine::Session) -> Result<serde_json::Value, String> {
        let document = session.document();
        let elements = &document.pages[0].layers.layers[0].cells[0].elements;
        let id = elements
            .last()
            .ok_or_else(|| "filtered fixture: no object in cell".to_owned())?;
        Ok(serde_json::json!({
            "index": id.index(),
            "generation": id.generation(),
        }))
    }
    fn attach(
        session: &mut engine::Session,
        shape: serde_json::Value,
        shape_name: &str,
        effect: serde_json::Value,
    ) -> Result<(), String> {
        apply(session, &format!("autoshape.{shape_name}"), placed(shape))?;
        let mut payload = effect;
        if let Some(fields) = payload.as_object_mut() {
            fields.insert("id".to_owned(), newest(session)?);
        }
        apply(session, "filter.add", payload)
    }

    let mut registry = engine::CommandRegistry::new();
    engine::catalog::install(&mut registry).map_err(|e| format!("catalog install: {e}"))?;
    let mut session = engine::Session::with_registry(doc::Document::default(), registry);
    apply(
        &mut session,
        "page.add",
        serde_json::json!({ "index": null }),
    )?;
    apply(
        &mut session,
        "page.setCanvasSize",
        serde_json::json!({ "page": 0, "width": 64.0, "height": 48.0 }),
    )?;
    attach(
        &mut session,
        serde_json::json!({
            "centerX": 16.0, "centerY": 14.0,
            "outer": 10.0, "inner": 5.0,
            "outerRound": 0.0, "innerRound": 0.0,
            "points": 5, "startAngle": -std::f64::consts::FRAC_PI_2,
        }),
        "star",
        serde_json::json!({
            "name": "gaussianBlur", "category": "Blur",
            "params": [
                { "key": "EffectMoaID", "value": "{d04ef8c0-71b3-11d1-8c8200a024cdc039}" },
                { "key": "gaussian_blur_radius", "value": "2.000000" },
            ],
        }),
    )?;
    attach(
        &mut session,
        serde_json::json!({
            "centerX": 44.0, "centerY": 14.0,
            "outer": 10.0, "inner": 0.0,
            "start": -std::f64::consts::FRAC_PI_2, "end": std::f64::consts::PI,
        }),
        "pie",
        serde_json::json!({
            "name": "brightnessContrast", "category": "Adjust Color",
            "params": [
                { "key": "EffectMoaID", "value": "{3439b08c-1921-11d3-9bde00e02910d580}" },
                { "key": "brightness_amount", "value": "25.000000" },
                { "key": "contrast_amount", "value": "0.000000" },
            ],
        }),
    )?;
    attach(
        &mut session,
        serde_json::json!({
            "centerX": 16.0, "centerY": 36.0,
            "radius": 9.0, "tightnessDeg": 80.0,
            "clockwise": true, "closed": false,
        }),
        "spiral",
        serde_json::json!({
            "name": "invert", "category": "Adjust Color",
            "params": [
                { "key": "EffectMoaID", "value": "{d2541291-70d6-11d1-8c8000a024cdc039}" },
            ],
        }),
    )?;
    attach(
        &mut session,
        serde_json::json!({
            "centerX": 44.0, "centerY": 36.0,
            "outer": 9.0, "inner": 4.5,
            "start": -std::f64::consts::FRAC_PI_2,
            "end": 3.0 * std::f64::consts::FRAC_PI_2,
        }),
        "doughnut",
        serde_json::json!({
            "name": "dropShadow", "category": "Shadow and Glow",
            "params": [
                { "key": "EffectMoaID", "value": "{deadbeef-0000-11d1-8c8000a024cdc039}" },
                { "key": "ShadowBlur", "value": "4.000000" },
            ],
        }),
    )?;
    let (tree, stats) = doc::fwmap::write(session.document(), &doc::fwmap::UnknownBag::default())
        .map_err(|e| format!("filtered fixture: doc write: {e}"))?;
    if stats.unconsumed_unknown != 0 {
        return Err(format!(
            "filtered fixture: doc write left {} unknown node(s)",
            stats.unconsumed_unknown
        ));
    }
    let payload = fwpng::nodetree::serialize(&tree);
    let standard = encode_rgba8(64, 48, |_, _| [255, 255, 255, 255])?;
    insert_before_iend(
        &standard,
        &[(b"mkTS", zlib(&payload)?), (b"mkBS", zlib(&payload)?)],
    )
}

/// Parses a standard PNG we just encoded and re-emits it with `extra`
/// private chunks inserted before IEND.
fn insert_before_iend(standard: &[u8], extra: &[(&[u8; 4], Vec<u8>)]) -> Result<Vec<u8>, String> {
    let chunks = parse_chunks(standard).map_err(|e| format!("re-parse own encoder output: {e}"))?;
    let mut file = fwpng::chunk::PNG_SIGNATURE.to_vec();
    for chunk in &chunks {
        if &chunk.ctype == b"IEND" {
            for (ctype, data) in extra {
                write_chunk(&mut file, ctype, data);
            }
        }
        write_chunk(&mut file, &chunk.ctype, &chunk.data);
    }
    Ok(file)
}

fn encode_rgba8(
    width: u32,
    height: u32,
    px: impl Fn(u32, u32) -> [u8; 4],
) -> Result<Vec<u8>, String> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&px(x, y));
        }
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("png header: {e}"))?;
        writer
            .write_image_data(&pixels)
            .map_err(|e| format!("png pixels: {e}"))?;
    }
    Ok(out)
}

/// prVW: CA FE BE EF magic + 128x128 display size + two palette entries
/// + 00 00 00 terminator + deterministic indices.
fn prvw_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&[0xCA, 0xFE, 0xBE, 0xEF]);
    payload.extend_from_slice(&128u16.to_be_bytes());
    payload.extend_from_slice(&128u16.to_be_bytes());
    payload.extend_from_slice(&[255, 0, 0]);
    payload.extend_from_slice(&[0, 255, 0]);
    payload.extend_from_slice(&[0, 0, 0]);
    for i in 0..fwpng::thumbnail::THUMB_BYTES {
        payload.push((i % 2) as u8);
    }
    payload
}

/// mkBT: FA CE CA FE magic + 24-bit index + reserved + grayscale flag +
/// 64 reserved bytes + zlib of a deterministic 64 KiB pixel span.
fn mkbt_payload(index: u32, grayscale: bool) -> Result<Vec<u8>, String> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&[0xFA, 0xCE, 0xCA, 0xFE]);
    payload.extend_from_slice(&(index << 8).to_be_bytes());
    payload.extend_from_slice(&[0, 0, 0]);
    payload.push(u8::from(grayscale));
    payload.extend_from_slice(&[0u8; 64]);
    let raw: Vec<u8> = (0..fwpng::tile::TILE_STREAM_BYTES)
        .map(|i| (i % 251) as u8)
        .collect();
    payload.extend_from_slice(&zlib(&raw)?);
    Ok(payload)
}

fn zlib(raw: &[u8]) -> Result<Vec<u8>, String> {
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(raw).map_err(|e| format!("zlib write: {e}"))?;
    enc.finish().map_err(|e| format!("zlib finish: {e}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::util::hex;

    #[test]
    fn generated_fixtures_pass_their_own_battery() {
        for (name, built) in [
            ("minimal", build_minimal()),
            ("tiles-split", build_tiles_split()),
            ("legacy-drift", build_legacy_drift()),
            ("composed", build_composed()),
            ("scripted", build_scripted()),
            ("filtered", build_filtered()),
        ] {
            let checks = round_trip_checks(&built.unwrap(), true);
            assert!(checks.iter().all(|(_, ok)| *ok), "{name}: {checks:?}");
        }
        // Control: detection must be false, and only the image-level
        // checks apply.
        let checks = round_trip_checks(&build_plain().unwrap(), false);
        assert!(checks.iter().all(|(_, ok)| *ok), "{checks:?}");
    }

    #[test]
    fn scripted_fixture_renders_all_four_shapes() {
        let bytes = build_scripted().unwrap();
        let render = crate::golden::render_entry(&bytes)
            .unwrap()
            .expect("scripted fixture must be renderable");
        assert_eq!((render.width, render.height), (64, 48));
        // A determinism pin of an empty canvas would pass forever while
        // proving nothing. The four autoshapes sit in the page's four
        // quadrants with the default fill: their opaque pixels must
        // span the whole canvas without flooding it.
        let width = u32::from(render.width);
        let mut opaque = 0usize;
        let mut bounds = Option::<(u32, u32, u32, u32)>::None; // x0 y0 x1 y1
        for (index, pixel) in render.pixels.chunks_exact(4).enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            opaque += 1;
            let (x, y) = ((index as u32) % width, (index as u32) / width);
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
        let total = usize::from(render.width) * usize::from(render.height);
        assert!(opaque > 200, "only {opaque} opaque pixel(s) rendered");
        assert!(
            opaque * 2 < total,
            "{opaque}/{total} opaque: canvas flooded"
        );
        let (x0, y0, x1, y1) = bounds.expect("no opaque pixels");
        // Exact corner pin (anti-aliasing reaches one pixel past the
        // pure geometry: star top y = 14−10 = 4, rasterized at 3).
        assert_eq!(
            (x0, y0, x1, y1),
            (7, 3, 54, 45),
            "shapes must span all quadrants"
        );
    }

    #[test]
    fn filtered_fixture_filters_and_passes_unknown_through() {
        // Both fixtures place the same four shapes on the same canvas,
        // so each quadrant pairs: filtered vs unfiltered. The three
        // interpreted effects must change their quadrant; the
        // dropShadow's unknown GUID must leave its quadrant untouched.
        let plain = build_scripted().unwrap();
        let filtered = build_filtered().unwrap();
        let plain = crate::golden::render_entry(&plain)
            .unwrap()
            .expect("scripted fixture must be renderable");
        let filtered = crate::golden::render_entry(&filtered)
            .unwrap()
            .expect("filtered fixture must be renderable");
        assert_eq!(
            (plain.width, plain.height),
            (filtered.width, filtered.height)
        );
        let width = u32::from(filtered.width);
        let height = u32::from(filtered.height);
        // Inset 6px per side: the pie's bottom edge rasterizes into the
        // doughnut's quadrant row, and its brightness lift must not
        // contaminate the pass-through control.
        const INSET: u32 = 6;
        let quadrant = |render: &compose::Render, x0: u32, y0: u32| -> Vec<u8> {
            let mut pixels = Vec::new();
            for y in y0 + INSET..y0 + height / 2 - INSET {
                for x in x0 + INSET..x0 + width / 2 - INSET {
                    let index = ((y * width + x) * 4) as usize;
                    pixels.extend_from_slice(&render.pixels[index..index + 4]);
                }
            }
            pixels
        };
        for (name, x0, y0, must_differ) in [
            ("star (gaussian blur)", 0, 0, true),
            ("pie (brightness)", width / 2, 0, true),
            ("spiral (invert)", 0, height / 2, true),
            ("doughnut (unknown GUID)", width / 2, height / 2, false),
        ] {
            let a = quadrant(&plain, x0, y0);
            let b = quadrant(&filtered, x0, y0);
            assert_eq!(a.is_empty(), b.is_empty(), "{name}: coverage changed");
            assert_ne!(a.len(), 0, "{name}: quadrant is empty");
            if must_differ {
                assert_ne!(a, b, "{name}: effect did not change the quadrant");
            } else {
                assert_eq!(a, b, "{name}: unknown effect must render unfiltered");
            }
        }
    }

    #[test]
    fn composed_fixture_has_pages_and_renders() {
        let bytes = build_composed().unwrap();
        let render = crate::golden::render_entry(&bytes)
            .unwrap()
            .expect("composed fixture must be renderable");
        assert_eq!((render.width, render.height), (16, 16));
    }

    #[test]
    fn hex_is_lowercase_and_padded() {
        assert_eq!(hex(&[0x0a, 0xff]), "0aff");
    }
}
