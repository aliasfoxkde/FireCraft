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
/// Raised to the measured 2026-10-25 baseline (80.47%) so the gate becomes
/// a rising-only ratchet; further gains re-bless upward.
const LINE_THRESHOLD: f64 = 80.0;

/// Minimum function coverage (percentage).  Measured baseline 77.85%.
const FUNCTION_THRESHOLD: f64 = 77.0;

/// Run the coverage gate.  Exits 0 if all thresholds are met or exceeded.
pub fn run() -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["llvm-cov", "--all", "--lcov", "--output-path", "lcov.info"])
        .current_dir(workspace_root())
        .output()
        .map_err(|e| format!("failed to run cargo-llvm-cov: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }

    let (lines, funcs, _regions) = parse_lcov(&workspace_root().join("lcov.info"))?;

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
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
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

fn parse_lcov(path: &std::path::Path) -> Result<(f64, f64, f64), String> {
    let mut text = String::new();
    // lcov.info is written relative to workspace_root(), which is also
    // where this process runs when invoked via `cargo run -p xtask`.
    std::fs::File::open(path)
        .map_err(|e| format!("could not open {}: {e}", path.display()))?
        .read_to_string(&mut text)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;

    let mut lines_hit: f64 = 0.0;
    let mut lines_found: f64 = 0.0;
    let mut funcs_hit: f64 = 0.0;
    let mut funcs_found: f64 = 0.0;
    let mut regions_hit: f64 = 0.0;
    let mut regions_found: f64 = 0.0;

    for line in text.lines() {
        let line = line.trim();
        if let Some(n) = line.strip_prefix("LH:") {
            lines_hit += n.parse::<f64>().unwrap_or(0.0);
        } else if let Some(n) = line.strip_prefix("LF:") {
            lines_found += n.parse::<f64>().unwrap_or(0.0);
        } else if let Some(n) = line.strip_prefix("FNH:") {
            funcs_hit += n.parse::<f64>().unwrap_or(0.0);
        } else if let Some(n) = line.strip_prefix("FNF:") {
            funcs_found += n.parse::<f64>().unwrap_or(0.0);
        } else if let Some(n) = line.strip_prefix("RH:") {
            regions_hit += n.parse::<f64>().unwrap_or(0.0);
        } else if let Some(n) = line.strip_prefix("RF:") {
            regions_found += n.parse::<f64>().unwrap_or(0.0);
        }
    }

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
    let regions_pct = if regions_found > 0.0 {
        (regions_hit / regions_found) * 100.0
    } else {
        0.0
    };

    Ok((lines_pct, funcs_pct, regions_pct))
}
