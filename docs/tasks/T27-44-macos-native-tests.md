# T27-44 — Test the macOS build natively on a supported host

> **RP:** RP-03 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-43

**Objective:** prove the macOS artifact works on a real Mac, including the case where
the user drags the app out of the mounted DMG.

**Inputs:** the `.app` and `.dmg` from T27-43; a supported macOS host; the release
receipt procedure.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §10 — every required native package
is installed, launched and functionally smoke-tested on its target OS.

**Acceptance criteria:** install, launch, open/create/edit/save/export, upgrade/replace
and uninstall all pass; the app still works after being moved outside the mounted DMG;
the results are recorded per macOS version.

**Verification:** the macOS release receipt.

**Evidence:** none yet.

**Blockers:** a supported macOS host.

**Stopping condition:** the receipt exists with per-version results.
