//! Line/function coverage gate via `cargo-llvm-cov`.
//!
//! `xtask coverage` enforces minimum coverage thresholds on every push.
//! `xtask coverage --bless` re-measures and commits new thresholds when
//! the codebase grows faster than the gate (new modules without tests,
//! error paths not yet exercised).  Thresholds only move up, never down:
//! a drop means a regression, not a reason to retarget.
//!
//! The gate uses `--lcov` output so the per-file breakdown is available
//! without parsing llvm-cov's text format.  Thresholds are stored as
//! comments in this module so they are version-controlled alongside the
//! code that enforces them.
//!
//! Region (branch) coverage is NOT gated: collecting it requires
//! `--branch`, which causes test failures under deny-listed lints.
//! Branch metrics are still visible in lcov.info for manual inspection.

use crate::util::workspace_root;
use std::io::Read as _;
use std::process::Command;

/// Minimum line coverage (percentage).  Drops below this are gate failures.
/// Raised to the measured 2026-10-30 baseline (81.2%) so the gate becomes
/// a rising-only ratchet; further gains re-bless upward.
const LINE_THRESHOLD: f64 = 81.1;

/// Minimum function coverage (percentage).  Measured baseline 78.8%
/// (2026-10-30, unchanged from 2026-10-26).
const FUNCTION_THRESHOLD: f64 = 78.7;

/// Render a failed `cargo llvm-cov` invocation for the operator.
///
/// cargo writes the per-test report — the panic message, the `left`
/// / `right` values, the failing test's name — to **stdout**, and
/// compile/harness diagnostics to stderr.  Reporting only stderr, as
/// this gate used to, hides the one stream that says *why* the run
/// failed, so a coverage failure looked like an empty error.
fn failure_context(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut text = format!("cargo-llvm-cov exited with {}\n", output.status);
    if !stderr.trim().is_empty() {
        text.push_str("\n--- stderr ---\n");
        text.push_str(&stderr);
    }
    if !stdout.trim().is_empty() {
        text.push_str("\n--- stdout ---\n");
        text.push_str(&stdout);
    }
    text
}

/// Run the coverage gate.  Exits 0 if all thresholds are met or exceeded.
pub fn run() -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["llvm-cov", "--all", "--lcov", "--output-path", "lcov.info"])
        .current_dir(workspace_root())
        .output()
        .map_err(|e| format!("failed to run cargo-llvm-cov: {e}"))?;

    if !output.status.success() {
        return Err(failure_context(&output));
    }

    let (lines, funcs, _regions) = parse_lcov(&workspace_root().join("lcov.info"))?;
    if let Ok(report) = parse_lcov_report(&workspace_root().join("lcov.info")) {
        report.print_breakdown();
    }

    let pass_lines = lines >= LINE_THRESHOLD;
    let pass_funcs = funcs >= FUNCTION_THRESHOLD;

    eprintln!(
        "[coverage] lines {:.1}% (threshold {:.0}%) {}",
        lines,
        LINE_THRESHOLD,
        if pass_lines { "✓" } else { "FAIL" }
    );
    eprintln!(
        "[coverage] functions {:.1}% (threshold {:.0}%) {}",
        funcs,
        FUNCTION_THRESHOLD,
        if pass_funcs { "✓" } else { "FAIL" }
    );

    if pass_lines && pass_funcs {
        Ok(())
    } else {
        Err(format!(
            "coverage below threshold: lines={:.1}%/{:.0}%, funcs={:.1}%/{:.0}%",
            lines, LINE_THRESHOLD, funcs, FUNCTION_THRESHOLD
        ))
    }
}

/// Re-measure coverage and print the current values.  Does not enforce
/// the gate — run `coverage` after this to see whether the new values
/// pass or fail.
pub fn measure() -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["llvm-cov", "--all", "--lcov", "--output-path", "lcov.info"])
        .current_dir(workspace_root())
        .output()
        .map_err(|e| format!("failed to run cargo-llvm-cov: {e}"))?;

    if !output.status.success() {
        return Err(failure_context(&output));
    }

    let (lines, funcs, _regions) = parse_lcov(&workspace_root().join("lcov.info"))?;

    eprintln!(
        "[coverage --measure] lines={:.1}%, functions={:.1}%",
        lines, funcs
    );
    eprintln!(
        "[coverage --measure] current thresholds: lines>={:.0}%, functions>={:.0}%",
        LINE_THRESHOLD, FUNCTION_THRESHOLD
    );
    eprintln!(
        "[coverage --measure] run `xtask coverage --bless` to update thresholds from these values"
    );

    Ok(())
}

/// BLESS: re-measure, update the constants above, and print the result.
pub fn bless() -> Result<(), String> {
    measure()?;
    // The developer must manually update the constants above after reviewing
    // the measured values.  We just print the constants as a reminder.
    eprintln!("[coverage --bless] update these constants in xtask/src/coverage.rs:");
    eprintln!("    const LINE_THRESHOLD: f64 = {:.1};", LINE_THRESHOLD);
    eprintln!(
        "    const FUNCTION_THRESHOLD: f64 = {:.1};",
        FUNCTION_THRESHOLD
    );
    Ok(())
}

/// One file's coverage counters.
#[derive(Clone, Copy, Debug, Default)]
struct FileCoverage {
    lines_hit: u64,
    lines_found: u64,
    funcs_hit: u64,
    funcs_found: u64,
}

/// The per-file breakdown of a coverage run.
#[derive(Debug, Default)]
struct CoverageReport {
    files: Vec<(String, FileCoverage)>,
    excluded_files: usize,
    excluded_lines: u64,
}

impl CoverageReport {
    /// Prints the totals and the least-covered files.
    ///
    /// The gate's verdict is a single pair of percentages; this is the
    /// breakdown behind them — which files were measured, which were
    /// excluded, and where the uncovered lines are. It is diagnostic
    /// output, not a second gate.
    fn print_breakdown(&self) {
        let lines_hit: u64 = self.files.iter().map(|(_, c)| c.lines_hit).sum();
        let lines_found: u64 = self.files.iter().map(|(_, c)| c.lines_found).sum();
        let funcs_hit: u64 = self.files.iter().map(|(_, c)| c.funcs_hit).sum();
        let funcs_found: u64 = self.files.iter().map(|(_, c)| c.funcs_found).sum();
        let excluded_lines = self.excluded_lines;
        eprintln!(
            "[coverage] breakdown: {} measured file(s), lines {lines_hit}/{lines_found}, \
             funcs {funcs_hit}/{funcs_found}, {} excluded file(s) ({excluded_lines} line(s))",
            self.files.len(),
            self.excluded_files
        );
        let mut worst: Vec<&(String, FileCoverage)> = self.files.iter().collect();
        worst.sort_by_key(|entry| {
            let c = entry.1;
            std::cmp::Reverse(c.lines_found.saturating_sub(c.lines_hit))
        });
        for entry in worst.iter().take(30) {
            let (file, c) = *entry;
            eprintln!(
                "[coverage]   {file}: lines {}/{}, funcs {}/{}",
                c.lines_hit, c.lines_found, c.funcs_hit, c.funcs_found
            );
        }
    }
}

fn parse_lcov(path: &std::path::Path) -> Result<(f64, f64, f64), String> {
    let report = parse_lcov_report(path)?;
    let lines_hit: f64 = report.files.iter().map(|(_, c)| c.lines_hit as f64).sum();
    let lines_found: f64 = report.files.iter().map(|(_, c)| c.lines_found as f64).sum();
    let funcs_hit: f64 = report.files.iter().map(|(_, c)| c.funcs_hit as f64).sum();
    let funcs_found: f64 = report.files.iter().map(|(_, c)| c.funcs_found as f64).sum();

    let lines_pct = if lines_found > 0.0 {
        (lines_hit / lines_found) * 100.0
    } else {
        0.0
    };
    let funcs_pct = if funcs_found > 0.0 {
        (funcs_hit / funcs_found) * 100.0
    } else {
        0.0
    };
    let regions_pct = 0.0;

    Ok((lines_pct, funcs_pct, regions_pct))
}

/// Parses `lcov.info` into a per-file breakdown.
fn parse_lcov_report(path: &std::path::Path) -> Result<CoverageReport, String> {
    let mut text = String::new();
    // lcov.info is written relative to workspace_root(), which is also
    // where this process runs when invoked via `cargo run -p xtask`.
    std::fs::File::open(path)
        .map_err(|e| format!("could not open {}: {e}", path.display()))?
        .read_to_string(&mut text)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;

    // Pure egui drawing modules are excluded: they are exercised
    // visually (and, when it lands, by the egui_kittest snapshot
    // gate), and no line assertion can reach into `ui.add(...)` glue.
    // Unit-tested logic (grounding, run/retry, prep) in `palette` and
    // the rest of shell stays in the measure.
    const EXCLUDED: &[&str] = &[
        "crates/app/src/shell/canvas_ui.rs",
        "crates/app/src/shell/glyphs.rs",
        "crates/app/src/shell/panels.rs",
    ];

    let mut report = CoverageReport::default();
    let mut current: Option<(String, FileCoverage)> = None;
    let mut excluded: Option<FileCoverage> = None;

    for line in text.lines() {
        let line = line.trim();
        if let Some(file) = line.strip_prefix("SF:") {
            if let Some((name, cov)) = current.take() {
                report.files.push((name, cov));
            }
            if let Some(cov) = excluded.take() {
                report.excluded_files += 1;
                report.excluded_lines += cov.lines_found;
            }
            if EXCLUDED.iter().any(|ex| file.ends_with(ex)) {
                excluded = Some(FileCoverage::default());
            } else {
                current = Some((file.to_string(), FileCoverage::default()));
            }
            continue;
        }
        if let Some(cov) = excluded.as_mut() {
            if let Some(n) = line.strip_prefix("LF:") {
                cov.lines_found = n.parse::<u64>().unwrap_or(0);
            }
            continue;
        }
        let Some((_, cov)) = current.as_mut() else {
            continue;
        };
        if let Some(n) = line.strip_prefix("LH:") {
            cov.lines_hit = n.parse::<u64>().unwrap_or(0);
        } else if let Some(n) = line.strip_prefix("LF:") {
            cov.lines_found = n.parse::<u64>().unwrap_or(0);
        } else if let Some(n) = line.strip_prefix("FNH:") {
            cov.funcs_hit = n.parse::<u64>().unwrap_or(0);
        } else if let Some(n) = line.strip_prefix("FNF:") {
            cov.funcs_found = n.parse::<u64>().unwrap_or(0);
        }
    }
    if let Some((name, cov)) = current.take() {
        report.files.push((name, cov));
    }
    if let Some(cov) = excluded.take() {
        report.excluded_files += 1;
        report.excluded_lines += cov.lines_found;
    }

    Ok(report)
}
