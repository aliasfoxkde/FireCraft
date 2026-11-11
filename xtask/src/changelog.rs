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
//!
//! The file has two kinds of content: the *released* sections, each
//! opened by a `## <version>` heading, and the generated *unreleased*
//! block. The released sections are history: they are preserved
//! verbatim, and the commits they record are not re-offered to the
//! unreleased block. Two verbs bracket the release: `--check` fails
//! when the committed file is not what the generator produces, and
//! `--cut <version>` freezes the unreleased block into a released
//! version section. See decision 0094.

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

/// The heading that opens the generated block.
const UNRELEASED_HEADING: &str = "## Unreleased";

/// Regenerates `docs/CHANGELOG.md`, then verifies the release stamps.
pub fn run() -> Result<(), String> {
    let (doc, unreleased, released) = generate()?;
    let path = workspace_root().join("docs/CHANGELOG.md");
    std::fs::write(&path, &doc).map_err(|e| format!("write {}: {e}", path.display()))?;
    stamps()?;
    eprintln!(
        "[xtask] changelog: {unreleased} unreleased entries, {released} in released sections \
         -> docs/CHANGELOG.md; {STAMP_SUMMARY}"
    );
    Ok(())
}

/// The freshness gate: the committed changelog must be exactly what
/// the generator produces for every commit but the tip.
///
/// This is a *release* gate, not a CI gate. CI runs the generator
/// (`run`), which brings the file up to date with the log; this verb
/// asks the opposite question — is the file that is committed current?
/// It fails when the committed file is stale, so a release cannot
/// ship release notes that omit the commits it is releasing.
///
/// The tip commit is exempt, and it has to be: a file cannot record
/// the hash of the commit that writes it. The release flow therefore
/// regenerates the notes in the release commit itself, and the
/// exemption covers exactly that one commit.
///
/// It needs git (the log), like the generator, so it cannot run in
/// the CI image. See decision 0086.
pub fn check() -> Result<(), String> {
    let (doc, _, _) = generate_with(true)?;
    let path = workspace_root().join("docs/CHANGELOG.md");
    let committed =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    if committed == doc {
        return Ok(());
    }
    Err(format!(
        "docs/CHANGELOG.md is not what the generator produces.\n\
         Run `cargo run -p xtask -- changelog` and commit the result.\n{}",
        first_difference(&committed, &doc)
    ))
}

/// Freezes the unreleased block into a released version section.
///
/// The release order is: stamp the workspace to the version first
/// (`xtask stamp <version>`), then cut the changelog at that same
/// version — the cut refuses a version the workspace does not carry,
/// because a changelog section and a "Current version" line that
/// disagree identify nothing. The cut moves the unreleased entries
/// into a `## <version>` section, verbatim; the next regeneration
/// starts an empty unreleased block and adds only what lands after
/// the cut.
pub fn cut(version: &str) -> Result<(), String> {
    let version = version.trim();
    if version.is_empty() || version.starts_with('-') {
        return Err("the version to cut must be a version string, e.g. 0.2.6".to_owned());
    }
    let workspace_version = crate::dist::workspace_version()?;
    if version != workspace_version {
        return Err(format!(
            "the version to cut ({version}) is not the workspace version ({workspace_version}).\n\
             Stamp the workspace first: `cargo run -p xtask -- stamp {version}`"
        ));
    }
    let path = workspace_root().join("docs/CHANGELOG.md");
    let (doc, _, _) = generate()?;
    let refreshed = Committed::parse(&doc);
    if refreshed.entries.is_empty() {
        return Err("the unreleased block is empty; there is nothing to cut".to_owned());
    }
    if refreshed
        .frozen
        .iter()
        .any(|line| line.trim() == format!("## {version}"))
    {
        return Err(format!(
            "the changelog already carries a released section for {version}"
        ));
    }
    let grouped = group_entries(&refreshed.entries);
    let mut frozen = refreshed.frozen.clone();
    frozen.push(format!("## {version}"));
    frozen.push(String::new());
    let mut block = String::new();
    push_sections(&mut block, &grouped);
    for line in block.trim_start_matches('\n').lines() {
        frozen.push(line.to_owned());
    }
    let cut_doc = render(&workspace_version, &frozen, &BTreeMap::new());
    std::fs::write(&path, &cut_doc).map_err(|e| format!("write {}: {e}", path.display()))?;
    eprintln!(
        "[xtask] changelog: cut {version} — {} entries frozen into a released section",
        refreshed.entries.len()
    );
    Ok(())
}

/// The first line where the committed file and the generated one
/// differ, so a stale file says *where* it went wrong.
fn first_difference(committed: &str, generated: &str) -> String {
    for (number, (left, right)) in committed.lines().zip(generated.lines()).enumerate() {
        if left != right {
            return format!(
                "first difference at line {}:\n  committed: {left}\n  generated: {right}",
                number + 1
            );
        }
    }
    format!(
        "the committed file has {} lines, the generated one {}",
        committed.lines().count(),
        generated.lines().count()
    )
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

/// Which part of the file a line belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    /// The generated header, before any section.
    Header,
    /// A released section, opened by a `## <version>` heading.
    Released,
    /// The generated unreleased block.
    Unreleased,
}

/// The committed changelog, parsed.
///
/// The released sections are history and are kept verbatim; only the
/// unreleased block is regenerated. The recorded hashes span both, so
/// a commit that a released section already accounts for is not
/// offered to the unreleased block a second time.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Committed {
    /// The released sections, line by line, verbatim.
    frozen: Vec<String>,
    /// The unreleased block's entries, with the section they belong to.
    entries: Vec<Entry>,
    /// Every commit hash the file records, released sections included.
    recorded: BTreeSet<String>,
}

impl Committed {
    /// Parses a committed changelog.
    fn parse(text: &str) -> Committed {
        let mut committed = Committed::default();
        let mut kind = "other".to_owned();
        let mut region = Region::Header;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("## ") {
                region = if trimmed == UNRELEASED_HEADING {
                    Region::Unreleased
                } else {
                    Region::Released
                };
                kind = "other".to_owned();
            }
            match region {
                Region::Released => {
                    committed.frozen.push(line.to_owned());
                    // A released section's commits are recorded too, so
                    // the log does not offer them to the unreleased
                    // block a second time.
                    if trimmed.starts_with("- ") {
                        record_hash(&mut committed, trimmed);
                    }
                }
                Region::Unreleased => {
                    if let Some(title) = trimmed.strip_prefix("### ") {
                        kind = kind_for_section(title);
                    } else if trimmed.starts_with("- ") {
                        record_hash(&mut committed, trimmed);
                        committed.entries.push(Entry {
                            kind: kind.clone(),
                            line: trimmed.to_owned(),
                        });
                    }
                }
                Region::Header => {}
            }
        }
        while committed
            .frozen
            .last()
            .is_some_and(|line| line.trim().is_empty())
        {
            committed.frozen.pop();
        }
        committed
    }

    /// Reads the committed changelog; a missing file records nothing.
    fn read(path: &Path) -> Committed {
        std::fs::read_to_string(path)
            .map_or_else(|_| Committed::default(), |text| Committed::parse(&text))
    }
}

/// Records an entry line's commit hash, if it carries one.
fn record_hash(committed: &mut Committed, line: &str) {
    if let Some(hash) = entry_hash(line) {
        committed.recorded.insert(hash.to_owned());
    }
}

/// The section kind a `### Title` heading belongs to; an unknown
/// title lands under "Other".
fn kind_for_section(title: &str) -> String {
    SECTIONS
        .iter()
        .find(|(_, section)| *section == title)
        .map_or_else(|| "other".to_owned(), |(key, _)| (*key).to_owned())
}

/// The conventional-commit log, grouped by section.
///
/// The log is the *new* material only. It cannot be the whole
/// changelog: a PR that lands as a squash merge is in `main`'s
/// history as a single commit, so the branch's own commits are
/// unreachable from `HEAD` and never appear in the log again.
///
/// `exclude_tip` drops the tip commit's entry. A file cannot record
/// the hash of the commit that writes it, so the freshness gate
/// compares against the log without the tip.
fn log_entries(exclude_tip: bool) -> Result<BTreeMap<String, Vec<String>>, String> {
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
    let tip = if exclude_tip {
        Some(head_hash()?)
    } else {
        None
    };

    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in log.lines().filter(|line| !line.is_empty()) {
        let (hash, subject) = entry
            .split_once('\t')
            .ok_or_else(|| format!("git log line without hash: {entry}"))?;
        if tip.as_deref() == Some(hash) {
            continue;
        }
        let (kind, scope, rest) = parse_subject(subject);
        let line = match scope {
            Some(scope) => format!("- **{scope}**: {rest} ({hash})"),
            None => format!("- {rest} ({hash})"),
        };
        grouped.entry(kind).or_default().push(line);
    }
    Ok(grouped)
}

/// The tip commit's short hash.
fn head_hash() -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(workspace_root())
        .output()
        .map_err(|e| format!("spawn git rev-parse: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
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
/// to them, and never repeats an entry the file already has — including
/// an entry a released section already accounts for.
fn merge_sections(
    logged: BTreeMap<String, Vec<String>>,
    committed: &Committed,
) -> BTreeMap<String, Vec<String>> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (kind, lines) in logged {
        let fresh: Vec<String> = lines
            .into_iter()
            .filter(|line| match entry_hash(line) {
                Some(hash) => !committed.recorded.contains(hash),
                None => true,
            })
            .collect();
        // A kind whose every entry the file already records must not
        // leave an empty section behind.
        if !fresh.is_empty() {
            grouped.entry(kind).or_default().extend(fresh);
        }
    }
    for entry in &committed.entries {
        grouped
            .entry(entry.kind.clone())
            .or_default()
            .push(entry.line.clone());
    }
    grouped
}

/// The entries of one document, grouped by section.
fn group_entries(entries: &[Entry]) -> BTreeMap<String, Vec<String>> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in entries {
        grouped
            .entry(entry.kind.clone())
            .or_default()
            .push(entry.line.clone());
    }
    grouped
}

/// Appends the sections, in display order, to a document.
fn push_sections(doc: &mut String, grouped: &BTreeMap<String, Vec<String>>) {
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
}

/// The generated markdown and the entry count it covers.
fn render(version: &str, frozen: &[String], grouped: &BTreeMap<String, Vec<String>>) -> String {
    let mut doc = String::from(
        "# Changelog\n\n\
         Generated by `cargo run -p xtask -- changelog` from the\n\
         conventional-commit log; do not edit by hand. Sections follow\n\
         git log order (newest first). The generation is additive: the\n\
         entries already in this file are kept, because a squash merge\n\
         leaves a branch's own commits unreachable from the log.\n\n",
    );
    let _ = writeln!(doc, "Current version: **{version}**.\n");
    for line in frozen {
        doc.push_str(line);
        doc.push('\n');
    }
    if !frozen.is_empty() {
        doc.push('\n');
    }
    doc.push_str(UNRELEASED_HEADING);
    doc.push('\n');
    push_sections(&mut doc, grouped);
    doc
}

/// The generated markdown, the unreleased entry count, and the count
/// of the entries the released sections already carry.
fn generate() -> Result<(String, usize, usize), String> {
    generate_with(false)
}

/// The generated markdown, over the whole log or over the log
/// without the tip commit.
fn generate_with(exclude_tip: bool) -> Result<(String, usize, usize), String> {
    let root = workspace_root();
    let path = root.join("docs/CHANGELOG.md");
    let committed = Committed::read(&path);
    let logged = log_entries(exclude_tip)?;
    let grouped = merge_sections(logged, &committed);
    let unreleased: usize = grouped.values().map(Vec::len).sum();
    let released = committed
        .frozen
        .iter()
        .filter(|line| line.trim().starts_with("- "))
        .count();
    let version = crate::dist::workspace_version()?;
    let doc = render(&version, &committed.frozen, &grouped);
    Ok((doc, unreleased, released))
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
    use super::{
        Committed, Entry, UNRELEASED_HEADING, entry_hash, group_entries, kind_for_section,
        merge_sections, parse_subject, push_sections, render,
    };
    use std::collections::BTreeMap;

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

    /// A committed changelog with a released section in front of the
    /// unreleased block.
    const RELEASED: &str = "\
# Changelog

Current version: **0.2.5**.

## 0.2.5

### Features

- **app**: the animation timeline (dcb1e48)

## Unreleased

### Fixes

- **xtask**: the sandbox race (fa88ebd)
";

    /// The entries a body records in its unreleased block.
    fn entries_of(text: &str) -> Vec<Entry> {
        Committed::parse(text).entries
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
        let entries = entries_of(FILE);
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
    fn an_unknown_section_title_lands_in_other() {
        assert_eq!(kind_for_section("Features"), "feat");
        assert_eq!(kind_for_section("Whatever"), "other");
    }

    #[test]
    fn an_entry_hash_is_read_off_its_line() {
        assert_eq!(
            entry_hash("- **app**: the animation timeline (dcb1e48)"),
            Some("dcb1e48")
        );
        // A line without a hash carries none.
        assert_eq!(entry_hash("- **app**: the animation timeline"), None);
    }

    #[test]
    fn a_missing_file_records_nothing() {
        let missing = std::path::Path::new("/nonexistent/firecraft-changelog.md");
        assert!(Committed::read(missing).entries.is_empty());
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
        let committed = Committed::parse(FILE);
        let merged = merge_sections(logged, &committed);
        let features = merged.get("feat").expect("the feature section");
        assert_eq!(features.len(), 3);
        assert!(features[0].contains("abc1234"));
        assert!(features.iter().any(|line| line.contains("dcb1e48")));
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
        let committed = Committed::parse(FILE);
        let merged = merge_sections(logged, &committed);
        let features = merged.get("feat").expect("the feature section");
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

    #[test]
    fn a_released_section_survives_a_regeneration() {
        let committed = Committed::parse(RELEASED);
        // The released section is kept verbatim...
        assert!(
            committed
                .frozen
                .iter()
                .any(|line| line.trim() == "## 0.2.5")
        );
        // ...and it is not part of the unreleased block.
        assert_eq!(committed.entries.len(), 1);
        assert_eq!(committed.entries[0].kind, "fix");
        // Its commit is recorded, so the log does not offer it again.
        assert!(committed.recorded.contains("dcb1e48"));
        let mut logged = BTreeMap::new();
        logged.insert(
            "feat".to_owned(),
            vec!["- **app**: the animation timeline (dcb1e48)".to_owned()],
        );
        let merged = merge_sections(logged, &committed);
        assert!(!merged.contains_key("feat"));
    }

    #[test]
    fn the_rendered_document_reparses_to_the_same_entries() {
        let committed = Committed::parse(RELEASED);
        let grouped = group_entries(&committed.entries);
        let doc = render("0.2.5", &committed.frozen, &grouped);
        let reparsed = Committed::parse(&doc);
        assert_eq!(reparsed.entries, committed.entries);
        assert_eq!(reparsed.frozen, committed.frozen);
    }

    #[test]
    fn the_cut_moves_the_unreleased_entries_into_the_version_section() {
        // The cut is the render of a version section built from the
        // unreleased entries, with an empty unreleased block after it.
        let committed = Committed::parse(FILE);
        let grouped = group_entries(&committed.entries);
        let mut block = String::new();
        push_sections(&mut block, &grouped);
        let mut frozen = vec!["## 0.2.6".to_owned(), String::new()];
        for line in block.trim_start_matches('\n').lines() {
            frozen.push(line.to_owned());
        }
        let doc = render("0.2.6", &frozen, &BTreeMap::new());
        let cut = Committed::parse(&doc);
        // Every entry moved into the released section...
        assert_eq!(cut.entries.len(), 0);
        assert_eq!(cut.recorded.len(), 3);
        assert!(cut.frozen.iter().any(|line| line.trim() == "## 0.2.6"));
        assert!(
            cut.frozen
                .iter()
                .any(|line| line.contains("the animation timeline"))
        );
        // ...and the unreleased block that follows it is empty.
        let (_, unreleased) = doc
            .split_once(UNRELEASED_HEADING)
            .expect("the unreleased block");
        assert!(!unreleased.contains("- **app**"));
        assert!(!unreleased.contains("- **xtask**"));
    }
}
