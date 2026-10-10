//! Fuzz gates over the standalone `fuzz/` workspace.
//!
//! - `quick`: stable, deterministic anchor driver (fixed seeds, bounded
//!   iterations). Part of `ci`.
//! - `full`: compiles the libfuzzer targets with cargo-fuzz under
//!   nightly. Part of `ci --full`; skipped with a note when cargo-fuzz
//!   is not installed.

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
