# P8 — Quality ratchet

**Goal:** quality is measured per crate and only moves up. That covers
coverage, docs, dependencies, input fuzzing, pixel snapshots of panels, the
wasm size and the web load time. The machinery mostly exists already; this
phase tightens it.

**What already exists on `main`** (do not rebuild it):
- `xtask coverage`: a workspace gate in `ci`, at 81.1 % line and 78.7 %
  function (`xtask/src/coverage.rs`).
- `xtask perf`: open, frame and save budgets plus peak memory, in `ci`.
- `xtask fuzz` / `fuzz-quick`: 12 targets in `fuzz/fuzz_targets/`.
- `xtask panel-snapshot`: the panel *inventory* golden. Pixel snapshots are
  blocked on a GPU adapter in CI (decision 0087).
- `xtask wasm`: builds the wasm crates. The size ceiling is 25 MB warn and
  50 MB hard (decision 0090).
- `#![deny(missing_docs)]`: only `geom` and `color` have it.

**Exit check:**
- S13 ≥ 85 % line, with per-crate floors;
- `missing_docs` denied in every L0–L3 crate;
- S8 ≤ 16,000,000;
- dependency audit in `ci`;
- pixel panel snapshots in CI.

**Lane:** quality. Can run alongside other phases. Repeatable rungs are
marked ↻: clone them as `C8.nn.k` for each crate.

---

### C8.01 — Re-measure coverage on the pinned toolchain `[ ]`
- **Needs:** C2.01, C0.04 · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. With `rust-toolchain.toml` in place, run
     `cargo run -p xtask -- coverage --measure`.
  2. If the measurement is above the floors, run `--bless`.
  3. Record both numbers in the SCOREBOARD row.
  4. If the measurement is *below* the floors, do not lower them. Mark the
     rung `[!]` with the output, because that means the gate in `ci` is red.
- **Done when:** S13 is recorded from a pinned-toolchain run.
- **Verify:** `cargo run -p xtask -- coverage`.
- **Moves:** S13 becomes a real number.

### C8.02 — Per-crate coverage floors `[ ]`
- **Needs:** C8.01 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. `coverage.rs` already reads lcov. Group the `SF:` records by
     `crates/<name>/`.
  2. Add a committed `.coverage-floor.tsv` (crate, line %, function %),
     written by `--bless`. Bless never lowers a floor, the same rule as
     reachability.
  3. The gate fails if any crate falls below its floor.
  4. Add unit tests for the grouping and the never-lower rule.
- **Done when:** the per-crate table is in `ci`, and a synthetic drop fails it
  in a test.
- **Verify:** `cargo test -p xtask coverage`.
- **Moves:** S13 becomes per-crate.

### C8.03 ↻ — Raise the lowest crate's coverage `[ ]`
- **Needs:** C8.02 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Pick the crate with the lowest line % in `.coverage-floor.tsv`.
  2. Use the lcov output to find its uncovered error paths and branches.
  3. Write real tests with real fixtures (from the corpus or generated). Do
     not write tests that just call functions without asserting anything.
  4. Raise the crate by at least 3 points, then run `--bless`.
- **Done when:** that crate's floor rose by at least 3 points.
- **Verify:** `cargo run -p xtask -- coverage`.
- **Moves:** S13 ↑, S6 ↑.

### C8.04 ↻ — Deny `missing_docs` in one more crate (#29) `[x]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** S per crate
- **Order** (lowest layer first; `geom` and `color` are done): `fwpng` →
  `raster` → `vector` → `text` → `filters` → `compose` → `doc` → `io` →
  `webexport` → `engine` → `scripting`.
- **Steps:**
  1. Add `#![deny(missing_docs)]` to the crate's `lib.rs`.
  2. Document every public item. Each doc says what the item does and its
     contract (units, errors, invariants), not its name restated.
  3. If a public item exists only for tests or for one sibling crate, make it
     `pub(crate)` instead of documenting it.
- **Done when:** `cargo doc -p <crate> --no-deps` and clippy are clean.
- **Verify:** `cargo clippy -p <crate> -- -D warnings`.
- **Moves:** after the last crate, close #29 and #80.
- **Landed:** the whole order list, as eleven clones of this rung, on the
  branch `climb/C8.04-missing-docs-campaign`. The count came first: a
  scan of every `pub` item, field, variant and trait item in the eleven
  crates found **32 undocumented public items** — `vector` 9 (`LShapeParams`
  and `ArrowParams` height, the `MAX_POINTS`/`MAX_TIGHTNESS_DEG` ceilings,
  the `Seg` variants, the `Piece` fields), `engine` 3 (`History::new`,
  `Journal::new`, `MemoryFs::new`), `doc` 6 (`EffectProperty::value`, the
  three `SymbolType` variants, `Registry::new`, `UnknownBag::new`),
  `scripting` 4 (`ScriptReport::outcome`, three `AttachView` flags),
  `filters` 3 (`Surface::width`, `Surface::height`, `SplitMix64::new`),
  `io` 2 (the `webp_lossy` quality ceilings) and `webexport` 0 — the one
  hit there was a `pub` field on a `pub(crate)` struct, which the lint
  does not see. `fwpng`, `raster`, `text` and `compose` had none, so their
  clones are the lint line alone. Every doc says what the item does and
  its contract; nothing was demoted to `pub(crate)`, because every gap
  was reachable public API.
  Two counts matter for the next agent. `missing_docs` does **not** fire
  on a `pub` item inside a private module, so the shell's and the
  catalog's private enums are not gaps; and it does **not** fire on a
  `pub use` re-export of an item documented at its definition — so the
  crates' long `pub use` lists are not gaps either. The `pub mod`
  declarations are satisfied by the `//!` inner docs of the module files.
  The residual: `app` and `app-cli` are the two public crates still
  without the lint. They are L5 shells and the rung's order list stops at
  `scripting`, so they are clones C8.04.12 and C8.04.13, not part of this
  landing.
- **Landed (clones 12 and 13):** the two shells are denied the lint
  too, so every public crate in the workspace now carries it. `app`
  closed 32 gaps — the `ShellAction` and `Arrange` variants that name
  the CS6 rows, the `Item`/`Entry`/`Menu`/`Mods`/`Shortcut` fields, and
  the editor's two bounds functions; one of them was a misplaced doc
  paragraph that described `object_bounds` while sitting on
  `union_selection_bounds`, and it moved to the function it describes.
  `app-cli` closed one: the public tuple field of `CliError`. Nothing
  was demoted to `pub(crate)`.
- **What CI caught that the scan did not:** a struct variant's named
  fields each need their own doc, not the variant's — `missing_docs`
  points at the field, not at the variant. The static scan missed the
  class because it looked for a doc comment above the item, and an
  inline variant (`DanglingMember { symbol: String, id: ObjectId }`) has
  none to find. The `doc` and `filters` clones failed on it — seven
  fields in two crates — and were fixed on their own branches
  (`e3fdedb8`, `d419a5c1`). The lesson for the next agent: a scan is a
  lower bound on the lint, and the compiler is the authority.
- **Clones:**

  | Clone | Crate | Commit | Branch |
  |---|---|---|---|
  | C8.04.1 | `fwpng` | `7b19d548` | `climb/C8.04.1-fwpng-missing-docs` |
  | C8.04.2 | `raster` | `918fc479` | `climb/C8.04.2-raster-missing-docs` |
  | C8.04.3 | `vector` | `8ba8bfe5` | `climb/C8.04.3-vector-missing-docs` |
  | C8.04.4 | `text` | `0dc9e4ff` | `climb/C8.04.4-text-missing-docs` |
  | C8.04.5 | `filters` | `c298dc91` + `d419a5c1` | `climb/C8.04.5-filters-missing-docs` |
  | C8.04.6 | `compose` | `c1163484` | `climb/C8.04.6-compose-missing-docs` |
  | C8.04.7 | `doc` | `be69d741` + `e3fdedb8` | `climb/C8.04.7-doc-missing-docs` |
  | C8.04.8 | `io` | `bf432810` | `climb/C8.04.8-io-missing-docs` |
  | C8.04.9 | `webexport` | `57cb39de` | `climb/C8.04.9-webexport-missing-docs` |
  | C8.04.10 | `engine` | `98f71ba5` | `climb/C8.04.10-engine-missing-docs` |
  | C8.04.11 | `scripting` | `423f646c` | `climb/C8.04.11-scripting-missing-docs` |
  | C8.04.12 | `app` | `3341c449` | `climb/C8.04.12-app-missing-docs` |
  | C8.04.13 | `app-cli` | `98b26468` | `climb/C8.04.13-app-cli-missing-docs` |

### C8.05 — Pixel panel snapshots in CI (decision 0087) `[ ]`
- **Needs:** C2.01 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. Add `mesa-vulkan-drivers` (lavapipe, a software Vulkan) to
     `infrastructure/docker/ci-rust.Dockerfile`. Bump the image tag to
     `firecraft-ci-rust:3`, and update every reference to the image
     (`git grep -n "ci-rust:2"`).
  2. Prove that `egui_kittest`'s `wgpu` snapshot feature finds the adapter in
     the image (test log).
  3. Add pixel snapshots for 5 panels: Layers, Properties (path), Optimize,
     Tools, States. Use a tolerance that the decision justifies.
  4. Add a decision, since 0087 is being superseded.
- **Done when:** the snapshots pass in CI, and a deliberate 1-pixel UI change
  fails them.
- **Verify:** `cargo run -p xtask -- panel-snapshot`, plus the CI log.
- **Moves:** S6 ↑; UI regressions become visible.

### C8.06 — Dependency audit in the gate `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Add `deny.toml`:
     - licences allow-list (Apache-2.0, MIT, BSD-2/3, ISC, Zlib, Unicode-3.0,
       MPL-2.0 if a dependency already needs it, and so on). Generate the
       start of the list from `cargo deny list`.
     - advisories: deny;
     - bans: duplicate versions are warned, not failed.
  2. Install `cargo-deny` in the CI image (same bump as C8.05, if they land
     together).
  3. Add an `xtask` `deny` step to `ci`, after `layers`.
- **Done when:** `ci` runs cargo-deny, and its output is clean or each
  exception has a reason comment.
- **Verify:** `cargo deny check`.
- **Moves:** release readiness (licences of SBOM components).

### C8.07 — Perf scenario: large document `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Add a generated fixture: 4000×3000, 50 objects, 10 live filters. Make
     it with a corpus-gen-style generator, not a checked-in binary.
  2. Add `open_large`, `frame_large` and `save_large` scenarios to
     `xtask perf`.
  3. Run `--measure`, then `--bless` the budgets with a headroom that is
     documented in the code.
- **Done when:** the scenarios are in `ci` with budgets.
- **Verify:** `cargo run -p xtask -- perf`.
- **Moves:** feeds C7.13.

### C8.08 — Wasm size: measure where the bytes go `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` `[DOCS]` · **Size:** S
- **Steps:**
  1. Build the release wasm: `cargo run -p xtask -- dist --web`.
  2. Run `twiggy top -n 40` and `twiggy dominators` on it. Also look at
     `cargo bloat --target wasm32-unknown-unknown --crates`, if it works for
     the target.
  3. Check the branch `perf/wasm-accesskit-probe` (AUDIT §2) for earlier
     findings.
  4. Write `docs/research/wasm-size-2026.md`: the top 20 contributors by
     crate, and candidate savings. Candidates include:
     - the boa scripting engine;
     - the PSD and TIFF decoders;
     - fonts embedded with `include_bytes!`;
     - `opt-level="z"` and `wasm-opt -Oz`;
     - LTO and `codegen-units=1` settings.
- **Done when:** the research doc is merged.
- **Verify:** review.
- **Moves:** plan for C8.09.

### C8.09 ↻ — Wasm size: one trim `[ ]`
- **Needs:** C8.08 · **Tags:** `[HOST:linux]` `[HOST:browser]` · **Size:** S–M
- **Steps:**
  1. Apply one candidate from C8.08. Examples:
     - lazy-load the scripting engine into a second wasm module;
     - put rarely used importers behind a web feature;
     - tune the release profile for size.
  2. Prove that the web build still works through the C3.17 smoke.
  3. Lower the ceiling in decision 0090's gate toward 16 MB.
- **Done when:** S8 falls, and the browser smoke passes.
- **Verify:** `cargo run -p xtask -- dist --web && stat -c %s dist/web/*.wasm`.
- **Moves:** S8 ↓.

### C8.10 — Web load-time budget `[ ]`
- **Needs:** C3.11, C3.17 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. A Playwright test measures navigation → bridge `ready` message → first
     canvas frame, on a throttled connection ("Fast 3G" profile) and a cold
     cache.
  2. Record the result in a budget file. CI fails above the budget.
- **Done when:** the budget is enforced in the web smoke job.
- **Verify:** the Playwright report.
- **Moves:** S8's user-visible twin.

### C8.11 — Fuzz anchors for every decoder `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. The current targets are listed in `ls fuzz/fuzz_targets`. Add the
     missing decoders:
     - `png_decode`, `jpeg_decode`, `gif_decode`;
     - `webp_decode`, `tiff_decode`, `bmp_decode`, `wbmp_decode`;
     - plus the bridge message parser, if C3.08 has not added it.
  2. Register each target in `xtask/src/fuzz.rs` (quick and full).
  3. Seed each target's corpus from tiny, originally generated files.
- **Done when:** `fuzz-quick` runs every target clean.
- **Verify:** `cargo run -p xtask -- fuzz-quick`.
- **Moves:** robustness.

### C8.12 — Run `panic-hunt` to zero `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** S–M
- **Steps:**
  1. Run `cargo run -p xtask -- panic-hunt`, the catalog fuzz campaign.
  2. For each finding, add a regression test and fix it by returning a
     typed error. Do not catch the panic.
  3. Record the clean run in the PR.
- **Done when:** a full campaign run reports 0 findings.
- **Verify:** that command.
- **Moves:** robustness; S6 ↑.

### C8.13 — Clippy lint parity across crates `[ ]`
- **Needs:** C2.07 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. List every `[lints]` table in `crates/*/Cargo.toml` that overrides
     `[workspace.lints]` (C2.07 fixed `io`).
  2. For each override, either remove it and fix the findings, or keep it
     with a comment that cites a decision.
- **Done when:** `git grep -n "\[lints" crates/*/Cargo.toml` shows only
  `workspace = true` or overrides with a cited reason.
- **Verify:** clippy is clean.
- **Moves:** consistency.

### C8.14 — Quality-phase summary `[ ]`
- **Needs:** C8.01–C8.13 · **Tags:** `[DOCS]` · **Size:** XS
- **Steps:** update `docs/HANDOFF.md`'s CI runbook to list every gate in
  `ci`, with its floor file and bless verb.
- **Done when:** every `ci` step appears in the runbook.
- **Verify:** compare the runbook with the `run_ci` step list in
  `xtask/src/main.rs`.
- **Moves:** —
