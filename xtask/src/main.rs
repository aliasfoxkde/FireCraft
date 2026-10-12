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
//! (milestone status). Diagnostics go to stderr; stdout stays clean so
//! output can be piped.

mod corpus;
mod fuzz;
mod golden;
mod layers;
mod library;
mod scripts;
mod status;
#[allow(clippy::module_inception)]
mod svggolden;
mod util;
mod wasm;

use std::process::ExitCode;

const USAGE: &str = "usage: cargo run -p xtask -- <ci [--full]|layers|wasm|corpus [--report]|corpus-gen|library-gen|scripts|golden [--bless]|svg-golden [--bless]|svg-roundtrip [--measure]|fuzz|fuzz-quick|panic-hunt|parity|scorecard>";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("ci") => run_ci(std::env::args().nth(2).as_deref()),
        Some("layers") => finish("layers", layers::run()),
        Some("wasm") => finish("wasm", wasm::run()),
        Some("corpus") => match std::env::args().nth(2).as_deref() {
            Some("--report") => finish("corpus --report", corpus::report()),
            _ => finish("corpus", corpus::run()),
        },
        Some("corpus-gen") => finish("corpus-gen", corpus::gen_fixtures()),
        Some("library-gen") => finish("library-gen", library::run()),
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
        Some("fuzz") => finish("fuzz", fuzz::full()),
        Some("fuzz-quick") => finish("fuzz-quick", fuzz::quick()),
        Some("panic-hunt") => finish("panic-hunt", fuzz::panic_hunt()),
        Some("parity") => finish("parity", status::parity()),
        Some("scorecard") => finish("scorecard", status::scorecard()),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// One gate: a name plus the check it runs.
type Gate = (&'static str, fn() -> Result<(), String>);

fn run_ci(full: Option<&str>) -> ExitCode {
    let mut steps: Vec<Gate> = vec![
        ("fmt", || util::cargo(&["fmt", "--all", "--", "--check"])),
        ("clippy", || {
            util::cargo(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])
        }),
        ("test", || util::cargo(&["test", "--workspace"])),
        ("layers", layers::run),
        ("wasm", wasm::run),
        ("corpus", corpus::run),
        ("scripts", scripts::run),
        ("golden", golden::run),
        ("svg-golden", svggolden::run),
        ("svg-roundtrip", svggolden::roundtrip),
        ("fuzz", fuzz::quick),
        ("parity", status::parity),
        ("scorecard", status::scorecard),
    ];
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
