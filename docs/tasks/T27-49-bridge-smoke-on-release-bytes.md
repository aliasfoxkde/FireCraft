# T27-49 — Run the bridge smoke against the release bytes with no skip

> **RP:** RP-05 · **Priority:** P0 · **Status:** BUILD VERIFIED
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

**Evidence:** The smoke runs and is not skipped. `dist --web` calls `smoke_bridge` ([`xtask/src/dist.rs`](https://github.com/aliasfoxkde/FireCraft/blob/main/xtask/src/dist.rs), line 65), the release workflow installs headless Chrome and fails closed without it (`.github/workflows/release.yml:106`), and `FIRECRAFT_DIST_SKIP_SMOKE` appears nowhere in `.gitforge.yml` or the GitHub workflows — only as a comment at `xtask/src/dist.rs:52`. The rehearsal's `build artifacts` job (run 42937560976) therefore ran the smoke against the tagged bundle and succeeded, and the GitForge release job logs `… smoke green` and `[xtask] dist --web ok` — most recently job `ecb488d1-8965-443f-8309-71405883d754` for commit `fcfe6db1`, finished 2026-11-12T06:18:54Z; twelve release jobs carry that line. **Done:** 2026-11-12: the bridge smoke ran green against the built bundle.

**Blockers:** —

**Stopping condition:** a passing smoke run against the exact candidate bundle is
recorded.
