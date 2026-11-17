# P6 — CS6 surface: tools, panels, dialogs and menus

**Goal:** the UI a CS6 user expects is there:
- every tool;
- every panel that is not legacy;
- the CS6 dock layout;
- a shortcut editor;
- no "not available in this build" rows, except those recorded as out of
  scope.

**Exit check:**
- S3 (ReportGap rows) ≤ 8, and every one left is listed in C6.30's
  out-of-scope decision;
- S5c ≤ 3;
- S4 = 0.

**Lane:** shell. Needs C2.04–C2.06. Rungs are independent unless `Needs`
says otherwise.

**Pattern for any rung that replaces a `ReportGap` row:**
1. Implement the feature.
2. Swap the row's `Action::Shell(ShellAction::ReportGap(..))` for the real
   action.
3. Add a shell test that activates the row and asserts the effect.
4. Update the `docs/CS6-PARITY.md` row.
5. Run `xtask climb --append`.

The menu census test `inert_count_never_rises` keeps the score honest.

**Evidence rule:** CS6 behaviour comes from these sources, recorded in a
decision when it is non-obvious:
- the help text;
- `Configuration/` `.jsf` scripts;
- `docs/research/fw-ui-evidence.md`;
- observed behaviour under Wine (HANDOFF oracle).

Never copy proprietary assets (icons, patterns, libraries, templates).

---

## A. Tools

### C6.01 — Path Scrubber tools (additive/subtractive) `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Add both tools to the Pen group in `crates/app/src/toolstrip.rs`.
  2. Dragging along a path adjusts the per-point pressure or width that the
     stroke renderer already honours. Check `crates/vector` for the
     pressure field.
  3. If no such field exists, write an `[EVIDENCE]` decision first.
- **Done when:** a drag changes the stroke width along a pressure-sensitive
  brush stroke; there is a shell test.
- **Verify:** `cargo test -p app scrubber`; a browser proof.
- **Moves:** S5c −1.

### C6.02 — Crop Selected Bitmap `[ ]`
- **Needs:** — · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Add an engine verb `bitmap.cropSelected` in `catalog/bitmap.rs`. It
     crops the bitmap's pixels to the marquee or a dragged rectangle, and
     needs an undo test.
  2. Replace the Edit row's ReportGap (`menus.rs` ≈922).
- **Done when:** the row works and the registry count goes up; bind it in the
  same PR.
- **Verify:** `cargo test -p engine crop`; `cargo test -p app`.
- **Moves:** S3 −1, S2 +1 / S2_registry +1.

## B. Panels

### C6.03 — Path panel `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Add a panel that groups the path verbs already in the registry: combine
     (union, intersect, punch, crop), alter (simplify, expand, inset), and
     points/select.
  2. Its buttons dispatch existing `path.*` commands.
  3. Add it to the Window menu at CS6's position.
- **Done when:** every button runs its command on the selection, with a shell
  test per group.
- **Verify:** `cargo test -p app path_panel`.
- **Moves:** S5c −1.

### C6.04 — Image Editing panel `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:** add a panel of bitmap tools and verbs (marquee, lasso, magic
  wand, brush, eraser, blur/sharpen, dodge/burn, rubber stamp, replace
  color, red-eye, plus the bitmap filters). It mirrors CS6's command panel.
- **Done when:** the panel exists and each button selects its tool or runs its
  command.
- **Verify:** a shell test.
- **Moves:** S5c −1.

### C6.05 — Special Characters panel `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. Show a grid of common special characters (©®™ •…–— ¡¿ €£¥ and so on).
  2. Clicking a character inserts it at the text caret through the existing
     text-insert command.
- **Done when:** inserting into a text block works; there is a shell test.
- **Verify:** `cargo test -p app special_chars`.
- **Moves:** S5c −1.

### C6.06 — Info panel pixel statistics `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:** add CS6's live readouts to `panels.rs::info_panel`:
- **Readouts:**
  - the pointer position;
  - the color under the pointer (RGB/hex, with alpha);
  - the selection's W/H/X/Y.
- **Done when:** the readouts update on hover, and a unit test covers the
  sampling helper.
- **Verify:** `cargo test -p app info_panel`.
- **Moves:** S5b −1.

### C6.07 — Editable CSS Properties panel `[ ]`
- **Needs:** C4.16 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:** make `css_properties_panel` editable. Edits write back through
  the slice and web verbs, and the export uses them.
- **Done when:** an edited property appears in the exported CSS, with a golden
  test.
- **Verify:** `cargo run -p xtask -- web-golden --measure`.
- **Moves:** S5b −1.

### C6.08 — Common Library: FireCraft-original symbols `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` `[DOCS]` · **Size:** M
- **Steps:**
  1. Author a small original symbol set: buttons, arrows, UI widgets and
     bullets, saved as FireCraft `.png`/`.graphic.png` files under
     `assets/common-library/`.
  2. Add an `ATTRIBUTION.md` row for each file, in the same PR.
  3. The panel lists them by category. Dragging one places an instance.
- **Done when:** at least 3 categories exist, the attribution rows are there,
  and drag-to-place works.
- **Verify:** a shell test plus a browser proof.
- **Moves:** S5b −1.

### C6.09 — CS6 tab-group dock as the default (#27 part) `[ ]`
- **Needs:** C2.09 · **Tags:** `[HOST:browser]` · **Size:** L
- **Steps:**
  1. `panel_windows.rs::dock` stacks 18 panels in one flat column. Implement
     tab groups: several panels share a frame and switch by tab.
  2. The default layout follows CS6's five right-dock groups. The source is
     `docs/research/fw-ui-evidence.md`; record the layout in a decision.
  3. Persist the layout in `ShellLayout`, which already serializes. Bump its
     version and migrate old layouts to the default.
  4. Add Window ▸ Workspace Layouts ▸ Reset.
- **Done when:**
  - the default layout matches the decision;
  - tabs switch;
  - the layout persists across restarts;
  - a migration test passes.
- **Verify:** `cargo test -p app layout`; browser screenshot vs the decision's
  diagram.
- **Moves:** CS6-PARITY §C "missing" row → implemented.

### C6.10 — Main toolbar contents (#27) `[ ]`
- **Needs:** C6.09 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. Fill the existing toolbar row with CS6's main toolbar buttons: New,
     Open, Save, Import, Export, Print; Undo/Redo; Group/Ungroup; Join/Split;
     Align; Rotate/Flip; Bring forward/back.
  2. Icons must be FireCraft originals (`shell/icons.rs`); never copy CS6's.
- **Done when:** each button dispatches, and the CS6-PARITY row "Top dock" is
  implemented.
- **Verify:** a shell test plus a browser proof.
- **Moves:** S5b −1.

### C6.11 — Properties panel in the bottom dock `[ ]`
- **Needs:** C6.09 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:** CS6 docks Properties horizontally at the bottom. Make that the
  default, with a wide layout that matches CS6's field order. The right dock
  stays an option.
- **Done when:** the default layout puts Properties at the bottom, and it
  persists.
- **Verify:** a browser screenshot.
- **Moves:** S5b −1.

### C6.12 — Inspector remainder (#88) `[ ]`
- **Needs:** — · **Tags:** `[EVIDENCE]` `[HOST:browser]` · **Size:** M
- **Steps:** close the 22.4 partial rows in CS6-PARITY:
  - nothing selected → document fields (canvas size, colour, page);
  - group → "enter group";
  - instance → effects list (the function currently returns early);
  - multiple selection → shared-field editing.
  The bitmap X/Y/W/H refusal (decision 0019) stays unless new evidence
  changes it.
- **Done when:** each listed row is implemented, with a shell test each.
- **Verify:** `cargo test -p app properties`.
- **Moves:** S5b −3…4; #88 narrows or closes.

## C. Dialogs and menu gaps (ReportGap burn-down)

### C6.13 — Open Recent `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. Keep a list of up to 10 recent files, with clear-list.
     - desktop: paths in preferences;
     - web: `FileSystemFileHandle`s in IndexedDB when C3.15 has landed,
       otherwise names that re-prompt the picker.
  2. Replace the ReportGap row (`menus.rs` ≈562) with a submenu.
- **Done when:** open, restart, and the file appears in Open Recent and
  reopens.
- **Verify:** a shell test plus a browser reload proof.
- **Moves:** S3 −1, S5c −1.

### C6.14 — Templates (New from Template / Save as Template) `[ ]`
- **Needs:** C6.13 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. Save as Template writes the document to the user template folder
     (IndexedDB on web).
  2. New from Template lists the folder plus FireCraft-original starter
     templates (web banner sizes, mobile screens, icon grid), each with an
     `ATTRIBUTION.md` row.
  3. Replace both ReportGap rows (≈543, ≈614).
- **Done when:** a round trip works; there is a shell test.
- **Verify:** `cargo test -p app template`.
- **Moves:** S3 −2, S5c −1.

### C6.15 — Keyboard shortcut editor (#92) `[ ]`
- **Needs:** C2.06 · **Tags:** `[HOST:browser]` · **Size:** L
- **Steps:**
  1. Make the read-only table in `help_panel` editable:
     - select a command, press keys to bind them;
     - a conflict warning names the other command;
     - Reset.
  2. Store user bindings in preferences, overriding `menus` keymap tables;
     the tests `cs6_keymap_entries_are_bound` must still pass for the
     default set.
  3. Replace the Edit ▸ Keyboard Shortcuts ReportGap (≈943).
- **Done when:** a rebound key works after a restart, a conflict is detected,
  and there is a unit test of the override merge.
- **Verify:** `cargo test -p app keymap`.
- **Moves:** S3 −1, S5b −1.

### C6.16 — Alternative keymap sets (#92) `[ ]`
- **Needs:** C6.15 · **Tags:** `[EVIDENCE]` · **Size:** M
- **Steps:**
  1. CS6 ships Illustrator, Photoshop, FreeHand and Web Standard sets. The
     binding *facts* (key ↔ action) are behaviour, not copyrightable
     expression. Build them from the help documentation's shortcut tables;
     record the source per set in a decision.
  2. Add a set picker in the editor.
- **Done when:** switching sets changes the bindings; there is a test per set
  for 5 sample bindings.
- **Verify:** `cargo test -p app keymap_sets`.
- **Moves:** S5c −1.

### C6.17 — Manage Saved Commands `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. Add a dialog listing the saved commands: user `.jsf` and `.fwsteps`
     files in the commands folder (IndexedDB on web). It supports rename,
     delete and run.
  2. Saved commands appear under the Commands menu.
  3. Replace the inert row (S4).
- **Done when:** a saved History panel command appears in the menu, can be
  renamed and deleted, and has a shell test.
- **Verify:** `cargo test -p app saved_commands`.
- **Moves:** S4 −1, S5b −1.

### C6.18 — Preferences dialog: CS6 tabs `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Preferences holds only UI scale. Add CS6's tabs, keeping only settings
     FireCraft actually honours:
     - **General:** undo steps, color defaults, interpolation;
     - **Edit:** pen preview, pick distance, brush cursors;
     - **Guides & Grids:** colours and snap distance;
     - **Type:** default font, smoothing;
     - **Import:** PSD/AI options that `io` supports.
  2. Each setting must be read somewhere. Add a test per setting proving it
     is honoured.
- **Done when:** every visible setting has a test.
- **Verify:** `cargo test -p app prefs`.
- **Moves:** S5b −1.

### C6.19 — Edit Guides dialog + Snap To Pixel `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. View ▸ Guides ▸ Edit Guides opens a dialog with the guide colour, show,
     snap and lock settings, and Clear All.
  2. Snap To Pixel snaps vector points and bounds to whole pixels. It is a
     view snap mode, used in the canvas drag code.
  3. Replace three ReportGap rows (≈1730, ≈2656, ≈2893).
- **Done when:** both work; there is a shell test for the snapped
  coordinates.
- **Verify:** `cargo test -p app snap_pixel`.
- **Moves:** S3 −3.

### C6.20 — Selection history (Previous/Next Selection) `[ ]`
- **Needs:** — · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Keep a bounded ring of selection sets per document.
  2. Previous and Next walk the ring; a stale object ID is skipped.
  3. Replace both rows (≈3423, ≈3431).
- **Done when:** walking the history restores the earlier selections; there
  is a test.
- **Verify:** `cargo test -p app selection_history`.
- **Moves:** S3 −2.

### C6.21 — Paste Attributes and Copy as Vectors / Copy HTML Code `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. **Paste Attributes:** copy the fill, stroke, effects, opacity and blend
     of the copied object onto the selection, as one undoable command.
  2. **Copy as Vectors:** put SVG text on the OS clipboard, reusing the svg
     export.
  3. **Copy HTML Code:** put the slice HTML for the selection on the
     clipboard, reusing `webexport`.
  4. On web, use `navigator.clipboard.writeText`.
  5. Replace rows ≈869, ≈877, ≈900 and ≈3375.
- **Done when:** each works; there is a shell test per verb, plus a browser
  clipboard proof.
- **Verify:** `cargo test -p app paste_attributes`.
- **Moves:** S3 −4.

### C6.22 — Swap Symbol `[ ]`
- **Needs:** C5.05 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. Add an engine verb that re-targets an instance to another library
     symbol, keeping its transform.
  2. Add a dialog to pick the symbol.
  3. Replace row ≈1360.
- **Done when:** the instance renders the new symbol in place, undo works,
  and there is a test.
- **Verify:** `cargo test -p engine swap_symbol`.
- **Moves:** S3 −1, S2_registry +1.

### C6.23 — Pop-up menu editor (#93 part) `[ ]`
- **Needs:** C4.16 · **Tags:** `[EVIDENCE]` `[HOST:browser]` · **Size:** L
- **Steps:**
  1. CS6's Pop-up Menu Editor has four tabs: Content (label, link, target),
     Appearance (HTML/image, colours), Advanced (cell size, padding, delay)
     and Position.
  2. Store the menu in the slice behaviour structure. Establish the node
     encoding from the dictionary and corpus first, in a decision.
  3. Export as CSS/JS menus. The modern output is plain CSS hover menus plus
     a tiny JS snippet; do not copy CS6's `mm_menu.js`.
  4. Replace three rows (≈1373–1390).
- **Done when:** a pop-up menu exports and works in a browser (Playwright
  hover test).
- **Verify:** `cargo run -p xtask -- web-golden --measure`; a Playwright
  hover test.
- **Moves:** S3 −3, S5c −1.

### C6.24 — Slice behaviours (#93) `[ ]`
- **Needs:** C6.23 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Add a Behaviors panel: Simple Rollover, Swap Image, Swap Image Restore,
     Set Nav Bar Image, Set Text of Status Bar.
  2. Export them as modern JS: event listeners, not `MM_swapImage`.
- **Done when:** the rollover golden test passes and the Playwright hover
  swaps the image.
- **Verify:** web-golden plus Playwright.
- **Moves:** #93 closes.

### C6.25 — Spell check `[ ]`
- **Needs:** — · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. Write a decision choosing a pure-Rust, wasm-safe spell checker
     (`spellbook`, the Hunspell-compatible crate, if its licence fits) and a
     permissively licensed English dictionary. The dictionary needs an
     `ATTRIBUTION.md` row.
  2. Text ▸ Check Spelling walks the text objects; Spelling Setup sets the
     language and a personal dictionary.
  3. Replace rows ≈1920 and ≈1928.
- **Done when:** a misspelled word is found and replaced, and the personal
  dictionary persists.
- **Verify:** `cargo test -p app spelling`.
- **Moves:** S3 −2, S5c −1.

### C6.26 — Text: custom size dialog + vertical alignment `[ ]`
- **Needs:** — · **Tags:** `[EVIDENCE]` · **Size:** S
- **Steps:**
  1. Text ▸ Size ▸ Other opens a numeric dialog (row ≈1776).
  2. Vertical text alignment (four rows, ≈1844–1868) is blocked on a format
     fact. Establish the node or field from the dictionary and corpus in a
     decision, then implement the verb. If it is unprovable, record it as
     evidence-gated and leave the rows.
- **Done when:** the size dialog works, and vertical alignment is either
  implemented or recorded as gated.
- **Verify:** `cargo test -p app text_size`.
- **Moves:** S3 −1…−5.

### C6.27 — Print and Page Setup `[ ]`
- **Needs:** — · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. **Web:** render the page to an image in a hidden iframe and call
     `window.print()`.
  2. **Desktop:** export a temporary PDF (C4.15) and open it with the system
     viewer. Record that approach in a decision.
  3. Replace rows ≈748 and ≈756.
- **Done when:** the print path is exercised; the web test checks that the
  print hook is called.
- **Verify:** a shell test.
- **Moves:** S3 −2.

### C6.28 — Preview browser selection `[ ]`
- **Needs:** — · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. **Desktop:** pick a browser executable for the primary and secondary
     preview; F12 uses it.
  2. **Web:** the rows stay a recorded refusal, since a browser cannot launch
     another browser. Put the reason in the row text.
  3. Rows ≈729 and ≈737.
- **Done when:** desktop launches the chosen executable (unit test with a
  fake launcher).
- **Verify:** `cargo test -p app preview_browser`.
- **Moves:** S3 −2 on desktop.

### C6.29 — Animation and mask residuals (#24, #114) `[ ]`
- **Needs:** C5.04, C5.11, C5.14 · **Tags:** `[HOST:browser]` `[MAINTAINER]`
  (acceptance) · **Size:** S
- **Steps:**
  1. Re-read #24 and #114.
  2. Implement any residual not covered by C4.20, C5.04 and C5.11–C5.14.
  3. Capture the acceptance screenshots that #114 asks for, and attach them.
- **Done when:** the maintainer accepts, and #24 and #114 are closed.
- **Verify:** issue state.
- **Moves:** S11 −2.

### C6.30 — Out-of-scope decision for the remaining refusals `[ ]`
- **Needs:** C6.13–C6.28 · **Tags:** `[DOCS]` · **Size:** XS
- **Steps:**
  1. Write `docs/decisions/NNNN-cs6-rows-out-of-scope.md`. It lists every
     ReportGap row left, with its reason:
     - Adobe Bridge (×2): proprietary product;
     - TWAIN (×2): obsolete scanner API with no web equivalent;
     - jQuery widget panel: Adobe-hosted service;
     - Project/Site panels: legacy, tied to Dreamweaver;
     - Extensions: Adobe Extension Manager.
  2. Each row's text must say "not part of FireCraft", not "not available
     in this build".
- **Done when:** S3 equals that decision's row count, which is at most 8.
- **Verify:** `cargo run -q -p xtask -- climb`.
- **Moves:** S3 at summit.
