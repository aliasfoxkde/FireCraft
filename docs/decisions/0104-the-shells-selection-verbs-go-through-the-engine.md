# 0104 — The shell's selection verbs go through the engine's selection door

Date: 2026-11-12 · Issue: [#138](https://github.com/aliasfoxkde/FireCraft/issues/138) ·
Status: accepted

## Context

The Select menu's three verbs — Select All, Deselect, Select Inverse —
never reached the engine. `Shell::run_action` read the editor's *current
cell* (the active layer's cell in the current state), copied its element
ids into the view's selection, and stopped there. Two things were wrong
with that.

**The scope was not CS6's.** CS6's `dom.selectAll()` selects every
element of the page in the current frame — "all elements on all layers",
decision 0024. The shell selected the active layer's cell: one layer, one
state. A user with two layers pressed Ctrl+A and got half the page.
Select Inverse had the same scope, so "select everything that is not
selected" left the other layers' objects out of the complement — the
complement of a smaller set is a smaller set.

**The selection was view state only.** The engine's selection-scoped
commands (the clipboard family, the flatten family, the filter family)
read `Document::object_selection`; the shell tracks the selection as view
state (`Editor::selection_set`). The two were bridged in one direction
only — `Editor::sync_selection` pushes the view's selection into the
document — and only where a command needed it. Select All and Deselect
never crossed that bridge, so the document's selection kept whatever the
last bridged command left there: "Deselect" did not deselect the
document, and a later selection-scoped command could act on a selection
the user had already emptied on screen.

The engine already carried the right commands, with CS6's scopes:
`selection.selectAll`, `selection.selectNone`, `selection.selectInverse`
and `selection.selectAllOnLayer`.

## Decision

- **The three menu verbs dispatch the engine's commands** and adopt the
  document's answer as the view's selection. The shell keeps no scope of
  its own — the scope is the engine's, which is also the scope the
  scripting host sees.
- **Select Inverse syncs first.** The engine's complement is taken
  against the document's selection, so the view's selection lands there
  before the command and comes back after it. This is the idiom the
  Select Parents / Select Children rows already use.
- **The Layers panel gains "select this layer's objects"** —
  `selection.selectAllOnLayer` with the plain replace semantics. It is a
  FireCraft extension, not a CS6 menu row: the menu census is
  unchanged.
- **A no-op deselect sends no command.** Clearing an already-empty
  selection would record a journal entry and an undo step that changes
  nothing.

## Evidence

- The engine's four commands carry CS6's scopes and decision 0024's
  page-and-frame rule; the shell was the only place that reimplemented
  a selection scope, and it reimplemented it smaller.
- `Editor::sync_selection` and `Editor::adopt_document_selection` are
  the two bridges between the view's selection and the document's, and
  the Select Parents row already used them.
- The reachability gate measured `selection.selectAll`,
  `selection.selectNone`, `selection.selectInverse` and
  `selection.selectAllOnLayer` as unbound: the engine could execute
  them, the shell's designed surface could not reach them.

## Consequences

- Select All covers every object of the page in the current state,
  across layers — CS6's scope.
- A selection made from the menu is journaled: undo takes it back and
  the History panel lists it.
- Deselect empties the document's selection as well as the view's, so
  the two cannot disagree.
- The reachability floor rises by four commands. The three selection
  commands the shell already had rows for stay one row each; nothing in
  the menu table changed.

See also decision [0024](0024-object-selection.md) — the scope rule this
change finally reaches the menu through.
