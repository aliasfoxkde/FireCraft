# T27-22 — Verify DEB reinstall/remove/purge and ship a desktop entry and icon

> **RP:** RP-01 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-21

**Objective:** complete the DEB's lifecycle story and give the package the desktop
integration a Linux desktop release is expected to have.

**Inputs:** `xtask/src/dist.rs` (`build_deb` staging list); `packaging/deb/control`;
`packaging/appimage/firecraft.desktop` (the existing desktop entry, for reuse); the
in-tree icon assets.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1 — the DEB requirements include
desktop-entry, icon and MIME validation.

**Acceptance criteria:** reinstall, remove and purge each succeed with no user data
lost; the package ships a `.desktop` entry with `Icon=firecraft` and the matching icon
files at the standard hicolor path; the desktop entry's `Exec` path matches the
installed binary.

**Verification:** `dpkg -l; dpkg -P; dpkg-deb --contents … | grep -E 'desktop|icons'`

**Evidence:** `build_deb` currently stages only `usr/bin/{firecraft,firecraft-cli}` and
`usr/share/doc/firecraft/copyright` — no `.desktop`, no icons, no MIME association.
The 2026-11-06 clean-container smoke exercised the CLI only, so it does not cover
desktop integration.

**Blockers:** none beyond T27-21.

**Stopping condition:** the package contents include the desktop entry and icons, and
the install → reinstall → remove → purge sequence passes with the outputs recorded.
