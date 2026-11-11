//! The CLI's `--version` door (T27-51): a packaged `firecraft-cli`
//! must be able to say which build it is. The answer is the crate's
//! own stamp — the one the workspace gate keeps equal to every other
//! crate's and to the release version the artifacts are named with.
//!
//! The flag is answered on stdout with a zero exit code, so a
//! packaging script can read it without treating the answer as a
//! failure.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_firecraft-cli");

#[test]
fn version_flag_prints_the_stamped_version() {
    let output = Command::new(BIN)
        .arg("--version")
        .output()
        .expect("run firecraft-cli --version");
    assert!(
        output.status.success(),
        "--version exited {:?}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout).expect("version output is utf-8");
    assert_eq!(stdout.trim(), format!("firecraft-cli {}", app_cli::VERSION));
}

#[test]
fn short_version_flag_agrees_with_the_long_one() {
    let long = Command::new(BIN)
        .arg("--version")
        .output()
        .expect("run firecraft-cli --version");
    let short = Command::new(BIN)
        .arg("-V")
        .output()
        .expect("run firecraft-cli -V");
    assert!(long.status.success() && short.status.success());
    assert_eq!(long.stdout, short.stdout);
}

#[test]
fn the_stamp_is_a_release_version() {
    // The stamp is a semver version, not a placeholder: the archive
    // receipt quotes it, so it has to be the crate's real one. A
    // rehearsal stamp may carry a pre-release suffix
    // (`0.2.5-dryrun.3`), so only the core is required to be
    // `MAJOR.MINOR.PATCH`.
    let core = app_cli::VERSION.split('-').next().unwrap_or_default();
    let core = core.split('+').next().unwrap_or_default();
    let parts: Vec<&str> = core.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "version stamp is not major.minor.patch: {}",
        app_cli::VERSION
    );
    assert!(
        parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())),
        "version stamp is not numeric: {}",
        app_cli::VERSION
    );
}
