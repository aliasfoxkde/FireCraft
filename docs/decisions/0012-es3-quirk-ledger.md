# Decision 0012 — ES3/host quirk ledger

Date: 2026-10-12 · Milestone: M9 task 4 · Status: accepted (living ledger)

## Context

Shipped Fireworks scripts are ES3-era JS written for a host that is
not a modern engine: BOM+CRLF files, Flash-era globals, live host
arrays. FireCraft's host is boa 0.20 plus a prelude, so every
behavioral gap between the two must be either shimmed or recorded —
a script that "almost runs" is worse than one that fails typed.
This ledger is the record; each entry names the evidence and the
test that pins the decision. New quirks discovered by the corpus run
harness (M9 task 5) append here with their pins in the same change.

## Entries

### 1. BOM + CRLF normalization (shimmed)

Shipped `.jsf` files are UTF-8 **with BOM** and CRLF line endings.
`scripting::prepare_source` strips a leading `U+FEFF` and normalizes
CRLF→LF before every evaluation — including scripts loaded through
`fw.runScript` from the attached `FileSystem`.

Pinned by: `spike` battery's BOM test; `tests/files.rs::
command_script_loads_from_virtual_fs_and_runs` (BOM+CRLF files load
through the virtual FS and run).

### 2. No preprocessor directives in the shipped corpus (negative)

The 89 shipped scripts (extracted under `/var/tmp` scratch) contain
**no** `#target` / `#include` lines; cross-file composition happens
through `fw.runScript(path)` chains instead. The loader therefore
does not pre-process `#` lines: a script containing one is a syntax
error (typed `Outcome::Threw`), never a silently ignored directive.
If the corpus harness meets a real directive, this entry gains a
shim with its own pin.

Pinned by: `tests/files.rs::preprocessor_directives_are_not_processed`.

### 3. `fw.selection` is a plain assignable property (shimmed)

Corpus scripts replace the selection by plain assignment
(`fw.selection = arr`, e.g. the `new Array()` + distribute pattern)
and mutating methods read it back within the same run. The projection
supports exactly that: `fw.selection` projects from host-side state,
assignments are plain JS property writes, and `__fwId` on each
projected element carries the command-payload id form
(`{index, generation}`) verbatim (decision 0011 §5).

Pinned by: `tests/mutate.rs::
move_selection_to_layer_is_direct_and_replay_equivalent`
(assign in `selectAllOnLayer`, read back in `moveSelectionToLayer`);
`tests/mutate.rs::created_element_becomes_the_selection` (selection
persists across runs).

### 4. Read staleness under the queue-drain contract (documented, gated)

CS6 applies mutations live, so `dom.addNewText(...)` is readable on
the next line. FireCraft queues mutations and drains at run end
(decision 0011), so between queue and drain, reads project the
pre-mutation snapshot. This is a semantic difference the corpus
harness must carry: shipped mutating scripts that read back what they
just created mid-run will show it in the report, and the mapping
gains shims there.

Pinned by: `tests/mutate.rs::failing_script_leaves_document_untouched`
and every script ≡ direct ≡ replay equivalence test (the drain model
is the contract under test).

### 5. Modern `JSON` global is a harmless superset (shimmed)

ES3-era scripts predate the `JSON` global — the corpus builds strings
by concatenation — so boa's modern `JSON` never collides with shipped
code, and the FireCraft mutators use `JSON.stringify` for payload
construction. Providing it is a strict superset; no quirk.

Pinned by: every payload-building mutator test
(`tests/mutate.rs::add_new_text_is_direct_and_replay_equivalent`
et al.).

## Consequences

- `Files.read/write/exists` and `fw.runScript` resolve only through
  the attached `engine::platform::FileSystem` (`run_with_file_system`
  scoping) — the same code path on desktop and the wasm virtual FS;
  a run with no FS attached fails typed, never silently.
- `fw.runCommands` queues each string argument with an empty payload:
  commands whose arguments are all optional drain successfully;
  the rest fail typed at drain with the command id in the report.
- Files.* names beyond read/write/exists (`getLanguageDirectory`,
  `enumFiles`, `copy`, …) refuse typed naming M9 task 5, which owns
  wiring the ones the corpus run actually needs.
