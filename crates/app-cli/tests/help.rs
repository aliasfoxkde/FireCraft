//! `firecraft-cli --help` — the usage surface of the packaged CLI.
//!
//! The release campaign requires the CLI to be testable on its own
//! ("test CLI separately"): the package verification runs the binary
//! and reads its output, so the usage must be printed on stdout with a
//! successful exit code. Before this, `--help` fell through to the
//! positional parser and the usage arrived as an error message on
//! stderr with a failing exit code.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

/// Runs the built `firecraft-cli` binary with the given arguments.
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_firecraft-cli"))
        .args(args)
        .output()
        .expect("the app-cli binary is built for this test")
}

#[test]
fn help_prints_the_usage_on_stdout_and_succeeds() {
    let output = cli(&["--help"]);
    assert!(
        output.status.success(),
        "`--help` must exit successfully, got {:?}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("usage: firecraft-cli"),
        "the usage must be on stdout, got {stdout:?}"
    );
    assert!(
        stdout.contains("export-web"),
        "the usage must list the verbs, got {stdout:?}"
    );
}

#[test]
fn the_short_help_flag_behaves_the_same() {
    let output = cli(&["-h"]);
    assert!(output.status.success(), "`-h` must exit successfully");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("usage: firecraft-cli"),
        "the usage must be on stdout, got {stdout:?}"
    );
}

#[test]
fn help_writes_nothing_to_stderr() {
    let output = cli(&["--help"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "`--help` must not write to stderr, got {stderr:?}"
    );
}

#[test]
fn an_unknown_verb_still_fails() {
    // The help path must not swallow the failure posture of a real
    // mistake: an unknown verb is still an error.
    let output = cli(&["definitely-not-a-verb"]);
    assert!(
        !output.status.success(),
        "an unknown verb must not exit successfully"
    );
}
