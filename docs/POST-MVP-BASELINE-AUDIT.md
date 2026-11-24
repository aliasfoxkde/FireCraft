# FireCraft — Post-MVP Baseline Audit

> **Document type:** verified baseline audit required by
> [POST-MVP-AGENT-HANDOFF.md](POST-MVP-AGENT-HANDOFF.md) §3.
> **Date:** 2026-11-23.
> **Audited baseline:** `origin/main` = `6c68c9e36983b2aa3b38439502188a76cde52e9e`
> (2026-11-20), plus the docs-only branch `origin/docs/post-mvp-product-roadmap`
> = `645f6699c66f4f5f81cb6328dadbac38d4565ca6` (2026-11-24) which adds
> [POST-MVP-ROADMAP.md](POST-MVP-ROADMAP.md), the handoff,
> [MARKETING-SITE-ROADMAP.md](MARKETING-SITE-ROADMAP.md), and the
> [creative-app benchmark](research/creative-app-benchmark.md), and changes no code.
> **Audit worktree:** `.claude/worktrees/wt-post-mvp-exec`, branch
> `feat/post-mvp-execution`.
> **Companion:** [POST-MVP-EXECUTION-PLAN.md](POST-MVP-EXECUTION-PLAN.md) (tickets,
> sequencing, task-board proposal, verification plan, risk register).
> **Status vocabulary:** the eight handoff statuses, plus `UNVERIFIED` only inside
> evidence text — never as a substitute for a status where evidence exists.

This audit classifies every workstream in the roadmap against source, tests,
generated evidence, branches, PRs, and the live task board. It deliberately does
**not** start feature implementation: the handoff's §2 stop condition is met (see
§1.4), so the correct deliverable is this audit and a merge-safe sequence.

---

## 1. Preflight record

### 1.1 Repository and tooling

| Item | Observed value |
|---|---|
| Root | `/nas/Temp/repos/FireCraft` |
| Remotes | `gitforge` (self-hosted forge), `origin` (github.com/aliasfoxkde/FireCraft) |
| Integration baseline | `origin/main` `6c68c9e3` (2026-11-20) |
| Workspace version | `0.2.5` (`Cargo.toml`) |
| MSRV / pinned toolchain | `1.95` / `1.99.1` (`rust-toolchain.toml`, decision 0102) |
| Crates | 15: app, app-cli, color, compose, doc, engine, filters, fwpng, geom, io, raster, scripting, text, vector, webexport |
| Registered command IDs | 260 (`rg` over `crates/engine/src/catalog/*.rs`); `engine::catalog::all_ids()` is the authority |
| `#[test]` functions | 1,811 across `crates/` |
| Corpus fixtures | 32 committed entries under `corpus/fixtures` |
| Decision records | 130 under `docs/decisions/` |
| Task tickets | 75 at baseline; 8 PM tickets added by this change under `docs/tasks/` |
| Worktrees | 109 (`git worktree list`) |
| Local branches | 158 (`git branch`), plus remote branches |
| Open GitHub issues | 49 |
| Open PRs | 3 (#182, #181, #178) |

The documented counts (253 commands, 296 menu rows, 257/257 reachability, 81.1 %
coverage, 21.6 MB / 95.6 MB WASM) are **dated snapshots**. They are recorded in
§7 as unverified at this SHA; none is used below as proof that a workflow works.

### 1.2 Working-tree state (main checkout)

The main checkout is on `feat/wasm-cdn-deploy` (PR #181's head) with **9 modified
files and 10 untracked paths**, 1,296 insertions / 82 deletions in the tracked set:

- `Cargo.lock`, `crates/app/Cargo.toml`
- `crates/app/src/main.rs`, `editor.rs`
- `crates/app/src/shell/{mod,panels,constants,colors,dialogs}.rs`
- `crates/doc/src/highlevel.rs`
- Untracked: `crates/app/src/shell/welcome_overlay.rs`, `crates/app/src/logo.png`,
  `logo.png`, `docs/validation/`, `node_modules/`, `package.json`,
  `package-lock.json`, `r2-cors.json`, `test-results/`

This is unattributed local work. Per handoff §2, it is preserved, not stashed,
reset, cleaned, or committed by this audit.

### 1.3 Open integration work

| PR | Branch | State | Size | Note |
|---|---|---|---|---|
| #182 | `docs/post-mvp-product-roadmap` | Pushed at `bbf3a782`; docs-only CI | docs only, plus this audit, the execution plan, and PM-01…PM-08 | PR head `645f6699` adds the roadmap, handoff, [MARKETING-SITE-ROADMAP.md](MARKETING-SITE-ROADMAP.md), [creative-app benchmark](research/creative-app-benchmark.md), and SITE-01. Four gate false positives (two view-navigation words, one analytics phrase, one spelled-out header name) are reworded or adjudicated; `aegis` and the link check pass locally. |
| #181 | `feat/wasm-cdn-deploy` | `DIRTY` | 233 files, +23,275 / −1,668 | Carries the WASM CDN deploy, god-file reduction, production-readiness work, and the uncommitted local UI changes above are on its checkout. |
| #178 | `feature/issue-93-behavior-export` | `DIRTY` | — | Webexport behavior-target fix; needs rebase and a re-run. |

### 1.4 Gate 0 verdict

**BLOCKED for feature implementation.** The handoff §2 stop condition ("current work
is uncommitted or integration is unstable") is satisfied twice over: an unattributed
1,296-line local diff plus three dirty/unstable PRs. No shared implementation file
may be edited and no competing feature branch may start until Gate 0 closes
([POST-MVP-EXECUTION-PLAN.md](POST-MVP-EXECUTION-PLAN.md) §2).

This audit and the ticket set are merge-safe because they add only new documentation
and do not modify code, generated evidence, `TASKS.md`, or `NEXT.md`.

---

## 2. Local-work reconciliation inventory

| Item | Location | Intended objective (inferred) | Evidence of tests | Conflicts / risk | Recommended action |
|---|---|---|---|---|---|
| Shell UI + branding group | `crates/app/src/shell/*`, `editor.rs`, `main.rs`, `welcome_overlay.rs`, `logo.png` | Welcome overlay, logo/branding, panel behavior | None run in this audit | Shared shell files (`panels.rs` +740, `mod.rs`) conflict with PR #181's god-file work | Commit as one focused PR off #181's branch after `cargo test -p app` + `cargo fmt`; do not mix with feature work |
| `crates/doc/src/highlevel.rs` | untracked | High-level document helper | Unknown | Touches `doc`, which PM-01/PM-03 also touch | Review with the shell PR or its own; needs `cargo test -p doc` |
| CraftAnything validation screenshots | `docs/validation/craft-anything-checks/` | Screenshot evidence rounds | Images only | Large binary set; not referenced by a doc | Either add an index/consumer doc or keep out of git; do not commit blind |
| Playwright tooling | `package.json`, `package-lock.json`, `test-results/`, `node_modules/` | Browser verification harness | None | **Not gitignored** — `node_modules/` and `test-results/` would be committed by `git add -A` | Add both to `.gitignore`; commit only `package.json`/lock if the browser harness is adopted repo-wide |
| `r2-cors.json` | repo root | R2 CORS configuration for the CDN | — | Deployment config may encode bucket IDs or rules | Inspect for secrets; keep as deployment documentation only if non-sensitive and referenced |
| PR #181 | `feat/wasm-cdn-deploy` | WASM CDN proxy, shell reduction, readiness | CI `DIRTY` | 233 files; conflicts with local uncommitted UI work | Rebase after the local UI change is committed or extracted; land before Wave 1 |
| PR #178 | behavior export fix | Slice-target resolution | — | DIRTY | Rebase and re-run focused `webexport` tests |
| 109 worktrees / 158 branches | `.claude/worktrees/`, `/var/tmp/` | Historical agent work | — | Drowning the repo; stale branches obscure active work | Prune per Phase A-3; keep only worktrees with unmerged, attributable work |

**Ordering rule:** local uncommitted work → PR #181 → PR #178 → PR #182 → Wave 1.
PR #182 is docs-only and can land first if its `docs-only-check` failure is an
Aegis baseline finding rather than a conflict marker.

---

## 3. Workstream matrix

Legend — **Status:** `VI` verified implemented · `UX` implemented, UX incomplete ·
`PA` partial · `LM` local / not merged · `IP` in progress · `MI` missing ·
`BL` blocked · `DF` deferred.
**Surfaces:** D desktop · W web/WASM · C CLI · S scripting · M MCP. `•` = available
through the shared command registry; `—` = not exposed on that surface.

### 3.1 Reliability (§5.1)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| REL-01 | `PA` | Autosave + desktop journal sidecar + web localStorage restore (`shell/mod.rs::check_recovery`, `recovery_ui`; CS6-PARITY §E). No `tempfile`/`rename`/`atomic` in save paths. | No transactional-write proof, single recovery generation, no timestamps chooser, no external-modification check, no fault injection | D W C — M• | **PM-01** |
| REL-02 | `PA` | `xtask/src/perf.rs` memory probe; tiled COW store; `compose/src/cache.rs`; 16 MiPix render budget. No benchmark table at this SHA. | No versioned corpus, no published startup/open/edit/save/RSS table, no cache accounting, budgets not rising-only | C (authoritative) | **PM-02** |
| REL-03 | `VI` | `forbid(unsafe)`, no `unwrap`/`panic` lints, typed errors, allocation caps (`raster::TileStore`, `fwpng` caps), `fuzz/` anchors + `xtask fuzz`, 1,811 tests | Continuous duty: every new parser/plugin/IPC surface needs an anchor | all | Keep as an invariant, not a one-off ticket |

### 3.2 Interface, Help, keyboard (§5.2)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| UX-01 | `MI` | Help menu opens hosted wiki; `help_panel` is a read-only binding table; no help topics or search (`rg -i 'help center'` → 0) | No offline Help Center, no task-language search, no contextual help, no coverage check | D W | **PM-04** |
| UX-02 | `PA` | Ctrl+P command palette over `all_ids()` with fuzzy scoring/recents (CS6-PARITY §G/§J) | Commands only; no tools/panels/help/fonts/styles/assets; no aliases, explainable ranking, guided parameters, capability states | D W — — M• | **PM-05** |
| UX-03 | `PA` | `keymap.rs` has `KeymapSet {Cs6, Illustrator, Photoshop, FreeHand, WebStandard}` overrides; 167 bindings; CS6 keymap gates | No rebinding UI, conflict detection/precedence, profile import/export, reset, or generated tables from the authority | D W — — — | **PM-05** |
| UX-04 | `PA` | Panel docking, tab groups, layout persistence, 3 screen modes, HiDPI, context menus, multi-doc tabs (Phase 21/22) | Responsive audit not started; empty/loading/error state audit incomplete; workspace presets absent | D W | Gate 0 owns the in-flight polish; new work after |

### 3.3 Color, print, precision (§5.3)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| COLOR-01 | `MI` | No ICC/profile dependency or type in `color`/`io`; 27 blend modes in `color/blend.rs`; PSD CMYK decode only | No doc/working/display/export spaces, no assignment vs conversion, no soft proof, no gamut warnings, no mismatch warnings | — | ADR + research first (Wave 2) |
| COLOR-02 | `PA` | 8-bit premultiplied RGBA; deterministic CPU compositor; CPU/GPU parity harness patterns exist | No 16-bit path; gradient/blend interpolation spec undocumented; profile-aware precision absent | all | Fold into COLOR-01 ADR |
| COLOR-03 | `MI` | `File ▸ Print`/Page Setup report a named gap; no PDF export | No CMYK output, output intent, bleed/trim/marks, spot handling, print preview | — | Deferred behind COLOR-01 |

### 3.4 Typography, brushes, resources (§5.4)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| TYPE-01 | `PA` | `crates/text` (cosmic-text 0.19 + bundled Liberation faces), text objects, rich runs, text-on-path, in-canvas editor | No font discovery/manager, favorites, variable axes, OpenType UI, glyph browser, paragraph styles, missing-font report, RTL/complex-script fixtures, embedding | D W C• S• M• | Wave 2 |
| RES-01 | `PA` | `engine/src/brush.rs` deterministic stamp (shape/size/edge/aspect/angle), paint family, Edit Brush slots, pressure multiplier | No preset library/categories/previews/import-export, texture/scatter, stabilization, vector brushes, device capability reporting | D W | Wave 2 |
| RES-02 | `PA` | Styles panel, Swatches panel, style library import (`.fw.stl`), effect rows | No unified resource browser, usage/dependency graph, linked resources, theme/semantic palettes, safe delete/replace | D W | Wave 2 |
| RES-03 | `MI` | No asset browser or dependency graph found | No linked/embedded asset inventory, relink, duplicate-by-hash, unused review, project packaging | — | Wave 2 |

### 3.5 Pages, artboards, components, design systems (§5.5)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| DOC-01 | `PA` | Fireworks pages/states/layers/symbols round-trip in `doc` + `fwmap`; page model in `doc/model.rs` | No artboard semantics, presets, per-artboard export, artboard navigator; no page/artboard/state/component ADR | D W C• S• M• | ADR before any serialization change |
| COMP-01 | `PA` | Symbols/instances, graphic/button/animation types, nested editing, 9-slice guides, symbol library | No variants, exposed properties, component states, overrides surviving definition updates, detach/reset/relink, cross-document libraries | D W C• S• M• | Behind DOC-01 |
| DESIGN-01 | `MI` | No token/variable model (`rg 'design token'` → 0) | No typed tokens, modes/themes, usage graph, DTCG import/export, CSS custom properties | — | Wave 4 |
| DESIGN-02 | `MI` | No constraints/auto-layout/breakpoints | No responsive reflow, stacks, grid tracks, viewport preview | — | Wave 4 |

### 3.6 Vector and raster depth (§5.6)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| EDIT-01 | `PA` | Paths, primitives, boolean ops, simplify, offset/inset, knife, freeform/reshape, autoshapes, stroke joins | No variable-width strokes, live corners, mesh depth, path-aware layout beyond text-on-path, SVG-native inspection | D W C• S• M• | Use parity evidence; Wave 2 |
| EDIT-02 | `PA` | Bitmap objects, marquee/lasso/wand with modes, masks, live filters, retouch, blend modes | No feather/grow/shrink/smooth/color-range/save-restore audit, clone/healing, adjustment layers, channel inspection, high-bit depth | D W C• S• M• | Use parity evidence; Wave 2 |

### 3.7 Import, export, interoperability (§5.7)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| IO-01 | `MI` | Format list hardcoded in `dialog.rs` (`EXPORT_FORMATS`), CLI match arms, FEATURES prose | No machine-readable registry, feature mapping, editability classes, fixture coverage, dialog/Help/CLI exposure | D W C | **PM-03** |
| IO-02 | `PA` | Import PNG/JPEG/GIF/BMP/TIFF/WebP/WBMP; export + SVG/PSD/web workflows; PSD fidelity decision 0006 | No AVIF/JXL/PDF/OpenRaster/APNG; no EPS/AI path; animated WebP/AVIF absent | D W C• | Registry first, then demand-driven |
| IO-03 | `PA` | Native node-tree round-trip; unknown-node preservation; corpus floors; 32 fixtures | No per-format compatibility matrix, conversion report, real-world fixture set, semantic diff | C | **PM-03** |

### 3.8 Export and asset production (§5.8)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| EXPORT-01 | `PA` | Export dialog workflows (images/HTML/CSS/sprites/pages/states/layers/animated GIF), `EXPv` presets, `webexport` batch | No multi-scale presets, named recipes, manifests/hashes, cancellation/retry, watch mode, content-hash reuse | D W C• S• M• | Wave 3; depends on PM-03 |
| ASSET-01 | `MI` | No icon/ICO/ICNS/adaptive-icon generator | Whole studio | — | Wave 3 |
| ASSET-02 | `MI` | `webexport` sprite output exists; no atlas packer (`rg atlas` → 0) | Packing, trimming, metadata schemas, overflow/split, preview | C• | Wave 3 |
| BATCH-01 | `PA` | `app-cli batch`, `webexport/src/batch.rs` | No recipes, dry-run, collision policy, per-file JSON summary, resumability, GUI queue | C• | Wave 3 |
| SVG-01 | `MI` | SVG import/export only (`io/src/svg.rs`, `svgimport.rs`) | No source editing/hierarchy inspection, diagnostics with locations, sanitization report, cross-engine preview | — | Wave 3 |
| ANIM-01 | `PA` | States strip, per-state timing, onion skin, playback clock (decisions 0094/0096), `symbol.tween` | No keyframe tracks/easing/motion paths, loop count, thumbnails, APNG/animated WebP export, frame-sequence manifest | D W C• S• | Wave 4 |

### 3.9 Prototyping, handoff, accessibility (§5.10–5.12)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| PROTO-01 | `MI` | `rg prototype` → 0 | Whole interaction model, overlays, variables, transitions, export package | — | Wave 4 |
| DEV-01 | `MI` | `rg handoff` only in `dialogs.rs` | No inspect mode, measurements, CSS/token readouts, JSON spec, asset refs | — | Wave 5 |
| DEV-02 | `MI` | No design-to-code representation | Framework-neutral spec + adapters | — | Wave 5 |
| A11Y-01 | `MI` | No inspector; `rg -i 'a11y' crates` → 0 | Contrast/touch/focus/alt-text reports, suppression, export | — | **PM-06** |
| A11Y-02 | `PA` | accesskit enabled in `app`; keyboard-operable shell; decision 0067 records web tree unreachable | No object navigator; no desktop AT receipts; no criterion-by-criterion report | D (partial), W (documented limit) | **PM-06** |

### 3.10 Automation and plugins (§5.13–5.14)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| AUTO-01 | `PA` | One registry drives GUI/CLI/journal/ExtendScript/browser bridge; `deny_unknown_fields` payloads; session commands | No versioned IDs, capability discovery, structured error codes, progress/cancel, transaction boundaries, read/mutation scopes, deprecation policy, audit trail | all | Wave 5 ADR |
| AUTO-02 | `PA` | MCP server (`app-cli/src/mcp.rs`): `firecraft.apply`, `runScript`, `info`, `export`, `undo`, `redo`, `listCommands`, one resource | Write-capable by default; no scopes/roots/confirmation/preview/transactions/audit; not provider-independent beyond JSON-RPC | M | Wave 5, security-first |
| AUTO-03 | `PA` | CLI verbs: convert, export, export-web, import, batch, replay, script, mcp; help/version tests | No inspect/validate/optimize/package/report verbs, JSON mode contract, dry-run/progress, pipeline files, generated schema docs | C | Wave 5 |
| PLUG-01 | `MI` | No plugin manifest/API/sandbox | Whole SDK | — | Wave 5, deferred |
| PLUG-02 | `MI` | No catalog/distribution | Whole catalog | — | Wave 6, deferred |

### 3.11 Community, collaboration, AI (§5.15–5.17)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| COMMUNITY-01–06 | `MI` | GitHub used manually; no in-app client, templates, diagnostics, duplicate search, or status surface | Whole client and privacy design | — | Wave 6 after auth/privacy ADR |
| COMMUNITY-07 | `PA` | Apache-2.0, `NOTICE`, `ATTRIBUTION.md`, clean-room policy, decisions, SBOM/provenance code, release checklist | Contribution/code-of-conduct/security templates not audited; reproducible builds/signing not certified | — | Continuous |
| COLLAB-01 | `MI` | Session undo/rewind only; no named checkpoints/visual diff | Whole version history | — | Wave 6 |
| COLLAB-02 | `MI` | No identity/conflict model | Whole collaboration | — | Deferred (P3) |
| AI-01 | `MI` | No AI adapter | Whole feature | — | Deferred (P3), consent-gated |
| AI-02 | `MI` | MCP exists but no agent-scope/preview/audit contract | Whole agent contract | M | Wave 5/6 |

### 3.12 Platform and documentation (§5.18–5.19)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| PLATFORM-01 | `PA` | Linux tar/deb/CLI + web bundle built; CDN Worker deploy documented; Windows/macOS/AppImage/Flatpak recipes only; **nothing CERTIFIED**; scope unapproved (decision 0075) | No receipts for off-fleet targets; readiness table mixes recipes and artifacts | all | **PM-07** |
| PLATFORM-02 | `PA` | `rfd` dialogs, `arboard` clipboard, HiDPI, tabs; platform traits in `engine/platform.rs` | Pen/tablet, display profiles, multi-monitor, file associations, update/uninstall, WebOS host-version matrix unverified | D W | Wave 5 |
| PLATFORM-03 | `PA` | SBOM/provenance/manifest implementations, checksums, release workflow; signing absent | No published SBOM/manifest; no signatures/attestation; no SDK | all | Wave 5 |
| DOCS-01 | `PA` | USAGE/FEATURES/CS6-PARITY/PROGRESS/wiki/decisions/130 ADRs | No localization readiness, no doc link check, screenshots not build-stamped, feature docs not enforced same-change | — | Continuous; PM-04 consumes it |

### 3.13 Public marketing site (§5.20)

| ID | Status | Evidence | Remaining behavior | Surfaces | Next action |
|---|---|---|---|---|---|
| SITE-01 | `PA` | `site/` is now a seven-page static site — Home, Features, Gallery, Formats, Downloads, Docs, Project — plus `404.html`, `_headers`, `robots.txt`, and a single stylesheet/script; the mark ships as `firecraft-mark.svg` derived from the approved vector path; real captures are copied into `site/assets/` with provenance in `site/assets/README.md`; `app.js` allowlists `github.com` and the GitHub asset hosts over HTTPS and falls back on API failure; `.github/workflows/deploy-site.yml` validates every page and rejects third-party font/analytics hosts | **Not deployed:** `firecraft-site.pages.dev` still does not resolve from this host and no deployment receipt exists, so no page calls the site live. No sitemap (production hostname unconfirmed). A headless Chrome pass at 500 and 1440 px confirmed layout, the mobile-nav toggle, asset loading, and absence of page-level overflow, but this host cannot produce screenshots, so there is no pixel, axe, or Lighthouse baseline. Claim review is per-page copy, not yet a signed-off matrix. The brand PNGs at `docs/img/logo.png`, the branding master, the hicolor icon, and `web/favicon.png` were invalid 15,000-byte non-PNG blobs and are now 512/256/64 px rasterizations of the canonical vector path | W | **PM-08** (Phase 0 repo work done; deploy owner-gated) |

### 3.14 Status totals

| Status | Count |
|---|---|
| Verified implemented | 1 (REL-03) |
| Implemented, UX incomplete | 0 |
| Partial | 28 |
| Local / not merged | 0 as a workstream verdict; the Gate 0 diff and PR #181 are tracked in §2 |
| In progress | 0 attributable (the uncommitted local work has no declared workstream) |
| Missing | 26 |
| Blocked | 0 as a workstream verdict; 5 depend on a decision (DOC-01, COLOR-01/03, AUTO-02, PLUG-01) |
| Deferred | 0 formally; COLLAB-02, AI-01/02 and PLUG-02 are recommended P3 deferrals |

The roadmap contains 55 workstream IDs: 54 labelled plus the unlabelled §5.19
documentation stream, labelled `DOCS-01` here for traceability.

---

## 4. Cross-cutting findings

1. **Documentation drift exists and is measurable.** `FEATURES.md` says 253
   commands; the catalog now yields 260. `CS6-PARITY.md` §H says the four
   alternative keymap sets are missing; `keymap.rs` implements `Illustrator`,
   `Photoshop`, `FreeHand`, and `WebStandard`. `HANDOFF.md` §1 is dated 2026-11-14
   against `main ad3fc655`, while `main` is `6c68c9e3` (2026-11-20). Reconcile in
   Gate 0; do not rewrite history.
2. **`node_modules/` and `test-results/` are untracked and not ignored.** A
   `git add -A` on the main checkout would commit a dependency tree. Add both to
   `.gitignore` before any commit from that tree.
3. **No atomic-save implementation was found.** `rg 'tempfile|persist\(|rename\(|atomic'`
   over `app`, `io`, and `fwpng` returns nothing. The "autosave/recovery" claim is
   real; the "never truncate the last good file" claim is not demonstrated.
4. **There is no color-management substrate.** No ICC/lcms/moxcms dependency or
   profile type exists; CMYK appears only in PSD decode. Any print or pro-color
   claim is unsupported.
5. **MCP is write-capable by default.** The server exposes `apply`, `runScript`,
   and `export` without read-only default, scoped roots, confirmation, preview,
   transactions, or audit. This is the single largest security gap in the roadmap.
6. **Format breadth is narrower than the roadmap's target list.** No AVIF, JXL,
   PDF, OpenRaster, APNG, EPS/AI. The `io` crate also has no JPEG XL/AVIF
   dependencies. Do not advertise them.
7. **The web accessibility limitation is recorded, not hidden** (decision 0067).
   The audit treats it as a documented limitation, not an implementation gap, until
   an end-to-end browser assistive-technology test passes.
8. **The open-issue count in the docs (71) does not match GitHub (49).** Triage
   state has moved; re-derive before planning community work.
9. **The corpus is native-format heavy (32 fixtures).** Format fidelity beyond
   `.fw.png` has no representative corpus, which is why IO-03 is a Wave 1 ticket.
10. **Performance and size numbers in circulation conflict.** PRODUCTION-TODO
    records 95.6 MB raw WASM; CS6-PARITY recorded 21.27 MB in November. These are
    different builds/features; neither is a current measurement. PM-02 exists to
    replace both with one measurement basis.
11. **Release scope is unapproved.** Decision 0075 is still *recommended — awaiting
    maintainer approval*; nothing is `CERTIFIED`; no signing identities exist.
    PM-07 may inventory but not change claims.
12. **The marketing site is rebuilt and deploy-ready, but not deployed.** Seven
    pages plus a custom 404, `robots.txt`, and response headers; no third-party
    font, analytics, or image host; a release loader that validates download
    origins; and a deploy workflow that validates the whole page set. The deploy
    path's missing-logo defect is fixed. `firecraft-site.pages.dev` still does not
    resolve and no receipt exists, so SITE-01 stays `PA` and nothing here calls it
    live.

---

## 5. What already exists (do not rebuild)

- Native `.fw.png` read/write with unknown-node preservation; tile rasters; corpus
  floors; `fwpng` standalone and wasm-clean.
- One command registry (260 IDs) driven by GUI, CLI, journal replay, ExtendScript
  host, browser bridge, and MCP.
- Vector editing (paths, booleans, simplify, offset, autoshapes, knife, freeform),
  bitmap painting/retouching, pixel selections, masks, live effects, 27 blend modes.
- Pages/states/layers, symbols/instances with 9-slice, slices/hotspots/behaviors,
  web export workflows, animated GIF.
- Panel docking/tab groups, palette v2, multi-document tabs, history rewind,
  autosave/recovery, preferences, HiDPI, 3 screen modes, CS6 keymap gates.
- 1,811 tests, fuzz anchors, rising-only parity floors, reachability gate, Aegis
  secret/PII scan, coverage gate, signed-artifact scripts.

---

## 6. Decisions required before implementation

| Decision | Blocks | Owner |
|---|---|---|
| Atomic save / recovery semantics | PM-01 | maintainer + agent |
| Benchmark corpus and budget policy | PM-02 | agent |
| Format capability & fidelity contract | PM-03, EXPORT-01, DEV-01 | maintainer + agent |
| Command metadata & keybinding profile schema | PM-05 | agent |
| Accessibility model & conformance reporting | PM-06 | agent |
| Release scope approval (revisit 0075) | PM-07, certification | maintainer |
| Page / artboard / state / component semantics | DOC-01, COMP-01, DESIGN, PROTO | maintainer + agent |
| Color-management model and dependency | COLOR-01/02/03 | maintainer + agent |
| Plugin runtime and permissions | PLUG-01/02 | maintainer |
| MCP write scopes and transaction model | AUTO-02, AI-02 | maintainer + agent |
| GitHub auth/privacy for in-app reporting | COMMUNITY-01–06 | maintainer |
| Collaboration merge model | COLLAB-01/02 | maintainer |

---

## 7. Unverified at this SHA (must be measured, not assumed)

- `257/257` shell reachability, the menu census (2 inert of 296), and the CS6 DOM
  parity counts. Regenerate with `xtask reachability`, `xtask parity`,
  `xtask scorecard`.
- Coverage (81.1 % / 78.7 %) and the coverage floor.
- WASM raw/gzip size, the budgets, and which toolchain builds wasm32 today.
- Live deployment provenance and `version.json` contents.
- The `firecraft-site` Pages project, its deployment mode/branch/domain, and
  whether any production deployment exists (the default hostname does not
  resolve from this host).
- Corpus round-trip percentages and golden-image floors.
- PSD fidelity beyond decision 0006's recorded limits.
- Desktop screen-reader behavior; browser file-system access/OPFS quota behavior.
- Every performance claim; no benchmark table exists at this SHA.

None of these is used as a completion argument in §3; where a claim mattered, the
audit cites code, tests, or a decision instead.
