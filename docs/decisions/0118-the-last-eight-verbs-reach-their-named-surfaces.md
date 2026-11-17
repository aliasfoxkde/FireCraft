# 0118 — The last eight verbs reach their named surfaces

Date: 2026-11-14 · Issues:
[#139](https://github.com/aliasfoxkde/FireCraft/issues/139),
[#140](https://github.com/aliasfoxkde/FireCraft/issues/140),
[#141](https://github.com/aliasfoxkde/FireCraft/issues/141) ·
Status: accepted

## Context

Decision 0117 closed the panel-binding wave with fourteen measured
unreachable commands: five bound at once, nine carrying recorded
reasons. The wave-2 work answers those reasons — not by fake-binding
a near-duplicate, but by building the surface each reason named: the
canvas gesture, the dialog, the confirmed action. `autoshape.rect`
had already been bound by the Auto Shapes panel's insert; the other
eight were still unreachable at 249/257 (96.89%).

Each binding below is a real dispatch (a string literal handed to the
engine from code, not a comment), each writes values the renderer or
exporter honours, and each has a regression test that fails when the
binding is removed.

## Decision

- **`path.warp` — the Freeform tool** (split off ReshapeArea): the
  drag pushes through `warp_selection` on release, a radius-limited
  pull of the selected paths' nodes dispatched as `path.warp`.
  0117's reason: "both are canvas gestures… they need the drag
  interaction the canvas owns." This is that interaction.
  Test: `the_freeform_tool_pushes_through_path_warp`.
- **`path.cropSelection` — the Crop tool**: when the selection is
  non-empty and every selected object is a path, the crop dispatches
  `path.cropSelection` with the crop rect; with no path selection the
  tool keeps its canvas-resize behaviour. Test:
  `the_crop_tool_cuts_the_selected_paths`,
  `the_crop_tool_moves_the_canvas_without_a_path_selection`.
- **`object.resize` — Modify ▸ Transform ▸ "Scale To..."**: the row
  opens the dialog 0117 itself named as missing ("the CS6 Transform
  ▸ Scale ▸ To… dialog that would [distinguish scale-in-place from
  set-the-bounds] is not built"); it seeds from the selection's union
  bounds and applies `object.resize` with the exact box, top-left
  anchored. Test: `scale_to_dialog_resizes_to_the_exact_box`.
- **`object.setGroupType` — the mask panel's "make plain group"
  row**: gated on an enabled mask (`GRTi` 1/2), it selects the owning
  mask group and retypes it to `normal` with `MNAb` untouched, so the
  mask's own artwork joins the group as an ordinary member — distinct
  from *Disable Mask*, which hides that artwork. This answers 0117's
  no-op reading, which held only where the group is already plain:
  the row lives where the group is not. Test:
  `the_mask_panel_row_makes_the_mask_group_plain`.
- **`document.deleteAllInDocument` — Edit ▸ "Delete All..." behind
  the shell's first confirmation**: 0117 said the verb "needs a
  confirmed action that does not exist yet." This builds it — a
  reusable `ConfirmDialog` (question, Yes/No, `ConfirmAction`), and
  the row is this build's own (CS6's verb has no menu row of its
  own), moving the menu census to 298. "No" leaves the render
  byte-identical; "Yes" empties the document in one undo step named
  for the verb. Test: `delete_all_asks_before_emptying_the_document`.
- **`slice.appendPoint` / `hotspot.appendPoint` — the polygon tools'
  commit replays through the append state machine** (decision 0029's
  Design C): the click-to-plot draft UX is unchanged, but commit now
  clears the selection so point 1 takes the create branch, reads the
  minted id back out of the delta, appends points 2..n against it,
  and closes the outline by handing the final anchor exactly the
  distance to the start — the engine's own close formula. It all
  folds into one transaction: one undo step named for the verb, or
  an abort that leaves the document untouched. The observable is the
  HTML export: the replayed anchors are the
  `<area shape="poly" coords=...>` a browser hits (compose renders
  slices as nothing by design). Tests:
  `the_polygon_hotspot_tool_commits_through_append_point`,
  `the_polygon_slice_tool_commits_through_append_point`.
- **`autoshape.redraw` — the Auto Shapes panel's Insert/Redraw
  toggle**: 0117 read the redraw as having "no faithful source…
  re-generating from the panel's current parameters would move the
  shape rather than redraw it." The mode makes the source explicit:
  in Redraw mode a gallery click re-runs the chosen generator into
  the selected path from the panel's box and corner — `{id, kind,
  params}`, the insert payload minus placement, every key matching
  the engine's `deny_unknown_fields` param structs. The redrawn
  contours equal a fresh insert's for the same parameters (the same
  generator, run again), and with no path selected the row reports
  and dispatches nothing. The two primitives stand down in redraw
  mode — no library entry re-runs. Test:
  `the_autoshapes_panel_redraws_the_selected_path`.

## Consequences

- Reachability **249/257 (96.89%) → 257/257 (100%)**; the one-way
  floor deliberately re-blessed to 100.00% on 2026-11-14. The
  "8 unreachable" list that #143's closeout left is empty.
- Menu census 296 → **298**: "Scale To..." (CS6's row, no resource id
  in the static mirror) and "Delete All..." (this build's own row) —
  the pin, its "three above the CS6 transcription" note, and every
  doc quote moved with them.
- The shell gains two mechanisms: the `ScaleToDialog` and the
  `ConfirmDialog` (the confirmation was the blocking dependency 0117
  named).
- Parity and the registry are unchanged — the engine already carried
  all eight verbs; this wave moved shell surfaces only.
- Issues #139–#141's binding, renderer-honours, floor re-pin and
  regression-test criteria are met by these changes; issue closure
  follows CI evidence.
