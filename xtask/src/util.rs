//! Shared helpers for the xtask gates.

use sha2::{Digest, Sha256};
use std::io::Read as _;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Absolute path of the workspace root.
///
/// Resolved at *runtime* from the current directory, not at compile
/// time. The compile-time path (`env!("CARGO_MANIFEST_DIR")`) names
/// the tree the binary was *built* in, which is not necessarily the
/// tree it is *run* in: two worktrees of this repository share one
/// `CARGO_TARGET_DIR`, and when their `xtask` sources are
/// byte-identical cargo does not rebuild, so the binary in the shared
/// target directory carries the first worktree's path and every gate
/// silently acts on that other tree. A gate that blesses another
/// worktree's baseline while reporting success is worse than no gate.
///
/// The runtime resolution walks up from the current directory to the
/// nearest `Cargo.toml` that declares a workspace, so running the
/// binary from a subdirectory still finds the right root. The
/// compile-time path remains as a fallback for the case where the
/// current directory is not inside a workspace at all.
#[must_use]
pub fn workspace_root() -> PathBuf {
    runtime_workspace_root()
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
}

/// The nearest ancestor of the current directory whose `Cargo.toml`
/// declares a workspace that contains this crate.
fn runtime_workspace_root() -> Option<PathBuf> {
    workspace_root_from(&std::env::current_dir().ok()?)
}

/// The nearest ancestor of `start` whose `Cargo.toml` declares a
/// workspace that contains this crate.
///
/// The member test matters: the repository has a nested `fuzz/`
/// workspace, so "declares a workspace" alone would resolve to the
/// fuzz tree when the gate is run from inside it.
fn workspace_root_from(start: &std::path::Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            if let Ok(text) = std::fs::read_to_string(&manifest) {
                if text.contains("[workspace]") && dir.join("xtask").join("Cargo.toml").is_file() {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Lowercase hex of a digest (the manifest's spelling).
#[must_use]
pub fn hex(digest: &[u8]) -> String {
    let mut s = String::with_capacity(digest.len() * 2);
    for byte in digest {
        s.push_str(&format!("{byte:02x}"));
    }
    s
}

/// The SHA-256 of `data`, lowercase hex.
#[must_use]
pub fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

/// Inflates a zlib payload (the mkTS/mkBS/mkBT inner streams).
pub fn inflate(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(data)
        .read_to_end(&mut out)
        .map_err(|e| format!("zlib inflate failed: {e}"))?;
    Ok(out)
}

/// Runs `cargo` with arguments from the workspace root.
pub fn cargo(args: &[&str]) -> Result<(), String> {
    run("cargo", args)
}

/// Runs a program from the workspace root, failing with its output tail.
pub fn run(program: &str, args: &[&str]) -> Result<(), String> {
    run_in(None, program, args)
}

/// Runs a program from `dir` (or the workspace root when `None`).
pub fn run_in(dir: Option<&std::path::Path>, program: &str, args: &[&str]) -> Result<(), String> {
    let output = spawn(dir, program, args)?;
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

/// Runs a program from `dir` and returns its stdout and stderr without
/// interpreting the exit status.
///
/// The Aegis scanner exits non-zero whenever it reports findings, so
/// the gate parses the document it prints rather than reading the
/// status; a scanner that dies without printing leaves stdout empty,
/// which the caller reports with the stderr diagnostics.
pub fn capture_in(
    dir: Option<&std::path::Path>,
    program: &str,
    args: &[&str],
) -> Result<(String, String), String> {
    capture_in_with_env(dir, program, args, &[])
}

/// Runs a program from `dir` with extra environment variables and
/// returns its stdout and stderr, whatever its exit status.
///
/// The Aegis gate needs this to run the scanner against a private
/// `HOME`, so the developer's own Aegis configuration can never change
/// what the scan reports.
pub fn capture_in_with_env(
    dir: Option<&std::path::Path>,
    program: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<(String, String), String> {
    let output = spawn_with_env(dir, program, args, envs)?;
    Ok((
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

/// Runs `cargo metadata --no-deps` and parses the JSON document.
pub fn cargo_metadata() -> Result<serde_json::Value, String> {
    let output = spawn(
        None,
        "cargo",
        &["metadata", "--format-version=1", "--no-deps"],
    )?;
    if !output.status.success() {
        return Err(format!("cargo metadata failed:\n{}", output_tail(&output)));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("cargo metadata JSON: {e}"))
}

fn spawn(dir: Option<&std::path::Path>, program: &str, args: &[&str]) -> Result<Output, String> {
    spawn_with_env(dir, program, args, &[])
}

fn spawn_with_env(
    dir: Option<&std::path::Path>,
    program: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<Output, String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    for (key, value) in envs {
        cmd.env(key, value);
    }
    match dir {
        Some(d) => cmd.current_dir(d),
        None => cmd.current_dir(workspace_root()),
    };
    cmd.output()
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn the_workspace_root_is_resolved_at_runtime() {
        // The root must name the tree the binary is *run* in, not the
        // tree it was *built* in: two worktrees share one
        // CARGO_TARGET_DIR, and a byte-identical `xtask` is not
        // rebuilt, so the binary in the shared target directory carries
        // the other worktree's compile-time path. A gate that then
        // blesses that other tree's baseline reports success while
        // doing nothing to this one.
        let root = workspace_root();
        assert!(root.join("Cargo.toml").is_file(), "root has no manifest");
        assert!(root.join("xtask").join("Cargo.toml").is_file());
        assert!(root.join("crates").is_dir());
    }

    #[test]
    fn the_runtime_root_ignores_a_nested_workspace() {
        // `fuzz/` is its own workspace; resolving it would point the
        // gate at a tree that has no `crates/`.
        let nested = workspace_root().join("fuzz");
        if !nested.join("Cargo.toml").is_file() {
            return; // no nested workspace in this tree
        }
        assert!(!workspace_root_from(&nested).is_some_and(|dir| dir.ends_with("fuzz")));
    }

    #[test]
    fn the_runtime_root_finds_the_root_from_a_subdirectory() {
        let root = workspace_root();
        let sub = root.join("crates");
        if !sub.is_dir() {
            return;
        }
        assert_eq!(workspace_root_from(&sub), Some(root));
    }
}
