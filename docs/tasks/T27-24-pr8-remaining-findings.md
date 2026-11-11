# T27-24 — Verify the remaining PR #8 findings, fix the valid ones, document the rejected ones

> **RP:** Phase-2 (PR #8 remediation) · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-03

**Objective:** close out every remaining Major/Critical review finding on PR #8 by
verifying it against current code, fixing it if valid, and rejecting it with a written
reason if not.

**Inputs:** the PR #8 inline comments; the census in `docs/release/EXECUTION-BOARD.md`
§6; `crates/app/src/editor.rs`; `crates/app/src/shell/panels_web.rs`;
`crates/engine/src/catalog/retouch.rs`; `xtask/src/main.rs`; `docs/CS6-PARITY.md`,
`docs/NEXT.md`, `docs/PROGRESS.md`, `docs/HANDOFF.md`, `docs/wiki/`, `README.md`.

**Authority:** the PR #8 reviews; `docs/RELEASE-ENGINEERING-PLAN.md` §9 (a PR must
state unverified gaps explicitly).

**Acceptance criteria:** every remaining Major/Critical finding is independently
verified against current code; valid ones are fixed with a regression test; rejected
ones carry a written reason naming the code that is actually correct.

**Verification:** `cargo test -p app -p engine`

**Evidence:** candidates — `editor.rs:619` `drag_quadrangle` moves the wrong corners and
its doc comment is inaccurate; `editor.rs:2376` Scale/Skew/Distort read
`document().object_selection` instead of the live selection; `editor.rs:2595` Redraw
Path drag must send the sampled pointer trace, not a two-point segment;
`panels_web.rs:574` sends unsupported width/height in `symbol.createInstance`;
`panels_web.rs:595` a second symbol/insert_box payload finding; `retouch.rs:656` and
`retouch.rs:1149` require saturating multiplication for a radius→diameter conversion;
`xtask/src/main.rs:222` a Minor finding on the release branch; plus documentation-accuracy
findings across `docs/CS6-PARITY.md`, `docs/NEXT.md`, `docs/PROGRESS.md`,
`docs/research/fw-ui-evidence.md`, `docs/HANDOFF.md`, `docs/wiki/` and `README.md`.

**Blockers:** none.

**Stopping condition:** every enumerated finding has a disposition — fixed with a
regression test, or rejected with a reason — and the dispositions are recorded.
