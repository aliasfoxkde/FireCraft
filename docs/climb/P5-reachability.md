# P5 — Reachability: every command a user can reach

**Goal:** each of the 57 commands still unbound after #143 (AUDIT §7) is one
of two things:
- bound to a designed surface: a menu, panel, dialog or tool;
- recorded as **script-only by decision**, as CS6 itself exposes some of
  these only to `.jsf`.

Issues #136–#141 close.

**Exit check:**
- `xtask reachability --list` prints only allowlisted IDs;
- S2 = 253 − (allowlist size);
- #136–#141 are closed.

**Lane:** shell. Needs C0.07, C2.04/C2.05 and C2.06, so that the splits
prevent conflicts. Rungs C5.01–C5.20 are independent of each other unless
`Needs` says otherwise. Claim one per PR.

---

## The wiring pattern (every C5 rung follows it)

1. **Find the engine command.** Look in
   `crates/engine/src/catalog/<family>.rs`, then read its parameters and its
   doc comment. If the command's behaviour is incomplete, fix it there
   first, with an engine test.
2. **Pick the CS6 surface.** Find where CS6 shows the command:
   - the CS6 menu tree (`docs/planning/generated/dom-parity.md` names the
     DOM verb);
   - `docs/CS6-PARITY.md`;
   - the CS6 help text (behavioural; working notes per PLANNING §5.4).
   If CS6 shows it nowhere, go to C5.00's allowlist and do not invent UI.
3. **Wire it:**
   - menu rows go in `crates/app/src/menus/<menu>.rs`;
   - dispatch arms go in `shell/dispatch_sel.rs` or `dispatch_shell.rs`;
   - panel controls go in `shell/panels*.rs`.
   Pass the literal command ID to the engine; that is what the gate scans.
4. **Shell test.** Drive the surface (menu activation or panel interaction)
   and assert the document change. Use the existing
   `shell::tests` helpers, the same way as the tests landed by #143.
5. **Ratchet:**
   - `cargo run -p xtask -- reachability` shows the count rising;
   - then run `--bless`;
   - then `cargo run -q -p xtask -- climb --append C5.nn`.
6. **Parity ledger.** Update the row in `docs/CS6-PARITY.md`, and
   regenerate `dom-parity.md` (HANDOFF §5).
7. **Issue.** Tick the command ID in its issue (#136–#141) and cite the PR.

**Done when (every rung):**
- the listed IDs are absent from `reachability --list`;
- the shell test passes;
- the gate is green.

**Verify (every rung):**
- `cargo run -q -p xtask -- reachability --list | grep -E '<ids>'` prints
  nothing;
- `cargo run -p xtask -- ci`.

---

### C5.00 — Script-only allowlist for the reachability gate `[ ]`
- **Needs:** C0.07 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Add `.reachability-script-only.tsv`, with columns `command_id`,
     `decision`, `reason`.
  2. `xtask reachability` treats a listed ID as accounted for:
     - it is not counted as bound;
     - it is removed from the unreachable list;
     - it is reported in a separate `script_only` count.
  3. The gate refuses a row whose `decision` file does not exist in
     `docs/decisions/`. Add a unit test for that refusal.
  4. Write `docs/decisions/NNNN-script-only-commands.md`. It states the
     test: "CS6 exposes this only to scripting (DOM verb, no menu, panel
     or tool)". Start it with zero rows; rungs add rows as they prove them.
- **Done when:** an allowlisted ID disappears from `--list`, and a row with a
  bogus decision fails the gate.
- **Verify:** `cargo test -p xtask reachability`.
- **Moves:** gives P5 a finish line that does not need invented UI.

### C5.01 — Text character and font markup (#136, 4) `[ ]`
- **IDs:** `text.applyCharacterMarkup`, `text.removeCharacterMarkup`,
  `text.applyFontMarkup`, `text.removeFontMarkup`
- **Needs:** C0.02 · **Tags:** `[HOST:browser]` · **Size:** M
- **Surface:**
  - the Properties/Text panel's bold, italic and underline toggles, and the
    font picker, applied to the **selected run** while editing text;
  - the Text menu ▸ Style rows.
- **Note:** this must be the same restyle path that
  `the_markup_verbs_restyle_the_runs` (fixed in C0.02) covers. Extend that
  test; do not write a parallel one.
- **Moves:** S2 +4.

### C5.02 — Button text replace (#136, 2) `[ ]`
- **IDs:** `text.replaceButtonTextStrings`,
  `text.replaceButtonTextStringsInInstances`
- **Needs:** C5.03 · **Tags:** `[HOST:browser]` · **Size:** S
- **Surface:** the button-instance Properties panel has a "Text" field. Its
  edit updates either the instance only or all instances. CS6 shows this
  field on a button instance.
- **Moves:** S2 +2.

### C5.03 — Button symbol options (#137, 5) `[ ]`
- **IDs:** `symbol.setButtonOptions`, `symbol.setButtonAutoSlice`,
  `symbol.setButtonIncludeDownState`,
  `symbol.setButtonIncludeOverWhileDownState`,
  `symbol.setButtonShowDownOnLoad`
- **Needs:** C2.04 · **Tags:** `[HOST:browser]` · **Size:** M
- **Surface:**
  - the Button editor's states: Up, Over, Down, Over While Down and Active
    Area;
  - its options: "Include Down state", "Include Over While Down state",
    "Show Down state on load";
  - auto-slice on the Active Area.
- **Moves:** S2 +5.

### C5.04 — Animated symbol instance properties (#137, 6) `[ ]`
- **IDs:** `symbol.setAnimInstanceLoopCount`, `…NumFrames`, `…OffsetDist`,
  `…RotationAmount`, `…ScaleAmount`, `…StartEndOpacity`
- **Needs:** C2.04 · **Tags:** `[HOST:browser]` · **Size:** M
- **Surface:**
  - Modify ▸ Animation ▸ Settings… opens a dialog with these fields:
    frames, move (distance and direction), scale to, opacity (from/to),
    rotate (amount and direction), and loop;
  - the Properties panel shows the same fields for an animation instance.
- **Moves:** S2 +6; the #24 residual shrinks.

### C5.05 — Symbol library operations (#137, 3) `[ ]`
- **IDs:** `symbol.duplicateSymbolForAlias`, `symbol.editDetach`,
  `symbol.editReorder`
- **Needs:** C2.04 · **Tags:** `[HOST:browser]` · **Size:** S
- **Surface:**
  - the Document Library panel menu: Duplicate;
  - Modify ▸ Symbol ▸ Break Apart / Detach;
  - drag-reorder in the library list.
- **Check:** confirm each verb's CS6 surface in `dom-parity.md`. Any verb with
  no surface goes to C5.00.
- **Moves:** S2 +3.

### C5.06 — Symbol custom data and params (#137, 2) `[ ]`
- **IDs:** `symbol.setCustomData`, `symbol.setParam`
- **Needs:** C5.00 · **Tags:** `[EVIDENCE]` · **Size:** S
- **Steps:**
  1. CS6 exposes `setParam` through the Symbol Properties panel (rich symbol
     parameters). `setCustomData` is believed to be DOM-only.
  2. Verify both claims against `dom-parity.md` and the `.jsf` scripts.
  3. Wire `setParam` into the Symbol Properties panel (a parameter table).
  4. Allowlist `setCustomData` with a decision row, if the evidence agrees.
- **Moves:** S2 +1, script-only +1.

### C5.07 — Filter selection and rename (#138, 4) `[ ]`
- **IDs:** `filter.selection`, `filter.selectionByID`,
  `filter.selectionByName`, `filter.setEffectName`
- **Needs:** C2.04 · **Tags:** `[HOST:browser]` · **Size:** S
- **Surface:**
  - in the Properties panel's Filters list, clicking an entry selects it;
  - double-clicking the name renames it;
  - choosing an item from the Filters menu applies that filter to the
    selection.
- **Note:** the `ByID` and `ByName` variants may be script-only. Use C5.00
  if they have no UI.
- **Moves:** S2 +2…4.

### C5.08 — Convolve and motion blur on the selection (#138, 2) `[ ]`
- **IDs:** `filter.convolveSelection`, `filter.motionBlurSelection`
- **Needs:** C2.04 · **Tags:** `[HOST:browser]` · **Size:** S
- **Surface:**
  - Filters ▸ Blur ▸ Motion Blur…;
  - Filters ▸ Other ▸ Convolver…, as bitmap-selection variants.
  Both use the existing `shell/filters_ui.rs` dialog pattern.
- **Moves:** S2 +2.

### C5.09 — Master pages (#139, 3) `[ ]`
- **IDs:** `page.setMaster`, `page.unsetMaster`, `page.addMasterLayer`
- **Needs:** C2.04 · **Tags:** `[HOST:browser]` · **Size:** M
- **Surface:**
  - Pages panel context menu: "Set as Master Page" / "Reset Master Page";
  - the master layer shown at the top of the Layers panel;
  - the master page's content composited under other pages. Check whether
    `compose` already handles that; add a render test if it doesn't.
- **Moves:** S2 +3; top gap #3 closes.

### C5.10 — Delete all in document (#139, 1) `[ ]`
- **ID:** `document.deleteAllInDocument`
- **Needs:** C2.05 · **Tags:** `[EVIDENCE]` · **Size:** XS
- **Steps:** find its CS6 surface in `dom-parity.md`. If there is none,
  allowlist it via C5.00. Otherwise wire it with an undo test.
- **Moves:** S2 +1 or script-only +1.

### C5.11 — Mask commands (#140, 4) `[ ]`
- **IDs:** `mask.moveBy`, `mask.moveContentsBy`, `mask.setMode`,
  `mask.setShowAttrs`
- **Needs:** C0.03 (#113 merged). **Re-measure first.** #113 may already
  bind some of these. · **Tags:** `[HOST:browser]` · **Size:** S
- **Surface:**
  - the Properties panel shows mask mode (alpha/grayscale) and "show fill
    and stroke";
  - the link toggle and dragging on the canvas move the mask or its
    contents.
- **Moves:** S2 +≤4; mask acceptance (#114) moves closer.

### C5.12 — Pixel mask (#140, 2) `[ ]`
- **IDs:** `bitmap.setPixelMask`, `bitmap.movePixelMaskBy`
- **Needs:** C5.11 · **Tags:** `[HOST:browser]` · **Size:** S
- **Surface:** Modify ▸ Mask ▸ Reveal All / Hide All / Reveal Selection /
  Hide Selection on a bitmap, plus a canvas drag with the mask unlinked.
- **Moves:** S2 +2.

### C5.13 — Shared layers (#140, 3) `[ ]`
- **IDs:** `layer.setSharing`, `layer.shareToPages`, `layer.detachShared`
- **Needs:** C5.09 · **Tags:** `[HOST:browser]` · **Size:** M
- **Surface:** the Layers panel menu has:
  - "Share Layer to States";
  - "Share Layer to Pages…", a page checklist dialog;
  - "Detach Shared Layer".
- **Moves:** S2 +3.

### C5.14 — Object resize and Group as Mask (#140, 2) `[ ]`
- **IDs:** `object.resize`, `object.setGroupType`
- **Needs:** C5.11 · **Tags:** `[HOST:browser]` · **Size:** S
- **Surface:**
  - `object.resize`: the Properties panel's W/H fields. Check what they call
    today; if they use a different verb, decide which is canonical.
  - `object.setGroupType`: Modify ▸ Mask ▸ Group as Mask.
- **Moves:** S2 +2; top gap #8 closes.

### C5.15 — Path crop, warp and pattern fill (#140, 3) `[ ]`
- **IDs:** `path.cropSelection`, `path.warp`, `path.setPatternFill`
- **Needs:** C2.04 · **Tags:** `[HOST:browser]` · **Size:** M
- **Surface:**
  - Modify ▸ Combine Path ▸ Crop;
  - the Free Transform / Distort tool for warp;
  - the fill category "Pattern" in the Properties fill pop-up. Patterns must
    be FireCraft originals with `ATTRIBUTION.md` rows; never copy CS6's
    pattern files.
- **Moves:** S2 +3.

### C5.16 — Redefine a style (#140, 1) `[ ]`
- **ID:** `style.redefineByName`
- **Needs:** C2.04 · **Tags:** `[HOST:browser]` · **Size:** XS
- **Surface:** the Styles panel menu: "Redefine Style" from the selection.
- **Moves:** S2 +1.

### C5.17 — Auto Shape rectangle (#141, 2) `[ ]`
- **IDs:** `autoshape.rect`, `autoshape.redraw`
- **Needs:** C2.04 · **Tags:** `[HOST:browser]` · **Size:** S
- **Surface:**
  - the Rectangle tool, when CS6's "rectangle is an auto shape" behaviour
    applies;
  - the corner control handles redraw the shape.
  Confirm against `docs/CS6-PARITY.md` and the auto-shape decision.
- **Moves:** S2 +2.

### C5.18 — Rectangle primitive properties (#141, 2) `[ ]`
- **IDs:** `primitive.setRectRoundness`, `primitive.setRectSides`
- **Needs:** C5.17 · **Tags:** `[HOST:browser]` · **Size:** XS
- **Surface:** the Properties panel's rectangle roundness field (with a %/px
  unit toggle) and per-corner values.
- **Moves:** S2 +2.

### C5.19 — Hotspot and slice editing (#141, 5) `[ ]`
- **IDs:** `hotspot.appendPoint`, `hotspot.copyToHotspot`,
  `hotspot.setRectangle`, `slice.appendPoint`, `slice.setBackground`
- **Needs:** C2.04 · **Tags:** `[HOST:browser]` · **Size:** M
- **Surface:**
  - the Polygon Hotspot and Polygon Slice tools: click to append points;
  - Edit ▸ Insert ▸ Hotspot from the selection;
  - the hotspot/slice Properties: X/Y/W/H, and the slice background color
    for the HTML export. See `shell/slice_editor.rs`.
- **Moves:** S2 +5.

### C5.20 — Paste between channels (#141, 1) `[ ]`
- **ID:** `clipboard.pasteFromChannelToChannel`
- **Needs:** C5.00 · **Tags:** `[EVIDENCE]` · **Size:** XS
- **Steps:** this is most likely DOM-only in CS6. Verify, then allowlist it
  or wire it.
- **Moves:** S2 +1 or script-only +1.

### C5.21 — Close out P5 `[ ]`
- **Needs:** C5.01–C5.20 · **Tags:** `[DOCS]` `[MAINTAINER]` (closing issues) ·
  **Size:** XS
- **Steps:**
  1. Run `reachability --list`. It should print only allowlisted IDs.
  2. Draft closing comments for #136–#141 that list the binding PR for each
     ID.
  3. Update GAPS-style counts in `docs/TASKS.md`.
- **Done when:** `--list` is empty apart from the allowlist, and the issues
  are closed by the maintainer.
- **Verify:** `cargo run -q -p xtask -- reachability --list | wc -l` = 0.
- **Moves:** S2 at summit.
