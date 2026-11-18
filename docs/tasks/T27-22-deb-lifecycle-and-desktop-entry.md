# T27-22 — Verify DEB reinstall/remove/purge and ship a desktop entry and icon

> **RP:** RP-01 · **Priority:** P0 · **Status:** IMPLEMENTED
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

**Evidence:** The row's premise is fixed in the tree. `build_deb` stages the desktop entry and the icon from the in-tree originals — `usr/share/applications/firecraft.desktop` and `usr/share/icons/hicolor/256x256/apps/firecraft.png` (`xtask/src/dist.rs:973`) — and the gate reads the built package back with `dpkg-deb --contents` and fails closed when either is missing. **The exercise against the fixed package has not been run**: [`packaging/deb/verify-install.sh`](https://github.com/aliasfoxkde/FireCraft/blob/main/packaging/deb/verify-install.sh) carries the eight stages (install, desktop entry, icon, CLI, CLI document round trip, GUI launch, upgrade, uninstall, reinstall), and the recorded negative control is the *published* v0.2.3 package, which installs cleanly and then fails stage 2 — no desktop entry (audit §11.6). The defect is fixed. **The clean-container receipt against a package that carries the desktop entry now exists** (2026-11-17): the eight-stage run passes on Ubuntu 24.04.5 LTS and Debian 12 — stage 2 `PASS: desktop entry installs and names firecraft / firecraft`, `PASS: desktop-file-validate accepts the entry`; stage 3 `PASS: icon installed at /usr/share/icons/hicolor/256x256/apps/firecraft.png`; stage 7 `PASS: removal took the binaries, the desktop entry and the icon with it`; stage 8 `PASS: reinstall restored the payload and the CLI runs`. The row stays `IMPLEMENTED` rather than closing, because **purge is not separately exercised** — stage 7 is an uninstall, and no `dpkg -P` run is in the receipt. **Done:** 2026-11-17: the clean-container eight-stage receipt exists on Ubuntu 24.04 and Debian 12 for a package that carries the desktop entry and the icon.

**Blockers:** purge (`dpkg -P`) is not exercised by the eight-stage script; a purge receipt is still outstanding

**Stopping condition:** the package contents include the desktop entry and icons, and
the install → reinstall → remove → purge sequence passes with the outputs recorded.
