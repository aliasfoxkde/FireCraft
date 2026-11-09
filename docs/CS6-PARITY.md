# CS6 user-visible parity matrix

> Audited 2026-11-04 against the `main` tree. This is the
> **user-visible** inventory demanded by
> [`tasks/T26-parity-matrix.md`](tasks/T26-parity-matrix.md): every
> Fireworks CS6 workflow and UI area a person can reach, classified
> with a citation. It is *not* the DOM verb matrix
> ([`planning/generated/dom-parity.md`](planning/generated/dom-parity.md)),
> which scores **scripting** verbs. A verb can be fully carried in the
> DOM and still have no UI path; several rows below are exactly that.

## How to read this

| class | meaning |
|---|---|
| **implemented** | FireCraft has an executable UI path for the CS6 workflow, and that path works |
| **partial** | the UI path exists but a recorded part of the CS6 behaviour is missing |
| **missing** | no UI path exists |
| **evidence-gated** | the CS6 behaviour itself is not yet established, so FireCraft's coverage cannot be judged; the gate is named |
| **not applicable** | the CS6 workflow has no counterpart in FireCraft's scope (or the CS6 side is a documented refusal) |

Evidence codes: **`M`** = `crates/app/src/menus.rs` (the CS6 `RT_MENU`
transcription); **`E`** = `crates/app/src/editor.rs`; **`S`** =
`crates/app/src/shell/`; **`C`** = `crates/engine` command registry
(`engine::catalog::ids()`); **`R`** =
[`research/fw-ui-evidence.md`](research/fw-ui-evidence.md) (CS6 install
evidence); **`D`** = `docs/decisions/`; **`X`** =
[`tasks/T26-parity-matrix.md`](tasks/T26-parity-matrix.md) itself.

CS6 command ids come from `MENU_DESCRIPTION_MAP` (**R**); the id →
panel-name table is in [`research/fw-ui-evidence.md`](research/fw-ui-evidence.md#the-cs6-panel-command-id-table-menu_description_map).

**Rule used throughout:** a row counts as *implemented* only when a
person can reach it from the running shell. "The engine has the
command" is recorded as evidence, never as the verdict.

---

## A. Menu bar and commands

CS6's menu bar is ten menus: **File, Edit, View, Select, Modify,
Text, Commands, Filters, Window, Help** (**R**). FireCraft carries
all ten in the same order (`M:2095`).

| CS6 menu | CS6 rows | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|---|
| File | 32 | 32 rows, same order; 14 rows dispatch to a documented "not available in this build" report | **partial** | `M:459` | — |
| Edit | 25 | 25 rows; Insert submenu carries 11 of CS6's entries | **partial** | `M:695` | — |
| View | 22 | 22 rows; Magnification 6%–6400%, Grid/Guides/Smart Guides submenus all dispatch | **implemented** | `M:872` | — |
| Select | 19 | 19 rows; the four marquee modes and the bitmap-selection save/restore submenu dispatch | **implemented** | `M:1049` | — |
| Modify | 25 | 25 rows, 11 submenus; Canvas/Animation/Symbol/Pop-up Menu/Mask/Selective JPEG/Transform/Arrange/Align/Combine Paths/Alter Path all present | **partial** | `M:1163` | 22.4 |
| Text | 15 | 15 rows, 5 submenus; the Font submenu dispatches `text.setFont` over the renderer's five families (CS6 populates it at runtime from the system inventory) | **implemented (static list)** | `M:1647`, `M:1652`, RT_MENU 128 re-read 2026-11-06 | 22.5 |
| Commands | 4 | 4 rows; *Manage Saved Commands…* is inert | **partial** | `M:1817` | 21.6 |
| Filters | 10 | 10 rows; 30 filter rows over 6 submenus; *Repeat Filter* re-applies the session's last preset | **implemented** | `M:1844`, `S/filters_ui.rs:30` | — |
| Window | 34 | 34 rows; the four unimplemented panels are inert rows | **partial** | `M:1927` | 22.5, 22.1 |
| Help | 6 | 6 rows; Online Help opens the hosted wiki, Keyboard Shortcuts opens the binding table | **implemented** | `M:2061`, `S/panels.rs:70` | — |

**Inert-row census.** Two of 296 rows render inert, pinned by
`menus::tests::inert_count_never_rises`. They are **Window ▸
Extensions** (the unimplemented CS6 panels row, 22.1) and
**Commands ▸ Manage Saved Commands…** (the scripting family, 21.6).
The Text ▸ Font placeholder and Filters ▸ Repeat Filter were wired
2026-11-06; the four unimplemented Window panels land with their
features (22.1).

**Gap-reporting rows.** Thirty-six rows dispatch to
`ShellAction::ReportGap` — they *are* clickable and say plainly that
the feature is absent (`M`). This is the honest middle state between
"inert" and "implemented", and it is why the matrix does not read the
inert census as a coverage number.

### Open evidence item: the Window menu's window-arrangement rows — CLOSED 2026-11-06

`MENU_DESCRIPTION_MAP` names `57650 = cmd_CascadeWindows`,
`57651 = ID_WINDOW_TILE_HORZ`, `57652 = ID_WINDOW_TILE_VERT` (**R**).
FireCraft's transcription carried 57650 *Cascade* and 57652 *Tile*,
and no 57651 (**M:2046**, `M:2052`). The two open questions resolved
without decoding the bar resource:

1. The transcription now carries the full trio — 57650 *Cascade*,
   57651 *Tile Horizontally*, 57652 *Tile Vertically* — gated by
   `menus::tests::the_window_arrangement_trio_matches_the_resource`;
   the pinned item census rose 289 → 290 in the same change (the row
   was always part of the CS6 surface; the transcription was short
   one).
2. The 57652 label is CS6's documented *Tile Vertically*
   (`ID_WINDOW_TILE_VERT`); the earlier "Tile" was the
   transcription's shorthand, dropped in favor of the resource's
   name. All three rows report the single-document limitation
   instead of acting, as before.

---

## B. Tools and flyouts

CS6's tool strip is a vertical palette (`cmd_Toolbars_Art`,
`cmd_Toolbars_Main` — **R**). FireCraft draws it in a 34 px left
panel (`S/mod.rs:2277`, `S/canvas_ui.rs:33`).

| CS6 tool area | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|
| Select / Subselect | Pointer + Subselect, both in the strip | **implemented** | `E:18`, `S/canvas_ui.rs:34` | — |
| Shape tools (rect, ellipse, line, text) | Rect, Ellipse, Line, Text | **implemented** | `E:18` | — |
| Auto Shape tools (10 CS6 shapes) | AutoShape(star, polygon, spiral, pie, doughnut, L-shape, arrow, arrow-line, measure, connector) with a flyout gallery | **implemented** | `E:179`, `S/panels_web.rs:97` | — |
| Paint family (brush, pencil, eraser, bucket, gradient, eyedropper) | all six, with the CS6 Edit Brush slots | **implemented** | `E:18`, `S/panels.rs:888` | — |
| Pixel selection (marquee, elliptical, lasso, polygonal, wand) | all five, with the four blend modes | **implemented** | `E:18`, `S/panels.rs` selection panel | — |
| Web tools (slice, hotspot) | Slice, Hotspot | **implemented** | `E:18` | — |
| View tools (zoom, hand) | Zoom, Hand | **implemented** | `E:18` | — |
| **Flyout mechanics** (press-and-hold opens the group's list) | implemented for the strip groups and the Auto Shape gallery | **implemented** | `S/canvas_ui.rs:78-100`, `S/mod.rs:198` | — |
| **Tool one-key selects** | 24 group keys (V, A, Q, C, M, L, W, B, E, R, D, S, N, P, U, T, O, Y, J, K, 2, I, G, H, Z); CS6 gives a *group* one key and repeated presses cycle within it | **implemented** — the group-cycling rule and every key come from the book's Table 1-1 and the CS6 help; CS6's `one_tool` GUID table has no tool names, so the GUID→tool mapping stays an open evidence gap | `R:fw-ui-evidence.md` (tool-strip section), `S/toolstrip.rs` | — |
| **CS6 tool strip ORDER** | the strip is CS6's: Select, Bitmap, Vector, Web, Colors, View — each with its groups in CS6's order | **implemented** | `R:fw-ui-evidence.md` (help 1703 + the book's ch. 1), `S/toolstrip.rs` | — |
| **CS6 tool strip as data** | `crates/app/src/toolstrip.rs` holds the categories, groups, labels, keys and flyout memberships; the shell renders from it | **implemented** | `S/toolstrip.rs` | — |
| **The Modify trio** (Scale, Skew, Distort) | one group behind the Scale button; Skew and Distort drag the selection | **implemented** | `R` (help 1951), `S/editor.rs` | — |
| **Retouch trio** (Blur, Sharpen, Smudge) and **Dodge/Burn** | all five, in the Bitmap section, sharing the paint brush options | **implemented** | `R` (help 3530–3555), `S/editor.rs` | — |
| **Knife** | clicks cut the path under the pointer | **implemented** | `S/editor.rs` | — |
| **Freeform** | the Freeform tool draws and the Reshape Area flyout member distorts the selected path | **implemented** | `R` (help 4508), `S/editor.rs` | — |
| **Colors section** (the activate-stroke-or-fill icon, Set Default, No Stroke Or Fill, Swap Stroke/Fill, and the two color boxes) | all five, as panel *buttons* that act on click rather than becoming the tool | **implemented** | `R` (help 5409–5429), `S/canvas_ui.rs` | — |
| **Hide/Show Slices** control | a panel button on the book's `2` key | **implemented** | `R` (book Table 1-1), `S/canvas_ui.rs` | — |
| **Screen mode** (Standard Screen / Full Screen with Menus / Full Screen) | one control that **cycles the three view modes** (fwref 1178–1185): Standard, Full Screen with Menus (menus kept, gray backdrop), Full Screen (all chrome folded, black backdrop, no tab strip) | **implemented** | `R` (help 1175–1185), `S/mod.rs` (`ScreenMode`), `S/panels.rs` | — |
| **3D tools** | absent | **missing** — CS6's 3D tool is a legacy feature FireCraft does not carry | `R` (book ch. 1) | — |
| **Alt-hold tool variants** (Alt between Blur/Sharpen/Smudge and Dodge/Burn) | Alt swaps the retouch pair at the dab edge (Blur↔Sharpen, Dodge↔Burn; Smudge has no variant); the active tool never changes | **implemented** | `R` (help 3555, 3580), `S/toolstrip.rs::alt_variant`, `S/editor.rs` | — |

---

## C. Panels

CS6's panel set is **twenty-three** (**R**, `MENU_DESCRIPTION_MAP`).
Eighteen have a Window-menu row; five do not.

| CS6 panel (CS6 id) | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|
| Properties (35586) | right-dock panel; the CS6 inspector's shared region | **implemented** | `S/panels.rs:778` | — |
| Pages (35798) | panel; page list, click selects | **implemented** | `S/panels.rs:188` | — |
| States (43410, `cmd_Panels_Frames`) | panel; state list | **implemented** | `S/panels.rs:206` | — |
| Layers (43412) | panel; name, eye, lock, reorder | **implemented** | `S/panels.rs:235` | — |
| History (43418) | panel; journal rows, Save Steps / Replay | **implemented** | `S/panels.rs:404` | 20.6 |
| Styles (43415) | panel; style rows, New From Selection, Import | **implemented** | `S/panels.rs:453` | — |
| Document Library (43416, `cmd_Panels_SymLibrary`) | panel; symbol rows, enter/exit symbol | **implemented** | `S/panels.rs:533` | — |
| Color Mixer (43405) | panel; fill/stroke pickers + gradient editor | **implemented** | `S/panels.rs:636` | — |
| Swatches (43408) | panel; 6-column grid applying fill | **implemented** | `S/panels.rs:713` | — |
| Info (43404) | panel; canvas size, selection count, bounds — **read-only** | **partial** — CS6's Info panel is read-only too, but FireCraft's does not yet carry the per-object pixel statistics CS6 shows | `S/panels.rs:757` | 22.1 |
| Auto Shapes (43422) | panel; shape gallery | **implemented** | `S/panels_web.rs:97` | — |
| URL (43414, `cmd_Panels_URLMgr`) | panel; link + alt text, Apply | **implemented** | `S/panels_web.rs:169` | — |
| Behaviors (43413) | panel; list, Add menu, Add swap image | **implemented** | `S/panels_web.rs:217` | — |
| Tools (35058, `cmd_Toolbars_Art`) | the left tool strip | **implemented** | `S/canvas_ui.rs:33` | 22.2 |
| **Optimize** (43409, `cmd_Panels_ExportPanel`) | **inert Window-menu row** | **missing** | `M:1962` | 22.1 / 20.2 |
| **Common Library** (45816) | **inert Window-menu row** | **missing** | `M:1970` | 22.1 |
| **CSS Properties** (49125) | **inert Window-menu row** | **missing** | `M:1972` | 22.1 |
| **Find and Replace** (43402, `cmd_PanelFind`) | **inert Window-menu row** | **missing** | `M:2043` | 22.1 |
| **Slices** | no panel at all | **missing** — CS6's Slices panel is the slice/hotspot tree; FireCraft has slice editing in the URL/Behaviors/Web Layer panels but no slice list | `S/panels_web.rs` | 22.1 / 20.7 |
| **Project** (43403, `cmd_PanelProject`) | none | **missing** | **R** | 22.1 |
| **Site** (43420, `cmd_Panels_Site`) | none | **missing** | **R** | 22.1 |
| **Answers** (43421, `cmd_Panels_Answers`) | none | **not applicable** — CS6 opens it from the Help menu as a web panel | **R** | — |
| **What's New** (35425, `cmd_Help_WhatsNew`) | none | **not applicable** — CS6 opens it from the Help menu | **R** | — |
| Web Layer | FireCraft's own always-present web panel (slice list, link, fg/bg type) | **modern addition** — see §J | `S/dialogs.rs:507` | — |

### Panel *arrangement* (not just inventory)

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:fafccb9d7a47b510bd460b2e19723cfca2dee9e2a8f482cccf8b94df53067946 -->

| Property | CS6 | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|---|
| Right dock is **five stacked tab-groups** | yes | one flat column of 13 collapsible panels + the Web Layer panel | **missing** | **R** (`Expanded Mode.xml`) | 22.1 |
| Left dock holds the ToolPalette | yes | left tool panel, 34 px | **partial** — placement matches; the *toolbar* form does not | **R**, `S/mod.rs:2277` | 22.2 |
| Top dock holds the Main toolbar (778×28) | yes | a main toolbar row exists | **partial** — the row exists; its *contents* are not the CS6 main toolbar | **R** | 22.5 |
| Bottom dock holds the Properties tab-group | yes | properties live in the right dock instead | **partial** | **R** | 22.1 |
| Panels dockable left↔right, collapsible, resizable | yes | resizable + collapsible + reopenable from the Window menu; **not** left↔right movable | **partial** | `S/panel_windows.rs:150` | 22.1 |
| Dock geometry persists across restart | yes | eframe `persistence` + `ShellLayout` snapshot | **implemented** | `D` 0067/0069, `S/mod.rs` | — |
| The dock **reserves** width so the canvas shrinks rather than being covered | yes (a dock) | yes — `egui::Panel::right`, 258 pt, scrolling column | **implemented** | `S/panel_windows.rs:154` | 22.5 |
| Every panel reachable at 1280×800 | yes | T22 captures and reachability check complete for the recorded build; dock scroll verified at 1280×400. The newer dock/panel inventory still needs a capture. | **verified for captured build; newer dock evidence-gated** | [T22 receipts](tasks/T22-screenshot-ui.md#receipts--2026-11-04) | 22.5 / T22 |

<!-- /coderabbit-ai-generated:v1 -->
---

## D. Property inspector, per selection kind

CS6's Properties panel swaps its field set by selection kind. FireCraft
dispatches at `S/panels.rs:853-881`.

| Selection | CS6 field set | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|---|
| Nothing selected | document-level fields (canvas, page, zoom) | "nothing selected" | **partial** — no document-level fields in the inspector | `S/panels.rs:854` | 22.4 |
| Vector path | X/Y/W/H, opacity, blend, visibility, lock, name, fill, stroke, stroke style, effects | X/Y/W/H, opacity, blend, visible, locked, name, fill/stroke + gradient editor, effects list | **implemented** | `S/panels.rs:1183`, `:1318`, `:1348` | — |
| Text | the path set **plus** font, size, bold/italic/underline, alignment, text content | all present; characters apply on Enter | **implemented** | `S/panels.rs:1206` | 20.1 |
| Bitmap / image | the path set, minus the fill/stroke the `IMGv` node does not carry; CS6's bitmap-specific tools live in the separate **Image Editing** command panel | name, opacity, blend, visible, locked, a pixel-size readout, and the effects list. X/Y/W/H is a readout: `object.setBounds` refuses images (decision 0019) | **partial** | `S/panels.rs:1382`, `R:fw-ui-evidence.md` | 22.4 |
| Symbol instance | definition, symbol number, type, placed-at, Edit/Break Apart, per-state counts, nine-slice guides | all present | **implemented** | `S/panels.rs:1085` | — |
| Slice / hotspot | link, alt text, type, name, frame geometry | all present in the inspector; the link fields are the *same editor* the **URL** panel renders, so the two cannot disagree | **implemented** | `S/panels.rs:1418`, `S/panels_web.rs:190` | — |
| Group | member list, enter-group | member list, display controls, geometry; **no** enter-group | **partial** | `S/panels.rs:1332` | 22.4 |
| Effects list | per-effect rows with per-parameter fields, removable, addable | present for path, text, **group and bitmap**; **not** for instances (the function returns early) | **partial** | `S/panels.rs:1594`, `:869` | 22.4 |

---

## E. Document workflows

| CS6 workflow | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|
| New / Open / Save / Save As / Close | present | **implemented** | `M:459` | — |
| New from Template / Save As Template | both report "not available in this build" | **missing** (documented refusal) | `M:463`, `M:503` | — |
| Revert | present | **implemented** | `M:503` | — |
| Undo / Redo | present | **implemented** | `M:695` | — |
| Save All / Close All | present | **implemented** | `M:494`, `M:482` | — |
| Open Recent | one inert-ish row that reports the gap | **missing** (documented refusal) | `M:472` | — |
| Export Wizard | present | **implemented** | `M:604` | — |
| Batch Process | reports the gap | **missing** (documented refusal) | `M:644` | — |
| **Multi-document tabs** | FireCraft is single-document; *Duplicate Window*, *Cascade* and *Tile* all collapse to the same single-document action | **missing** | `M:1929`, `M:2046`, `M:2052` | 20.5 |
| Print / Page Setup | report the gap (no printing) | **not applicable** — FireCraft has no printing story | `M:663`, `M:679` | — |
| **Autosave / crash recovery** | none | **missing** | — | 21.1 |
| **Preferences dialog** | Edit ▸ Preferences… reports the gap | **missing** | `M:773` | 21.4 |
| **Keyboard Shortcuts editor** | the Help/Edit row opens a *read-only* binding table, not an editor | **partial** | `S/panels.rs:70` | 21.4 |
| Save Steps As Command / Replay | History panel's Save Steps…/Replay… | **implemented** | `S/panels.rs:404` | 20.6 |
| **In-app scripting** (Commands ▸ Run Script…) | the row is present but *Manage Saved Commands…* is inert; no script console | **partial** | `M:1817` | 21.6 |

---

## F. Import and export surfaces

| CS6 direction / format | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|
| Import PNG, JPEG, GIF, BMP, TIFF, WebP, WBMP | all present | **implemented** | `crates/io` | — |
| Import `.fw.png` (native) | present | **implemented** | `crates/fwpng` | — |
| Export PNG, JPEG (baseline + progressive), BMP, TIFF, WBMP, WebP, GIF | all present | **implemented** | `crates/io`, `S/dialogs.rs:198` | — |
| Export SVG, PSD (layered + merged) | present | **implemented** | `crates/io` | — |
| Export web (images, HTML, CSS, sprites, CSS properties, pages, states, layers, animated GIF) | 9 workflows in the Export dialog | **implemented** | `S/dialogs.rs:204` | — |
| JPEG quality, progressive, matte | present (JPEG-only controls) | **implemented** | `S/dialogs.rs` | — |
| GIF loop, repeat, crop/diff frames, dither | present (GIF-only controls) | **implemented** | `S/dialogs.rs` | — |
| **Update HTML** (re-export an existing site) | reports the gap | **missing** (documented refusal) | `M:582` | 20.7 |
| **HTML Setup** (export options for a site) | reports the gap | **missing** (documented refusal) | `M:679` | 20.7 |
| **Set Primary / Secondary Browser** | reports the gap (system default is used) | **partial** — preview works; browser choice does not | `M:644` | 20.7 |
| Send to E-mail | reports the gap | **not applicable** | `M:534` | — |
| Scan (TWAIN) | reports the gap | **not applicable** | `M:482` | — |
| 2-Up export preview (export vs PNG reference) | present | **implemented** | `S/dialogs.rs` | — |

---

## G. Canvas interactions

| CS6 interaction | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|
| Selection marquee and handles | transform handles: scale, rotate, skew, live preview, one undo step | **implemented** | `S/canvas_ui.rs`, 19.1 | — |
| Sub-select path nodes (anchors + bezier handles), move, add, delete | present | **implemented** | 19.2 / 19b.1 | — |
| Gradient on-canvas editing | present (stops + handles) | **implemented** | 19.3 | — |
| Grid, guides, drag-out guides, snap-to, Edit Grid dialog | present | **implemented** | 19.5, `S/guides_ui.rs` | — |
| Context menu that changes with the selection class | present (empty / bitmap / path / text / instance / slice / group / multi) | **implemented** | `S/canvas_ui.rs` (22.3) | — |
| Command palette over the whole registry (Ctrl+P) | present; every registry command is executable from the GUI | **implemented** | `S/palette.rs:256`, 19.4 | 21.3 |
| **In-canvas text editing** (type on canvas, caret, double-click to edit) | text is typed into the Properties panel instead | **missing** | `E` `Tool::Text` doc comment | 20.1 |
| Gradient / paint / pixel-selection canvas gestures | present | **implemented** | `E:18` | — |
| Multi-selection and multi-object transforms | present | **implemented** | `S/panels.rs:811` (align/distribute) | — |

---

## H. Keyboard access

| CS6 capability | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|
| Menu accelerators (Ctrl+N/O/S/Z, Ctrl+Shift+S/R, Alt+F4 …) | carried on the menu rows | **implemented** | `M:459` | — |
| A working shortcut table (52 bindings) | present and dispatched, and skipped while a text field has focus | **implemented** | `M:2157`, `S/mod.rs:2157` | — |
| F1 lists the bindings | present, in 11 labelled groups | **implemented** — the group layout is `menus::SHORTCUT_GROUPS`, asserted to tile the 60-row table exactly (`the_shortcut_groups_cover_the_table`; the mismatch that summed to 49 was fixed 2026-11-06) | `S/panels.rs:94`, `S/menus.rs` | — |
| **Tool one-key selects** | 24 group keys bound; repeated presses cycle within the group, as CS6's help describes | **implemented** — see §B | `S/toolstrip.rs`, `S/menus.rs` | — |
| **CS6's full keymap** (the `Fireworks.xml` set: 128 command ids) | 52 of CS6's 128 command shortcuts are bound | **partial** — the remaining 76 have no binding | **R** (keymap section) | 22.5 |
| Four alternative keymap sets (Illustrator / Photoshop / FreeHand / Web Standard) | none | **missing** | **R** | 21.4 |
| Screen-reader navigation | **not available** — the accesskit tree is unreachable on the web build; recorded refusal, not a gap | **not applicable** | `D` 0067 | 21.5 |

---

## I. Selection states

The eight selection classes CS6 renders differently, each with its
own inspector fields, context-menu rows, and status-bar readout.

| State | Inspector | Context menu | Status readout | Verdict | Ticket |
|---|---|---|---|---|---|
| Nothing selected | document fields | edit verbs | canvas size | **partial** | 22.4 |
| Single path | path fields | path verbs | bounds | **implemented** | — |
| Multiple paths | path fields (first) | multi verbs | bounds + count | **partial** — align/distribute work; the inspector shows only the first object | 22.4 |
| Text object | text fields | text verbs | bounds | **implemented** | 20.1 |
| Bitmap object | bitmap fields (transform strip read-only) | bitmap verbs | bounds | **partial** | 22.4 |
| Symbol instance | instance fields | instance verbs | bounds | **implemented** | — |
| Slice / hotspot | slice fields + the URL panel's editor | web verbs | bounds | **implemented** | 20.7 (behaviors) |
| Group | group fields (no enter-group) | group verbs | bounds | **partial** | 22.4 |
| Inside a slice | — | slice verbs | — | **partial** | 20.7 |

---

## J. Modern FireCraft additions (not CS6 parity work)

These have no CS6 counterpart and are **not** gaps. They are listed so
the matrix does not read them as missing CS6 features.

| Addition | Why it exists | Evidence |
|---|---|---|
| Command palette (Ctrl+P) over the whole registry | CS6 had no palette; FireCraft's registry is 245 commands and a palette makes every one reachable | `S/palette.rs` |
| Rust-native `.fw.png` engine, CLI, journal, MCP server | headless automation and scripting parity beyond CS6's surface | `crates/app-cli` |
| ExtendScript host running the CS6 `.jsf` corpus | scripting parity, headless | `crates/scripting` |
| The **Web Layer** panel | FireCraft's own always-present web panel; CS6's web state lives in the Properties/Slices panels | `S/dialogs.rs:507` |
| Live-effects inspector with per-parameter rows | CS6's inspector is field-based; FireCraft's effect list is engine-backed and editable | `S/panels.rs:1429` |
| A 2-Up export preview inside the Export dialog | FireCraft's own convenience | `S/dialogs.rs` |

---

## K. Gap ledger

Every **missing** and **partial** row above, with its owner ticket and
acceptance hook. Ordered by how much of the product it unlocks.

| # | Gap | Rows | Ticket | Priority |
|---|---|---|---|---|
| 1 | The five missing CS6 panels (Optimize, Common Library, CSS Properties, Find and Replace, Slices) plus Project and Site | C | 22.1 | P0 |
| 2 | The inspector's remaining per-kind gaps: document-level fields, multi-selection fields, the image transform strip, and enter-group | D, I | 22.4 | P0 |
| 3 | The right dock's five-tab-group structure and the bottom Properties dock | C | 22.1 | P0 |
| 4 | In-canvas text editing | E, H, I | 20.1 | P0 |
| 5 | The CS6 tool strip's remaining items: the 3D tool (the screen-mode cycle and the Alt-hold variants **landed 2026-11-06**) | B | 22.5 | P2 |
| 6 | Multi-document tabs (and the Window rows that collapse to them) | E | 20.5 | P1 |
| 7 | The animation timeline (States panel) | C | 20.3 | P1 |
| 8 | History click-to-rewind | C, E | 20.6 | P1 |
| 9 | Slice/hotspot behaviour editing and the HTML preview surface | C, F, I | 20.7 | P1 |
| 10 | Autosave and crash recovery | E | 21.1 | P1 |
| 11 | The Preferences dialog and the Keyboard Shortcuts editor; the four alternative keymaps | E, H | 21.4 | P1 |
| 12 | The in-app script console and saved commands | A, E | 21.6 | P1 |
| 13 | Command palette v2 (fuzzy, recents, save-steps) | G | 21.3 | P2 |
| 14 | ~~The Text ▸ Font submenu, *Repeat Filter*, the F1 grouping mismatch~~ **CLOSED 2026-11-06** — Font dispatches `text.setFont` over the renderer's five families (`RT_MENU` 128 ships a runtime-populated placeholder; the static list is the substitute, recorded as the evidence gap); Repeat Filter re-applies the session's last preset; the F1 groups now tile the table exactly via `SHORTCUT_GROUPS`. | A, H | — | — |
| 15 | ~~The Window menu's window-arrangement rows~~ **CLOSED 2026-11-06** — the trio (57650/57651/57652) is transcribed with CS6's labels (see §A) | A | — | — |
| 16 | HiDPI `pixels_per_point` rendering | — | 21.2 | P2 |
| 17 | accesskit desktop + web re-audit | H | 21.5 | P2 |

## L. Verification performed for this matrix

- The CS6-side inventory (panel ids, panel count, dock arrangement,
  menu-bar shape, keymap size, one-tool set) was read from the CS6
  install's own resources and `Configuration/` scripts and recorded in
  [`research/fw-ui-evidence.md`](research/fw-ui-evidence.md). The raw
  dumps are uncommitted working notes, per PLANNING §4.3.
- The FireCraft-side inventory was read from the current source
  (menu model, tool enum, panel inventory, dialog bodies, inspector
  dispatch, filter registry, shortcut table, CLI dispatch).
- Rows whose CS6 behaviour is not yet established are marked
  **evidence-gated**, not guessed.
- The 2026-11-04 inspector work (ticket 22.4) was verified by
  `cargo clippy -p app --all-targets -- -D warnings`, the app crate's
  test suite (142 unit tests plus the integration suites), and three
  new unit tests over the shared link editor's re-seed rule. The
  bitmap's fill/stroke absence is grounded in the vendored node
  dictionary (`IMGv` carries no fill/stroke node) and in decision
  0019 (`object.setBounds` refuses bitmaps); CS6's bitmap-specific
  tools are evidenced by the install's own **Image Editing** command
  panel, recorded in
  [`research/fw-ui-evidence.md`](research/fw-ui-evidence.md).
- The sampling check in the running browser at normal viewport sizes was
  performed on 2026-11-04 against a fresh optimized `xtask dist --web`
  bundle (wasm 21,271,259 B, under the 21,500,000 B ceiling). Six
  captures — empty, vector, and bitmap selection states at 1280×800 and
  1920×1080 — were taken in Chromium with the WebGL context alive in
  every one, and the objects were created through the page's own command
  surface (`object.createPath`, `bitmap.importFile`), not by poking the
  view. The tracked `docs/img/app-main.png` was replaced with the
  1280×800 vector capture. The measurements, the OCR transcripts, and the
  limits of the check are recorded in ticket **T22**.
- The first capture round for that check was **invalid and discarded**:
  the bundle in `dist/web` predated the inspector work, so the captures
  showed the pre-change fallback text instead of the new field sets. The
  receipts above come from a rebuilt bundle.
