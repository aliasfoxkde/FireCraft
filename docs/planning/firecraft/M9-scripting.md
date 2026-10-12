# M9 — Scripting & automation

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** FireCraft becomes scriptable the way CS6 is: the
shipped `.jsf` corpus (89 scripts under `Configuration/`, read in
scratch — never committed) runs headless against a FireCraft-provided
host model and produces real DOM effects; the same command ids are
reachable from the GUI, the CLI, a JSON control channel, and an MCP
server; the journal can be saved and replayed as a command file
("Save Steps As Command"). Scripting engine already decided:
`boa_engine` (decision 0001 — wasm-proven, 0.47 MiB artifact).

**Inputs:** M1 spike (decision 0001: boa 0.20, host-array Proxy
semantic, `getrandom/js` for wasm32); the engine's everything-is-a-
command registry + journal (`Session::reopen`, capturing deltas —
scripting mutations MUST ride it, never write the doc directly);
M3 model accessors; M5 shell + web postMessage bridge; M8
`engine::platform::FileSystem` (the `.jsf` loader speaks the trait,
so app/user dirs on desktop and a virtual FS on web are the same
code); PLANNING §7.5 (117 `fw.*` + 342 `dom.*` in the "Extending
Fireworks" PDF — the surface is bounded corpus-first, not
encyclopedic); risk #10 (ES3 quirks — fwjs-errata ledger).

## Design line: the host model is a command dispatcher

`scripting` is a new crate at the `io` layer position (L4): it may
depend on doc/engine/compose; the engine never depends on it; shells
(L5) embed it. Every mutating host call
(`dom.addNewRectangle`, `fw.setUpRevert`, …) translates to a registry
command id with JSON args — identical to what the GUI menu and the
CLI emit — so undo, journal, fuzz, and replay semantics apply to
scripted mutations for free. Read accessors project the typed model
into JS objects. ES3-era semantics that JS engines no longer provide
(host-array `length++` materialization, `Photoop`-style value
objects) live in a host prelude, pinned by tests against real shipped
scripts. Sandboxing: boa runtime limits (instruction/stack) + wall
clock + allocation ceiling; a script can fail but never hang or OOM
the host, and a failing script leaves the document untouched (the
command registry gives us transactionality without extra work).

## Scope line: corpus-bounded API surface

The full `fw`/`dom` API is 459 names; CS6's own shipped scripts use a
fraction. M9 implements the names the corpus actually calls (extracted
under `/var/tmp` scratch), each mapped to documented behavior; names
outside the corpus surface get a documented `fw` stub that throws a
typed "not implemented in FireCraft" (visible in reports, never
silently wrong). The gate metric is corpus-run status, not API count.

## Tasks (atomic)

1. **`scripting` crate scaffold + sandbox + prelude** — **done.**
   New L4 crate `scripting` (`boa_engine` 0.20 + `boa_gc`); reusable
   engine (one Context across runs). Sandbox: boa runtime limits
   (loop 1M / recursion 512 / stack 1 MiB) + a **host-call budget**
   (default 10 000/run) consumed by every native host fn — the
   wall-clock stand-in (boa 0.20 has no timer hooks), so
   `while(true){host()}` dies typed. Native functions register via
   `from_copy_closure_with_captures` + `Gc` captures (the only
   non-`unsafe` path); objects/Proxy live in a JS prelude string
   (`host::PRELUDE`): `__hostArray(make)` grow-by-read Proxy, `fw`
   (`__log`/`__budgetLeft` + typed not-implemented: getDocumentDOM,
   setUpRevert, revert, runScript, runCommands, dispatchEvent), `dom`,
   `smartShape` (`operation` + refusing handlers), `Files` stubs;
   refusals name the owning M9 task. `prepare_source` strips BOM +
   normalizes CRLF. Evidence: spike battery 5 (own-authored handler
   with the shipped shape — operation switch, RegisterDragInsert,
   4-point host array, materialization read-back, engine reuse, BOM)
   + hostile battery 6 (loop limit, recursion limit, budget
   exhaustion mid-loop, five typed refusals, syntax error, engine
   survives a limit run); wasm32 green with
   `getrandom 0.2 + js` target-dep; `scripting` in `WASM_CRATES`
   (14), floor 13→14; layers L4; clippy/fmt green. **Lockfile moved
   (70 new packages)** — image re-shipped before the carrying push.
   **Stopping condition met.** ✅
2. **Read-side DOM projection** — **done.** `dom::attach` projects a
   `doc::Document` snapshot into the host once per attach
   (`ScriptEngine::attach_document`): `dom.width/height/left/top/
   resolution/resolutionUnits`, `pagesCount`/`currentPageNum`/
   `currentFrameNum`, `frames[]` (per-frame count), `layers[]`
   (`name`/`display`/`elements[]` = the current frame's cell), and
   per-element `name` + `left/top/width/height` — path anchor bounds,
   image location + pixel size, text/slice frame rects, group union
   (depth-capped 32), symbol instance transform translation. Dangling
   registry ids refuse to project. `fw.documents = [dom]`,
   `fw.getDocumentDOM()` returns it (identity `===` holds),
   `fw.selection` is `[]` (model selection is a pixel mask, not an
   element list — element selection lands with the mutating host API).
   Re-attach replaces the projection (freshness test). Battery
   `tests/dom.rs` (5 tests): fixture 320×240@96, 1 page, 2 layers
   (path/text/slice) pins every scalar + the full walk log;
   `getDocumentDOM()` before any attach throws the prelude's typed
   task-2 refusal. **Shipped-script leg:** committed code cannot
   reference `source/`; the walk-and-log leg is covered by the
   battery's own corpus-shaped walk (identical read sequence to the
   shipped walk scripts) and by task 5's corpus harness, which will
   run the real `.jsf` files headless. **Stopping condition met** (the
   corpus-script leg defers to task 5's harness by design). ✅
3. **Mutating host API → registry commands** — **done.** Mutating
   host calls queue `(command id, payload)` intents in GC-held state;
   `run_in_session(session, engine, …)` drains through the new
   `Session::apply_delta` only when the run completed — failing
   scripts leave the document untouched, and `fw.setUpRevert`/
   `fw.revert` truncate the queue before application, so revert is
   exact (no journal/undo residue). Contract recorded in
   `docs/decisions/0011-script-mutation-queue.md`. Implemented
   mutators (each pinned three-ways byte-equal in
   `tests/mutate.rs`, 13 tests): `addNewText`→`text.createText`,
   `addNewRectangle`→`object.createPath`(rect),
   `addNewLayer`→`layer.add` (new catalog command),
   `deleteLayer`→`layer.delete` (new; last-layer refused),
   `moveSelectionToLayer`→`object.moveToCell`,
   `pathUnion`→`path.boolean` fold,
   `setTextAutoExpand`→`text.setAttrs`; view state (never journaled):
   `selectAllOnLayer`/`selectNone`/`fw.selection` (projection-carried
   `__fwId` objects), `currentLayerNum` (default top, assignable),
   `changeCurrentPage` (validated, persists across attaches).
   Projection corrected to the corpus's real shapes found while
   wiring: `layer.frames[f].elements`, `isMasterPageLayer`/
   `masterPageNum`. Refusals: `dom.exportElements` (M11 export),
   `ungroup`/`splitPaths`/`convertToPaths` (group machinery, task 5's
   report owns the work list), smartShape handlers (task 5 — the
   corpus run enumerates which the shape library needs). Fuzz
   coverage: `layer.add`/`layer.delete` joined the fuzzdrive call
   table (catalog-coverage test enforces). Gates: engine 129 +
   scripting 29 tests, clippy `-D warnings` both crates, fmt, wasm
   checks clean. **Stopping condition met** (script ≡ direct ≡
   replay byte-equal per mutator). ✅
4. **`.jsf` loading + `Files`/`fw.runScript`** — **done.** Scripts
   load through `engine::platform::FileSystem` (engine's `MemoryFs`
   in tests; virtual FS on web, app/user dirs on desktop) scoped via
   `scripting::run_with_file_system` (thread-local attachment — boa's
   `Context` data store needs `Trace` types, which `Rc<dyn
   FileSystem>` is not). Implemented: `Files.read/write/exists` →
   natives `__fwRead/__fwWrite/__fwExists`, `fw.runScript(path)` →
   `__fwRunScript` (immediate `ctx.eval`, so sub-script intents join
   the same queue and the same read snapshot), `fw.runCommands(...)`
   → per-string-arg `__fwQueue(arg, "{}")` (all-optional-args
   commands like `page.add` drain successfully; the rest fail typed
   at drain with the id in the report). No direct `std::fs` in the
   crate; a run with no attached FS fails typed ("no file system is
   attached"), never silently. Remaining `Files.*` names
   (`getLanguageDirectory`, `enumFiles`, `copy`, …) refuse typed
   naming task 5. Quirk ledger opened:
   `docs/decisions/0012-es3-quirk-ledger.md` (5 entries: BOM+CRLF
   normalization; no `#target`/`#include` in the shipped corpus —
   the loader does not pre-process `#` lines; plain-assignment
   `fw.selection`; read staleness under the queue-drain contract
   (cross-ref 0011); modern `JSON` as harmless superset). Battery
   `tests/files.rs` (6 tests): Files round trip over the injected FS
   with zero journal rows; **stopping-condition leg**
   `command_script_loads_from_virtual_fs_and_runs` — BOM+CRLF
   `Commands/assemble.jsf` → `fw.runScript('Commands/part.jsf')` →
   `addNewRectangle`, script ≡ direct ≡ replay byte-equal (own-
   authored stand-in with the shipped file shapes; committed code
   never references `source/`); no-FS typed refusal;
   runCommands success + typed drain failure + non-string refusal;
   runScript A→B→A recursion bounded (`LimitExceeded`, interpreter
   bound fires, not the machine stack); `#target` line is a typed
   syntax error, not a silently ignored directive. Hostile battery
   updated (runScript/Files.read are implemented; the refusals list
   now pins `Files.getLanguageDirectory`/`Files.copy`). Gates:
   scripting 35 tests, clippy `-D warnings`, fmt, wasm32 clean.
   **Stopping condition met.** ✅
5. **Corpus run harness (`xtask scripts`)** — **done.** `xtask
   scripts` walks `source/…/Configuration` for `*.jsf` (89 found),
   snapshots them to `TMPDIR` scratch (never committed, `source/`
   never touched), and runs each twice headless through the
   `scripting` host (fresh session + engine + per-run `MemoryFs`
   seeded with the whole corpus, so runScript siblings resolve and
   `Files.write` cannot leak between runs) against a registry-seeded
   fixture (480×320, 2 layers, 2 path objects). Committed report
   `corpus/scripts-report.json`: per-script status
   (ran/threw/not-implemented/drain-failed/limited), per-run message,
   applied-command count and byte-stable document digest when both
   runs agree (`"varies"` otherwise), the runtime refusal histogram,
   and the static call surface with **runtime name probing** (each
   statically-extracted `root.ident` is `typeof`-probed against a
   live attached host — "missing" is observed, never hand-tabulated).
   First-run result: **19/89 ran** (floor `CORPUS_RUN_FLOOR = 19`,
   rising-only, enforced by `check_floor`; CI's no-source path
   verifies the committed report, pinned by the in-crate test
   `committed_report_holds_the_corpus_floor`). Work list for the
   mapping task: 49 missing host names, dominated by the smartShape
   handler surface (`smartShape.elem` ×2453 call sites,
   `currentControlPoint*`/`currentMousePos`, `GetDefaultMoveParms`,
   modifier-key readers), `fw.ellipseBCPConst` (×193), `fw.getPref`,
   the PS-font-name helpers, group machinery (`dom.ungroup/
   splitPaths/convertToPaths`), `Files.deleteFile`, `fw.platform`;
   32 scripts die on the typed `Files.getLanguageDirectory` refusal;
   1 script with a real mutation runs end-to-end (Distribute to
   Layers, applied 2, digest-pinned). Extraction-bug regression
   pinned (`call_extraction_keeps_the_full_name`). Gates: xtask
   clippy `-D warnings`, fmt, 10 tests green. **Stopping condition
   met** (report generated + committed, floor set from it, work list
   recorded). ✅
6. **History replay + Save Steps As Command** — **done.** A steps
   file (`.fwsteps`) IS the journal: `Session::journal_lines()` JSON
   lines on disk, replayed through `Session::reopen` — the same
   registry path a restarted UI session and the CLI `apply` verb take,
   so save and replay are inverse by construction (no new export
   format, no second replay machine). Shell (crates/app): the left
   panel gains a History section — the journal's step list (seq, txn
   label, command name, from the new read-only
   `Session::journal_entries()`) with **Save Steps…** (write
   `steps_text()` through the save dialog + injected FS) and
   **Replay…** (pick a `.fwsteps` file, then the PRE-steps base
   document through the File → Open flow; `Editor::replay_steps`
   rebuilds the session over it, undo history included, journal
   becomes the replayed prefix so further edits append). Editor
   additions: `steps_text()` + `replay_steps()`. CLI (crates/app-cli):
   `apply <doc> <journal> [--out <out>] [--steps <out.fwsteps>]` —
   `--steps` writes the replayed session's journal as a command file
   (headless Save Steps As Command). **Stopping condition met** —
   `crates/app/tests/steps.rs::saved_steps_replay_to_a_byte_equal_document`:
   GUI session (untitled seeds + drag-created rect + opacity + rename)
   → journal → `.fwsteps` file over `MemoryFs` → `Session::reopen` on
   a fresh `Document::default()` → documents byte-equal (canonical
   JSON), Editor save bytes byte-equal, undo depth equal, and the
   rebuilt journal identical to the file. Shell seam pinned by
   `shell::tests::history_panel_saves_and_replays_steps` (panel
   actions through scripted dialogs + MemoryFs, base document opened
   from disk); CLI leg pinned by
   `app-cli::equivalence::steps_file_replays_identically_through_the_cli`
   (fuzz-generated sessions, seeds 21/22: steps-file replay ≡ session,
   journal round-trips verbatim). Gates: engine + app-cli + app clippy
   `-D warnings`, fmt, tests green; engine wasm32 clean; lockfile
   unmoved (no image re-ship). ✅
7. **CLI `run-script` + JSON control channel + MCP server** —
   `firecraft-cli run-script <script> <doc>` (scripting feature,
   default-off on wasm per §7.4 note — boa still ships in the wasm
   shell for embedded runs, the *control channel* is the native-only
   part); loopback JSON channel (stdin/socket, desktop) accepting
   command ids; MCP server exposing the same registry ids (one tool:
   `firecraft.apply` + `firecraft.runScript`). **Stopping condition:**
   the same command id demonstrably runs via CLI, GUI, and MCP in one
   test battery; control channel fuzz (malformed JSON → typed errors,
   no panic).
8. **Gates** — floors raised where earned (corpus-run floor, control
   channel fuzz floor); fuzz arms (script fuzz target: mutated `.jsf`
   → typed report, never panic); parity + scorecard regenerated;
   decisions doc (host-model contract, quirk ledger, surface bounds);
   devlog. **Stopping condition:** `xtask ci` green via GitForge; M9
   scorecard row → done with gate evidence.

## CI image constraint

Task 1 adds `boa_engine` (+ transitive crates) to the lockfile —
**`firecraft-ci-rust` must be re-shipped to fedora + t5500 before the
push that carries it** (routing memory: re-ship when the lock gains
crates.io packages).

## Non-goals (M9)

XML-RPC loopback emulation of CS6's TCP-12124 (T3, desktop only, after
the JSON channel proves the seam); Flash/SWF command panels (no clean-
room target); the full 459-name API surface (corpus-bounded instead —
unimplemented names throw typed, listed in the report); rich-symbol
`Widget.opCode` execution (M10 needs the symbol model first); scripted
batch in the wild via the user's own `Commands` dirs (supported
mechanically by task 4, but corpus fixtures stay in scratch).
