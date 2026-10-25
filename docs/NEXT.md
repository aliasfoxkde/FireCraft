# FireCraft — Next-agent plan (2026-10-25)

> Start at `docs/HANDOFF.md` first. This file is the live backlog
> and the expanded plan. HANDOFF §4 has the phase descriptions.
> Rewritten 2026-10-25 from a full audit: coverage measured at
> 80.47% lines / 77.85% functions, production code panic-free,
> clippy zero-warning, v0.2.3 shipped.

---

## Measured baseline (2026-10-25)

| Metric | Value | Source |
|---|---|---|
| Line coverage | 80.47% (21,401 / 109,573) | `cargo llvm-cov --summary-only` |
| Function coverage | 77.85% (1,072 / 4,839 missed... see note) | same run |
| Region coverage | 79.83% | same run |
| Clippy (default + deny set) | 0 warnings | `xtask ci` |
| Pedantic clippy delta | ~1,825 hits (86% `cast_possible_truncation`) | manual run |
| Production unwrap/panic/expect | **0** (all hits in `#[cfg(test)]` files) | grep audit |
| Tests | 38 editor + 106 io + workspace suite | cargo test |
| Web crate | clippy-clean, fmt-gated, smoke-gated | manual + dist --web |

Note: the 77.85% function figure reads `4839` as the total function
count in the summary table, not the missed count — re-read the table
columns when re-blessing.

## Worst coverage files (missed lines, 2026-10-25)

| File | Missed | What's untested |
|---|---|---|
| crates/app/src/editor.rs | 665 | 75 public fns lack a test: import_styles, wire, replay_steps, symbol helpers |
| crates/engine/src/catalog/object.rs | 483 | refusal branches of the object family |
| crates/engine/src/catalog/mask.rs | 427 | mask refusal + edge paths |
| crates/compose/src/effects.rs | 360 | filter-stack edge parameters |
| crates/engine/src/catalog/path.rs | 271 | path family refusals |
| crates/engine/src/fuzzdrive.rs | 160 | driver-only (acceptable; fuzz is its own gate) |
| crates/io/src/psd.rs | 180 | PSD error paths (malformed headers) |

## Phases

### Phase 14 — Coverage ratchet to 99% (multi-session)

The floors were raised 60% → 80% lines / 77% functions (measured
baseline, 2026-10-25). The gate is rising-only from here.

- **14.1 Editor surface tests** (biggest single gap, 665 missed).
  Tests for: `import_styles` (good mkTS + corrupt + missing-mkBT),
  `replay_steps`, `wire` string table, symbol helpers, `pick_color`,
  `begin_stroke`/`stroke_to`/`end_stroke` family, `commit_polygon`,
  guide drag family, nine-slice toggles. ~30 tests. Accept: editor.rs
  line coverage ≥ 95%.
- **14.2 Catalog refusal branches** (object 483, mask 427, path 271,
  text 211, bitmap 193, web 152). Each command's error rows are
  mechanical: wrong payload shape, missing target, out-of-range
  params. Write a table-driven helper (payload → expect refusal) and
  cover every `Err` arm. Accept: each listed file ≥ 95%.
- **14.3 compose/effects.rs edge parameters** (360). Filter kernels
  with zero-size inputs, degenerate radii, boundary pixels.
- **14.4 io/psd.rs malformed-header tests** (180): truncated chunks,
  bad version, impossible dims.
- **14.5 Ratchet the floors**: after each of 14.1–14.4 lands, re-run
  `cargo llvm-cov --summary-only` and bless the floors to the new
  number. Target: 99% lines / 99% functions. Record each raise in
  the session log.
- **14.6 Coverage of the `web/` crate**: trunk builds it outside the
  workspace; add `cargo llvm-cov --manifest-path web/Cargo.toml` to
  the measure path (may need the wasm target excluded — measure,
  then decide).

### Phase 15 — Lint hardening

Current deny set: unwrap_used, expect_used, panic, todo,
unimplemented, unreachable, dbg_macro, print_stdout, exit +
unsafe_code=forbid. Production code is already clean.

- **15.1 Curated pedantic adoption.** Blanket pedantic is 1,825 hits,
  86% `cast_possible_truncation` (raster math does i64↔u32↔f64
  constantly). Do NOT blanket-enable. Instead enable per-lint:
  - `clippy::cast_possible_truncation` → add `#[allow]` with a
    one-line justification at the ~50 raster-math sites, deny
    elsewhere? No — measure first; if the allow density is too high
    (>1 per 100 lines), record a refusal decision instead.
  - `clippy::doc_markdown` (13 hits) → fix the docs, enable.
  - `clippy::needless_pass_by_value` (8) → fix, enable.
  - `clippy::must_use_candidate` (9) → evaluate; enable if the hits
    are real.
- **15.2 web/ clippy in the gate** — DONE 2026-10-25 (xtask clippy
  gate + `.gitforge.yml` addition below).
- **15.3 `.gitforge.yml`: add the web clippy step** to the
  clippy-test job (`cd web && cargo clippy --all-targets -- -D
  warnings`). Requires the CI image to carry the web/ manifest (it
  does — the fetch context includes web/).
- **15.4 `#![deny(missing_docs)]` evaluation** — public-API doc
  coverage. Count first; if the delta is under ~200 items, fix and
  enable per-crate starting with `geom` and `color`.

### Phase 16 — WCAG 2.1 AAA audit (web host)

The host surface is `web/index.html` (29 lines) + the egui canvas.
The egui app itself draws via egui/accesskit — egui has built-in
accesskit integration.

- **16.1 Static audit of index.html**: lang attribute, title, meta
  description, skip-link, canvas fallback text, focus outline policy.
  Small; fix in place.
- **16.2 egui/accesskit audit**: verify accesskit is compiled in on
  the wasm build (egui feature flag), test with a screen reader on
  the deployed site, document what is reachable. Record results in a
  decision doc — some AAA criteria (1.4.8 visual presentation,
  2.1.3 keyboard-no-exception) may be honest refusals for a
  canvas-drawing app; document them as such rather than fake conformance.
- **16.3 Contrast audit** of the default theme's text colors (egui
  dark theme + any custom styles in `app/src/shell.rs`). Compute
  ratios; bump to ≥ 7:1 (AAA) where the theme allows.
- **16.4 Keyboard map**: verify every tool and menu action is
  reachable by keyboard; document shortcuts in a help panel. AAA
  2.1.3 requires no keyboard trap.

### Phase 17 — Release engineering (from HANDOFF Phase 13)

- **17.1 Platform matrix recipes** (13.2, still open): run the
  AppImage/Windows/macOS scripts on real hardware as hardware
  becomes available; keep the honest "built on Linux x86_64" line
  until then.
- **17.2 Release automation**: a GitForge pipeline job that, on tag
  push, runs `xtask dist --web --native` and uploads artifacts —
  removes the manual gh release step from the runbook.
- **17.3 Smoke budget off measured load** (13.4): the bridge smoke
  uses `--virtual-time-budget=60000`; derive the budget from the
  measured wasm load time × margin, not a constant.

### Phase 18 — Documentation coverage

- **18.1 Update NEXT.md's stale facts as they change** (this file —
  the io/text "zero tests" claims were stale: io has 106 tests
  across 8 integration files).
- **18.2 Historical ticket notes**: `docs/planning/firecraft/M*.md`
  are closed milestones — add a banner note so agents don't treat
  them as open.
- **18.3 TOOLCHAINS.md + RELEASING.md accuracy pass** against the
  v0.2.3 reality (the 8-stamp bump list changed with web/Cargo.toml).
- **18.4 CLAUDE.md project status line** — update the "Status"
  paragraph per release (currently says v0.2.1).

### Parked (evidence-gated, unchanged)

- Phase 12 holds (effect growth, TXFv texture) — need the Wine/CS6
  oracle. No action without it.
- 11.2/11.3 AVIF/HEIC — decided (0064/0065); revisit only when the
  `image` crate's AVIF state changes.

## Suggested order

14.1 → 14.2 → 14.5 (ratchet) → 15.1/15.3 → 16.1/16.3 (cheap wins) →
16.2/16.4 → 18.x → 14.3/14.4 → 17.2 → 17.1/17.3.

## What NOT to do

- Do not blanket-enable pedantic clippy (1,825 hits, mostly raster
  casts — the noise would bury real findings).
- Do not bless coverage floors above the measured number (a lying
  gate is worse than a low one).
- Do not fake WCAG conformance — record honest refusals per criterion.
- Do not run `cargo test --workspace` locally (hook-denied; use
  GitForge or per-crate commands).
- Do not touch `source/`, do not commit `dist/`, do not force-push.
- Do not add aegis baseline entries without verifying each finding is
  a false positive ("pan" in pan/zoom prose trips PCI; 65535-style
  literals trip zip-code — those two are the known classes).

## Quick reference

| Gate | Command | Timeout |
|---|---|---|
| Full CI | `cargo run -p xtask -- ci` | ~20 min |
| Aegis | `cargo run -p xtask -- aegis` | ~10s |
| Coverage measure | `cargo llvm-cov --summary-only` (hook: use GitForge) | ~6 min |
| Coverage gate | `cargo run -p xtask -- coverage` | ~6 min |
| Wasm only | `cargo run -p xtask -- wasm` | ~5 min |
| Web clippy | `cd web && cargo clippy --all-targets -- -D warnings` | ~2 min |
| Dist | `cargo run -p xtask -- dist --native` + `-- --web` | ~10 min |

| Fact | Value |
|---|---|
| Live site | <https://firecraft-web.pages.dev> (v0.2.3) |
| Latest release | v0.2.3 (2026-10-24) |
| Coverage floors | 80% lines / 77% functions (rising-only) |
| CI image | `firecraft-ci-rust:1` — rebuild + re-ship when ANY lockfile moves |
| GitForge API | 127.0.0.1:42780, bearer via `~/.config/gitforge/mkinney-credentials.env` |
