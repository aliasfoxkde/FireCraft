# Architecture

FireCraft is a Rust workspace with a shared document model and command engine behind desktop, browser, CLI, scripting and MCP surfaces.

## Dependency layers

| Layer | Crates | Responsibility |
|---|---|---|
| L0 | `fwpng`, `geom`, `color`, `raster` | Format primitives, geometry, color, tile storage |
| L1 | `doc` | Editable document model |
| L2 | `vector`, `filters`, `text`, `ops` | Feature engines |
| L3 | `compose`, `gpu`, `engine` | Rendering and command orchestration |
| L4 | `io`, `scripting`, `webexport` | Import/export codecs, scripting runtime and web export |
| L5 | `app`, `app-cli`, `automation`, `firecraft-web` | User and automation shells |
| L6–L7 | `testkit`, `xtask` | Test support and verification tooling |

The enforced dependency graph is [xtask/src/layers.rs](https://github.com/aliasfoxkde/FireCraft/blob/main/xtask/src/layers.rs). See [PLANNING.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PLANNING.md) for the complete architecture.

## Editing model

Mutations flow through the engine command registry and journal. GUI, CLI, script and bridge surfaces call the same commands, keeping undo, refusal behavior and document changes consistent.

## Platform and shared SDK direction

Keep FireCraft-owned document semantics, format compatibility and product UX in this repository. Reusable mechanisms may be adopted from [storytold/craft-libs](https://github.com/storytold/craft-libs) only after API/target compatibility and measured benefit are established. The planned SDK uses versioned command/capability schemas and native, browser, CLI/MCP and WebOS adapters over the same command service. See the [platform integration plan](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/WEBOS-PLATFORM-INTEGRATION.md).

## Rendering

The GPU shell presents the canvas; the deterministic CPU compositor is the reference path for golden-image checks. Document serialization remains independent of the UI shell.
