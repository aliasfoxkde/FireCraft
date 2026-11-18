# T27-49 — Run the bridge smoke against the release bytes with no skip

> **RP:** RP-05 · **Priority:** P0 · **Status:** SMOKE TESTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-48

**Objective:** prove the exact bytes that will ship actually run in a browser — an
open → apply → export round trip — rather than merely compiling.

**Inputs:** the bundle from T27-48; headless Chromium; `xtask/src/dist.rs`
(`smoke_bridge`, `strip_script_bodies`).

**Authority:** the repo's rule that a verdict string found in the page's own `<script>`
source always matches and is therefore not evidence — DOM assertions must be made
against the stripped page.

**Acceptance criteria:** the release bytes complete an open → apply → `exportPng` →
`exportDocument` round trip in headless Chromium; the smoke is not skipped; the
verdict is read from the rendered DOM.

**Verification:** `cargo run -p xtask -- dist --web` (the bridge-smoke stage).

**Evidence:** the smoke runs and is not skipped. `dist --web` calls `smoke_bridge`, the release workflow installs headless Chrome and fails closed without it, and `FIRECRAFT_DIST_SKIP_SMOKE` appears nowhere in `.gitforge.yml` or the GitHub workflows — only as a comment at `xtask/src/dist.rs:52`. The rehearsal's `build artifacts` job (run 44188107435) built the bundle and ran the smoke against it: the job log reads `[xtask] dist web: 7 file(s), wasm 15780814 B …, smoke green` and `[xtask] dist --web ok` at 2026-11-17T22:31:41Z. The GitForge `release` job logs the same line for the mirror builds. Receipt: [RECEIPT-0.2.5-DRYRUN-6.md](../release/RECEIPT-0.2.5-DRYRUN-6.md). **Done:** 2026-11-17: the smoke ran green against the rehearsal's bundle (run 44188107435).

**Blockers:** —

**Stopping condition:** a passing smoke run against the exact candidate bundle is
recorded.
