//! FireCraft verification gates (`cargo run -p xtask -- <gate>`).
//!
//! Gates: `ci [--full]` (everything; `--full` adds the libfuzzer
//! compile check), `layers` (workspace dependency tiers), `wasm`
//! (wasm32 check for pure crates), `corpus` (pinned corpus digests),
//! `golden` (render pins + semantic anchors + CS6 report), `fuzz`
//! (libfuzzer compile check alone), `fuzz-quick` (the stable driver
//! pair from `ci`; the CI pipeline's fuzz job), `panic-hunt` (catalog
//! fuzz
//! campaign), `parity` (rising-only parity floors), `scorecard`
//! (milestone status), `changelog` (release chore: generated
//! CHANGELOG + license/version stamps). Diagnostics go to stderr;
//! stdout stays clean so output can be piped.

mod changelog;
mod corpus;
mod coverage;
mod dist;
mod floor;
mod fuzz;
mod golden;
mod layers;
mod library;
mod perf;
mod scripts;
mod status;
mod styles;
#[allow(clippy::module_inception)]
mod svggolden;
mod util;
mod wasm;
mod webgolden;

use std::process::ExitCode;

const USAGE: &str = "usage: cargo run -p xtask -- <ci [--full]|ci-no-fmt|aegis|coverage [--bless|--measure]|layers|wasm|corpus [--report]|corpus-gen|library-gen|styles-gen|scripts|floor|golden [--bless]|svg-golden [--bless]|svg-roundtrip [--measure]|web-golden [--bless|--measure]|perf [--bless|--measure]|dist --web|--native|fuzz|fuzz-quick|panic-hunt|parity|scorecard>";

/// Run Aegis scan with the committed production-profile baseline.
/// Runs from the workspace root so the baseline path (.aegis-baseline.json)
/// resolves correctly.  Baseline was generated with:
///   aegis --format json -c production scan . --output-file .aegis-baseline.json -q
fn aegis() -> Result<(), String> {
    util::run_in(
        Some(util::workspace_root().as_path()),
        &aegis_binary()?,
        &[
            "--format",
            "json",
            "-c",
            "production",
            "scan",
            ".",
            "--baseline",
            ".aegis-baseline.json",
            "-q",
        ],
    )
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

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("ci") => run_ci(std::env::args().nth(2).as_deref(), false),
        Some("ci-no-fmt") => run_ci(std::env::args().nth(2).as_deref(), true),
        Some("aegis") => finish("aegis", aegis()),
        Some("coverage") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("coverage --bless", coverage::bless()),
            Some("--measure") => finish("coverage --measure", coverage::measure()),
            _ => finish("coverage", coverage::run()),
        },
        Some("layers") => finish("layers", layers::run()),
        Some("wasm") => finish("wasm", wasm::run()),
        Some("corpus") => match std::env::args().nth(2).as_deref() {
            Some("--report") => finish("corpus --report", corpus::report()),
            _ => finish("corpus", corpus::run()),
        },
        Some("corpus-gen") => finish("corpus-gen", corpus::gen_fixtures()),
        Some("floor") => finish("floor", floor::run()),
        Some("library-gen") => finish("library-gen", library::run()),
        Some("styles-gen") => finish("styles-gen", styles::run()),
        Some("scripts") => finish("scripts", scripts::run()),
        Some("golden") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("golden --bless", golden::bless()),
            _ => finish("golden", golden::run()),
        },
        Some("svg-golden") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("svg-golden --bless", svggolden::bless()),
            Some("--measure") => finish("svg-golden --measure", svggolden::measure()),
            _ => finish("svg-golden", svggolden::run()),
        },
        Some("svg-roundtrip") => match std::env::args().nth(2).as_deref() {
            Some("--measure") => finish("svg-roundtrip --measure", svggolden::roundtrip_measure()),
            _ => finish("svg-roundtrip", svggolden::roundtrip()),
        },
        Some("web-golden") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("web-golden --bless", webgolden::bless()),
            Some("--measure") => finish("web-golden --measure", webgolden::measure()),
            _ => finish("web-golden", webgolden::run()),
        },
        Some("perf") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("perf --bless", perf::bless()),
            Some("--measure") => finish("perf --measure", perf::measure()),
            _ => finish("perf", perf::run()),
        },
        // Hidden probe verb: `perf` re-execs this binary in a fresh
        // process so the VmHWM it reports is the open path's peak, not
        // the gate's lifetime high-water mark.
        Some("perf-child") => match std::env::args().nth(2) {
            Some(path) => finish("perf-child", perf::child(&path)),
            None => {
                eprintln!("{USAGE}");
                ExitCode::FAILURE
            }
        },
        Some("dist") => match std::env::args().nth(2).as_deref() {
            Some("--web") => finish("dist --web", dist::web()),
            Some("--native") => finish("dist --native", dist::native()),
            _ => {
                eprintln!("{USAGE}");
                ExitCode::FAILURE
            }
        },
        Some("fuzz") => finish("fuzz", fuzz::full()),
        Some("fuzz-quick") => finish("fuzz-quick", fuzz::quick()),
        Some("panic-hunt") => finish("panic-hunt", fuzz::panic_hunt()),
        Some("parity") => finish("parity", status::parity()),
        Some("scorecard") => finish("scorecard", status::scorecard()),
        Some("changelog") => finish("changelog", changelog::run()),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// One gate: a name plus the check it runs.
type Gate = (&'static str, fn() -> Result<(), String>);

fn run_ci(full: Option<&str>, skip_fmt: bool) -> ExitCode {
    // Build the step list. fmt is conditionally included so that `ci-no-fmt`
    // (used by the CI fmt-gate job) stays independent from the gate job.
    let mut steps: Vec<Gate> = vec![
        ("aegis", || {
            // Fast (~10 s): catches PII / credential regressions before fmt.
            // Baseline committed as `.aegis-baseline.json` (aegis 0.6.3, production profile).
            // Re-generate only when the aegis version changes.
            util::run_in(
                Some(util::workspace_root().as_path()),
                &aegis_binary()?,
                &[
                    "--format",
                    "json",
                    "-c",
                    "production",
                    "scan",
                    ".",
                    "--baseline",
                    ".aegis-baseline.json",
                    "-q",
                ],
            )
        }),
        ("coverage", coverage::run),
        ("fmt", || {
            util::cargo(&["fmt", "--all", "--", "--check"]).and_then(|()| {
                // `web` is workspace-excluded (trunk) — `--all` never
                // sees it, so it is checked here explicitly.
                util::cargo(&["fmt", "--manifest-path", "web/Cargo.toml", "--", "--check"])
            })
        }),
        ("clippy", || {
            util::cargo(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])
            .and_then(|()| {
                // `web` is workspace-excluded (trunk) — clippy needs the
                // explicit manifest, same as the fmt check above.
                util::run_in(
                    Some(std::path::Path::new("web")),
                    "cargo",
                    &["clippy", "--all-targets", "--", "-D", "warnings"],
                )
            })
        }),
        ("test", || util::cargo(&["test", "--workspace"])),
        ("layers", layers::run),
        ("wasm", wasm::run),
        ("corpus", corpus::run),
        ("floor", floor::run),
        ("scripts", scripts::run),
        ("golden", golden::run),
        ("svg-golden", svggolden::run),
        ("svg-roundtrip", svggolden::roundtrip),
        ("web-golden", webgolden::run),
        ("perf", perf::run),
        ("fuzz", fuzz::quick),
        ("parity", status::parity),
        ("scorecard", status::scorecard),
        ("changelog", changelog::run),
    ];
    if skip_fmt {
        steps.remove(2); // remove "fmt" at index 2
    }
    if full == Some("--full") || full == Some("full") {
        steps.insert(steps.len() - 2, ("fuzz-full", fuzz::full));
    }
    for (name, step) in steps {
        eprintln!("[xtask] {name} ...");
        if let Err(err) = step() {
            eprintln!("[xtask] {name} FAILED");
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
        eprintln!("[xtask] {name} ok");
    }
    eprintln!("[xtask] ci: all gates green");
    ExitCode::SUCCESS
}

fn finish(name: &str, result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => {
            eprintln!("[xtask] {name} ok");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("[xtask] {name} FAILED");
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
