# T27-32 — Assess a Linux aarch64 target

> **RP:** RP-01 · **Priority:** P2 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-04

**Objective:** produce a written verdict on whether a Linux aarch64 build is feasible
for this codebase, and what it would cost — without starting the port.

**Inputs:** the dependency graph (eframe, winit, glow, glutin); the DEB control file's
`Architecture: amd64`; the available cross-compilation toolchain; the artifact matrix's
aarch64 row.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1 (RP-01 includes the aarch64
assessment).

**Acceptance criteria:** a written assessment covers the aarch64 toolchain, native
dependencies (notably the GPU stack), the packaging path for arm64, the CI cost, and
a clear feasible / not-feasible / feasible-later verdict with its conditions.

**Verification:** `rustup target list --installed`

**Evidence:** the artifact matrix records Linux aarch64 as "unassessed" — no
assessment exists.

**Blockers:** none.

**Stopping condition:** the assessment is committed and the matrix's aarch64 row is
updated from "unassessed" to the verdict.
