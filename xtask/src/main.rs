//! FireCraft verification gates (`cargo run -p xtask -- <gate>`).
//!
//! Gates: `ci` (everything), `layers` (workspace dependency tiers),
//! `wasm` (wasm32 check for pure crates), `corpus` (pinned corpus
//! digests), `parity` (rising-only parity floors), `scorecard`
//! (milestone status). Diagnostics go to stderr; stdout stays clean so
//! output can be piped.

mod corpus;
mod layers;
mod status;
mod util;
mod wasm;

use std::process::ExitCode;

const USAGE: &str = "usage: cargo run -p xtask -- <ci|layers|wasm|corpus|parity|scorecard>";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("ci") => run_ci(),
        Some("layers") => finish("layers", layers::run()),
        Some("wasm") => finish("wasm", wasm::run()),
        Some("corpus") => finish("corpus", corpus::run()),
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

fn run_ci() -> ExitCode {
    let steps: [Gate; 8] = [
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
        ("parity", status::parity),
        ("scorecard", status::scorecard),
    ];
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
