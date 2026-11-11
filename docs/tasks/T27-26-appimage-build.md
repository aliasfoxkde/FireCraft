# T27-26 — Build an actual AppImage with linuxdeploy

> **RP:** RP-01 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-06

**Objective:** run the AppImage recipe to completion so a real `.AppImage` exists,
proving that the previously-identified blocker is actually cleared.

**Inputs:** `packaging/build-appimage.sh`; `packaging/appimage/firecraft.desktop`;
the in-tree icon at `assets/icons/hicolor/256x256/apps/firecraft.png`; `linuxdeploy`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1 (AppImage requirements);
`docs/TOOLCHAINS.md` (toolchain pinning for the operator build).

**Acceptance criteria:** a continuous linuxdeploy build — not an extract-and-run
substitute — produces a `.AppImage` file; the build log is captured; the recipe fails
closed on a missing desktop entry as it did before.

**Verification:** `packaging/build-appimage.sh && ls -l dist/firecraft-*-linux-x86_64.AppImage`

**Evidence:** the 2026-11-06 linuxdeploy run failed with "ERROR: Icon entry missing"
and produced no image; the fix landed as `a4d74a0` (in-tree original icon + `Icon=firecraft`
+ staging). A full linuxdeploy run has not been re-executed since.

**Blockers:** `linuxdeploy` is not available on the build host.

**Stopping condition:** an `.AppImage` exists, its size and SHA-256 are recorded, and
the build log shows a continuous linuxdeploy run.
