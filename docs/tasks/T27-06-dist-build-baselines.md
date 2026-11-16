# T27-06 — Baseline: run the four dist builds and record sizes and digests

> **RP:** RP-00 · **Priority:** P0 · **Status:** BUILD VERIFIED
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

**Evidence:** All four `dist` lanes ran from the release tag. The rehearsal's `build artifacts` job (run 42937560976, head SHA `384687f`) produced the native set, the web bundle, the source archive and the SBOM and wrote the release-wide checksum list. The eight published assets carry their sizes and SHA-256 digests: the native tarball, the CLI tarball, the DEB, the web ZIP (7,394,859 B), the source archive, the SBOM, `MANIFEST.sha256` (425 B) and `RELEASE_SHA256.txt` (seven entries covering the other seven assets). Receipt: `docs/release/RECEIPT-0.2.5-DRYRUN-3.md`. **Done:** 2026-11-12: the rehearsal produced the four lanes with their digests.

**Blockers:** —

**Stopping condition:** a dated receipt listing each artifact's path, size and digest,
or an explicit statement of which dist command could not run and why.
