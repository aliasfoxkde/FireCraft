# Project Status

**Snapshot reviewed:** 2026-11-11\
**Source:** `main` at `b61f198`\
**Latest published release:** [v0.2.5](https://github.com/aliasfoxkde/FireCraft/releases/tag/v0.2.5) (2026-11-03)

FireCraft is a functional Rust graphics editor with a shared document engine and command registry across the desktop GUI, WebAssembly app, CLI, scripting host, and MCP server. The current source is newer than the latest published release; do not assume the hosted app contains every change on `main`.

## Current snapshot

| Surface | Current state |
|---|---|
| Engine commands | 257 registered command IDs |
| Shell reachability | 257/257 = 100% in the 2026-11-16 measurement (wave-2, #150); the command palette exposes the full registry, but this gate measures the CS6-visible shell |
| CS6 DOM matrix | 268 carried / 3 partial / 0 missing of 356 verbs; this is scripting/API parity, not GUI parity |
| Menu census | 2 inert rows of 298; eight other rows are explicitly feature/evidence-gated |
| User-visible parity | Full inventory published in [CS6-PARITY.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md) |
| Phase 20 | 20.1–20.7 all landed; Optimize/timeline in [PR #118](https://github.com/aliasfoxkde/FireCraft/pull/118), mask stack in [PR #113](https://github.com/aliasfoxkde/FireCraft/pull/113) with both [issue #114](https://github.com/aliasfoxkde/FireCraft/issues/114) follow-ups closed 2026-11-19 (mask sampling at the mask's own resolution; two-row panel screenshot) |
| Phase 21 | User-facing modern deliverables landed in the 2026-11-11 train |
| Phase 22 | Partial: core panel/tool/context surfaces landed; remaining inspector and chrome details are tracked |
| Published artifacts | Linux x86_64 GUI/CLI archives, DEB, and web bundle in v0.2.5 |
| Other native formats | Recipes exist, but Windows, macOS, AppImage and Flatpak need target-native build/install/launch receipts before being called supported |

## Deployment and release

The live deployment serves **`firecraft-web-3841839c20c25d86`** — deployment `29225fe2`, a "v0.3.0 experiment" build from the `feat/wasm-cdn-deploy` branch, measured 2026-11-24: it passes a headless-browser proof (painted canvas, same-origin wasm 200, no console/page errors), but it names no release, is not built from `main`, and serves **no `MANIFEST.sha256`** (the Pages SPA fallback answers that path with `index.html`), so its bytes cannot be checked against a checksum list. It overwrote — at the maintainer's direction to leave the parallel workstream alone — a fully verified publish made earlier the same day: deployment `7f9adb41` from `main` @ `6c68c9e3` (bundle `firecraft-web-d25b8751233853ef`, wasm 15 783 714 B `44652f43…`, every manifest file matched served bytes, browser proof PASS), which replaced the earlier unidentified bundle (`2274ac8c2925b61d`, measured 2026-11-11). Both receipts are in [T18](../tasks/T18-release.md). Reconciliation closes when a tagged publish (v0.2.7) is live with its manifest; until then do not infer the live build from the version stamp — `appinfo.json` reports `0.2.5` for every commit since the v0.2.5 stamp.

The current release program keeps standalone FireCraft and embeddable developer artifacts as complementary targets. A packaging recipe or cross-compile is not a verified release.

## Where to go next

- [Current task board](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TASKS.md)
- [Detailed roadmap](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md)
- [Progress and receipts](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PROGRESS.md)
- [User-visible CS6 parity matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md)
- [Release artifact matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/release/ARTIFACT-MATRIX.md)
- [Known gaps](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/GAPS.md) (historical audit; use TASKS/PROGRESS for current status)

The full measured state, including limitations and dates, is in the repository documentation. This page is a navigation summary, not a certification claim.
