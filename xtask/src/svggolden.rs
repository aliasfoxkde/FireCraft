//! SVG golden loop (M8 task 3 stopping condition).
//!
//! For every corpus fixture whose document lives inside the exported
//! SVG surface, the gate serializes the document to SVG, rasterizes
//! that SVG with `resvg` (an independent renderer — not our stack),
//! and compares the result pixel-for-pixel against `compose::render`
//! of the same document at a recorded tolerance. The SVG bytes are
//! additionally sha-pinned like the render pins, so an unintended
//! serializer change fails before the comparison does.
//!
//! Fixtures are *ineligible* — skipped, never silently — when the
//! document carries anything whose SVG and compose semantics provably
//! diverge today: text (font-stack dependent), images (compose renders
//! them as nothing), symbol instances (compose draws them, decision
//! 0014; the SVG drops them), or a filter stack with an unmapped op
//! (compose applies, SVG drops). The export report is the eligibility
//! oracle: counters plus its notes.

use crate::corpus::manifest_entries;
use crate::util::{inflate, workspace_root};
use fwpng::chunk::parse_chunks;
use io::svgimport::ImportedSvg;
use sha2::{Digest, Sha256};
use std::path::Path;

const FIXTURE_PREFIX: &str = "corpus/fixtures/";

/// Maximum per-channel delta (0–255) tolerated at any pixel between
/// the resvg raster of our SVG and compose's own render. Measured on
/// the eligible fixture set (`xtask svg-golden --measure`, M8): flat
/// interiors are bit-exact (composed is Δ 0 everywhere); the only
/// divergence is at thin curved autoshape features (petal tips) and
/// blurred alpha ramps, where tiny-skia's scanline AA and premultiplied
/// filter arithmetic legitimately differ from vello_cpu's. Observed
/// max 77; pinned at ~1.25×. A structural bug (wrong geometry, blend,
/// or filter mapping) saturates to 255 over a region and cannot hide
/// under this — the mean bound and the SVG sha pins catch the rest.
pub const MAX_CHANNEL_DELTA: u8 = 96;

/// Maximum mean absolute channel delta across the whole frame —
/// guards against a systematically shifted (rather than edge-noisy)
/// raster passing on the max alone. Observed worst mean 0.64
/// (filtered.fw.png, 64×48); pinned at 0.9, a ~1.4× headroom.
pub const MAX_MEAN_DELTA: f64 = 0.9;

/// Gate entry point.
pub fn run() -> Result<(), String> {
    run_inner(false)
}

/// `xtask svg-golden --bless`: re-serialize and rewrite every SVG pin.
pub fn bless() -> Result<(), String> {
    run_inner(true)
}

fn run_inner(write_pins: bool) -> Result<(), String> {
    let mut compared = 0usize;
    let mut skipped: Vec<String> = Vec::new();
    let mut errors = Vec::new();
    for (_, rel) in manifest_entries()? {
        if !rel.starts_with(FIXTURE_PREFIX) {
            continue;
        }
        let bytes = match std::fs::read(workspace_root().join(&rel)) {
            Ok(b) => b,
            Err(err) => {
                errors.push(format!("{rel}: {err}"));
                continue;
            }
        };
        let Some(document) = parse_document(&bytes) else {
            continue; // not a compositor fixture; `golden` owns those
        };
        let name = rel[FIXTURE_PREFIX.len()..].to_owned();

        let out = match io::svg::encode(&document, &io::svg::SvgOptions::default()) {
            Ok(out) => out,
            Err(err) => {
                errors.push(format!("{name}: svg encode: {err}"));
                continue;
            }
        };
        let report = out.report;
        if ineligible(&report) {
            skipped.push(format!("{name} ({})", skip_reason(&report)));
            continue;
        }

        let reference = match compose::render(&document, 0, 0) {
            Ok(r) => r,
            Err(err) => {
                errors.push(format!("{name}: compose: {err}"));
                continue;
            }
        };
        let width = u32::from(reference.width);
        let height = u32::from(reference.height);
        let svg_pixels = match rasterize(&out.svg, width, height) {
            Ok(pixels) => pixels,
            Err(err) => {
                errors.push(format!("{name}: resvg: {err}"));
                continue;
            }
        };

        // compose renders premultiplied; resvg's pixmap is
        // premultiplied too, so deltas compare like for like.
        match compare(&svg_pixels, &reference.pixels) {
            Ok((max, mean)) => {
                eprintln!("[xtask] svg-golden: {name}: max Δ {max}, mean Δ {mean:.4}");
            }
            Err(err) => {
                errors.push(format!("{name}: {err}"));
                continue;
            }
        }
        compared += 1;

        let pin = Path::new(&rel).with_file_name(format!("{name}.svgpin"));
        let pin_path = workspace_root().join(&pin);
        let digest = bytes_digest(out.svg.as_bytes());
        if write_pins {
            if let Err(err) = std::fs::write(&pin_path, format!("{digest}\n")) {
                errors.push(format!("{}: write pin: {err}", pin.display()));
            }
        } else {
            match std::fs::read_to_string(&pin_path) {
                Ok(want) if want.trim() == digest => {}
                Ok(want) => errors.push(format!(
                    "{name}: svg sha256 {} != pinned {} \
                     (intentional? `cargo run -p xtask -- svg-golden --bless`)",
                    digest,
                    want.trim()
                )),
                Err(_) => errors.push(format!(
                    "{}: missing svg pin — run `cargo run -p xtask -- svg-golden --bless`",
                    pin.display()
                )),
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    if compared == 0 {
        return Err("svg-golden: no fixture was eligible — the loop compares nothing".to_owned());
    }
    eprintln!(
        "[xtask] svg-golden: {compared} fixture(s) within tolerance \
         (max Δ ≤ {MAX_CHANNEL_DELTA}, mean Δ ≤ {MAX_MEAN_DELTA}), {} skipped: {}",
        skipped.len(),
        skipped.join(", ")
    );
    Ok(())
}

/// Parses a fixture into a `Document`, mirroring `golden::render_entry`'s
/// read path. `None` when the file has no `mkTS`/pages (non-compositor
/// fixtures belong to the plain golden gate).
fn parse_document(bytes: &[u8]) -> Option<doc::Document> {
    let chunks = parse_chunks(bytes).ok()?;
    let chunk = chunks.iter().find(|c| &c.ctype == b"mkTS")?;
    let inflated = inflate(&chunk.data).ok()?;
    let nodes = fwpng::nodetree::parse(&inflated).ok()?;
    let (document, _bag) = doc::fwmap::read(&nodes).ok()?;
    (!document.pages.is_empty()).then_some(document)
}

/// A fixture is ineligible when the export *report* says the SVG
/// provably diverges from compose or is lossy: text/images render as
/// data URIs, filters dropped, or any note (a symbol instance composes
/// but the SVG drops it — decision 0014; a slice drops in both).
/// Notes are the encoder's contract: nothing diverges silently, and
/// nothing ineligible is compared anyway.
fn ineligible(report: &io::svg::SvgReport) -> bool {
    report.text_objects > 0
        || report.image_objects > 0
        || report.dropped_filters > 0
        || !report.notes.is_empty()
}

/// The skip message: counters plus the notes that triggered it.
fn skip_reason(report: &io::svg::SvgReport) -> String {
    let mut reason = format!(
        "text={} image={} dropped_filters={}",
        report.text_objects, report.image_objects, report.dropped_filters
    );
    if !report.notes.is_empty() {
        reason.push_str(&format!("; notes: {}", report.notes.join("; ")));
    }
    reason
}

/// Rasterizes `svg` at exactly `width`×`height` with resvg. A size
/// mismatch is a serializer bug, not a tolerance question.
fn rasterize(svg: &str, width: u32, height: u32) -> Result<Vec<u8>, String> {
    use resvg::tiny_skia;
    use resvg::usvg;
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &options).map_err(|e| format!("usvg parse: {e}"))?;
    let size = tree.size().to_int_size();
    if size.width() != width || size.height() != height {
        return Err(format!(
            "resvg sized the canvas {}x{}, compose says {width}x{height}",
            size.width(),
            size.height()
        ));
    }
    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or_else(|| "pixmap alloc failed".to_owned())?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(1.0, 1.0),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap.data().to_vec())
}

/// Compares two same-length premultiplied RGBA frames under the
/// recorded tolerance. Returns `(max, mean)` absolute channel deltas.
fn compare(got: &[u8], want: &[u8]) -> Result<(u8, f64), String> {
    if got.len() != want.len() {
        return Err(format!(
            "raster size mismatch: resvg produced {} bytes, compose {}",
            got.len(),
            want.len()
        ));
    }
    let mut max: u8 = 0;
    let mut total = 0u64;
    for (a, b) in got.iter().zip(want.iter()) {
        let delta = a.abs_diff(*b);
        max = max.max(delta);
        total += u64::from(delta);
    }
    let mean = total as f64 / got.len() as f64;
    if max > MAX_CHANNEL_DELTA {
        return Err(format!(
            "channel delta {max} exceeds the recorded max {MAX_CHANNEL_DELTA}"
        ));
    }
    if mean > MAX_MEAN_DELTA {
        return Err(format!(
            "mean channel delta {mean:.4} exceeds the recorded max {MAX_MEAN_DELTA}"
        ));
    }
    Ok((max, mean))
}

/// Compact diagnostic: max, mean, the count of channel deltas above
/// 8/16/32/64, and the worst pixel's coordinates — enough to tell a
/// handful of edge pixels from a structural divergence without
/// dumping the frame.
fn delta_profile(got: &[u8], want: &[u8], width: u32) -> String {
    let mut max: u8 = 0;
    let mut total = 0u64;
    let (mut over8, mut over16, mut over32, mut over64) = (0u64, 0u64, 0u64, 0u64);
    let mut worst_at = (0u32, 0u32);
    let stride = u64::from(width);
    for (index, (a, b)) in got.iter().zip(want.iter()).enumerate() {
        let delta = a.abs_diff(*b);
        if delta > max {
            max = delta;
            let pixel = index as u64 / 4;
            worst_at = (
                u32::try_from(pixel % stride).unwrap_or(0),
                u32::try_from(pixel / stride).unwrap_or(0),
            );
        }
        total += u64::from(delta);
        if delta > 8 {
            over8 += 1;
        }
        if delta > 16 {
            over16 += 1;
        }
        if delta > 32 {
            over32 += 1;
        }
        if delta > 64 {
            over64 += 1;
        }
    }
    let mean = total as f64 / got.len() as f64;
    format!(
        "max Δ {max} at ({},{}) mean {mean:.4} | Δ>8: {over8} >16: {over16} >32: {over32} >64: {over64}",
        worst_at.0, worst_at.1
    )
}

fn bytes_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Round-trip tolerance constants (M8 task 4 stopping condition). Both
/// sides of this comparison render with compose — original document vs
/// export→import document — so deltas measure *import loss*, not
/// renderer disagreement. Measured (`xtask svg-roundtrip --measure`,
/// M8): **bit-exact at Δ 0** on the whole eligible set — the
/// serializer prints full-precision coordinates and compose's blend/
/// opacity/FOTb semantics survive the usvg walk identically — so the
/// gate pins zero tolerance; any delta at all is import loss and
/// demands a re-measure before raising. The filtered fixture is
/// skipped (SVG filters import unfiltered — the recorded both-directions
/// limitation in `docs/decisions/0008-svg-filter-mapping.md`), as are
/// the text fixtures. Documents the exporter painted from gradient
/// defs are skipped too: the importer drops gradient paint servers
/// (recorded limitation), so the svg-golden gate owns their parity.
pub const ROUNDTRIP_MAX_CHANNEL_DELTA: u8 = 0;

/// Mean-delta bound for the round-trip gate (see
/// [`ROUNDTRIP_MAX_CHANNEL_DELTA`]).
pub const ROUNDTRIP_MAX_MEAN_DELTA: f64 = 0.0;

/// Gate half: export → import → render ≈ original render, per eligible
/// fixture, under the recorded round-trip tolerance. SVG bytes are
/// already sha-pinned by the export gate, so the import input is
/// deterministic and no additional pin is needed.
pub fn roundtrip() -> Result<(), String> {
    roundtrip_inner(false)
}

/// `xtask svg-roundtrip --measure`: report observed deltas without
/// enforcing (used to record the tolerance).
pub fn roundtrip_measure() -> Result<(), String> {
    roundtrip_inner(true)
}

fn roundtrip_inner(measure: bool) -> Result<(), String> {
    let mut compared = 0usize;
    let mut skipped: Vec<String> = Vec::new();
    let mut errors = Vec::new();
    for (_, rel) in manifest_entries()? {
        if !rel.starts_with(FIXTURE_PREFIX) {
            continue;
        }
        let bytes = match std::fs::read(workspace_root().join(&rel)) {
            Ok(b) => b,
            Err(err) => {
                errors.push(format!("{rel}: {err}"));
                continue;
            }
        };
        let Some(document) = parse_document(&bytes) else {
            continue; // not a compositor fixture
        };
        let name = rel[FIXTURE_PREFIX.len()..].to_owned();

        let out = match io::svg::encode(&document, &io::svg::SvgOptions::default()) {
            Ok(out) => out,
            Err(err) => {
                errors.push(format!("{name}: svg encode: {err}"));
                continue;
            }
        };
        let report = out.report;
        if ineligible(&report) {
            skipped.push(format!("{name} ({})", skip_reason(&report)));
            continue;
        }
        if report.gradient_fills > 0 {
            // The importer drops gradient paint servers (recorded
            // limitation), so the bit-exact contract cannot even start
            // — the svg-golden gate owns these renders' parity.
            skipped.push(format!(
                "{name} ({} gradient fill(s) drop on import; \
                 svg-golden owns their parity)",
                report.gradient_fills
            ));
            continue;
        }

        let reference = match compose::render(&document, 0, 0) {
            Ok(r) => r,
            Err(err) => {
                errors.push(format!("{name}: compose: {err}"));
                continue;
            }
        };
        let imported = match io::svgimport::decode(out.svg.as_bytes()) {
            Ok(ImportedSvg { document, report }) => {
                if !report.notes.is_empty() {
                    skipped.push(format!(
                        "{name} ({} import note(s): {})",
                        report.notes.len(),
                        report.notes.join("; ")
                    ));
                    continue;
                }
                document
            }
            Err(err) => {
                errors.push(format!("{name}: svg import: {err}"));
                continue;
            }
        };
        let rendered = match compose::render(&imported, 0, 0) {
            Ok(r) => r,
            Err(err) => {
                errors.push(format!("{name}: compose after import: {err}"));
                continue;
            }
        };
        if rendered.width != reference.width || rendered.height != reference.height {
            errors.push(format!(
                "{name}: round trip resized the canvas {}x{} -> {}x{}",
                reference.width, reference.height, rendered.width, rendered.height
            ));
            continue;
        }

        if measure {
            eprintln!(
                "[xtask] svg-roundtrip: {name}: {}",
                delta_profile(
                    &rendered.pixels,
                    &reference.pixels,
                    u32::from(reference.width)
                )
            );
        } else {
            let (max, mean) = compare_roundtrip(&rendered.pixels, &reference.pixels)?;
            eprintln!("[xtask] svg-roundtrip: {name}: max Δ {max}, mean Δ {mean:.4}");
        }
        compared += 1;
    }

    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    if compared == 0 {
        return Err(
            "svg-roundtrip: no fixture was eligible — the loop compares nothing".to_owned(),
        );
    }
    eprintln!(
        "[xtask] svg-roundtrip: {compared} fixture(s) within tolerance \
         (max Δ ≤ {ROUNDTRIP_MAX_CHANNEL_DELTA}, mean Δ ≤ {ROUNDTRIP_MAX_MEAN_DELTA}), \
         {} skipped: {}",
        skipped.len(),
        skipped.join(", ")
    );
    Ok(())
}

/// Round-trip comparison under the (expected-bit-exact) tolerance.
fn compare_roundtrip(got: &[u8], want: &[u8]) -> Result<(u8, f64), String> {
    let mut max: u8 = 0;
    let mut total = 0u64;
    for (a, b) in got.iter().zip(want.iter()) {
        let delta = a.abs_diff(*b);
        max = max.max(delta);
        total += u64::from(delta);
    }
    let mean = total as f64 / got.len() as f64;
    if max > ROUNDTRIP_MAX_CHANNEL_DELTA {
        return Err(format!(
            "round-trip channel delta {max} exceeds the recorded max \
             {ROUNDTRIP_MAX_CHANNEL_DELTA} (import loss — re-measure with \
             `xtask svg-roundtrip --measure` before raising)"
        ));
    }
    if mean > ROUNDTRIP_MAX_MEAN_DELTA {
        return Err(format!(
            "round-trip mean delta {mean:.4} exceeds the recorded max \
             {ROUNDTRIP_MAX_MEAN_DELTA}"
        ));
    }
    Ok((max, mean))
}

/// Dev-only measurement pass: reports the observed deltas per fixture
/// without enforcing anything (used to record the tolerance; kept for
/// future re-measurement after renderers evolve).
pub fn measure() -> Result<(), String> {
    for (_, rel) in manifest_entries()? {
        if !rel.starts_with(FIXTURE_PREFIX) {
            continue;
        }
        let Ok(bytes) = std::fs::read(workspace_root().join(&rel)) else {
            continue;
        };
        let Some(document) = parse_document(&bytes) else {
            continue;
        };
        let name = rel[FIXTURE_PREFIX.len()..].to_owned();
        let Ok(out) = io::svg::encode(&document, &io::svg::SvgOptions::default()) else {
            continue;
        };
        let report = out.report;
        if ineligible(&report) {
            eprintln!("[xtask] svg-golden: {name}: ineligible (parity surface)");
            continue;
        }
        let Ok(reference) = compose::render(&document, 0, 0) else {
            continue;
        };
        match rasterize(
            &out.svg,
            u32::from(reference.width),
            u32::from(reference.height),
        ) {
            Ok(pixels) => {
                eprintln!(
                    "[xtask] svg-golden: {name}: {}",
                    delta_profile(&pixels, &reference.pixels, u32::from(reference.width))
                );
            }
            Err(err) => eprintln!("[xtask] svg-golden: {name}: {err}"),
        }
    }
    Ok(())
}
