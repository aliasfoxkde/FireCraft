# T27-48 — Build the web bundle from the exact tag SHA and record its manifest

> **RP:** RP-05 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-06

**Objective:** build the WASM bundle from the release candidate and record its manifest
and per-file hashes — the artifact the deployed site must eventually match.

**Inputs:** `xtask/src/dist.rs` (`web()`, `enforce_wasm_budget`); `trunk`;
`WASM_SIZE_CEILING = 21_500_000`; the version stamp.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5.1; the WebOS/web bundle is the
primary release surface.

**Acceptance criteria:** the bundle builds from the tag SHA; the WASM is optimized and
under the 21.5 MB ceiling; the version is stamped; a manifest plus per-file hashes are
recorded.

**Verification:** `cargo run -p xtask -- dist --web && (cd dist/web && sha256sum -c MANIFEST.sha256)`

**Evidence:** the 2026-11-06 build was green — wasm 21.27 MB (ceiling 21.5), 5 files,
digest `728936e0…fa447f`.

**Blockers:** none beyond T27-06.

**Stopping condition:** the bundle exists from the candidate SHA with a recorded
manifest, and the size ceiling check passes.
