# Project Status

**Snapshot reviewed:** 2026-11-11  
**Source:** `main` at `b61f198`  
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

The 2026-11-11 deployment reconciliation (#104) found that the live site's JavaScript, Wasm and WebOS app metadata match the v0.2.5 release bytes. Cloudflare injects an analytics beacon into the served HTML, which accounts for the remaining HTML-manifest difference. Republish from the canonical release artifact is pending.

The current release program keeps standalone FireCraft and embeddable developer artifacts as complementary targets. A packaging recipe or cross-compile is not a verified release.

## Where to go next

- [Current task board](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TASKS.md)
- [Detailed roadmap](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md)
- [Progress and receipts](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PROGRESS.md)
- [User-visible CS6 parity matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md)
- [Release artifact matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/release/ARTIFACT-MATRIX.md)
- [Known gaps](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/GAPS.md) (historical audit; use TASKS/PROGRESS for current status)

The full measured state, including limitations and dates, is in the repository documentation. This page is a navigation summary, not a certification claim.
