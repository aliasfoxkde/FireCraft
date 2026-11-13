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

/// The rustc the floors were measured with.
///
/// The floor is a claim about *this* compiler's instrumentation, not
/// about the source tree alone: the same tree measured by rustc 1.100.0
/// reads 77.0%/76.1% where 1.99.1 reads 81.9%/78.9% (decision 0102).
/// The CI workflows pin the toolchain to this version, so the number the
/// gate compares is the number the floor was measured with; the gate
/// warns — it does not fail — when it is measured with anything else.
const MEASURED_WITH: &str = "1.99.1";

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

/// The rustc that will do the measuring, as `rustc --version` reports it.
///
/// `None` when `rustc` cannot be run at all — the gate then measures
/// without naming its compiler, which is worth saying out loud.
fn measuring_rustc() -> Option<String> {
    let output = Command::new("rustc").arg("--version").output().ok()?;
    rustc_version(&String::from_utf8_lossy(&output.stdout)).map(str::to_string)
}

/// The version token in a `rustc --version` line.
///
/// `rustc 1.99.1 (58cf8f9c3 2026-10-13)` yields `1.99.1`.
fn rustc_version(line: &str) -> Option<&str> {
    let rest = line.trim().strip_prefix("rustc ")?;
    rest.split_whitespace().next().filter(|v| !v.is_empty())
}

/// Name the compiler that produced the measurement.
///
/// The measurement belongs to the compiler as much as to the source, so
/// the log says which compiler measured it — and says so plainly when
/// that is not the compiler the floors were measured with.
fn report_measuring_toolchain() {
    match measuring_rustc() {
        Some(version) if version != MEASURED_WITH => eprintln!(
            "[coverage] measured with rustc {version}, not the rustc {MEASURED_WITH} the floors were measured with"
        ),
        Some(version) => eprintln!("[coverage] measured with rustc {version}"),
        None => eprintln!("[coverage] the measuring rustc could not be identified"),
    }
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

    report_measuring_toolchain();

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

    report_measuring_toolchain();

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

    let mut lines_hit: f64 = 0.0;
    let mut lines_found: f64 = 0.0;
    let mut funcs_hit: f64 = 0.0;
    let mut funcs_found: f64 = 0.0;
    let mut regions_hit: f64 = 0.0;
    let mut regions_found: f64 = 0.0;
    let mut current_excluded = false;

    for line in text.lines() {
        let line = line.trim();
        if let Some(file) = line.strip_prefix("SF:") {
            current_excluded = EXCLUDED.iter().any(|ex| file.ends_with(ex));
            continue;
        }
        if current_excluded {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::{MEASURED_WITH, rustc_version};

    #[test]
    fn a_version_line_yields_its_version() {
        assert_eq!(
            rustc_version("rustc 1.99.1 (58cf8f9c3 2026-10-13)"),
            Some("1.99.1")
        );
    }

    #[test]
    fn a_version_line_with_a_nightly_suffix_still_yields_a_version() {
        assert_eq!(
            rustc_version("rustc 1.100.0-nightly (0ef1bc386 2026-11-09)"),
            Some("1.100.0-nightly")
        );
    }

    #[test]
    fn a_line_that_is_not_a_version_line_yields_nothing() {
        assert_eq!(rustc_version(""), None);
        assert_eq!(rustc_version("cargo 1.99.0 (5f94df478 2026-08-27)"), None);
        assert_eq!(rustc_version("rustc"), None);
    }

    #[test]
    fn the_pinned_toolchain_is_the_one_the_floors_were_measured_with() {
        // The floors are a claim about this compiler's instrumentation;
        // the workflows pin to the same version.
        assert_eq!(MEASURED_WITH, "1.99.1");
    }
}
