//! Integration tests for the release-asset staging logic that the GitHub
//! Actions release workflow depends on.
//!
//! `.github/workflows/release.yml` used to assemble the `gh release create`
//! asset arguments inline. That made the construction untestable and wrong:
//! `gh` takes release assets as *positional* arguments after the tag and
//! has no `-a`/`--asset` flag, so the old `-a "$f"` loop aborted the command
//! with "unknown shorthand flag: 'a' in -a" and no draft release could be
//! created.
//!
//! The construction now lives in `packaging/release-assets.sh` so it can be
//! exercised directly instead of asserted on as a string inside YAML. These
//! tests drive that script against fixture trees and capture the exact
//! argument vector the `gh` CLI would receive, using a stub `gh` on `PATH`.

#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the xtask package directory has a parent")
        .to_path_buf()
}

fn script_path() -> PathBuf {
    repo_root().join("packaging").join("release-assets.sh")
}

/// A throwaway directory tree, removed when the test ends.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(label: &str) -> Sandbox {
        let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "firecraft-release-assets-{label}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create the sandbox root");
        Sandbox { root }
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create a sandbox parent directory");
        }
        fs::write(&path, contents).expect("write a sandbox file");
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// The names in a sandbox directory, sorted by byte value.
    fn dir_names(&self, relative: &str) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.path(relative))
            .expect("read a sandbox directory")
            .map(|entry| {
                entry
                    .expect("read a sandbox directory entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run_script(sandbox: &Sandbox, args: &[&str]) -> Output {
    Command::new("bash")
        .arg(script_path())
        .args(args)
        .current_dir(&sandbox.root)
        .output()
        .expect("run packaging/release-assets.sh")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Install a stub `gh` on `PATH` that records its argument vector, and
/// return the stub's directory plus the file it records into.
fn install_stub_gh(sandbox: &Sandbox) -> (PathBuf, PathBuf) {
    let bin = sandbox.path("stub-bin");
    fs::create_dir_all(&bin).expect("create the stub bin directory");
    let stub = bin.join("gh");
    fs::write(
        &stub,
        "#!/bin/sh\n\
         : > \"$GH_ARGV_OUT\"\n\
         for arg in \"$@\"; do printf '%s\\0' \"$arg\" >> \"$GH_ARGV_OUT\"; done\n",
    )
    .expect("write the stub gh");
    let mut permissions = fs::metadata(&stub).expect("stat the stub gh").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&stub, permissions).expect("make the stub gh executable");
    (bin, sandbox.path("gh-argv.bin"))
}

fn run_create(sandbox: &Sandbox, args: &[&str], bin: &Path, argv_out: &Path) -> Output {
    let path = match std::env::var("PATH") {
        Ok(existing) => format!("{}:{existing}", bin.display()),
        Err(_) => bin.display().to_string(),
    };
    Command::new("bash")
        .arg(script_path())
        .arg("create")
        .args(args)
        .current_dir(&sandbox.root)
        .env("PATH", path)
        .env("GH_ARGV_OUT", argv_out)
        .output()
        .expect("run packaging/release-assets.sh create")
}

/// The argument vector the stub `gh` recorded, one entry per element.
fn recorded_argv(argv_out: &Path) -> Vec<String> {
    let bytes = fs::read(argv_out).expect("read the recorded gh argument vector");
    bytes
        .split(|byte| *byte == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect()
}

/// The NUL-delimited asset list the staging step wrote.
fn staged_assets(sandbox: &Sandbox, list: &str) -> Vec<String> {
    let bytes = fs::read(sandbox.path(list)).expect("read the staged asset list");
    bytes
        .split(|byte| *byte == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect()
}

/// The eight artifacts the build job produces for tag `v0.2.5`, laid out
/// the way `upload-artifact` stores them: the archive root is the least
/// common ancestor of the uploaded paths, so the artifact arrives as a
/// tree rather than a flat file list.
fn write_release_tree(sandbox: &Sandbox) {
    sandbox.write("src/native/firecraft-0.2.5-linux-x86_64.tar.gz", "native");
    sandbox.write("src/native/firecraft-cli-0.2.5-linux-x86_64.tar.gz", "cli");
    sandbox.write("src/native/firecraft_0.2.5_amd64.deb", "deb");
    sandbox.write("src/native/MANIFEST.sha256", "manifest");
    sandbox.write("src/firecraft-web-0.2.5.zip", "web");
    sandbox.write("src/source/FireCraft-0.2.5-source.tar.gz", "source");
    sandbox.write("src/sbom/firecraft-0.2.5-sbom.cdx.json", "sbom");
    sandbox.write("src/RELEASE_SHA256.txt", "checksums");
}

#[test]
fn stage_flattens_the_artifact_tree_into_the_release_asset_set() {
    let sandbox = Sandbox::new("normal");
    write_release_tree(&sandbox);

    let output = run_script(&sandbox, &["stage", "src", "flat", "assets.zlist"]);
    assert!(
        output.status.success(),
        "staging failed: {}",
        stderr_of(&output)
    );

    // Every artifact is staged under its own basename, so the flat
    // directory is what both `sha256sum -c` and the release step read.
    assert_eq!(
        sandbox.dir_names("flat"),
        vec![
            "FireCraft-0.2.5-source.tar.gz".to_string(),
            "MANIFEST.sha256".to_string(),
            "RELEASE_SHA256.txt".to_string(),
            "firecraft-0.2.5-linux-x86_64.tar.gz".to_string(),
            "firecraft-0.2.5-sbom.cdx.json".to_string(),
            "firecraft-cli-0.2.5-linux-x86_64.tar.gz".to_string(),
            "firecraft-web-0.2.5.zip".to_string(),
            "firecraft_0.2.5_amd64.deb".to_string(),
        ]
    );

    // The checksum list is a release asset in its own right — it is the
    // canonical digest list, not a manifest of the asset set.
    assert!(sandbox.path("flat/RELEASE_SHA256.txt").is_file());

    // The staged list is deterministic: same paths, same order, every run.
    assert_eq!(
        staged_assets(&sandbox, "assets.zlist"),
        vec![
            "flat/RELEASE_SHA256.txt".to_string(),
            "flat/firecraft-web-0.2.5.zip".to_string(),
            "flat/MANIFEST.sha256".to_string(),
            "flat/firecraft-0.2.5-linux-x86_64.tar.gz".to_string(),
            "flat/firecraft-cli-0.2.5-linux-x86_64.tar.gz".to_string(),
            "flat/firecraft_0.2.5_amd64.deb".to_string(),
            "flat/firecraft-0.2.5-sbom.cdx.json".to_string(),
            "flat/FireCraft-0.2.5-source.tar.gz".to_string(),
        ]
    );
}

#[test]
fn create_passes_the_assets_positionally_to_gh_release_create() {
    let sandbox = Sandbox::new("argv");
    write_release_tree(&sandbox);
    sandbox.write("notes.md", "release notes");
    let output = run_script(&sandbox, &["stage", "src", "flat", "assets.zlist"]);
    assert!(
        output.status.success(),
        "staging failed: {}",
        stderr_of(&output)
    );

    let (bin, argv_out) = install_stub_gh(&sandbox);
    let output = run_create(
        &sandbox,
        &["v0.2.5", "assets.zlist", "notes.md"],
        &bin,
        &argv_out,
    );
    assert!(
        output.status.success(),
        "creating the release failed: {}",
        stderr_of(&output)
    );

    let argv = recorded_argv(&argv_out);
    let expected = vec![
        "release".to_string(),
        "create".to_string(),
        "v0.2.5".to_string(),
        "--title".to_string(),
        // The release title is the bare version, without the product
        // name: the repository name and the asset filenames already
        // carry it. This expectation was written before the naming
        // cleanup and kept the old "FireCraft <version>" title.
        "0.2.5".to_string(),
        "--draft".to_string(),
        "--notes-file".to_string(),
        "notes.md".to_string(),
        "flat/RELEASE_SHA256.txt".to_string(),
        "flat/firecraft-web-0.2.5.zip".to_string(),
        "flat/MANIFEST.sha256".to_string(),
        "flat/firecraft-0.2.5-linux-x86_64.tar.gz".to_string(),
        "flat/firecraft-cli-0.2.5-linux-x86_64.tar.gz".to_string(),
        "flat/firecraft_0.2.5_amd64.deb".to_string(),
        "flat/firecraft-0.2.5-sbom.cdx.json".to_string(),
        "flat/FireCraft-0.2.5-source.tar.gz".to_string(),
    ];
    assert_eq!(argv, expected);

    // `gh release create` has no -a/--asset flag; passing one aborts the
    // command, which is what broke the release job.
    assert!(!argv.iter().any(|arg| arg == "-a" || arg == "--asset"));
}

#[test]
fn create_keeps_a_filename_with_a_space_in_one_argument() {
    let sandbox = Sandbox::new("space");
    sandbox.write("src/native/firecraft 0.2.5 linux.tar.gz", "spaced");
    sandbox.write("src/RELEASE_SHA256.txt", "checksums");
    sandbox.write("notes.md", "release notes");
    let output = run_script(&sandbox, &["stage", "src", "flat", "assets.zlist"]);
    assert!(
        output.status.success(),
        "staging failed: {}",
        stderr_of(&output)
    );

    let (bin, argv_out) = install_stub_gh(&sandbox);
    let output = run_create(
        &sandbox,
        &["v0.2.5", "assets.zlist", "notes.md"],
        &bin,
        &argv_out,
    );
    assert!(
        output.status.success(),
        "creating the release failed: {}",
        stderr_of(&output)
    );

    let argv = recorded_argv(&argv_out);
    assert!(
        argv.iter()
            .any(|arg| arg == "flat/firecraft 0.2.5 linux.tar.gz"),
        "the spaced filename was not passed as a single argument: {argv:?}"
    );
    // A word-split filename would show up as two separate arguments.
    assert!(!argv.iter().any(|arg| arg == "flat/firecraft"));
}

#[test]
fn stage_refuses_an_empty_artifact_tree() {
    let sandbox = Sandbox::new("empty");
    fs::create_dir_all(sandbox.path("src")).expect("create an empty source directory");

    let output = run_script(&sandbox, &["stage", "src", "flat", "assets.zlist"]);
    assert!(!output.status.success(), "staging an empty tree must fail");
    assert!(
        stderr_of(&output).contains("no release artifacts"),
        "the failure must explain itself: {}",
        stderr_of(&output)
    );
}

#[test]
fn create_refuses_to_run_with_an_empty_asset_list() {
    let sandbox = Sandbox::new("no-assets");
    sandbox.write("notes.md", "release notes");
    sandbox.write("assets.zlist", "");

    let (bin, argv_out) = install_stub_gh(&sandbox);
    let output = run_create(
        &sandbox,
        &["v0.2.5", "assets.zlist", "notes.md"],
        &bin,
        &argv_out,
    );
    assert!(
        !output.status.success(),
        "creating a release with no assets must fail"
    );
    assert!(
        stderr_of(&output).contains("no release assets"),
        "the failure must explain itself: {}",
        stderr_of(&output)
    );
    // `gh` must not have been reached at all.
    assert!(
        !argv_out.exists(),
        "gh was invoked with an empty asset list"
    );
}

#[test]
fn stage_skips_a_duplicate_asset_name_and_says_so() {
    let sandbox = Sandbox::new("duplicate");
    sandbox.write("src/a/same.tar.gz", "first");
    sandbox.write("src/b/same.tar.gz", "second");
    sandbox.write("src/RELEASE_SHA256.txt", "checksums");

    let output = run_script(&sandbox, &["stage", "src", "flat", "assets.zlist"]);
    assert!(
        output.status.success(),
        "staging failed: {}",
        stderr_of(&output)
    );

    // A release asset is named by its basename, so only one of the two
    // can be attached; the other is skipped with a warning rather than
    // silently overwriting it.
    assert_eq!(
        sandbox.dir_names("flat"),
        vec!["RELEASE_SHA256.txt".to_string(), "same.tar.gz".to_string()]
    );
    assert!(
        stderr_of(&output).contains("duplicate release asset name"),
        "the skip must be reported: {}",
        stderr_of(&output)
    );
    let assets = staged_assets(&sandbox, "assets.zlist");
    assert_eq!(
        assets.len(),
        2,
        "the duplicate must appear once: {assets:?}"
    );
}

#[test]
fn stage_skips_a_duplicate_asset_name_that_is_not_adjacent() {
    // The sorted source order is a/same.tar.gz, b/other.txt, c/same.tar.gz:
    // the two colliding basenames are NOT adjacent. A duplicate check that
    // only compares against the previous basename resets on the file in
    // between, so the later duplicate silently overwrites the earlier one
    // and the release ships the wrong bytes under that name.
    let sandbox = Sandbox::new("duplicate-nonadjacent");
    sandbox.write("src/a/same.tar.gz", "first");
    sandbox.write("src/b/other.txt", "between");
    sandbox.write("src/c/same.tar.gz", "second");

    let output = run_script(&sandbox, &["stage", "src", "flat", "assets.zlist"]);
    assert!(
        output.status.success(),
        "staging failed: {}",
        stderr_of(&output)
    );

    assert_eq!(
        sandbox.dir_names("flat"),
        vec!["other.txt".to_string(), "same.tar.gz".to_string()],
        "the colliding basename must be staged once"
    );
    assert!(
        stderr_of(&output).contains("duplicate release asset name"),
        "the skip must be reported: {}",
        stderr_of(&output)
    );
    let assets = staged_assets(&sandbox, "assets.zlist");
    assert_eq!(
        assets.len(),
        2,
        "the duplicate must appear once: {assets:?}"
    );
    // The first source wins; the second is skipped, not silently
    // substituted over the top of it.
    assert_eq!(
        std::fs::read_to_string(sandbox.path("flat/same.tar.gz")).expect("read the staged asset"),
        "first",
        "the later duplicate must not overwrite the staged bytes"
    );
}

#[test]
fn stage_refuses_a_destination_that_already_holds_files() {
    let sandbox = Sandbox::new("stale");
    sandbox.write("src/RELEASE_SHA256.txt", "checksums");
    sandbox.write("flat/stale.tar.gz", "left over from an earlier run");

    let output = run_script(&sandbox, &["stage", "src", "flat", "assets.zlist"]);
    assert!(
        !output.status.success(),
        "staging into a populated directory must fail"
    );
    assert!(
        stderr_of(&output).contains("not empty"),
        "the failure must explain itself: {}",
        stderr_of(&output)
    );
}

#[test]
fn gh_release_create_takes_assets_positionally_and_has_no_asset_flag() {
    // The contract this workflow depends on, checked against the real
    // `gh` when it is installed. The stub-based test above proves what we
    // send; this proves the CLI accepts it.
    let help = Command::new("gh")
        .args(["release", "create", "--help"])
        .output();
    let Ok(help) = help else {
        return;
    };
    let text = String::from_utf8_lossy(&help.stdout).into_owned();
    let usage = text
        .lines()
        .find(|line| line.trim_start().starts_with("gh release create"))
        .unwrap_or_else(|| panic!("no usage line in the gh help text: {text}"));
    // The assets are the positional file arguments: "<tag>" then the
    // files. Older `gh` builds spell the file list "[...]", newer ones
    // "[<files>...]"; both keep it positional.
    assert!(
        usage.contains("[<tag>]"),
        "unexpected gh usage line: {usage}"
    );
    assert!(
        usage.contains("..."),
        "gh must take the release assets positionally: {usage}"
    );
    assert!(
        !text.contains("--asset"),
        "gh release create must not grow an --asset flag: {text}"
    );
}
