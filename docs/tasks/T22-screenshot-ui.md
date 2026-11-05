# T22 — Screenshot-driven shell and responsive UI audit

**Status:** in progress. The tracked screenshot (`docs/img/app-main.png`) has been replaced with a fresh 1280×800 capture of the current shell with a vector selection; the 1920×1080 capture and the empty/vector/bitmap state set are recorded below. The remaining open item is the panel-inventory work that Phase 22.1 still owns.

**Objective:** align the editor's working shell with the clean-room CS6 reference notes while keeping controls visible and usable at ordinary browser sizes.

**Inputs:** `docs/NEXT.md` Phase 22; `docs/research/fw-ui-evidence.md`; current shell modules under `crates/app/src/shell/`; `docs/img/app-main.png`; the documented clean-room reference boundary.

**Authority:** source behavior and licensed/open visual assets only. Do not copy Adobe artwork or assets. Do not claim screenshot parity from inventory or layout tests alone; capture and inspect the running browser UI.

**Acceptance:**

1. Capture the current browser at 1280×800 and 1920×1080 with a representative document and selection; record viewport, build SHA, and date.
2. At 1280×800, all active panels are reachable without a 3200 px viewport; panel headers, canvas, and tool controls do not overlap or disappear below the fold.
3. Check tool ordering/flyouts, menu and tab chrome, inspector fields, and at least empty, vector, and bitmap selection states against the documented evidence.
4. Link the fresh screenshot from README/docs and label it as evidence for its exact build/state.
5. Verify a fresh optimized `xtask dist --web` and the browser path before publishing replacement screenshots or changing `web/`.

**Observed starting gaps:** [`panel_windows.rs`](../../crates/app/src/shell/panel_windows.rs) implemented independently positioned windows in a right-side column; session evidence shows lower panels required a 3200 px viewport. The tracked screenshot predated the latest mainline changes and should be replaced after the comparison captures are reviewed.

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

**Still open (owned elsewhere):** the tool strip's order and flyouts
(22.2), the CS6 chrome audit (22.5), and the missing panel surfaces
(22.1). This ticket's own acceptance criteria are met.

**Verification:** browser captures at both sizes, visual inspection, targeted interaction checks, and `cargo run -p xtask -- ci` before a code change lands. See [`TASKS.md`](../TASKS.md) for the live priority.
