# FireCraft — Roadmap detail

> Current task state is tracked in [`TASKS.md`](TASKS.md), with a dated
> snapshot in [`PROGRESS.md`](PROGRESS.md). This file is the detailed
> phase plan, not a second live ledger.
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
  web + native dist, **smokes the web bundle in a headless chromium
  baked into the CI image** (the bridge round trip must report
  `green` — a missing browser or a failed round trip fails the job),
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

**Current front:** Phase 22 shell fidelity and the screenshot-driven
responsive audit. Phases 19 and 19b are complete; Phase 20 and 21
features are queued in [`TASKS.md`](TASKS.md).

**Quality-ratchet backlog (14.2–14.5):** 14.2 → 14.5 (ratchet) →
15.1/15.3 → 16.1/16.3 (cheap wins) → 16.2/16.4 → 18.x → 14.3/14.4 →
17.2 → 17.1/17.3.

## Audit context: the daily-driver expansion (2026-10-31)

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
inert count is 2 of 296 items (was 190); 19.6's menu-wiring

campaign is complete and the Text ▸ Font submenu landed 2026-11-06. The remaining rows require separate feature or
evidence work, tracked outside that campaign.

### Phase 19 — Canvas competence (P0, feeds usability directly)

- **19.1 Transform handles** (L): corner/edge handles scale/rotate/skew
  live with preview over `object.transform`/`scale`/`skew`; Numeric
  Transform dialog. Accept: handle drag is one undo step with live
  preview. — **LANDED 2026-10-31** (`7329087`).
- **19.2 Path node editing** (M–L): Subselect draws anchors/handles;
  drag via `path.movePoints`; double-click adds/deletes. Accept: node
  drag round-trips save/open. — **LANDED 2026-10-31** (`5e4f0ce`):
  anchors + handles drawn, drag folded into one "Reshape" undo step;
  add/delete completed in 19b.1.
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
- **19.6 Inert-menu burn-down — COMPLETE** (L, multi-session): family batches in
  GAPS order (selection/mask → page/frame → symbol/animation → text);
  every wired item proven by a shell test through the journal. — **DONE**:
  190 → 178 (19.1) → 171 (19.5) → 144 (19.6 batch 1,
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
  Mask/fill-edge rows stay inert — no engine verbs yet (Repeat Filter
  landed 2026-11-06: the session's last preset re-applied);
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
  of sitting inert) → **24 inert of 289 items** (batch 15, 2026-11-04 —
  the Edit remainder + Text rows: Insert ▸ New Button appends a
  button definition via `library.add`; Hotspot and Rectangular Slice
  call `hotspot.create` / `slice.create` over the selection bounds
  (canvas-center rect when nothing is selected); Polygon Slice
  traces the same rect's four corners as a polygon; Find and
  Replace, Copy as Vectors, Copy HTML Code, Paste Attributes, Crop
  Selected Bitmap, Preferences, Keyboard Shortcuts, Text ▸ Size ▸
  Other, the four vertical-align rows (still evidence-gated — no
  documented wire node), and both spelling rows report their gap
  plainly via `ShellAction::ReportGap`) → **11 inert of 289 items**
  (batch 16, 2026-11-04 — the Modify remainder: Edit Symbol / Edit
  in Place enter the selected instance's definition scope
  (`enter_symbol`, view state, no journal row); Hard / Anti-Alias /
  Feather Fill drive `path.setFillEdgeMode` on every selected path
  (feather rides the panel radius); the animation and selective-JPEG
  Settings rows, Swap Symbol, Group as Mask, the three Pop-up Menu
  rows, and Snap To Pixel report their gap via `ReportGap`) → **8 inert
  of 289 items** (batch 17, 2026-11-04 — the CS6 web panels: Auto
  Shapes, URL, and Behaviors become real panels
  (`shell::panels_web`) whose every action is one engine command —
  `object.createPath` / `autoshape.*` for the gallery, `slice.setLink`
  for the URL, `slice.addBehavior` / `slice.removeBehavior` /
  `slice.setData` for the behaviors; decision 0069). The remaining
  inert rows: Commands ▸ Manage Saved Commands (the scripting
  family) and Window ▸ Extensions, plus the Window panels that land
  with their features (Phases 20–22). The Text Font placeholder and
  Filters ▸ Repeat Filter were wired 2026-11-06 — RT_MENU 128 ships
  Font as a runtime-populated popup, so the static transcription
  carries five renderer-known families through `text.setFont`, and
  Repeat Filter re-applies the session's last preset. 19.6's programmable surface is done;
  the rest is feature-gated, not reachability debt.

### Phase 19b — Follow-ups from the landed batches

- **19b.1 Path-node add/delete** (S): the double-click half of 19.2 —
  add an anchor on a segment, delete a selected anchor.
- **19b.2 Reachability gate** (S): the set difference
  `catalog::ids()` − UI-bound commands may only shrink. Without it the
  unreachable-command number drifts silently.
  — **LANDED 2026-11-02** (partial): the catalog side is pinned by
  `palette_covers_every_catalog_command` (every registry command is
  executable from the palette); the shell side is pinned by
  `menus::tests::inert_count_never_rises`, which derives the census
  from the menu table instead of quoting it. Neither pins the *engine*
  commands the shell's designed surface binds.
  — **LANDED 2026-11-10** (the gate itself): an xtask step,
  `cargo run -p xtask -- reachability`, wired into
  `cargo run -p xtask -- ci`. It derives the bound set from the shell
  crate's own source (the command-id literals it hands the engine,
  `#[cfg(test)]` code excluded), divides it by
  `engine::catalog::ids()`, and fails when the share drops below the
  floor recorded in **`.reachability-floor.tsv`**. The floor is a
  one-way ratchet: `--bless` may raise it (reachability improved) and
  refuses to lower it (a regression is fixed, not forgiven). Measured
  on `main` at 2026-11-11: **246 registry commands, 147 bound by the
  shell = 59.76%**, 99 unreachable; the floor is pinned at 59.76%.
  (The command palette makes the *palette* reach 100% — the gate
  deliberately measures the CS6-visible surface instead, which is the
  surface the 19.x batches widen. Decision
  [`0087`](decisions/0087-ui-work-gates.md).)
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
  support — 10.87 MB at M12, 15.25 MB at 2026-11-02). Re-pinned
  again 2026-11-10 (decision 0079): measured **21,608,249 B** on the
  Phase 21/22 feature surface; the historical observed-size pin was
  22,000,000 B. Decision 0080 now sets a 25,000,000 B warning and
  50,000,000 B hard cap while the production-readiness plan drives
  measured reduction. The cap is not a target; performance timing and
  memory budgets remain separate and falling-only.

### Phase 20 — Daily-driver features (P1)

- **20.1 In-canvas text editing** (L): **LANDED 2026-11-06** — egui
  `TextEdit` overlay at the text frame; Text-tool click or Pointer
  double-click enters, Esc cancels, focus loss / tool switch commits
  ([T20](tasks/T20-text-editing.md)). Kittest focus tests deferred —
  see the ticket's residual note.
- **20.2 Optimize panel** (M): palette/bitrate over `ExportOptions` +
  live preview (export preview exists).
- **20.3 Animation timeline** (M): frames strip, per-frame timing,
  onion skinning, `symbol.tween`.
- **20.4 Mask stack UI** (M): add/remove/mode/link/move. — **PARTIAL
  2026-11-11**: the level's combine mode is modelled, persisted, and
  editable (`mask.setStackMode`, the Properties panel's "Mask stack"
  section; decision [0091](decisions/0091-mask-stack-combine-mode.md),
  ticket [T9](tasks/T9-mask-stack.md)). The operator changes no
  pixels yet — the compositor still folds mask-structured groups
  unmasked — and add/remove/link/move are still open.
- **20.5 Multi-document tabs** (M): **LANDED 2026-11-07** — Open/New
  park the active document as a `DocTab`; Close promotes the newest
  parked tab (refuses dirty documents); the tab strip shows the
  active doc plus parked tabs, click to switch, a dot marks unsaved
  edits. Gate: `open_parks_the_previous_document`,
  `activate_tab_swaps_state`, `close_promotes_the_last_parked_tab`,
  `new_parks_the_current_document`,
  `close_refuses_a_dirty_document`. (Window-menu panel toggles landed
  in Phase 19.6 batches 3 and 10.)
- **20.6 History panel click-to-rewind** (S): journal replay to seq N.
- **20.7 Slice/hotspot behavior editing + HTML preview** (M).

### Phase 21 — Modern differentiators (P2)

- **21.1 Autosave + crash recovery** (M): periodic journal snapshot to
  a sidecar; restore prompt on open. — **LANDED 2026-11-07**, per
  surface: the web build persists the whole document (canonical save
  bytes, base64) into localStorage on every change stamp and
  restores it at boot, clearing on File ▸ New; the desktop build
  snapshots the session journal to a temp-dir sidecar
  (`FIRECRAFT_RECOVERY` overrides, resolved once at construction so
  the tick never reads the environment), offers Recover/Discard at
  startup, replays with undo intact across the crash boundary, and
  clears on a clean save or File ▸ New. Gate:
  `recovery_snapshot_round_trips_through_the_sidecar`.
- **21.2 Zoom-aware/HiDPI rendering** (L): re-render vectors at
  `zoom × pixels_per_point`; extend `compose::cache::EffectCache` to
  view transforms. The single-texture blit is blurry above 100%. —
  **LANDED 2026-11-07** in two steps: `compose::render_scaled`
  renders the page under an affine scale and the shell caches it per
  zoom bucket (re-render past 2× drift, 16 MiPix budget), so the
  canvas is crisp past 100% (`6e35c1e`); the render scale now rides
  `zoom × pixels_per_point`, so the blit is crisp on hidpi displays
  too (this session, `render_scale_rides_pixels_per_point_under_the_budget`).
  The `EffectCache` view-transform extension remains open for
  effect-heavy pages; the canvas path itself is done.
- **21.3 Command palette v2** (S after 19.4): fuzzy, recents,
  save-steps-as-command. — **LANDED 2026-11-07**: substring
  filtering (`4458f3a`), then fuzzy subsequence scoring with a
  position bonus (tighter matches outrank spread-out ones), recents
  (10 most recent successful runs, listed before the catalog on an
  empty filter) persisted in `ShellLayout` with stale-name
  dropping, and save-steps-as-command via the History panel's Save
  Steps…/Replay… (the journal's own `.fwsteps` file). Gates:
  `palette_fuzzy_scores_subsequences_with_position_bonus`,
  `palette_recents_round_trip_through_the_layout`.
- **21.4 Preferences dialog + `pixels_per_point`** (S): **LANDED
  2026-11-07** — Edit ▸ Preferences… opens the dialog; a UI-scale
  drag (0.5×–3×, Reset to platform default) applies to the context
  each frame and rides `ShellLayout`, so it persists like the panel
  set. Gate: `preferences_scale_round_trips_the_layout`. The
  Preferences menu row also left the inert census, now 2 of 294.
- **21.5 accesskit** desktop + web re-audit (0067 records the web
  blocker).
- **21.6 In-app scripting** (S): Commands menu → run `.jsf`/`.fwsteps`.
  — **LANDED 2026-11-07**: `.jsf` running through the scripting host
  had landed with the automation milestone (Commands ▸ Run Script /
  Help ▸ Manage Extensions); this session added the `.fwsteps` arm —
  the Commands picker takes both formats, a steps file replays onto
  the live document through the same door the History panel uses,
  and a malformed journal is refused with the document untouched.
  Gate: `run_script_runs_a_steps_file_through_the_replay_door`.

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

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:42ecf8717bacd73d9fd56477bf4610ea61f6985566267456938b76e4d56354c9 -->

- **22.1 CS6 panel inventory + docking** (L): the left tool dock and
  the right panel stack with the CS6 panel set (Properties, Layers,
  Pages, Swatches, Color Mixer, Info, History, Optimize, Symbols,
  Styles, Common Library, CSS Properties, Auto Shapes, URL,
  Behaviors, Find and Replace, Slices). Panels are dockable left or
  right, collapsible, resizable, and re-openable from the Window
  menu; the dock geometry persists. Accept: a panel can be collapsed
  and reopened, the right dock can be resized, and the layout survives
  a restart. Left↔right movement remains open work toward the CS6 target.
  — **PARTLY LANDED 2026-11-04**: `shell::panel_windows` uses a
  width-reserving `egui::Panel::right` dock (258 pt default width),
  with a scrolling column of collapsible, closable panel sections
  and CS6's
  fresh-document roster (Properties/Pages/States/Layers open, the
  rest closed until asked for) and the Window menu toggling each.
  The layout now persists across restarts (`eframe`'s `persistence`
  feature): a `ShellLayout` snapshot (panel set, dock folds, display
  toggles) saved through `App::save` — native to `storage_dir`, web
  to localStorage — and restored at startup, filtered against the
  current panel inventory. The web panels landed 2026-11-04
  (`shell::panels_web`, decision 0069): Auto Shapes, URL, and
  Behaviors are real panels whose actions are engine commands. The
  remaining panel surfaces (Optimize, Common Library, CSS Properties,
  Find and Replace, Slices) landed 2026-11-06; the panel inventory is
  complete (see `TASKS.md`). Left↔right docking movement remains open.
  The normal-viewport reachability pass is recorded in
  [`tasks/T22-screenshot-ui.md`](tasks/T22-screenshot-ui.md).
  — **REACHABILITY MEASURED 2026-11-04**: the stack is now a dock that
  reserves its width rather than a set of floating windows, and the
  capture round measured it — at 1280×800 every open panel is inside
  the viewport, and at a deliberately short 1280×400 a wheel over the
  dock moves the stack. The receipts, and the limits of the check, are
  in the ticket.

- **22.2 CS6 tool strip** (M): the CS6 tool set in CS6 order with
  tool flyouts (shape tools, the modify sub-tools, the fill/stroke
  pair, zoom/hand, and the eyedropper), each with an open-license
  icon. Accept: every tool in the strip selects a working tool; the
  flyout opens on press-and-hold. — **LANDED 2026-11-05**
  (`app/src/toolstrip.rs`): the strip is CS6's, as data — the six
  categories in CS6's order (Select, Bitmap, Vector, Web, Colors,
  View), the twenty-four groups with their one-key shortcuts, and the
  flyout memberships, all read from the CS6 help set and the book's
  Table 1-1 (evidence in
  [`research/fw-ui-evidence.md`](research/fw-ui-evidence.md)). CS6's
  group-cycling rule is implemented: a group's key advances within
  the group. The Modify trio (Scale/Skew/Distort), the retouch trio
  (Blur/Sharpen/Smudge), Dodge/Burn, Knife, Freeform/Reshape Area and
  the Colors section's four panel buttons are all in the strip. Open:
  the Alt-hold tool variants and the three-mode screen switch
  (ticket 22.5).
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
  geometry strip now applies through `object.setBounds`. — **PER-KIND
  FIELD SETS LANDED 2026-11-04**: the inspector dispatches every
  selection kind. A group shows its display controls, geometry and
  member list; a bitmap shows name, opacity, blend, visibility, lock,
  a pixel-size readout and the effect list; a slice shows its name,
  type (foreground / background / hotspot), frame geometry, and the
  anchor link and alt text — the same editor the Window ▸ URL panel
  renders, so the two surfaces cannot disagree. Still open: the
  bitmap's X/Y/W/H strip (`object.setBounds` refuses bitmaps —
  decision 0019) and any bitmap fill/stroke editor (`IMGv` carries
  no fill/stroke node; CS6's bitmap tools are the separate Image
  Editing command panel), the
  document-level field set, the multi-selection field set, and
  enter-group. — **DOC/MULTI/ENTER LANDED 2026-11-06**: nothing
  selected shows the document field set (canvas W/H through
  `document.setCanvasRect`, resolution + per-cm unit through
  `document.setResolution`, canvas color through
  `document.setCanvasColor` over the shared ABGR palette); a
  2+-object selection shows the multi-selection set (count, union
  bounds, opacity-min/blend-min/visible-AND/locked-OR with every
  edit fanned out per id); a group's member rows select the child
  (enter-group by selection). Test
  `inspector_document_and_multi_selection_sets`. Remaining 22.4:
  the bitmap X/Y/W/H strip and bitmap fill/stroke, both blocked by
  decision 0019 and the absent `IMGv` fill/stroke nodes — recorded,
  not faked.
- **22.5 CS6 chrome** (M): menu bar order, tool strip placement,
  document tab strip, status bar, panel headers with the CS6
  collapse/close affordances, and the CS6 color scheme (a dark
  neutral chrome, not egui's default light look). **Landed
  2026-11-06:** the three-mode screen cycle (Standard / Full Screen
  with Menus / Full Screen, fwref 1178–1185 — the control no longer
  aliases Tab), the Alt-hold retouch swap (Blur↔Sharpen,
  Dodge↔Burn, fwref 3555/3580; Smudge has no variant), and the
  Window-menu arrangement trio (57650/57651/57652 with CS6's
  labels; census 289→290). **Also landed 2026-11-06:** the Text ▸ Font
  submenu (RT_MENU 128 ships a runtime-populated placeholder; the
  transcription carries five renderer-known families through
  `text.setFont`, census 290→294, gated by
  `the_text_font_submenu_dispatches_set_font`). **Repeat Filter landed 2026-11-06** (`Action::RepeatFilter` re-applies
  the session's last preset through `filter.add`, gated by
  `repeat_filter_reapplies_the_last_preset`; inert census 3→2).
  **CS6 keymap landed 2026-11-11** (decision 0080): the transcribed
  tree prints CS6's own accelerators and every live printed
  accelerator is bound to its row's action, held by
  `menus::tests::cs6_keymap_is_fully_bound`; the binding table
  carries its own section per row, gated by
  `menus::tests::shortcut_sections_are_contiguous`. **The residual
  keymap commands landed 2026-11-11**: the 16 bindings that close
  them bring the table to 167 bindings in 40 sections, and every one
  of CS6's 133 keymap-file entries across its 128 command ids now
  resolves to a binding carrying the action this build answers it
  with, held by `menus::tests::cs6_keymap_entries_are_bound` over
  the transcribed `menus::tests::CS6_KEYMAP`. Remaining on 22.5:
  the Main toolbar's contents (CS6-PARITY §A rows 54, 255, 257,
  164).
- **22.6 Icon pack integration** (M): ship the surveyed open-license
  icon set, subset it to the Fireworks verb set, and record the
  attribution rows. Accept: the wasm bundle grows by a documented,
  measured amount and stays under the pinned ceiling.

<!-- /coderabbit-ai-generated:v1 -->

### Quality gates for UI work (new)

1. **Panel snapshots** — theme and WCAG regressions become mechanical.
   — **PARTIAL, DONE for structure (2026-11-10)**: `egui_kittest` was
   checked before the gate was written. It is *obtainable* —
   `egui_kittest` 0.36.2 is on crates.io and requires `egui ^0.36.2`,
   exactly the egui this lockfile pins, and the CI image is not
   offline (`CARGO_NET_OFFLINE=false`; decision 0073 — the baked
   registry cache is a fast floor, not a wall). The blocker is its
   *pixel* path: the snapshot machinery sits behind the crate's
   optional `wgpu` / `egui-wgpu` features, and the CI image installs
   no graphics stack (no mesa, no Vulkan drivers), so a wgpu adapter
   is not available in the sandbox. The panel set is therefore pinned
   the way this repository pins everything else — a committed golden
   file, **`crates/app/panels.snapshot`**, compared against a snapshot
   rendered from the shell's own inventory by the xtask step
   `cargo run -p xtask -- panel-snapshot` (in `ci`; `--bless`
   re-pins). The golden pins the panel *structure* — id, dock side,
   dock order, header text, fresh-document visibility, Window-menu
   togglability — for all **14** panels (13 toggleable + the
   always-present Web Layer). A panel added, renamed, reordered or
   re-docked without a deliberate re-pin fails the build. The pixel
   content of a panel is still unpinned; that half stays open and
   needs `egui_kittest` plus a proven software adapter (lavapipe) in
   the CI image. Decision [`0087`](decisions/0087-ui-work-gates.md).
2. **Reachability gate** in xtask: the set difference
   `catalog::ids()` − UI-bound commands only shrinks (mirror of the
   call-table test). Unreachable commands with no gate means drift.
   — **DONE (2026-11-10)**, tracked as 19b.2: `cargo run -p xtask --
   reachability`, wired into `ci`. Measures **246 registry commands,
   147 bound by the shell = 59.76%**, 99 unreachable; the floor lives
   in **`.reachability-floor.tsv`** (pinned at 59.76%) and is a one-way
   ratchet.
3. **Inert-drift test**: an inert menu row may not be added silently.
   — **DONE (2026-11-02)**, tracked as 19b.3: the census is derived
   from `MENUBAR` and gated against `INERT_CEILING` (8 of 289 items),
   so a newly inert row fails the test until the ceiling is re-pinned.
   Per-item tracking ticket ids were dropped as unnecessary — the
   count is the honest metric and the gate refuses any rise in it.
4. **Wasm budget**: keep `WASM_SIZE_CEILING` honest — panel work must
   fit or re-pin with evidence.
   — **DONE (2026-11-03)**, tracked as 19b.4: measured 21,235,599 B;
   the ceiling moved 14,000,000 → 21,500,000 with the dated cause in
   the constant's doc comment. Ceiling only moves down from here.
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
  literals trip zip-code — those two are the known classes). The
  baseline is `.aegis-baseline.tsv`, keyed on content (pattern, file,
  digest) rather than on line (decision 0074); regenerate it with
  `cargo run -p xtask -- aegis --bless`, which prints every added key
  for review before the rewrite is committed. The rule class of every
  baselined literal is recorded in `xtask/src/aegis.rs` (`CLASS_NOTES`)
  next to the rows it covers, and the evidence for each is in decision
  0074, which supersedes the rule-class suppression of decision 0070;
  a new class needs the same kind of evidence, not a convenience.
- Do not switch off a whole aegis rule class to silence a finding.
  The class-level suppression this tree once carried
  (`.aegis-pattern-state.json` plus a per-finding allowlist) was
  superseded by the content-keyed baseline in decision 0074, which
  keeps every rule armed and
  adjudicates each occurrence individually. A rule class that fires
  here gets a class note beside its rows in the baseline, never a
  class-wide switch.



## Quick reference

| Gate | Command | Timeout |
|---|---|---|
| Full CI | `cargo run -p xtask -- ci` | ~20 min |
| Aegis | `cargo run -p xtask -- aegis` | ~10s |
| Aegis report | `cargo run -p xtask -- aegis --report` | ~15s |


| Aegis baseline regen | `cargo run -p xtask -- aegis --bless` | ~10s |
| Coverage measure | `cargo llvm-cov --summary-only` (hook: use GitForge) | ~6 min |
| Coverage gate | `cargo run -p xtask -- coverage` | ~6 min |
| Wasm only | `cargo run -p xtask -- wasm` | ~5 min |
| Web clippy | `cd web && cargo clippy --all-targets -- -D warnings` | ~2 min |
| Dist | `cargo run -p xtask -- dist --native` + `-- --web` | ~10 min |

| Fact | Value |
|---|---|
| Live site | <https://firecraft-web.pages.dev> (manifest differs from local dist; deployed revision unknown) |
| Latest release | v0.2.5 (2026-11-03) |
| Coverage floors | 81.1% lines / 78.7% functions (rising-only; `xtask/src/coverage.rs`) |
| CI image | `firecraft-ci-rust:1` — rebuild + re-ship when ANY lockfile moves |
| GitForge API | 127.0.0.1:42780, bearer via `~/.config/gitforge/mkinney-credentials.env` |
