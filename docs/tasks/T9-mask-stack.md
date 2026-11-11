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

**Landed** (decisions [0092](../decisions/0092-mask-stack-combine-mode.md)
and [0093](../decisions/0093-mask-stack-levels-and-folding.md)):

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
- **A stack is nested mask groups** (0093): a level's content child
  is the level below it, the outermost group is the top level. The
  bottom level replaces the coverage and each level above combines
  with the coverage below it by its operator. A disabled level stays
  in the stack in place and contributes no coverage.
- **The compositor applies the stack.** `compose` folds the levels
  bottom-up into one coverage and scales the content's pixels by it;
  the mask artwork is never composited as content.
- **The panel lists the levels** — one row per level, top first,
  numbered from the bottom the way CS6 numbers its masks. Each row
  has the enable checkbox, the operator combo, the reorder arrows and
  the remove button; the section's foot adds a level.
- **A level links and unlinks its own mask.** The link flag rides the
  level's mask group (`LNKb` is written only when the mask is
  *un*linked), so the flag is per level: unlinking one level leaves
  the levels around it linked. The panel row carries the checkbox.
- The structural level commands (`mask.addLevel`,
  `mask.removeLevel`, `mask.moveLevel`) undo byte-exactly — the
  engine's fuzz property covers them.

**Open:**

- The compositor samples a level's mask at the canvas resolution
  rather than at the mask image's own resolution, and the fold is a
  per-pixel operation with no feathering beyond the mask's own gray
  level.
- No screenshot acceptance yet — the section is unstyled relative to
  the rest of the Properties panel, and the Phase 22 screenshot audit
  has not covered it.

