# T27-49 — Run the bridge smoke against the release bytes with no skip

> **RP:** RP-05 · **Priority:** P0 · **Status:** NOT STARTED
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

**Evidence:** `xtask` strips `<script>` bodies before its DOM assertions.

**Blockers:** chromium on the build host; the CI image currently lacks it (see
T27-16).

**Stopping condition:** a passing smoke run against the exact candidate bundle is
recorded.
