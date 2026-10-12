# Packaging recipes — toolchain requirements

`xtask dist` builds what the release fleet can build and verify:
`dist --web` (the static web bundle + bridge smoke) and `dist --native`
(the Linux x86_64 tar.gz / deb / CLI set, manifest-verified). Everything
else under `packaging/` is a **recipe** — a checked-in script the
operator runs on a host that has the toolchain.

**The no-untested-claims rule:** the repo never carries an artifact it
did not build, and never claims a platform is "supported" beyond what a
gate exercised. A recipe shipped in `packaging/` has been parse-checked
(`bash -n` for the shell scripts, by `xtask dist --native`); its
*output* is the operator's to build and verify on the target platform.

| Recipe | Target artifact | Toolchain | Runs on |
|---|---|---|---|
| `web/` (trunk) | `dist/web/` bundle | rustup wasm32-unknown-unknown, trunk, wasm-opt | release fleet (`xtask dist --web`) |
| `packaging/deb/control` | `firecraft_<v>_amd64.deb` | dpkg-deb | release fleet (`xtask dist --native`) |
| `packaging/build-appimage.sh` | `firecraft-<v>-linux-x86_64.AppImage` | linuxdeploy-x86_64.AppImage (FUSE or `--appimage-extract-and-run`), GNU coreutils | any Linux x86_64 host with linuxdeploy |
| `packaging/build-macos-dmg.sh` | `firecraft-<v>-macos-universal.dmg` | macOS + Xcode CLT, rustup targets aarch64/x86_64-apple-darwin, lipo, codesign, hdiutil | macOS only |
| `packaging/build-windows.ps1` + `packaging/windows/firecraft.iss` | portable zip + optional Inno Setup installer | Windows, PowerShell 5.1+/7, MSVC (rustup target x86_64-pc-windows-msvc + VS Build Tools), optional Inno Setup 6 | Windows only |

## Verification burden per artifact

- **web / tar.gz / deb / CLI archives** — verified by the dist gate
  itself (manifest `sha256sum -c`, `dpkg-deb --info`/`--contents`,
  bridge smoke on the release bytes).
- **AppImage, dmg, Windows zip/installer** — built and verified by the
  operator from these recipes; record the sha256 of what you ship in
  the release notes. The dist manifest covers only what the fleet
  built.
- **Parse status of the recipes**: `build-appimage.sh` and
  `build-macos-dmg.sh` are `bash -n`-checked by `xtask dist --native`
  on every run, so a broken recipe fails the gate. The PowerShell and
  Inno Setup recipes cannot be parsed on this fleet (no pwsh, no ISCC);
  their syntax is verified on the target platform by whoever first
  runs them — if you do, fold the parse check into the recipe run.

## Versioning

Every recipe derives its version from `[workspace.package] version` in
the root `Cargo.toml` — the same number `packaging/deb/control` pins
literally (the dist gate fails on drift between the two).
