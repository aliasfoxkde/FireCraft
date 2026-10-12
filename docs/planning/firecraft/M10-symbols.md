# M10 — Symbols, libraries, components

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** FireCraft gains CS6's symbol system: graphic / button /
animation symbols with a document library, symbol create/edit/break
through the registry, 9-slice scaling guides (`NGSv`, deferred from
M6), rich ("component") symbols whose `Widget.opCode` scripts
re-derive their geometry (`setDefaultValues` / `applyCurrentValues` —
M9's scripting host is the engine), library import/export as
`.fw.png` (the Common Library file shape), and the Common Library
re-created as original assets (ATTRIBUTION rows). **Gate:** symbol
round-trip floors rise; rich symbols from CS6 docs render and stay
editable.

**Inputs:** the model seeds (`doc::SymbolDefinition` = `MSTv`,
`doc::SymbolInstanceObject` = `ALSv`); the node dictionary
(`docs/research/fireworks-png-nodes.md`: `MSLv`/`MSNi`/`MSTv`/`MSYi`/
`ALSv`, the `ANI*` animation parameter family, `NGSv` nine-slice,
`SYMi`, `ATMv`); corpus docs carrying symbols (`corpus/incoming/`,
git-ignored — CS6-authored reads are the format authority per
PLANNING §6); the M9 scripting host (`smartShape`/`Widget` globals
already stubbed in `scripting/src/host.rs`); the M6 deferral note
(9-slice needs the symbol tree); PLANNING tier T1/T3 split (graphic
symbols + 9-slice = T1; button/animation/rich + library = T3 — M10
does both).

## Design line: a symbol is a definition plus instances, and every
mutation is a command

The library is `document.symbols` (already in the model). An instance
is an object that renders its definition's layers through the instance
transform — compose resolves `MSNi` → definition and recurses (depth
ceiling; self-reference is a typed refusal, never a cycle). Editing a
symbol means editing the definition's `LayerList` — the same cell /
object commands the page tree takes, addressed through a symbol
scope. Rich symbols add an `opCode` script to the definition: instance
parameter changes re-run the script through the M9 host and drain
through the registry (so a rich-symbol re-derivation is journaled,
undoable, and replayable like everything else). 9-slice is `NGSv` on
the definition plus per-instance override; compose and the vector
offset path already carry the geometry halves.

## Tasks

1. **Format: symbol depth in the read/write path** — `MSYi` type
   (graphic omits the node; button/animation values), the animation
   parameter family on definitions (`ANIf`/`ANRi`/`ASCf`/`ASOi`/
   `AEOi`/`ANXf`/`ANYf`, frame count / rotation / translation /
   scaling / opacity), `NGSv` on `MSTv` (guide insets) and its
   instance override, `MSLv` document list semantics, `SYMi`. Reader
   + writer + unknown-bag round-trip on every node; corpus symbol
   files parsed without panic (typed refusals only). **Stopping
   condition:** a symbol-bearing corpus file round-trips
   byte-comparably (equal + idempotent, the M3 battery rules) and the
   new nodes never land in the unknown bag.
2. **Compose: instances render** — definition layers composed through
   the instance transform; button symbols render their states;
   animation symbols generate their frame span into states (the
   `ANI*` parameters, smooth transition via the existing tween
   machinery); 9-slice instances scale by guide insets instead of
   stretch; self-referential definitions are a typed refusal with a
   depth ceiling. **Stopping condition:** golden pins for a graphic
   instance, a 9-slice instance at two sizes, and an animation
   symbol's state strip; determinism + anchor pins green.
3. **Registry: symbol commands** — `symbol.create` (from selection),
   `symbol.createInstance`, `symbol.breakApart`, `symbol.edit*`
   (definition-scoped object commands), `symbol.setType`,
   `symbol.set9Slice`, `library.add/remove/rename`; list-capturing
   deltas for library mutations; journal/undo/replay for all.
   **Stopping condition:** every new command in the fuzz driver's
   catalog with the four contract invariants (never panic, failed
   call never mutates, undo/redo exact, replay exact); equivalence
   battery extended with a symbol seed.
4. **Shell: Library panel + symbol editor + 9-slice guides** —
   document library panel (definitions, drag-to-place, rename);
   enter-symbol / exit-symbol editing (definition scope in the
   editor); 9-slice guide overlay with draggable insets; button-state
   inspector. **Stopping condition:** scripted golden (the M6 shape)
   drives create → place → edit-definition → break-apart through the
   shell's own apply path and the file stays round-trip-clean.
5. **Rich symbols (`Widget.opCode`)** — definition carries the
   `opCode` source; instance parameter sets run
   `setDefaultValues`/`applyCurrentValues` through the M9 host
   (`Widget` global finished: no more `__notImplemented` stub);
   re-derivation drains through the registry (Decision 0011
   semantics). **Stopping condition:** a rich symbol from the CS6
   corpus renders, accepts a parameter change, re-derives, and stays
   editable — journal replays the whole chain byte-exactly.
6. **Library import/export + Common Library re-creation** — `.fw.png`
   library files (document + `MSLv`) open and import per-symbol;
   export selection/library to the same shape; the 5 CS6 Common
   Library files re-created as original FireCraft assets (vector,
   from observed contents — never copied) with `docs/ATTRIBUTION.md`
   rows in the same change. **Stopping condition:** library export →
   import → place round-trips byte-comparably; the re-created assets
   open, place, and render; attribution ledger complete.
7. **Gates** — symbol corpus floor + round-trip floors in the parity
   table; fuzz arms for every new command; `NGSv` proof pins in
   `vector` (the M6 debt closes here); parity + scorecard
   regenerated; decisions doc (symbol render model + opCode
   contract); devlog. **Stopping condition:** `xtask ci` green via
   GitForge; M10 scorecard row → done with gate evidence.

## Non-goals (M10)

- Shared-content sync across documents (CS6 "shared to pages" runs in
  one document only; cross-document libraries ride the M11/M12
  release surfaces).
- The `mkBT` writer + TileBank host bridge (M7-owed debt) stays
  tracked separately — it is bitmap tiling, not symbols.
- Common Library beyond the 5 shipped reference files (the fuller CS6
  library needs assets we re-create only as far as observed contents
  justify).
