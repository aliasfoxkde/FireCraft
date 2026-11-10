//! The memory probe re-execs this binary to measure peak RSS in a fresh
//! process. The CI gate's nested `cargo` steps re-uplift
//! `target/debug/xtask` while the gate is still running, so the path the
//! gate was launched from stops resolving and the probe died with
//! `spawn perf-child: No such file or directory (os error 2)`.
//!
//! This test reproduces that shape deterministically: it runs the gate
//! binary from a link of its own, the probe removes that link (the
//! running inode survives), and the probe must still complete the
//! measurement. `/proc/self/exe` names the inode, so it survives the
//! removal; `current_exe()` does not.

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The gate binary under test, as cargo built it.
fn gate_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_xtask"))
}

/// The repo root (the package manifest is `<root>/xtask`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the xtask package lives in the repo root")
        .to_path_buf()
}

/// A link of the gate binary under a name of its own.
///
/// The probe removes the path it runs from, so the probe must run from
/// a link the test owns rather than from the build output itself. A hard
/// link shares the inode and costs no space; it must live beside the
/// original so the link crosses no filesystem boundary.
fn owned_link(bin: &Path, label: &str) -> PathBuf {
    let dir = bin.parent().expect("the gate binary lives in a directory");
    let link = dir.join(format!("xtask-selfexec-{label}-{}", std::process::id()));
    let _ = std::fs::remove_file(&link);
    std::fs::hard_link(bin, &link).expect("link the gate binary");
    link
}

/// The probe removes its own path, so the test's fixture is only read.
fn fixture() -> PathBuf {
    repo_root().join("corpus/fixtures/composed.fw.png")
}

/// The regression: with the gate's binary replaced underneath it, the
/// memory probe still measures.
#[test]
fn the_memory_probe_survives_its_own_binary_being_replaced() {
    let bin = gate_binary();
    let link = owned_link(&bin, "replaced");
    let output = Command::new(&link)
        .arg("perf-selfexec")
        .arg(fixture())
        .env("FIRECRAFT_PERF_SELFEXEC", "1")
        .output()
        .expect("run the self-exec probe");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "perf-selfexec failed:\n{stderr}");
    assert!(
        stderr.contains("perf-selfexec: peak"),
        "the probe reported no measurement:\n{stderr}"
    );
    // The probe removed the link it ran from; nothing is left behind.
    assert!(
        !link.exists(),
        "the probe left its own link in place: {}",
        link.display()
    );
}

/// The counterexample: the probe refuses to delete what it runs from
/// without the opt-in, so the regression above is the replacement doing
/// the work rather than a probe that always fails.
#[test]
fn the_probe_refuses_to_delete_the_binary_it_runs_from_without_opt_in() {
    let bin = gate_binary();
    let link = owned_link(&bin, "guarded");
    let output = Command::new(&link)
        .arg("perf-selfexec")
        .arg(fixture())
        .output()
        .expect("run the self-exec probe");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "the probe ran without the opt-in:\n{stderr}"
    );
    assert!(
        stderr.contains("FIRECRAFT_PERF_SELFEXEC"),
        "the refusal did not name the opt-in:\n{stderr}"
    );
    // The refusal happens before the deletion, so the link survives.
    assert!(link.exists(), "the probe deleted its link without opt-in");
    let _ = std::fs::remove_file(&link);
}
