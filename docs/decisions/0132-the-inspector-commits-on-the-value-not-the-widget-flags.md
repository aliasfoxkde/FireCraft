# 0132 — The inspector commits on the value, and a bitmap's bounds resample its pixels

Date: 2026-11-16 · Issue:
[#88](https://github.com/aliasfoxkde/FireCraft/issues/88) · Status: accepted

## Context

Issue #88 asks for the Property inspector to show CS6's field set
for the selected object's kind, and for a bitmap's fill and stroke to
be editable from it. Working the issue turned up three separate
things, and each needed a decision rather than a patch.

**The geometry strip was inert.** `crates/app/src/shell/panels.rs`
committed a numeric field with `response.lost_focus() && response.
changed()`. In `egui` 0.36.2 those two signals cannot be true in the
same frame:

- `Response::changed()` is a per-frame flag, set only in the frame the
  value moved (`egui/src/widgets/drag_value.rs`: the widget compares
  the value with the one it started the frame with and calls
  `mark_changed()`).
- `Response::lost_focus()` is live-evaluated
  (`egui/src/response.rs`: `self.ctx.memory(|mem| mem.lost_focus(self.id))`),
  and `egui`'s focus memory answers it over a two-frame window
  (`egui/src/memory/mod.rs`: the widget was focused at the start of
  this frame or the one before, and is not focused now).

A typed value moves in one frame and loses the focus in another, so
the conjunction was false in both — the strip drew an editable field
that never wrote anything. The same pattern was on the document's
canvas width/height and resolution, and on the slice's link, alt text
and name.

**A bitmap's bounds were a readout.** `object.setBounds` refused an
image target: the transform family maps geometry with an affine
matrix, and a bitmap's pixels do not ride a matrix — they are a grid
that has to be resampled. The CS6 manual is explicit that the
Property inspector takes a new width or height for an object
("In the Property inspector or Info panel, enter new width (W) or
height (H) measurements"), and for a bitmap that means new pixels.

**A symbol instance had no effect list.** The inspector returned
early for an instance, so the effects section never rendered.

## Decision

- **The inspector commits on the value, not on the widget's flags.**
  A field writes when its value differs from the value the document
  holds and the widget reports the change in that frame. The numeric
  fields are built with `update_while_editing(false)`, so a typed
  value lands once — when the edit is committed — and a dragged
  spinner still lands on every frame it moves. The text fields commit
  when they lose focus and their text differs from the stored text.
  The rule is the same everywhere in the panel, and it is a rule
  about the *value*, so it does not depend on which frame the widget
  happens to report focus in.
- **A bitmap target resamples; every other target takes the matrix.**
  `object.setBounds` partitions its targets. A bitmap's pixel grid
  takes the selection's scale, its assembled pixels are resampled
  (the bilinear pass behind `document.setImageSize`, decision 0041),
  and it is re-tiled over fresh bank slots; its frame lands on the
  requested box and its sub-pixel offset rides the same scale. Every
  other target — path, text, group, slice, instance — keeps the
  affine map it always had. The refusal for a bitmap *inside a group*
  stands: a group's members ride the caller's matrix, and the
  transform family has no resample path through it.
- **A bitmap carries no fill or stroke, exactly as in CS6.** The
  `IMGv` node has no fill/stroke child
  (`docs/research/fireworks-png-nodes.md`), and CS6's bitmap tools
  live in the separate **Image Editing** command panel, not in the
  Property inspector. The absent editors are therefore CS6's own
  field set, not a gap.
- **An instance lists the definition's filters, read-only.** The
  `EFLv` spine's parents are EFDv, GRPv, IMGv, PRIv, PTHv, TXTv and
  ZSTv — an instance (`ALSv`) is not one of them, so an instance
  carries no effect list of its own. What CS6 shows on an instance is
  the definition's stack: "modifying an instance with the Properties
  panel affects the symbol and all other instances". The inspector
  therefore lists the definition's elements' filters under the label
  "inherited from \<definition\>", read-only. The edit belongs to the
  definition's own objects, and the instance inherits it.

## Consequences

- The Property inspector's fields are live: a width typed into the
  strip reaches the document, and a bitmap's pixels follow it. The
  per-kind field set is pinned by a regression that renders the
  inspector for each selection class and asserts the field list
  against the rows of `docs/CS6-PARITY.md` §D.
- The commit rule is uniform, so the panel has one rule to reason
  about rather than a family of near-misses.
- `object.setBounds` on a bitmap is undoable like any other command:
  the inverse restores the exact pixel grid and the minted tiles are
  removed with it.
- A bitmap inside a group still refuses `object.setBounds`. The
  refusal is explicit and covered by a test, and the resample path
  for a group member is a separate piece of work with its own owner.
