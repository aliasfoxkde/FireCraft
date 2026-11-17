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

**Evidence:** Candidates: `editor.rs:619` drag_quadrangle, `editor.rs:2376` redraw trace, `editor.rs:2595` live_selection, `panels_web.rs:574/595` payload, `retouch.rs:656/1149` saturating multiplication. **One candidate is already verified live on `main`** at `978215e9`: the Vector Path node drag that a pure click never ends (§6, "PR #8 findings raised after the 2026-11-08 baseline") — `begin_node_drag` on click (`canvas_ui.rs:1268`), `end_node_drag` only on `drag_stopped` (`canvas_ui.rs:1152`)

**Blockers:** —

**Stopping condition:** every enumerated finding has a disposition — fixed with a
regression test, or rejected with a reason — and the dispositions are recorded.
