# 0001 — Scripting engine: `boa_engine`

- **Status:** Accepted (M1 spike, 2026-10-10)
- **Deciders:** Michael Kinney, Claude
- **Scope:** Every layer that executes Fireworks `.jsf` scripts and the
  scripting host API (`fw`, `smartShape`, document DOM objects).
- **Supersedes:** none · **Superseded by:** none

## Context

FireCraft reimplements the Fireworks scripting surface: shipped `.jsf`
files (Auto Shape handlers like `InsertSmartShapeAt`) are executed
against a FireCraft-provided host object model. The engine must work in
**every release target, WASM first** (PLANNING: WebOS embedding is the
primary target). Candidates: `rquickjs` (QuickJS-NG bindings, C compiled
via `cc`) and `boa_engine` (pure-Rust JS engine).

## Spike

Two throwaway bench crates (kept out of the workspace, under scratch)
embedded each engine and ran the same protocol:

1. Load a real shipped file: `Auto Shape Tools/Rounded Rectangle.jsf`
   (31,955 bytes, 941 lines, UTF-8 BOM + CRLF).
2. Provide a trivial host prelude: `fw`, `smartShape`, `Files`,
   `__tooltips`, `Path`/`Contour` constructors, and a Proxy-based
   "host array" that materializes point objects when script code does
   `array.length++` (the Fireworks host semantic the shipped scripts
   rely on; a plain JS array leaves `undefined` holes and both engines
   failed identically until the Proxy stub was in place — useful
   confirmation that the host-object contract, not the engine, is the
   integration surface).
3. Execute the top-level `switch (smartShape.operation)` with
   `operation = "BeginDragInsert"`, which runs
   `RegisterDragInsert(InsertSmartShapeAt())` — the real insert handler
   end to end.
4. Measure and try wasm32 builds.

## Evidence (measured 2026-10-10, this host)

| Criterion | rquickjs 0.9 | boa_engine 0.20 |
|---|---|---|
| Script executes (native release) | OK | OK |
| Native exec time (median of 3) | **2.4 ms** | 13.9 ms |
| Native binary size | **1.61 MiB** | 11.5 MiB |
| wasm32-unknown-unknown | **fails** — `cc` cannot build QuickJS `libregexp.o` for freestanding wasm | **builds, 0.47 MiB .wasm** (after `getrandom` `js` feature) |
| wasm32-wasip1 | **fails** — `dlopen` crate does not compile | not required (unknown-unknown works) |
| Toolchain drag | C compiler required for every target, incl. cross builds | none (pure Rust) |

## Decision

Adopt **`boa_engine`** for the `scripting` layer.

The primary release target is WASM; rquickjs does not build for any
wasm32 target without a C-to-wasm toolchain story that does not exist
today, while boa builds out of the box. The 5.8× native interpreter
slowdown is acceptable because scripting runs at UI-event granularity
(drag/insert handlers, batch automation), never per-pixel or per-frame;
the 10× native binary-size penalty does not affect the wasm artifact,
which is smaller than rquickjs could have produced anyway (it produces
none).

## Consequences

- The future `scripting` crate depends on `boa_engine` + (wasm32 only)
  `getrandom/js`.
- The host-array Proxy semantic (length-growth materializes objects) is
  part of the host model contract and belongs to the host layer, not the
  engine choice.
- If a native-only automation path ever needs more speed, rquickjs can
  be reintroduced behind a native-only feature flag without touching the
  wasm path. That is explicitly out of scope until measured as a need.
- Boa 0.20 native memory footprint and GC pauses are unmeasured; the
  engine crate milestone must add a long-running-session benchmark
  (batch over N scripts) before the first alpha.

## Validation

- Spike protocol above; both benches exit 0 on the shipped script.
- Re-run recipe: rebuild either bench under the scratch dir and compare
  against this table before revisiting the decision.
