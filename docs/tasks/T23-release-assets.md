# T23 — Broaden release assets to photocraft breadth

**Objective:** the v0.2.5 GitHub release carries the same asset breadth
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

**Stopping condition:** every planned artifact built from the release
SHA, attached to the GitHub release, checksums recorded; artifacts the
host cannot build carry a recorded blocker (e.g. macOS signing) rather
than a silent omission.

**Verification:** download each artifact, verify it launches (or note
the un-testable host), sizes match the build log; release page
complete.

**Status:** feeding T18. Audit `xtask/src/dist.rs` targets first; add
missing recipes (AppImage, win installer, macOS dmg) one per commit
with a build receipt each.
