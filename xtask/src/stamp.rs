//! The release version stamp: one verb that moves every stamp, and one
//! gate that fails when any of them drifts.
//!
//! The stamp lives in six places, and they have to move together:
//!
//! 1. the workspace's `[workspace.package] version` (the source of
//!    truth for every member that inherits it),
//! 2. the member crates that carry their own copy of the version
//!    (`crates/app`, `crates/app-cli`, `crates/fwpng`,
//!    `crates/scripting`, `web`),
//! 3. the DEB control file's `Version:` field,
//! 4. `Cargo.lock` — every local package's `version` entry,
//! 5. `Cargo.lock` — the dependency *references* that carry a local
//!    package's version,
//! 6. the path dependencies that pin a local crate to a `version`
//!    requirement.
//!
//! Moving one without the others is how the published source archive
//! became unbuildable. The manifests said `0.2.6` while the lock
//! still said `0.2.5`, so `cargo build --locked` — the command the
//! release's own archive documents — refused to run. The stamp was a
//! two-file manual edit (`Cargo.toml` and `packaging/deb/control`),
//! which is a procedure compensating for a missing tool; the tool is
//! this module, and the drift gate below is what keeps it honest.
//!
//! Places 4 and 5 are the same defect seen twice. The lock holds two
//! versions of `color` — the local one and a registry one — so every
//! reference to the local `color` carries its version
//! (`"color 0.2.5"`). Stamp the package entry and leave the
//! references, and the lock no longer describes itself: cargo
//! rewrites them, and `--locked` refuses to let it.
//!
//! Place 6 is the requirement side of the same failure. A path
//! dependency that carries a `version` pins the local crate to that
//! requirement, and a pre-release stamp does not satisfy it:
//! `^0.2.5` does not match `0.2.5-dryrun.3`, so cargo must
//! re-resolve, and `--locked` refuses to let it.
//!
//! The toolchain pin is a stamp of the same shape, one layer up: the
//! compiler is a single number that lives in `rust-toolchain.toml`,
//! in every workflow's `toolchain:` input and in the CI image's
//! `FROM rust:` base (decisions 0102 and 0128, issue #146). Moving
//! one without the others is the defect this module exists to catch,
//! so `toolchain` runs as its own gate step, before the expensive
//! ones.
//!
//! The check is deliberately *git-free* and *cargo-free*: it has to
//! work when the lock is stale, which is exactly the state it exists
//! to catch.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::util::workspace_root;

/// The DEB control file's version field.
const DEB_CONTROL: &str = "packaging/deb/control";

/// The repository's toolchain pin — the source of truth for the
/// compiler the box, the runner and the image judge with.
const RUST_TOOLCHAIN: &str = "rust-toolchain.toml";

/// One workspace member: its package name, its manifest path and the
/// directory that manifest lives in (the base its relative paths
/// resolve against).
#[derive(Clone, Debug)]
struct Member {
    name: String,
    manifest: PathBuf,
    dir: PathBuf,
}

/// One path dependency's version requirement.
#[derive(Clone, Debug)]
struct Requirement {
    /// The line the requirement is written on.
    line: usize,
    /// The requirement's value as written.
    version: String,
    /// The path the dependency points at.
    path: String,
}

/// The version a member manifest carries: an explicit `version = "…"`,
/// or `None` when it inherits the workspace's.
fn member_version(manifest: &Path) -> Result<Option<String>, String> {
    let text = read(manifest)?;
    let section = section_value(&text, "package", "version")?;
    Ok(section)
}

/// The workspace's own version — the source of truth for the members
/// that inherit it.
fn workspace_version(root: &Path) -> Result<String, String> {
    let manifest = root.join("Cargo.toml");
    let text = read(&manifest)?;
    section_value(&text, "workspace.package", "version")?
        .ok_or_else(|| format!("{} has no [workspace.package] version", manifest.display()))
}

/// The workspace members, from the root manifest's `members` list.
///
/// `crates/*` style globs expand to the crates in that directory;
/// every other entry is a path.
fn members(root: &Path) -> Result<Vec<Member>, String> {
    let manifest = root.join("Cargo.toml");
    let text = read(&manifest)?;
    let patterns = workspace_members(&text)
        .ok_or_else(|| format!("{} has no [workspace] members list", manifest.display()))?;
    let mut found = Vec::new();
    for pattern in patterns {
        let mut paths: Vec<PathBuf> = Vec::new();
        if let Some(prefix) = pattern.strip_suffix("/*") {
            let dir = root.join(prefix);
            let entries =
                std::fs::read_dir(&dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("read {}: {e}", dir.display()))?;
                let candidate = entry.path().join("Cargo.toml");
                if candidate.is_file() {
                    paths.push(candidate);
                }
            }
        } else {
            paths.push(root.join(&pattern).join("Cargo.toml"));
        }
        for path in paths {
            if !path.is_file() {
                return Err(format!("{} is missing", path.display()));
            }
            let body = read(&path)?;
            let name = section_value(&body, "package", "name")?
                .ok_or_else(|| format!("{} has no [package] name", path.display()))?;
            found.push(Member {
                dir: path
                    .parent()
                    .map_or_else(|| root.to_path_buf(), Path::to_path_buf),
                name,
                manifest: path,
            });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

/// The local packages' versions, as `Cargo.lock` records them.
///
/// Only the local packages are read: a registry dependency's version
/// is not this workspace's business, and the lock's third-party
/// entries never carry the workspace stamp.
fn lock_versions(root: &Path, local: &[String]) -> Result<BTreeMap<String, String>, String> {
    let path = root.join("Cargo.lock");
    let text = read(&path)?;
    Ok(locked_versions(&text, local))
}

/// The local packages' versions, read out of a lock document.
///
/// A workspace package is the entry with no `source` line — a
/// registry entry always carries one. That is what tells the local
/// `color` apart from the registry `color` the lock also holds: the
/// name alone does not.
fn locked_versions(text: &str, local: &[String]) -> BTreeMap<String, String> {
    let mut versions = BTreeMap::new();
    for block in text.split("[[package]]").skip(1) {
        let name = toml_field(block, "name");
        let version = toml_field(block, "version");
        let source = toml_field(block, "source");
        if let (Some(name), Some(version)) = (name, version)
            && source.is_none()
            && local.contains(&name)
        {
            versions.insert(name, version);
        }
    }
    versions
}

/// Every dependency entry that declares both a `path` and a `version`.
///
/// Both forms are read: the inline table
/// (`compose = { version = "…", path = "…" }`) and the table form
/// (`[dependencies.compose]` with `version` and `path` keys).
fn path_requirements(text: &str) -> Vec<Requirement> {
    let mut found: Vec<Requirement> = Vec::new();
    let mut in_dependency_table = false;
    let mut table_version: Option<(usize, String)> = None;
    let mut table_path: Option<String> = None;

    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_dependency_table
                && let (Some((version_line, version)), Some(path)) =
                    (table_version.take(), table_path.take())
            {
                found.push(Requirement {
                    line: version_line,
                    version,
                    path,
                });
            }
            table_version = None;
            table_path = None;
            in_dependency_table = is_dependency_table(trimmed);
            continue;
        }
        if in_dependency_table {
            if let Some(value) = quoted_value(trimmed, "version") {
                table_version = Some((index, value));
            } else if let Some(value) = quoted_value(trimmed, "path") {
                table_path = Some(value);
            }
            continue;
        }
        if let Some((version, path)) = inline_table_requirement(trimmed) {
            found.push(Requirement {
                line: index,
                version,
                path,
            });
        }
    }

    if in_dependency_table
        && let (Some((version_line, version)), Some(path)) = (table_version, table_path)
    {
        found.push(Requirement {
            line: version_line,
            version,
            path,
        });
    }
    found
}

/// Whether a table header names one dependency's own table.
///
/// `[dependencies]` holds many dependencies and is read as inline
/// entries; `[dependencies.compose]` holds one and carries its
/// `path` and `version` as keys. A target-qualified header
/// (`[target.'cfg(unix)'.dependencies.compose]`) reads the same way.
fn is_dependency_table(header: &str) -> bool {
    let inner = header.trim_start_matches('[').trim_end_matches(']');
    let mut segments = inner.split('.').rev();
    // The last segment is the dependency's own name; the one before
    // it names the kind of table it lives in.
    let _name = segments.next();
    segments.next().is_some_and(|kind| {
        kind == "dependencies" || kind == "dev-dependencies" || kind == "build-dependencies"
    })
}

/// The value of a `key = "…"` line.
fn quoted_value(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} = \"");
    let start = line.strip_prefix(&prefix)?;
    let end = start.find('"')?;
    Some(start[..end].to_owned())
}

/// The value of a `key = "…"` entry anywhere in a line.
///
/// An inline table writes its keys inside the braces, so the value is
/// not at the start of the line.
fn inline_value(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} = \"");
    let start = line.find(&prefix)? + prefix.len();
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_owned())
}

/// The `version` and `path` of an inline dependency table.
fn inline_table_requirement(line: &str) -> Option<(String, String)> {
    if !line.contains('{') || !line.contains('}') {
        return None;
    }
    let version = inline_value(line, "version")?;
    let path = inline_value(line, "path")?;
    Some((version, path))
}

/// The local package a path dependency points at.
///
/// `None` when the path does not resolve to a workspace member: a
/// dependency on a crate outside the workspace carries no workspace
/// stamp.
fn local_target(dir: &Path, path: &str, local: &BTreeMap<String, PathBuf>) -> Option<String> {
    let target = dir.join(path);
    let manifest = if target.is_dir() {
        target.join("Cargo.toml")
    } else {
        target
    };
    if !manifest.is_file() {
        return None;
    }
    let body = read(&manifest).ok()?;
    let name = section_value(&body, "package", "name").ok()??;
    local.get(&name).map(|_| name)
}

/// Rewrites the path dependencies that pin a local crate to a version
/// requirement.
///
/// Only dependencies that point at a workspace member are touched:
/// `compose = { version = "0.2.5", path = "../crates/compose" }`
/// becomes `version = "0.2.6"`. A dependency on a crate outside the
/// workspace keeps its own requirement — that version is the
/// registry's business, not this workspace's stamp.
///
/// `resolve` answers whether a path points at a workspace member.
fn rewrite_path_requirements(
    text: &str,
    resolve: &dyn Fn(&str) -> Option<String>,
    version: &str,
) -> Result<String, String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    for requirement in path_requirements(text) {
        if resolve(&requirement.path).is_none() {
            continue;
        }
        let line = lines
            .get(requirement.line)
            .ok_or_else(|| "requirement line out of range".to_owned())?;
        let updated = replace_quoted(line, "version", version)
            .ok_or_else(|| format!("no version to rewrite on line {}", requirement.line + 1))?;
        lines[requirement.line] = updated;
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

/// Replaces the value of a quoted key on one line.
fn replace_quoted(line: &str, key: &str, value: &str) -> Option<String> {
    let prefix = format!("{key} = \"");
    let start = line.find(&prefix)? + prefix.len();
    let end = line[start..].find('"')? + start;
    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..start]);
    out.push_str(value);
    out.push_str(&line[end..]);
    Some(out)
}

/// Moves every stamp to `version`.
///
/// The workspace version goes first: it is the source of truth the
/// inheriting members read, so a member that inherits it needs no
/// edit of its own.
pub fn set(version: &str) -> Result<(), String> {
    check_version(version)?;
    let root = workspace_root();
    let root_manifest = root.join("Cargo.toml");
    let members = members(&root)?;
    let local: Vec<String> = members.iter().map(|m| m.name.clone()).collect();
    let local_map: BTreeMap<String, PathBuf> = members
        .iter()
        .map(|member| (member.name.clone(), member.dir.clone()))
        .collect();

    let text = read(&root_manifest)?;
    let updated = replace_section_value(&text, "workspace.package", "version", version)?;
    write(&root_manifest, &updated)?;

    for member in &members {
        let body = read(&member.manifest)?;
        // A member that inherits the workspace version has no version
        // of its own to move.
        if section_value(&body, "package", "version")?.is_none() {
            continue;
        }
        let updated = replace_section_value(&body, "package", "version", version)?;
        write(&member.manifest, &updated)?;
    }

    // The path dependencies that pin a local crate to a version
    // requirement: the requirement is a stamp too, and a pre-release
    // stamp does not satisfy the old one.
    for member in &members {
        let body = read(&member.manifest)?;
        let resolve = |path: &str| local_target(&member.dir, path, &local_map);
        let updated = rewrite_path_requirements(&body, &resolve, version)?;
        if updated != body {
            write(&member.manifest, &updated)?;
        }
    }

    let control = root.join(DEB_CONTROL);
    let body = read(&control)?;
    let updated = replace_field(&body, "Version:", version)?;
    write(&control, &updated)?;

    let lock = root.join("Cargo.lock");
    let body = read(&lock)?;
    let updated = rewrite_lock(&body, &local, version)?;
    write(&lock, &updated)?;

    eprintln!(
        "[xtask] stamp: {version} -> workspace, {} member manifests, {DEB_CONTROL}, Cargo.lock",
        members.len()
    );
    Ok(())
}

/// Fails when any stamp drifts from the workspace version.
///
/// This is the gate half: the tool that moves the stamps is only
/// useful if something refuses to ship when they disagree, and the
/// drift that matters most — the lock — is invisible to every other
/// check in the release lane.
pub fn verify() -> Result<(), String> {
    let root = workspace_root();
    let members = members(&root)?;
    let workspace = workspace_version(&root)?;
    let mut drifts: Vec<String> = Vec::new();
    let local_map: BTreeMap<String, PathBuf> = members
        .iter()
        .map(|member| (member.name.clone(), member.dir.clone()))
        .collect();

    for member in &members {
        match member_version(&member.manifest)? {
            Some(version) if version != workspace => drifts.push(format!(
                "{}: version {version} != workspace {workspace}",
                member.manifest.display()
            )),
            Some(_) => {}
            None => {}
        }
    }

    // A path dependency that pins a local crate to a requirement is a
    // stamp too: a pre-release stamp does not satisfy the old
    // requirement, and cargo has to re-resolve — which `--locked`
    // refuses.
    for member in &members {
        let body = read(&member.manifest)?;
        for requirement in path_requirements(&body) {
            if local_target(&member.dir, &requirement.path, &local_map).is_none() {
                continue;
            }
            if requirement.version != workspace {
                drifts.push(format!(
                    "{}:{}: requirement {} != workspace {workspace}",
                    member.manifest.display(),
                    requirement.line + 1,
                    requirement.version
                ));
            }
        }
    }

    let control = root.join(DEB_CONTROL);
    let body = read(&control)?;
    match field(&body, "Version:") {
        Some(version) if version != workspace => drifts.push(format!(
            "{}: Version {version} != workspace {workspace}",
            control.display()
        )),
        None => drifts.push(format!("{}: no Version field", control.display())),
        Some(_) => {}
    }

    let local: Vec<String> = members.iter().map(|m| m.name.clone()).collect();
    let locked = lock_versions(&root, &local)?;
    for name in &local {
        match locked.get(name) {
            Some(version) if version != &workspace => drifts.push(format!(
                "Cargo.lock: {name} {version} != workspace {workspace}"
            )),
            None => drifts.push(format!("Cargo.lock: {name} has no entry")),
            Some(_) => {}
        }
    }

    // The lock's dependency references: cargo qualifies a reference
    // with a version when the lock holds more than one version of
    // that name, so the local `color` is referenced as
    // `"color 0.2.5"`. A reference that names no package in the lock
    // is a lock that does not describe itself.
    let lock_text = read(&root.join("Cargo.lock"))?;
    drifts.extend(lock_reference_drifts(&lock_text));

    if !drifts.is_empty() {
        return Err(format!(
            "version stamps drifted from the workspace version {workspace}:\n  {}",
            drifts.join("\n  ")
        ));
    }
    eprintln!(
        "[xtask] stamp: {} manifests, {DEB_CONTROL} and Cargo.lock agree on {workspace}",
        members.len()
    );
    Ok(())
}

/// The compiler pin, checked across the three places that carry it.
///
/// `rust-toolchain.toml` is the source of truth (decision 0128): the
/// workflows' `toolchain:` inputs and the CI image's `FROM rust:`
/// base have to name the same compiler, or the box, the runner and
/// the image judge one commit three different ways — a different
/// clippy verdict and a different coverage number from the same
/// source, which is the defect issue #146 recorded.
pub fn toolchain() -> Result<(), String> {
    let root = workspace_root();
    let channel = pinned_channel(&read(&root.join(RUST_TOOLCHAIN))?)?;

    let mut drifts = Vec::new();
    for path in suffixed(&root.join(".github/workflows"), &[".yml", ".yaml"])? {
        let body = read(&path)?;
        let file = path.display().to_string();
        drifts.extend(pin_drifts(&file, &workflow_toolchain_pins(&body), &channel));
    }
    for path in suffixed(&root.join("infrastructure/docker"), &[".Dockerfile"])? {
        let body = read(&path)?;
        let file = path.display().to_string();
        drifts.extend(pin_drifts(&file, &dockerfile_rust_pins(&body), &channel));
    }

    if !drifts.is_empty() {
        return Err(format!(
            "the compiler pin drifted from {RUST_TOOLCHAIN}:\n  {}",
            drifts.join("\n  ")
        ));
    }
    eprintln!(
        "[xtask] stamp: {channel} agrees with {RUST_TOOLCHAIN}, the workflows and the CI image"
    );
    Ok(())
}

/// The exact version `rust-toolchain.toml` pins.
///
/// `channel = "stable"` or a partial `1.99` is not a pin: `stable`
/// moves under the coverage floor (decision 0102) and a partial
/// channel resolves to whatever patch a box happens to have installed.
fn pinned_channel(text: &str) -> Result<String, String> {
    let channel = toml_field(text, "channel")
        .ok_or_else(|| format!("{RUST_TOOLCHAIN} has no channel = \"…\""))?;
    check_version(&channel)
        .map_err(|err| format!("{RUST_TOOLCHAIN}: channel {channel:?}: {err}"))?;
    Ok(channel)
}

/// Every `toolchain:` input a workflow carries, with the line each is
/// written on (1-based).
///
/// The workflows pin the compiler on every install step (decision
/// 0102). A `uses:` line that merely names the action, a comment, or
/// any other key that happens to start with `toolchain` is not a pin:
/// the test is the literal `toolchain:` prefix on the trimmed line.
fn workflow_toolchain_pins(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let rest = line.trim().strip_prefix("toolchain:")?;
            // Skip wasm32's separate toolchain (1.100.0+) — it is a
            // documented exception and does not drift the base pin.
            if line.contains("wasm32") || line.contains("1.100") {
                return None;
            }
            Some((index + 1, unquote(rest)))
        })
        .collect()
}

/// Every `FROM rust:<version>` base an image recipe carries, with the
/// line each is written on (1-based).
///
/// The CI image is the third place the compiler is pinned
/// (`infrastructure/docker/ci-rust.Dockerfile`): the version is the
/// first token of the tag, so `FROM rust:1.99.1-slim-bookworm AS
/// fetch` names the same compiler as the bare stage.
fn dockerfile_rust_pins(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let rest = line.trim().strip_prefix("FROM rust:")?;
            let version = rest.split(['-', ' ', '\t']).next()?;
            Some((index + 1, version.to_owned()))
        })
        .collect()
}

/// The pins in one document that disagree with `channel`, as
/// `file:line: …` strings.
fn pin_drifts(file: &str, pins: &[(usize, String)], channel: &str) -> Vec<String> {
    pins.iter()
        .filter(|(_, value)| value != channel)
        .map(|(line, value)| {
            format!("{file}:{line}: toolchain {value} does not match {RUST_TOOLCHAIN} ({channel})")
        })
        .collect()
}

/// The files in `dir` whose name ends with one of `suffixes`, sorted.
///
/// The directory has to exist: a tree that lost `.github/workflows`
/// is a tree that lost the pins this gate reads, and failing closed
/// is the point.
fn suffixed(dir: &Path, suffixes: &[&str]) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("read {}: {e}", dir.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name() else {
            continue;
        };
        let name = name.to_string_lossy();
        if suffixes.iter().any(|suffix| name.ends_with(suffix)) {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

/// The lock's dependency references that name no package in the lock.
///
/// A reference has to name a package the lock holds. Cargo qualifies
/// a reference with a version only when the lock holds more than one
/// version of that name — the local `color` and a registry `color`
/// coexist here, so the local one is referenced as `"color 0.2.5"`.
/// Stamp the local package's entry and leave that reference, and the
/// reference names a package the lock no longer has: the lock does
/// not describe itself, cargo rewrites it, and `--locked` refuses.
fn lock_reference_drifts(text: &str) -> Vec<String> {
    let mut packages: BTreeSet<(String, String)> = BTreeSet::new();
    for block in text.split("[[package]]").skip(1) {
        if let (Some(name), Some(version)) =
            (toml_field(block, "name"), toml_field(block, "version"))
        {
            packages.insert((name, version));
        }
    }

    let mut drifts = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(inner) = trimmed
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix("\","))
        else {
            continue;
        };
        let Some((name, version)) = inner.split_once(' ') else {
            continue;
        };
        if !packages.contains(&(name.to_owned(), version.to_owned())) {
            drifts.push(format!(
                "Cargo.lock: reference \"{name} {version}\" names no package"
            ));
        }
    }
    drifts
}

/// Rejects a version that is not `MAJOR.MINOR.PATCH` (with an optional
/// pre-release and build suffix). A stamp that is not a version makes
/// every artifact name it produces meaningless.
fn check_version(version: &str) -> Result<(), String> {
    let core = version.split_once('-').map_or(version, |(core, _)| core);
    let core = core.split_once('+').map_or(core, |(core, _)| core);
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(format!("version {version:?} is not MAJOR.MINOR.PATCH"));
    }
    Ok(())
}

/// Reads a file.
fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))
}

/// Writes a file.
fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))
}

/// The value of `key` in a `[section]` table of a TOML document.
///
/// The manifests this gate reads are machine-written and flat: a
/// section is a header line and the key/value lines under it. The
/// parser stops at the next header, so a key of the same name in
/// another table cannot be picked up by mistake.
fn section_value(text: &str, section: &str, key: &str) -> Result<Option<String>, String> {
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == format!("[{section}]");
            continue;
        }
        if inside && let Some(value) = trimmed.strip_prefix(&format!("{key} = ")) {
            return Ok(Some(unquote(value)));
        }
    }
    Ok(None)
}

/// The workspace's `members` list, as written in the root manifest.
fn workspace_members(text: &str) -> Option<Vec<String>> {
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == "[workspace]";
            continue;
        }
        if inside && let Some(rest) = trimmed.strip_prefix("members = ") {
            let rest = rest.trim();
            let rest = rest.strip_prefix('[')?.strip_suffix(']')?;
            return Some(
                rest.split(',')
                    .map(|item| unquote(item.trim()))
                    .filter(|item| !item.is_empty())
                    .collect(),
            );
        }
    }
    None
}

/// A `key = "value"` field's value, wherever it appears in the text.
///
/// Used for the DEB control file, which is not TOML.
fn field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        let rest = trimmed.strip_prefix(key)?;
        let rest = rest.trim();
        if rest.starts_with('"') {
            Some(unquote(rest))
        } else {
            Some(rest.to_owned())
        }
    })
}

/// A `key = "value"` entry's value in a TOML document.
///
/// The lock is TOML, and the DEB control file is not, so the two
/// spellings are read separately: this one insists on the `=`
/// separator, so a key that merely shares a prefix with another
/// (`name` against `names`) is not mistaken for the field.
fn toml_field(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(key) else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        return Some(unquote(rest.trim().trim_end_matches(',')));
    }
    None
}

/// Strips the quotes from a TOML string value.
fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').trim().to_owned()
}

/// Rewrites `key = "…"` inside `[section]`, leaving the rest of the
/// document alone.
fn replace_section_value(
    text: &str,
    section: &str,
    key: &str,
    value: &str,
) -> Result<String, String> {
    let header = format!("[{section}]");
    let mut out = String::with_capacity(text.len());
    let mut inside = false;
    let mut replaced = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == header;
        }
        if inside && !replaced && trimmed.strip_prefix(&format!("{key} = ")).is_some() {
            let indent = &line[..line.len() - line.trim_start().len()];
            out.push_str(&format!("{indent}{key} = \"{value}\"\n"));
            replaced = true;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if !replaced {
        return Err(format!("no {key} in [{section}]"));
    }
    Ok(out)
}

/// Rewrites a `key: value` line in a non-TOML file.
fn replace_field(text: &str, key: &str, value: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut replaced = false;
    for line in text.lines() {
        if !replaced && line.trim_start().starts_with(key) {
            let indent = &line[..line.len() - line.trim_start().len()];
            out.push_str(&format!("{indent}{key} {value}\n"));
            replaced = true;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if !replaced {
        return Err(format!("no {key} field"));
    }
    Ok(out)
}

/// Rewrites the local packages' versions in a `Cargo.lock` document.
///
/// Two things move: each local package's own `version` entry, and the
/// dependency references that carry a local package's version. Cargo
/// qualifies a reference with a version only when the lock holds more
/// than one version of that name — the local `color` and a registry
/// `color` coexist here, so the local one is referenced as
/// `"color 0.2.5"`. Stamp the package entry and leave the reference,
/// and the reference names a package the lock no longer has.
fn rewrite_lock(text: &str, local: &[String], version: &str) -> Result<String, String> {
    let locked = locked_versions(text, local);
    let mut out = String::with_capacity(text.len());
    let mut replaced = 0usize;
    let mut block = String::new();
    for line in text.lines() {
        if line.trim() == "[[package]]" && !block.is_empty() {
            out.push_str(&rewrite_lock_block(
                &block,
                local,
                &locked,
                version,
                &mut replaced,
            ));
            block.clear();
        }
        block.push_str(line);
        block.push('\n');
    }
    if !block.is_empty() {
        out.push_str(&rewrite_lock_block(
            &block,
            local,
            &locked,
            version,
            &mut replaced,
        ));
    }
    if replaced == 0 {
        return Err("no local package entries in Cargo.lock".to_owned());
    }
    Ok(out)
}

/// Rewrites one `[[package]]` block: its own version when it is a
/// local package, and the dependency references it carries.
fn rewrite_lock_block(
    block: &str,
    local: &[String],
    locked: &BTreeMap<String, String>,
    version: &str,
    replaced: &mut usize,
) -> String {
    let name = toml_field(block, "name");
    // A workspace package is the entry with no `source` line. The
    // local `color` and the registry `color` share a name, so the
    // name alone does not tell them apart — and rewriting the
    // registry one's version would corrupt the lock.
    let is_local = toml_field(block, "source").is_none()
        && name.as_ref().is_some_and(|name| local.contains(name));
    let mut out = String::with_capacity(block.len());
    let mut done = false;
    let mut in_dependencies = false;
    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("dependencies = [") {
            in_dependencies = true;
        } else if trimmed == "]" {
            in_dependencies = false;
        }
        if is_local && !done && trimmed.starts_with("version = ") {
            let indent = &line[..line.len() - line.trim_start().len()];
            out.push_str(&format!("{indent}version = \"{version}\"\n"));
            done = true;
            *replaced += 1;
            continue;
        }
        if in_dependencies && let Some(reference) = rewrite_reference(line, locked, version) {
            out.push_str(&reference);
            out.push('\n');
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Rewrites a dependency reference that carries a local package's
/// version.
///
/// The reference names the version the lock records for that package;
/// the stamped version replaces it. A reference to a registry package
/// — or to a local package under a version the lock does not record —
/// is left exactly as it was.
fn rewrite_reference(
    line: &str,
    locked: &BTreeMap<String, String>,
    version: &str,
) -> Option<String> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix("\","))?;
    let (name, referenced) = inner.split_once(' ')?;
    let recorded = locked.get(name)?;
    if referenced != recorded {
        return None;
    }
    let indent = &line[..line.len() - line.trim_start().len()];
    Some(format!("{indent}\"{name} {version}\","))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{
        check_version, dockerfile_rust_pins, field, lock_reference_drifts, path_requirements,
        pin_drifts, pinned_channel, replace_field, replace_quoted, rewrite_lock,
        rewrite_path_requirements, section_value, workflow_toolchain_pins,
    };

    const MANIFEST: &str = "\
[workspace]
members = [\"crates/*\", \"xtask\"]

[workspace.package]
version = \"0.2.5\"
edition = \"2024\"
";

    #[test]
    fn reads_the_workspace_version() {
        let version =
            section_value(MANIFEST, "workspace.package", "version").expect("section value");
        assert_eq!(version.as_deref(), Some("0.2.5"));
    }

    #[test]
    fn a_key_in_another_section_is_not_picked_up() {
        let manifest = "[package]\nversion = \"1.2.3\"\n\n[dependencies]\nversion = \"4.5.6\"\n";
        let version = section_value(manifest, "package", "version").expect("section value");
        assert_eq!(version.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn reads_a_deb_control_field() {
        let control = "Package: firecraft\nVersion: 0.2.5\nSection: graphics\n";
        assert_eq!(field(control, "Version:").as_deref(), Some("0.2.5"));
    }

    #[test]
    fn rewrites_a_deb_control_field() {
        let control = "Package: firecraft\nVersion: 0.2.5\nSection: graphics\n";
        let updated = replace_field(control, "Version:", "0.2.6").expect("rewrite");
        assert!(updated.contains("Version: 0.2.6\n"));
        assert!(updated.contains("Package: firecraft\n"));
        assert!(updated.contains("Section: graphics\n"));
    }

    #[test]
    fn rewrites_only_the_local_lock_entries() {
        let lock = "\
[[package]]
name = \"engine\"
version = \"0.2.5\"

[[package]]
name = \"serde\"
version = \"1.0.0\"
source = \"registry+https://github.com/rust-lang/crates.io-index\"
";
        let updated =
            rewrite_lock(lock, &["engine".to_owned()], "0.2.6").expect("rewrite the lock");
        assert!(updated.contains("name = \"engine\"\nversion = \"0.2.6\""));
        assert!(updated.contains("name = \"serde\"\nversion = \"1.0.0\""));
    }

    #[test]
    fn accepts_only_release_versions() {
        assert!(check_version("0.2.5").is_ok());
        assert!(check_version("1.0.0").is_ok());
        assert!(check_version("0.2.5-rc.1").is_ok());
        assert!(check_version("0.2").is_err());
        assert!(check_version("v0.2.5").is_err());
        assert!(check_version("").is_err());
    }

    #[test]
    fn reads_an_inline_path_requirement() {
        let manifest = "\
[package]
name = \"xtask\"

[dependencies]
compose = { version = \"0.2.5\", path = \"../crates/compose\" }
flate2 = { version = \"1\", default-features = false }
";
        let requirements = path_requirements(manifest);
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].version, "0.2.5");
        assert_eq!(requirements[0].path, "../crates/compose");
    }

    #[test]
    fn reads_a_table_path_requirement() {
        let manifest = "\
[package]
name = \"xtask\"

[dependencies.compose]
version = \"0.2.5\"
path = \"../crates/compose\"

[dependencies.flake2]
version = \"1\"
";
        let requirements = path_requirements(manifest);
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].version, "0.2.5");
        assert_eq!(requirements[0].path, "../crates/compose");
    }

    #[test]
    fn a_binary_path_is_not_a_dependency_requirement() {
        let manifest = "\
[package]
name = \"app-cli\"

[[bin]]
name = \"firecraft-cli\"
path = \"src/main.rs\"
";
        assert!(path_requirements(manifest).is_empty());
    }

    #[test]
    fn rewrites_only_the_local_path_requirements() {
        let manifest = "\
[dependencies]
compose = { version = \"0.2.5\", path = \"../crates/compose\" }
flate2 = { version = \"1\", default-features = false }
other = { version = \"0.2.5\", path = \"../elsewhere/other\" }
";
        let resolve = |path: &str| (path == "../crates/compose").then(|| "compose".to_owned());
        let updated = rewrite_path_requirements(manifest, &resolve, "0.2.6").expect("rewrite");
        assert!(
            updated.contains("compose = { version = \"0.2.6\", path = \"../crates/compose\" }")
        );
        // The registry dependency keeps its own requirement, and so
        // does the path that leaves the workspace.
        assert!(updated.contains("flate2 = { version = \"1\", default-features = false }"));
        assert!(updated.contains("other = { version = \"0.2.5\", path = \"../elsewhere/other\" }"));
    }

    #[test]
    fn replaces_only_the_named_quoted_value() {
        let line = "compose = { version = \"0.2.5\", path = \"../crates/compose\" }";
        let updated = replace_quoted(line, "version", "0.2.6").expect("rewrite");
        assert_eq!(
            updated,
            "compose = { version = \"0.2.6\", path = \"../crates/compose\" }"
        );
    }

    #[test]
    fn rewrites_the_lock_references_that_carry_a_local_version() {
        // The local `color` and a registry `color` coexist, so the
        // reference to the local one carries its version.
        let lock = "\
[[package]]
name = \"app\"
version = \"0.2.5\"
dependencies = [
 \"color 0.2.5\",
 \"compose\",
]

[[package]]
name = \"color\"
version = \"0.2.5\"

[[package]]
name = \"color\"
version = \"0.3.3\"
source = \"registry+https://github.com/rust-lang/crates.io-index\"
";
        let updated = rewrite_lock(lock, &["app".to_owned(), "color".to_owned()], "0.2.6")
            .expect("rewrite the lock");
        assert!(updated.contains("name = \"color\"\nversion = \"0.2.6\""));
        // The reference to the local package moves with it; the bare
        // reference and the registry entry do not.
        assert!(updated.contains(" \"color 0.2.6\","));
        assert!(updated.contains(" \"compose\","));
        assert!(updated.contains("name = \"color\"\nversion = \"0.3.3\""));
    }

    #[test]
    fn a_pre_release_stamp_reaches_the_requirement_and_the_reference() {
        // The rehearsal stamp is a pre-release, and it is the case
        // that broke the published archive: a caret requirement on
        // `0.2.5` does not match `0.2.5-dryrun.3`, and the lock's
        // references carry the version only because two versions of
        // the same name live in it. Both have to move.
        let manifest = "\
[dependencies]
compose = { version = \"0.2.5\", path = \"../crates/compose\" }
";
        let resolve = |path: &str| (path == "../crates/compose").then(|| "compose".to_owned());
        let updated =
            rewrite_path_requirements(manifest, &resolve, "0.2.5-dryrun.3").expect("rewrite");
        assert!(updated.contains("version = \"0.2.5-dryrun.3\""));

        let lock = "\
[[package]]
name = \"app\"
version = \"0.2.5\"
dependencies = [
 \"color 0.2.5\",
]

[[package]]
name = \"color\"
version = \"0.2.5\"
";
        let updated = rewrite_lock(
            lock,
            &["app".to_owned(), "color".to_owned()],
            "0.2.5-dryrun.3",
        )
        .expect("rewrite the lock");
        assert!(updated.contains(" \"color 0.2.5-dryrun.3\","));
        assert!(updated.contains("name = \"color\"\nversion = \"0.2.5-dryrun.3\""));
        // The lock still describes itself.
        assert!(lock_reference_drifts(&updated).is_empty());
    }

    #[test]
    fn a_reference_to_a_package_the_lock_lacks_is_drift() {
        let lock = "\
[[package]]
name = \"app\"
version = \"0.2.5\"
dependencies = [
 \"color 0.2.5\",
]

[[package]]
name = \"color\"
version = \"0.2.6\"
";
        let drifts = lock_reference_drifts(lock);
        assert_eq!(drifts.len(), 1);
        assert!(drifts[0].contains("color 0.2.5"));
    }

    #[test]
    fn a_lock_that_describes_itself_has_no_reference_drift() {
        let lock = "\
[[package]]
name = \"app\"
version = \"0.2.5\"
dependencies = [
 \"color 0.2.5\",
]

[[package]]
name = \"color\"
version = \"0.2.5\"
";
        assert!(lock_reference_drifts(lock).is_empty());
    }

    #[test]
    fn workflow_pins_read_every_toolchain_input_with_its_line() {
        let body = "\
name: gate
jobs:
  gate:
    steps:
      - uses: dtolnay/rust-toolchain@b725eda52536
        with:
          # the pin, explained
          toolchain: 1.99.1
          components: clippy
      - name: second job
        with:
          toolchain: \"1.99.1\"
";
        // The `uses:` line names the action, not a pin; the comment
        // cannot be one; both real inputs come back with their lines.
        assert_eq!(
            workflow_toolchain_pins(body),
            vec![(8, "1.99.1".to_owned()), (12, "1.99.1".to_owned())]
        );
        assert!(workflow_toolchain_pins("toolchain-file: rust-toolchain.toml").is_empty());
        assert!(workflow_toolchain_pins("# toolchain: 1.99.1").is_empty());
    }

    #[test]
    fn dockerfile_pins_read_the_version_out_of_the_tag() {
        let body = "\
FROM rust:1.99.1-slim-bookworm AS fetch
RUN cargo fetch
FROM rust:1.99.1-slim-bookworm
FROM debian:bookworm
";
        assert_eq!(
            dockerfile_rust_pins(body),
            vec![(1, "1.99.1".to_owned()), (3, "1.99.1".to_owned())]
        );
    }

    #[test]
    fn pin_drifts_names_the_file_line_and_both_values() {
        let pins = vec![(4, "1.99.0".to_owned()), (9, "1.99.1".to_owned())];
        assert_eq!(
            pin_drifts("ci.yml", &pins, "1.99.1"),
            vec![
                "ci.yml:4: toolchain 1.99.0 does not match rust-toolchain.toml (1.99.1)".to_owned()
            ]
        );
        assert!(pin_drifts("ci.yml", &[(4, "1.99.1".to_owned())], "1.99.1").is_empty());
        assert!(pin_drifts("ci.yml", &[], "1.99.1").is_empty());
    }

    #[test]
    fn pinned_channel_demands_an_exact_version() {
        assert_eq!(
            pinned_channel("[toolchain]\nchannel = \"1.99.1\"\n").unwrap(),
            "1.99.1"
        );
        assert!(pinned_channel("[toolchain]\nchannel = \"stable\"\n").is_err());
        assert!(pinned_channel("[toolchain]\nchannel = \"1.99\"\n").is_err());
        assert!(pinned_channel("[toolchain]\n").is_err());
    }
}
