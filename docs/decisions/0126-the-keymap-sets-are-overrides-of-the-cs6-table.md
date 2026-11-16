# 0126 — The keymap sets are overrides of the CS6 table

Date: 2026-11-16 · Issue:
[#92](https://github.com/aliasfoxkde/FireCraft/issues/92) · Status: accepted

## Context

FireCraft shipped one keymap: CS6's own, transcribed from the CS6
`Fireworks.xml` keymap resource into `menus::SHORTCUTS` (167
bindings). The binding table was compiled in, so the shell could not
offer another set, and the bindings existed only inside the app —
`docs/USAGE.md` pointed at the in-app F1 panel, so the shortcuts could
not be searched, diffed, or read without launching FireCraft.

Issue #92 asks for two things: alternative keymap sets (Illustrator,
Photoshop, FreeHand, Web Standard) that a user can select, and the
bindings published as a readable table.

Two properties constrain the shape of the answer. First, CS6 is the
reference implementation: its bindings are the ones the CS6 parity
matrix and the `cs6_keymap_is_fully_bound` gate are written against,
and the issue's own instruction is that the CS6 set must stay
byte-identical in behaviour. Second, the shell learns which verbs
exist *from* the CS6 table — a set that named a verb the table did not
bind would be naming a verb the shell does not have.

## Decision

- **A set is a set of overrides, not a second copy of the table.**
  `crates/app/src/keymap.rs` carries a `KeymapSet` enum and one
  `Binding` list per set. A binding names a verb, a modifier set, a
  key, and the vendor's own name for the command it comes from.
  `KeymapSet::bindings()` resolves a set by walking the CS6 table and
  substituting the set's binding where the set has one; every other
  verb keeps the CS6 binding. A verb the set overrides therefore
  carries exactly one binding in the resolved table — the set's — so
  one keystroke cannot fire a verb twice.
- **CS6 is the default and stays byte-identical.**
  `KeymapSet::Cs6` carries no overrides, and its resolution is the
  CS6 table exactly — same verbs, same modifiers, same keys, same
  sections. The set is `Default`, so a layout saved before the sets
  existed (which carries no set) is CS6.
- **Only verbs the CS6 table binds can be overridden.**
  `KeymapSet::unbound_overrides()` reports any override naming a verb
  the CS6 table does not bind; a test asserts the list is empty for
  every set. Such an override could not take effect, so it is a
  defect in the table, not a silent no-op.
- **One keystroke binds one verb.** The shell dispatches *every*
  binding whose key and modifiers match, so a resolved table that
  repeated a keystroke would fire two verbs on one press. The CS6
  table holds this invariant (no two of its 167 bindings share a
  keystroke), and each set must hold it too: a set that parked an
  override on a key the table already used would break it. A test
  asserts it per set.
- **The choice lives in the preferences and is persisted.**
  Edit ▸ Preferences ▸ Keyboard lists the sets; the choice is stored
  in `ShellLayout::keymap` (`#[serde(default)]`, so an older layout
  file is CS6) and re-applied by `Shell::apply_layout`.
- **The published table is generated, never hand-written.**
  `docs/SHORTCUTS.md` is written by `cargo run -p xtask -- shortcuts
  --bless` from the same tables the shell dispatches from, and the
  gate (`xtask ci`, and CI's `generated-docs` job) fails on any
  difference. The document cannot drift from the code, and a
  hand-edit of it is a defect rather than a preference.

## The data and its sources

The bindings are transcribed from the vendors' own published
references; nothing is invented. Each set's source is named in the
generated document, and each override row carries the vendor's own
command name so the row can be found in that reference.

| Set | Source | Rebinds |
|---|---|---|
| Illustrator | Adobe, *Default keyboard shortcuts in Adobe Illustrator* (`helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html`) | deselect, bring-to-front, send-to-back (3) |
| Photoshop | Adobe, *Default keyboard shortcuts in Adobe Photoshop* (`helpx.adobe.com/photoshop/using/default-keyboard-shortcuts.html`) | new layer, bring-to-front, send-to-back (3) |
| FreeHand | Macromedia, *FreeHand MX Keyboard Shortcuts* (the Mac sheet Macromedia shipped with FreeHand MX; `Command` read as `Ctrl`, `Option` read as `Alt`) | redo, fit-all, fit-selection, panel hiding, eight tool keys, italic (13) |
| Web Standard | the Web Standard conventions — the browser and platform keyboard conventions | redo, italic (2) |

A vendor shortcut that would collide with a CS6 binding the set
leaves in place is *not* carried over: the collision would bind one
keystroke to two verbs, which the invariant above forbids. Three
collisions were found and cost the sets their binding:

- Adobe's `Ctrl`+`]` and `Ctrl`+`[` (Illustrator's *bring forward* /
  *bring backward*, Photoshop's *move target layer down/up*) are
  already CS6's *show/hide tooltips*, so both sets keep CS6's
  forward/backward keys.
- Adobe's `Ctrl`+`Shift`+`G` (*ungroup*) is already CS6's *mask:
  reveal selection*, so neither set carries the ungroup binding.

FreeHand's stacking verbs are the other worked example — Macromedia
binds them to `Ctrl`+arrow, and CS6 binds `Ctrl`+arrow to
select-parent and select-child, so the FreeHand set keeps CS6's
stacking keys. Where a vendor has no published binding for a verb
FireCraft has, the verb keeps the CS6 binding.

Every alternative set is therefore *partial* by construction, and the
count is a property of the sources, not a design choice: a vendor
sheet that publishes a binding on a key CS6 already uses contributes
nothing, because carrying it would break the one-keystroke-one-verb
invariant that the shell's dispatch loop depends on.

## Consequences

- The shell gains a runtime-selected keymap with no change to the
  CS6 default: the same 167 bindings, the same accelerators printed
  in the menus, the same dispatch.
- `docs/SHORTCUTS.md` exists and is gated, so the shortcuts are
  readable, searchable and diffable without launching the app.
- The alternative sets are deliberately partial: they rebind the
  shared verbs the vendors agree on and leave the rest to CS6, which
  is why no set is a regression against the default.
- A set's table is data, not code: adding a binding is a one-line
  table edit plus the gate's regenerated document.
