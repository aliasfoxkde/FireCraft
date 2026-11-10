//! Shared helpers for the xtask gates.

use sha2::{Digest, Sha256};
use std::io::Read as _;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Absolute path of the workspace root (parent of the xtask crate).
///
/// Resolved from the current working directory at call time, not from a
/// compile-time constant. The compile-time path is a property of the
/// *binary*, but every gate command acts on a *tree*: a stale binary
/// left in a shared target directory would otherwise scan, bless, or
/// package a different worktree than the one it was invoked in — and
/// would report success for it. The compile-time path is only the
/// fallback for an invocation from outside any workspace.
#[must_use]
pub fn workspace_root() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(root) = find_workspace_root(&cwd) {
            return root;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The nearest ancestor of `start` that is this workspace's root: a
/// directory holding both the workspace manifest and the `xtask` crate.
/// The second condition keeps a nested crate with its own `[workspace]`
/// table (the `web/` crate has one) from being mistaken for the root.
fn find_workspace_root(start: &std::path::Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(candidate) = dir {
        let manifest = candidate.join("Cargo.toml");
        if manifest.is_file() && candidate.join("xtask").join("Cargo.toml").is_file() {
            return Some(candidate.to_path_buf());
        }
        dir = candidate.parent();
    }
    None
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
    let output = spawn(dir, program, args)?;
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
    let mut cmd = Command::new(program);
    cmd.args(args);
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
    use super::{find_workspace_root, workspace_root};
    use std::path::{Path, PathBuf};

    /// A scratch directory that cleans itself up.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("firecraft-xtask-root-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            assert!(
                std::fs::create_dir_all(&path).is_ok(),
                "could not create the scratch dir: {path:?}"
            );
            Scratch(path)
        }

        fn dir(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            assert!(
                std::fs::create_dir_all(&path).is_ok(),
                "could not create the scratch subdir: {path:?}"
            );
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn touch(path: &Path) {
        assert!(
            std::fs::write(path, "[workspace]\n").is_ok(),
            "could not write the manifest: {path:?}"
        );
    }

    #[test]
    fn the_live_workspace_root_is_the_repository() {
        let root = workspace_root();
        assert!(
            root.join("Cargo.toml").is_file(),
            "root has no workspace manifest: {root:?}"
        );
        assert!(
            root.join("xtask").join("Cargo.toml").is_file(),
            "root has no xtask crate: {root:?}"
        );
    }

    #[test]
    fn a_nested_directory_resolves_up_to_the_root() {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let parent = crate_dir.parent().map(std::path::Path::to_path_buf);
        assert_eq!(
            find_workspace_root(&crate_dir).as_deref(),
            parent.as_deref(),
            "the xtask crate must resolve to the repository root"
        );
    }

    #[test]
    fn a_directory_outside_any_workspace_resolves_to_nothing() {
        let scratch = Scratch::new("outside");
        let found = find_workspace_root(&scratch.0);
        assert_eq!(found, None, "a stray directory must not claim to be a root");
    }

    #[test]
    fn a_manifest_alone_is_not_enough_to_claim_a_root() {
        let scratch = Scratch::new("manifest-only");
        touch(&scratch.0.join("Cargo.toml"));
        let found = find_workspace_root(&scratch.0);
        assert_eq!(
            found, None,
            "a manifest without the xtask crate is not this workspace"
        );
    }

    #[test]
    fn the_nearest_qualifying_ancestor_wins() {
        let scratch = Scratch::new("nested");
        let root = scratch.dir("root");
        touch(&root.join("Cargo.toml"));
        assert!(
            std::fs::create_dir_all(root.join("xtask")).is_ok(),
            "could not create the xtask dir"
        );
        touch(&root.join("xtask").join("Cargo.toml"));
        let inner = scratch.dir("root/inner/deeper");
        assert_eq!(
            find_workspace_root(&inner).as_deref(),
            Some(root.as_path()),
            "resolution must stop at the nearest workspace root"
        );
    }
}
