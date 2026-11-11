# T27-27 — Inspect the built AppImage and launch it both ways

> **RP:** RP-01 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-26

**Objective:** verify the AppImage's contents are correct and that it starts under both
FUSE and the `--appimage-extract-and-run` fallback.

**Inputs:** the `.AppImage` from T27-26; `squashfs-tools` or `--appimage-extract` for
inspection; an X server or Xvfb for the launch.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1 — an AppImage must be
inspectable, installable-free, and launchable on a clean Linux host.

**Acceptance criteria:** the image carries a desktop entry with `Icon=firecraft`, the
icon file itself, executable bits on the binaries, and the bundled libraries it needs;
it launches under FUSE and again via `--appimage-extract-and-run`.

**Verification:** `./firecraft-*-linux-x86_64.AppImage --appimage-extract-and-run`

**Evidence:** none yet — the image has never been built since the icon fix.

**Blockers:** FUSE availability on the host.

**Stopping condition:** the inspection listing and both launch attempts are recorded.
