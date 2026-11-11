# CS6 user-visible parity matrix

> Audited 2026-11-04 against the `main` tree; **revised 2026-11-07**
> after the Phase 20/21/22 landings (tabs, autosave, Preferences,
> Run Script, in-canvas text editing, and the five Window panels).
> This is the
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
all ten in the same order (`M` (MENUBAR)).

| CS6 menu | CS6 rows | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|---|
| File | 32 | 32 rows, same order; 14 rows dispatch to a documented "not available in this build" report | **partial** | `M` (File) | — |
| Edit | 25 | 25 rows; Insert submenu carries 11 of CS6's entries | **partial** | `M` (Edit) | — |
| View | 22 | 22 rows; Magnification 6%–6400%, Grid/Guides/Smart Guides submenus all dispatch | **implemented** | `M` (View) | — |
| Select | 19 | 19 rows; the four marquee modes and the bitmap-selection save/restore submenu dispatch | **implemented** | `M` (Select) | — |
| Modify | 25 | 25 rows, 11 submenus; Canvas/Animation/Symbol/Pop-up Menu/Mask/Selective JPEG/Transform/Arrange/Align/Combine Paths/Alter Path all present | **partial** | `M` (Modify) | 22.4 |
| Text | 15 | 15 rows, 5 submenus; the Font submenu dispatches `text.setFont` over the renderer's five families (CS6 populates it at runtime from the system inventory) | **implemented (static list)** | `M` (Text), `M` (Text), RT_MENU 128 re-read 2026-11-06 | 22.5 |
| Commands | 4 | 4 rows; *Manage Saved Commands…* is inert | **partial** | `M` (Commands) | 21.6 |
| Filters | 10 | 10 rows; 30 filter rows over 6 submenus; *Repeat Filter* re-applies the session's last preset | **implemented** | `M` (Filters), `S/filters_ui.rs` (`FILTER_FAMILIES`) | — |
| Window | 34 | 34 rows; every row dispatches except the `Extensions` placeholder (the only inert Window row left) | **partial** — the dock *structure* in §C is still not CS6's | `M` (Window) | 22.5, 22.1 |
| Help | 6 | 6 rows; Online Help opens the hosted wiki, Keyboard Shortcuts opens the binding table | **implemented** | `M` (Help), `S/panels.rs::help_panel` | — |

**Inert-row census.** Two of 296 rows render inert, pinned by
`menus::tests::inert_count_never_rises`. They are **Window ▸
Extensions** (the CS6 extension-package submenu placeholder) and
**Commands ▸ Manage Saved Commands…** (the scripting family, 21.6).
The Text ▸ Font placeholder and Filters ▸ Repeat Filter were wired
2026-11-06, and the five Window panels Optimize / Slices / CSS
Properties / Find and Replace / Common Library were wired the same
day (22.1) — see §C.

**Gap-reporting rows.** Thirty-five menu rows dispatch to
`ShellAction::ReportGap` — they *are* clickable and say plainly that
the feature is absent (`M`). This is the honest middle state between
"inert" and "implemented", and it is why the matrix does not read the
inert census as a coverage number.

### Open evidence item: the Window menu's window-arrangement rows — CLOSED 2026-11-06

`MENU_DESCRIPTION_MAP` names `57650 = cmd_CascadeWindows`,
`57651 = ID_WINDOW_TILE_HORZ`, `57652 = ID_WINDOW_TILE_VERT` (**R**).
FireCraft's transcription carried 57650 *Cascade* and 57652 *Tile*,
and no 57651 (**`M` id 57650**, M` id 57652`). The two open questions resolved
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

**The basis of the tile-horizontally placement.** The *command
definition* is directly evidenced by `MENU_DESCRIPTION_MAP` (**R**).
The *Window-menu placement* is not read off a decoded menu dump — the
bar resource was never decoded. It follows from CS6's own
resource-naming convention: `ID_WINDOW_…` is the command-id family
CS6 reserves for window-menu items, and the sibling
`ID_WINDOW_TILE_VERT` is unambiguously the Window menu's *Tile
Vertically* row. The row's Window-menu placement therefore rests on
the naming convention, and this matrix records it that way rather
than as a decoded-menu observation.

---

## B. Tools and flyouts

CS6's tool strip is a vertical palette (`cmd_Toolbars_Art`,
`cmd_Toolbars_Main` — **R**). FireCraft draws it in a 34 px left
panel (`S/canvas_ui.rs::tools_panel`, `S/mod.rs` (the left `Panel` arm)).

| CS6 tool area | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|
| Select / Subselect | Pointer + Subselect, both in the strip | **implemented** | `E` (`Tool`), `S/canvas_ui.rs::tools_panel` | — |
| Shape tools (rect, ellipse, line, text) | Rect, Ellipse, Line, Text | **implemented** | `E` (`Tool`) | — |
| Auto Shape tools (10 CS6 shapes) | AutoShape(star, polygon, spiral, pie, doughnut, L-shape, arrow, arrow-line, measure, connector) with a flyout gallery | **implemented** | `E` (`AutoShape`), `S/panels_web.rs::auto_shapes_panel` | — |
| Paint family (brush, pencil, eraser, bucket, gradient, eyedropper) | all six, with the CS6 Edit Brush slots | **implemented** | `E` (`Tool`), `S/panels.rs::brush_panel` | — |
| Pixel selection (marquee, elliptical, lasso, polygonal, wand) | all five, with the four blend modes | **implemented** | `E` (`Tool`), `S/panels.rs` selection panel | — |
| Web tools (slice, hotspot) | Slice, Hotspot | **implemented** | `E` (`Tool`) | — |
| View tools (zoom, hand) | Zoom, Hand | **implemented** | `E` (`Tool`) | — |
| **Flyout mechanics** (press-and-hold opens the group's list) | implemented for the strip groups and the Auto Shape gallery | **implemented** | `S/canvas_ui.rs::tools_panel` (press-and-hold detect), `S/canvas_ui.rs::tool_flyout_ui` | — |
| **Tool one-key selects** | 25 group keys (V, A, Q, C, M, L, W, B, E, R, D, S, N, P, U, T, O, Y, J, K, 2, I, G, H, Z); CS6 gives a *group* one key and repeated presses cycle within it | **implemented** — the group-cycling rule and every key come from the book's Table 1-1 and the CS6 help; CS6's `one_tool` GUID table has no tool names, so the GUID→tool mapping stays an open evidence gap | `R:fw-ui-evidence.md` (tool-strip section), `S/toolstrip.rs` | — |
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
| Properties (35586) | right-dock panel; the CS6 inspector's shared region | **implemented** | `S/panels.rs::properties_panel` | — |
| Pages (35798) | panel; page list, click selects | **implemented** | `S/panels.rs::pages_section` | — |
| States (43410, `cmd_Panels_Frames`) | panel; state list | **implemented** | `S/panels.rs::states_section` | — |
| Layers (43412) | panel; name, eye, lock, reorder | **implemented** | `S/panels.rs::layers_section` | — |
| History (43418) | panel; journal rows, Save Steps / Replay | **implemented** | `S/panels.rs::history_section` | 20.6 |
| Styles (43415) | panel; style rows, New From Selection, Import | **implemented** | `S/panels.rs::styles_panel` | — |
| Document Library (43416, `cmd_Panels_SymLibrary`) | panel; symbol rows, enter/exit symbol | **implemented** | `S/panels.rs::library_panel` | — |
| Color Mixer (43405) | panel; fill/stroke pickers + gradient editor | **implemented** | `S/panels.rs::fill_stroke_panel`, `S/panels.rs::gradient_editor_section` | — |
| Swatches (43408) | panel; 6-column grid applying fill | **implemented** | `S/panels.rs::swatches_panel` | — |
| Info (43404) | panel; canvas size, selection count, bounds — **read-only** | **partial** — CS6's Info panel is read-only too, but FireCraft's does not yet carry the per-object pixel statistics CS6 shows | `S/panels.rs::info_panel` | 22.1 |
| Auto Shapes (43422) | panel; shape gallery | **implemented** | `S/panels_web.rs::auto_shapes_panel` | — |
| URL (43414, `cmd_Panels_URLMgr`) | panel; link + alt text, Apply | **implemented** | `S/panels_web.rs::url_panel` | — |
| Behaviors (43413) | panel; list, Add menu, Add swap image | **implemented** | `S/panels_web.rs::behaviors_panel` | — |
| Tools (35058, `cmd_Toolbars_Art`) | the left tool strip | **implemented** | `S/canvas_ui.rs::tools_panel` | 22.2 |
| **Optimize** (43409, `cmd_Panels_ExportPanel`) | panel; the page's export settings (`EXPv`: format, JPEG quality/progressive, dither, GIF loop, interlace) with an Apply | **implemented** — landed 2026-11-06 | `S/panels.rs::optimize_panel`, `M` id 43409 | — |
| **Common Library** (45816) | panel; lists the document's own symbols (drag-to-place like Document Library) | **partial** — CS6's *shipped* libraries are proprietary resources and are deliberately not copied; the panel says so | `S/panels_web.rs::common_library_panel`, `M` id 45816 | 22.1 |
| **CSS Properties** (49125) | panel; the selected slice's generated CSS, read-only | **partial** — CS6 edits properties here; FireCraft's edit surfaces are the slice verbs | `S/panels_web.rs::css_properties_panel`, `M` id 49125 | 22.1 |
| **Find and Replace** (43402, `cmd_PanelFind`) | panel; find over text objects with per-match replace / replace-all (`text.setText`) | **implemented** — landed 2026-11-06 | `S/panels_web.rs::find_replace_panel`, `M` id 43402 | — |
| **Slices** | panel; the slice list with kind, link and export override, click-to-select | **implemented** — landed 2026-11-06 (CS6's tree also lists hotspots as first-class rows; FireCraft's hotspots ride the URL/Behaviors panels) | `S/panels_web.rs::slices_panel` | 20.7 |
| **Project** (43403, `cmd_PanelProject`) | none | **missing** | **R** | 22.1 |
| **Site** (43420, `cmd_Panels_Site`) | none | **missing** | **R** | 22.1 |
| **Answers** (43421, `cmd_Panels_Answers`) | none | **not applicable** — CS6 opens it from the Help menu as a web panel | **R** | — |
| **What's New** (35425, `cmd_Help_WhatsNew`) | none | **not applicable** — CS6 opens it from the Help menu | **R** | — |
| Web Layer | FireCraft's own always-present web panel (slice list, link, fg/bg type) | **modern addition** — see §J | `S/dialogs.rs::web_panel` | — |

### Panel *arrangement* (not just inventory)

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:fafccb9d7a47b510bd460b2e19723cfca2dee9e2a8f482cccf8b94df53067946 -->

| Property | CS6 | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|---|
| Right dock is **five stacked tab-groups** | yes | one flat column of 18 collapsible panels + the Web Layer panel (the inventory landed 2026-11-06; the *tab-group* structure did not) | **missing** | **R** (`Expanded Mode.xml`), `S/panel_windows.rs::dock` | 22.1 |
| Left dock holds the ToolPalette | yes | left tool panel, 34 px | **partial** — placement matches; the *toolbar* form does not | **R**, `S/mod.rs` (the left `Panel` arm) | 22.2 |
| Top dock holds the Main toolbar (778×28) | yes | a main toolbar row exists | **partial** — the row exists; its *contents* are not the CS6 main toolbar | **R** | 22.5 |
| Bottom dock holds the Properties tab-group | yes | properties live in the right dock instead | **partial** | **R** | 22.1 |
| Panels dockable left↔right, collapsible, resizable | yes | resizable + collapsible + reopenable from the Window menu; **not** left↔right movable | **partial** | `S/panel_windows.rs::dock_section` | 22.1 |
| Dock geometry persists across restart | yes | eframe `persistence` + `ShellLayout` snapshot | **implemented** | `D` 0067/0069, `S/mod.rs` | — |
| The dock **reserves** width so the canvas shrinks rather than being covered | yes (a dock) | yes — `egui::Panel::right`, 258 pt, scrolling column | **implemented** | `S/panel_windows.rs:154` | 22.5 |
| Every panel reachable at 1280×800 | yes | T22 captures and reachability check complete for the recorded build; dock scroll verified at 1280×400. The newer dock/panel inventory still needs a capture. | **verified for captured build; newer dock evidence-gated** | [T22 receipts](tasks/T22-screenshot-ui.md#receipts--2026-11-04) | 22.5 / T22 |


<!-- /coderabbit-ai-generated:v1 -->
---

## D. Property inspector, per selection kind

CS6's Properties panel swaps its field set by selection kind. FireCraft
dispatches at `S/panels.rs::properties_panel`.

| Selection | CS6 field set | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|---|
| Nothing selected | document-level fields (canvas, page, zoom) | "nothing selected" | **partial** — no document-level fields in the inspector | `S/panels.rs::properties_panel` | 22.4 |
| Vector path | X/Y/W/H, opacity, blend, visibility, lock, name, fill, stroke, stroke style, effects | X/Y/W/H, opacity, blend, visible, locked, name, fill/stroke + gradient editor, effects list | **implemented** | `S/panels.rs::path_properties`, `S/panels.rs::geometry_fields`, `S/panels.rs::display_controls` | — |
| Text | the path set **plus** font, size, bold/italic/underline, alignment, text content | all present; characters apply on Enter | **implemented** | `S/panels.rs::text_properties` | 20.1 |
| Bitmap / image | the path set, minus the fill/stroke the `IMGv` node does not carry; CS6's bitmap-specific tools live in the separate **Image Editing** command panel | name, opacity, blend, visible, locked, a pixel-size readout, and the effects list. X/Y/W/H is a readout: `object.setBounds` refuses images (decision 0019) | **partial** | `S/panels.rs::image_properties`, `R:fw-ui-evidence.md` | 22.4 |
| Symbol instance | definition, symbol number, type, placed-at, Edit/Break Apart, per-state counts, nine-slice guides | all present | **implemented** | `S/panels.rs::symbol_instance_properties` | — |
| Slice / hotspot | link, alt text, type, name, frame geometry | all present in the inspector; the link fields are the *same editor* the **URL** panel renders, so the two cannot disagree | **implemented** | `S/panels.rs::slice_properties`, `S/panels_web.rs::slice_link_editor` | — |
| Group | member list, enter-group | member list, display controls, geometry; **no** enter-group | **partial** | `S/panels.rs::group_properties` | 22.4 |
| Effects list | per-effect rows with per-parameter fields, removable, addable | present for path, text, **group and bitmap**; **not** for instances (the function returns early) | **partial** | `S/panels.rs::filter_panel`, `S/panels.rs::filter_param_rows` | 22.4 |

---

## E. Document workflows

| CS6 workflow | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|
| New / Open / Save / Save As / Close | present | **implemented** | `M` (File) | — |
| New from Template / Save As Template | both report "not available in this build" | **missing** (documented refusal) | `M` id 37866, `M` id 37867 | — |
| Revert | present | **implemented** | `M` id 35036 | — |
| Undo / Redo | present | **implemented** | `M` (Edit) | — |
| Save All / Close All | present | **implemented** | `M` id 35850, `M` id 35789 | — |
| Open Recent | one inert-ish row that reports the gap | **missing** (documented refusal) | `M` (File ▸ Open Recent) | — |
| Export Wizard | present | **implemented** | `M` id 35181 | — |
| Batch Process | reports the gap | **missing** (documented refusal) | `M` id 35302 | — |
| **Multi-document tabs** | open a second document parks the first in a tab strip; Close promotes the last parked tab, dirty documents refuse to close silently, the strip folds away in Full Screen (**20.5, landed 2026-11-07**) | **implemented** | `M` id 57648, `S/mod.rs::activate_tab` | — |
| Print / Page Setup | report the gap (no printing) | **not applicable** — FireCraft has no printing story | `M` id 57605, id 57607 | — |
| **Autosave / crash recovery** | web: whole-document localStorage persistence with boot restore; desktop: journal sidecar snapshot with a Recover/Discard offer, undo intact across the boundary (**21.1, landed 2026-11-07**) | **implemented** | `S/mod.rs::check_recovery`, `S/mod.rs::recovery_ui` | — |
| **Preferences dialog** | Edit ▸ Preferences… opens a real dialog (UI scale, persisted in `ShellLayout`) (**21.4, landed 2026-11-07**) | **implemented** | `M` id 35034, `S/panels.rs::preferences_ui` | — |
| **Keyboard Shortcuts editor** | the Help row opens a *read-only* binding table, not an editor; the four alternative keymap sets are absent | **partial** | `S/panels.rs::help_panel` | 21.4 |
| Save Steps As Command / Replay | History panel's Save Steps…/Replay… | **implemented** | `S/panels.rs::history_section` | 20.6 |
| **In-app scripting** (Commands ▸ Run Script…) | Run Script runs `.jsf` through the scripting host and `.fwsteps` through journal replay (**21.6, landed 2026-11-07**); the sibling *Manage Saved Commands…* row is still inert and CS6's extension installer is a recorded gap | **partial** — the runner is there, the saved-command manager is not | `M` id 35319, `S/mod.rs::run_script_file` | 21.6 |

---

## F. Import and export surfaces

| CS6 direction / format | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|
| Import PNG, JPEG, GIF, BMP, TIFF, WebP, WBMP | all present | **implemented** | `crates/io` | — |
| Import `.fw.png` (native) | present | **implemented** | `crates/fwpng` | — |
| Export PNG, JPEG (baseline + progressive), BMP, TIFF, WBMP, WebP, GIF | all present | **implemented** | `crates/io`, `S/dialogs.rs` (`EXPORT_FORMATS`) | — |
| Export SVG, PSD (layered + merged) | present | **implemented** | `crates/io` | — |
| Export web (images, HTML, CSS, sprites, CSS properties, pages, states, layers, animated GIF) | 9 workflows in the Export dialog | **implemented** | `S/dialogs.rs` (`WEB_WORKFLOWS`) | — |
| JPEG quality, progressive, matte | present (JPEG-only controls) | **implemented** | `S/dialogs.rs` | — |
| GIF loop, repeat, crop/diff frames, dither | present (GIF-only controls) | **implemented** | `S/dialogs.rs` | — |
| **Update HTML** (re-export an existing site) | reports the gap | **missing** (documented refusal) | `M` id 35416 | 20.7 |
| **HTML Setup** (export options for a site) | reports the gap | **missing** (documented refusal) | `M` id 35452 | 20.7 |
| **Set Primary / Secondary Browser** | reports the gap (system default is used) | **partial** — preview works; browser choice does not | `M` id 35314 / id 35316 | 20.7 |
| Send to E-mail | reports the gap | **not applicable** | `M` (File ▸ Send to E-mail) | — |
| Scan (TWAIN) | reports the gap | **not applicable** | `M` (File ▸ Scan) | — |
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
| Command palette over the whole registry (Ctrl+P) | present; every registry command is executable from the GUI | **implemented** | `S/palette.rs::run_palette_command`, 19.4 | 21.3 |
| **In-canvas text editing** (type on canvas, caret, double-click to edit) | double-click a text object to edit it in place: an egui `TextEdit` overlay on the text frame, commit on Enter/defocus, cancel on Esc, focus routing that keeps the canvas from stealing the caret (**20.1, landed 2026-11-06**) | **implemented** | `S/mod.rs::begin_text_edit`, `S/mod.rs::commit_text_edit`, `S/mod.rs::text_edit_session_buffers_and_commits` | — |
| Gradient / paint / pixel-selection canvas gestures | present | **implemented** | `E` (`Tool`) | — |
| Multi-selection and multi-object transforms | present | **implemented** | `S/panels.rs::multi_selection_properties` (align/distribute) | — |

---

## H. Keyboard access

| CS6 capability | FireCraft | Verdict | Evidence | Ticket |
|---|---|---|---|---|
| Menu accelerators (Ctrl+N/O/S/Z, Ctrl+Shift+S/R, Alt+F4 …) | carried on the menu rows | **implemented** | `M:459` | — |
| A working shortcut table (137 bindings) | present and dispatched, and skipped while a text field has focus; no two bindings share a (modifiers, key) pair | **implemented** | `M:SHORTCUTS`, `S/mod.rs:shortcuts` | — |
| F1 lists the bindings | present, in 43 labelled groups | **implemented** — the group layout is `menus::SHORTCUT_GROUPS`, asserted to tile the 137-row table exactly (`the_shortcut_groups_cover_the_table`; the mismatch that summed to 49 was fixed 2026-11-06) | `S/panels.rs:94`, `S/menus.rs` | — |
| **Tool one-key selects** | 24 group keys bound; repeated presses cycle within the group, as CS6's help describes | **implemented** — see §B | `S/toolstrip.rs`, `S/menus.rs` | — |
| **CS6's full keymap** (the `Fireworks.xml` set: 128 command ids) | 37 of the 40 live menu rows that print an accelerator are bound. Those 40 rows carry 40 distinct CS6 command ids; the transcribed menu tree tracks 261 command ids across 295 rows, of which 41 print an accelerator | **partial** — the three printed accelerators with no binding are *Close* (`Ctrl+F4`), *Snap to Grid* (`Ctrl+Alt+Shift+G`) and *Split* (`Ctrl+Shift+J`); the `feat/keymap-cs6-bindings` branch binds every printed CS6 accelerator — 120 binding entries, 106 printed accelerators, 100 live rows, none unbound — and adds the parity gate (`menus::tests::cs6_keymap_is_fully_bound`) that keeps them bound. The 37/40 figure is a static measurement of `menus::SHORTCUTS` against the menu tree — no test on this branch asserts it | `M:SHORTCUTS`, `M:tests::the_shortcut_groups_cover_the_table`, `R` (keymap section) | 22.5 |

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
| Command palette (Ctrl+P) over the whole registry | CS6 had no palette; FireCraft's registry is 252 commands and a palette makes every one reachable | `S/palette.rs` |
| Rust-native `.fw.png` engine, CLI, journal, MCP server | headless automation and scripting parity beyond CS6's surface | `crates/app-cli` |
| ExtendScript host running the CS6 `.jsf` corpus | scripting parity, headless | `crates/scripting` |
| The **Web Layer** panel | FireCraft's own always-present web panel; CS6's web state lives in the Properties/Slices panels | `S/dialogs.rs::web_panel` |
| Live-effects inspector with per-parameter rows | CS6's inspector is field-based; FireCraft's effect list is engine-backed and editable | `S/panels.rs::filter_panel`, `S/panels.rs::filter_param_rows` |
| A 2-Up export preview inside the Export dialog | FireCraft's own convenience | `S/dialogs.rs` |

---

## K. Gap ledger

Every **missing** and **partial** row above, with its owner ticket and
acceptance hook. Ordered by how much of the product it unlocks.

| # | Gap | Rows | Ticket | Priority |
|---|---|---|---|---|
| 1 | ~~The five missing CS6 panels (Optimize, Common Library, CSS Properties, Find and Replace, Slices)~~ **CLOSED 2026-11-06** — all five have Window-menu rows and real content (`S/panels.rs::optimize_panel`, `S/panels_web.rs::slices_panel`, `::css_properties_panel`, `::find_replace_panel`, `::common_library_panel`); **Project** and **Site** (CS6 panels with no FireCraft counterpart) remain **missing** | C | 22.1 | P0 |
| 2 | The inspector's remaining per-kind gaps: document-level fields, multi-selection fields, the image transform strip, and enter-group | D, I | 22.4 | P0 |
| 3 | The right dock's five-tab-group structure and the bottom Properties dock — the **panel inventory landed 2026-11-06** (18 panels in CS6's order, collapse/close headers, scrolling column); what is left is the *structure*: CS6 stacks them as five tab-groups with Properties in the bottom dock, FireCraft runs one flat column | C | 22.1 | P0 |
| 4 | ~~In-canvas text editing~~ **CLOSED 2026-11-06** — double-click edits in place: `TextEdit` overlay on the text frame, commit/cancel/focus routing (§G) | E, H, I | 20.1 | — |
| 5 | The CS6 tool strip's remaining items: the 3D tool (the screen-mode cycle and the Alt-hold variants **landed 2026-11-06**) | B | 22.5 | P2 |
| 6 | ~~Multi-document tabs~~ **CLOSED 2026-11-07** — park on Open/New, Close promotes the last tab, dirty dots, the strip folds in Full Screen (§E) | E | 20.5 | — |
| 7 | The animation timeline (States panel) | C | 20.3 | P1 |
| 8 | History click-to-rewind | C, E | 20.6 | P1 |
| 9 | Slice/hotspot behaviour editing and the HTML preview surface | C, F, I | 20.7 | P1 |
| 10 | Autosave and crash recovery — **LANDED 2026-11-07** — web: whole-document localStorage persistence with boot restore; desktop: journal sidecar snapshot with Recover/Discard prompt, undo intact across the crash boundary | E | 21.1 | P1 |
| 11 | The Keyboard Shortcuts editor and the four alternative keymaps — the **Preferences dialog CLOSED 2026-11-07** (§E) | E, H | 21.4 | P1 |
| 12 | The in-app script console and saved commands — **LANDED 2026-11-07**: Commands ▸ Run Script runs `.jsf` (scripting host, landed with automation) and `.fwsteps` (journal replay onto the live document); CS6's broader extension installer remains a recorded gap | A, E | 21.6 | P1 |
| 13 | ~~Command palette v2 (fuzzy, recents, save-steps)~~ **CLOSED 2026-11-07** — fuzzy subsequence scoring with a position bonus; recents persisted in `ShellLayout`; save-steps-as-command is the History panel's Save Steps…/Replay… journal file pair. | G | 21.3 | P2 |
| 14 | ~~The Text ▸ Font submenu, *Repeat Filter*, the F1 grouping mismatch~~ **CLOSED 2026-11-06** — Font dispatches `text.setFont` over the renderer's five families (`RT_MENU` 128 ships a runtime-populated placeholder; the static list is the substitute, recorded as the evidence gap); Repeat Filter re-applies the session's last preset; the F1 groups now tile the table exactly via `SHORTCUT_GROUPS`. | A, H | — | — |
| 15 | ~~The Window menu's window-arrangement rows~~ **CLOSED 2026-11-06** — the trio (57650/57651/57652) is transcribed with CS6's labels (see §A) | A | — | — |
| 16 | ~~HiDPI `pixels_per_point` rendering~~ **CLOSED 2026-11-07** — the canvas raster renders at zoom × pixels_per_point under the 16 MiPix budget; the canvas is crisp on hidpi displays at any zoom | — | 21.2 | P2 |
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
