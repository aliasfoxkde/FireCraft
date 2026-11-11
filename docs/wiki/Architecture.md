# Architecture

FireCraft is a Rust workspace with a shared document model and command engine behind desktop, browser, CLI, scripting, and MCP surfaces.

## Dependency layers

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:79ec6440ff8af37975a294760ec2bf87e73c24d31dd69dd6eebe1d33517887d5 -->

| Layer | Crates | Responsibility |
|---|---|---|
| L0 | `fwpng`, `geom`, `color`, `raster` | Format primitives, geometry, color, tile storage |
| L1 | `doc` | Editable document model |
| L2 | `vector`, `filters`, `text`, `ops` | Feature engines |
| L3 | `compose`, `gpu`, `engine` | Rendering and command orchestration |
| L4 | `io`, `scripting`, `webexport` | Import/export codecs, scripting runtime, and web export |
| L5 | `app`, `app-cli`, `automation`, `firecraft-web` | User and automation shells |
| L6–L7 | `testkit`, `xtask` | Test support and verification tooling |

<!-- /coderabbit-ai-generated:v1 -->

The enforced source is [`xtask/src/layers.rs`](https://github.com/aliasfoxkde/FireCraft/blob/main/xtask/src/layers.rs); see [PLANNING.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PLANNING.md) for the complete architecture and decisions.

## Editing model

Mutations flow through the engine command registry and journal. GUI, CLI, script, and bridge surfaces call the same commands, keeping undo, refusal behavior, and document changes consistent.

## Rendering

The GPU shell presents the canvas; the deterministic CPU compositor is the reference path for golden-image checks. Document serialization remains independent of the UI shell.
