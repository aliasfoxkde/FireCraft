# 0133 — A behavior's target names a slice; the exporter resolves the file it wrote

Date: 2026-11-17 · Tickets: Phase 20.7 (issue #93) · Status: accepted

## Context

A slice behavior carries its parameters in the slice's `DTAv` custom-data
map under the key `swapTarget` (decision 0017). Two surfaces write that
key, and they write the *same* kind of value:

- The engine's `slice.addSwapImageBehaviorFromPoint` — CS6's "add swap
  image behavior from point" verb — stores the **name** of the slice
  under the point.
- The shell's Behaviors panel writes the same string: its target
  dropdown lists the document's other slices **by name**
  (`panels_web::swap_targets`).

The exporter, meanwhile, emitted that value verbatim into the markup:

```html
<a onMouseOver="MM_swapImage('left','','right',1);" ...>
<body onload="MM_preloadImages('right')">
```

The Dreamweaver runtime those calls imitate sets `img.src` from the third
argument and `new Image().src` from the preload list, so both must name
a **file the export wrote**. The export writes `right.png`.

The names the export writes are export-time artifacts: decision 0017
records that the auto-naming scheme is applied at export time and is
**never stored on the slice**. So the model cannot hold the file name
either — the file name is not known until the export lays the grid out,
names the pieces and encodes them.

The result was that every behavior attached through the command surface
— the only way the shell attaches one — produced HTML that points at a
file that does not exist. The swap silently did nothing in a browser and
the preload 404'd. The defect was invisible to the tests: the exporter's
own behavior test (`webexport`'s `behaviors_emit_hooks_and_the_runtime`)
built the behavior **directly on the model** with a hand-written file
name (`swap.png`), a value the command surface never writes, so it
asserted a case the product does not produce.

## Decision

1. **The model stores the name of the slice a behavior points at.** The
   engine's verbs and the shell's dropdown keep writing names; the file
   a swap points to stays an export-time artifact.
2. **The exporter resolves that name to the file it wrote.** The
   resolution map is keyed by every name the object can be addressed by:
   the slice's object name, its internal name, and each of those
   cleaned. The value is the object's first piece's file — the file the
   export actually produced for it.
3. **A target that names no slice is a file name the user typed** and is
   emitted verbatim, so a hand-authored `swap.png` still works and the
   exporter's existing markup is unchanged for it.
4. **A file the export wrote is referenced where it was written.** The
   resolved target and the preload list carry the export's
   `images_dir` prefix, exactly as the `<img src>` that names the same
   file does. A file the user typed is their own reference and is used as
   given.

## Why

The alternative — storing the file name on the slice — cannot be built
without breaking two rules at once. The name depends on the export's
grid layout and on the slice's `EXPv` codec, so the engine would have to
reach the exporter's naming rules to compute it; `engine` is L3 and
`webexport` is L4, and an upward edge is a layer violation. And storing
the name would freeze an export-time artifact on the slice, which is the
rule decision 0017 set.

Resolving at the exporter keeps the model honest (it names a slice, and
a slice can be renamed — the swap follows) and keeps the exporter's
promise (the markup references files that exist, at the paths they were
written).

## Consequences

- The shell's own workflow now produces working swaps: pick a target
  slice in the dropdown, and the exported page swaps to that slice's
  image.
- The exporter's markup for a hand-written file name target is
  unchanged (`swap.png` still emits `swap.png`), so no existing golden
  moves.
- The export-level regression that closes issue #93 drives the behavior
  through the command surface (`slice.create`, `selection.set`,
  `slice.addSwapImageBehaviorFromPoint`, `slice.addBehavior`,
  `slice.setData`) and asserts the emitted HTML carries the behavior's
  markup — and that the file it points at is among the exported files.
- A target naming a slice that the export did not write (a slice off the
  grid, or one the layout dropped) falls back to the verbatim name: the
  markup then names the slice, not a file. That is the same shape as a
  hand-written target, and it is visible in the exported HTML rather
  than silent.

## References

- `crates/webexport/src/html.rs` — `target_files` (the resolution map)
  and `hooks` (the markup).
- `crates/engine/src/catalog/web.rs` —
  `slice.addSwapImageBehaviorFromPoint`, which writes the name.
- `crates/app-cli/tests/web_behavior_export.rs` — the export-level
  regression, driven through the command surface.
- [decision 0017](0017-web-layer-and-export.md) — the Web Layer, the
  `DTAv` parameter map and export-time naming.
