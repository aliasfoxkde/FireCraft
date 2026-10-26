//! Script-corpus run harness (M9 task 5): copies the shipped `.jsf`
//! corpus from `source/` into scratch (never committed, never
//! mutated), runs every script headless against a fixture document
//! through the `scripting` host, and writes the committed per-script
//! report ([`REPORT`]). The report is the artifact: statuses, the
//! not-implemented refusal histogram (the next task's work list), the
//! static call surface, and effect hashes where the script's effect
//! is deterministic.
//!
//! Without `source/` (CI), the gate verifies the committed report
//! against the rising-only [`CORPUS_RUN_FLOOR`]: the report must
//! exist, cover a non-empty corpus, and hold the floor.

use crate::util::{sha256_hex, workspace_root};
use doc::{Canvas, Cell, Document, Layer, LayerList, Page, PageInfo};
use engine::Session;
use engine::platform::{FileSystem, MemoryFs};
use scripting::{Limits, Outcome, ScriptEngine, run_in_session, run_with_file_system};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

/// Rising-only floor: committed scripts that must still run headless.
/// Set from the first corpus run (19 of 89 completed; 2026-10-12,
/// see the report and the M9 ticket). Never lower; raise only with a
/// regenerated report proving the higher count.
pub const CORPUS_RUN_FLOOR: usize = 19;

/// The committed report (machine-checked by [`verify`]).
const REPORT: &str = "corpus/scripts-report.json";

/// The shipped Configuration tree the corpus walks. Git-ignored.
const SOURCE_DIR: &str = "source/Adobe Fireworks CS6/Configuration";

/// Scratch snapshot directory under `TMPDIR` (`NVMe` — small files).
const SCRATCH_DIR: &str = "firecraft-jsf-corpus";

/// Host refusal prefix (emitted by the prelude's `__notImplemented`).
const NOT_IMPLEMENTED: &str = "not implemented in FireCraft: ";

/// Static-extraction roots: the host globals scripts call through.
const ROOTS: &[&str] = &["fw", "dom", "Files", "smartShape"];

/// One corpus file: path relative to the Configuration root + bytes.
type CorpusFile = (String, Vec<u8>);

/// Regenerates the report when `source/` is present; verifies the
/// committed report against the floor when it is not (CI posture).
pub fn run() -> Result<(), String> {
    let source = workspace_root().join(SOURCE_DIR);
    if source.is_dir() {
        generate(&source)
    } else {
        verify()
    }
}

/// Runs every corpus script twice and writes [`REPORT`].
fn generate(source: &Path) -> Result<(), String> {
    let files = collect(source)?;
    scratch_copy(&files)?;
    let mut rows: Vec<Value> = Vec::new();
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    let mut static_calls: BTreeMap<String, usize> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();

    // Static surface first: every `root.ident` call site, then probed
    // against a live host so "missing" is observed, never tabulated
    // by hand (a hand list drifts the moment the host grows).
    for (_name, bytes) in &files {
        let text = String::from_utf8_lossy(bytes);
        for (call, count) in static_calls_of(&text) {
            *static_calls.entry(call).or_insert(0) += count;
        }
    }
    let names: Vec<String> = static_calls.keys().cloned().collect();
    let name_status = probe_names(&names)?;
    let missing: Vec<&String> = name_status
        .iter()
        .filter(|(_, status)| status.as_str() == "missing")
        .map(|(name, _)| name)
        .collect();

    // The fixture's own journal/digest: `applied` subtracts it so a
    // no-effect completion reads as 0, and `seed_sha` makes "no
    // effect" visible as digest equality.
    let seed = fixture()?;
    let seed_sha = doc_sha(seed.document());
    let seed_journal_len = seed.journal_len();

    for (name, bytes) in &files {
        let text = String::from_utf8_lossy(bytes);
        let script_names = extract_calls(&text);
        let script_missing: Vec<String> = script_names
            .iter()
            .filter(|call| name_status.get(*call).map(String::as_str) == Some("missing"))
            .cloned()
            .collect();

        // Two fresh runs: an effect hash is recorded as a pin only
        // when both runs land byte-identical documents.
        let first = run_one(&files, name)?;
        let second = run_one(&files, name)?;
        let status = status_of(name, &first.report);
        *counts.entry(status.to_owned()).or_insert(0) += 1;
        if let Some(refused) = refusal_name(&first.report) {
            *refusals.entry(refused).or_insert(0) += 1;
        }

        let mut row = json!({
            "name": name,
            "status": status,
            "message": first.report.message.as_deref().map(truncate),
            "missing": script_missing,
        });
        if status == "ran" {
            row["applied"] = json!(first.session.journal_len() - seed_journal_len);
            row["doc_sha"] = match (first.sha(), second.sha()) {
                (Some(sha), Some(other)) if sha == other => json!(sha),
                _ => json!("varies"),
            };
        }
        rows.push(row);
    }

    let ran = counts.get("ran").copied().unwrap_or(0);
    let report = json!({
        "source": SOURCE_DIR,
        "generated_unix": unix_seconds(),
        "scripts_total": files.len(),
        "floor": CORPUS_RUN_FLOOR,
        "counts": counts,
        "seed_journal_len": seed_journal_len,
        "seed_sha": seed_sha,
        "refusals": refusals,
        "name_status": name_status,
        "static_calls": static_calls,
        "scripts": rows,
    });
    let path = workspace_root().join(REPORT);
    let serialized = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("{REPORT} does not serialize: {error}"))?;
    std::fs::write(&path, serialized).map_err(|error| format!("write {REPORT}: {error}"))?;
    check_floor(&report)?;
    eprintln!(
        "[xtask] scripts: {} script(s) run headless, {ran} ran, {} host name(s) missing, report at {REPORT}",
        files.len(),
        missing.len()
    );
    Ok(())
}

/// `typeof`-probes every statically-extracted host name against a
/// live attached host. `undefined` is reported as `missing`; anything
/// else is the observed type ("function", "object", "string", …).
/// Typed refusals (`Files.getLanguageDirectory`) probe as functions —
/// they are separately visible in the refusals histogram.
fn probe_names(names: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut statuses = BTreeMap::new();
    if names.is_empty() {
        return Ok(statuses);
    }
    let session = fixture()?;
    let mut engine =
        ScriptEngine::new(Limits::default()).map_err(|error| format!("engine: {error}"))?;
    if let Err(error) = engine.attach_document(session.document()) {
        // No projection, no honest probes: report everything missing
        // rather than guessing.
        for name in names {
            statuses.insert(name.clone(), format!("probe failed: {error}"));
        }
        return Ok(statuses);
    }
    for name in names {
        let report = engine.run("probe", &format!("fw.__log(typeof ({name}));"));
        let observed = report
            .notes
            .first()
            .map(String::as_str)
            .unwrap_or("probe error");
        statuses.insert(
            name.clone(),
            if observed == "undefined" {
                "missing".to_owned()
            } else {
                observed.to_owned()
            },
        );
    }
    Ok(statuses)
}

/// CI posture: the committed report must exist and hold the floor.
fn verify() -> Result<(), String> {
    let path = workspace_root().join(REPORT);
    let bytes = std::fs::read(&path).map_err(|error| {
        format!(
            "committed script-corpus report missing at {REPORT} ({error}); \
             run `cargo run -p xtask -- scripts` where source/ is checked out"
        )
    })?;
    let report: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{REPORT} does not parse: {error}"))?;
    check_floor(&report)
}

/// The rising-only gate: `ran` at or above the floor over a non-empty
/// corpus.
fn check_floor(report: &Value) -> Result<(), String> {
    let ran = report["counts"]["ran"].as_u64().unwrap_or(0) as usize;
    let total = report["scripts_total"].as_u64().unwrap_or(0) as usize;
    if total == 0 {
        return Err(format!(
            "{REPORT}: empty corpus — regenerate where source/ exists"
        ));
    }
    if ran < CORPUS_RUN_FLOOR {
        return Err(format!(
            "{REPORT}: {ran} script(s) ran headless, below the rising-only floor {CORPUS_RUN_FLOOR}"
        ));
    }
    Ok(())
}

/// One headless run: fresh session + engine + FS, the script loaded
/// from the collected corpus.
struct RunOutcome {
    session: Session,
    report: scripting::ScriptReport,
}

impl RunOutcome {
    /// Document-bytes digest when the run completed.
    fn sha(&self) -> Option<String> {
        match self.report.completed() {
            true => doc_sha(self.session.document()).ok(),
            false => None,
        }
    }
}

fn run_one(files: &[CorpusFile], name: &str) -> Result<RunOutcome, String> {
    let source = files
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
        .unwrap_or_default();
    let fs = corpus_fs(files)?;
    let mut session = fixture()?;
    let mut engine =
        ScriptEngine::new(Limits::default()).map_err(|error| format!("engine: {error}"))?;
    let script_name = name.to_owned();
    let report = run_with_file_system(fs, || {
        run_in_session(&mut session, &mut engine, &script_name, &source)
    });
    Ok(RunOutcome { session, report })
}

/// Status classification from the run report. Eval-time throws carry
/// the `{script name}: ` prefix; drain-time failures do not.
fn status_of(name: &str, report: &scripting::ScriptReport) -> &'static str {
    match report.outcome {
        Outcome::Completed => "ran",
        Outcome::LimitExceeded => "limited",
        Outcome::Threw => {
            if refusal_name(report).is_some() {
                "not-implemented"
            } else if report
                .message
                .as_deref()
                .is_some_and(|message| message.starts_with(&format!("{name}: ")))
            {
                "threw"
            } else {
                "drain-failed"
            }
        }
    }
}

/// The refused name ("dom.selectAll") when the run died on a typed
/// not-implemented refusal.
fn refusal_name(report: &scripting::ScriptReport) -> Option<String> {
    let message = report.message.as_deref()?;
    let at = message.find(NOT_IMPLEMENTED)?;
    let rest = &message[at + NOT_IMPLEMENTED.len()..];
    let end = rest.find(" (M9 task").unwrap_or(rest.len());
    Some(rest[..end].to_owned())
}

/// SHA-256 (first 16 hex) of the document's canonical JSON bytes.
fn doc_sha(document: &Document) -> Result<String, String> {
    let bytes = serde_json::to_vec(document)
        .map_err(|error| format!("document does not serialize: {error}"))?;
    Ok(sha256_hex(&bytes)[..16].to_owned())
}

/// The fixture document every corpus script runs against: 480×320
/// canvas, one page, two layers; the registry seeds two 32×32
/// rectangles on `bottom` so selection-carrying scripts have targets.
fn fixture() -> Result<Session, String> {
    let mut document = Document::default();
    document.info.canvas = Canvas {
        x: 0.0,
        y: 0.0,
        width: 480.0,
        height: 320.0,
        resolution: 96.0,
        resolution_unit: 0,
        background_raw: 0,
    };
    document.info.frame_count = 1;
    let layer = |name: &str| Layer {
        name: name.to_owned(),
        display: true,
        cells: vec![Cell {
            locked: false,
            visible: true,
            elements: Vec::new(),
        }],
        ..Layer::default()
    };
    document.pages.push(Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: 1,
            frame_count_duplicate: 1,
            request_id: 0,
            canvas: Canvas::default(),
            onion_before: 0,
            onion_after: 0,
            frame_durations: Vec::new(),
        },
        name: Some("Page 1".to_owned()),
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList {
            layers: vec![layer("bottom"), layer("top")],
        },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    });

    let mut session = Session::new(document);
    for (x, name) in [(8.0, "fixture-a"), (64.0, "fixture-b")] {
        session
            .apply(
                "object.createPath",
                json!({
                    "kind": "rect",
                    "x": x, "y": 8.0, "width": 32.0, "height": 32.0,
                    "page": 0, "layer": 0, "cell": 0, "name": name,
                }),
            )
            .map_err(|error| format!("fixture rectangle: {error}"))?;
    }
    Ok(session)
}

/// A fresh in-memory FS seeded with every corpus file under its
/// Configuration-relative path and its bare name (sibling
/// `fw.runScript` chains resolve either way). Per-run: scripts that
/// `Files.write` cannot leak state into the next run.
fn corpus_fs(files: &[CorpusFile]) -> Result<Rc<dyn FileSystem>, String> {
    let fs = MemoryFs::new();
    for (name, bytes) in files {
        fs.write_file(name, bytes)
            .map_err(|error| format!("seed {name}: {error}"))?;
        let base = name.rsplit('/').next().unwrap_or(name);
        fs.write_file(base, bytes)
            .map_err(|error| format!("seed {base}: {error}"))?;
    }
    Ok(Rc::new(fs))
}

/// Walks `root` for `*.jsf`, sorted by relative path, read to memory.
fn collect(root: &Path) -> Result<Vec<CorpusFile>, String> {
    let mut files: Vec<CorpusFile> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|error| format!("walk {}: {error}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("walk {}: {error}", dir.display()))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "jsf") {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|error| format!("{}: {error}", path.display()))?
                    .to_string_lossy()
                    .into_owned();
                let bytes = std::fs::read(&path)
                    .map_err(|error| format!("read {}: {error}", path.display()))?;
                files.push((relative, bytes));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// Copies the corpus into the scratch snapshot (the run input is a
/// disposable copy; `source/` is never touched). The in-memory runs
/// read from [`collect`]'s bytes; this snapshot is the ticket's
/// provenance step and keeps the corpus inspectable between runs.
fn scratch_copy(files: &[CorpusFile]) -> Result<(), String> {
    let scratch = std::env::temp_dir().join(SCRATCH_DIR);
    std::fs::create_dir_all(&scratch)
        .map_err(|error| format!("scratch {}: {error}", scratch.display()))?;
    for (name, bytes) in files {
        let target = scratch.join(name);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("scratch {}: {error}", parent.display()))?;
        }
        std::fs::write(&target, bytes)
            .map_err(|error| format!("scratch {}: {error}", target.display()))?;
    }
    Ok(())
}

/// Every `root.ident` call site (over-approximates: strings and
/// comments count too — the report says so).
fn static_calls_of(source: &str) -> BTreeMap<String, usize> {
    let mut calls: BTreeMap<String, usize> = BTreeMap::new();
    for name in extract_calls(source) {
        *calls.entry(name).or_insert(0) += 1;
    }
    calls
}

/// Scans for the [`ROOTS`] globals followed by `.identifier`.
fn extract_calls(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (at, _) in source.char_indices() {
        let boundary_before = !source[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if !boundary_before {
            continue;
        }
        for root in ROOTS {
            if !source[at..].starts_with(root) {
                continue;
            }
            let after = at + root.len();
            let Some(rest) = source.get(after..) else {
                continue;
            };
            let mut chars = rest.chars();
            if chars.next() != Some('.') {
                continue;
            }
            let Some(next) = chars.next() else {
                continue;
            };
            if !(next.is_ascii_alphabetic() || next == '_') {
                continue;
            }
            // `after` is the '.'; the identifier starts right behind it.
            let name_start = after + 1;
            let end = source[name_start..]
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .map_or(source.len(), |offset| name_start + offset);
            found.push(format!("{root}.{}", &source[name_start..end]));
        }
    }
    found
}

/// Seconds since the Unix epoch (the report's generation stamp).
fn unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Report messages are capped so the committed file stays readable.
fn truncate(message: &str) -> String {
    if message.len() <= 200 {
        return message.to_owned();
    }
    let mut cut = 200;
    while !message.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}…", &message[..cut])
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The committed report exists and holds the rising-only floor —
    /// the same check CI's verify path runs where `source/` is absent.
    #[test]
    fn committed_report_holds_the_corpus_floor() {
        verify().expect("committed report verifies");
    }

    /// Identifier extraction keeps the full name (regression: the
    /// scan once dropped the first character).
    #[test]
    fn call_extraction_keeps_the_full_name() {
        let calls = extract_calls("var x = fw.getDocumentDOM().dom.selectAll();");
        assert!(calls.contains(&"fw.getDocumentDOM".to_owned()), "{calls:?}");
        assert!(calls.contains(&"dom.selectAll".to_owned()), "{calls:?}");
        // Comments and strings count too (documented over-approximation).
        let calls = extract_calls("// Files.getLanguageDirectory()\nvar s = 'fw.runScript';");
        assert_eq!(calls.len(), 2, "{calls:?}");
    }
}
