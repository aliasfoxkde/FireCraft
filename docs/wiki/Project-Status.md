# Project Status

**Snapshot reviewed:** 2026-11-11\
**Source:** `main` at `b61f198`\
**Latest published release:** [v0.2.5](https://github.com/aliasfoxkde/FireCraft/releases/tag/v0.2.5) (2026-11-03)

FireCraft is a functional Rust graphics editor with a shared document engine and command registry across the desktop GUI, WebAssembly app, CLI, scripting host, and MCP server. The current source is newer than the latest published release; do not assume the hosted app contains every change on `main`.

## Current snapshot

| Surface | Current state |
|---|---|
| Engine commands | 253 registered command IDs |
| Shell reachability | 164/253 = 64.82% in the 2026-11-11 measurement; the command palette exposes the full registry, but this gate measures the CS6-visible shell |
| CS6 DOM matrix | 268 carried / 3 partial / 0 missing of 356 verbs; this is scripting/API parity, not GUI parity |
| Menu census | 2 inert rows of 296; eight other rows are explicitly feature/evidence-gated |
| User-visible parity | Full inventory published in [CS6-PARITY.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md) |
| Phase 20 | 20.1 and 20.5–20.7 are on main; Optimize/timeline are in [PR #118](https://github.com/aliasfoxkde/FireCraft/pull/118), mask stack in [PR #113](https://github.com/aliasfoxkde/FireCraft/pull/113) |
| Phase 21 | User-facing modern deliverables landed in the 2026-11-11 train |
| Phase 22 | Partial: core panel/tool/context surfaces landed; remaining inspector and chrome details are tracked |
| Published artifacts | Linux x86_64 GUI/CLI archives, DEB, and web bundle in v0.2.5 |
| Other native formats | Recipes exist, but Windows, macOS, AppImage and Flatpak need target-native build/install/launch receipts before being called supported |

## Deployment and release

The live deployment is **not** the v0.2.5 release artifact and is **not** reproducible from the v0.2.5 tag — measured 2026-11-11 by digest. The live bundle's trunk asset name (`2274ac8c2925b61d`) and wasm digest (`f4f0a3f0…`) match neither the published v0.2.5 release asset (`f9dbe2f14305b2d9` / `a8a4fa4f…`) nor a rebuild of the v0.2.5 tag (`103c62d3366aef17` / `5934702f…`), so the live build corresponds to no identifiable release and its source revision is unknown. The served `index.html` differs from the build's own only by the substituted asset names and the Cloudflare analytics beacon — that difference is explained. Do not infer the live build from the source version alone: every commit since the v0.2.5 stamp reports version `0.2.5`. The measurement is in [T18](../tasks/T18-release.md#deployment-receipt--2026-11-11).

The current release program keeps standalone FireCraft and embeddable developer artifacts as complementary targets. A packaging recipe or cross-compile is not a verified release.

## Where to go next

- [Current task board](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TASKS.md)
- [Detailed roadmap](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md)
- [Progress and receipts](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PROGRESS.md)
- [User-visible CS6 parity matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md)
- [Release artifact matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/release/ARTIFACT-MATRIX.md)
- [Known gaps](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/GAPS.md) (historical audit; use TASKS/PROGRESS for current status)

The full measured state, including limitations and dates, is in the repository documentation. This page is a navigation summary, not a certification claim.
