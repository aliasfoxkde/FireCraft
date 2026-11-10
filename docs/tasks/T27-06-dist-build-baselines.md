# T27-06 — Baseline: run the four dist builds and record sizes and digests

> **RP:** RP-00 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-01

**Objective:** run `dist --native`, `dist --web`, `dist --source` and `dist --sbom` on
the release candidate and record the size and SHA-256 of everything produced, so later
rows compare against a known starting point rather than against nothing.

**Inputs:** `xtask/src/dist.rs`; the fleet toolchain (rustup stable, trunk, dpkg-deb,
chromium).

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5.1; `docs/RELEASE-ENGINEERING-PLAN.md`
acceptance criterion "every required artifact is built from the exact release tag SHA
with locked dependencies".

**Acceptance criteria:** all four dist commands run; every produced artifact has a
recorded size and SHA-256; failures are recorded with their cause.

**Verification:** `cargo run -p xtask -- dist --native && cargo run -p xtask -- dist --web && cargo run -p xtask -- dist --source && cargo run -p xtask -- dist --sbom`

**Evidence:** none yet — this row has not been run on the candidate.

**Blockers:** toolchain availability on the executing host.

**Stopping condition:** a dated receipt listing each artifact's path, size and digest,
or an explicit statement of which dist command could not run and why.
