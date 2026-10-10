# M5 — Editor shell (native + web)

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** a usable app: open → inspect → edit → save `.fw.png`, in
the browser and on desktop, where every mutation — UI click, CLI
invocation, scripted batch — goes through one command registry, so the
same operation sequence produces byte-identical documents from any
surface.

**Inputs:** M3 `doc` model + M4 `compose` renders; stack decisions
PLANNING §7.1/§7.6 (egui 0.36 + eframe, winit, trunk web build);
scripting decision [0001](../../decisions/0001-scripting-engine.md)
(boa is M9's concern — M5 only reserves the command-id surface).

**Authority:** ROADMAP M5 gate. The gate's core claim is *surface
equivalence*: UI, CLI, and journal replay are three frontends over one
registry — equivalence is enforced by making non-registry document
mutation unrepresentable in shell code.

## Tasks (atomic)

1. **`engine` crate (L3): command registry + inverse deltas** —
   `Command` trait: name (static catalog id), typed payload (serde),
   `apply(&mut Document) -> Result<Delta>`; `Delta` carries the inverse
   data captured before mutation (no reverse-command duplication). A
   name-keyed registry; unknown names error; duplicate registration
   errors. serde everywhere payloads and deltas flow (journal, CLI).
   **Stopping condition:** property test — for a randomized command
   sequence, apply→undo→apply reproduces the same document as
   apply-once; undo(apply(x)) restores the original document.
2. **`engine`: transactional history + journal replay** — undo/redo
   stacks over `(command, delta)` entries; transactions group commands
   atomically (one undo step); an append-only journal records applied
   commands with payloads; replay of a journal on a fresh session
   rebuilds the identical document (byte-compare through `doc::fwmap`
   write). **Stopping condition:** the gate item — undo/redo survives
   session-restart journal replay: apply, record, replay in a fresh
   session, undo, and the documents match the original path.
3. **`engine`: session + platform services** — `Session { document,
   history, journal, dirty }`; platform-service traits (`FileSystem`,
   `Dialogs`, `Clipboard`) injected at construction — engine never
   touches `std::fs`, so the crate is wasm-clean by construction.
   **Stopping condition:** `cargo check -p engine --target
   wasm32-unknown-unknown` green; a testkit `MemoryFs` exercises
   open/save.
4. **Built-in catalog v1** — the editing commands M5's tools need, each
   with captured deltas: set layer display, set cell visibility, set
   object opacity/blend/visible/locked, set fill/stroke solid colour,
   rename layer/object, add/delete page, set canvas size, move object
   between cells, delete object, reorder z. **Stopping condition:**
   `panic_hunt` fuzz target — random command sequences against random
   documents never panic (fuzz anchor + 60 s campaign).
5. **`app-cli` crate (L5)** — `firecraft-cli info <file>` (document
   summary as JSON), `convert <in> <out>` (canonical re-emit through
   the full read/write path), `apply <journal>` (replay through the
   same registry). **Stopping condition:** the equivalence gate in
   headless form — replaying a journal via CLI and via engine in-test
   produce identical documents (same bytes).
6. **`app` crate (L5): egui shell v1** — eframe desktop app: canvas
   widget with pan/zoom + HiDPI (compose renders the page to an
   `ColorImage` texture; GPU tiling is later), tools (pointer, subselect,
   rect/ellipse/line creation, zoom/hand), panels (layers, pages,
   states, properties), menus generated from the registry. Shell code
   may not touch `Document` except through `Session::apply`. Native
   open/save via the injected traits.
   **Stopping condition:** open a corpus fixture, apply commands from
   the UI, save; the saved file round-trips and replays.
7. **`web/` trunk build** — the same `app` shell compiled for wasm32
   (trunk): browser file open/save (File System Access API with
   download fallback), postMessage bridge (`openDocument`,
   `exportPng`) for WebOS embedding.
   **Stopping condition:** `trunk build` produces the bundle; the
   bridge handlers replay through the registry (no side door).
8. **Gates** — `xtask layers` (engine L3, app/app-cli L5), `xtask wasm`
   extended to engine/app + the web bundle compile, `panic_hunt`
   campaign recorded, equivalence tests green, `ci` green, devlog +
   parity floors (command count floor? recorded in devlog first,
   floored only when stable), commit + push gitforge→origin.

**Verification (milestone gate):** same operations scripted via CLI and
UI produce identical documents; `panic_hunt` fuzzes every registered
command; `xtask wasm` ships the web bundle; undo/redo survives
session-restart journal replay.

**Out of scope:** scripting host / `.jsf` execution (M9 — the registry
reserves the ids), GPU canvas path (wgpu `vello_gpu` later — M5 paints
through `compose`), text layout (M6), vector ops (M6), path booleans
(M6), filters (M7), symbols (M10), performance work beyond keeping the
16×16-class corpus renders interactive.
