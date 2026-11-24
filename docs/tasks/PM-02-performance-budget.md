# PM-02 — Large-document performance and memory budget

> **Parent workstream:** REL-02 (POST-MVP-ROADMAP §5.1).
> **Priority:** P0/P1 — budgets must exist before optimization claims.
> **Status:** `NOT STARTED` — gated on Gate 0.
> **Owner:** unassigned. **Created:** 2026-11-23.
> **Dependencies:** PM-01 is not a prerequisite; both touch the save/open path and
> should not land in the same files concurrently.

## 1. User problem

Users cannot know whether FireCraft is fast enough for their documents, and
maintainers cannot tell a regression from noise. There is no versioned benchmark
corpus, no published startup/open/render/edit/save measurements, and no memory
accounting standard.

## 2. Current implementation and evidence

- `xtask/src/perf.rs` exists and the gate runs a memory probe (decision 0072).
- `crates/raster/src/store.rs` is a copy-on-write tiled store; `compose/src/cache.rs`
  is a content-keyed cache; `compose/src/render.rs` has a 16 MiPix budget.
- PRODUCTION-ROADMAP Phase H and PRODUCTION-TODO record WASM size only; no
  document-scale benchmark table exists at `6c68c9e3`.
- All "fast" or "memory-improved" claims in docs are dated and **UNVERIFIED** at
  this SHA.

## 3. Scope

In scope: a versioned benchmark corpus (small/ordinary/large/stress), a repeatable
harness that records host, toolchain, profile, corpus version, and source SHA;
metrics for startup, open, first render, viewport navigation, selection, common edits, save,
reopen, export, peak RSS, and cache hit/eviction; a published results table; and a
diagnostic report for large assets/expensive effects.

**Non-goals:** GPU optimization work, changing the renderer, or setting arbitrary
latency targets before measuring.

## 4. Files and surfaces likely affected

`xtask/src/perf.rs`, `xtask/src/corpus.rs`, `corpus/` metadata (fixtures stay
uncommitted where Adobe-authored), `crates/compose/src/cache.rs`,
`crates/raster/src/store.rs`, `docs/planning/generated/` (new measured table),
`docs/PROGRESS.md`. Surfaces: CLI/headless (authoritative), desktop and web for
confirmation only.

## 5. Decisions required

- Corpus composition and generation recipe (synthetic documents committed;
  Adobe-authored oracles never committed).
- The memory metric definition (peak RSS vs allocator accounting) and the
  acceptable variance window.
- Whether budgets are gate-enforced (fail CI) or report-only in this iteration.

## 6. Implementation steps

1. Define the corpus manifest (size classes, feature mix, expected element counts).
2. Add a `xtask perf --measure` mode that runs each scenario headless and emits
   JSON with environment metadata.
3. Add a cache/allocator counter surface so the report can name evictions and
   peak live bytes.
4. Record a baseline table for the integrated tree.
5. Add a rising-only floor for one or two stable metrics (open time, peak RSS on
   the large fixture) only after three consecutive runs agree.
6. Publish the table and the limitations.

## 7. Acceptance criteria

- `xtask perf --measure` is deterministic enough that repeated runs report the
  same corpus hash and within the documented variance.
- Every published number names SHA, toolchain, profile, host, and corpus version.
- A repeated open/edit/undo/export/close loop shows no unbounded growth.
- Rendering/editing remain correct when GPU init fails and CPU compositing is used.
- The diagnostic report names the largest assets and effects in a document.

## 8. Test plan

- Harness unit tests for manifest parsing and JSON schema.
- A CI job that runs the smallest corpus and fails on a >X% regression of the
  pinned metric, X chosen from observed variance.
- `cargo test -p xtask`, `cargo run -p xtask -- ci`.

## 9. Manual verification

One run on a low-memory host and one on a GPU-less host; record both.

## 10. Performance, security, portability

The harness itself must not load documents from untrusted paths without caps; it
is CLI-only and network-free. WASM measurements are reported separately and never
mixed with native numbers.

## 11. Risks and rollback

Risk: flaky CI budgets. Mitigation: report-only first, floors only after three
agreeing runs; revert the floor commit if variance exceeds the window.

## 12. Stopping condition and closure evidence

Stop if the integrated tree cannot build headless on the measurement host. Close
with the published table, the harness, and one regression receipt showing the
floor firing.
