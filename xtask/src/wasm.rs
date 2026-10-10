//! Wasm gate: the crates that power the editor core must always compile
//! for `wasm32-unknown-unknown` (the primary FireCraft release target is a
//! WASM build for the user's WebOS). Anything that breaks wasm breaks the
//! product.
//!
//! When `wasm-bindgen-test-runner` is on PATH, the gate additionally
//! *executes* the fwpng smoke suite (`crates/fwpng/tests/wasm_smoke.rs`)
//! under node, so wasm evidence is runtime, not just compile-only.

use crate::util::cargo;

/// Crates that must stay wasm-clean. Grows as milestones land; a crate
/// added here must not use std-only APIs, threads, or C dependencies.
const WASM_CRATES: &[&str] = &["fwpng", "geom", "color", "raster"];

/// Integration-test target exercising fwpng under wasm.
const SMOKE_TARGET: &str = "wasm_smoke";

/// Runs `cargo check` for each pure crate against the wasm target, then
/// the runtime smoke suite when a runner is available.
pub fn run() -> Result<(), String> {
    for name in WASM_CRATES {
        cargo(&["check", "--target", "wasm32-unknown-unknown", "-p", name])
            .map_err(|e| format!("wasm gate failed for `{name}`:\n{e}"))?;
    }
    if runner_on_path().is_some() {
        cargo(&[
            "test",
            "-p",
            "fwpng",
            "--target",
            "wasm32-unknown-unknown",
            "--test",
            SMOKE_TARGET,
            "--config",
            "target.wasm32-unknown-unknown.runner=\"wasm-bindgen-test-runner\"",
        ])
        .map_err(|e| format!("wasm smoke failed:\n{e}"))?;
    } else {
        eprintln!(
            "[xtask] wasm: wasm-bindgen-test-runner not on PATH; compile-only check for this run"
        );
    }
    Ok(())
}

/// Finds `wasm-bindgen-test-runner` on PATH, if present.
fn runner_on_path() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("wasm-bindgen-test-runner"))
        .find(|candidate| candidate.is_file())
}
