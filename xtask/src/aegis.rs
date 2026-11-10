//! Aegis secret/PII scan gate with a content-keyed baseline.
//!
//! Aegis's own `--baseline` filter matches a finding on the full
//! fingerprint `pattern:file:line:digest`, so the line is part of the
//! identity: a prose edit that pushes a documented false positive down
//! the file reports it as a *new* finding, and the gate fails on noise.
//! The remedy that grew around that — re-pin the baseline after every
//! doc edit — is the defect wearing a procedure as a disguise.
//!
//! The digest aegis already computes is content-derived and
//! line-independent (the same literal hashes identically in every file
//! and at every line), so this gate does its own matching on the
//! content key `(pattern, file, digest)` and keeps the multiplicity.
//! A finding that moved is not new. A new literal, a new file, or an
//! extra copy of a known literal is new, and fails.
//!
//! The committed baseline is `.aegis-baseline.tsv`: one record per
//! content key, with the number of occurrences adjudicated when it was
//! written. It is small enough to review in a diff — the old 1.8 MB
//! JSON blob never could be.
//!
//! Regenerate with `cargo run -p xtask -- aegis --bless`, and only
//! after verifying each newly added key is a false positive.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::util;

/// The committed baseline: one record per content key, tab-separated.
pub const BASELINE: &str = ".aegis-baseline.tsv";

/// Header written above the baseline records.
const HEADER: &str = "\
# Aegis content baseline — content-keyed and line-independent.
#
# One record per (pattern, file, content digest):
#     <pattern> TAB <file> TAB <content-digest> TAB <count>
#
# The gate fails when the scan finds MORE occurrences of a key than the
# recorded count. Moving a baselined finding to another line is not new;
# a new literal, a new file, or an extra copy of a known literal is.
#
# No rule class is switched off: every rule the scanner has stays armed,
# and a new occurrence of any of these literals fails the gate. The
# comments below each group record why those particular occurrences were
# adjudicated as false positives, so a reviewer can read the reason
# next to the rows it covers.
#
# The gate's own source quotes the literals it explains, so it carries
# rows of its own; they are adjudicated like any other.
#
# Regenerate with: cargo run -p xtask -- aegis --bless
# Only after verifying each newly added key is a false positive.
";

/// Why the occurrences of each rule class were adjudicated as false
/// positives.
///
/// The comment is emitted above the class's records, so the reason
/// sits next to the rows it covers. The class itself stays armed.
const CLASS_NOTES: &[(&str, &str)] = &[
    (
        "australian-tfn",
        "Matches the 9-digit observed value range in the PNG node dictionary \
         (0..141663582); this repository holds no tax identifiers.",
    ),
    (
        "bank-routing-number",
        "Matches the same 9-digit observed value range in the PNG node \
         dictionary; this repository holds no banking data.",
    ),
    (
        "bearer-token-url",
        "The GitForge API row names the credential file that holds the \
         bearer token; the row contains no token.",
    ),
    (
        "credit-card-amex",
        "A glyph-position golden value (27.345314025878906); its digit run \
         matches the Amex shape, and it is a float literal, not a card number.",
    ),
    (
        "credit-card-number-generic",
        "The same glyph-position golden value, seen by the generic \
         card-number rule.",
    ),
    (
        "eval-usage",
        "The scripting crate's own boa context.eval() call — that crate \
         implements the CS6 scripting DOM's eval() — plus Rust curve-segment \
         .eval(u) calls.",
    ),
    (
        "international-phone",
        "Matches the build metadata of a Cargo.lock version string \
         (0.6.0+11769913).",
    ),
    (
        "path-traversal",
        "The rule's regex is JavaScript-shaped (open / fs. / readFile, \
         then user / input / query) and fires on English prose; this \
         repository has no server-side file read.",
    ),
    (
        "security-hardening-path-traversal",
        "The same JavaScript-shaped regex, seen by the hardening rule.",
    ),
    (
        "pci-cardholder-data",
        "Matches the word \"pan\" in pan-zoom and hand-pan identifiers; the \
         five credit-card-* rules stay armed.",
    ),
    (
        "phone-number",
        "Matches any 10-digit run: u32::MAX, CRC and chunk values, observed \
         PNG value ranges.",
    ),
    (
        "ssrf-localhost",
        "A loopback listener in the release smoke test, a localhost \
         maintainer address, prose naming the local GitForge API, and the \
         local preview URL in the site deployment guide; the workspace has \
         no HTTP client dependency.",
    ),
    (
        "ssn-no-dashes",
        "Matches the same 9-digit observed value range in the PNG node \
         dictionary; this repository holds no social-security numbers.",
    ),
    (
        "stack-trace-exposure",
        "A comment quoting a V8 crash dump observed in CI; it documents a \
         failure, and the build prints nothing.",
    ),
    (
        "xml-external-entity",
        "The SVG importer's entity-expansion fixture, which must declare an \
         external entity so the test can observe the importer refusing it, \
         and the Apple plist doctype's quoted public identifier.",
    ),
    (
        "security-hardening-xml-external-entity",
        "The same two fixtures, seen by the hardening rule.",
    ),
    (
        "zip-code",
        "Matches any 5-digit run: CS6 command ids, the JPEG dimension \
         ceiling (65535), menu ids. This repository holds no postal data.",
    ),
];

/// The comment for a rule class, wrapped as comment lines.
fn class_note(pattern: &str) -> String {
    let note = CLASS_NOTES
        .iter()
        .find(|(name, _)| *name == pattern)
        .map(|(_, note)| *note)
        .unwrap_or("Individually adjudicated as a false positive; see decision 0074.");
    let mut out = String::new();
    let mut line = String::new();
    for word in note.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > 70 {
            out.push_str(&format!("# {line}\n"));
            line = word.to_owned();
        } else if line.is_empty() {
            line = word.to_owned();
        } else {
            line.push(' ');
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        out.push_str(&format!("# {line}\n"));
    }
    out
}

/// One scanned finding, reduced to what the baseline keys on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The scanner's rule name (`pci-cardholder-data`, `zip-code`, ...).
    pub pattern: String,
    /// The scanned path, relative to the workspace root.
    pub file: String,
    /// The content digest — the fingerprint's trailing component, which
    /// hashes the matched literal alone and so ignores the line.
    pub digest: String,
    /// The line the finding was reported at (for reporting only — it is
    /// not part of the identity).
    pub line: u64,
}

/// The content key: pattern, file, and content digest.
type Key = (String, String, String);

/// Runs the gate: scan, compare against the baseline, fail on anything
/// the baseline does not cover.
pub fn run() -> Result<(), String> {
    let baseline = read_baseline()?;
    let findings = scan()?;
    let fresh = uncovered(&findings, &baseline);
    let drift = stale(&findings, &baseline);
    if !drift.is_empty() {
        eprintln!(
            "[xtask] aegis: {} baseline key(s) no longer present (re-bless to refresh)",
            drift.len()
        );
    }
    if fresh.is_empty() {
        return Ok(());
    }
    Err(format!(
        "aegis found {} finding(s) the baseline does not cover:\n{}",
        fresh.len(),
        render_findings(&fresh)
    ))
}

/// Rewrites the baseline from the current tree, reporting what changed.
///
/// This is the only way to admit a new finding, and it is a manual
/// step: the printed additions must be verified as false positives
/// before the rewrite is committed.
pub fn bless() -> Result<(), String> {
    let path = util::workspace_root().join(BASELINE);
    let previous = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| parse_baseline(&text).ok())
        .unwrap_or_default();
    let findings = scan()?;
    let counts = counts_of(&findings);
    let added: Vec<&Key> = counts
        .keys()
        .filter(|key| !previous.contains_key(*key))
        .collect();
    let removed: Vec<&Key> = previous
        .keys()
        .filter(|key| !counts.contains_key(*key))
        .collect();
    std::fs::write(&path, render_baseline(&counts))
        .map_err(|e| format!("could not write {BASELINE}: {e}"))?;
    eprintln!(
        "[xtask] aegis --bless: {} key(s) written to {BASELINE} ({} added, {} removed)",
        counts.len(),
        added.len(),
        removed.len()
    );
    for (pattern, file, digest) in added {
        eprintln!("[xtask] aegis --bless: + {pattern} {file} {digest}");
    }
    for (pattern, file, digest) in removed {
        eprintln!("[xtask] aegis --bless: - {pattern} {file} {digest}");
    }
    Ok(())
}

/// Scans the workspace and returns the findings.
fn scan() -> Result<Vec<Finding>, String> {
    let root = util::workspace_root();
    let (stdout, stderr) = util::capture_in(
        Some(root.as_path()),
        &aegis_binary()?,
        &["--format", "json", "-c", "production", "scan", ".", "-q"],
    )?;
    if stdout.trim().is_empty() {
        return Err(format!(
            "aegis wrote no JSON to stdout (exit diagnostics follow):\n{}",
            stderr.trim()
        ));
    }
    parse_scan(&stdout)
}

/// The Aegis scanner's CLI. An unrelated tool also ships under the
/// bare name `aegis` (a security-file generator), so the versioned
/// scanner binary is preferred and the plain name is used only when
/// it really is the scanner — decided by its own `--help`, which lists
/// the `scan` subcommand the gate needs. `FIRECRAFT_AEGIS` overrides
/// both.
fn aegis_binary() -> Result<String, String> {
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(explicit) = std::env::var("FIRECRAFT_AEGIS") {
        candidates.push(explicit);
    }
    candidates.push("aegis-0.6.3".to_owned());
    candidates.push("aegis".to_owned());
    for candidate in &candidates {
        let Ok(output) = std::process::Command::new(candidate).arg("--help").output() else {
            continue;
        };
        let help = String::from_utf8_lossy(&output.stdout);
        if help.contains("scan") {
            return Ok(candidate.clone());
        }
    }
    Err(format!(
        "no Aegis scanner found (looked for {}); set FIRECRAFT_AEGIS to the scanner's path",
        candidates.join(", ")
    ))
}

/// Parses the scanner's JSON document into findings.
fn parse_scan(json: &str) -> Result<Vec<Finding>, String> {
    let document: Value =
        serde_json::from_str(json).map_err(|e| format!("aegis output is not JSON: {e}"))?;
    let list = document
        .get("findings")
        .and_then(Value::as_array)
        .ok_or("aegis output has no `findings` array")?;
    list.iter().map(parse_finding).collect()
}

/// Reduces one JSON finding to its content key.
fn parse_finding(value: &Value) -> Result<Finding, String> {
    let pattern = value
        .get("pattern")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let location = value.get("location");
    let file = location
        .and_then(|at| at.get("file"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let line = location
        .and_then(|at| at.get("line"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let fingerprint = value
        .get("fingerprint")
        .and_then(Value::as_str)
        .ok_or("aegis finding carries no fingerprint")?;
    let digest = fingerprint
        .rsplit_once(':')
        .map(|(_, digest)| digest)
        .filter(|digest| !digest.is_empty())
        .ok_or_else(|| format!("aegis fingerprint carries no digest component: {fingerprint}"))?
        .to_owned();
    Ok(Finding {
        pattern,
        file,
        digest,
        line,
    })
}

/// Reads and parses the committed baseline.
fn read_baseline() -> Result<BTreeMap<Key, usize>, String> {
    let path = util::workspace_root().join(BASELINE);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "{BASELINE} is unreadable ({e}); regenerate it with \
             `cargo run -p xtask -- aegis --bless`"
        )
    })?;
    parse_baseline(&text)
}

/// Parses the baseline's tab-separated records.
fn parse_baseline(text: &str) -> Result<BTreeMap<Key, usize>, String> {
    let mut counts: BTreeMap<Key, usize> = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = trimmed.split('\t').collect();
        if fields.len() != 4 {
            return Err(format!(
                "{BASELINE}:{}: expected 4 tab-separated fields, found {}",
                index + 1,
                fields.len()
            ));
        }
        let count: usize = fields[3]
            .parse()
            .map_err(|e| format!("{BASELINE}:{}: unreadable count: {e}", index + 1))?;
        let key = (
            fields[0].to_owned(),
            fields[1].to_owned(),
            fields[2].to_owned(),
        );
        if counts.insert(key.clone(), count).is_some() {
            return Err(format!(
                "{BASELINE}:{}: duplicate key {}/{}/{}",
                index + 1,
                key.0,
                key.1,
                key.2
            ));
        }
    }
    Ok(counts)
}

/// Renders the baseline records, with each rule class's reason
/// emitted above its rows.
fn render_baseline(counts: &BTreeMap<Key, usize>) -> String {
    let mut out = String::from(HEADER);
    let mut current = String::new();
    for ((pattern, file, digest), count) in counts {
        if *pattern != current {
            current = pattern.clone();
            out.push_str(&class_note(pattern));
        }
        out.push_str(&format!("{pattern}\t{file}\t{digest}\t{count}\n"));
    }
    out
}

/// The occurrences each content key was reported at.
fn counts_of(findings: &[Finding]) -> BTreeMap<Key, usize> {
    let mut counts: BTreeMap<Key, usize> = BTreeMap::new();
    for finding in findings {
        *counts.entry(key_of(finding)).or_insert(0) += 1;
    }
    counts
}

/// The content key of a finding.
fn key_of(finding: &Finding) -> Key {
    (
        finding.pattern.clone(),
        finding.file.clone(),
        finding.digest.clone(),
    )
}

/// The occurrences the baseline does not cover.
///
/// The baseline records how many occurrences of each content key were
/// adjudicated; the scan's occurrences beyond that count are new. The
/// baseline cannot say *which* of several identical occurrences is
/// the new one, so the tail of each over-full key is reported — the
/// count is what decides pass or fail.
fn uncovered(fresh: &[Finding], baseline: &BTreeMap<Key, usize>) -> Vec<Finding> {
    let mut grouped: BTreeMap<Key, Vec<&Finding>> = BTreeMap::new();
    for finding in fresh {
        grouped.entry(key_of(finding)).or_default().push(finding);
    }
    let mut out = Vec::new();
    for (key, mut group) in grouped {
        let allowed = baseline.get(&key).copied().unwrap_or(0);
        if group.len() > allowed {
            group.sort_by_key(|finding| finding.line);
            out.extend(group.split_off(allowed).into_iter().cloned());
        }
    }
    out
}

/// The baseline keys whose recorded count exceeds the scan's — the
/// findings that are no longer in the tree.
fn stale(fresh: &[Finding], baseline: &BTreeMap<Key, usize>) -> Vec<Key> {
    let counts = counts_of(fresh);
    let mut stale: Vec<Key> = Vec::new();
    for (key, allowed) in baseline {
        if counts.get(key).copied().unwrap_or(0) < *allowed {
            stale.push(key.clone());
        }
    }
    stale
}

/// Renders findings for the failure message.
fn render_findings(findings: &[Finding]) -> String {
    let mut out = String::new();
    for finding in findings {
        out.push_str(&format!(
            "  {} {}:{}  {}\n",
            finding.pattern,
            finding.file,
            finding.line,
            &finding.digest[..finding.digest.len().min(12)]
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// A finding with the given content key.
    fn finding(pattern: &str, file: &str, digest: &str, line: u64) -> Finding {
        Finding {
            pattern: pattern.to_owned(),
            file: file.to_owned(),
            digest: digest.to_owned(),
            line,
        }
    }

    /// A baseline holding one occurrence of a key.
    fn baseline_of(keys: &[(&str, &str, &str, usize)]) -> BTreeMap<Key, usize> {
        let mut counts = BTreeMap::new();
        for (pattern, file, digest, count) in keys {
            counts.insert(
                (
                    (*pattern).to_owned(),
                    (*file).to_owned(),
                    (*digest).to_owned(),
                ),
                *count,
            );
        }
        counts
    }

    #[test]
    fn a_line_shift_is_not_a_new_finding() {
        // The baselined finding sat at line 412; a prose edit pushed it
        // to line 420. Same content, same file, same pattern — not new.
        let baseline = baseline_of(&[("path-traversal", "./docs/HANDOFF.md", "aaa", 1)]);
        let fresh = vec![finding("path-traversal", "./docs/HANDOFF.md", "aaa", 420)];
        assert!(uncovered(&fresh, &baseline).is_empty());
    }

    #[test]
    fn a_new_literal_is_a_new_finding() {
        let baseline = baseline_of(&[("path-traversal", "./docs/HANDOFF.md", "aaa", 1)]);
        let fresh = vec![
            finding("path-traversal", "./docs/HANDOFF.md", "aaa", 420),
            finding("path-traversal", "./docs/HANDOFF.md", "bbb", 421),
        ];
        let new = uncovered(&fresh, &baseline);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].digest, "bbb");
    }

    #[test]
    fn the_same_literal_in_a_new_file_is_new() {
        // The digest is content-only, so the file is what makes this
        // finding new rather than the same one moved.
        let baseline = baseline_of(&[("zip-code", "./crates/io/src/bitmap.rs", "ccc", 1)]);
        let fresh = vec![
            finding("zip-code", "./crates/io/src/bitmap.rs", "ccc", 90),
            finding("zip-code", "./site/app.js", "ccc", 3),
        ];
        let new = uncovered(&fresh, &baseline);
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].file, "./site/app.js");
    }

    #[test]
    fn an_extra_copy_of_a_known_literal_is_new() {
        let baseline = baseline_of(&[("pci-cardholder-data", "./canvas.rs", "ddd", 2)]);
        let fresh = vec![
            finding("pci-cardholder-data", "./canvas.rs", "ddd", 10),
            finding("pci-cardholder-data", "./canvas.rs", "ddd", 20),
            finding("pci-cardholder-data", "./canvas.rs", "ddd", 30),
        ];
        assert_eq!(uncovered(&fresh, &baseline).len(), 1);
    }

    #[test]
    fn a_deleted_finding_is_not_a_failure() {
        let baseline = baseline_of(&[("zip-code", "./crates/io/src/bitmap.rs", "ccc", 2)]);
        let fresh = vec![finding("zip-code", "./crates/io/src/bitmap.rs", "ccc", 90)];
        assert!(uncovered(&fresh, &baseline).is_empty());
        assert_eq!(stale(&fresh, &baseline).len(), 1);
    }

    #[test]
    fn an_identical_scan_matches_its_own_baseline() {
        let findings = vec![
            finding("zip-code", "./crates/io/src/bitmap.rs", "ccc", 90),
            finding("zip-code", "./crates/io/src/bitmap.rs", "ccc", 95),
            finding("path-traversal", "./docs/HANDOFF.md", "aaa", 420),
        ];
        let baseline = baseline_of(&[
            ("zip-code", "./crates/io/src/bitmap.rs", "ccc", 2),
            ("path-traversal", "./docs/HANDOFF.md", "aaa", 1),
        ]);
        assert!(uncovered(&findings, &baseline).is_empty());
        assert!(stale(&findings, &baseline).is_empty());
    }

    #[test]
    fn the_baseline_round_trips() {
        let counts = baseline_of(&[
            ("zip-code", "./crates/io/src/bitmap.rs", "ccc", 2),
            ("path-traversal", "./docs/HANDOFF.md", "aaa", 1),
        ]);
        let rendered = render_baseline(&counts);
        assert_eq!(parse_baseline(&rendered).unwrap(), counts);
    }

    #[test]
    fn a_duplicate_baseline_key_is_refused() {
        let text = "a\tb\tc\t1\na\tb\tc\t2\n";
        assert!(parse_baseline(text).is_err());
    }

    #[test]
    fn a_malformed_baseline_line_is_refused() {
        assert!(parse_baseline("a\tb\tc\n").is_err());
        assert!(parse_baseline("a\tb\tc\tx\n").is_err());
    }

    #[test]
    fn a_class_reason_is_rendered_above_its_rows() {
        let counts = baseline_of(&[
            ("zip-code", "./crates/io/src/bitmap.rs", "aaa", 1),
            ("zip-code", "./docs/NEXT.md", "bbb", 2),
            ("a-rule-with-no-note", "./docs/NEXT.md", "ccc", 1),
        ]);
        let rendered = render_baseline(&counts);
        let lines: Vec<&str> = rendered.lines().collect();
        let zip = lines
            .iter()
            .position(|line| line.starts_with("zip-code\t"))
            .unwrap();
        // The reason is wrapped, so the block of comment lines above the
        // first record carries it.
        let mut block = Vec::new();
        let mut cursor = zip;
        while cursor > 0 && lines[cursor - 1].starts_with('#') {
            block.push(lines[cursor - 1]);
            cursor -= 1;
        }
        assert!(block.iter().any(|line| line.contains("5-digit")));
        // The unknown class falls back to the generic reason rather
        // than being rendered without one.
        let unknown = lines
            .iter()
            .position(|line| line.starts_with("a-rule-with-no-note\t"))
            .unwrap();
        assert!(lines[unknown - 1].contains("Individually adjudicated"));
        // Comments never become records.
        assert_eq!(parse_baseline(&rendered).unwrap(), counts);
    }

    #[test]
    fn the_scan_document_yields_content_keys() {
        let json = r#"{"findings":[{"pattern":"zip-code","severity":"low",
            "location":{"file":"./crates/io/src/bitmap.rs","line":90,"column":12},
            "fingerprint":"zip-code:./crates/io/src/bitmap.rs:90:cccc"}]}"#;
        let findings = parse_scan(json).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].pattern, "zip-code");
        assert_eq!(findings[0].file, "./crates/io/src/bitmap.rs");
        assert_eq!(findings[0].line, 90);
        assert_eq!(findings[0].digest, "cccc");
    }

    #[test]
    fn a_scan_document_without_findings_is_refused() {
        assert!(parse_scan("{}").is_err());
        assert!(parse_scan("not json").is_err());
    }
}
