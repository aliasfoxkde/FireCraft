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

## Worst coverage files (missed lines, refreshed 2026-10-29 from lcov)

| File | Missed | What's untested |
|---|---|---|
| crates/app/src/shell.rs | ~2741 | UI shell (menus, panels, interaction glue) — largest single gap; not in the original plan |
| crates/app/src/editor.rs | — | 14.1 landed 30 tests; residual misses folded into the shell.rs work |
| xtask/src/corpus.rs | 1245 | xtask tooling (gated, lower priority than app code) |
| xtask/src/dist.rs | 519 | dist packaging paths (partially covered by CI) |
| crates/app-cli/src/main.rs | 387 | CLI arg paths |
| crates/compose/src/effects.rs | ~360 | 14.3 filter-stack edge parameters |
| crates/engine/src/catalog/mask.rs | ~319 | 14.2 refusal branches |
| crates/engine/src/catalog/object.rs | ~317 | 14.2 refusal branches (down from 483 after earlier batteries) |
| crates/io/src/psd.rs | ~180 | 14.4 PSD error paths |

## Phases

### Phase 14 — Coverage ratchet to 99% (multi-session)

The floors were raised 60% → 80% lines / 77% functions (measured
baseline, 2026-10-25). The gate is rising-only from here.
**2026-10-26: 81.1/78.8 measured, floors 81.0/78.5; 14.1 landed.**

- **14.1 Editor surface tests** — LANDED 2026-10-26 (`cbfe7eb`, 30
  tests). Remaining sub-items for a later pass: `import_styles` good-
  mkTS path, `replay_steps`, `wire` string table, symbol helpers,
  nine-slice toggles (the landed battery covers strokes, selections,
  drags, guide drags, library round-trips, refusal paths).
- **14.2 Catalog refusal branches** (mask ~319, object ~317, path,
  text, bitmap, web — numbers refreshed 2026-10-29 in the table
  above). Each command's error rows are
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
- **14.6 Coverage of the `web/` crate**: CORRECTED 2026-10-29 —
  the premise was wrong. `web/` is a workspace member (root
  `Cargo.toml` `members` since 3295ef0, 2026-10-10), so the
  workspace-wide `cargo llvm-cov` measure already includes it. No
  separate manifest-path measure is needed; audit the lcov for
  `web/src` entries if its share ever matters.

### Phase 15 — Lint hardening

Current deny set: unwrap_used, expect_used, panic, todo,
unimplemented, unreachable, dbg_macro, print_stdout, exit +
unsafe_code=forbid. Production code is already clean.

- **15.1 Curated pedantic adoption.** Blanket pedantic is 1,825 hits,
  86% `cast_possible_truncation` (raster math does i64↔u32↔f64
  constantly). Do NOT blanket-enable. Instead enable per-lint:
  - `clippy::doc_markdown` — DONE 2026-10-26 (`173f05b`, ~250 docs
    fixed, denied workspace-wide; xtask's share rode `4486f5f`).
  - `clippy::must_use_candidate` — DONE 2026-10-26 (same commit;
    ~44 real accessors annotated).
  - `clippy::needless_pass_by_value` — DONE 2026-10-26 (`196277c`,
    ~11 real sites; justified allows on the MCP reply builders).
  - `clippy::cast_possible_truncation` → REFUSED per the density
    rule below (allow density would far exceed 1/100 lines in the
    raster crates).
- **15.2 web/ clippy in the gate** — DONE 2026-10-25. Note
  (corrected 2026-10-29): since `web/` is a workspace member, the
  workspace clippy sweep already covers it; the dedicated web step
  in `.gitforge.yml` is belt-and-braces (kept — it fails fast with
  a shorter log).
- **15.3 `.gitforge.yml`: add the web clippy step** to the
  clippy-test job (`cd web && cargo clippy --all-targets -- -D
  warnings`). DONE — but note (2026-10-29): it is redundant with
  the workspace sweep (web/ is a member); it stays as a fast-fail
  step, not as the only web coverage.
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
- **17.2 Release automation** — DONE 2026-10-26 (`5d83ed3`): a
  `release` job chains after generated-docs; on a tag push it builds
  web + native dist (bridge smoke skipped — the CI image bakes no
  chromium, recorded as image debt via `FIRECRAFT_DIST_SKIP_SMOKE`),
  verifies manifests, and stages into `<workspace>/artifacts/` for
  the runner's artifact collection. Note: GitForge stamps tag pushes
  as `TriggerType::Push` — the tag gate is in-step
  (`git describe --exact-match`). gh-release upload remains manual
  (operator's signing ceremony).
- ~~**17.3 Smoke budget off measured load** (13.4)~~ **Decided
  (0066): no change.** Virtual time in headless chromium covers only
  JS execution — wasm fetch/compile run on wall clock, outside the
  budget — so deriving the budget from load time addresses the wrong
  quantity. The real load budget is the wasm size ceiling
  (`enforce_wasm_budget`); the 60000 virtual-time ceiling stays.

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

## The daily-driver expansion (2026-10-31)

A judge-panel audit against Fireworks CS6 and Photopea (screenshots in
`/var/tmp/fc-audit`, method: headless Playwright interaction + vision
judges) found the web build far below the bar, and the root cause is
structural: **the engine has 243 registered commands but the shell
binds only 91 (37%)** — 152 commands are engine-complete with zero UI,
and 181 of 295 menu items render `Action::Inert`. The owner's bar is a
daily-driver Fireworks CS6 replacement; reaching it is a
reachability-and-canvas program, not a repair program.

Round-1 fixes landed 2026-10-31 (this branch): web CS6 theme gap
(`apply_cs6_theme` was desktop-only — the deployed build rendered
egui's default light chrome), rubber band no longer paints during
object moves, selection bounding box + CS6 handles, mid-gesture
repaint, page drop shadow, image import places into a drawing instead
of replacing it, fill/stroke swatch panel bound to
`object.setFill`/`setStroke`.

### Phase 19 — Canvas competence (P0, feeds usability directly)

- **19.1 Transform handles** (L): corner/edge handles scale/rotate/skew
  live with preview over `object.transform`/`scale`/`skew`; Numeric
  Transform dialog. Accept: handle drag is one undo step with live
  preview.
- **19.2 Path node editing** (M–L): Subselect draws anchors/handles;
  drag via `path.movePoints`; double-click adds/deletes. Accept: node
  drag round-trips save/open.
- **19.3 Gradient fill editor** (M): stops + on-canvas handles over
  `path.setFill`/`path.setFillVector`. Accept: linear gradient drawn,
  edited, survives save.
- **19.4 Command palette** (S): Ctrl+P over `engine::catalog::ids()`
  with payload templates — makes all 152 unreachable commands
  executable immediately. Accept: 100% registry coverage, testable like
  `the_call_table_covers_the_catalog`.
- **19.5 Guides + grid + snapping** (M): `page.addGuide`, drag-out
  guides, snap-to on object drag.
- **19.6 Inert-menu burn-down** (L, multi-session): family batches in
  GAPS order (selection/mask → page/frame → symbol/animation → text);
  every wired item proven by a shell test through the journal.

### Phase 20 — Daily-driver features (P1)

- **20.1 In-canvas text editing** (L): egui TextEdit overlay at the
  text frame; budget kittest focus-regression tests.
- **20.2 Optimize panel** (M): palette/bitrate over `ExportOptions` +
  live preview (export preview exists).
- **20.3 Animation timeline** (M): frames strip, per-frame timing,
  onion skinning, `symbol.tween`.
- **20.4 Mask stack UI** (M): add/remove/mode/link/move.
- **20.5 Multi-document tabs + Window-menu panel toggles** (M).
- **20.6 History panel click-to-rewind** (S): journal replay to seq N.
- **20.7 Slice/hotspot behavior editing + HTML preview** (M).

### Phase 21 — Modern differentiators (P2)

- **21.1 Autosave + crash recovery** (M): periodic journal snapshot to
  a sidecar; restore prompt on open.
- **21.2 Zoom-aware/HiDPI rendering** (L): re-render vectors at
  `zoom × pixels_per_point`; extend `compose::cache::EffectCache` to
  view transforms. The single-texture blit is blurry above 100%.
- **21.3 Command palette v2** (S after 19.4): fuzzy, recents,
  save-steps-as-command.
- **21.4 Preferences dialog + `pixels_per_point`** (S), persisting via
  platform storage (decision 0067 follow-up).
- **21.5 accesskit** desktop + web re-audit (0067 records the web
  blocker).
- **21.6 In-app scripting** (S): Commands menu → run `.jsf`/`.fwsteps`.

### Quality gates for UI work (new)

1. **egui_kittest snapshots** per panel/toolbar — theme and WCAG
   regressions become mechanical.
2. **Reachability gate** in xtask: the set difference
   `catalog::ids()` − UI-bound commands only shrinks (mirror of the
   call-table test). 63% unreachable with no gate means drift.
3. **Inert-drift test**: every non-Help/Command `Action::Inert` item
   carries a tracking ticket id in its doc comment.
4. **Wasm budget**: keep `WASM_SIZE_CEILING` honest — the 12.78 MB
   current size leaves ~1.2 MB headroom against the 14 MB ceiling;
   panel work must fit or re-pin with evidence.
5. **Render-stamp migration** when 21.2 lands: the undo/redo/journal
   tuple-stamp misses view-dependent re-renders — move to a dirty
   counter in `Editor`.
6. **shell.rs split**: 5.7k lines; split panels into modules during
   19.1, before more canvases accrete.

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
