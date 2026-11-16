//! Altimeter: measures the `SCOREBOARD.tsv` metrics from the tree.
//!
//! Every number the scoreboard carries is computed here from the
//! repository itself — measured, never transcribed. Metrics that depend
//! on state outside the tree (CI status, the live deploy, the issue
//! tracker, a coverage run) are written as `n/a` rather than copied from
//! an older snapshot, because a copied number is how the board drifted
//! to a 164 that no command produced.
//!
//! `climb --check` holds the ratchet: a metric that moved the wrong way
//! against the last recorded row fails.

use std::path::Path;

use crate::{reachability, util};

/// The scoreboard columns, in the order `SCOREBOARD.tsv` declares them.
#[derive(Debug, Clone)]
pub struct Row {
    date: String,
    commit: String,
    rung: String,
    gate: String,
    s2_bound: usize,
    s2_registry: usize,
    s3_reportgap: usize,
    s4_inert: usize,
    s5a_impl: usize,
    s5b_partial: usize,
    s5c_missing: usize,
    s6_tests: usize,
    s7_godfiles: usize,
    s8_wasm_raw: Option<usize>,
    s9_live_traceable: String,
    s10_smoke_receipts: String,
    s11_open_issues: String,
    s12_branches: String,
    s13_coverage: String,
    note: String,
}

/// `xtask climb [rung]`: print one measured row to stdout.
pub fn emit(rung: Option<&str>) -> Result<(), String> {
    let row = measure(rung.unwrap_or("measure"))?;
    util::write_stdout_line(&row.to_tsv())?;
    eprintln!(
        "[xtask] climb: measured {} ({} bound of {})",
        row.rung, row.s2_bound, row.s2_registry
    );
    Ok(())
}

/// `xtask climb --append [rung]`: append the measured row to the board.
pub fn append(rung: Option<&str>) -> Result<(), String> {
    let row = measure(rung.unwrap_or("measure"))?;
    let path = util::workspace_root().join("docs/climb/SCOREBOARD.tsv");
    let mut text = std::fs::read_to_string(&path).map_err(|e| format!("read scoreboard: {e}"))?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&row.to_tsv());
    text.push('\n');
    std::fs::write(&path, text).map_err(|e| format!("write scoreboard: {e}"))?;
    eprintln!(
        "[xtask] climb --append: recorded {} at {}",
        row.rung, row.commit
    );
    Ok(())
}

/// `xtask climb --check`: fail when a ratcheted metric moved backwards.
///
/// A gate stays off stdout: the measured row goes to stdout only when a
/// human asks for it, so `ci` keeps its data channel clean.
pub fn check() -> Result<(), String> {
    let row = measure("check")?;
    let base = baseline()?;
    let regressions = regressions(&row, &base);
    if regressions.is_empty() {
        eprintln!(
            "[xtask] climb --check: every ratcheted metric holds {}",
            base.commit
        );
        return Ok(());
    }
    Err(format!(
        "climb --check: {} metric(s) moved backwards against {}:\n  {}",
        regressions.len(),
        base.commit,
        regressions.join("\n  ")
    ))
}

/// The metrics that moved the wrong way against `base`.
///
/// Split from `check` so the direction rules — and the set of metrics
/// that carries one — are testable without running a measurement.
fn regressions(row: &Row, base: &Baseline) -> Vec<String> {
    let mut out = Vec::new();

    // (name, now, was): a value that must never fall.
    for (name, now, was) in [
        ("S2_bound", row.s2_bound, base.s2_bound),
        ("S5a_impl", row.s5a_impl, base.s5a_impl),
        ("S6_tests", row.s6_tests, base.s6_tests),
    ] {
        if now < was {
            out.push(format!("{name} fell {was} -> {now}"));
        }
    }

    // A value that must never rise.
    for (name, now, was) in [
        ("S3_reportgap", row.s3_reportgap, base.s3_reportgap),
        ("S4_inert", row.s4_inert, base.s4_inert),
        ("S5b_partial", row.s5b_partial, base.s5b_partial),
        ("S5c_missing", row.s5c_missing, base.s5c_missing),
        ("S7_godfiles", row.s7_godfiles, base.s7_godfiles),
    ] {
        if now > was {
            out.push(format!("{name} rose {was} -> {now}"));
        }
    }

    // S12 is measured from the remotes, so it is `n/a` when no remote
    // answers — the CI image is offline by contract. A metric that could
    // not be measured has not moved: it is not compared.
    if let (Ok(now), Some(was)) = (row.s12_branches.parse::<usize>(), base.s12_branches)
        && now > was
    {
        out.push(format!("S12_branches rose {was} -> {now}"));
    }

    if let (Some(now), Some(was)) = (row.s8_wasm_raw, base.s8_wasm_raw)
        && now > was
    {
        out.push(format!("S8_wasm_raw rose {was} -> {now}"));
    }

    out
}

/// One measurement of the whole board.
fn measure(rung: &str) -> Result<Row, String> {
    let root = util::workspace_root();
    let registry = engine::catalog::ids();
    let bound = reachability::bound_commands(&root.join("crates/app/src"), &registry)?;
    let (_total, inert) = app::menus::menu_census();
    let parity = std::fs::read_to_string(root.join("docs/CS6-PARITY.md"))
        .map_err(|e| format!("read docs/CS6-PARITY.md: {e}"))?;
    let menus = std::fs::read_to_string(root.join("crates/app/src/menus.rs"))
        .map_err(|e| format!("read crates/app/src/menus.rs: {e}"))?;

    Ok(Row {
        date: capture_line(&root, "date", &["-u", "+%F"])?,
        commit: capture_line(&root, "git", &["rev-parse", "--short", "HEAD"])?,
        rung: rung.to_owned(),
        gate: "n/a".to_owned(),
        s2_bound: bound.len(),
        s2_registry: registry.len(),
        s3_reportgap: menus.matches("ReportGap(").count(),
        s4_inert: inert,
        s5a_impl: parity.matches("| **implemented**").count(),
        s5b_partial: parity.matches("| **partial**").count(),
        s5c_missing: parity.matches("| **missing**").count(),
        s6_tests: test_count(&root)?,
        s7_godfiles: godfiles(&root),
        s8_wasm_raw: wasm_bytes(&root),
        s9_live_traceable: "n/a".to_owned(),
        s10_smoke_receipts: "n/a".to_owned(),
        s11_open_issues: "n/a".to_owned(),
        s12_branches: remote_branches(&root),
        s13_coverage: "n/a".to_owned(),
        note: if rung == "check" {
            "measured by xtask climb --check; S1/S9/S10/S11/S13 need external state".to_owned()
        } else {
            "measured by xtask climb; S1/S9/S10/S11/S13 need external state".to_owned()
        },
    })
}

/// The last recorded row of the board.
struct Baseline {
    commit: String,
    s2_bound: usize,
    s3_reportgap: usize,
    s4_inert: usize,
    s5a_impl: usize,
    s5b_partial: usize,
    s5c_missing: usize,
    s6_tests: usize,
    s7_godfiles: usize,
    s8_wasm_raw: Option<usize>,
    s12_branches: Option<usize>,
}

fn baseline() -> Result<Baseline, String> {
    let path = util::workspace_root().join("docs/climb/SCOREBOARD.tsv");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read scoreboard: {e}"))?;
    let row = text
        .lines()
        .rfind(|line| {
            !line.starts_with('#') && !line.trim().is_empty() && line.split('\t').count() == 20
        })
        .ok_or_else(|| "scoreboard carries no data row".to_owned())?;
    let f: Vec<&str> = row.split('\t').collect();
    Ok(Baseline {
        commit: f.get(1).copied().unwrap_or("?").to_owned(),
        s2_bound: number(f.get(4)),
        s3_reportgap: number(f.get(6)),
        s4_inert: number(f.get(7)),
        s5a_impl: number(f.get(8)),
        s5b_partial: number(f.get(9)),
        s5c_missing: number(f.get(10)),
        s6_tests: number(f.get(11)),
        s7_godfiles: number(f.get(12)),
        s8_wasm_raw: f.get(13).and_then(|v| v.parse().ok()),
        s12_branches: f.get(17).and_then(|v| v.parse().ok()),
    })
}

fn number(cell: Option<&&str>) -> usize {
    cell.and_then(|v| v.parse().ok()).unwrap_or(0)
}

fn capture_line(root: &Path, program: &str, args: &[&str]) -> Result<String, String> {
    let (stdout, stderr) = util::capture_in(Some(root), program, args)?;
    let line = stdout.trim();
    if line.is_empty() {
        return Err(format!(
            "{program} {} produced no output: {stderr}",
            args.join(" ")
        ));
    }
    Ok(line.to_owned())
}

fn test_count(root: &Path) -> Result<usize, String> {
    let (stdout, stderr) = util::capture_in(
        Some(root),
        "grep",
        &[
            "-rE",
            r"#\[test\]",
            "crates",
            "xtask",
            "web",
            "--include=*.rs",
        ],
    )?;
    let count = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    if count == 0 && !stderr.trim().is_empty() {
        return Err(format!("count #[test]: {stderr}"));
    }
    Ok(count)
}

/// The number of branch heads on the remotes, besides `main`.
///
/// Measured FROM THE REMOTES, with `git ls-remote --heads`, not from the
/// clone's remote-tracking refs. The two are different numbers, and the
/// old measurement was the one that cannot mean anything:
///
/// - A clone's remote-tracking refs are whatever its last fetch brought.
///   The CI runner's checkout measured 0 remote branches on 2026-11-16
///   while a full clone measured 154, and older CI checkouts measured
///   23 and 78 — the number measured the clone, not the repository.
/// - A clone with two remotes counts every branch twice, once per
///   remote it exists on: this checkout carries 79 `origin` refs and 77
///   `gitforge` refs for 77 distinct branches.
///
/// The union over the remotes keeps the metric's meaning ("remote
/// branches besides `main`") and makes it a property of the remotes
/// rather than of the clone's fetch history.
///
/// When no remote answers — the CI image is offline by contract, and the
/// runner's checkout's remote is a host path its container does not
/// mount — the metric is `n/a`, like the other measurements that need
/// state outside the tree. A number no environment can reproduce is
/// worse than an honest gap: it makes the ratchet fire on the clone that
/// fetched most recently and stay silent on the one that did not.
fn remote_branches(root: &Path) -> String {
    let (remotes, _) = util::capture_in(Some(root), "git", &["remote"]).unwrap_or_default();
    let mut branches: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for remote in remotes.lines().map(str::trim).filter(|r| !r.is_empty()) {
        let (stdout, _) =
            util::capture_in(Some(root), "git", &["ls-remote", "--heads", remote]).unwrap_or_default();
        for line in stdout.lines() {
            let Some(reference) = line.split_whitespace().nth(1) else {
                continue;
            };
            let Some(name) = reference.strip_prefix("refs/heads/") else {
                continue;
            };
            if name == "main" || name.ends_with("/HEAD") {
                continue;
            }
            branches.insert(name.to_owned());
        }
    }
    if branches.is_empty() {
        "n/a".to_owned()
    } else {
        branches.len().to_string()
    }
}

fn wasm_bytes(root: &Path) -> Option<usize> {
    let dir = std::fs::read_dir(root.join("dist/web")).ok()?;
    let mut largest: Option<usize> = None;
    for entry in dir.filter_map(Result::ok) {
        if entry.path().extension().and_then(|e| e.to_str()) != Some("wasm") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let size = usize::try_from(meta.len()).unwrap_or(0);
        if largest.is_none_or(|best| size > best) {
            largest = Some(size);
        }
    }
    largest
}

fn godfiles(root: &Path) -> usize {
    fn walk(dir: &Path, out: &mut usize) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
                && std::fs::read_to_string(&path).is_ok_and(|text| text.lines().count() > 2000)
            {
                *out += 1;
            }
        }
    }
    let mut out = 0;
    for dir in ["crates", "web", "xtask"] {
        walk(&root.join(dir), &mut out);
    }
    out
}

impl Row {
    /// The row as one TSV line, in `SCOREBOARD.tsv` column order.
    fn to_tsv(&self) -> String {
        let wasm = self
            .s8_wasm_raw
            .map_or_else(|| "n/a".to_owned(), |b| b.to_string());
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.date,
            self.commit,
            self.rung,
            self.gate,
            self.s2_bound,
            self.s2_registry,
            self.s3_reportgap,
            self.s4_inert,
            self.s5a_impl,
            self.s5b_partial,
            self.s5c_missing,
            self.s6_tests,
            self.s7_godfiles,
            wasm,
            self.s9_live_traceable,
            self.s10_smoke_receipts,
            self.s11_open_issues,
            self.s12_branches,
            self.s13_coverage,
            self.note,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{Baseline, Row, regressions};

    fn base() -> Baseline {
        Baseline {
            commit: "1ac3dbc".to_owned(),
            s2_bound: 249,
            s3_reportgap: 52,
            s4_inert: 2,
            s5a_impl: 82,
            s5b_partial: 25,
            s5c_missing: 11,
            s6_tests: 1885,
            s7_godfiles: 18,
            s8_wasm_raw: Some(21_271_259),
            s12_branches: Some(123),
        }
    }

    fn row() -> Row {
        Row {
            date: "2026-11-15".to_owned(),
            commit: "abc1234".to_owned(),
            rung: "C0.08".to_owned(),
            gate: "n/a".to_owned(),
            s2_bound: 249,
            s2_registry: 257,
            s3_reportgap: 52,
            s4_inert: 2,
            s5a_impl: 82,
            s5b_partial: 25,
            s5c_missing: 11,
            s6_tests: 1885,
            s7_godfiles: 12,
            s8_wasm_raw: Some(21_271_259),
            s9_live_traceable: "n/a".to_owned(),
            s10_smoke_receipts: "n/a".to_owned(),
            s11_open_issues: "n/a".to_owned(),
            s12_branches: "123".to_owned(),
            s13_coverage: "n/a".to_owned(),
            note: "test".to_owned(),
        }
    }

    #[test]
    fn tsv_has_the_scoreboard_column_count() {
        let line = row().to_tsv();
        assert_eq!(line.split('\t').count(), 20, "{line}");
    }

    #[test]
    fn missing_wasm_is_spelled_n_a() {
        let mut r = row();
        r.s8_wasm_raw = None;
        let line = r.to_tsv();
        let cells: Vec<&str> = line.split('\t').collect();
        assert_eq!(cells[13], "n/a");
        assert_eq!(cells[4], "249");
    }

    #[test]
    fn a_row_at_its_baseline_regresses_nothing() {
        let mut r = row();
        r.s2_bound = 249;
        r.s6_tests = 1885;
        r.s12_branches = "123".to_owned();
        assert!(regressions(&r, &base()).is_empty());
    }

    #[test]
    fn an_unmeasurable_branch_count_is_not_a_regression() {
        // The CI image is offline by contract, so S12 measures `n/a`.
        // A metric that could not be measured has not moved.
        let mut r = row();
        r.s12_branches = "n/a".to_owned();
        assert!(regressions(&r, &base()).is_empty());
    }

    #[test]
    fn a_rising_branch_count_is_flagged() {
        let mut r = row();
        r.s12_branches = "124".to_owned();
        assert!(
            regressions(&r, &base())
                .iter()
                .any(|m| m.contains("S12_branches rose 123 -> 124"))
        );
    }

    #[test]
    fn a_rising_test_count_is_not_a_regression() {
        let mut r = row();
        r.s6_tests = 1896;
        assert!(regressions(&r, &base()).is_empty());
    }

    #[test]
    fn each_direction_is_flagged() {
        let mut r = row();
        r.s2_bound = 248;
        r.s7_godfiles = 19;
        let found = regressions(&r, &base());
        assert!(found.iter().any(|m| m.contains("S2_bound fell 249 -> 248")));
        assert!(
            found
                .iter()
                .any(|m| m.contains("S7_godfiles rose 18 -> 19"))
        );
        assert_eq!(found.len(), 2, "{found:?}");
    }

    #[test]
    fn wasm_growth_is_flagged_only_when_both_sides_measured() {
        let mut r = row();
        r.s8_wasm_raw = Some(21_300_000);
        assert!(
            regressions(&r, &base())
                .iter()
                .any(|m| m.contains("S8_wasm_raw"))
        );
        r.s8_wasm_raw = None;
        assert!(regressions(&r, &base()).is_empty());
    }
}
