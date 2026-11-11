# T23 — Broaden and verify platform release assets

**Objective:** prepare the next release to carry the same asset breadth
as photocraft v0.20.1
(https://github.com/storytold/photocraft/releases/tag/v0.20.1):
macOS (arm64 + x64, dmg/zip), Windows x64 (installer + portable zip),
Linux (deb, tarball, AppImage; Flatpak where feasible), plus the web
bundle.

**Inputs:** `xtask dist` recipes (native + web + deb already exist);
`docs/packaging/`; `docs/RELEASING.md`; photocraft's release page as
the breadth reference (names/sizes only — no code copying); CI runners
(GitForge, Fedora host) for cross-target builds.

**Authority:** build everything from this repo's source; cross-compile
or cross-docker for macOS/Windows per the existing toolchain docs
(`docs/TOOLCHAINS.md`).

**Stopping condition:** every planned artifact built from the next
release SHA, checksums recorded, and attached to that release; artifacts the
host cannot build carry a recorded blocker (e.g. macOS signing) rather
than a silent omission.

**Verification:** download each artifact, verify it launches (or note
the un-testable host), sizes match the build log; release page
complete.

**Status:** open. The asset set is broader than before; verify each
target-platform artifact and determine whether Flatpak is feasible.

**Current disposition:** recipes now exist for AppImage, macOS dmg, and
Windows packaging. The remaining work is target-platform build and
launch verification (plus Flatpak feasibility), not adding duplicate
recipes. This task is still open until those receipts exist.
