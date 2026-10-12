# Releasing FireCraft

Everything below is one verb per artifact set, run from a clean tree.
Artifacts are local; the signing ceremony is the operator's. The repo
never carries an untested claim of a built artifact — platforms this
fleet cannot build ship as recipes under `packaging/`
(docs/TOOLCHAINS.md).

## Preconditions

- `cargo run -p xtask -- ci` — all gates green.
- Version stamp: `[workspace.package] version` in the root `Cargo.toml`
  **and** the literal `Version:` in `packaging/deb/control` move
  together. `xtask changelog` and `xtask dist --native` fail on drift.
- Every crate carries the workspace version; `xtask changelog`
  enforces one stamp across all workspace-local packages.

## Web bundle (WASM-first — the primary target)

    cargo run -p xtask -- dist --web

Produces `dist/web/`:

| artifact | note |
|---|---|
| `index.html`, one `.js`, one `.wasm` | trunk `--release` bundle |
| `MANIFEST.sha256` | over the bundle; `sha256sum -c` verifies |

The gate enforces the wasm size ceiling (parity table) and runs the
bridge smoke **on these release bytes**: the WEBOS.md host page loads
the bundle in headless chromium and completes one
open → apply → exportPng → exportDocument round trip through the M5
postMessage bridge. A dist run that cannot prove the bridge does not
claim the artifact.

## Linux native set

    cargo run -p xtask -- dist --native

Produces `dist/native/`:

| artifact | note |
|---|---|
| `firecraft-<v>-linux-x86_64.tar.gz` | desktop shell + CLI, `firecraft-<v>/{bin,share/licenses}` layout |
| `firecraft-cli-<v>-linux-x86_64.tar.gz` | CLI alone |
| `firecraft_<v>_amd64.deb` | built from the pinned `packaging/deb/control` |
| `MANIFEST.sha256` | over the three archives; `sha256sum -c` verifies |

The gate itself runs `dpkg-deb --info` and `--contents` on the deb
(name/version fields, both binaries present). Install test by hand:
`apt install ./firecraft_<v>_amd64.deb` or `dpkg -i`.

## Off-fleet platforms (recipes — the operator builds these)

- AppImage: `packaging/build-appimage.sh` (needs
  linuxdeploy-x86_64.AppImage; parse-checked by `dist --native`)
- macOS universal dmg: `packaging/build-macos-dmg.sh` (needs macOS +
  Xcode CLT; lipo universal + ad-hoc codesign + hdiutil)
- Windows portable zip: `packaging/build-windows.ps1`; optional Inno
  Setup installer from `packaging/windows/firecraft.iss`

Record the sha256 of every recipe-built artifact you ship in the
release notes — the dist manifests cover only what the fleet built.

## Release notes

    cargo run -p xtask -- changelog

Regenerates `docs/CHANGELOG.md` from the conventional-commit log
(grouped by type, git log order, every commit accounted for) and
verifies the license files against the dual claim and the version
stamps. Refresh and commit the copy at release time; CI runs the verb
(it re-verifies licenses/versions on the runner) but does not
byte-check the file — a changelog generated from the log cannot
contain its own tip commit.

## Checklist

1. `xtask ci` green.
2. Version stamped in `Cargo.toml` **and** `packaging/deb/control`.
3. `xtask dist --web` green — manifest verifies, size budget holds,
   bridge smoke green on the release bytes.
4. `xtask dist --native` green — manifest verifies, `dpkg-deb`
   read-back passes.
5. Off-fleet recipes executed on their platforms (only for those
   targets being released); hashes recorded.
6. `xtask changelog` refreshed and committed.
7. Tag `v<version>` and push both remotes
   (gitforge + origin); GitForge CI green on the tag.
