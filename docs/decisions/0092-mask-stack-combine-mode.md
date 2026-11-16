# 0092 — The mask stack's combine mode is a namespaced data entry the model owns

Date: 2026-11-11 · Status: accepted

## Context

FireCraft's element mask is one mask per object, and that is what CS6
ships. The reference install's resource DLL
(`source/Adobe Fireworks CS6/English/Resources/Fireworks Resources.dll`)
carries the whole Modify ▸ Mask submenu — `&Mask`, `&Reveal All`,
`&Hide All`, `&Paste as Mask`, `&Group as Mask`, `&Disable Mask`,
`&Delete Mask` — and **no** string for a mask *stack*, a mask
*level*, or a mask *combine* mode. CS6 has one element mask per
object and no stack of levels to combine.

A stack is nonetheless the shape modern raster editors use: Photoshop's
layer masks, and the mask stacks in Affinity, each hold several levels
that combine with a per-level operator — union, subtract, intersect,
invert — rather than one mask replacing another.

FireCraft wanted that shape (task 143, Phase 20.4). The question this
decision answers is *where the operator lives*, because FireCraft's
document format is Fireworks' PNG node tree and a CS6-authored file
has to survive the round trip untouched.

The format dictionary
(`docs/research/fireworks-png-nodes.md`) settles the container: a
group's children may include a `DTAv` data node, and a data node's
children may include `DCEv` entries whose first child is a `DCKs`
string naming the key and whose second child is the value. CS6
authors use that channel for `tooltips`, `radius1`, `radius0`,
`alrtmsg`, and `round0` (dictionary line 4051) — so the channel is
real, read by CS6, and crowded with names FireCraft does not own.

## Decision

- **The mode is a FireCraft extension, stored in the group.** CS6 has
  no stack, so the operator is FireCraft's own: it rides in the mask
  group's `DTAv` data node under a **namespaced** `DCEv` key,
  `firecraft.mask.mode` (`doc::MASK_STACK_MODE_KEY`). A bare key like
  `mode` could collide with a CS6-authored `DCEv` entry; the
  namespace cannot.
- **The model owns the key.** The reader parses the entry into
  `GroupObject::stack_mode` (`Option<MaskCombineMode>`) **and** bags
  the `DTAv` node verbatim, as it does for every unmapped node. The
  writer then rewrites the entry **in place** inside the bagged data
  node — preserving the node's child order, so a file FireCraft did
  not change round-trips byte-for-byte — and drops the entry when the
  model holds no mode. Foreign entries (`tooltips` and friends) are
  untouched. The model is therefore the single source of truth for
  the mode; the bag is not a competing copy.
- **An unrecognized value canonicalizes to the absent mode.** The
  reader accepts only `add`, `subtract`, `intersect`, and `invert`;
  anything else reads as no mode (the default union) rather than as
  an error, because a file authored by a future FireCraft must still
  open. Such an entry is *not* dropped on write: the model cannot
  express it, and the bag's contract is that content the model cannot
  express survives verbatim. Only a spelling the model *can* read is
  the model's to replace.
- **The vocabulary is the model's.** `MaskCombineMode::as_str` and
  `parse` are the single definition of the four spellings; the
  command `mask.setStackMode` takes `mode` as one of those strings
  and rejects any other with `EngineError::BadPayload` naming the
  offending value. `mask.setStackMode` acts on the **mask group**
  (the group that owns the element mask), and the engine publishes a
  read side — `engine::catalog::element_mask_stack(document, id)`,
  which lists every level of the stack, top level first, with the
  level's group, mask, combine mode, and enabled flag — so a shell
  can render the control without reaching into the model.
- **The Properties panel draws the control.** The panel's "Mask
  stack" section appears only when the selection carries an element
  mask, and its combo box shows the level's mode. It is a real
  binding: the shell hands `mask.setStackMode` to the engine, so the
  registry command is reachable from the shell's designed surface.

### What this does not do

**The mode is stored and persisted, but not yet rendered.** The
compositor composites mask-structured groups unmasked in this milestone
(`crates/compose/src/render.rs`: "Mask-structured groups composite all
children unmasked this milestone (masks as pixel content are M6/M7;
`MNAb` is carried by the model only)"). Element masks are not applied
to pixels at all yet, so a combine mode has nothing to combine: a
single level has no coverage below it. The panel row is therefore a
control whose value round-trips through the document, not a control
that changes pixels yet.

The open half of the work is the compositor's mask fold and
multi-level stacks (task 143): the renderer must apply each level
against the coverage below it using the level's operator. That work
depends on element masks reaching pixel content at all, which the
render milestone owns.

## Consequences

- The registry grows by one command (`engine::catalog::ids()`: 245 →
  246) and the fuzz call table gains the matching payload template,
  so the fuzz gate's call-table/catalog equality still holds.
- The reachability share rises: the shell now binds 147 of the 246
  registry commands (59.76%), so the floor is re-pinned to that
  measurement.
- The model carries the mode the way it carries the other mask
  attributes (`MNAb`, `LNKb`): the document knows it, the renderer
  does not yet act on it. That is a deliberate, recorded gap, not a
  silent one.
- The doc crate's FWMap tests pin the round trip in both directions:
  a mode survives write→read, a foreign `DCEv` entry in the same data
  node survives verbatim, setting a mode rewrites the entry in place
  rather than appending a second one, and a spelling this build does
  not know survives the round trip untouched.
