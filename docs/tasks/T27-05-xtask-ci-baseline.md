# T27-05 — Baseline: run the canonical `xtask ci` gate on the release candidate

> **RP:** RP-00 · **Priority:** P0 · **Status:** BUILD VERIFIED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-01

**Objective:** run the project's own gate — fmt, clippy `-D warnings`, tests, layer
enforcement, wasm build — on the release candidate and record the exact result.

**Inputs:** `xtask` crate (`xtask/src/main.rs`, `ci` command); `.aegis-baseline.tsv`;
`.aegisignore`; the pinned Aegis 0.6.3 binary.

**Authority:** `AGENTS.md` — the project gate is `cargo run -p xtask -- ci`.

**Acceptance criteria:** `xtask ci` completes green on the candidate SHA, or every
failure is recorded with its cause and its owning file.

**Verification:** `cargo run -p xtask -- ci`

**Evidence:** The repository's canonical gate is green on `main` `e39705e` — the `fmt` check (131002886797) and the `gate` check (131002941116) are both `completed`/`success` — and on every candidate head. `gate` runs `xtask ci-no-fmt` and `fmt` runs the formatting check, together the `xtask ci` gate. The 2026-11-08 failure on runs `42401632606`/`42401626535` was Aegis baseline drift, closed by T27-12 and T27-13. **Done:** 2026-11-12: gate green on `main` `e39705e` and on the candidate heads.

**Blockers:** —

**Stopping condition:** a green run on the candidate SHA, or a written failure list
that T27-12/13 consume. Do not proceed past a red gate.
