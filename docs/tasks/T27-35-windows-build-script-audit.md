# T27-35 — Audit `build-windows.ps1` and the Inno Setup configuration

> **RP:** RP-02 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-04

**Objective:** read the Windows build script and the Inno Setup configuration line by
line and record their environment assumptions, product identity, shortcuts and version
metadata — the fleet cannot run them, so a written audit is the only inspection
available.

**Inputs:** `packaging/build-windows.ps1`; `packaging/windows/firecraft.iss`;
`docs/TOOLCHAINS.md`; the version stamps.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.2 (Windows requirements, including
the "Audit Windows build script environment assumptions" task named in the campaign).

**Acceptance criteria:** the script's environment assumptions are listed with their
risks; the installer's product identity, publisher, version, install scope and shortcut
set are documented; every value that must match a version stamp is identified.

**Verification:** `read packaging/build-windows.ps1 packaging/windows/firecraft.iss`

**Evidence:** neither file is parse-checkable on this fleet — there is no `pwsh` and no
`ISCC` — which is exactly why the audit is a written deliverable rather than a run.

**Blockers:** no Windows host.

**Stopping condition:** the audit is committed with a per-assumption risk list.
