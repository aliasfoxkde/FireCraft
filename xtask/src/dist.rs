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
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// The release bundle's wasm size ceiling, in bytes. Pinned from
/// observation (trunk --release, wasm-opt 2, rustc 1.99): 10,873,780
/// bytes on 2026-10-12 — see `log/devlog.md`. Bundle size moves only
/// when the shipped code or its toolchain does, which is exactly what
/// this budget exists to surface; like every M12 ceiling it only
/// moves down.
const WASM_SIZE_CEILING: u64 = 14_000_000;

/// Wall-clock backstop for the headless browser: a hung WebGL init
/// kills the smoke instead of hanging `dist`.
const SMOKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Assembles the web bundle under `dist/web/`.
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
        "[xtask] dist web: {} file(s), wasm {wasm_bytes} B (ceiling {WASM_SIZE_CEILING}), manifest {manifest}, bridge smoke green",
        staged.len()
    );
    Ok(())
}

/// Assembles the native + CLI set under `dist/native/` (M12 task 5):
/// Linux x86_64 tar.gz of the desktop shell, a CLI-only archive, and a
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

/// Builds the release bundle with trunk and stages a fresh copy under
/// `dist/web/` — the staging directory starts empty every run, so a
/// dist artifact can never carry leftovers from an older tree.
/// WebOS packaging files (`appinfo.json`, `host.html`) from
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

    // Copy and stamp WebOS packaging files.
    let version = workspace_version()?;
    let webos_src = workspace_root().join("packaging/webos");
    for entry in
        std::fs::read_dir(&webos_src).map_err(|e| format!("read packaging/webos: {e}"))?
    {
        let entry = entry.map_err(|e| format!("readdir packaging/webos: {e}"))?;
        let source = entry.path();
        let name = entry.file_name();
        let target = out.join(&name);
        let contents = std::fs::read_to_string(&source)
            .map_err(|e| format!("read {}: {e}", name.to_string_lossy()))?;
        let stamped = contents.replace("@VERSION@", &version);
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

/// Path of a release binary from the shared build.
fn release_bin(name: &str) -> Result<PathBuf, String> {
    let path = workspace_root().join("target/release").join(name);
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
    for name in ["LICENSE-MIT", "LICENSE-APACHE"] {
        std::fs::copy(workspace_root().join(name), licenses.join(name))
            .map_err(|e| format!("copy {name}: {e}"))?;
    }
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
    for name in ["LICENSE-MIT", "LICENSE-APACHE"] {
        copyright.push_str(&format!("\n----- {name} -----\n"));
        copyright.push_str(
            &std::fs::read_to_string(workspace_root().join(name))
                .map_err(|e| format!("read {name}: {e}"))?,
        );
    }
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
    for name in ["build-appimage.sh", "build-macos-dmg.sh"] {
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
