# T27-34 — Publish a Linux platform compatibility table

> **RP:** RP-01 · **Priority:** P1 · **Status:** IMPLEMENTED
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

**Evidence:** [LINUX-COMPAT.md](LINUX-COMPAT.md) is published and linked from `docs/release/ARTIFACT-MATRIX.md`. It lists the three tested distributions with their versions and architecture, the artifact under test in each cell with its bytes and SHA-256, the formats with no tested row (AppImage and Flatpak — never built; RPM — deferred), and the untested combinations explicitly: every other distribution and release, every architecture but x86_64, real GPUs and vendor drivers, real desktop sessions, session-level desktop integration, the DEB's upgrade path, and the published v0.2.5 assets. **Done:** 2026-11-17: table published and linked.

**Blockers:** —

**Stopping condition:** the table exists, is linked from the artifact matrix, and its
"untested" column is non-empty where testing was not performed.
