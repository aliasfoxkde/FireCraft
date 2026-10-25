# M0 — Bootstrap & method

> **CLOSED milestone (historical reference).** See docs/PROGRESS.md for the closure date and docs/HANDOFF.md for current work.

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** turn this docs-only repo into a working Rust workspace with
the full photocraft-style method installed (gates, license, operating
manual) — before any Fireworks-specific code.

**Inputs:** PLANNING.md (this repo), photocraft conventions (AGENTS.md §2/§3/§5
patterns), research report §7 (architecture), §11 (method).

**Authority:** docs/planning/firecraft/ROADMAP.md M0 gate. Deviations get a
decision note in `docs/decisions/`.

## Tasks (atomic)

1. **Workspace scaffold** — `Cargo.toml` (workspace, resolver "3",
   edition 2024, `[workspace.package]` shared keys, `[workspace.lints]`
   with `unsafe_code = "forbid"` + clippy strictness, `[workspace.dependencies]`
   path deps). Members: `crates/*`, `xtask`. Dir skeleton: `crates/`,
   `xtask/`, `web/`, `assets/`, `packaging/`, `docs/decisions/`,
   `docs/research/` (exists). **Stopping condition:** `cargo metadata` clean.
2. **License switch** — add `LICENSE-MIT`, keep Apache-2.0 as
   `LICENSE-APACHE`, root `LICENSE` → pointer/dual-license text, `NOTICE`
   with Adobe trademark disclaimer, README disclaimer paragraph.
   **Verification:** `cargo` manifests declare `license = "MIT OR Apache-2.0"`.
3. **`fwpng` crate skeleton** — standalone, zero workspace deps; manifest
   only + module layout (`chunk`, `nodetree`, `thumbnail`, `tile`) each with
   a real smoke test (e.g. chunk CRC round-trip, tiny node-tree encode/decode
   of `MKBv{}`), so the crate is testable and *not* placeholder code.
4. **`geom`, `color`, `raster` skeletons** — manifests wired to workspace
   lints; each ships one real behavior test (affine identity/inverse; blend
   of two known pixels; tile COW semantics) proving the lint/gate pipeline
   end-to-end.
5. **`xtask`** — subcommands: `ci` (fmt-check, clippy `-D warnings`
   all-targets, test, layers, wasm), `layers` (declared layer table +
   dependency-direction check via `cargo metadata`), `wasm`
   (`cargo build --target wasm32-unknown-unknown` over pure-Rust crates +
   report any C-FFI leakage), `corpus` (manifest fetch + sha256 check;
   no-op until M2), `parity`/`scorecard` (generate empty tables;
   `enforce = false`). **Stopping condition:** `cargo run -p xtask -- ci`
   green locally.
6. **`AGENTS.md`** — operating manual per PLANNING §11.1 (orientation,
   workspace map, golden rules, work-picking, finish checklist incl.
   `xtask ci` + wasm, parallel-agent etiquette, tracking pointers). ≤ ~150
   lines, dense.
7. **Repo hygiene** — `.gitignore` += `/target/` `/plan/` `/log/`
   `/corpus/`; gitignored `plan/` + `log/` dirs created with `.gitkeep`;
   `docs/ATTRIBUTION.md` exists with the MIT vendored research row;
   conventional-commit policy referenced from AGENTS.md.
8. **Session tracking convention** — AGENTS.md states: session task list
   mirrors the active milestone; devlog entry closes every session.
   First devlog entry written.

**Verification (milestone gate):** `cargo run -p xtask -- ci` green —
fmt, clippy `-D warnings`, tests (real, failing-if-broken), layers, wasm —
plus AGENTS.md/NOTICE/license files present. No placeholder markers in
tracked source (the anti-pattern list in `.claude/rules/quality.md`):
a repo-wide marker grep must come back empty.

**Out of scope:** any real geometry/color/format logic (M1/M2), CI hosting
setup (decide GitForge pipeline in M0's devlog, wire in M1).
