# T27-31 — Re-validate the RPM deferral against current demand

> **RP:** RP-01 · **Priority:** P2 · **Status:** READY
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-03

**Objective:** confirm that RPM remains the right call for the 0.2.x matrix, and either
retain the deferral or formally reinstate RPM with a spec and a recipe.

**Inputs:** `docs/decisions/0070-rpm-deferred.md`; `packaging/`; open issues
(`gh issue list`); the artifact matrix's RPM row.

**Authority:** `docs/decisions/0070-rpm-deferred.md` — RPM is not supported in the
0.2.x matrix; RP-01 scope is tar.gz / DEB / AppImage / Flatpak; and **RP-08 must not
list RPM as a skipped required target**.

**Acceptance criteria:** decision 0070 is re-checked against current demand; it is
either retained with a dated note, or formally superseded by a new decision that names
the spec, the recipe and the owner; the artifact matrix's RPM row reflects the
outcome.

**Verification:** `grep -n 'Status' docs/decisions/0070-rpm-deferred.md`

**Evidence:** `docs/decisions/0070-rpm-deferred.md`, accepted 2026-11-06. The matrix
records RPM as "unsupported (decision 0070)" with no artifact.

**Blockers:** none.

**Stopping condition:** the deferral is retained or formally superseded, and the
matrix and the certification report agree.
