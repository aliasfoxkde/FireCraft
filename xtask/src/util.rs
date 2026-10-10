//! Shared helpers for the xtask gates.

use std::path::PathBuf;
use std::process::{Command, Output};

/// Absolute path of the workspace root (parent of the xtask crate).
#[must_use]
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Runs `cargo` with arguments from the workspace root.
pub fn cargo(args: &[&str]) -> Result<(), String> {
    run("cargo", args)
}

/// Runs a program from the workspace root, failing with its output tail.
pub fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let output = spawn(program, args)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{program} {}` failed:\n{}",
            args.join(" "),
            output_tail(&output)
        ))
    }
}

/// Runs `cargo metadata --no-deps` and parses the JSON document.
pub fn cargo_metadata() -> Result<serde_json::Value, String> {
    let output = spawn("cargo", &["metadata", "--format-version=1", "--no-deps"])?;
    if !output.status.success() {
        return Err(format!("cargo metadata failed:\n{}", output_tail(&output)));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("cargo metadata JSON: {e}"))
}

fn spawn(program: &str, args: &[&str]) -> Result<Output, String> {
    Command::new(program)
        .args(args)
        .current_dir(workspace_root())
        .output()
        .map_err(|e| format!("failed to spawn {program}: {e}"))
}

/// Last 40 lines of combined output — enough to diagnose, small enough to
/// read.
fn output_tail(output: &Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stderr).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(40);
    lines[start..].join("\n")
}
