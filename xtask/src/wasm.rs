//! Wasm gate: the crates that power the editor core must always compile
//! for `wasm32-unknown-unknown` (the primary `FireCraft` release target is a
//! WASM build for the user's `WebOS`). Anything that breaks wasm breaks the
//! product.
//!
//! When `wasm-bindgen-test-runner` is on PATH, the gate additionally
//! *executes* the fwpng smoke suite (`crates/fwpng/tests/wasm_smoke.rs`)
//! under node, so wasm evidence is runtime, not just compile-only.

use crate::util::cargo;

/// Crates that must stay wasm-clean. Grows as milestones land; a crate
/// added here must not use std-only APIs, threads, or C dependencies.
/// `engine` and the shells joined in M5 — the editor core and both
/// surfaces must compile for the browser even though their *native*
/// platform services (`StdFs`) would never run there. `vector` joined
/// in M6: L2 pure math over `doc`+`geom`, no IO, no threads. `filters`
/// joined in M7 at birth: L2 pure kernels over `color`, integer math,
/// no IO. `text` joined in M7 task 7: cosmic-text shaping over
/// `include_bytes!` fonts — no filesystem, no system font scan, no
/// threads.
const WASM_CRATES: &[&str] = &[
    "fwpng",
    "geom",
    "color",
    "raster",
    "doc",
    "vector",
    "compose",
    "engine",
    "app-cli",
    "app",
    "filters",
    "text",
    "io",
    "scripting",
];

/// Integration-test target exercising fwpng under wasm.
const SMOKE_TARGET: &str = "wasm_smoke";

/// Runs `cargo check` for each pure crate against the wasm target, then
/// the runtime smoke suite when a runner is available, then the trunk
/// bundle build when trunk is available.
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
    bundle_check()?;
    Ok(())
}

/// Builds the web bundle (`trunk build`) so the browser release target
/// is exercised, not just the wasm compile. Skipped (with a note) when
/// trunk is not installed.
fn bundle_check() -> Result<(), String> {
    if trunk_on_path().is_none() {
        eprintln!("[xtask] wasm: trunk not on PATH; web bundle skipped for this run");
        return Ok(());
    }
    let status = std::process::Command::new("trunk")
        .args(["build"])
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../web"))
        .status()
        .map_err(|e| format!("trunk build failed to start:\n{e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("trunk build failed (web bundle)".to_owned())
    }
}

/// Finds `trunk` on PATH, if present.
fn trunk_on_path() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("trunk"))
        .find(|candidate| candidate.is_file())
}

/// Finds `wasm-bindgen-test-runner` on PATH, if present.
fn runner_on_path() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("wasm-bindgen-test-runner"))
        .find(|candidate| candidate.is_file())
}
