# T27-25 — Run the full application suite, the CI gate and the web smoke for PR #8

> **RP:** Phase-2 (PR #8 remediation) · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-23, T27-24

**Objective:** prove the PR #8 branch is releasable by running the application test
suite, the canonical CI gate and the web build with its browser smoke on the PR head.

**Inputs:** `xtask` (`ci`, `dist --web`); `crates/app`; `crates/engine`; chromium on the
build host.

**Authority:** `AGENTS.md` — the project's gate is `cargo run -p xtask -- ci`; the repo
rule that the web smoke verdict must come from the rendered DOM, not from the page's
own `<script>` source.

**Acceptance criteria:** the app test suite, `xtask ci`, the web build and the browser
smoke all pass on the PR head, and the results are recorded with the SHA.

**Verification:** `cargo run -p xtask -- ci && cargo run -p xtask -- dist --web`

**Evidence:** none yet.

**Blockers:** T27-23, T27-24; the Aegis findings in T27-12/13 also gate the CI leg.

**Stopping condition:** all four checks pass on the PR head SHA, or every failure is
recorded with its cause and owner.
