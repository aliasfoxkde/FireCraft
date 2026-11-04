# FireCraft — Next-agent plan (2026-11-01)

> Start at `docs/HANDOFF.md` first. This file is the live backlog
> and the expanded plan. HANDOFF §4 has the phase descriptions.
> Rewritten 2026-10-25 from a full audit: coverage measured at
> 80.47% lines / 77.85% functions, production code panic-free,
> clippy zero-warning, v0.2.3 shipped. Phase 19–21 (the daily-driver
> expansion) appended 2026-10-31; 19.1–19.5 landed 10-31 → 11-01.

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
| crates/app/src/shell/ | ~2741 (pre-split) | UI shell (menus, panels, interaction glue) — largest single gap; not in the original plan. Now split into `shell/{mod,glyphs,filters_ui,dialogs,panels,canvas_ui}` — re-measure the per-module split |
| crates/app/src/editor.rs | — | 14.1 landed 30 tests; residual misses folded into the shell work |
| xtask/src/corpus.rs | 1245 | xtask tooling (gated, lower priority than app code) |
| xtask/src/dist.rs | 519 | dist packaging paths (partially covered by CI) |
| crates/app-cli/src/main.rs | 387 | CLI arg paths |
| crates/compose/src/effects.rs | ~360 | 14.3 filter-stack edge parameters |
| crates/engine/src/catalog/mask.rs | ~319 | 14.2 refusal branches |
| crates/engine/src/catalog/object.rs | ~317 | 14.2 refusal branches (down from 483 after earlier batteries) |
| crates/io/src/psd.rs | ~180 | 14.4 PSD error paths |

## Phases

### Phase 14 — Coverage ratchet to 99% (multi-session)

The floors were raised 60% → 81.1% lines / 78.7% functions (measured
baseline, 2026-10-26). The gate is rising-only from here.
**2026-10-26: 81.1/78.8 measured, floors 81.1/78.7; 14.1 landed.**

- **14.1 Editor surface tests** — LANDED 2026-10-26 (`cbfe7eb`, 30
  tests). The listed residuals are all covered since: `import_styles`
  good path (crates/app/tests/styles.rs battery), `replay_steps`
  (editor.rs:4228 + styles.rs), nine-slice toggles (symbols.rs +
  editor_surface.rs). **CLOSED 2026-11-02** — no residual sub-items.
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
  Small; fix in place. — **DONE 2026-10-25** (decision 0067): the
  host page carries lang, title, description and canvas fallback;
  recorded in `docs/decisions/0067-wcag-aaa-web-host.md`.
- **16.2 egui/accesskit audit**: verify accesskit is compiled in on
  the wasm build (egui feature flag), test with a screen reader on
  the deployed site, document what is reachable. Record results in a
  decision doc — some AAA criteria (1.4.8 visual presentation,
  2.1.3 keyboard-no-exception) may be honest refusals for a
  canvas-drawing app; document them as such rather than fake conformance.
  — **DONE as an honest refusal (2026-10-25, decision 0067)**: the
  eframe web build discards the accesskit tree, so a screen-reader
  pass over the canvas is not available; 1.4.8 and 2.1.3 are recorded
  as refusals, not conformance. Nothing further is actionable here.
- **16.3 Contrast audit** of the default theme's text colors (egui
  dark theme + any custom styles in `app/src/shell/`). Compute
  ratios; bump to ≥ 7:1 (AAA) where the theme allows. — **DONE
  2026-10-25** (decision 0067): the theme's text/background pairs
  were raised to ≥ 7:1.
- **16.4 Keyboard map**: verify every tool and menu action is
  reachable by keyboard; document shortcuts in a help panel. AAA
  2.1.3 requires no keyboard trap. — **DONE**: the shortcut help
  panel (F1 → Help ▸ Keyboard Shortcuts, `crates/app/src/shell/
  panels.rs`) lists every binding in the `SHORTCUTS` table.

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
  them as open. — **DONE**: all 16 milestone tickets carry the
  "CLOSED milestone (historical reference)" banner.
- **18.3 TOOLCHAINS.md + RELEASING.md accuracy pass** against the
  v0.2.3 reality (the 8-stamp bump list changed with web/Cargo.toml).
  — **DONE**: both files describe the stamp set (workspace
  `[workspace.package] version` + `packaging/deb/control`) and the
  dist/release flow; `xtask changelog` enforces one stamp across all
  workspace-local packages.
- **18.4 CLAUDE.md project status line** — update the "Status"
  paragraph per release (currently says v0.2.1). — **DONE 2026-11-02**
  (v0.2.4).

### Parked (evidence-gated, unchanged)

- Phase 12 holds (effect growth, TXFv texture) — need the Wine/CS6
  oracle. No action without it.
- 11.2/11.3 AVIF/HEIC — decided (0064/0065); revisit only when the
  `image` crate's AVIF state changes.

## Suggested order

**Current front:** 19.6 (inert burn-down) → 19b.2/19b.3 (reachability
+ inert-drift gates) → 19b.1 (path-node add/delete) → 19b.4 (wasm
budget re-pin) → 20.1 (in-canvas text) → 20.4 (mask stack) → 20.3
(animation timeline) → 21.x.

**Quality-ratchet backlog (14.2–14.5):** 14.2 → 14.5 (ratchet) →
15.1/15.3 → 16.1/16.3 (cheap wins) → 16.2/16.4 → 18.x → 14.3/14.4 →
17.2 → 17.1/17.3.

## The daily-driver expansion (2026-10-31)

A judge-panel audit against Fireworks CS6 and Photopea (screenshots in
`/var/tmp/fc-audit`, method: headless Playwright interaction + vision
judges) found the web build far below the bar, and the root cause is
structural: **the engine has 243 registered commands but the shell
binds only 91 (37%)** — 152 commands are engine-complete with zero UI,
and 181 of 295 menu items rendered `Action::Inert` at audit time (144
of 293 as of 2026-11-02; the census test in `menus.rs` is the live
authority). The owner's bar is a
daily-driver Fireworks CS6 replacement; reaching it is a
reachability-and-canvas program, not a repair program.

Round-1 fixes landed 2026-10-31 (this branch): web CS6 theme gap
(`apply_cs6_theme` was desktop-only — the deployed build rendered
egui's default light chrome), rubber band no longer paints during
object moves, selection bounding box + CS6 handles, mid-gesture
repaint, page drop shadow, image import places into a drawing instead
of replacing it, fill/stroke swatch panel bound to
`object.setFill`/`setStroke`.

Phase 19 batches 19.1–19.5 landed 2026-10-31 → 11-01: transform
handles + Numeric Transform, Subselect path-node editing, the gradient
fill editor, the command palette, and the grid/guides/snap set. The
inert count is 42 of 289 items (was 190); 19.6 is the remaining
burn-down.

### Phase 19 — Canvas competence (P0, feeds usability directly)

- **19.1 Transform handles** (L): corner/edge handles scale/rotate/skew
  live with preview over `object.transform`/`scale`/`skew`; Numeric
  Transform dialog. Accept: handle drag is one undo step with live
  preview. — **LANDED 2026-10-31** (`7329087`).
- **19.2 Path node editing** (M–L): Subselect draws anchors/handles;
  drag via `path.movePoints`; double-click adds/deletes. Accept: node
  drag round-trips save/open. — **LANDED 2026-10-31** (`5e4f0ce`):
  anchors + handles drawn, drag folded into one "Reshape" undo step;
  the double-click add/delete half is **still open**.
- **19.3 Gradient fill editor** (M): stops + on-canvas handles over
  `path.setFill`/`path.setFillVector`. Accept: linear gradient drawn,
  edited, survives save. — **LANDED 2026-10-31** (`1c84ee8`).
- **19.4 Command palette** (S): Ctrl+P over `engine::catalog::ids()`
  with payload templates — makes all 152 unreachable commands
  executable immediately. Accept: 100% registry coverage, testable like
  `the_call_table_covers_the_catalog`. — **LANDED 2026-10-31**
  (`4458f3a`); `palette_covers_every_catalog_command` pins the
  coverage.
- **19.5 Guides + grid + snapping** (M): `page.addGuide`, drag-out
  guides, snap-to on object drag. — **LANDED 2026-11-01** (`eb221ac`):
  grid overlay, guide drag-out, per-axis snap, Edit Grid dialog.
- **19.6 Inert-menu burn-down** (L, multi-session): family batches in
  GAPS order (selection/mask → page/frame → symbol/animation → text);
  every wired item proven by a shell test through the journal. — **IN
  FLIGHT**: 190 → 178 (19.1) → 171 (19.5) → 144 (19.6 batch 1,
  selection/mask) → 129 (19b.1 + batch 2 — canvas dialogs,
  Duplicate/Clone; 2026-11-03) → 120 (batch 3 — Window menu: Hide
  Panels/Tab, Main toolbar, and 7 docked-panel toggles; 2026-11-03) →
  **114 inert of 289 items** (batch 4 — page/frame: Insert ▸ State
  via `frame.add`, Trim/Fit Canvas via the extents verbs, canvas
  Rotate 180/90CW/90CCW via `document.rotate` through the new
  `Action::Document` arm; 2026-11-03) → **111 inert of 289 items**
  (batch 5 — symbol/animation: Convert to Symbol via `symbol.create`,
  Animate Selection via `symbol.create` with the animation type,
  Tween Instances via `symbol.tween`; Edit Symbol/Edit in Place/
  Swap Symbol/Remove Animation stay inert — no engine verbs yet;
  2026-11-03) → **99 inert of 289 items** (batch 6 — View menu:
  Fit Selection, Gamma 1.8, Tooltips, Show All, Rulers, Smart Guides
  (show + snap), 9-Slice Lock Guides, Slice Guides, Slice Overlay,
  Edges; 2026-11-03) → **85 inert of 289 items** (batch 7 — Text
  menu: Plain, the four horizontal Align rows over `JSTi`, Smaller/
  Larger over the size presets, the four Orientation rows over
  `TAMi`, Detach from Path, Reverse Direction, Convert to Paths;
  2026-11-03) → **83 inert of 289 items** (batch 8 — Attach In Path/
  Attach to Path over `text.attachPath`: the selection must carry one
  text and one path, the path's contours become the rail; missing
  either half reports instead of commanding. The vertical-align rows
  (Top/Centered Vertically/Bottom/Justified Vertically) are
  evidence-gated — the dictionary documents no text
  vertical-justification node, and wiring them would invent model
  state; 2026-11-03) → **79 inert of 289 items** (batch 9 — Modify ▸
  Transform: Free Transform and Distort set the handle mode
  (`TransformMode`; Distort drags one corner independently via
  `object.setQuadrangle`, the other three corners frozen at drag
  start), Split via `path.splitPaths`, Remove Transformations via
  `object.transform` with each stored `MTXv`'s inverse; Group as
  Mask/Repeat Filter/fill-edge rows stay inert — no engine verbs yet;
  2026-11-03) → **72 inert of 289 items** (batch 10 — Window family:
  Tools folds the left dock, Color Mixer/Swatches/Info dock as
  toggled right-dock panels (Swatches = the ABGR palette as a grid
  over fill, Info = canvas/selection facts), Duplicate Window/
  Cascade/Tile report the single-document shell honestly instead of
  sitting inert; Optimize/Common Library/CSS Properties/Auto
  Shapes/URL/Behaviors/Find and Replace stay inert until their
  features land; 2026-11-03) → **68 inert of 289 items** (batch 11 —
  Edit ▸ Insert: Empty Bitmap over `bitmap.addImage` spanning the
  canvas on the active layer; Bitmap Via Copy / Via Cut over
  `bitmap.addImageViaCopy`/`Cut` on a selected bitmap (Cut clears the
  source per the fresh-tile law); Crop Document reuses
  `document.setDocumentCanvasSizeToDocumentExtents` with `grow`
  false; 2026-11-03) → **67 inert of 289 items** (batch 12 — Modify ▸
  Animation ▸ Remove Animation dissolves the selected symbol instance
  via `symbol.breakApart`; no instance in the selection reports
  instead of commanding)
  2026-11-03) → **59 inert of 289 items** (batch 13 — the File family:
  Close / Close All start a fresh document (refusing to discard
  unsaved edits — CS6 prompts, this build does not yet), Save All
  saves the one open document, Revert reloads the file the document
  was last saved to, File Info reports pages/layers/objects/canvas,
  Export Wizard and the Send to E-mail format rows open the export
  dialog on that format with the e-mail delivery gap stated plainly)
  → **42 inert of 289 items** (batch 14 — the File remainder: the
  Preview-in-Browser rows actually export PNG(s) — next to the saved
  document or in the temp dir — and open them via the system handler
  (All Pages exports one PNG per page); Bridge/TWAIN/templates/
  browser-configuration/printing/HTML round-trip/batch-process/recent
  files report their gap plainly via `ShellAction::ReportGap` instead
  of sitting inert). Next batch: the remaining Modify rows and the
  Text remainder.

### Phase 19b — Follow-ups from the landed batches

- **19b.1 Path-node add/delete** (S): the double-click half of 19.2 —
  add an anchor on a segment, delete a selected anchor.
- **19b.2 Reachability gate** (S): the set difference
  `catalog::ids()` − UI-bound commands may only shrink (a test, not an
  xtask step). Without it the 63%-unreachable number drifts silently.
  — **LANDED 2026-11-02**: the catalog side is pinned by
  `palette_covers_every_catalog_command` (every registry command is
  executable from the palette); the shell side is pinned by
  `menus::tests::inert_count_never_rises`, which derives the census
  from the menu table instead of quoting it.
- **19b.3 Inert-drift test** (S): every non-Help/Command `Action::Inert`
  item carries a tracking reference, so an inert item cannot be added
  silently. — **LANDED 2026-11-02** in the stronger form: the census
  is derived from `MENUBAR` and gated against a ceiling
  (`INERT_CEILING`), so a newly inert row fails the test until the
  ceiling is deliberately re-pinned. Per-item tracking references were
  dropped as unnecessary — the count is the honest metric and the gate
  refuses any rise in it.
- **19b.4 Wasm budget re-pin** (S) — **DONE (2026-11-03)**: re-measured
  and re-pinned with evidence. `xtask dist --web` observed
  **21,235,599 B** (trunk --release, wasm-opt 2, rustc 1.99, wasm-opt
  132); ceiling moved 14,000,000 → 21,500,000 in `xtask/src/dist.rs:30`
  with the dated cause recorded in the constant's doc comment (Phase 19
  shell surface: Lucide icons, command palette, gradient editor, image
  support — 10.87 MB at M12, 15.25 MB at 2026-11-02). Ceiling only
  moves down from here.

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
  view transforms. The single-texture blit is blurry above 100%. —
  **PARTLY LANDED 2026-11-01** (`6e35c1e`): `compose::render_scaled`
  renders the page under an affine scale and the shell caches it per
  zoom bucket (re-render past 2× drift, 16 MPix budget), so the canvas
  is crisp past 100%. **Still open**: `pixels_per_point` (HiDPI)
  plumbing — the scale is zoom-only today.
- **21.3 Command palette v2** (S after 19.4): fuzzy, recents,
  save-steps-as-command. — **PARTLY LANDED 2026-10-31** (`4458f3a`):
  the palette filters by substring (case-insensitive). Still open:
  fuzzy scoring, recents, save-steps-as-command.
- **21.4 Preferences dialog + `pixels_per_point`** (S), persisting via
  platform storage (decision 0067 follow-up).
- **21.5 accesskit** desktop + web re-audit (0067 records the web
  blocker).
- **21.6 In-app scripting** (S): Commands menu → run `.jsf`/`.fwsteps`.

### Phase 22 — CS6 shell fidelity (P0, the "looks and acts like Fireworks" phase)

Set 2026-11-03 by user directive: the shell must present as CS6 — same
dockable panel system, same panel inventory, same tool-strip look,
same adaptive context menus — not FireCraft's own chrome. CS6 was
abandoned in 2013; the point of this phase is that the abandoned app
is usable again, with modern capabilities underneath.

**Reference material (clean-room boundary).** Behavior comes from the
CS6 install's own `Configuration/` Extendscript specs, the exe's
string tables (e.g. the `string:contextmenu.*` verb family: 18
panel/document/tab/drawer verbs), and observed UI behavior. Adobe's
*assets* (icons, bitmaps, panel artwork) are **not** copied — they
are replaced with open-license equivalents per the icon survey
(`docs/planning/generated/icon-resources-survey.md`) with
`docs/ATTRIBUTION.md` rows in the same change. The CS6 install cannot
be launched as an oracle: under Wine it fails its own configuration
check ("Error: 1"), and the activation machinery is explicitly out of
scope, so no license path is attempted.

- **22.1 CS6 panel inventory + docking** (L): the left tool dock and
  the right panel stack with the CS6 panel set (Properties, Layers,
  Pages, Swatches, Color Mixer, Info, History, Optimize, Symbols,
  Styles, Common Library, CSS Properties, Auto Shapes, URL,
  Behaviors, Find and Replace, Slices). Panels are dockable left or
  right, collapsible, resizable, and re-openable from the Window
  menu; the dock geometry persists. Accept: a panel can be moved
  left↔right, collapsed, and reopened, and the layout survives a
  restart. — **PARTLY LANDED 2026-11-03**: the panel stack is now
  one window per panel (`shell::panel_windows`) — draggable,
  collapsible, resizable, closable, docked by default in the
  right-hand column (`default_pos`, so a drag moves a panel free and
  egui's persisted area memory keeps the new spot), with CS6's
  fresh-document roster (Properties/Pages/States/Layers open, the
  rest closed until asked for) and the Window menu toggling each.
  The layout now persists across restarts (`eframe`'s `persistence`
  feature): a `ShellLayout` snapshot (panel set, dock folds, display
  toggles) saved through `App::save` — native to `storage_dir`, web
  to localStorage — and restored at startup, filtered against the
  current panel inventory. Still open: the panels this build does
  not implement yet (Optimize, Common Library, CSS Properties, Auto
  Shapes, URL, Behaviors, Find and Replace, Slices).
- **22.2 CS6 tool strip** (M): the CS6 tool set in CS6 order with
  tool flyouts (shape tools, the modify sub-tools, the fill/stroke
  pair, zoom/hand, and the eyedropper), each with an open-license
  icon. Accept: every tool in the strip selects a working tool; the
  flyout opens on press-and-hold.
- **22.3 Adaptive canvas context menu** (M): the right-click menu
  changes with the selection — nothing selected, a bitmap, a vector
  path, a text object, an instance, a multi-selection, and inside a
  slice. Each variant is a CS6 verb subset over the same engine
  commands the menus use. Accept: right-clicking each selection
  class yields that class's verbs, and every one of them executes. —
  **LANDED 2026-11-03** (`shell::canvas_ui::context_rows`): the popup
  classifies the live selection (empty / bitmap / path / text /
  instance / slice / group / multi) and offers that class's verb set;
  no row is an inert placeholder.
- **22.4 Property inspector** (L): the Properties panel shows the
  selection's real attributes (fill, stroke, stroke style, effects,
  text attributes) and edits them through the engine — not a
  read-only summary. — **PARTLY LANDED 2026-11-03**: the Properties
  panel edits opacity, blend, visibility, lock, name, text runs and
  font attributes, and the live effect list; the CS6 X/Y/W/H
  geometry strip now applies through `object.setBounds`. Still open:
  the fill/stroke editors for bitmaps, and the CS6 inspector's
  per-kind field sets.
- **22.5 CS6 chrome** (M): menu bar order, tool strip placement,
  document tab strip, status bar, panel headers with the CS6
  collapse/close affordances, and the CS6 color scheme (a dark
  neutral chrome, not egui's default light look).
- **22.6 Icon pack integration** (M): ship the surveyed open-license
  icon set, subset it to the Fireworks verb set, and record the
  attribution rows. Accept: the wasm bundle grows by a documented,
  measured amount and stays under the pinned ceiling.

### Quality gates for UI work (new)

1. **egui_kittest snapshots** per panel/toolbar — theme and WCAG
   regressions become mechanical.
2. **Reachability gate** in xtask: the set difference
   `catalog::ids()` − UI-bound commands only shrinks (mirror of the
   call-table test). 63% unreachable with no gate means drift.
   — **open, tracked as 19b.2.**
3. **Inert-drift test**: every non-Help/Command `Action::Inert` item
   carries a tracking ticket id in its doc comment.
   — **open, tracked as 19b.3.**
4. **Wasm budget**: keep `WASM_SIZE_CEILING` honest — the 12.78 MB
   current size leaves ~1.2 MB headroom against the 14 MB ceiling;
   panel work must fit or re-pin with evidence.
   — **open, tracked as 19b.4** (the figure predates Phase 19).
5. **Render-stamp migration** when 21.2 lands: the undo/redo/journal
   tuple-stamp misses view-dependent re-renders — move to a dirty
   counter in `Editor`.
6. **shell.rs split**: 5.7k lines; split panels into modules during
   19.1, before more canvases accrete. — **DONE** (`32a110d`,
   `shell.rs` → `shell/{mod,glyphs,filters_ui,dialogs,panels,canvas_ui}`).

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
| Live site | <https://firecraft-web.pages.dev> (v0.2.5 pending deploy) |
| Latest release | v0.2.5 (2026-11-03) |
| Coverage floors | 81.1% lines / 78.7% functions (rising-only; `xtask/src/coverage.rs`) |
| CI image | `firecraft-ci-rust:1` — rebuild + re-ship when ANY lockfile moves |
| GitForge API | 127.0.0.1:42780, bearer via `~/.config/gitforge/mkinney-credentials.env` |
