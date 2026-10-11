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
    let fixtures: [Fixture; 6] = [
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
