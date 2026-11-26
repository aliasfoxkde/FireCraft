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
//!
//! The gate prints its own inputs beside its verdict: the compiler that
//! measured, the profile files the export merged, and, per file, the
//! export's counters next to the file's `DA` record count and its
//! on-disk line count.
//!
//! The export reports two different line counts for the same file, and
//! the difference is not a defect: `LF` is the lines the export
//! *found*, `DA` is the lines it wrote a counter for, and the second is
//! normally a little smaller than the first.  So `LF > DA` means
//! nothing on its own — it is true of most files in an ordinary
//! export.  What is a defect is a file whose reported line count
//! outruns the file itself: a file cannot have more lines than it
//! has.  That is a gate failure, and the gate names the file.
//!
//! A file that is compiled more than once — the app library and its
//! unit-test binary both compile `crates/app/src/shell/mod.rs` —
//! reports roughly *twice* its own lines, so its coverage reads at
//! about half and the drop looks like a regression.  That is the
//! ratio the gate reports, not the inequality (decision 0104).

use crate::util::workspace_root;
use std::io::Read as _;
use std::process::Command;

/// Minimum line coverage (percentage).  Drops below this are gate failures.
/// Raised to the measured 2026-10-30 baseline (81.2%) so the gate becomes
/// a rising-only ratchet; further gains re-bless upward.
const LINE_THRESHOLD: f64 = 80.2;

/// Minimum function coverage (percentage).  Measured baseline 78.8%
/// (2026-10-30, unchanged from 2026-10-26).
const FUNCTION_THRESHOLD: f64 = 76.7;

/// The rustc the floors were measured with.
///
/// The floor is a claim about *this* compiler's instrumentation, not
/// about the source tree alone: the same tree measured by rustc 1.100.0
/// reads 77.0%/76.1% where 1.99.1 reads 81.9%/78.9% (decision 0102).
/// The CI workflows pin the toolchain to this version, so the number the
/// gate compares is the number the floor was measured with; the gate
/// warns — it does not fail — when it is measured with anything else.
const MEASURED_WITH: &str = "1.99.1";

/// The ratio of reported lines to `DA` records above which the export
/// is counted once per build.
///
/// An ordinary export reports a few percent more lines than it wrote
/// counters for: `LF` counts the lines found, `DA` counts the lines
/// carrying a counter.  Across the 171 files of the pinned run the
/// largest ordinary ratio was 1.33 (a six-record file), and the
/// measured tree sat at 1.03.  A file compiled in two builds reports
/// roughly twice its own lines: the 1.100.0 export of
/// `crates/app/src/shell/mod.rs` read 8138 lines for 5297 records,
/// 1.54, and claimed more lines than the file had.  A doubled file
/// lands near 2.0, so the cut sits well below it and well above the
/// ordinary spread (decision 0104).
const DOUBLE_COUNT_RATIO: f64 = 1.5;

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

/// One file's counters, as the coverage export reports them.
///
/// The export's own numbers are the file's; `da_records` and `on_disk`
/// are the file's own, and the difference between them is what makes a
/// merged measurement visible.
#[derive(Debug, Clone, PartialEq)]
pub struct FileMetrics {
    /// The path the export named (`SF:`).
    pub name: String,
    /// Instrumented lines the export listed (`DA:` records).
    pub da_records: f64,
    /// Lines the export found (`LF:`).
    pub lines_found: f64,
    /// Lines the export covered (`LH:`).
    pub lines_hit: f64,
    /// Functions the export found (`FNF:`).
    pub funcs_found: f64,
    /// Functions the export covered (`FNH:`).
    pub funcs_hit: f64,
    /// The file's own line count on disk, when it can be read.
    pub on_disk: Option<usize>,
}

impl FileMetrics {
    /// Whether the export counted this file more than once.
    ///
    /// The export lists each source line once but reports the lines it
    /// found for every build of the file, so a file compiled twice
    /// reports roughly twice its own lines — and reads at about half
    /// its coverage.  The ordinary export reports a little *more*
    /// lines than it wrote counters for, so the ratio, not the
    /// inequality, is the signal.
    fn counted_more_than_once(&self) -> bool {
        self.da_records > 0.0 && self.lines_found >= self.da_records * DOUBLE_COUNT_RATIO
    }

    /// The ratio of reported lines to `DA` records.
    fn line_ratio(&self) -> f64 {
        if self.da_records > 0.0 {
            self.lines_found / self.da_records
        } else {
            0.0
        }
    }

    /// Whether the export claims more lines than the file has.  A file
    /// cannot have more lines than it has, so this is a broken
    /// measurement whatever produced it.
    fn impossible(&self) -> bool {
        match self.on_disk {
            Some(lines) => self.da_records > lines as f64 || self.lines_found > lines as f64,
            None => false,
        }
    }
}

/// The file's own line count on disk.
///
/// `None` when the file cannot be read — a path outside the workspace,
/// or a file the measurement saw before the tree changed under it.
fn on_disk_lines(name: &str) -> Option<usize> {
    std::fs::read_to_string(workspace_root().join(name))
        .ok()
        .map(|text| text.lines().count())
}

/// The number of `.profraw` files the measurement merged.
///
/// The profiles are the measurement's other input: the export's numbers
/// are the merge of these files, so the count belongs in the log beside
/// the numbers.  `cargo-llvm-cov` writes them under the target
/// directory's `llvm-cov` profile directory.
fn count_profraws() -> usize {
    profraws_in(&target_dir(), 3)
}

/// The directory cargo writes its build artifacts into.
///
/// `CARGO_TARGET_DIR` wins when it is set: the profiles the measurement
/// merged were written wherever the build wrote them.
fn target_dir() -> std::path::PathBuf {
    match std::env::var("CARGO_TARGET_DIR") {
        Ok(dir) if !dir.trim().is_empty() => std::path::PathBuf::from(dir),
        _ => workspace_root().join("target"),
    }
}

/// The `.profraw` files under one directory, to a bounded depth.
fn profraws_in(dir: &std::path::Path, depth: usize) -> usize {
    if depth == 0 {
        return 0;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut count = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            count += profraws_in(&path, depth - 1);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("profraw") {
            count += 1;
        }
    }
    count
}

/// Print the per-file breakdown beside the verdict.
///
/// Each row carries the export's counters next to the file's `DA`
/// record count and its on-disk line count, so a reader can see at a
/// glance whether the reported line count outruns the file it claims
/// to measure.  A file the export counted once per build is marked
/// with the ratio that says so.
fn report_breakdown(measurement: &Measurement) {
    let files = &measurement.files;
    eprintln!("[coverage] breakdown: {} measured file(s)", files.len());
    for file in files {
        let on_disk = match file.on_disk {
            Some(lines) => lines.to_string(),
            None => "unreadable".to_owned(),
        };
        eprintln!(
            "  {}: lines {}/{}, funcs {}/{}, DA {} record(s), {} line(s) on disk{}",
            file.name,
            file.lines_hit,
            file.lines_found,
            file.funcs_hit,
            file.funcs_found,
            file.da_records,
            on_disk,
            if file.counted_more_than_once() {
                format!(
                    "  <- counted once per build (LF/DA {:.2})",
                    file.line_ratio()
                )
            } else {
                String::new()
            },
        );
    }
}

/// The files the export counted once per build.
fn counted_twice(files: &[FileMetrics]) -> Vec<&FileMetrics> {
    files
        .iter()
        .filter(|file| file.counted_more_than_once())
        .collect()
}

/// What one measurement says.
///
/// The export's totals, every file's own counters, and what the gate
/// excluded — the parts of the run that a reader needs to judge the
/// verdict.
#[derive(Debug)]
pub struct Measurement {
    /// Line coverage, as a percentage.
    pub lines_pct: f64,
    /// Function coverage, as a percentage.
    pub funcs_pct: f64,
    /// Region (branch) coverage, as a percentage.  Not gated.
    pub regions_pct: f64,
    /// Every measured file, with its own counters.
    pub files: Vec<FileMetrics>,
    /// How many files the gate excluded.
    pub excluded_files: usize,
    /// How many lines the excluded files hold.
    pub excluded_lines: f64,
}

/// The files whose export claims more lines than they have.
fn impossible(files: &[FileMetrics]) -> Vec<&FileMetrics> {
    files.iter().filter(|file| file.impossible()).collect()
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

    let measurement = parse_lcov(&workspace_root().join("lcov.info"))?;
    let lines = measurement.lines_pct;
    let funcs = measurement.funcs_pct;

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
    eprintln!(
        "[coverage] regions {:.1}% (not gated)",
        measurement.regions_pct
    );

    report_inventory(&measurement);
    report_breakdown(&measurement);

    if let Some(file) = impossible(&measurement.files).first() {
        return Err(format!(
            "the coverage export claims {} line(s) found and {} instrumented line(s) for {}, which has {} line(s) on disk",
            file.lines_found,
            file.da_records,
            file.name,
            file.on_disk.unwrap_or(0),
        ));
    }

    if pass_lines && pass_funcs {
        Ok(())
    } else {
        Err(format!(
            "coverage below threshold: lines={:.1}%/{:.0}%, funcs={:.1}%/{:.0}%{}",
            lines,
            LINE_THRESHOLD,
            funcs,
            FUNCTION_THRESHOLD,
            counted_twice_reason(&measurement.files),
        ))
    }
}

/// Name the files whose measurement counts them more than once.
///
/// A red gate whose denominator counts a file once per build is not a
/// coverage regression; it is the export's merge showing through, and
/// the message says which files are in it rather than leaving the
/// reader to guess from a percentage.
fn counted_twice_reason(files: &[FileMetrics]) -> String {
    let twice = counted_twice(files);
    if twice.is_empty() {
        return String::new();
    }
    format!(
        "; {} file(s) counted once per build inflate the denominator: {}",
        twice.len(),
        twice
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Print the measurement's other inputs: the profiles it merged and the
/// files it counted once per build.
fn report_inventory(measurement: &Measurement) {
    eprintln!(
        "[coverage] inventory: {} profile file(s) merged, {} file(s) measured, {} excluded file(s) ({} line(s))",
        count_profraws(),
        measurement.files.len(),
        measurement.excluded_files,
        measurement.excluded_lines
    );
    let twice = counted_twice(&measurement.files);
    if !twice.is_empty() {
        eprintln!(
            "[coverage] {} file(s) counted once per build: {}",
            twice.len(),
            twice
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
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

    let measurement = parse_lcov(&workspace_root().join("lcov.info"))?;
    let lines = measurement.lines_pct;
    let funcs = measurement.funcs_pct;

    report_measuring_toolchain();
    report_inventory(&measurement);

    eprintln!(
        "[coverage --measure] lines={:.1}%, functions={:.1}%",
        lines, funcs
    );
    eprintln!(
        "[coverage --measure] current thresholds: lines>={:.0}%, functions>={:.0}%",
        LINE_THRESHOLD, FUNCTION_THRESHOLD
    );
    report_breakdown(&measurement);
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

/// Parse the export: the totals, and every file's own counters.
fn parse_lcov(path: &std::path::Path) -> Result<Measurement, String> {
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
    let mut excluded_files: usize = 0;
    let mut excluded_lines: f64 = 0.0;
    let mut files: Vec<FileMetrics> = Vec::new();
    let mut current: Option<FileMetrics> = None;

    for line in text.lines() {
        let line = line.trim();
        if let Some(file) = line.strip_prefix("SF:") {
            if let Some(finished) = current.take() {
                files.push(finished);
            }
            current_excluded = EXCLUDED.iter().any(|ex| file.ends_with(ex));
            if current_excluded {
                excluded_files += 1;
            } else {
                current = Some(FileMetrics {
                    name: file.to_owned(),
                    da_records: 0.0,
                    lines_found: 0.0,
                    lines_hit: 0.0,
                    funcs_found: 0.0,
                    funcs_hit: 0.0,
                    on_disk: on_disk_lines(file),
                });
            }
            continue;
        }
        if current_excluded {
            // The excluded file's lines are counted so the log can say
            // how much the exclusions took out of the measure.
            if let Some(n) = line.strip_prefix("LF:") {
                excluded_lines += n.parse::<f64>().unwrap_or(0.0);
            }
            continue;
        }
        if line.starts_with("DA:") {
            // The record's payload is `<line>,<count>`; what matters here
            // is how many records the file has.
            if let Some(file) = current.as_mut() {
                file.da_records += 1.0;
            }
        } else if let Some(n) = line.strip_prefix("LH:") {
            let value = n.parse::<f64>().unwrap_or(0.0);
            lines_hit += value;
            if let Some(file) = current.as_mut() {
                file.lines_hit += value;
            }
        } else if let Some(n) = line.strip_prefix("LF:") {
            let value = n.parse::<f64>().unwrap_or(0.0);
            lines_found += value;
            if let Some(file) = current.as_mut() {
                file.lines_found += value;
            }
        } else if let Some(n) = line.strip_prefix("FNH:") {
            let value = n.parse::<f64>().unwrap_or(0.0);
            funcs_hit += value;
            if let Some(file) = current.as_mut() {
                file.funcs_hit += value;
            }
        } else if let Some(n) = line.strip_prefix("FNF:") {
            let value = n.parse::<f64>().unwrap_or(0.0);
            funcs_found += value;
            if let Some(file) = current.as_mut() {
                file.funcs_found += value;
            }
        } else if let Some(n) = line.strip_prefix("RH:") {
            regions_hit += n.parse::<f64>().unwrap_or(0.0);
        } else if let Some(n) = line.strip_prefix("RF:") {
            regions_found += n.parse::<f64>().unwrap_or(0.0);
        }
    }
    if let Some(finished) = current.take() {
        files.push(finished);
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

    Ok(Measurement {
        lines_pct,
        funcs_pct,
        regions_pct,
        files,
        excluded_files,
        excluded_lines,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{FileMetrics, MEASURED_WITH, counted_twice, impossible, parse_lcov, rustc_version};

    fn metrics(name: &str, da: f64, lf: f64, on_disk: Option<usize>) -> FileMetrics {
        FileMetrics {
            name: name.to_owned(),
            da_records: da,
            lines_found: lf,
            lines_hit: 0.0,
            funcs_found: 0.0,
            funcs_hit: 0.0,
            on_disk,
        }
    }

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

    #[test]
    fn a_file_counted_once_per_build_is_named() {
        // The export sums the line counts of a file's builds but lists
        // each line once: LF outruns DA by about half, and the file is
        // counted twice.  This is the 1.100.0 export of shell/mod.rs.
        let files = vec![metrics("shell/mod.rs", 5297.0, 8138.0, Some(6309))];
        let twice = counted_twice(&files);
        assert_eq!(twice.len(), 1);
        assert_eq!(twice[0].name, "shell/mod.rs");
    }

    #[test]
    fn an_ordinary_export_gap_is_not_a_double_count() {
        // The ordinary export reports more lines found than it wrote
        // counters for.  LF > DA is true of most files and says
        // nothing; only the ratio does.  These are the pinned run's
        // numbers for shell/mod.rs.
        let files = vec![metrics("shell/mod.rs", 5167.0, 5304.0, Some(7993))];
        assert!(counted_twice(&files).is_empty());
    }

    #[test]
    fn a_file_counted_once_is_not_named() {
        let files = vec![metrics("shell/mod.rs", 5297.0, 5297.0, Some(6309))];
        assert!(counted_twice(&files).is_empty());
    }

    #[test]
    fn more_instrumented_lines_than_the_file_has_is_impossible() {
        let files = vec![metrics("short.rs", 100.0, 100.0, Some(10))];
        assert_eq!(impossible(&files).len(), 1);
    }

    #[test]
    fn more_lines_found_than_the_file_has_is_impossible() {
        // The export's line count is the claim that broke: 8138 lines
        // found for a file with 6309 lines on disk.  The `DA` count was
        // inside the file, so only checking it would have missed this.
        let files = vec![metrics("shell/mod.rs", 5297.0, 8138.0, Some(6309))];
        assert_eq!(impossible(&files).len(), 1);
    }

    #[test]
    fn an_unreadable_file_is_not_impossible() {
        let files = vec![metrics("gone.rs", 100.0, 100.0, None)];
        assert!(impossible(&files).is_empty());
    }

    #[test]
    fn the_export_keeps_each_file_da_records_beside_its_line_count() {
        // The whole point of the breakdown: LF and DA are different
        // numbers for the same file, and both are reported.
        let dir =
            std::env::temp_dir().join(format!("firecraft-lcov-{}-{}", std::process::id(), line!()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("lcov.info");
        std::fs::write(
            &path,
            "SF:crates/app/src/shell/mod.rs\n\
             DA:302,1\nDA:303,0\nDA:304,1\n\
             LH:2\nLF:9\nFNH:1\nFNF:3\nend_of_record\n",
        )
        .unwrap();

        let measurement = parse_lcov(&path).unwrap();
        assert_eq!(measurement.files.len(), 1);
        assert_eq!(measurement.files[0].da_records, 3.0);
        assert_eq!(measurement.files[0].lines_found, 9.0);
        assert_eq!(measurement.files[0].lines_hit, 2.0);
        assert_eq!(measurement.files[0].funcs_found, 3.0);
        assert!(
            (measurement.lines_pct - 22.222222).abs() < 0.001,
            "{measurement:?}"
        );
        assert!(
            (measurement.funcs_pct - 33.333333).abs() < 0.001,
            "{measurement:?}"
        );

        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn an_excluded_file_leaves_the_measure_but_is_still_reported() {
        // The exclusions take lines out of the score; the log says how
        // many, so the score can be read against what was left out.
        let dir = std::env::temp_dir().join(format!(
            "firecraft-lcov-x-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("lcov.info");
        std::fs::write(
            &path,
            "SF:crates/app/src/shell/panels.rs\n\
             DA:1,0\nLH:0\nLF:3402\nFNH:0\nFNF:9\nend_of_record\n\
             SF:crates/app/src/shell/mod.rs\n\
             DA:302,1\nLH:1\nLF:9\nFNH:1\nFNF:3\nend_of_record\n",
        )
        .unwrap();

        let measurement = parse_lcov(&path).unwrap();
        assert_eq!(measurement.excluded_files, 1);
        assert_eq!(measurement.excluded_lines, 3402.0);
        assert_eq!(measurement.files.len(), 1);
        assert_eq!(measurement.files[0].name, "crates/app/src/shell/mod.rs");
        // The excluded file's lines are not in the denominator.
        assert!(
            (measurement.lines_pct - 11.111111).abs() < 0.001,
            "{measurement:?}"
        );

        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }
}
