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
2. **Read-side DOM projection** — `fw.documents[]`,
   `fw.getDocumentDOM()`, page/layer/element trees as JS objects over
   `doc::Document` (selection read, `elements[]`, `width/height`,
   frame/state counts). No mutation yet. **Stopping condition:**
   battery pins projected values against a fixture document
   (counts, ids, names, bounds); a shipped walk-and-log script runs
   and its log matches expectation.
3. **Mutating host API → registry commands** — the corpus's mutating
   calls (`dom.addNew*`, `dom.set*`, `fw.setUpRevert`, smartShape
   handlers) dispatch to engine command ids with JSON args; unknown
   names throw typed; the command id + args land in the journal like
   any GUI mutation. **Stopping condition:** replay equivalence —
   script run ≡ journal replay ≡ direct command invocation (byte-equal
   documents); battery covers every implemented mutating name.
4. **`.jsf` loading + `Files`/`fw.runScript`** — script files load
   through `engine::platform::FileSystem` (app/user Command dirs,
   virtual FS on web); `fw.runScript(path)`, `fw.runCommands(...)`,
   `Files.read/write/exists` (scoped to the FS trait — no direct
   `std::fs` in the crate); ES3 quirk shims recorded in a
   `docs/decisions/` quirk ledger as discovered. **Stopping
   condition:** a shipped `Commands/` script loads from the virtual FS
   and runs; quirk ledger has its first entries with pinned repro
   tests.
5. **Corpus run harness (`xtask scripts`)** — copies the 89 shipped
   scripts from `source/` into scratch (never committed), runs each
   headless against fixture documents, emits a committed report:
   per-script status (ran/threw/not-implemented-name/hung→refused),
   implemented-name coverage, DOM effect diff where the script's
   effect is deterministic. **Stopping condition:** report generated +
   committed; **gate floor set from it** (corpus scripts running
   headless, rising-only); the not-implemented list becomes the next
   task's work list.
6. **History replay + Save Steps As Command** — journal rows export
   as a runnable command file; shell History panel gains save/replay;
   `firecraft-cli` replays command files identically to journals
   (same registry path). **Stopping condition:** equivalence test —
   GUI-session journal → saved command file → replay on a fresh doc ≡
   original document (byte-equal).
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
