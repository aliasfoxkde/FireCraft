# 0007 — Text run (`TRNs`) wire encoding

- **Status:** Accepted (M7 task 7, 2026-10-11)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `doc::TextRun`, the `fwmap` reader/writer for `TXTv`/`TFSv`.
- **Supersedes:** none · **Superseded by:** none

## Context

The node dictionary documents `TRNs` only as "Text Run — used for parts
of text which share formatting options"; it records no child list and no
payload type. No Fireworks-authored file with a text object exists in
the corpus yet (`corpus/incoming/` is empty; the CS6 install ships no
`.fw.png` sample; the symbol libraries carry no `TXTv` chunks), so there
is no observational ground truth for how the run characters are laid
out on disk. The M7 task-7 stopping condition still requires *attribute
round-trip through the format* — which FireCraft can satisfy for files
it writes regardless of what Fireworks writes — without inventing a
claim about Adobe's encoding.

## Decision

1. **The model holds runs as plain characters** (`TextRun { text:
   String }`) under the enclosing `TFSv` style set. One style per
   `TFSv`, multiple `TRNs` per `TFSv`.
2. **The writer emits each run as a string node** — `TRNs` with a
   `NodeValue::Str` payload, the same shape as `OBNs`/`FONs`.
3. **The reader accepts a string node** as a run's characters. Any
   other payload shape (container, int, float) bags verbatim at its
   recorded position — a foreign file's runs still round-trip whole,
   just unmapped.
4. **A second `TFSv` under one `TXTv` bags verbatim**; the first is the
   model slot, mirroring how `PATv` and `PBLv` behave on `PTHv`.
5. **`TXTv`-level style mirrors stay bagged.** The dictionary lists
   `FONs`/`PTSf`/`BOLb`/… copies at the `TXTv` level alongside the
   `TFSv`; FireCraft treats `TFSv` as the styling authority and
   preserves the mirrors byte-exactly without modeling them.

## Consequences

- FireCraft-authored files round-trip byte-exactly today, and foreign
  files never lose bytes — the bag covers every shape we cannot map.
- When the Wine-driven corpus generation (M8) produces a
  Fireworks-authored text object, the observed `TRNs` shape becomes the
  authority: the reader maps it natively, and this decision is
  superseded with the recorded discrepancy — per the standing rule that
  corpus beats dictionary.
- Until then, cross-application text interchange is a non-goal: M8's
  import/export targets images and vectors; text interop waits on
  corpus evidence like every other unmapped encoding.
