# T9 — 20.4 Mask stack UI

**Objective:** a mask stack the user can edit: add a level, remove a
level, choose how a level combines with the coverage below it, link
and unlink a level, and move a level in the stack.

**Inputs:** the element-mask machinery (mask groups, `mask.enable`,
`mask.setShowAttrs`, `mask.link`); the Properties panel's mask
section; the compositor's group fold.

**Authority:** CS6 has **no** mask stack — one element mask per object
— so the stack is a FireCraft extension. The evidence for the absence
is in the reference install's resource DLL: the Modify ▸ Mask submenu
carries `&Mask`, `&Reveal All`, `&Hide All`, `&Paste as Mask`,
`&Group as Mask`, `&Disable Mask`, `&Delete Mask` and nothing that
names a stack, a level, or a combine mode. The wire contract is
FireCraft's own; the *shape* follows the modern raster editors
(Photoshop layer masks, Affinity mask stacks): per-level operators —
union, subtract, intersect, invert — over the coverage below.

**Stopping condition:** the mode a level carries survives a save/load
round trip; a mode the user picks is what the model holds; an
unrecognized spelling from a future file does not break the load; and
the operator actually changes the rendered coverage.

**Verification:** `cargo test -p doc` (the FWMap round trip),
`cargo test -p engine` (the command), `cargo test -p app` (the panel),
plus the repo gate `cargo run -p xtask -- ci`.

## Status — PARTIAL (2026-11-11)

**Landed** (decision [0092](../decisions/0092-mask-stack-combine-mode.md)):

- The model owns the operator. `doc::MaskCombineMode` is `Add`,
  `Subtract`, `Intersect`, `Invert`; `GroupObject::stack_mode` holds
  it; the wire key is the namespaced `DCEv` entry
  `firecraft.mask.mode` inside the mask group's `DTAv` data node.
- The FWMap round trip works in both directions. The reader reads the
  entry into the model *and* bags the data node; the writer rewrites
  the entry in place inside the bagged node (so an unchanged file
  re-serializes byte-identically) and leaves foreign entries
  (`tooltips` and friends) verbatim. An unrecognized spelling
  canonicalizes to the default on read and survives the write
  untouched, because the model cannot express it.
- `mask.setStackMode` sets the level's operator, resolving the mask
  group from either the group or the object it masks, and refuses an
  unknown spelling with `EngineError::BadPayload`.
- The Properties panel draws a "Mask stack" section with a combine
  combo box, only when the selection carries an element mask.

**Open:**

- **The compositor does not apply masks at all yet.** `compose`'s
  group fold composites mask-structured groups unmasked this milestone
  ("masks as pixel content are M6/M7; `MNAb` is carried by the model
  only"). So the operator is stored and persisted but changes no
  pixels, and a single level has no coverage below it to combine
  with. The rendering half is the open scope of this ticket: the
  compositor must apply each level against the coverage below it
  using the level's operator, and the stack must hold more than one
  level.
- Add / remove / link / move for stack levels (the rest of 20.4) are
  not implemented: the model has one level per mask group today, and
  the panel exposes only the operator.
- No screenshot acceptance yet — the section is unstyled relative to
  the rest of the Properties panel, and the Phase 22 screenshot audit
  has not covered it.
