# FireCraft Production Audit — 2026-11-20

> **Authority:** Consolidated findings from prior Claude session (544e31f5), omp session, CraftAnything experiment, and current state of `main` and `feat/release-provenance`.

## Current State

| Metric | Value |
|---|---|
| Version | v0.2.5 (2026-11-03) — source advanced beyond tag |
| Engine commands | 257 registered, 257/257 (100%) shell-bound |
| Menu census | 298 items, 2 inert |
| Tests | 1,885 passing |
| Coverage | 81.1% lines / 78.7% functions |
| WASM bundle | ~21.6 MB compressed (wasm-bindgen output); raw wasm32 ~92 MB |
| CI | Green on Linux (ci.yml), macOS+Windows (ci-cross.yml) |
| Releases | v0.2.0–v0.2.5 published; web on Pages (unidentifiable build) |
| Parity | 257/257 commands bound; 23 CS6 panels; 167 keymap bindings |

## Blockers

| Blocker | Type | Impact | Fix |
|---|---|---|---|
| rustc .rmeta version bug | Compiler | Blocks `xtask ci` — dashmap won't compile | Fix in rustc fork; cannot work around |
| WASM 92 MB > Pages 25 MB | Infrastructure | Blocks Cloudflare Pages deploy | Dynamic WASM loading from CDN |
| No artifact CERTIFIED | Process | Zero platforms verified | Maintainer approval + external hosts |
| Live site != v0.2.5 | Process | Deployed site has no identifiable source | Re-deploy from canonical bytes |

## Priority Work

### P0: Critical Path
1. **rustc .rmeta fix** — blocker for all CI. File as bug against rustc fork.
2. **WASM sharding** — split WASM into loader (JS) + runtime modules (fetch at startup).
3. **Merge omp session** — scripting optional, gzip staging, ci-cross.yml, homepage removal.

### P1: Important
4. **Deploy WASM from GitHub Releases** — JS loads WASM at runtime via `fetch()`.
5. **Merge CraftAnything evidence** — 100+ Ghidra evidence shards, mechanics.json, screenshots.
6. **Fix remaining clippy** — cast_possible_truncation (~650 warnings via #[allow]).
7. **Update task list** — reconcile all session findings.

### P2: Nice to Have
8. **shell/mod.rs refactor** — 7,993 lines -> split into modules.
9. **Coverage ratchet** — blocked by rustc bug. Target 99%.
10. **Cross-platform verification** — Windows/macOS/AppImage/Flatpak.
11. **Web deployment reconciliation** — deploy canonical v0.2.5 to Pages.

## Restored Tasks from Session 544e31f5

| Task | Priority | Status | Notes |
|---|---|---|---|
| 15.4 missing_docs evaluation | P0 | DONE | Zero warnings across all crates |
| RP-02 Windows packaging (Inno Setup) | P1 | BLOCKED | Needs Windows build host |
| RP-05 Web/CLI/source reconciliation | P0 | DONE | Reconciled in trunk-truth sweep |
| RP-06 SBOM/provenance/signatures | P1 | DONE | provenance.rs + jsonschema.rs committed |
| RP-07 Release orchestration | P1 | BLOCKED | Needs maintainer scope approval |
| Phase 22 CS6 shell fidelity | P1 | IN PROGRESS | 22.4/22.5 partial |
| Phase 20 daily-driver | P1 | IN PROGRESS | 20.4 PARTIAL (mask stack) |
| T27 cross-platform release | P2 | IN PROGRESS | No target CERTIFIED |
| Coverage floor ratchet | P1 | BLOCKED | By rustc .rmeta bug |
| WASM size reduction | P0 | IN PROGRESS | Scripting optional done |

## Task List

| # | Task | Priority | Status |
|---|---|---|---|
| 1 | rustc .rmeta version bug (blocks xtask ci) | P0 | BLOCKED |
| 2 | WASM sharding + CDN loading | P0 | IN PROGRESS |
| 3 | Merge omp session into main | P0 | PENDING |
| 4 | Sync TASKS.md with all findings | P0 | PENDING |
| 5 | Deploy WASM from GitHub Releases | P1 | PENDING |
| 6 | Fix cast_possible_truncation | P1 | PARTIAL |
| 7 | Merge CraftAnything evidence | P1 | PENDING |
| 8 | Update docs/boards | P1 | PENDING |
| 9 | shell/mod.rs refactor | P2 | PENDING |
| 10 | Coverage ratchet | P2 | BLOCKED |
| 11 | Cross-platform verification | P2 | PENDING |
| 12 | Web deployment reconciliation | P2 | PENDING |
