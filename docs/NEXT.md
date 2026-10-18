# Next-agent task list (2026-10-18)

Start at `docs/HANDOFF.md` first. This file is the live backlog;
HANDOFF §4 has the full phase descriptions. Entries are roughly in
suggested order (quick wins first, blockers before dependents).

---

## Phase 11 — Image support enhancements

- **#121** Audit + align CLI import format list with actual decoder support
  (`io::raster::decode` sniffs bmp/tiff/gif; CLI error strings may lag;
  check web docs too). Quick win.

- **#123** Decision doc: lossy WebP — decide first (libwebp-sys C dep vs
  wasm-breaker vs feature-gate native-only). `image` crate's VP8L
  (lossless) landed in decision 0063; lossy needs a design decision
  before any code.

- **#124** Decision doc: AVIF decode — investigate `image` avif-native
  feature state; no pure-Rust production decoder exists; honest door is
  OS decoders (desktop) or wasm-friendly fallback. Import-only acceptable;
  export out of scope until encoder exists.

- **#125** Decision doc: HEIC refusal — record refusal and the desktop-only
  import path if any. No maintainable open encoder/decoder without
  patents-or-C problems.

## Phase 13 — Release engineering hygiene

- **#122** WebOS packaging dry-run: real `appinfo.json` + `host.html` in
  `dist/web` staging. Operator-supplied icons (ATTRIBUTION rows in same
  change). The sketch in WEBOS.md is load-bearing for the smoke test —
  the staging artifact should be the proven one.

- **#131** Consolidate release notes into `docs/releases/vX.Y.Z.md` (tracked,
  not gitignored like `dist/release-notes.txt`). Keep the SHA block
  generated.

- **#134** Platform matrix: test + document the AppImage / macOS dmg /
  Windows recipes in `packaging/`. Each run on real hardware produces
  its artifact; release notes stay honest ("built and verified on Linux
  x86_64") until then.

- **#130** Decision doc: smoke budget — wasm load time vs the hardcoded
  `--virtual-time-budget=60000`. If wasm grows past the boot budget on
  slow machines, the smoke fails spuriously.

## Phase 12 — Evidence-gated holds (parked, not lost)

- **#132** `setFillNColorNTexture`: needs an observed TXFv shape. Oracle
  setup: `source/FireworksPortable/` exists for CS6 under Wine. Without
  new evidence the row stays P.

- **#133** Out-of-bounds effect growth: needs a CS6 filter/flatten bounds
  oracle. Same Wine corpus setup. Without new evidence the settled
  posture (bake clips to image surface) stands.

## Ad-hoc / recurring (run before any web/shell change)

- **#126** Browser smoke against live URL — **mandatory before pushing any
  `web/` or `shell.rs` change** (the two Phase 10 defects were
  invisible to every desktop gate). Pattern: playwright headless against
  https://firecraft-web.pages.dev, check served hash first
  (`MANIFEST.sha256`), then ping → ready, openDocument (ArrayBuffer)
  → renders with zero input events, no pageerror. See HANDOFF §6.2.

- **#127** Cloudflare Pages redeploy — after any verified web/shell change:
  `wrangler pages deploy dist/web --project-name firecraft-web`. Verify
  served hash matches `dist/web` before diagnosing production issues.

## Release engineering

- **#128** Cut v0.2.2 — when the next change batch is ready. 8 stamps
  (workspace + 6 member Cargo.toml + xtask compose dep pin +
  `packaging/deb/control`), `xtask changelog`, `dist --native` +
  `dist --web`, zip web at root, SHA block in notes, tag, push gitforge
  then origin, `gh release create` with 5 assets, Pages redeploy. See
  HANDOFF §6.3.

## Phase 11-adjacent

- **#129** Clarify `exportDocument` save path — WEBOS.md shows it
  replying with bytes for the host to save; the in-app File > Save
  uses a different (blob-download) code path. If host-driven direct
  download is desired, needs a decision doc first.

---

## Completed (do not reopen without new evidence)

- Phase 10 (web bridge: as_any_mut + repaint + honest smoke) — landed
  `67532fa`, verified in production, v0.2.1 released.
- DOM verb-surface parity campaign — 268 C / 3 P / 0 M / 85 N,
  registry 241, all Phase 9 debts closed (#126–#133).
- `setAnimInstanceStartFrame` and `setSliceAutonaming` — permanent
  refusals (no wire nodes exist).
