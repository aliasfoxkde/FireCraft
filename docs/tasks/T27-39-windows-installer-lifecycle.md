# T27-39 — Verify installer upgrade, uninstall, reinstall and user-data preservation

> **RP:** RP-02 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-37

**Objective:** prove the Windows installer behaves correctly over its lifetime —
upgrading over a previous version, uninstalling, reinstalling — and that user data is
handled as documented.

**Inputs:** the installer from T27-37 and the previously published v0.2.5 build; a
clean Windows VM.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.2 (installer lifecycle) and §10.

**Acceptance criteria:** upgrade over the previous version succeeds, uninstall
succeeds, reinstall succeeds, and user data (settings, recent files, documents) is
preserved or removed exactly as the documentation states.

**Verification:** the Windows runner release receipt.

**Evidence:** none yet.

**Blockers:** a clean Windows VM.

**Stopping condition:** all four transitions are recorded with their outcomes and the
user-data behaviour is documented.
