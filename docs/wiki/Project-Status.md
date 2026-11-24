# Project Status

**Snapshot reviewed:** 2026-11-24\
**Source:** `main` at `e14aa291`\
**Latest published release:** [v0.2.6](https://github.com/aliasfoxkde/FireCraft/releases/tag/v0.2.6) (2026-11-24)

FireCraft is a functional Rust graphics editor with a shared document engine and command registry across the desktop GUI, WebAssembly app, CLI, scripting host, and MCP server. The current source is newer than the latest published release; do not assume the hosted app contains every change on `main`.

## Current snapshot

| Surface | Current state |
|---|---|
| Engine commands | **257** registered command IDs — all 257 bound (100%), reachability wave merged 2026-11-16 |
| Shell reachability | 257/257 = 100% in the 2026-11-11 measurement; the command palette exposes the full registry, but this gate measures the CS6-visible shell |
| CS6 DOM matrix | 268 carried / 3 partial / 0 missing of 356 verbs; this is scripting/API parity, not GUI parity |
| Menu census | 2 inert rows of 298; eight other rows are explicitly feature/evidence-gated |
| User-visible parity | Full inventory published in [CS6-PARITY.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md) |
| Phase 20 | 20.1 and 20.5–20.7 are on main; Optimize/timeline (#118) and mask stack (#113) merged into main |
| Phase 21 | User-facing modern deliverables landed in the 2026-11-11 train |
| Phase 22 | Partial: core panel/tool/context surfaces landed; remaining inspector and chrome details are tracked |
| Published artifacts | WASM CDN via proxy Worker deployed (CORS-enabled, 95.6 MB WASM from GitHub Releases under Pages 25 MB limit); Linux x86_64 GUI/CLI archives, DEB, and web bundle in v0.2.6 |
| Other native formats | Recipes exist, but Windows, macOS, AppImage and Flatpak need target-native build/install/launch receipts before being called supported |

## Deployment and release

The WASM CDN deployment and live site reconciliation are documented in PR #184 (on branch `feat/wasm-cdn-deploy`). The proxy Worker at `firecraft-wasm-proxy.cyopsys.workers.dev` serves the WASM binary from GitHub Releases v0.2.6 with CORS headers, resolving the cross-origin WASM fetch issue. The live site loads from the CDN proxy instead of direct GitHub Releases fetches. Deployment receipts (bundle hashes, manifest verification, headless smoke) are recorded in PR #184.

The current release program keeps standalone FireCraft and embeddable developer artifacts as complementary targets. A packaging recipe or cross-compile is not a verified release.

## Where to go next

- [Current task board](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TASKS.md)
- [Detailed roadmap](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md)
- [Progress and receipts](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PROGRESS.md)
- [User-visible CS6 parity matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md)
- [Release artifact matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/release/ARTIFACT-MATRIX.md)
- [Known gaps](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/GAPS.md) (historical audit; use TASKS/PROGRESS for current status)

The full measured state, including limitations and dates, is in the repository documentation. This page is a navigation summary, not a certification claim.
