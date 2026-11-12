# T22 — Screenshot-driven shell and responsive UI audit

**Status:** screenshot audit complete. The tracked screenshot (`docs/img/app-main.png`), 1920×1080 capture, and empty/vector/bitmap state set are recorded below. Panel inventory (22.1) and tool strip (22.2) landed on `main` with the shell-lineage train (#107) on 2026-11-11; the remaining Phase 22 work is the separate CS6 chrome audit (22.5).

**Objective:** align the editor's working shell with the clean-room CS6 reference notes while keeping controls visible and usable at ordinary browser sizes.

**Inputs:** `docs/NEXT.md` Phase 22; `docs/research/fw-ui-evidence.md`; current shell modules under `crates/app/src/shell/`; `docs/img/app-main.png`; the documented clean-room reference boundary.

**Authority:** source behavior and licensed/open visual assets only. Do not copy Adobe artwork or assets. Do not claim screenshot parity from inventory or layout tests alone; capture and inspect the running browser UI.

**Acceptance:**

1. Capture the current browser at 1280×800 and 1920×1080 with a representative document and selection; record viewport, build SHA, and date.
2. At 1280×800, all active panels are reachable without a 3200 px viewport; panel headers, canvas, and tool controls do not overlap or disappear below the fold.
3. Check tool ordering/flyouts, menu and tab chrome, inspector fields, and at least empty, vector, and bitmap selection states against the documented evidence.
4. Link the fresh screenshot from README/docs and label it as evidence for its exact build/state.
5. Verify a fresh optimized `xtask dist --web` and the browser path before publishing replacement screenshots or changing `web/`.

**Historical starting gaps at ticket creation:** [`panel_windows.rs`](../../crates/app/src/shell/panel_windows.rs) implemented independently positioned windows in a right-side column; session evidence shows lower panels required a 3200 px viewport. The tracked screenshot predated the latest mainline changes and should be replaced after the comparison captures are reviewed.

## Receipts — 2026-11-04

**Build.** `cargo run -p xtask -- dist --web` on source `5fac7a2` plus the
uncommitted dock and inspector work. Optimized bundle in `dist/web/`: wasm
21,271,259 B (under the 21,500,000 B ceiling), manifest
`dist/web/MANIFEST.sha256`, assets
`firecraft-web-25ef8a220b81e769_bg.wasm` / `.js`.

**Captures.** Chromium (SwiftShader) against the built bundle served over
HTTP; the reply listener was installed with `addInitScript` before any page
script ran, so every capture is of the app's own state.

| Viewport | State | Object | Command reply | WebGL |
|---|---|---|---|---|
| 1280×800 | empty | — | — | alive |
| 1280×800 | vector | 200×150 rect at (100, 100) | `object.createPath` → `ok: true` | alive |
| 1280×800 | bitmap | 32×32 RGBA checkerboard at (100, 100) | `bitmap.importFile` → `ok: true` | alive |
| 1920×1080 | empty | — | — | alive |
| 1920×1080 | vector | as above | `ok: true` | alive |
| 1920×1080 | bitmap | as above | `ok: true` | alive |

The only console output is the benign `integrity`-attribute preload notice
and the SwiftShader `GPU stall due to ReadPixels` performance warning. No
`CONTEXT_LOST_WEBGL`.

**Criterion 1 — viewport, build, date.** Recorded above; the tracked
`docs/img/app-main.png` is now the 1280×800 vector capture
(sha256 `5e96b6d413dc74f75d4b7b26e3dcd2c83460a30692c01d29654fa8ba326c7cf3`).

**Criterion 2 — panels reachable at 1280×800.** Measured from the pixels:
the left tool strip ends at x=70, the canvas runs 71–961, and the right
dock occupies 962–1279 (318 px). The dock's drawn content ends at row 682
of 800, so every open panel is inside the viewport with no clipping. The
column is scrollable, and a wheel event over the dock at a deliberately
short 1280×400 viewport moves the dock content (7,082 dock pixels changed
between the before and after frames), so panels stay reachable when the
stack outgrows the window.

**Criterion 3 — empty, vector, and bitmap states.** Inspected by OCR and
by pixel analysis of the crop:

- *Empty* (1280×800): the menu bar reads `File Edit View Select Modify
  Text Commands Filters Window Help`; the dock shows `Fill & Stroke`,
  `Align`, `Properties` → `nothing selected`, `Page 1`, `State 1`, `Web
  Layer`; the status bar reads `ready untitled - page 1 - state 1 - undo 2
  - redo 0 - journal 2`.
- *Vector* (1280×800): the Properties panel shows the path field set —
  `X 100`, `Y 100`, `W 200`, `H 150`, plus `opacity`, `blend`, `visible`,
  `locked`. The canvas carries the created rectangle: its fill occupies
  x 213–499, y 221–436.
- *Bitmap* (1280×800 and 1920×1080): the Properties panel shows the
  bitmap field set — `bitmap 32x32 px, 1 tile(s) of 128 px`, `at (100.0,
  100.0), offset (0.0, 0.0)`, the name field `Bitmap`, `opacity`,
  `visible`, `locked`, and the live `Filters` list at `(none)`. The
  checkerboard renders on the canvas at x 214–258, y 222–267.

The earlier capture round for this ticket was **invalid**: the bundle in
`dist/web` predated the inspector work, so the captures showed the
pre-change fallback text (`selection has no editable properties yet`).
The receipts above are from a rebuilt bundle.

**Criterion 4 — the fresh screenshot is linked and labelled.** README's
stale-sample note now points at the replaced capture with its build and
state.

**Criterion 5 — fresh optimized bundle and the browser path.**
`xtask dist --web` rebuilt the bundle; the browser path was exercised
against that bundle for all six captures.

**Still open (owned elsewhere):** the CS6 chrome audit (22.5). The
panel inventory (22.1) and tool strip/flyouts (22.2) have since landed
in this PR branch. This ticket's own screenshot acceptance criteria
are met; it does not claim the broader chrome audit is complete.

**Verification:** browser captures at both sizes, visual inspection, targeted interaction checks, and `cargo run -p xtask -- ci` before a code change lands. See [`TASKS.md`](../TASKS.md) for the live priority.

## Receipt — 22.2 tool strip, 2026-11-05

**Evidence.** The strip's shape is CS6's, read from three independent
sources and recorded in
[`research/fw-ui-evidence.md`](../research/fw-ui-evidence.md): the CS6
help set ("The Tools panel is organized into six categories: Select,
Bitmap, Vector, Web, Colors, and View"; the group triangle and the
"repeatedly press the group keyboard shortcut" rule; the Modify trio;
the pixel-selection family; the retouch trio and its Alt toggle; the
Colors section's four buttons; the three view modes), the book's Table
1-1 "Fireworks Tools" for the names and the one-key shortcuts, and the
install's own `Fireworks.xml` for the shape of the tool-shortcut table
(37 `<one_tool>` blocks, 40 entries, decimal `MODS VK`, **GUIDs with no
tool names** — so the GUID→tool mapping stays an open evidence gap and
is recorded as one rather than guessed).

**Implementation.** `crates/app/src/toolstrip.rs` is new and holds the
strip as data: six categories in CS6's order, twenty-four groups with
their key and their members, CS6's labels, and the flyout memberships.
The shell renders from it; nothing about the order is FireCraft's
own. `crates/app/src/shell/canvas_ui.rs` draws one button per group,
with the triangle on a flyout's leader, and opens the flyout on
press-and-hold or a secondary click.

**Keyboard.** CS6 gives a *group* one key and cycles within it on
repeated presses. `ShellAction::PickToolGroup` implements that: the key
advances from the group's current member, or picks the leader when
another tool is active. The Hide/Show Slices control is the exception —
it keeps the book's `2` key but *acts* on the control instead of
becoming the active tool, because it is one of CS6's panel buttons.

**Tools added to the strip this round:** Skew and Distort (the Modify
trio, behind the Scale button on `Q`), Blur, Sharpen, Smudge, Dodge,
Burn, Rubber Stamp, Red-eye Removal and Replace Color (the retouch
tools, in the Bitmap section, sharing the paint brush options), Knife,
Freeform and Reshape Area (one group on `O`), and the Web section's
hotspot and slice variants. The Colors section's four controls — the
activate-stroke-or-fill icon, Set Default, No Stroke Or Fill, and Swap
Stroke/Fill — plus the two color boxes are CS6 **panel buttons**: they
act on click and never become the active tool.

**Bugs found and fixed on the way.**

1. `apply_default_stroke_fill` wrote the path paint as **ARGB**, but
   the engine reads path fill and stroke as **ABGR** — the red and blue
   channels were swapped. The shell now has both helpers (`argb_raw`
   for the bitmap/retouch commands, `abgr_raw` for the path paint) and
   each call site names the convention it means.
2. `end_drag`'s hotspot payload sent the shape name `"oval"`; the
   engine's `WebShape` is `"rect"` / `"ellipse"` / `"polygon"`.
3. `Editor::commit_polygon` was **never called from anywhere** — the
   polygonal marquee could be started but never finished. The
   properties panel now carries the in-progress shape's panel (point
   count, finish, cancel), and Enter/Escape finish or abandon a
   plotted path on the canvas.
4. The icon table had been regenerated by appending rather than
   replacing, leaving 117 entries for 73 icons — every lookup went
   through a hash map that silently kept the first copy. The table is
   now one entry per icon.

**Icons.** 29 new Lucide SVGs were vendored for the new tools, and
the tool→icon mapping now covers all 50 tools. Every new file was
diffed against upstream `lucide-static@1.69.0` and matches byte for
byte (after the license comment and the `class` attribute are
stripped); the pre-existing 43 were checked the same way. The one file
that is **not** Lucide — `slice-knife.svg`, an original drawing, since
Lucide has no such icon — now carries its own row in
[`ATTRIBUTION.md`](../ATTRIBUTION.md), and the Lucide row's count is
72.

**Verification.** `cargo clippy -p app --all-targets -- -D warnings`
clean; the app crate's suite green at 150 unit tests plus the
integration suites; eight new unit tests over the strip's own
invariants (the six categories in order, the uniqueness of the group
keys, the named flyout memberships, the Modify trio's shared key, the
panel buttons' status, and every tool's label).

The **full workspace gate is green**: `cargo run -p xtask -- ci`
finished `[xtask] ci: all gates green` (`XTASK_EXIT=0`) on
2026-11-05 — aegis, coverage, fmt, clippy (`-D warnings`, workspace
*and* `web/`), test, layers, wasm, corpus, floor, scripts, golden,
svg-golden, svg-roundtrip, web-golden, perf, fuzz, parity, scorecard
and changelog all `ok`. The perf step enforced its 12 budgets (worst
headroom 1.4× on `open_scripted_hwm_kib`).

**A pre-existing defect the gate surfaced.** The gate failed at its
*first* step, Aegis, before this change was even in play: the
committed baseline had gone stale, because the baseline then in use
keyed each finding on `file:line:content-digest` and this change
rewrote `menus.rs`, `document.rs` and four docs, shifting every
finding below it. The 307 new findings were all the known
false-positive classes — 292 `zip-code` on numeric literals, 8
`pci-cardholder-data` on the word "pan", 5 `sql-query` on the words
"select from a tool group", 1 each of `ssrf-localhost` and
`bearer-token-url` — and the baseline was regenerated over the whole
tree with the documented recipe, after which the scan was clean (exit
0, zero new findings). The stale baseline had been failing the local
gate since the last regeneration; it is recorded here because a gate
that fails on noise is a gate nobody trusts.

That defect is now fixed at the tool level rather than by procedure:
the baseline is content-keyed (decision 0074), so a line shift is not
a new finding and no whole-tree regeneration is needed.
