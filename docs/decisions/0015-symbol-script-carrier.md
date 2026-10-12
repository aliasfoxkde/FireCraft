# Decision 0015 — Rich symbols: script carrier, parameter store, host re-derivation

Date: 2026-10-12 · Milestone: M10 task 5 · Status: accepted

## Context

CS6 rich symbols (Common Library "components") are symbol definitions
whose script lives in a companion `.jsf` beside the library art; the
host runs it with `Widget.opCode` 1 (set defaults) or 2 (apply current
values) when an instance is placed or re-parameterized. The script
reads `Widget.elem.customData["currentValues"]` (`{name, type, value}`
triples) and mutates definition members by name through
`Widget.GetObjectByName`. FireCraft needed all three halves: a wire
carrier for the script and the parameter store, engine commands to
install them, and a re-derivation path that journals like every other
mutation. (Corpus probe: none of the five Common Library `.fw.png`
files carries script text inside its mkTS stream — the carrier below
is FireCraft's own, modeled on where CS6 *does* embed text streams.)

## Decision

**1. Wire carriers, claimed only in canonical form.**
- Definition script: `JSVv` (children `JSSs` string pieces, split at
  65 535 bytes on char boundaries) directly under `MSTv`, after
  `NGSv`. The reader claims it only when every child is a `JSSs`
  string; otherwise the node bags whole.
- Instance parameters: `DTAv` on `ALSv` between `OBNs` and the
  animation family — one `DCEv` (`DCKs` key + `DCVs` string) per
  entry, canonical key order. The reader claims it only when every
  child is `DCKs`+`DCVs` and no key repeats; anything else bags the
  node. Values are opaque strings; the corpus's `currentValues` array
  marshals through JSON inside one `DCVs`.

**2. Engine commands are plain journaled mutations.**
`symbol.setScript {symbol, source?}` installs/clears the definition
script (whole-library capture — the script is library state, not
per-object state). `symbol.setParam {id, name, value, kind}` and
`symbol.setCustomData {id, key, value?}` are create-or-update /
set-remove writes on the instance's store with
`ObjectInverse::Mutated` captures. A refused call mutates nothing.

**3. Re-derivation drains through the registry (Decision 0011).**
The engine layer cannot depend on the scripting layer, so
`scripting::rederive_symbol(session, engine, instance, op_code)`
owns the chain: attach → project the `Widget` global → run the
definition script → drain the queued intents through
`Session::apply_delta` in queue order → re-attach. The `Widget`
projection carries only the property shapes corpus scripts use —
`elem.customData` (a Proxy whose writes queue `symbol.setCustomData`),
`GetObjectByName` over named definition members
(`pathAttributes.fillColor`/`brushColor`, `textChars`, `font`),
`opCode`, and `propString` (the Component panel's `&;&`/`&|&`
encoding, read/write). Unnamed members are invisible to
`GetObjectByName`; colors are CSS `#RRGGBB` at the script boundary
and unsigned ABGR raw on the wire (`| 0` int32 coercion is a bug —
it flips the alpha bit negative).

**4. Journal replay needs no JS engine.** Because every
re-derivation effect is a registry command, `Session::reopen`
reproduces the whole chain byte-exactly — the golden
(`crates/app/tests/rich_symbols.rs`) pins replay plus save → open →
save stream stability, and `corpus/fixtures/rich.fw.png` pins the
wire shape end to end (built through the registry + host, no
hand-written nodes).

## Consequences

- `app` takes `scripting` as a dev-dependency only; the editor stays
  pure and L3 never depends on L4. A future shell re-derive button
  will need an explicit host handle in the app layer — that is the
  point where app gains the production dependency, deliberately.
- The parameter UI (if added later) writes `symbol.setParam` and
  calls `rederive_symbol`; it must never mutate members directly.
- Reader claim rules keep unknown-but-plausible `DTAv`/`JSVv` shapes
  lossless in the bag (Decision 0008 posture).
