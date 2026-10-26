//! Fuzz gates over the standalone `fuzz/` workspace.
//!
//! - `quick`: stable, deterministic drivers (anchors + `panic_hunt`).
//!   Part of `ci`.
//! - `full`: compiles the libfuzzer targets with cargo-fuzz under
//!   nightly. Part of `ci --full`; skipped with a note when cargo-fuzz
//!   is not installed.
//! - `panic_hunt`: the wide campaign — the full stable driver, then
//!   60 s of libfuzzer over the `engine_commands` target when
//!   cargo-fuzz is installed.

use crate::util::{self, workspace_root};

fn fuzz_dir() -> std::path::PathBuf {
    workspace_root().join("fuzz")
}

/// Quick gate: deterministic bounded fuzzing on the stable toolchain.
pub fn quick() -> Result<(), String> {
    let dir = fuzz_dir();
    if !dir.join("Cargo.toml").exists() {
        return Err("fuzz/ workspace is missing (expected fuzz/Cargo.toml)".into());
    }
    util::run_in(
        Some(&dir),
        "cargo",
        &["run", "--bin", "anchors", "--", "2048"],
    )?;
    util::run_in(Some(&dir), "cargo", &["run", "--bin", "panic_hunt"])?;
    // M9 task 8: the scripting host under mutation, and the two
    // machine surfaces under hostile traffic. Release: the boa
    // interpreter is an order of magnitude faster optimized, and the
    // sweep's cost is per-case engine builds.
    util::run_in(
        Some(&dir),
        "cargo",
        &["run", "--release", "--bin", "script_mutation"],
    )?;
    util::run_in(
        Some(&dir),
        "cargo",
        &["run", "--release", "--bin", "control_channel"],
    )
}

/// Full gate: compile-check every libfuzzer target.
pub fn full() -> Result<(), String> {
    let dir = fuzz_dir();
    if !dir.join("Cargo.toml").exists() {
        return Err("fuzz/ workspace is missing (expected fuzz/Cargo.toml)".into());
    }
    if util::run_in(Some(&dir), "cargo", &["fuzz", "--version"]).is_err() {
        eprintln!(
            "[xtask] fuzz-full: cargo-fuzz not installed; skipping \
             libfuzzer compile check (install: cargo install cargo-fuzz)"
        );
        return Ok(());
    }
    // cargo-fuzz resolves the scaffold at `<cwd>/fuzz/`, so it runs from
    // the project root, not from inside `fuzz/`. `--features fuzzer` is
    // required because the libfuzzer targets are gated behind it (keeps
    // the stable driver buildable without nightly).
    util::run_in(
        None,
        "cargo",
        &["+nightly", "fuzz", "build", "--features", "fuzzer"],
    )
}

/// The `panic_hunt` campaign: the whole stable driver across a wide seed
/// range, then 60 s of libfuzzer over `engine_commands` when cargo-fuzz
/// is available. `xtask panic-hunt`.
pub fn panic_hunt() -> Result<(), String> {
    let dir = fuzz_dir();
    if !dir.join("Cargo.toml").exists() {
        return Err("fuzz/ workspace is missing (expected fuzz/Cargo.toml)".into());
    }
    // 4096 seeds x 48 steps: a few seconds on the stable toolchain.
    util::run_in(
        Some(&dir),
        "cargo",
        &["run", "--release", "--bin", "panic_hunt", "--", "4096"],
    )?;
    if util::run_in(Some(&dir), "cargo", &["fuzz", "--version"]).is_err() {
        eprintln!(
            "[xtask] panic-hunt: cargo-fuzz not installed; stable sweep \
             only (install: cargo install cargo-fuzz)"
        );
        return Ok(());
    }
    util::run_in(
        None,
        "cargo",
        &[
            "+nightly",
            "fuzz",
            "run",
            "engine_commands",
            "--release",
            // The libfuzzer targets are feature-gated (keeps the stable
            // driver buildable without nightly) — same as `fuzz::full`.
            "--features",
            "fuzzer",
            "--",
            "-max_total_time=60",
        ],
    )
}
