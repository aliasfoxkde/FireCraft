# T27-34 — Publish a Linux platform compatibility table

> **RP:** RP-01 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-33

**Objective:** publish the tested-support statement that users and the certification
report can both cite, including what was not tested.

**Inputs:** the results of T27-21, T27-22, T27-28, T27-30 and T27-33; the artifact
matrix.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §10 — release notes must state exact
platform and architecture support and residual limitations.

**Acceptance criteria:** a published table lists every tested distribution, version and
architecture for the DEB, tar.gz, AppImage and Flatpak, and explicitly lists every
untested combination; a reader can tell what is supported from what is merely built.

**Verification:** `test -f docs/release/LINUX-COMPAT.md`

**Evidence:** none yet.

**Blockers:** T27-33.

**Stopping condition:** the table exists, is linked from the artifact matrix, and its
"untested" column is non-empty where testing was not performed.
