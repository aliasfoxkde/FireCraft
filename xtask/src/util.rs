//! Shared helpers for the xtask gates.

use sha2::{Digest, Sha256};
use std::io::Read as _;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Absolute path of the workspace root (parent of the xtask crate).
#[must_use]
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
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
    run_in_with_env(dir, program, args, &[])
}

/// Runs a program from `dir` with extra environment variables set.
///
/// The Aegis gate needs a private `HOME`: the scanner reads its
/// pattern-suppression state from `$HOME/.config/aegis/`, and the gate
/// installs the committed suppression there rather than in the
/// developer's own home.
pub fn run_in_with_env(
    dir: Option<&std::path::Path>,
    program: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<(), String> {
    let output = spawn_with_env(dir, program, args, envs)?;
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

/// Runs a program from `dir` with extra environment variables and
/// returns its raw output, whatever its exit status.
///
/// The Aegis gate needs this: the scanner exits non-zero when it finds
/// anything, which is not an error for a gate that filters the findings
/// itself, and it must run against the gate's own private `HOME`.
pub fn capture_with_env(
    dir: Option<&std::path::Path>,
    program: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<Output, String> {
    spawn_with_env(dir, program, args, envs)
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
