//! Wasm gate: the crates that power the editor core must always compile
//! for `wasm32-unknown-unknown` (the primary FireCraft release target is a
//! WASM build for the user's WebOS). Anything that breaks wasm breaks the
//! product.

use crate::util::cargo;

/// Crates that must stay wasm-clean. Grows as milestones land; a crate
/// added here must not use std-only APIs, threads, or C dependencies.
const WASM_CRATES: &[&str] = &["fwpng", "geom", "color", "raster"];

/// Runs `cargo check` for each pure crate against the wasm target.
pub fn run() -> Result<(), String> {
    for name in WASM_CRATES {
        cargo(&["check", "--target", "wasm32-unknown-unknown", "-p", name])
            .map_err(|e| format!("wasm gate failed for `{name}`:\n{e}"))?;
    }
    Ok(())
}
