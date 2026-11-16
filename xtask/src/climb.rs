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
    s12_branches: usize,
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
pub fn check() -> Result<(), String> {
    let row = measure("check")?;
    let base = baseline()?;
    let mut regressions = Vec::new();

    // (name, now, was): a value that must never fall.
    for (name, now, was) in [
        ("S2_bound", row.s2_bound, base.s2_bound),
        ("S5a_impl", row.s5a_impl, base.s5a_impl),
        ("S6_tests", row.s6_tests, base.s6_tests),
    ] {
        if now < was {
            regressions.push(format!("{name} fell {was} -> {now}"));
        }
    }

    // A value that must never rise.
    for (name, now, was) in [
        ("S3_reportgap", row.s3_reportgap, base.s3_reportgap),
        ("S4_inert", row.s4_inert, base.s4_inert),
        ("S5b_partial", row.s5b_partial, base.s5b_partial),
        ("S5c_missing", row.s5c_missing, base.s5c_missing),
        ("S7_godfiles", row.s7_godfiles, base.s7_godfiles),
        ("S12_branches", row.s12_branches, base.s12_branches),
    ] {
        if now > was {
            regressions.push(format!("{name} rose {was} -> {now}"));
        }
    }

    if let (Some(now), Some(was)) = (row.s8_wasm_raw, base.s8_wasm_raw)
        && now > was
    {
        regressions.push(format!("S8_wasm_raw rose {was} -> {now}"));
    }

    util::write_stdout_line(&row.to_tsv())?;
    if regressions.is_empty() {
        eprintln!("[xtask] climb --check: every ratcheted metric holds its baseline");
        return Ok(());
    }
    Err(format!(
        "climb --check: {} metric(s) moved backwards against {}:\n  {}",
        regressions.len(),
        base.commit,
        regressions.join("\n  ")
    ))
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
        s12_branches: remote_branches(&root)?,
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
    s12_branches: usize,
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
        s12_branches: number(f.get(17)),
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

fn remote_branches(root: &Path) -> Result<usize, String> {
    let (stdout, _) = util::capture_in(
        Some(root),
        "git",
        &["for-each-ref", "refs/remotes", "--format=%(refname:short)"],
    )?;
    Ok(stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !line.ends_with("/HEAD") && !line.ends_with("/main"))
        .count())
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

    use super::Row;

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
            s12_branches: 123,
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
}
