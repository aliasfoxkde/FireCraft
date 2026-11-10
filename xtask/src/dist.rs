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

use crate::util::{run_in, sha256_hex, workspace_root};
use base64::Engine as _;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// The release bundle's wasm size ceiling, in bytes. Pinned from
/// observation (trunk --release, wasm-opt 2, rustc 1.99, wasm-opt 132):
/// 21,235,599 B on 2026-11-03 via `xtask dist --web` — see
/// `log/devlog.md`. The 19.x shell batches (Lucide icons, command
/// palette, gradient editor, image support) grew the bundle from the
/// 15,245,342 B observed 2026-11-02; the doubling vs the M12 pin
/// (10,873,780 B) is deliberate feature surface, and the re-pin is the
/// dated 19b.4 observation, not a silent bump. Bundle size moves only
/// when the shipped code or its toolchain does, which is exactly what
/// this budget exists to surface.
const WASM_SIZE_CEILING: u64 = 21_500_000;

/// Wall-clock backstop for the headless browser: a hung WebGL init
/// kills the smoke instead of hanging `dist`.
const SMOKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Assembles the web bundle under `dist/web/`.
///
/// `FIRECRAFT_DIST_SKIP_SMOKE=1` skips the bridge smoke for environments
/// without a browser (the CI image has no chromium yet); every local
/// release run keeps the smoke — this is an explicit debt of the CI
/// image, not a release-option.
pub fn web() -> Result<(), String> {
    let staged = stage_web()?;
    let dir = staged
        .first()
        .and_then(|p| p.parent())
        .ok_or_else(|| "staging dir vanished".to_owned())?;
    let manifest = write_manifest(dir, &staged)?;
    let wasm_bytes = enforce_wasm_budget(&staged)?;
    let smoke_skipped = std::env::var("FIRECRAFT_DIST_SKIP_SMOKE").is_ok_and(|v| v == "1");
    if smoke_skipped {
        eprintln!(
            "[xtask] dist web: bridge smoke SKIPPED (FIRECRAFT_DIST_SKIP_SMOKE=1 — CI image debt: no chromium baked)"
        );
    } else {
        smoke_bridge(&staged)?;
    }
    eprintln!(
        "[xtask] dist web: {} file(s), wasm {wasm_bytes} B (ceiling {WASM_SIZE_CEILING}), manifest {manifest}, smoke {}",
        staged.len(),
        if smoke_skipped { "skipped" } else { "green" }
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
    run_in(Some(&web), "trunk", &["build", "--release"])?;

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
/// licenses in `usr/share/doc/firecraft/copyright`.
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
    let mut copyright = String::from(
        "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\n\
         Upstream-Name: FireCraft\n\
         Source: https://github.com/aliasfoxkde/FireCraft\n\n\
         FireCraft is dual-licensed under the MIT License and the Apache\n\
         License 2.0 (SPDX-License-Identifier: MIT OR Apache-2.0).\n",
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

/// The stopping condition's read-back: `dpkg-deb --info` must parse
/// the package (name + version fields present) and `--contents` must
/// list both binaries.
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
    if !contents.status.success()
        || !listing.contains("usr/bin/firecraft")
        || !listing.contains("usr/bin/firecraft-cli")
    {
        return Err(format!(
            "dpkg-deb --contents: payload binaries missing from {}:\n{}",
            deb.display(),
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
            if size > WASM_SIZE_CEILING {
                return Err(format!(
                    "wasm bundle {}: {size} B exceeds the {WASM_SIZE_CEILING} B ceiling — \
                     a size regression, or the budget needs re-pinning (ceilings only move down)",
                    path.display()
                ));
            }
            return Ok(size);
        }
    }
    Err("no .wasm file in the staged bundle".to_owned())
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

#[cfg(test)]
mod sbom_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

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
