# FireCraft — Platform Artifact Status

> Built and verified on **Linux x86_64** only. Other platforms are
> off-fleet recipes — the release notes honestly state: *built and verified
> on Linux x86_64*. Never ship an untested artifact with an implied
> claim of the release fleet's verification.

| Platform | Recipe | Artifact | Last verified | Blocker |
|---|---|---|---|---|
| Linux x86_64 | `xtask dist --native` | `firecraft-X.Y.Z-linux-x86_64.tar.gz`, `firecraft_X.Y.Z_amd64.deb` | **2026-10-19** (v0.2.2) | None |
| Linux AppImage | `packaging/build-appimage.sh` | `.AppImage` | Never | `linuxdeploy-x86_64.AppImage` not on this host; FUSE required |
| macOS universal | `packaging/build-macos-dmg.sh` | `.dmg` | Never | No macOS SDK or Apple toolchain on this host |
| Windows | `packaging/build-windows.ps1` | `.zip` + `.exe` (Inno) | Never | No Windows toolchain; cross-compile not configured |

## Linux x86_64

Produced by `cargo run -p xtask -- dist --native`. Verified at every
release cycle. The deb is built with `dpkg-deb`. The tar.gz is GNU tar.

## Linux AppImage

Recipe: `packaging/build-appimage.sh`. Requires:
- `linuxdeploy-x86_64.AppImage` (download pin in `packaging/build-appimage.sh`)
- FUSE kernel support (or `--appimage-extract-and-run` for rootless)
- GNU tar on PATH

The script stages `dist/native/firecraft-X.Y.Z-linux-x86_64.tar.gz`
and wraps it with linuxdeploy. Not verified on this host — linuxdeploy
is not present.

## macOS universal

Recipe: `packaging/build-macos-dmg.sh`. **Off-fleet.** Requires:
- macOS host (Intel or Apple Silicon)
- Xcode command-line tools
- `cargo install create-dmg` or equivalent

The repo carries the recipe; no CI runner has a macOS toolchain. The
operator who has a Mac owns the artifact and is responsible for verifying it.

## Windows

Recipe: `packaging/build-windows.ps1`. **Off-fleet.** Requires:
- Windows host (or Wine/CrossOver)
- MSVC toolchain via `rustup target add x86_64-pc-windows-msvc`
- Visual Studio Build Tools (link.exe)
- PowerShell 5.1+ (or 7)

The repo carries the recipe; no CI runner has a Windows toolchain. The
operator who has a Windows machine owns the artifact and is responsible
for verifying it.

## Adding a new platform

1. Write the recipe in `packaging/`.
2. Add a header comment with the toolchain requirements.
3. Add a row to this table with "Never" as last-verified and the
   known blocker.
4. When verified, update the date and remove the "Never" note.
5. Never add a row for a platform whose recipe has never been executed.
