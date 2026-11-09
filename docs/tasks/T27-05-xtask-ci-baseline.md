# T27-05 — Baseline: run the canonical `xtask ci` gate on the release candidate

> **RP:** RP-00 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-01

**Objective:** run the project's own gate — fmt, clippy `-D warnings`, tests, layer
enforcement, wasm build — on the release candidate and record the exact result.

**Inputs:** `xtask` crate (`xtask/src/main.rs`, `ci` command); `.aegis-baseline.json`;
`.aegisignore`; the pinned Aegis 0.6.3 binary.

**Authority:** `AGENTS.md` — the project gate is `cargo run -p xtask -- ci`.

**Acceptance criteria:** `xtask ci` completes green on the candidate SHA, or every
failure is recorded with its cause and its owning file.

**Verification:** `cargo run -p xtask -- ci`

**Evidence:** the GitHub `gate` job fails on PR #9 (runs `42401632606`,
`42401626535`) with 5 Aegis findings absent from the 520-entry baseline — reproduced
locally 2026-11-08.

**Blockers:** Aegis baseline drift — T27-12 and T27-13.

**Stopping condition:** a green run on the candidate SHA, or a written failure list
that T27-12/13 consume. Do not proceed past a red gate.
