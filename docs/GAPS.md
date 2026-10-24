# FireCraft — Gaps, Improvements & Next Steps

> Audited 2026-10-21. Replaces the "Now" snapshot in PROGRESS.md until
> the next session close. A living document — update in-place, do not
> maintain a separate stale copy.
>
> **This revision answers the standing question**: *is FireCraft fully
> reverse engineered, fully functional, working end to end, and
> deployed?* The answer is **no, on the application surface** — see
> §1. The engine and command surface are complete; the interactive
> shell exposes a fifth of them. Phases 8–18 below close that gap.

---

## 1. The audit that produced this plan (2026-10-21)

### 1.1 What is genuinely complete

| Surface | State | Evidence |
|---|---|---|
| Fireworks PNG format | Complete | `crates/fwpng`, `crates/doc/src/fwmap.rs`; node tree round-trips |
| Engine command registry | **243 commands** | `engine::catalog::ids()` — 243 |
| CS6 DOM verb parity | **268 C / 3 P / 0 M / 85 N** | `docs/planning/generated/dom-parity.md` (356 verbs) |
| ExtendScript host | Complete | `crates/scripting` — runs the CS6 `.jsf` corpus headless |
| Web bridge | Exposes **all** commands | `web/src/main.rs` — `apply(name, payload)` |
| CLI | Complete | `crates/app-cli` — every verb drivable headless |
| Browser build + smoke | **Green** | `xtask dist --web` — wasm 12,778,687 B, "bridge smoke green" |
| Smoke is not vacuous | Fixed | `xtask/src/dist.rs:strip_script_bodies` — verdict read outside `<script>` |
| Live effects rendered | 6 (shadow×2, glow×2, bevel, fill) | `crates/compose/src/effects.rs:924` |
| Filters exposed | 30 | `crates/app/src/menus.rs` — `Action::Filter` |

### 1.2 What is NOT complete — the real finding

**The engine has 243 commands. The interactive shell reached 54 of
them when this audit ran — 188 commands (78%) unreachable from the
UI. After the Tier-1 batch (Phases 8–10 and the Phase 16 slice) the
shell reaches 72; after Phase 11's paint family it reaches 77;
166 commands (68%) are still unreachable.**

```
$ comm -23 <(engine::catalog::ids()) <(command names in crates/app/src)
188   # at audit time
170   # after the Tier-1 batch
166   # after Phase 11 (the paint family)
```

| Family | Unexposed | What the user cannot do |
|---|---|---|
| `text.*` | 22 | character/font markup, text-on-path, text rectangle, paragraph spacing/indent, anti-alias control, text runs |
| `bitmap.*` | 15 | **feather, pixel selection (rect/ellipse/lasso/polygon/wand), pixel edits** |
| `symbol.*` | 18 | symbol graph editing, instance placement, symbol type, transforms |
| `page.*` | 14 | page properties, canvas/export presets, page guides |
| `mask.*` | 14 | masks of every kind, mask mode/link/move, JPEG mask stash |
| `path.*` | 13 | path point editing, stroke properties, convert-to-path |
| `selection.*` | 12 | select children/parents/none, save/restore selection |
| `object.*` | 10 | object-level properties beyond the exposed ones |
| `slice.*` `document.*` | 14 | slice/link editing depth, document properties |
| `filter.*` `retouch.*` | 11 | selection-scoped filters, retouch tools |
| `frame.*` `hotspot.*` `style.*` `layer.*` `cell.*` `primitive.*` `autoshape.*` `clipboard.*` | 22 | frame animation depth, hotspot editing, style library, layer opacity/blend/duplicate, state handling, channel-to-channel paste |

Supporting evidence:

- **190 of 265 CS6 menu items render disabled** (`Action::Inert`,
  `crates/app/src/menus.rs`). The shell refuses to pretend.
- **The app could not open a raster image.** *(fixed 2026-10-21)*
  `File > Open` called `Editor::open` → `app_cli::io::open`, which
  parses only a Fireworks PNG (`mkTS` node tree) and rejected
  anything else with *"no mkTS node tree in file (not a Fireworks
  document?)"* (`crates/app-cli/src/io.rs:59`); `File > Import…` was
  **inert** and `bitmap.importFile` unreachable. Both are wired now.
- **The app could not copy, cut, or paste.** *(fixed 2026-10-21)*
  All six `clipboard.*` commands existed; none was wired, and no
  `Ctrl+C`/`Ctrl+V` shortcut existed. All five are wired now (the
  sixth, paste-from-channel-to-channel, still is not).
- **The app could not paint.** *(fixed 2026-10-21)* `bitmap.brush`,
  `bitmap.pencil`, `bitmap.erase`, `bitmap.fill` existed; the tool
  strip had 11 tools (`crates/app/src/editor.rs:17`) and none of them
  edited a pixel. The M15 brush model — shape, size, edge, aspect,
  angle — is complete and was unreachable. The paint family is in now
  (Phase 11); `bitmap.erase` and the pixel-selection commands are
  still out (11.4).
- **The app had no active layer.** *(fixed 2026-10-21)* The Editor
  tracked `page` and `frame` but no layer index; the Layers panel was
  display-only. Both now exist, and the panel's rows drive the
  placement target.

### 1.3 Verdict against the goal question

| Question | Answer |
|---|---|
| Fully reverse engineered? | **Yes** for the command/DOM/format surface (0 missing verbs, 3 documented partials). **No** for the application surface — 166 commands unreachable. |
| Fully functional? | **No.** A user can now import an image, place it on a chosen layer, copy/cut/paste, group, flatten, transform, rename and reorder layers, and paint on an image with the brush, pencil, eraser, bucket, gradient and eyedropper — but the raster tools stop at the paint family: no pixel selection, no mask surfaces. |
| Working end to end? | **Yes** for the command surface (CLI, scripting, web bridge all drive every command) and **yes** for the browser round trip (open → apply → export PNG → export document, headless Chromium green). **No** for a human editing an image. |
| Deployed? | **Partly.** v0.2.2 on GitHub; web bundle live at firecraft-web.pages.dev; browser smoke green. Desktop artifacts: Linux x86_64 only, never executed on macOS/Windows. |

**Conclusion: the goal is not met. Phases 11–18 are the work.**

---

## 2. State snapshot

| Dimension | Status | Notes |
|---|---|---|
| **Shipped** | v0.2.2 on GitHub (5 assets) | ✅ |
| **CI** | GitHub Actions: `fmt` (fast) + `gate` (`xtask ci-no-fmt`) | ✅ |
| **aegis** | Baseline clean; scan 0.17 s | `.aegisignore` excludes `target/`, `web/dist/`, `fuzz/` |
| **Coverage** | 80.3 % lines / 78.0 % funcs (floor 60 %) | Local `llvm-cov` fails on stale artifacts; CI works |
| **Code** | 110,928 Rust lines | No `unwrap()`/`panic!()` outside `#[cfg(test)]`; no `todo!`/`unimplemented!` |
| **Registry** | 243 commands | 77 reachable from the UI (54 at audit time) — **the gap** |
| **Parity** | 268 C / 3 P / 0 M / 85 N | 3 P rows are evidence-gated (Phase 12 oracle) |
| **Decision docs** | 66 (0001–0066) | Next: 0067 |
| **Web bundle** | 12,778,687 B (ceiling 14,000,000) | Re-measure if it grows >1 MB |

---

## 3. Phase 8 — Raster import (the app's first real act) · S

**Why first:** without it the app cannot open a JPEG or a PNG. Every
other editing feature is unreachable in practice.

- **8.1 `File > Import…`** — read a raster file (PNG/JPEG/GIF/BMP/TIFF/
  WebP/WBMP), decode through `io::raster::decode`, and place it with
  `bitmap.importFile` (bounds = the document canvas, maintain aspect
  ratio on).
- **8.2 `File > Open` sniffs** — `.fw.png` opens as a document; any
  other supported raster opens as an import. One door, no guessing.
- **8.3 Place on the active layer** — needs 8.1's payload
  (`page`/`layer`/`cell`). Lands with Phase 9's active-layer concept,
  defaulting to the topmost layer until then.

**Authority:** `bitmap.importFile` (CS6 `dom.importFile(fileURL,
boundingRectangle, bMaintainAspectRatio)`), `io::raster::decode`.
**Verification:** a shell test that imports a PNG fixture and asserts
one new image object of the right size; a CLI round-trip that is
already green stays green.

**Status (2026-10-21): done.** `File ▸ Import…` reads a raster
through `io::raster::decode` and places it with `bitmap.importFile`
(8.1); `File ▸ Open` sniffs — a `.fw.png` opens as a document, any
other supported raster opens as an import sized to the image (8.2);
the placement lands on the active layer (8.3). The engine's import
payload also accepts a base64 plane, so the browser sends one string
instead of a million-element JSON array.

---

## 4. Phase 9 — Active layer + Layers panel · M

- **9.1 `Editor::active_layer`** — an index into the current page's
  layer list, defaulting to the topmost, changed by clicking a row.
  Every placement command (`bitmap.importFile`, `clipboard.paste`,
  `layer.add`, `object.createPath`) resolves its `layer` from it.
- **9.2 Layers panel depth** — select (active), add, delete, rename,
  duplicate, reorder; per-layer display, opacity, blend mode, lock.
  Uses the 7 unexposed `layer.*` commands.
- **9.3 The panel shows the active layer** so the placement target is
  never a guess.

**Verification:** layer rows render state; add/delete/rename/reorder
land in the document and survive save/reload.

**Status (2026-10-21): 9.1 and 9.3 done; 9.2 partially.** The active
layer is a view-state index (topmost by default, set by clicking a
row) and every placement command — `bitmap.importFile`,
`clipboard.paste`, `object.createPath`, `text.createText`,
`AutoShape::payload` — resolves its `layer` from it; the editor's
current cell is the active layer's cell, so hit-testing and placement
agree. The panel rows carry display, lock (per state), rename,
reorder (up/down), add and delete. Still open in 9.2: layer
duplicate, opacity and blend mode.

---

## 5. Phase 10 — Clipboard · S

- **10.1 `Ctrl+C` / `Ctrl+X` / `Ctrl+V`** and the matching Edit menu
  items (CS6 shows them inert today) → `clipboard.copy`,
  `clipboard.cut`, `clipboard.paste`.
- **10.2 `Paste as Mask…` / `Paste Inside`** → `clipboard.pasteAsMask`,
  `clipboard.pasteInside`.
- **10.3 `Paste from Channel to Channel…`** → the
  `clipboard.pasteFromChannelToChannel` command.
- **10.4 Placement** uses the active layer (Phase 9).

**Verification:** copy an object, paste, assert the document carries
one more object; cut, assert the source is gone and the clipboard
holds it.

**Status (2026-10-21): 10.1, 10.2, 10.4 done; 10.3 open.** The
clipboard commands read the *document's* selection while the shell
tracks the selection as view state, so `Editor::sync_selection` is
the bridge: the view's ids land in the document through the same
command door as every other mutation (journaled, undoable), and an
unchanged selection is not re-sent. `Ctrl+C`/`Ctrl+X`/`Ctrl+V` and
the Edit menu items drive `clipboard.copy` / `clipboard.cut` /
`clipboard.paste`; the pasted objects re-mint onto the active layer
and become the selection. `Paste as Mask` / `Paste Inside` drive
`clipboard.pasteAsMask` / `clipboard.pasteInside`. Open in 10.3:
`clipboard.pasteFromChannelToChannel`.

---

## 6. Phase 11 — Raster tools · L

- **11.1 Tool strip** grows the paint family: **Paintbrush**,
  **Pencil**, **Eraser**, **Paint Bucket**, **Gradient**,
  **Eyedropper** (CS6 order).
- **11.2 Canvas interactions** drive `bitmap.brush`, `bitmap.pencil`,
  `bitmap.erase`, `bitmap.fill` with the M15 brush model
  (shape/size/edge/aspect/angle).
- **11.3 Properties panel** gains a **Brush** section (the CS6 Edit
  Brush dialog's slots) and a **Fill** section.
- **11.4 Pixel selection tools** — marquee (rect), ellipse, lasso,
  polygonal, wand → `bitmap.selectRect` / `selectEllipse` /
  `selectLasso` / `selectPolygon` / `selectWand` / `selectSimilar`.
- **11.5 `bitmap.feather`, `bitmap.movePixels`,
  `bitmap.moveSelectionMaskBy`, `bitmap.setPixelMask`,
  `bitmap.adjustPixelSel`** reach the UI.

**Verification:** a shell test paints a pixel and asserts the tile
bank changed; a selection test asserts the mask moved.

**Status (2026-10-21): 11.1, 11.2, 11.3 done; 11.4, 11.5 open.** The
tool strip carries the paint family in CS6 order (Paintbrush, Pencil,
Eraser, Paint Bucket, Gradient, Eyedropper), each with its one-key
select (B / N / Y / G / D / I). The canvas drives `bitmap.brush`
(soft-edged dabs along the drag, spaced a quarter-diameter apart),
`bitmap.pencil` (the hard one-pixel stamp), `retouch.eraser` (the
brush-shaped eraser), `bitmap.fill` (the bucket) and — new in this
phase — `bitmap.gradient`, the raster mode of CS6's Gradient tool:
a linear ramp between the press and the release points, clipped to
the selection. The eyedropper reads through the engine's new
`engine::bitmap::sample_pixel`, so the color model and the document
stay separate. The whole stroke folds into one undo step
(`Session::begin_transaction`), the way a moved gesture does. The
properties panel gains the **Brush** section (size, edge, aspect,
angle, pressure, strength, shape, paint color — the Edit Brush
dialog's slots), which serves as the fill section too.

Two engine corrections fell out of the work and are recorded here
because they change observed behavior:

- **`bitmap.pencil` stamps the pixel the point falls IN.** The
  one-pixel round stamp only covered the pixel whose center it
  crossed, so a pencil dab on a pixel corner painted nothing; the
  command now writes the containing pixel directly.
- **`bitmap.gradient` is a new registry command** (registry 241 →
  243) — CS6's Gradient tool has no raster equivalent in the
  engine until now.

Open in 11.4: the pixel-selection tools (marquee, ellipse, lasso,
polygon, wand). Open in 11.5: `bitmap.feather`, `bitmap.movePixels`,
`bitmap.moveSelectionMaskBy`, `bitmap.setPixelMask`,
`bitmap.adjustPixelSel`.

---

## 7. Phase 12 — Selection and mask surfaces · M

`selection.*` (13) and `mask.*` (13): select all / all-on-layer /
children / parents / inverse / none, save & restore named selections,
`selection.duplicateToFrame(s)` / `duplicateToFrameRange` /
`distributeToFrames`; mask add / remove / clear / enable / mode /
link / move / moveContentsBy / marqueeToPath / pathToMarquee, and the
JPEG mask stash (`mask.saveJPEG` / `mask.restoreJPEG`).

**Evidence-gated hold (do not attempt):** the out-of-bounds effect
growth bake and `setFillNColorNTexture` need a CS6 oracle. Wine and
Xvfb are installed on this host, but CS6 cannot launch: `mfc100u.dll`
(VC++ 2010 runtime) is absent from the install and from every Debian
package, and the Microsoft download URLs are dead. Record the
blocker; do not fake the evidence.

---

## 8. Phase 13 — Text depth · M

The 22 unexposed `text.*` commands: character markup
(`applyCharacterMarkup` / `removeCharacterMarkup`), font markup,
`convertToPaths`, `attachPath` / `detachPath` / `setTextOnPathMode` /
`setTextOnPathOffset` / `reversePathTextDirection`, `setTextRectangle`
(+ auto variants), paragraph indent / spacing-before / spacing-after,
anti-alias over-sample / sharpness / strength, `setTextRuns`,
`replaceButtonTextStrings` (+ in instances).

The CS6 Text panel (font, size, colour, weight, style, alignment,
leading, kerning, anti-alias) belongs in the Properties panel.

---

## 9. Phase 14 — Symbol and instance depth · M

The 19 unexposed `symbol.*` commands: definition editing, instance
placement and transforms, symbol types, nested-instance expansion.

---

## 10. Phase 15 — Page, frame, document, cell depth · M

`page.*` (14), `frame.*` (5), `document.*` (7), `cell.*` (2): page
properties and guides, canvas sizing and export presets, frame
animation timing, document-level properties, state (cell) handling.

---

## 11. Phase 16 — Remaining object, path, filter, style, slice, hotspot · M

`object.*` (14 remaining), `path.*` (13), `filter.*` (6 selection-scoped
filters + `filter.selection*`), `style.*` (3), `slice.*` (7),
`hotspot.*` (4), `flatten.*` (3), `primitive.*` (2), `autoshape.*` (2),
`retouch.*` (5: blur, smudge, clone, eraser, red-eye).

**Status (2026-10-21): the Tier-1 slice is done.** `Modify ▸ Group` /
`Ungroup` → `object.group` / `object.ungroup`; `Flatten Selection` /
`Merge Down` / `Flatten Layers` → `flatten.selection` /
`flatten.mergeDown` / `flatten.document`; `Transform ▸ Rotate 180° /
90° CW / 90° CCW` and `Flip Horizontal / Vertical` → `object.rotate` /
`object.reflect`; `Select ▸ Select Inverse` → the cell complement.
Three filter presets that were *dead* in the UI now reach a kernel:
`dropShadow` shipped zero parameters (the kernel reads four or
renders nothing) and the emboss presets were named `emboss` /
`pillowEmboss` while the kernel matched `raisedemboss` /
`insetemboss` — all three now change the rendered page, which a
regression test asserts. The export dialog gained WebP (appended to
the format list, so the stored `FMTi` indices of saved documents keep
their meaning). Still open: the scale/skew/distort/numeric-transform
dialogs, the six selection-scoped filters, `style.*`, `slice.*`,
`hotspot.*`, `primitive.*`, `autoshape.*` and `retouch.*`.

---

## 12. Phase 17 — The Optimize panel · M

CS6's Optimize panel is a first-class window: per-format lossy
settings, palette reduction with dithering, matte colour, and the
export preview. FireCraft has the format writers
(`crates/io/src/{png,jpeg,gif,bmp,tiff,webp}.rs`) but no tuning
surface. Build the panel over the existing `ExportOptions` / `EXPv`
model.

---

## 13. Phase 18 — Deployment and verification hardening · M

- **18.1 Desktop release** — build the Linux x86_64 binary on the
  release pipeline, ship it as a release asset, and add a real
  end-to-end CLI test (open → edit → export → reopen).
- **18.2 Browser smoke depth** — the current smoke proves the bridge
  round trip. Extend it to a *human* flow: import a PNG, paint, make a
  selection, copy/paste, export. It must fail if the UI regresses.
- **18.3 macOS / Windows artifacts** — blocked on hardware; the
  recipes exist (`docs/packaging/STATUS.md`) and stay honestly
  unverified.
- **18.4 WebOS on a real device** — blocked on device access.
  `docs/WEBOS.md` records the gap.

---

## 14. Quick-reference: task ownership

| Task | Phase | Size | Blocker |
|---|---|---|---|
| 8.1 File > Import (raster) | 8 | S | **done** |
| 8.2 File > Open sniffs | 8 | S | **done** |
| 9.1 Active layer concept | 9 | M | **done** |
| 9.2 Layers panel depth | 9 | M | partly (opacity/blend/duplicate open) |
| 10.1 Clipboard shortcuts + menu | 10 | S | **done** |
| 10.2 Paste as Mask / Inside | 10 | S | **done** |
| 10.3 Paste from Channel to Channel | 10 | M | open |
| 11.1–11.5 Raster tools | 11 | L | 11.1–11.3 **done** |
| 12 Selection + mask surfaces | 12 | M | 1 row needs the CS6 oracle |
| 13 Text depth | 13 | M | — |
| 14 Symbol depth | 14 | M | — |
| 15 Page/frame/document/cell | 15 | M | — |
| 16 Remaining families | 16 | M | Tier-1 slice **done** |
| 17 Optimize panel | 17 | M | — |
| 18.1 Desktop release + e2e test | 18 | M | — |
| 18.2 Browser smoke depth | 18 | M | — |
| 18.3/18.4 Off-fleet artifacts | 18 | — | Hardware |

---

## 15. Next action

**Phase 11.4 / 11.5 — the pixel-selection tools and the pixel-edit
surfaces.** The paint family is in: the tool strip, the strokes and
the brush slots landed in Phase 11. What is still missing is the
*selection* side of the raster tools — CS6's marquee, ellipse, lasso,
polygon and wand, and the pixel edits that ride on a selection
(`bitmap.feather`, `bitmap.movePixels`,
`bitmap.moveSelectionMaskBy`, `bitmap.setPixelMask`,
`bitmap.adjustPixelSel`). The engine commands all exist; they need a
tool strip row, canvas drags and a panel that shows the mask.

---

## 16. Closed work (for context — do not reopen without new evidence)

- **M0–M15** (2026-10-10 → 10-13): the whole milestone ladder.
- **The DOM verb-surface campaign** (tasks #86–#134): 356 CS6 verbs
  swept; 57 missing verbs closed family-by-family to 268 C / 3 P / 0 M;
  registry 105 → 241.
- **Phase 1–7 of the previous GAPS** (2026-10-20 → 10-21): docs and
  release hygiene, image support (AVIF decision 0064, HEIC refusal
  0065, import-surface parity, lossy WebP 0063 + implementation),
  coverage baseline, WebOS packaging proof, smoke budget, platform
  recipes, CI split into `fmt` + `gate`, cargo-llvm-cov and trunk
  caching.
- **Phase 7 (research holds)**: closed as infrastructure-blocked —
  CS6 cannot run under Wine here because `mfc100u.dll` is unobtainable
  (see §7).
