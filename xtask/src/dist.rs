//! Release assembly (M12 task 3): `xtask dist --web`. The native set
//! is a separate verb (`dist --native`, task 5).
//!
//! dist is a pure function of the tree, the same discipline as
//! export: build the release artifacts, stage them under `dist/` with
//! a pinned layout, sha256-manifest every file written, and enforce
//! the recorded budgets — no hand-assembled releases, no artifacts
//! from a stale tree (staging starts from an empty directory).
//!
//! The bundle is then smoked **on its own release bytes**: a headless
//! chromium loads a host page that embeds the staged bundle and drives
//! one open → apply → exportPng → exportDocument round trip through
//! the M5 postMessage bridge, with the verdict written into the DOM
//! for the gate to grep. A dist run that cannot prove the bridge
//! works does not claim an artifact.

use crate::util::{run_in, run_in_with_env, sha256_hex, workspace_root};
use base64::Engine as _;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Web bundle guardrails, in decimal bytes:
/// - 25,000,000 B: investigation warning; the build continues.
/// - 50,000,000 B: hard ceiling; the build fails above this value.
///
/// The measured Phase 21/22 baseline is 21,608,249 B (2026-11-10;
/// 8c68b0f; `GitForge` run 2bf47590-3f50-445c-9948-f0bc3587b483).
/// The 50 MB cap is an explicit policy safety limit, not a measured
/// target and not permission to let the bundle grow to 49 MB.
/// Decision 0079 retains the historical 22 MB measurement-based pin;
/// decision 0090 sets this larger temporary guardrail while optimization
/// work proceeds.
///
/// The web-only Cargo release profile uses opt-level=z, and Trunk runs
/// Binaryen with data-wasm-opt="z". Native desktop release settings are
/// unaffected. The twelve timing/memory budgets remain falling-only.
const WASM_SIZE_WARNING: u64 = 25_000_000;
const WASM_SIZE_CEILING: u64 = 50_000_000;

/// Wall-clock backstop for the headless browser: a hung WebGL init
/// kills the smoke instead of hanging `dist`.
const SMOKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Assembles the web bundle under `dist/web/`.
///
/// The bridge smoke ALWAYS runs. There used to be a
/// `FIRECRAFT_DIST_SKIP_SMOKE=1` escape hatch here, added while the
/// CI image baked no browser; it was a procedure compensating for that
/// tool defect, and it let the release gate ship a bundle it never
/// smoked. The CI image now bakes chromium, so the door is gone: a
/// missing browser or a failed round trip fails the dist run.
pub fn web() -> Result<(), String> {
    let staged = stage_web()?;
    let dir = staged
        .first()
        .and_then(|p| p.parent())
        .ok_or_else(|| "staging dir vanished".to_owned())?;
    let manifest = write_manifest(dir, &staged)?;
    let wasm_bytes = enforce_wasm_budget(&staged)?;
    smoke_bridge(&staged)?;
    eprintln!(
        "[xtask] dist web: {} file(s), wasm {wasm_bytes} B (warning {WASM_SIZE_WARNING}, hard ceiling {WASM_SIZE_CEILING}), manifest {manifest}, smoke green",
        staged.len(),
    );
    Ok(())
}

/// Assembles the native + CLI set under `dist/native/` (M12 task 5):
/// Linux `x86_64` tar.gz of the desktop shell, a CLI-only archive, and a
/// `.deb` built from the pinned control file — each sha256-manifested,
/// the deb read back with `dpkg-deb`, and the checked-in packaging
/// recipes parse-checked so a broken recipe fails the gate.
pub fn native() -> Result<(), String> {
    if std::env::consts::OS != "linux" || std::env::consts::ARCH != "x86_64" {
        return Err(format!(
            "dist --native pins Linux x86_64 (this fleet); this host is {} {} — \
             the other platforms ship as recipes (docs/TOOLCHAINS.md)",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }
    let version = workspace_version()?;
    let out = workspace_root().join("dist/native");
    let stage = out.join(".stage");
    clear_dir(&out)?;
    std::fs::create_dir_all(&stage).map_err(|e| format!("mkdir {}: {e}", stage.display()))?;

    // One release build feeds every archive (both bins in target/release).
    run_in(
        None,
        "cargo",
        &[
            "build",
            "--release",
            "--locked",
            "-p",
            "app",
            "-p",
            "app-cli",
        ],
    )?;
    let shell_bin = release_bin("firecraft")?;
    let cli_bin = release_bin("firecraft-cli")?;

    let desktop = stage_archive(
        &stage,
        &format!("firecraft-{version}"),
        &[
            ("bin/firecraft", &shell_bin),
            ("bin/firecraft-cli", &cli_bin),
        ],
        &out.join(format!("firecraft-{version}-linux-x86_64.tar.gz")),
    )?;
    let cli = stage_archive(
        &stage,
        &format!("firecraft-cli-{version}"),
        &[("bin/firecraft-cli", &cli_bin)],
        &out.join(format!("firecraft-cli-{version}-linux-x86_64.tar.gz")),
    )?;
    let deb = build_deb(
        &stage,
        &version,
        &shell_bin,
        &cli_bin,
        &out.join(format!("firecraft_{version}_amd64.deb")),
    )?;
    verify_deb(&deb, &version)?;
    let recipes = parse_recipes()?;

    let artifacts = vec![desktop, cli, deb];
    let manifest = write_manifest(&out, &artifacts)?;
    clear_dir(&stage)?;
    std::fs::remove_dir(&stage).map_err(|e| format!("rmdir stage: {e}"))?;
    eprintln!(
        "[xtask] dist native: 3 archives, manifest {manifest}, dpkg-deb verified, recipes parse-checked ({recipes} bash)"
    );
    Ok(())
}

/// Produces the source archive under `dist/source/` (T27 RP-05): a
/// `git archive` of the exact HEAD (the tree the gate itself runs on —
/// not a mutable branch), extracted to a scratch dir and verified by
/// a clean extraction check (Cargo.toml + Cargo.lock + every workspace
/// member present, no `source/` Adobe material) and `cargo metadata
/// --locked` parse inside the extraction. An archive that cannot be
/// re-extracted into a buildable tree is not shipped.
pub fn source() -> Result<(), String> {
    let version = workspace_version()?;
    let out = workspace_root().join("dist/source");
    if out.exists() {
        std::fs::remove_dir_all(&out).map_err(|e| format!("clear {}: {e}", out.display()))?;
    }
    std::fs::create_dir_all(&out).map_err(|e| format!("mkdir {}: {e}", out.display()))?;
    let archive = out.join(format!("FireCraft-{version}-source.tar.gz"));

    // git archive pins the commit identity inside the tar (pax header)
    // implicitly; the report below names HEAD so the receipt is
    // attributable to an exact commit even without re-deriving it.
    run_in(None, "git", &["rev-parse", "HEAD"])?;

    run_in(
        None,
        "git",
        &[
            "archive",
            "--format=tar.gz",
            "--prefix=FireCraft/",
            "-o",
            &archive.display().to_string(),
            "HEAD",
        ],
    )?;

    // Verification: clean extract into scratch (outside the tree), then
    // structural checks + `cargo metadata --locked` in the extraction.
    let scratch = out.join(".verify");
    std::fs::create_dir_all(&scratch).map_err(|e| format!("mkdir verify: {e}"))?;
    run_in(
        Some(&scratch),
        "tar",
        &["-xzf", &archive.display().to_string()],
    )?;
    let extracted = scratch.join("FireCraft");
    for must in [
        "Cargo.toml",
        "Cargo.lock",
        "LICENSE",
        "crates",
        "xtask",
        "web",
    ] {
        if !extracted.join(must).exists() {
            return Err(format!("source archive missing {must} after extraction"));
        }
    }
    if extracted.join("source").exists() {
        return Err("source archive leaked the git-ignored source/ tree".to_owned());
    }
    let meta = std::process::Command::new("cargo")
        .args(["metadata", "--locked", "--format-version", "1", "--no-deps"])
        .current_dir(&extracted)
        .output()
        .map_err(|e| format!("spawn cargo metadata: {e}"))?;
    if !meta.status.success() {
        return Err(format!(
            "cargo metadata failed in extracted source archive:\n{}",
            String::from_utf8_lossy(&meta.stderr).trim()
        ));
    }

    std::fs::remove_dir_all(&scratch).map_err(|e| format!("clean verify scratch: {e}"))?;
    let digest = sha256_hex_file(&archive)?;
    eprintln!(
        "[xtask] dist source: FireCraft-{version}-source.tar.gz ({digest}), \
         clean-extract + cargo metadata --locked verified"
    );
    Ok(())
}

/// sha256 of a file (hex), same digest form `write_manifest` uses.
fn sha256_hex_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

/// Emits the dependency SBOM under `dist/sbom/` (T27 RP-06): `CycloneDX`
/// 1.5 from the complete locked Cargo package inventory and resolved
/// dependency edges. Registry packages use Cargo purls; workspace,
/// path, and git packages use stable `FireCraft` bom-refs instead. No
/// signing/provenance claims are made here — those remain separate
/// deliverables once signing credentials exist.
pub fn sbom() -> Result<(), String> {
    let out = workspace_root().join("dist/sbom");
    if out.exists() {
        std::fs::remove_dir_all(&out).map_err(|e| format!("clear {}: {e}", out.display()))?;
    }
    std::fs::create_dir_all(&out).map_err(|e| format!("mkdir {}: {e}", out.display()))?;

    let raw = std::process::Command::new("cargo")
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(workspace_root())
        .output()
        .map_err(|e| format!("spawn cargo metadata: {e}"))?;
    if !raw.status.success() {
        return Err(format!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&raw.stderr).trim()
        ));
    }
    let meta: serde_json::Value =
        serde_json::from_slice(&raw.stdout).map_err(|e| format!("parse cargo metadata: {e}"))?;
    let version = workspace_version()?;

    let mut components = Vec::new();
    let packages = meta["packages"]
        .as_array()
        .ok_or("cargo metadata: no packages array")?;
    let mut package_refs = BTreeMap::new();
    for pkg in packages {
        let name = pkg["name"]
            .as_str()
            .ok_or("cargo metadata: package without name")?;
        let ver = pkg["version"]
            .as_str()
            .ok_or("cargo metadata: package without version")?;
        let (bom_ref, purl) = package_bom_ref(pkg)?;
        let id = pkg["id"]
            .as_str()
            .ok_or("cargo metadata: package without id")?;
        package_refs.insert(id.to_owned(), bom_ref.clone());
        let mut component = serde_json::json!({
            "type": "library",
            "name": name,
            "version": ver,
            "bom-ref": bom_ref,
        });
        if let Some(purl) = purl {
            component["purl"] = serde_json::json!(purl);
        }
        if let Some(license) = pkg["license"].as_str() {
            component["licenses"] = serde_json::json!([
                { "license": { "id": license } }
            ]);
        }
        components.push(component);
    }

    let root_ref = format!("urn:firecraft:application:firecraft@{version}");
    let dependencies = cyclonedx_dependencies(&meta, &package_refs, &root_ref)?;
    let serial = uuid_from_graph(packages)?;
    let bom = serde_json::json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": format!("urn:uuid:{serial}"),
        "version": 1,
        "metadata": {
            "timestamp": build_timestamp(),
            "component": {
                "type": "application",
                "name": "firecraft",
                "version": version,
                "bom-ref": root_ref,
            },
            "tools": [{
                "vendor": "FireCraft",
                "name": "xtask dist --sbom",
                "version": version,
            }],
        },
        "components": components,
        "dependencies": dependencies,
    });
    let path = out.join(format!("firecraft-{version}-sbom.cdx.json"));
    let text = serde_json::to_string_pretty(&bom).map_err(|e| format!("encode sbom: {e}"))?;
    std::fs::write(&path, text.as_bytes()).map_err(|e| format!("write {}: {e}", path.display()))?;
    let digest = sha256_hex_file(&path)?;
    eprintln!(
        "[xtask] dist sbom: {} ({} components, {digest})",
        path.display(),
        components.len()
    );
    Ok(())
}

/// Stable `CycloneDX` identity for one Cargo package. Only crates.io
/// registry packages receive a Cargo purl; path and git packages are
/// identified by a bom-ref derived from their source identity.
fn package_bom_ref(package: &serde_json::Value) -> Result<(String, Option<String>), String> {
    let name = package["name"]
        .as_str()
        .ok_or("cargo metadata: package without name")?;
    let version = package["version"]
        .as_str()
        .ok_or("cargo metadata: package without version")?;
    let source = package["source"].as_str();
    const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";
    if source == Some(CRATES_IO) {
        let purl = format!("pkg:cargo/{name}@{version}");
        return Ok((purl.clone(), Some(purl)));
    }

    let source_identity = if let Some(source) = source {
        source.to_owned()
    } else {
        let manifest = package["manifest_path"]
            .as_str()
            .ok_or_else(|| format!("cargo metadata: {name} has no manifest_path"))?;
        let path = Path::new(manifest);
        let relative = path
            .strip_prefix(workspace_root())
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        format!("path:{relative}")
    };
    let digest = sha256_hex(source_identity.as_bytes());
    Ok((
        format!("urn:firecraft:cargo:{name}@{version}:{digest}"),
        None,
    ))
}

/// Emits the `CycloneDX` dependency edges from Cargo's resolved graph,
/// plus a `FireCraft` application root pointing at all workspace members.
fn cyclonedx_dependencies(
    metadata: &serde_json::Value,
    package_refs: &BTreeMap<String, String>,
    root_ref: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("cargo metadata: resolve.nodes is missing")?;
    let mut dependencies = Vec::with_capacity(nodes.len() + 1);
    for node in nodes {
        let id = node["id"]
            .as_str()
            .ok_or("cargo metadata: resolve node without id")?;
        let reference = package_refs
            .get(id)
            .ok_or_else(|| format!("cargo metadata: no bom-ref for resolved package {id}"))?;
        let mut depends_on = BTreeSet::new();
        if let Some(edges) = node["deps"].as_array() {
            for edge in edges {
                let package_id = edge["pkg"]
                    .as_str()
                    .ok_or("cargo metadata: dependency edge without package id")?;
                let dependency_ref = package_refs.get(package_id).ok_or_else(|| {
                    format!("cargo metadata: no bom-ref for dependency {package_id}")
                })?;
                depends_on.insert(dependency_ref.clone());
            }
        }
        dependencies.push(serde_json::json!({
            "ref": reference,
            "dependsOn": depends_on.into_iter().collect::<Vec<_>>(),
        }));
    }

    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("cargo metadata: workspace_members is missing")?;
    let mut workspace_refs = BTreeSet::new();
    for member in members {
        let id = member
            .as_str()
            .ok_or("cargo metadata: workspace member id is not a string")?;
        let reference = package_refs
            .get(id)
            .ok_or_else(|| format!("cargo metadata: no bom-ref for workspace member {id}"))?;
        workspace_refs.insert(reference.clone());
    }
    dependencies.push(serde_json::json!({
        "ref": root_ref,
        "dependsOn": workspace_refs.into_iter().collect::<Vec<_>>(),
    }));
    Ok(dependencies)
}

/// Deterministic serial from the locked graph: UUIDv8-shaped digest of
/// the sorted package bom-refs, so source/version changes are reflected
/// without depending on absolute checkout paths.
fn uuid_from_graph(packages: &[serde_json::Value]) -> Result<String, String> {
    use sha2::Digest as _;
    let mut names: Vec<String> = packages
        .iter()
        .map(|package| package_bom_ref(package).map(|(reference, _)| reference))
        .collect::<Result<_, _>>()?;
    names.sort();
    names.dedup();
    let mut hasher = sha2::Sha256::new();
    for n in &names {
        hasher.update(n.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    // Format the first 16 bytes as a UUID; set version nibble to 8
    // (UUIDv8-style custom, deterministic) per RFC 9562.
    let mut b = [0u8; 16];
    b.copy_from_slice(&digest[..16]);
    b[6] = (b[6] & 0x0f) | 0x80;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex = |bytes: &[u8]| bytes.iter().map(|x| format!("{x:02x}")).collect::<String>();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        hex(&b[..4]),
        hex(&b[4..6]),
        hex(&b[6..8]),
        hex(&b[8..10]),
        hex(&b[10..16])
    ))
}

/// UTC timestamp for the SBOM metadata block (std-only, no chrono).
fn build_timestamp() -> String {
    // days since epoch -> civil date (Howard Hinnant's algorithm).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = now.div_euclid(86_400);
    let secs = now.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        m,
        d,
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Builds the release bundle with trunk and stages a fresh copy under
/// `dist/web/` — the staging directory starts empty every run, so a
/// dist artifact can never carry leftovers from an older tree.
/// `WebOS` packaging files (`appinfo.json`, `host.html`) from
/// `packaging/webos/` are copied and stamped with the workspace version.
fn stage_web() -> Result<Vec<PathBuf>, String> {
    let web = workspace_root().join("web");
    let out = workspace_root().join("dist/web");
    if out.exists() {
        std::fs::remove_dir_all(&out).map_err(|e| format!("clear {}: {e}", out.display()))?;
    }
    std::fs::create_dir_all(&out).map_err(|e| format!("mkdir {}: {e}", out.display()))?;
    run_in_with_env(
        Some(&web),
        "trunk",
        &["build", "--release"],
        &[("CARGO_PROFILE_RELEASE_OPT_LEVEL", "z")],
    )?;

    let mut staged = Vec::new();
    let entries =
        std::fs::read_dir(web.join("dist")).map_err(|e| format!("read trunk dist: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("readdir: {e}"))?;
        let source = entry.path();
        if !source.is_file() {
            continue;
        }
        let name = entry.file_name();
        let target = out.join(&name);
        std::fs::copy(&source, &target)
            .map_err(|e| format!("stage {}: {e}", name.to_string_lossy()))?;
        staged.push(target);
    }

    // Copy and stamp WebOS packaging files. appinfo.json MUST carry
    // the @VERSION@ placeholder: the v0.2.3→v0.2.4 window shipped a
    // WebOS package pinned to 0.2.3 because the template hardcoded a
    // literal and the replace below was silently a no-op.
    let version = workspace_version()?;
    let webos_src = workspace_root().join("packaging/webos");
    for entry in std::fs::read_dir(&webos_src).map_err(|e| format!("read packaging/webos: {e}"))? {
        let entry = entry.map_err(|e| format!("readdir packaging/webos: {e}"))?;
        let source = entry.path();
        let name = entry.file_name();
        let target = out.join(&name);
        let contents = std::fs::read_to_string(&source)
            .map_err(|e| format!("read {}: {e}", name.to_string_lossy()))?;
        // host.html is version-free by design (the smoke host page is
        // the WEBOS.md contract, extracted verbatim); appinfo.json is
        // the versioned surface.
        if name.to_string_lossy() == "appinfo.json" && !contents.contains("@VERSION@") {
            return Err(
                "packaging/webos/appinfo.json carries no @VERSION@ placeholder — \
                 the staged copy would keep a stale hardcoded version \
                 (the v0.2.4 WebOS appinfo claimed 0.2.3 this way)"
                    .to_owned(),
            );
        }
        let stamped = contents.replace("@VERSION@", &version);
        if stamped.contains("@VERSION@") {
            return Err(format!(
                "packaging/webos/{} still carries @VERSION@ after stamping",
                name.to_string_lossy()
            ));
        }
        std::fs::write(&target, stamped)
            .map_err(|e| format!("stage {}: {e}", name.to_string_lossy()))?;
        staged.push(target);
    }

    staged.sort();
    if staged.is_empty() {
        return Err("trunk produced no files — nothing to stage".to_owned());
    }
    Ok(staged)
}

/// Writes `MANIFEST.sha256` over every artifact in `dir` (sha256sum
/// format, so `sha256sum -c` verifies the directory).
fn write_manifest(dir: &Path, artifacts: &[PathBuf]) -> Result<String, String> {
    let mut lines = vec![
        format!(
            "# FireCraft {} manifest — generated by `xtask dist`.",
            dir.file_name().and_then(|n| n.to_str()).unwrap_or("dist")
        ),
        "# Verify: sha256sum -c MANIFEST.sha256".to_owned(),
    ];
    for path in artifacts {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("unnameable artifact: {}", path.display()))?;
        let bytes = std::fs::read(path).map_err(|e| format!("read {name}: {e}"))?;
        lines.push(format!("{}  {name}", sha256_hex(&bytes)));
    }
    let mut text = lines.join("\n");
    text.push('\n');
    let manifest = dir.join("MANIFEST.sha256");
    std::fs::write(&manifest, &text).map_err(|e| format!("write manifest: {e}"))?;
    Ok("MANIFEST.sha256".to_owned())
}

/// Removes and recreates an empty `dir` (dist staging starts from
/// nothing every run).
fn clear_dir(dir: &Path) -> Result<(), String> {
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|e| format!("clear {}: {e}", dir.display()))?;
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))
}

/// The workspace version (`[workspace.package] version`), via cargo
/// metadata — the same number every artifact name and the deb control
/// file must carry.
pub(crate) fn workspace_version() -> Result<String, String> {
    let meta = crate::util::cargo_metadata()?;
    meta["packages"]
        .as_array()
        .and_then(|pkgs| {
            pkgs.iter()
                .find(|p| p["name"] == "xtask" && p["source"].is_null())
                .and_then(|p| p["version"].as_str())
        })
        .map(str::to_owned)
        .ok_or_else(|| "cargo metadata: no workspace-local xtask version".to_owned())
}

/// Path of a release binary from the shared build. Honors
/// `CARGO_TARGET_DIR` — the CI release job builds with an override
/// (`/tmp/ci-dist-target`), so the hardcoded workspace `target/` never
/// exists there.
fn release_bin(name: &str) -> Result<PathBuf, String> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root().join("target"));
    let path = target.join("release").join(name);
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("release binary missing: {}", path.display()))
    }
}

/// Copies `files` (archive-relative path → source) under
/// `stage/<top>/`, adds the license files, and tars+gzips the single
/// top-level directory into `out` (sorted names, zeroed mtimes and
/// owners — the archive is a function of the tree, not of the clock).
fn stage_archive(
    stage: &Path,
    top: &str,
    files: &[(&str, &Path)],
    out: &Path,
) -> Result<PathBuf, String> {
    let root = stage.join(top);
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).map_err(|e| format!("mkdir {}: {e}", root.display()))?;
    let licenses = root.join("share/licenses");
    std::fs::create_dir_all(&licenses).map_err(|e| format!("mkdir {licenses:?}: {e}"))?;
    std::fs::copy(workspace_root().join("LICENSE"), licenses.join("LICENSE"))
        .map_err(|e| format!("copy LICENSE: {e}"))?;
    for (rel, source) in files {
        let target = root.join(rel);
        std::fs::copy(source, &target).map_err(|e| format!("stage {rel}: {e}"))?;
    }
    normalize_tree(&root, "bin")?;
    tar_gz(stage, top, out)?;
    Ok(out.to_owned())
}

/// Normalizes a staged tree's modes before it is archived: directories
/// 0755, files 0644 — except executables under `exec_prefix`, which get
/// 0755. The NAS mounts every directory setgid, so an unstaged tree
/// would carry 2775 dirs and 0664 files into otherwise
/// deterministic archives (and dpkg-deb refuses setgid outright).
fn normalize_tree(root: &Path, exec_prefix: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt as _;
    let set = |path: &Path, mode: u32| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
            .map_err(|e| format!("chmod {}: {e}", path.display()))
    };
    let mut stack = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        set(&dir, 0o755)?;
        for entry in std::fs::read_dir(&dir).map_err(|e| format!("readdir {dir:?}: {e}"))? {
            let entry = entry.map_err(|e| format!("readdir {dir:?}: {e}"))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let exec = entry
                    .path()
                    .strip_prefix(root)
                    .map(|rel| rel.starts_with(exec_prefix))
                    .unwrap_or(false);
                set(&path, if exec { 0o755 } else { 0o644 })?;
            }
        }
    }
    Ok(())
}

/// GNU tar | gzip -n into `out`. Deterministic by construction:
/// `--sort=name --mtime=@0 --owner=0 --group=0` on the tar side,
/// `-n` (no timestamp in the gzip header) on the compression side.
fn tar_gz(stage: &Path, top: &str, out: &Path) -> Result<(), String> {
    let file = std::fs::File::create(out).map_err(|e| format!("create {}: {e}", out.display()))?;
    let mut tar = std::process::Command::new("tar")
        .args([
            "--sort=name",
            "--mtime=@0",
            "--owner=0",
            "--group=0",
            "--numeric-owner",
            "-cf",
            "-",
            "--",
            top,
        ])
        .current_dir(stage)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn tar: {e}"))?;
    let mut gzip = std::process::Command::new("gzip")
        .args(["-n", "-c"])
        .stdin(tar.stdout.take().ok_or_else(|| "tar stdout".to_owned())?)
        .stdout(std::process::Stdio::from(file))
        .spawn()
        .map_err(|e| format!("spawn gzip: {e}"))?;
    let tar_ok = tar.wait().map_err(|e| format!("wait tar: {e}"))?;
    let gz_ok = gzip.wait().map_err(|e| format!("wait gzip: {e}"))?;
    if tar_ok.success() && gz_ok.success() {
        Ok(())
    } else {
        Err(format!(
            "tar({tar_ok})/gzip({gz_ok}) failed building {}",
            out.display()
        ))
    }
}

/// A GLIBC symbol-version requirement, e.g. `2.34` or the historic
/// three-component `2.2.5`.
///
/// Compared component-wise, not as a float: `2.9` is older than
/// `2.10`, and a float comparison of the two would order them
/// backwards. A missing component is zero, so `2.2` is `2.2.0`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct GlibcVersion(Vec<u64>);

impl GlibcVersion {
    /// Parses a dotted version. `None` for anything that is not a
    /// purely numeric dotted version (`GLIBC_PRIVATE`, an empty
    /// column, a truncated string).
    fn parse(text: &str) -> Option<Self> {
        let mut parts = Vec::new();
        for part in text.split('.') {
            if part.is_empty() {
                return None;
            }
            parts.push(part.parse::<u64>().ok()?);
        }
        if parts.is_empty() {
            return None;
        }
        Some(Self(parts))
    }
}

impl std::fmt::Display for GlibcVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rendered: Vec<String> = self.0.iter().map(|n| n.to_string()).collect();
        write!(f, "{}", rendered.join("."))
    }
}

/// Reads the declared `libc6 (>= X)` floor out of a control file.
fn declared_libc_floor(control: &str) -> Result<GlibcVersion, String> {
    let depends = control
        .lines()
        .find(|l| l.starts_with("Depends:"))
        .ok_or_else(|| "packaging/deb/control: no Depends field".to_owned())?;
    let raw = depends
        .split("libc6 (>= ")
        .nth(1)
        .and_then(|s| s.split(')').next())
        .map(str::trim)
        .ok_or_else(|| "packaging/deb/control: no parsable libc6 floor in Depends".to_owned())?;
    GlibcVersion::parse(raw).ok_or_else(|| {
        format!("packaging/deb/control: libc6 floor {raw:?} is not a dotted version")
    })
}

/// The highest GLIBC symbol version referenced by `objdump -T` output.
///
/// Modern binutils prints the version in parentheses — `(GLIBC_2.34)`
/// — while older binutils printed it bare (`GLIBC_2.2.5`). Both forms
/// are accepted here; `GLIBC_PRIVATE` carries no release meaning and
/// is ignored.
///
/// Returns `None` when the output holds no versioned GLIBC reference
/// at all: a statically linked binary has no libc dependency, so
/// there is nothing for the floor to cover.
fn max_glibc_requirement(objdump_stdout: &str) -> Option<GlibcVersion> {
    let mut best: Option<GlibcVersion> = None;
    let mut rest = objdump_stdout;
    while let Some(idx) = rest.find("GLIBC_") {
        // Each iteration consumes the "GLIBC_" prefix, so this always
        // makes progress even when no version follows it.
        let after = &rest[idx + "GLIBC_".len()..];
        let end = after
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(after.len());
        if let Some(found) = GlibcVersion::parse(&after[..end])
            && best.as_ref().is_none_or(|b| found > *b)
        {
            best = Some(found);
        }
        rest = &after[end..];
    }
    best
}

/// True when the `objdump -T` output describes a dynamic symbol
/// table. Without one there is no libc dependency to floor.
fn has_dynamic_symbols(objdump_stdout: &str) -> bool {
    objdump_stdout.contains("DYNAMIC SYMBOL TABLE")
}

/// Compares the declared floor against the binary's requirement.
fn check_glibc_floor(
    floor: &GlibcVersion,
    needed: &GlibcVersion,
    binary: &Path,
) -> Result<(), String> {
    if needed > floor {
        return Err(format!(
            "{binary:?} needs GLIBC_{needed} but packaging/deb/control floors libc6 at \
             {floor} — raise the Depends floor (or build on an older glibc host)"
        ));
    }
    Ok(())
}

/// Checks that the control file's `libc6 (>= X)` floor is not below
/// the binary's highest GLIBC symbol-version requirement. A floor
/// below the real requirement produces a package that installs on
/// older systems and then fails at exec time — the exact defect a
/// clean-container smoke exposed (floor said 2.31, binaries needed
/// 2.35, found 2026-11-06). Parsed with `objdump -T` (binutils), not
/// recalled from a table.
///
/// The requirement is set by the *build host's* glibc, not by Rust's
/// std: the same program built on Debian 12 (glibc 2.36) references
/// `pidfd_spawnp`/`pidfd_getpid` as weak undefined symbols with no
/// version tag, while a build on Ubuntu 24.04 (glibc 2.39) tags them
/// `(GLIBC_2.39)` and makes the package un-installable on Debian 12.
/// So the floor is only meaningful if the build host is pinned — see
/// `docs/decisions/0077-deb-glibc-floor.md`.
fn verify_deb_glibc_floor(control: &str, binary: &Path) -> Result<(), String> {
    let floor = declared_libc_floor(control)?;
    let out = std::process::Command::new("objdump")
        .args(["-T"])
        .arg(binary)
        .output()
        .map_err(|e| format!("spawn objdump on {binary:?}: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        // A statically linked binary has no dynamic symbol table at
        // all and therefore no libc dependency to floor.
        if stderr.contains("not a dynamic object") {
            return Ok(());
        }
        return Err(format!("objdump -T {binary:?} failed: {}", stderr.trim()));
    }
    match max_glibc_requirement(&stdout) {
        Some(needed) => check_glibc_floor(&floor, &needed, binary),
        None if has_dynamic_symbols(&stdout) => Err(format!(
            "{binary:?} has a dynamic symbol table but no GLIBC version could be parsed \
             from `objdump -T` — refusing to assume the floor covers it"
        )),
        // No dynamic symbol table: nothing to floor.
        None => Ok(()),
    }
}

/// Builds the `.deb` from the pinned control file
/// (`packaging/deb/control`, whose Version must equal the workspace
/// version) over the standard payload: both binaries in `usr/bin`,
/// the desktop entry and the application icon, and licenses in
/// `usr/share/doc/firecraft/copyright`.
fn build_deb(
    stage: &Path,
    version: &str,
    shell: &Path,
    cli: &Path,
    out: &Path,
) -> Result<PathBuf, String> {
    let control = std::fs::read_to_string(workspace_root().join("packaging/deb/control"))
        .map_err(|e| format!("read packaging/deb/control: {e}"))?;
    let pinned = control
        .lines()
        .find_map(|line| line.strip_prefix("Version: "))
        .ok_or_else(|| "packaging/deb/control: no Version field".to_owned())?;
    if pinned != version {
        return Err(format!(
            "packaging/deb/control pins Version {pinned} but the workspace is \
             {version} — bump the control file with the workspace version"
        ));
    }
    verify_deb_glibc_floor(&control, shell)?;
    verify_deb_glibc_floor(&control, cli)?;

    let deb = stage.join("deb");
    let debian = deb.join("DEBIAN");
    let bin = deb.join("usr/bin");
    let doc = deb.join("usr/share/doc/firecraft");
    for dir in [&deb, &debian, &bin, &doc] {
        std::fs::create_dir_all(dir).map_err(|e| format!("mkdir {dir:?}: {e}"))?;
    }
    std::fs::write(debian.join("control"), &control).map_err(|e| format!("write control: {e}"))?;
    for (name, source) in [("firecraft", shell), ("firecraft-cli", cli)] {
        let target = bin.join(name);
        std::fs::copy(source, &target).map_err(|e| format!("stage deb {name}: {e}"))?;
    }
    // Desktop integration. A package that ships only binaries installs
    // no launcher and no icon: the host's menu shows nothing and the
    // desktop file that names `firecraft` has nowhere to live. Both
    // files are in-tree originals (docs/ATTRIBUTION.md), so the
    // package carries them verbatim.
    //
    // The desktop entry is the single canonical file, shared with the
    // AppImage recipe (`packaging/desktop/firecraft.desktop`): two
    // copies would drift, and the drift would only show up as a
    // missing icon on one format.
    let apps = deb.join("usr/share/applications");
    let icons = deb.join("usr/share/icons/hicolor/256x256/apps");
    for dir in [&apps, &icons] {
        std::fs::create_dir_all(dir).map_err(|e| format!("mkdir {dir:?}: {e}"))?;
    }
    for (source, target) in [
        (
            workspace_root().join("packaging/desktop/firecraft.desktop"),
            apps.join("firecraft.desktop"),
        ),
        (
            workspace_root().join("assets/icons/hicolor/256x256/apps/firecraft.png"),
            icons.join("firecraft.png"),
        ),
    ] {
        std::fs::copy(&source, &target)
            .map_err(|e| format!("stage deb {}: {e}", source.display()))?;
    }
    let mut copyright = String::from(
        "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\n\
         Upstream-Name: FireCraft\n\
         Source: https://github.com/aliasfoxkde/FireCraft\n\n\
         FireCraft is licensed under the Apache License 2.0\n\
         (SPDX-License-Identifier: Apache-2.0).\n",
    );
    copyright.push_str("\n----- full license text -----\n");
    copyright.push_str(
        &std::fs::read_to_string(workspace_root().join("LICENSE"))
            .map_err(|e| format!("read LICENSE: {e}"))?,
    );
    std::fs::write(doc.join("copyright"), copyright)
        .map_err(|e| format!("write copyright: {e}"))?;
    // dpkg-deb refuses setgid on the control directory (and the NAS
    // mounts every directory setgid); the pass also keeps group-writable
    // residue out of the data payload.
    normalize_tree(&deb, "usr/bin")?;

    let status = std::process::Command::new("dpkg-deb")
        .args(["--build", "--root-owner-group"])
        .arg(&deb)
        .arg(out)
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .map_err(|e| format!("spawn dpkg-deb: {e}"))?;
    if !status.status.success() {
        return Err(format!(
            "dpkg-deb --build failed: {}",
            String::from_utf8_lossy(&status.stderr).trim()
        ));
    }
    Ok(out.to_owned())
}

/// The payload every `FireCraft` deb must carry. `verify_deb` reads the
/// built package back with `dpkg-deb --contents` and requires each of
/// these paths: the two binaries, the desktop entry, and the icon.
///
/// The desktop entry and the icon are part of the package contract,
/// not decoration. A deb that installs the binaries but not the
/// launcher produces a host with a `firecraft` on `$PATH` and nothing
/// in its menus — a package that passes `dpkg -i` and fails the user's
/// actual question ("how do I start this?").
const REQUIRED_DEB_PAYLOAD: &[&str] = &[
    "usr/bin/firecraft",
    "usr/bin/firecraft-cli",
    "usr/share/applications/firecraft.desktop",
    "usr/share/icons/hicolor/256x256/apps/firecraft.png",
];

/// The stopping condition's read-back: `dpkg-deb --info` must parse
/// the package (name + version fields present) and `--contents` must
/// list the whole required payload.
fn verify_deb(deb: &Path, version: &str) -> Result<(), String> {
    let info = std::process::Command::new("dpkg-deb")
        .args(["--info"])
        .arg(deb)
        .output()
        .map_err(|e| format!("spawn dpkg-deb --info: {e}"))?;
    let text = String::from_utf8_lossy(&info.stdout);
    if !info.status.success()
        || !text.contains("Package: firecraft")
        || !text.contains(&format!("Version: {version}"))
    {
        return Err(format!(
            "dpkg-deb --info did not read {} as firecraft {version}:\n{}",
            deb.display(),
            text.trim()
        ));
    }
    let contents = std::process::Command::new("dpkg-deb")
        .args(["--contents"])
        .arg(deb)
        .output()
        .map_err(|e| format!("spawn dpkg-deb --contents: {e}"))?;
    let listing = String::from_utf8_lossy(&contents.stdout);
    if !contents.status.success() {
        return Err(format!(
            "dpkg-deb --contents failed for {}:\n{}",
            deb.display(),
            listing.trim()
        ));
    }
    let missing: Vec<&str> = REQUIRED_DEB_PAYLOAD
        .iter()
        .copied()
        .filter(|path| !listing.contains(path))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "dpkg-deb --contents: required payload missing from {}: {}\n{}",
            deb.display(),
            missing.join(", "),
            listing.trim()
        ));
    }
    Ok(())
}

/// Parse-checks the checked-in packaging recipes (`bash -n`), so a
/// broken recipe fails dist instead of the operator's release run.
/// The PowerShell and Inno Setup recipes cannot be parsed on this
/// fleet (no pwsh, no ISCC) — their parse proof happens on the target
/// platform (docs/TOOLCHAINS.md); the gate requires they exist.
fn parse_recipes() -> Result<usize, String> {
    let mut checked = 0;
    for name in [
        "build-appimage.sh",
        "build-flatpak.sh",
        "build-macos-dmg.sh",
        "deb/verify-install.sh",
    ] {
        let script = workspace_root().join("packaging").join(name);
        let status = std::process::Command::new("bash")
            .args(["-n"])
            .arg(&script)
            .output()
            .map_err(|e| format!("spawn bash -n: {e}"))?;
        if !status.status.success() {
            return Err(format!(
                "recipe {name} does not parse:\n{}",
                String::from_utf8_lossy(&status.stderr).trim()
            ));
        }
        checked += 1;
    }
    // The Flatpak manifest is data, not a script: a YAML parse proves
    // structure. The serde_yaml dependency lives in xtask's Cargo.toml.
    let manifest = workspace_root().join("packaging/flatpak/dev.firecraft.FireCraft.yml");
    let yaml = std::fs::read_to_string(&manifest)
        .map_err(|e| format!("read {}: {e}", manifest.display()))?;
    serde_yaml::from_str::<serde_yaml::Value>(&yaml).map_err(|e| {
        format!("packaging/flatpak/dev.firecraft.FireCraft.yml does not parse: {e}")
    })?;
    checked += 1;
    for name in ["build-windows.ps1", "windows/firecraft.iss"] {
        let recipe = workspace_root().join("packaging").join(name);
        if !recipe.is_file() {
            return Err(format!("packaging recipe missing: {name}"));
        }
    }
    Ok(checked)
}

/// The wasm size budget from the parity table, enforced on the
/// staged bytes. Returns the wasm size for the report line.
fn enforce_wasm_budget(staged: &[PathBuf]) -> Result<u64, String> {
    for path in staged {
        if path.extension().is_some_and(|ext| ext == "wasm") {
            let size = std::fs::metadata(path)
                .map_err(|e| format!("stat {}: {e}", path.display()))?
                .len();
            if wasm_size_warning(size)? {
                eprintln!(
                    "[xtask] warning: wasm bundle {} is {size} B, above the {WASM_SIZE_WARNING} B investigation threshold",
                    path.display()
                );
            }
            return Ok(size);
        }
    }
    Err("no .wasm file in the staged bundle".to_owned())
}

/// Returns whether the bundle crossed the investigation threshold.
/// Fails closed only above the hard policy ceiling.
fn wasm_size_warning(size: u64) -> Result<bool, String> {
    if size > WASM_SIZE_CEILING {
        return Err(format!(
            "wasm bundle is {size} B, above the hard {WASM_SIZE_CEILING} B ceiling; optimize before requesting a policy change (see docs/decisions/0090-wasm-bundle-size-policy.md)"
        ));
    }
    Ok(size > WASM_SIZE_WARNING)
}

/// The bridge smoke: serve the staged bundle over loopback http
/// (webassembly needs real MIME types; file:// would not do), drive
/// the round trip from the WEBOS.md host page, and require the
/// page's `green` verdict in the DOM headless chromium dumps.
fn smoke_bridge(staged: &[PathBuf]) -> Result<(), String> {
    let browser = chromium_path().ok_or_else(|| {
        "bridge smoke: no chromium on PATH (chromium|chromium-browser|google-chrome*)".to_owned()
    })?;
    let fixture = std::fs::read(workspace_root().join("corpus/fixtures/composed.fw.png"))
        .map_err(|e| format!("read smoke fixture: {e}"))?;
    let smoke = smoke_page(&fixture)?;
    let server = serve(staged, &smoke)?;

    // One virtual-time budget covers bundle load + wasm init + the
    // four-hop round trip; the verdict lands in the dumped DOM.
    let url = format!("http://{}/__smoke", server.addr);
    let output = spawned_with_timeout(
        browser.to_string_lossy().as_ref(),
        &[
            "--headless=new",
            "--no-sandbox",
            "--enable-unsafe-swiftshader",
            "--virtual-time-budget=60000",
            "--dump-dom",
            &url,
        ],
    )?;
    let dom = strip_script_bodies(&String::from_utf8_lossy(&output.stdout));
    // The verdict must appear OUTSIDE the page's own script source:
    // before this check stripped scripts, the literal verdict string
    // in the host page's JavaScript matched an app that had actually
    // panicked at boot, and the smoke reported green on a dead bridge
    // (shipped broken through v0.2.0 — found 2026-10-15).
    if dom.contains("FireCraft host: green") && !dom.contains("FireCraft host: FAIL") {
        return Ok(());
    }
    // The dumped DOM carries the page's verdict — a FAIL title or the
    // still-running default — so show it, not just the browser noise.
    Err(format!(
        "bridge smoke did not pass (no green verdict in the DOM); page title region:\n{}",
        dom.lines()
            .filter(|line| line.contains("<title>") || line.contains("FireCraft"))
            .take(4)
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

/// Drops every `<script>…</script>` body from a dumped DOM, so
/// verdict checks match runtime output, not the page's own source.
/// Unterminated script (a dump cut off mid-pause) drops the tail.
fn strip_script_bodies(dom: &str) -> String {
    let mut cleaned = String::with_capacity(dom.len());
    let mut rest = dom;
    while let Some(start) = rest.find("<script") {
        cleaned.push_str(&rest[..start]);
        let Some(after) = rest[start..].find("</script>") else {
            return cleaned;
        };
        rest = &rest[start + after + "</script>".len()..];
    }
    cleaned.push_str(rest);
    cleaned
}

/// The smoke's host page: the worked page from `docs/WEBOS.md`,
/// extracted **verbatim** between the begin/end markers — the guide is
/// load-bearing, so the page the doc teaches is the page the smoke
/// drives, byte for byte. The `__FIXTURE_B64__` stand-in (documented
/// in the guide) is the only substitution: the smoke inlines the
/// composed corpus fixture there.
fn smoke_page(fixture: &[u8]) -> Result<String, String> {
    let doc = std::fs::read_to_string(workspace_root().join("docs/WEBOS.md"))
        .map_err(|e| format!("read docs/WEBOS.md: {e}"))?;
    const BEGIN: &str = "<!-- begin: smoke host page -->";
    const END: &str = "<!-- end: smoke host page -->";
    let start = doc
        .find(BEGIN)
        .ok_or_else(|| "docs/WEBOS.md: missing the smoke host page begin marker".to_owned())?
        + BEGIN.len();
    let end = doc[start..]
        .find(END)
        .ok_or_else(|| "docs/WEBOS.md: missing the smoke host page end marker".to_owned())?
        + start;
    let fenced = doc[start..end].trim();
    let page = fenced
        .strip_prefix("```html")
        .and_then(|rest| rest.strip_prefix('\n'))
        .and_then(|rest| rest.strip_suffix("```"))
        .map(str::trim_end)
        .ok_or_else(|| {
            "docs/WEBOS.md: the smoke host page must be one ```html fenced block".to_owned()
        })?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(fixture);
    Ok(page.replace("__FIXTURE_B64__", &b64))
}

/// Serves the staged bundle plus the smoke page on a loopback port.
/// `application/wasm` matters: instantiateStreaming refuses the bundle
/// otherwise, and the smoke exists to exercise the real load path.
fn serve(staged: &[PathBuf], smoke: &str) -> Result<Server, String> {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).map_err(|e| format!("bind loopback: {e}"))?;
    let addr = listener.local_addr().map_err(|e| format!("addr: {e}"))?;
    let mut routes: Vec<Route> = Vec::new();
    routes.push((
        "/__smoke".to_owned(),
        "text/html; charset=utf-8",
        smoke.as_bytes().to_vec(),
    ));
    for path in staged {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("unnameable staged file: {}", path.display()))?;
        let kind = match Path::new(name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
        {
            "wasm" => "application/wasm",
            "js" => "text/javascript",
            "html" => "text/html; charset=utf-8",
            _ => "application/octet-stream",
        };
        let bytes = std::fs::read(path).map_err(|e| format!("read {name}: {e}"))?;
        routes.push((format!("/{name}"), kind, bytes));
    }
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            respond(stream, &routes);
        }
    });
    Ok(Server { addr })
}

/// One served route: path, content type, body.
type Route = (String, &'static str, Vec<u8>);

struct Server {
    addr: std::net::SocketAddr,
}

/// Handles one request: parse the GET line, answer from `routes`.
/// The smoke never needs keep-alive, ranges, or methods beyond GET.
fn respond(mut stream: TcpStream, routes: &[Route]) {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return,
        }
        if head.len() > 8 * 1024 {
            return;
        }
    }
    let line = String::from_utf8_lossy(&head);
    let path = line
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default()
        .to_owned();
    let (status, kind, body) = match routes.iter().find(|(route, _, _)| *route == path) {
        Some((_, kind, body)) => ("200 OK", *kind, body),
        None => ("404 Not Found", "text/plain", &b"not found".to_vec()),
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(body);
}

/// Runs `browser` with `args`, killing it if it outlives
/// [`SMOKE_TIMEOUT`] (a hung WebGL init must not hang `dist`).
fn spawned_with_timeout(browser: &str, args: &[&str]) -> Result<std::process::Output, String> {
    let child = std::process::Command::new(browser)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn {browser}: {e}"))?;
    let (sender, receiver) = mpsc::channel();
    let waiter = std::thread::spawn(move || {
        let output = child.wait_with_output();
        let _ = sender.send(output);
    });
    match receiver.recv_timeout(SMOKE_TIMEOUT) {
        Ok(output) => {
            let _ = waiter.join();
            output.map_err(|e| format!("wait {browser}: {e}"))
        }
        Err(_) => Err(format!(
            "{browser} outlived the {}s smoke timeout",
            SMOKE_TIMEOUT.as_secs()
        )),
    }
}

/// Finds a chromium-family browser on PATH.
fn chromium_path() -> Option<PathBuf> {
    for name in [
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
    ] {
        if which(name) {
            return Some(PathBuf::from(name));
        }
    }
    None
}

/// PATH lookup for one name.
fn which(name: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
}

// ---------------------------------------------------------------------------
// Release manifest (T27-54)
// ---------------------------------------------------------------------------

/// Schema identifier of the release manifest. A consumer that does not
/// recognise this string must refuse the document rather than guess at
/// its shape.
pub const MANIFEST_SCHEMA: &str = "firecraft.release-manifest/1";

/// The manifest's file name inside the release output directory.
pub const MANIFEST_FILE_NAME: &str = "release-manifest.json";

/// Verification status of an artifact whose digest was corroborated by
/// an independently recorded checksum list.
const VERIFICATION_VERIFIED: &str = "verified";

/// Verification status of an artifact whose digest the generator
/// computed itself, with no recorded list covering it.
const VERIFICATION_COMPUTED: &str = "computed";

/// Target triple recorded for artifacts that are not platform-specific
/// (the source archive, the SBOM, the checksum lists).
const TARGET_ANY: &str = "any";

/// The recorded checksum lists the generator corroborates digests
/// against, as paths relative to the release output directory.
const CHECKSUM_LISTS: &[&str] = &[
    "RELEASE_SHA256.txt",
    "native/MANIFEST.sha256",
    "web/MANIFEST.sha256",
];

/// One published release asset.
struct ArtifactSpec {
    /// Canonical published name — the asset's basename, which is how a
    /// release names it.
    name: String,
    /// Path relative to the release output directory.
    rel_path: String,
    /// Package format.
    format: &'static str,
    /// Target triple the artifact was built for.
    target: &'static str,
}

/// The build-time facts a manifest records about the run that produced
/// it.
///
/// The environment, the git checkout and the clock are read once at the
/// edge — in [`manifest_context`] — and handed to the generator as
/// data. The generator is therefore a pure function of its inputs, and
/// the same inputs produce the same manifest bytes.
struct ManifestContext {
    /// Workspace version the artifacts carry.
    version: String,
    /// Release tag the artifacts were cut from — absent when the build
    /// was not made from a tagged commit.
    tag: Option<String>,
    /// Git revision the artifacts were built from.
    source_sha: String,
    /// Workflow run URL — absent outside CI.
    run_url: Option<String>,
    /// Workflow run id — absent outside CI.
    run_id: Option<String>,
    /// UTC build timestamp — the one field that is build-time data by
    /// definition.
    built_at: String,
}

/// The published release asset set for `version`.
///
/// This is a fixed table, not a scan of the output directory. The
/// release names every asset explicitly — the workflow names each file
/// rather than globbing, because a glob such as `firecraft-web-*.zip`
/// picks up older versions left in `dist/` — and a scan would let a
/// stale file into the manifest.
fn release_specs(version: &str) -> Vec<ArtifactSpec> {
    vec![
        ArtifactSpec {
            name: format!("firecraft-{version}-linux-x86_64.tar.gz"),
            rel_path: format!("native/firecraft-{version}-linux-x86_64.tar.gz"),
            format: "tar.gz",
            target: "x86_64-unknown-linux-gnu",
        },
        ArtifactSpec {
            name: format!("firecraft-cli-{version}-linux-x86_64.tar.gz"),
            rel_path: format!("native/firecraft-cli-{version}-linux-x86_64.tar.gz"),
            format: "tar.gz",
            target: "x86_64-unknown-linux-gnu",
        },
        ArtifactSpec {
            name: format!("firecraft_{version}_amd64.deb"),
            rel_path: format!("native/firecraft_{version}_amd64.deb"),
            format: "deb",
            target: "x86_64-unknown-linux-gnu",
        },
        ArtifactSpec {
            name: "MANIFEST.sha256".to_owned(),
            rel_path: "native/MANIFEST.sha256".to_owned(),
            format: "sha256",
            target: "x86_64-unknown-linux-gnu",
        },
        ArtifactSpec {
            name: format!("firecraft-web-{version}.zip"),
            rel_path: format!("firecraft-web-{version}.zip"),
            format: "zip",
            target: "wasm32-unknown-unknown",
        },
        ArtifactSpec {
            name: format!("FireCraft-{version}-source.tar.gz"),
            rel_path: format!("source/FireCraft-{version}-source.tar.gz"),
            format: "tar.gz",
            target: TARGET_ANY,
        },
        ArtifactSpec {
            name: format!("firecraft-{version}-sbom.cdx.json"),
            rel_path: format!("sbom/firecraft-{version}-sbom.cdx.json"),
            format: "json",
            target: TARGET_ANY,
        },
        ArtifactSpec {
            name: "RELEASE_SHA256.txt".to_owned(),
            rel_path: "RELEASE_SHA256.txt".to_owned(),
            format: "sha256",
            target: TARGET_ANY,
        },
    ]
}

/// Parses a `sha256sum`-format list into `file name -> digest`.
///
/// The format is one `<digest>  <name>` line per file; `#` starts a
/// comment. A line that does not parse is an error, never a silently
/// skipped entry: a release whose checksum list is broken must not
/// produce a manifest that claims its assets are verified.
fn parse_checksum_list(text: &str, list_name: &str) -> Result<BTreeMap<String, String>, String> {
    let mut entries = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let digest = fields
            .next()
            .ok_or_else(|| format!("{list_name}:{}: empty entry", index + 1))?;
        let name = fields
            .next()
            .ok_or_else(|| format!("{list_name}:{}: no file name after the digest", index + 1))?;
        if digest.len() != 64 || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "{list_name}:{}: not a SHA-256 digest: {digest}",
                index + 1
            ));
        }
        if fields.next().is_some() {
            return Err(format!(
                "{list_name}:{}: unexpected text after the file name",
                index + 1
            ));
        }
        // `sha256sum` marks binary-mode entries with a leading `*`.
        let name = name.strip_prefix('*').unwrap_or(name);
        entries.insert(name.to_owned(), digest.to_ascii_lowercase());
    }
    Ok(entries)
}

/// A recorded checksum list: its path relative to the release output
/// directory, and the digests it records (file name -> digest).
type ChecksumList = (String, BTreeMap<String, String>);

/// Loads the recorded checksum lists that exist in the release output
/// directory.
///
/// A list that exists but does not parse fails the run: the manifest
/// must not describe a release whose own checksum list is broken.
fn load_checksum_lists(root: &Path) -> Result<Vec<ChecksumList>, String> {
    let mut lists = Vec::new();
    for rel in CHECKSUM_LISTS {
        let path = root.join(rel);
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("read {rel}: {e}"))?;
        lists.push(((*rel).to_owned(), parse_checksum_list(&text, rel)?));
    }
    Ok(lists)
}

/// The SHA-256 digest and byte size of a file.
///
/// The digest comes from [`sha256_hex_file`], the same helper the
/// release manifests themselves use, so the manifest records the same
/// digest form as the rest of the release path.
fn digest_and_size(path: &Path) -> Result<(String, u64), String> {
    let digest = sha256_hex_file(path)?;
    let size = std::fs::metadata(path)
        .map_err(|e| format!("stat {}: {e}", path.display()))?
        .len();
    Ok((digest, size))
}

/// Builds the manifest document for a release tree.
///
/// Fail-closed by design: a missing artifact, an empty artifact, a
/// checksum list that does not parse, or a file whose digest disagrees
/// with a recorded list all fail the run. A manifest that claims a
/// release the build cannot prove is worse than no manifest at all.
fn build_manifest(
    root: &Path,
    specs: &[ArtifactSpec],
    ctx: &ManifestContext,
) -> Result<serde_json::Value, String> {
    let lists = load_checksum_lists(root)?;
    let mut entries: Vec<(String, serde_json::Value)> = Vec::with_capacity(specs.len());
    for spec in specs {
        let path = root.join(&spec.rel_path);
        if !path.is_file() {
            return Err(format!("release artifact missing: {}", spec.name));
        }
        let (digest, size) = digest_and_size(&path)?;
        if size == 0 {
            return Err(format!("release artifact is empty: {}", spec.name));
        }
        let mut verification = VERIFICATION_COMPUTED;
        for (list_name, recorded) in &lists {
            if let Some(expected) = recorded.get(&spec.name) {
                if expected != &digest {
                    return Err(format!(
                        "digest mismatch for {}: {list_name} records {expected}, \
                         the file hashes to {digest}",
                        spec.name
                    ));
                }
                verification = VERIFICATION_VERIFIED;
            }
        }
        let value = serde_json::json!({
            "format": spec.format,
            "name": spec.name,
            "path": spec.rel_path,
            "sha256": digest,
            "size": size,
            "source_sha": ctx.source_sha,
            "target": spec.target,
            "verification": verification,
        });
        entries.push((spec.name.clone(), value));
    }
    // The artifact array is sorted by name, so the document does not
    // depend on the order the table happens to be written in.
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let artifacts: Vec<serde_json::Value> = entries.into_iter().map(|(_, value)| value).collect();
    Ok(serde_json::json!({
        "artifacts": artifacts,
        "built_at": ctx.built_at,
        "run_id": ctx.run_id,
        "run_url": ctx.run_url,
        "schema": MANIFEST_SCHEMA,
        "source_sha": ctx.source_sha,
        "tag": ctx.tag,
        "version": ctx.version,
    }))
}

/// Renders the manifest document: pretty-printed JSON with a trailing
/// newline.
///
/// Object keys are sorted and the artifact array is sorted by name, so
/// the same inputs always produce the same bytes.
fn render_manifest(value: &serde_json::Value) -> Result<String, String> {
    let mut text =
        serde_json::to_string_pretty(value).map_err(|e| format!("render the manifest: {e}"))?;
    text.push('\n');
    Ok(text)
}

/// Reads the build-time facts from the environment, git and the clock.
///
/// This is the edge — the only place that touches process-global state.
/// Everything downstream takes the values as data, so the generator can
/// be exercised in a test without touching the environment.
fn manifest_context(version: &str) -> Result<ManifestContext, String> {
    let root = workspace_root();
    let source_sha = git_value(&root, &["rev-parse", "HEAD"])?.ok_or_else(|| {
        "could not read the source revision: `git rev-parse HEAD` failed".to_owned()
    })?;
    // A git tag on the commit is the authoritative tag. The CI ref
    // name is the fallback for a checkout that did not fetch tags; it
    // only counts when it names this version, because on a branch
    // build the ref name is a branch, not a release.
    let git_tag = git_value(&root, &["describe", "--tags", "--exact-match", "HEAD"])?;
    if let Some(tag) = &git_tag {
        let tagged = tag.strip_prefix('v').unwrap_or(tag.as_str());
        if tagged != version {
            return Err(format!(
                "release tag {tag} does not carry the workspace version {version} — \
                 the tree and the tag disagree about what is being released"
            ));
        }
    }
    let tag = git_tag.or_else(|| {
        env_value("GITHUB_REF_NAME")
            .filter(|name| name.strip_prefix('v').unwrap_or(name.as_str()) == version)
    });
    Ok(ManifestContext {
        version: version.to_owned(),
        tag,
        source_sha,
        run_url: env_value("GITHUB_RUN_URL"),
        run_id: env_value("GITHUB_RUN_ID"),
        built_at: build_timestamp(),
    })
}

/// Runs a git command in `root` and returns its trimmed stdout.
///
/// `None` means the command produced no output — a commit that carries
/// no tag, for instance. A git that cannot be spawned at all is an
/// error: the manifest's source revision cannot be invented.
fn git_value(root: &Path, args: &[&str]) -> Result<Option<String>, String> {
    let (stdout, _stderr) = crate::util::capture_in(Some(root), "git", args)?;
    let trimmed = stdout.trim();
    Ok(if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    })
}

/// A non-empty environment value, or `None`.
fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Writes the machine-readable release manifest next to the artifacts
/// (T27-54).
///
/// The manifest describes the complete published asset set: every
/// artifact's canonical name, path relative to the output directory,
/// byte size, SHA-256 digest, package format, target triple and
/// verification status — plus the version, tag, source revision, run
/// URL and build timestamp of the run that produced it.
///
/// It is written into the release output directory, `dist/`, next to
/// the artifacts it describes. The build-time facts are read once, at
/// the edge, and passed in as data; the manifest bytes are therefore a
/// pure function of the artifact set and those facts.
pub fn manifest() -> Result<(), String> {
    let version = workspace_version()?;
    let out = workspace_root().join("dist");
    let ctx = manifest_context(&version)?;
    let specs = release_specs(&version);
    let value = build_manifest(&out, &specs, &ctx)?;
    let text = render_manifest(&value)?;
    let path = out.join(MANIFEST_FILE_NAME);
    std::fs::write(&path, &text).map_err(|e| format!("write {}: {e}", path.display()))?;
    eprintln!(
        "[xtask] dist manifest: {} artifact(s) -> {}",
        specs.len(),
        path.display()
    );
    Ok(())
}

/// Verifies `dist/release-manifest.json` against the files on disk.
///
/// Every artifact the manifest lists must exist with the recorded size
/// and digest, and every published artifact must be listed: a manifest
/// that omits an asset is as wrong as one that misdescribes it.
pub fn verify_manifest() -> Result<(), String> {
    let version = workspace_version()?;
    let out = workspace_root().join("dist");
    verify_manifest_at(&out, &release_specs(&version), &version)
}

/// The manifest verification, against a given release tree.
fn verify_manifest_at(root: &Path, specs: &[ArtifactSpec], version: &str) -> Result<(), String> {
    let path = root.join(MANIFEST_FILE_NAME);
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    // The document is read with `get`, never by indexing: a manifest
    // whose root is not an object must be rejected with an error, not
    // panic on the index.
    let schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if schema != MANIFEST_SCHEMA {
        return Err(format!(
            "{MANIFEST_FILE_NAME} carries an unknown schema: {schema}"
        ));
    }
    let described = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if described != version {
        return Err(format!(
            "{MANIFEST_FILE_NAME} describes version {described}, the workspace is {version}"
        ));
    }
    let artifacts = value
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("{MANIFEST_FILE_NAME} has no artifact array"))?;
    let mut listed: Vec<&str> = Vec::with_capacity(artifacts.len());
    for entry in artifacts {
        let name = entry
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{MANIFEST_FILE_NAME}: an artifact has no name"))?;
        let rel = entry
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{MANIFEST_FILE_NAME}: {name} has no path"))?;
        let expected_sha = entry
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{MANIFEST_FILE_NAME}: {name} has no sha256"))?;
        let expected_size = entry
            .get("size")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("{MANIFEST_FILE_NAME}: {name} has no size"))?;
        let file = root.join(rel);
        if !file.is_file() {
            return Err(format!("release artifact missing: {name}"));
        }
        let (digest, size) = digest_and_size(&file)?;
        if digest != expected_sha {
            return Err(format!(
                "digest mismatch for {name}: the manifest records {expected_sha}, \
                 the file hashes to {digest}"
            ));
        }
        if size != expected_size {
            return Err(format!(
                "size mismatch for {name}: the manifest records {expected_size} B, \
                 the file is {size} B"
            ));
        }
        listed.push(name);
    }
    for spec in specs {
        if !listed.contains(&spec.name.as_str()) {
            return Err(format!("the release manifest does not list {}", spec.name));
        }
    }
    for name in listed {
        if !specs.iter().any(|spec| spec.name == name) {
            return Err(format!(
                "the release manifest lists {name}, which is not a published artifact"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod sbom_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn wasm_size_policy_warns_at_25_mb_and_fails_above_50_mb() {
        assert_eq!(wasm_size_warning(25_000_000), Ok(false));
        assert_eq!(wasm_size_warning(25_000_001), Ok(true));
        assert_eq!(wasm_size_warning(50_000_000), Ok(true));
        assert!(wasm_size_warning(50_000_001).is_err());
    }

    #[test]
    fn registry_packages_get_cargo_purls_but_local_packages_do_not() {
        let registry = serde_json::json!({
            "id": "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0",
            "name": "serde",
            "version": "1.0.0",
            "source": "registry+https://github.com/rust-lang/crates.io-index",
        });
        assert_eq!(
            package_bom_ref(&registry).unwrap(),
            (
                "pkg:cargo/serde@1.0.0".to_owned(),
                Some("pkg:cargo/serde@1.0.0".to_owned())
            )
        );

        let local = serde_json::json!({
            "id": "path+file:///workspace/crates/engine#engine@0.1.0",
            "name": "engine",
            "version": "0.1.0",
            "source": null,
            "manifest_path": workspace_root().join("crates/engine/Cargo.toml").display().to_string(),
        });
        let (reference, purl) = package_bom_ref(&local).unwrap();
        assert!(reference.starts_with("urn:firecraft:cargo:engine@0.1.0:"));
        assert!(purl.is_none());
    }

    #[test]
    fn cyclonedx_dependencies_preserve_workspace_and_package_edges() {
        let workspace_id = "path+file:///workspace/crates/app#app@0.1.0";
        let registry_id = "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.0";
        let metadata = serde_json::json!({
            "workspace_members": [workspace_id],
            "resolve": { "nodes": [
                { "id": workspace_id, "deps": [{ "pkg": registry_id }] },
                { "id": registry_id, "deps": [] }
            ]}
        });
        let refs = BTreeMap::from([
            (workspace_id.to_owned(), "urn:firecraft:app".to_owned()),
            (registry_id.to_owned(), "pkg:cargo/serde@1.0.0".to_owned()),
        ]);
        let dependencies = cyclonedx_dependencies(
            &metadata,
            &refs,
            "urn:firecraft:application:firecraft@0.1.0",
        )
        .unwrap();
        let app = dependencies
            .iter()
            .find(|entry| entry["ref"] == "urn:firecraft:app")
            .unwrap();
        assert_eq!(app["dependsOn"][0], "pkg:cargo/serde@1.0.0");
        let root = dependencies.last().unwrap();
        assert_eq!(root["dependsOn"][0], "urn:firecraft:app");
    }
}

#[cfg(test)]
mod glibc_floor_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    /// A slice of real `objdump -T` output, in the parenthesized form
    /// modern binutils prints.
    const MODERN_OBJDUMP: &str = "\
/bin/ls:     file format elf64-x86-64

DYNAMIC SYMBOL TABLE:
0000000000000000      DF *UND*	0000000000000000 (GLIBC_2.3)  __ctype_toupper_loc
0000000000000000      DF *UND*	0000000000000000 (GLIBC_2.34) __libc_start_main
0000000000000000      w DF *UND*	0000000000000000 (GLIBC_2.34) _dl_find_dso_for_object
";

    /// The older bare form, including the historic three-component
    /// version and a `GLIBC_PRIVATE` marker that names no release.
    const LEGACY_OBJDUMP: &str = "\
DYNAMIC SYMBOL TABLE:
0000000000000000     DF *UND*	0000000000000000   GLIBC_2.2.5  getenv
0000000000000000     DF *UND*	0000000000000000   GLIBC_PRIVATE __libc_dlopen_mode
";

    const CONTROL: &str = "\
Package: firecraft
Version: 0.2.5
Depends: libc6 (>= 2.36)
";

    #[test]
    fn parenthesized_versions_are_parsed() {
        assert_eq!(
            max_glibc_requirement(MODERN_OBJDUMP),
            Some(GlibcVersion(vec![2, 34]))
        );
    }

    #[test]
    fn bare_versions_are_parsed() {
        assert_eq!(
            max_glibc_requirement(LEGACY_OBJDUMP),
            Some(GlibcVersion(vec![2, 2, 5]))
        );
    }

    #[test]
    fn glibc_private_is_not_a_requirement() {
        let only_private = "DYNAMIC SYMBOL TABLE:\n\
                            0000     DF *UND*  0000   GLIBC_PRIVATE __libc_dlopen_mode\n";
        assert_eq!(max_glibc_requirement(only_private), None);
    }

    #[test]
    fn three_component_versions_sort_below_their_next_minor() {
        // 2.2.5 is older than 2.10 — a float comparison would invert
        // this pair.
        let older = GlibcVersion(vec![2, 2, 5]);
        let newer = GlibcVersion(vec![2, 10]);
        assert!(older < newer);
        assert!(GlibcVersion(vec![2, 36]) > newer);
    }

    #[test]
    fn declared_floor_is_read_from_the_control_file() {
        assert_eq!(
            declared_libc_floor(CONTROL).unwrap(),
            GlibcVersion(vec![2, 36])
        );
    }

    #[test]
    fn a_control_file_without_a_libc6_floor_is_rejected() {
        let control = "Package: firecraft\nDepends: zlib1g\n";
        assert!(declared_libc_floor(control).is_err());
    }

    #[test]
    fn a_floor_below_the_requirement_is_rejected() {
        let floor = GlibcVersion(vec![2, 31]);
        let needed = GlibcVersion(vec![2, 39]);
        let err = check_glibc_floor(&floor, &needed, Path::new("/usr/bin/firecraft")).unwrap_err();
        assert!(err.contains("GLIBC_2.39"), "{err}");
        assert!(err.contains("2.31"), "{err}");
    }

    #[test]
    fn an_adequate_floor_is_accepted() {
        let floor = GlibcVersion(vec![2, 36]);
        let needed = GlibcVersion(vec![2, 35]);
        assert!(check_glibc_floor(&floor, &needed, Path::new("/usr/bin/firecraft")).is_ok());
    }

    #[test]
    fn the_check_fails_on_a_real_binary_with_a_too_low_floor() {
        // End-to-end through objdump. Skipped when objdump is absent —
        // the packaging path fails loudly in that case rather than
        // silently passing.
        if !std::path::Path::new("/usr/bin/objdump").exists() {
            return;
        }
        let binary = Path::new("/bin/ls");
        if !binary.exists() {
            return;
        }
        let low = "Package: firecraft\nDepends: libc6 (>= 2.2)\n";
        assert!(
            verify_deb_glibc_floor(low, binary).is_err(),
            "a floor of 2.2 cannot cover a modern glibc — the check must fail"
        );
        let high = "Package: firecraft\nDepends: libc6 (>= 99.0)\n";
        assert!(
            verify_deb_glibc_floor(high, binary).is_ok(),
            "an absurdly high floor must cover the real requirement"
        );
    }

    #[test]
    fn a_dynamic_binary_with_no_parsable_version_fails_loudly() {
        // No version column at all — the check must not assume the
        // floor covers a requirement it could not read.
        let opaque = "DYNAMIC SYMBOL TABLE:\n0000     DF *UND*  0000   ?   mystery_symbol\n";
        assert_eq!(max_glibc_requirement(opaque), None);
        assert!(has_dynamic_symbols(opaque));
    }
}
#[cfg(test)]
mod deb_desktop_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    /// The one canonical desktop entry, used by the deb payload and by
    /// the `AppImage` recipe.
    const DESKTOP_ENTRY: &str = "packaging/desktop/firecraft.desktop";

    /// The application icon, installed at the hicolor path the desktop
    /// entry's `Icon=` names.
    const ICON_ASSET: &str = "assets/icons/hicolor/256x256/apps/firecraft.png";

    fn desktop_entry() -> String {
        std::fs::read_to_string(workspace_root().join(DESKTOP_ENTRY)).unwrap()
    }

    fn desktop_field(entry: &str, key: &str) -> String {
        entry
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .unwrap_or_else(|| panic!("desktop entry has no {key} line"))
            .trim()
            .to_owned()
    }

    #[test]
    fn the_desktop_entry_exec_names_a_packaged_binary() {
        let entry = desktop_entry();
        let exec = desktop_field(&entry, "Exec=");
        let program = exec.split_whitespace().next().unwrap();
        let packaged = format!("usr/bin/{program}");
        assert!(
            REQUIRED_DEB_PAYLOAD.contains(&packaged.as_str()),
            "Exec={exec} names {program}, which the deb does not install"
        );
    }

    #[test]
    fn the_desktop_entry_icon_names_a_packaged_icon() {
        let entry = desktop_entry();
        let icon = desktop_field(&entry, "Icon=");
        // The icon theme resolves `Icon=firecraft` to a file named
        // `firecraft.png` under one of the hicolor size directories.
        let matches = REQUIRED_DEB_PAYLOAD
            .iter()
            .filter(|path| path.ends_with(&format!("/{icon}.png")))
            .count();
        assert_eq!(
            matches, 1,
            "Icon={icon} does not resolve to exactly one packaged icon: {REQUIRED_DEB_PAYLOAD:?}"
        );
    }

    #[test]
    fn the_icon_asset_is_in_tree() {
        assert!(
            workspace_root().join(ICON_ASSET).is_file(),
            "{ICON_ASSET} must exist — the deb payload copies it verbatim"
        );
    }

    #[test]
    fn the_required_payload_has_no_duplicate_paths() {
        let mut seen = std::collections::BTreeSet::new();
        for path in REQUIRED_DEB_PAYLOAD {
            assert!(
                seen.insert(*path),
                "duplicate required payload path: {path}"
            );
        }
    }

    #[test]
    fn the_desktop_entry_is_a_desktop_entry() {
        let entry = desktop_entry();
        assert_eq!(desktop_field(&entry, "Type="), "Application");
        assert!(!desktop_field(&entry, "Name=").is_empty());
        assert!(!desktop_field(&entry, "Exec=").is_empty());
    }
}

#[cfg(test)]
mod manifest_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::{
        ArtifactSpec, MANIFEST_FILE_NAME, MANIFEST_SCHEMA, ManifestContext, VERIFICATION_COMPUTED,
        VERIFICATION_VERIFIED, build_manifest, digest_and_size, parse_checksum_list, release_specs,
        render_manifest, verify_manifest_at,
    };
    use std::path::{Path, PathBuf};

    /// A fixture version that cannot collide with the workspace's own.
    const VERSION: &str = "9.9.9";

    /// A scratch release tree that cleans itself up.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "firecraft-release-manifest-{}-{tag}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Scratch(path)
        }

        fn root(&self) -> &Path {
            &self.0
        }

        fn write(&self, rel: &str, contents: &str) {
            let path = self.0.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, contents).unwrap();
        }

        fn digest(&self, rel: &str) -> String {
            digest_and_size(&self.0.join(rel)).unwrap().0
        }

        fn remove(&self, rel: &str) {
            std::fs::remove_file(self.0.join(rel)).unwrap();
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The build-time facts a fixture manifest carries.
    fn context() -> ManifestContext {
        ManifestContext {
            version: VERSION.to_owned(),
            tag: Some(format!("v{VERSION}")),
            source_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            run_url: Some("https://example.invalid/runs/1".to_owned()),
            run_id: Some("1".to_owned()),
            built_at: "2026-01-01T00:00:00Z".to_owned(),
        }
    }

    /// Builds a release tree with the published asset set and the two
    /// recorded checksum lists the release actually writes: the
    /// release-wide list covers every asset except itself, the native
    /// list covers the three native artifacts.
    fn release_tree(scratch: &Scratch) -> Vec<ArtifactSpec> {
        let specs = release_specs(VERSION);
        for spec in &specs {
            scratch.write(&spec.rel_path, &format!("contents of {}\n", spec.name));
        }
        // The native list covers the three native archives. It is
        // written before the release-wide list, because that list
        // records the native list's own digest.
        let native: Vec<String> = specs
            .iter()
            .filter(|spec| spec.rel_path.starts_with("native/") && spec.name != "MANIFEST.sha256")
            .map(|spec| format!("{}  {}", scratch.digest(&spec.rel_path), spec.name))
            .collect();
        scratch.write(
            "native/MANIFEST.sha256",
            &format!("{}\n", native.join("\n")),
        );
        // The release-wide list covers every asset except itself.
        let wide: Vec<String> = specs
            .iter()
            .filter(|spec| spec.name != "RELEASE_SHA256.txt")
            .map(|spec| format!("{}  {}", scratch.digest(&spec.rel_path), spec.name))
            .collect();
        scratch.write("RELEASE_SHA256.txt", &format!("{}\n", wide.join("\n")));
        specs
    }

    /// The manifest document for a fixture release.
    fn manifest_of(scratch: &Scratch, specs: &[ArtifactSpec]) -> serde_json::Value {
        build_manifest(scratch.root(), specs, &context()).unwrap()
    }

    #[test]
    fn the_manifest_describes_every_published_artifact() {
        let scratch = Scratch::new("shape");
        let specs = release_tree(&scratch);
        let value = manifest_of(&scratch, &specs);
        let artifacts = value["artifacts"].as_array().unwrap();
        assert_eq!(artifacts.len(), specs.len());
        for entry in artifacts {
            assert!(entry["name"].is_string(), "an artifact has no name");
            assert!(entry["path"].is_string(), "an artifact has no path");
            assert!(entry["sha256"].is_string(), "an artifact has no digest");
            assert!(entry["size"].is_u64(), "an artifact has no size");
            assert!(entry["format"].is_string(), "an artifact has no format");
            assert!(entry["target"].is_string(), "an artifact has no target");
            assert!(
                entry["verification"].is_string(),
                "an artifact has no verification status"
            );
            assert_eq!(
                entry["source_sha"].as_str().unwrap(),
                context().source_sha,
                "an artifact must record the revision it was built from"
            );
        }
        assert_eq!(value["schema"].as_str().unwrap(), MANIFEST_SCHEMA);
        assert_eq!(value["version"].as_str().unwrap(), VERSION);
        assert_eq!(value["tag"].as_str().unwrap(), format!("v{VERSION}"));
        assert_eq!(value["source_sha"].as_str().unwrap(), context().source_sha);
    }

    #[test]
    fn every_listed_artifact_matches_the_file_on_disk() {
        let scratch = Scratch::new("matches");
        let specs = release_tree(&scratch);
        let value = manifest_of(&scratch, &specs);
        for entry in value["artifacts"].as_array().unwrap() {
            let rel = entry["path"].as_str().unwrap();
            let (digest, size) = digest_and_size(&scratch.root().join(rel)).unwrap();
            assert_eq!(
                entry["sha256"].as_str().unwrap(),
                digest,
                "{rel} carries a digest that is not the file's"
            );
            assert_eq!(
                entry["size"].as_u64().unwrap(),
                size,
                "{rel} carries a size that is not the file's"
            );
        }
    }

    #[test]
    fn the_manifest_is_byte_identical_across_runs() {
        let scratch = Scratch::new("determinism");
        let specs = release_tree(&scratch);
        let first = render_manifest(&manifest_of(&scratch, &specs)).unwrap();
        let second = render_manifest(&manifest_of(&scratch, &specs)).unwrap();
        assert_eq!(
            first, second,
            "the same inputs must produce the same manifest bytes"
        );
        assert!(
            first.ends_with('\n'),
            "the manifest must end with a newline"
        );
    }

    #[test]
    fn the_manifest_keys_and_entries_are_sorted() {
        let scratch = Scratch::new("sorted");
        let specs = release_tree(&scratch);
        let value = manifest_of(&scratch, &specs);
        let text = render_manifest(&value).unwrap();
        let top: Vec<String> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::to_owned)
            .collect();
        let mut sorted = top.clone();
        sorted.sort();
        assert_eq!(top, sorted, "the top-level keys must be sorted");
        // The rendered document must carry the top-level keys in that
        // order too. The keys are read back from the rendered lines at
        // the top level's indentation — a plain substring search would
        // find a nested key of the same name first.
        let rendered: Vec<String> = text
            .lines()
            .filter_map(|line| line.strip_prefix("  \""))
            .filter_map(|line| line.split_once('"'))
            .map(|(key, _)| key.to_owned())
            .collect();
        assert_eq!(
            rendered, sorted,
            "the rendered keys must be in sorted order"
        );
        let names: Vec<&str> = value["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["name"].as_str().unwrap())
            .collect();
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(names, sorted_names, "the artifacts must be sorted by name");
    }

    #[test]
    fn an_artifact_corroborated_by_a_checksum_list_is_verified() {
        let scratch = Scratch::new("verified");
        let specs = release_tree(&scratch);
        let value = manifest_of(&scratch, &specs);
        for entry in value["artifacts"].as_array().unwrap() {
            let name = entry["name"].as_str().unwrap();
            let expected = if name == "RELEASE_SHA256.txt" {
                VERIFICATION_COMPUTED
            } else {
                VERIFICATION_VERIFIED
            };
            assert_eq!(
                entry["verification"].as_str().unwrap(),
                expected,
                "{name} carries the wrong verification status"
            );
        }
    }

    #[test]
    fn a_missing_artifact_is_rejected() {
        let scratch = Scratch::new("missing");
        let specs = release_tree(&scratch);
        scratch.remove(&specs[0].rel_path);
        let err = build_manifest(scratch.root(), &specs, &context()).unwrap_err();
        assert!(
            err.contains("missing"),
            "a missing artifact must be rejected: {err}"
        );
    }

    #[test]
    fn an_empty_artifact_is_rejected() {
        let scratch = Scratch::new("empty");
        let specs = release_tree(&scratch);
        scratch.write(&specs[0].rel_path, "");
        let err = build_manifest(scratch.root(), &specs, &context()).unwrap_err();
        assert!(
            err.contains("empty"),
            "an empty artifact must be rejected: {err}"
        );
    }

    #[test]
    fn a_digest_contradicting_the_checksum_list_is_rejected() {
        let scratch = Scratch::new("mismatch");
        let specs = release_tree(&scratch);
        // The file changed after the release-wide list was written.
        scratch.write(&specs[0].rel_path, "tampered\n");
        let err = build_manifest(scratch.root(), &specs, &context()).unwrap_err();
        assert!(
            err.contains("digest mismatch"),
            "a checksum-mismatched artifact must be rejected: {err}"
        );
    }

    #[test]
    fn a_checksum_list_that_cannot_be_parsed_is_rejected() {
        let scratch = Scratch::new("unparseable");
        let specs = release_tree(&scratch);
        scratch.write("RELEASE_SHA256.txt", "not a digest  firecraft.tar.gz\n");
        let err = build_manifest(scratch.root(), &specs, &context()).unwrap_err();
        assert!(
            err.contains("not a SHA-256 digest"),
            "a broken checksum list must be rejected: {err}"
        );
    }

    #[test]
    fn a_checksum_list_parses_comments_and_binary_marks() {
        let entries = parse_checksum_list(
            "# a comment\n\n0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  a.tar.gz\n\
             FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210 *b.tar.gz\n",
            "test.sha256",
        )
        .unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries.get("a.tar.gz").unwrap(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
        assert_eq!(
            entries.get("b.tar.gz").unwrap(),
            "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
        );
    }

    #[test]
    fn the_verifier_accepts_an_untouched_release() {
        let scratch = Scratch::new("verify-ok");
        let specs = release_tree(&scratch);
        let text = render_manifest(&manifest_of(&scratch, &specs)).unwrap();
        scratch.write(MANIFEST_FILE_NAME, &text);
        verify_manifest_at(scratch.root(), &specs, VERSION).unwrap();
    }

    #[test]
    fn the_verifier_rejects_a_corrupted_artifact() {
        let scratch = Scratch::new("verify-corrupt");
        let specs = release_tree(&scratch);
        let text = render_manifest(&manifest_of(&scratch, &specs)).unwrap();
        scratch.write(MANIFEST_FILE_NAME, &text);
        scratch.write(&specs[0].rel_path, "tampered\n");
        let err = verify_manifest_at(scratch.root(), &specs, VERSION).unwrap_err();
        assert!(
            err.contains("digest mismatch"),
            "a corrupted artifact must fail the verification: {err}"
        );
    }

    #[test]
    fn the_verifier_rejects_a_manifest_that_omits_an_artifact() {
        let scratch = Scratch::new("verify-omit");
        let specs = release_tree(&scratch);
        let mut value = manifest_of(&scratch, &specs);
        value
            .get_mut("artifacts")
            .and_then(serde_json::Value::as_array_mut)
            .unwrap()
            .pop();
        scratch.write(MANIFEST_FILE_NAME, &render_manifest(&value).unwrap());
        let err = verify_manifest_at(scratch.root(), &specs, VERSION).unwrap_err();
        assert!(
            err.contains("does not list"),
            "a manifest that omits an asset must fail the verification: {err}"
        );
    }

    #[test]
    fn the_verifier_rejects_a_manifest_that_is_not_an_object() {
        let scratch = Scratch::new("verify-shape");
        let specs = release_tree(&scratch);
        scratch.write(MANIFEST_FILE_NAME, "[]\n");
        let err = verify_manifest_at(scratch.root(), &specs, VERSION).unwrap_err();
        assert!(
            err.contains("unknown schema"),
            "a manifest that is not an object must fail the verification: {err}"
        );
    }
}
