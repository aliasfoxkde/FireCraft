# T27-48 — Build the web bundle from the exact tag SHA and record its manifest

> **RP:** RP-05 · **Priority:** P0 · **Status:** BUILD VERIFIED
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

**Evidence:** The bundle was built from the tag's commit and published with its manifest: the rehearsal's `build artifacts` job (run 42937560976, head SHA `384687f`) ran `dist --web` successfully and published `firecraft-web-0.2.5-dryrun.3.zip` (7,394,859 B, sha256 `57054d06…`) with `MANIFEST.sha256` (425 B, sha256 `e71ce485…`); the release-wide list covers the ZIP and the manifest verifies with `sha256sum -c`. Receipt: [`docs/release/RECEIPT-0.2.5-DRYRUN-3.md`](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/release/RECEIPT-0.2.5-DRYRUN-3.md). The WASM is stamped and inside the budget — the GitForge release job for `fcfe6db1` logged `wasm 15573974 B (warning 25000000, hard ceiling 50000000)`, 5 files. **Done:** 2026-11-12: the rehearsal published the web bundle and its manifest.

**Blockers:** —

**Stopping condition:** the bundle exists from the candidate SHA with a recorded
manifest, and the size ceiling check passes.
