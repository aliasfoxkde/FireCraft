//! Release chore (M12 task 6): `xtask changelog` regenerates
//! `docs/CHANGELOG.md` from the conventional-commit log — the same
//! committed-generated-doc discipline as parity/scorecard — and
//! verifies the release's stamps while it is at it: the Apache-2.0
//! license claim against the license file and the manifests, and one
//! workspace version across every shipped crate — plus the lock's own
//! copy of it (`stamp::verify`), which is the drift that makes a
//! published source archive unbuildable with `--locked`. The gate
//! fails the release chore when any stamp drifts.
//!
//! `xtask stamps` is the git-free half: the stamp checks alone. The
//! CI image ships no git, so the generation half cannot run there;
//! the split keeps the stamp checks running in CI instead of letting
//! them die with the generator. See decision 0086.
//!
//! The regeneration is **additive**: the log supplies the new entries
//! and the entries the committed file already carries are kept. The
//! log alone cannot reproduce the file — a PR that lands as a squash
//! merge is in `main`'s history as a single commit, so the branch's
//! own commits are unreachable from `HEAD` and a regeneration that
//! read only the log deleted every entry that arrived that way.

use crate::util::{cargo_metadata, workspace_root};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

/// Commit types in display order; anything else lands under "Other".
const SECTIONS: &[(&str, &str)] = &[
    ("feat", "Features"),
    ("fix", "Fixes"),
    ("perf", "Performance"),
    ("docs", "Documentation"),
    ("test", "Tests"),
    ("refactor", "Refactoring"),
    ("style", "Style"),
    ("ci", "Continuous Integration"),
    ("build", "Build"),
    ("bench", "Benchmarks"),
    ("chore", "Chores"),
];

/// Regenerates `docs/CHANGELOG.md`, then verifies the release stamps.
pub fn run() -> Result<(), String> {
    let (doc, commits) = generate()?;
    let path = workspace_root().join("docs/CHANGELOG.md");
    std::fs::write(&path, &doc).map_err(|e| format!("write {}: {e}", path.display()))?;
    stamps()?;
    eprintln!("[xtask] changelog: {commits} commits -> docs/CHANGELOG.md; {STAMP_SUMMARY}");
    Ok(())
}

/// What the stamp gate proves, in one line.
const STAMP_SUMMARY: &str = "license + version stamps verified";

/// The release stamps, verified without git.
///
/// The license claim and the workspace version stamps are the two
/// checks a release must pass, and neither needs the commit log:
/// `verify_licenses` reads the license file, `verify_versions` reads
/// `cargo metadata`. The changelog *generation* does need git, and the
/// CI image ships no git — so the two are split, and CI runs this
/// verb rather than the generator.
///
/// Before this split, CI ran `xtask changelog` and believed it
/// verified the stamps. It did not: the verb died at `git log`, and
/// the job step took its exit status from the last command (`cmp`),
/// so the failure was invisible and the stamp checks never ran
/// anywhere in CI. See decision 0086.
///
/// `verify_versions` reads the *resolved* versions, so it cannot see
/// the drift that matters most: `Cargo.lock` records its own copy of
/// every local package's version, and a release that stamps the
/// manifests without the lock ships a source archive that
/// `cargo build --locked` refuses to build. `stamp::verify` is the
/// git-free, cargo-free check that closes that hole, so it runs with
/// the other stamp checks.
pub fn stamps() -> Result<(), String> {
    verify_licenses()?;
    verify_versions()?;
    crate::stamp::verify()?;
    eprintln!("[xtask] stamps: {STAMP_SUMMARY}");
    Ok(())
}

/// One parsed subject: type key, optional scope, the remainder text
/// (everything after the conventional prefix).
fn parse_subject(subject: &str) -> (String, Option<String>, &str) {
    let body = subject.split_once(": ").filter(|(head, _)| {
        let head = head.trim_end_matches('!');
        let head = head.strip_suffix(')').unwrap_or(head);
        let (kind, _) = head.split_once('(').unwrap_or((head, ""));
        SECTIONS.iter().any(|(key, _)| *key == kind)
    });
    match body {
        Some((head, rest)) => {
            let head = head.trim_end_matches('!');
            let (kind, scope) = match head.split_once('(') {
                Some((kind, scope)) => (kind, Some(scope.trim_end_matches(')').to_owned())),
                None => (head, None),
            };
            (kind.to_owned(), scope, rest)
        }
        None => ("other".to_owned(), None, subject),
    }
}

/// One changelog entry: the section it belongs to and the line the
/// file carries for it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    kind: String,
    line: String,
}

/// The conventional-commit log, grouped by section.
///
/// The log is the *new* material only. It cannot be the whole
/// changelog: a PR that lands as a squash merge is in `main`'s
/// history as a single commit, so the branch's own commits are
/// unreachable from `HEAD` and never appear in the log again.
fn log_entries() -> Result<BTreeMap<String, Vec<String>>, String> {
    let output = std::process::Command::new("git")
        .args(["log", "--no-merges", "--format=%h%x09%s"])
        .current_dir(workspace_root())
        .output()
        .map_err(|e| format!("spawn git log: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git log failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let log = String::from_utf8_lossy(&output.stdout);

    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in log.lines().filter(|line| !line.is_empty()) {
        let (hash, subject) = entry
            .split_once('\t')
            .ok_or_else(|| format!("git log line without hash: {entry}"))?;
        let (kind, scope, rest) = parse_subject(subject);
        let line = match scope {
            Some(scope) => format!("- **{scope}**: {rest} ({hash})"),
            None => format!("- {rest} ({hash})"),
        };
        grouped.entry(kind).or_default().push(line);
    }
    Ok(grouped)
}

/// The entries the committed changelog already records.
///
/// The file is the historical record of what landed, including the
/// commits that a squash merge made unreachable; dropping them would
/// silently rewrite history on every regeneration.
fn existing_entries(path: &Path) -> Vec<Entry> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    let mut kind = "other".to_owned();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("### ") {
            kind = SECTIONS
                .iter()
                .find(|(_, section)| *section == title)
                .map_or_else(|| "other".to_owned(), |(key, _)| (*key).to_owned());
            continue;
        }
        if trimmed.starts_with("- ") {
            entries.push(Entry {
                kind: kind.clone(),
                line: trimmed.to_owned(),
            });
        }
    }
    entries
}

/// The commit hash an entry records, if it carries one.
fn entry_hash(line: &str) -> Option<&str> {
    let rest = line.strip_suffix(')')?;
    let (_, hash) = rest.rsplit_once(" (")?;
    Some(hash)
}

/// The merged sections: the log's new entries first (newest first),
/// then the entries the committed file already carries.
///
/// The committed file is the historical record, and the log cannot
/// reproduce all of it: a PR that lands as a squash leaves its own
/// commits unreachable from `HEAD`, so a regeneration that only read
/// the log would delete every entry that arrived that way. The
/// entries the file already carries are kept; the log only ever adds
/// to them, and never repeats an entry the file already has.
fn merge_sections(
    logged: BTreeMap<String, Vec<String>>,
    existing: Vec<Entry>,
) -> BTreeMap<String, Vec<String>> {
    let recorded: BTreeSet<String> = existing
        .iter()
        .filter_map(|entry| entry_hash(&entry.line).map(str::to_owned))
        .collect();
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (kind, lines) in logged {
        let fresh: Vec<String> = lines
            .into_iter()
            .filter(|line| match entry_hash(line) {
                Some(hash) => !recorded.contains(hash),
                None => true,
            })
            .collect();
        grouped.entry(kind).or_default().extend(fresh);
    }
    for entry in existing {
        grouped.entry(entry.kind).or_default().push(entry.line);
    }
    grouped
}

/// The generated markdown and the commit count it covers.
fn generate() -> Result<(String, usize), String> {
    let root = workspace_root();
    let path = root.join("docs/CHANGELOG.md");
    let logged = log_entries()?;
    let existing = existing_entries(&path);
    let grouped = merge_sections(logged, existing);
    let total: usize = grouped.values().map(Vec::len).sum();

    let version = crate::dist::workspace_version()?;
    let mut doc = String::from(
        "# Changelog\n\n\
         Generated by `cargo run -p xtask -- changelog` from the\n\
         conventional-commit log; do not edit by hand. Sections follow\n\
         git log order (newest first). The generation is additive: the\n\
         entries already in this file are kept, because a squash merge\n\
         leaves a branch's own commits unreachable from the log.\n\n",
    );
    let _ = writeln!(doc, "Current version: **{version}**.\n");
    doc.push_str("## Unreleased\n");
    for (key, title) in SECTIONS
        .iter()
        .map(|(k, t)| (*k, *t))
        .chain([("other", "Other")])
    {
        if let Some(lines) = grouped.get(key) {
            let _ = writeln!(doc, "\n### {title}\n");
            for line in lines {
                doc.push_str(line);
                doc.push('\n');
            }
        }
    }
    Ok((doc, total))
}

/// The Apache-2.0 claim: `LICENSE` carries the license text and the
/// SPDX identifier; the workspace manifest declares the same
/// identifier. The MIT text must stay gone — `FireCraft` is licensed
/// under Apache-2.0 alone.
fn verify_licenses() -> Result<(), String> {
    let root = workspace_root();
    let lic =
        std::fs::read_to_string(root.join("LICENSE")).map_err(|e| format!("read LICENSE: {e}"))?;
    for needle in ["Apache License", "Apache-2.0"] {
        if !lic.contains(needle) {
            return Err(format!(
                "LICENSE no longer states the Apache-2.0 claim ({needle} missing)"
            ));
        }
    }
    if lic.contains("MIT License") {
        return Err("LICENSE still carries the MIT text; FireCraft is Apache-2.0 only".to_string());
    }
    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|e| format!("read Cargo.toml: {e}"))?;
    if !manifest.contains("license = \"Apache-2.0\"") {
        return Err(
            "the workspace manifest no longer declares license = \"Apache-2.0\"".to_string(),
        );
    }
    Ok(())
}

/// One version across every workspace-local package: any member that
/// carries its own stamp must equal the workspace version the dist
/// artifacts are named with.
fn verify_versions() -> Result<(), String> {
    let meta = cargo_metadata()?;
    let packages = meta["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata: no packages".to_owned())?;
    let mut stamps: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for package in packages {
        if package["source"].is_null() {
            let name = package["name"].as_str().unwrap_or("?").to_owned();
            let version = package["version"].as_str().unwrap_or("?").to_owned();
            stamps.entry(version).or_default().push(name);
        }
    }
    if stamps.len() != 1 {
        let detail = stamps
            .iter()
            .map(|(version, names)| format!("  {version}: {}", names.join(", ")))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "workspace version stamps disagree — every crate must carry the \
             workspace version:\n{detail}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{Entry, entry_hash, existing_entries, merge_sections, parse_subject};
    use std::collections::BTreeMap;
    use std::path::Path;

    /// A committed changelog body with two sections.
    const FILE: &str = "\
# Changelog

Generated by `cargo run -p xtask -- changelog`.

Current version: **0.2.5**.

## Unreleased

### Features

- **app**: the animation timeline (dcb1e48)
- **app**: in-canvas text editing (ed82eae)

### Fixes

- **xtask**: the sandbox race (fa88ebd)
";

    /// The entries the committed file records, in file order.
    fn entries() -> Vec<Entry> {
        existing_entries_of(FILE)
    }

    /// The entries a body records.
    fn existing_entries_of(text: &str) -> Vec<Entry> {
        let path = std::env::temp_dir().join(format!(
            "firecraft-changelog-{}-{}.md",
            std::process::id(),
            text.len()
        ));
        std::fs::write(&path, text).expect("the fixture must be written");
        let entries = existing_entries(&path);
        let _ = std::fs::remove_file(&path);
        entries
    }

    #[test]
    fn parses_type_scope_subject() {
        let (kind, scope, rest) = parse_subject("feat(m12): dist web with bridge smoke");
        assert_eq!(rest, "dist web with bridge smoke");
        assert_eq!(kind, "feat");
        assert_eq!(scope.as_deref(), Some("m12"));
    }

    #[test]
    fn parses_type_without_scope() {
        let (kind, scope, rest) =
            parse_subject("ci: restore fuzz step, document offline cache rule");
        assert_eq!(rest, "restore fuzz step, document offline cache rule");
        assert_eq!(kind, "ci");
        assert_eq!(scope, None);
    }

    #[test]
    fn breaking_mark_is_dropped_not_kept() {
        let (kind, scope, _) = parse_subject("feat(m0)!: bootstrap");
        assert_eq!(kind, "feat");
        assert_eq!(scope.as_deref(), Some("m0"));
    }

    #[test]
    fn unknown_subject_lands_in_other() {
        let (kind, scope, rest) = parse_subject("Initial commit");
        assert_eq!(rest, "Initial commit");
        assert_eq!(kind, "other");
        assert_eq!(scope, None);
    }

    #[test]
    fn prefix_lookalikes_are_not_types() {
        // "feat-ish" without the colon separator must not parse as feat.
        let (kind, _, rest) = parse_subject("featish(x): not a conventional subject");
        assert_eq!(rest, "featish(x): not a conventional subject");
        assert_eq!(kind, "other");
    }

    #[test]
    fn the_file_entries_keep_their_section() {
        let entries = entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].kind, "feat");
        assert_eq!(
            entries[0].line,
            "- **app**: the animation timeline (dcb1e48)"
        );
        assert_eq!(entries[2].kind, "fix");
        assert_eq!(entries[2].line, "- **xtask**: the sandbox race (fa88ebd)");
    }

    #[test]
    fn an_entry_hash_is_read_off_its_line() {
        assert_eq!(
            entry_hash("**app**: the animation timeline (dcb1e48)"),
            Some("dcb1e48")
        );
        // A line without a hash carries none.
        assert_eq!(entry_hash("- **app**: the animation timeline"), None);
    }

    #[test]
    fn a_missing_file_records_nothing() {
        let missing = Path::new("/nonexistent/firecraft-changelog.md");
        assert!(existing_entries(missing).is_empty());
    }

    #[test]
    fn the_merge_keeps_the_entries_the_log_can_no_longer_see() {
        // The log holds only the squash commit; the file still holds
        // the branch's own commits. The merge must not drop them.
        let mut logged = BTreeMap::new();
        logged.insert(
            "feat".to_owned(),
            vec!["- **app**: the timeline (#118) (abc1234)".to_owned()],
        );
        let existing = vec![
            Entry {
                kind: "feat".to_owned(),
                line: "- **app**: the animation timeline (dcb1e48)".to_owned(),
            },
            Entry {
                kind: "fix".to_owned(),
                line: "- **xtask**: the sandbox race (fa88ebd)".to_owned(),
            },
        ];
        let merged = merge_sections(logged, existing);
        let features = merged.get("feat").expect("the feature section");
        assert_eq!(features.len(), 2);
        assert!(features[0].contains("abc1234"));
        assert!(features[1].contains("dcb1e48"));
        // And the section the log says nothing about survives.
        assert_eq!(merged.get("fix").map(Vec::len), Some(1));
    }

    #[test]
    fn the_merge_adds_what_the_file_lacks_and_never_repeats_it() {
        let mut logged = BTreeMap::new();
        logged.insert(
            "feat".to_owned(),
            vec![
                "- **app**: a new thing (aaaaaaa)".to_owned(),
                "- **app**: the animation timeline (dcb1e48)".to_owned(),
            ],
        );
        let existing = vec![Entry {
            kind: "feat".to_owned(),
            line: "- **app**: the animation timeline (dcb1e48)".to_owned(),
        }];
        let merged = merge_sections(logged, existing);
        let features = merged.get("feat").expect("the feature section");
        assert_eq!(features.len(), 2);
        // The new entry is in, the one the file already had is not
        // repeated.
        assert!(features[0].contains("aaaaaaa"));
        assert_eq!(
            features
                .iter()
                .filter(|line| line.contains("dcb1e48"))
                .count(),
            1
        );
    }
}
