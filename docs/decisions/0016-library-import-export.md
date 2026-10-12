# Decision 0016 — Library files: extract/import shape, closure discipline, Common Library re-creation

Date: 2026-10-12 · Milestone: M10 task 6 · Status: accepted

## Context

A library file is an ordinary `.fw.png` whose `MSLv` carries symbol
definitions and whose registry holds their member closure — nothing on a
page (probed: all five CS6 Common Library files have `MSLv`+`MSTv`+`MSNi`;
Buttons.png additionally carries `JSVv`, i.e. rich symbols live in library
files). FireCraft needed both directions — export a selection to that
shape, import a file's definitions into a working document — plus original
re-creations of the five Common Library assets.

## Decision

**1. The file shape is the document shape; nothing new on the wire.**
`doc::library::extract` builds a pageless `Document` from named
definitions (ids re-slotted into a fresh registry in first-touch order,
definitions renumbered `0..n`), written through the canonical
`fwmap::write` → `nodetree::serialize` → `mkTS`/`mkBS` path. A zero-page
document round-trips: the reader's `PDCv` loop simply pushes no pages.

**2. Import is a journaled command with closure discipline.**
`library.importSymbol {definition, members}` refuses unless every id
reachable from the definition — through nested group children, walked
transitively — is present exactly once in the payload; a partial import
would mint dangling ids, the defect `library.remove` refuses to create.
Payload members outside the reachable closure are *ignored*: a library
file's registry carries the members of all its definitions, and importing
one symbol must not drag the rest in as unreferenced registry orphans.
Members re-slot in walk order (deterministic across journal replay), the
definition renumbers to the next free `MSNi`, and the delta is one
`Created` capture per member plus one library capture — one undoable step
per definition, byte-exact on undo.

**3. The editor helpers are thin over those two doors.**
`Editor::export_library(names)` = `doc::library::extract` + the ordinary
save path (empty bag, no original chunks — a library file has no tiles to
preserve). `Editor::import_library(bytes, names)` = the ordinary read
path + one `library.importSymbol` per selected definition (`None` = all,
file order), returning the new `MSNi` numbers.

**4. The five Common Library assets are generated, not authored.**
`xtask library-gen` builds each asset through the engine registry —
original vector marks in the observed *categories* (animations, bullets,
buttons, halos, themes), no Adobe bytes, paths, colors, or names — then
extracts to the library shape. Buttons carries one rich symbol (a Widget
script under `JSVv`) mirroring the observed rich-symbol presence in CS6's
Buttons.png. Rows in `docs/ATTRIBUTION.md` landed in the same change as
the assets.

## Consequences

- Round-trip proof: two imports of one exported file place
  byte-identically; the placed document re-emits byte-stably;
  import undo restores pre-import bytes (`crates/app/tests/library.rs`).
- The re-created assets are integration fixtures: the app test opens,
  imports, places, and renders each, and drives the rich button through
  `rederive_symbol` — the T5 host against a T6-imported definition.
- The assets are not yet pinned in `corpus/MANIFEST.tsv` (the corpus-gen
  rewrite would drop hand-added rows); task 7's symbol-corpus floor
  wires them in properly.
- Per-definition imports of a multi-definition file duplicate shared
  members (each closure imports its own copy). CS6 libraries do not
  share members between definitions, so this matches observed behavior;
  revisit only if a corpus file proves otherwise.
