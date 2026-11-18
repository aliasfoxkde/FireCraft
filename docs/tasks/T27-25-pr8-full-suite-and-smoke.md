# T27-25 — Run the full application suite, the CI gate and the web smoke for PR #8

> **RP:** Phase-2 (PR #8 remediation) · **Priority:** P0 · **Status:** SMOKE TESTED
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

**Evidence:** the whole suite ran on the head `f710c524` and passed. PR #8 itself was closed unmerged on 2026-11-12, so the head that carries its re-hosted findings (PR #171, merged as `d3e45cf9`) is the branch head `f710c524`, and that is what was run. **GitHub**, 13 check runs, all terminal on that SHA (the PR run is [44194696167](https://github.com/aliasfoxkde/FireCraft/actions/runs/44194696167)): the two `gate` legs — each `cargo run --package xtask -- ci-no-fmt`, the canonical gate (format, clippy with `-D warnings`, the workspace test suite, layer enforcement, the wasm build) — both `success`; the two `fmt` legs `success`; the two `classify-change` legs `success`; the two `docs-only-check` legs `skipped` because this change is not documentation-only, and the Aegis secret scan runs in its own legs (`cargo run --locked --package xtask -- aegis`), which are `success`; CodeQL `success`; CodeRabbit `success`; the three `Analyze` legs `success`. **GitForge mirror**, pipeline run `65de365f` on the same SHA: all 12 jobs `succeeded` — `clippy-test` (the application test suite), `wasm-web` (`[xtask] wasm ok`), `release` (`[xtask] dist web: 6 file(s), wasm 15783798 B (warning 25000000, hard ceiling 50000000), manifest MANIFEST.sha256, smoke green` and `[xtask] dist --web ok` — the browser smoke against the built bundle), `aegis`, `layers`, `fmt-layers`, `corpus-golden`, `fuzz`, `perf-budgets`, `generated-docs` and `release-ledger` (the committed ledger has no problems). **Done:** 2026-11-17: the full gate, the application test suite, the web build and the browser smoke all green on the head `f710c524`.

**Blockers:** —

**Stopping condition:** all four checks pass on the PR head SHA, or every failure is
recorded with its cause and owner.
