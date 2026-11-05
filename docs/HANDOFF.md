# FireCraft — Agent Handoff

> **Start here.** This is the entry point for any agent (or human)
> picking up FireCraft work. It answers: where things stand, where
> everything lives, how to work here, and exactly what to do next.
> Day-to-day rules: [`../AGENTS.md`](../AGENTS.md). Master plan:
> [`PLANNING.md`](PLANNING.md). Live status: [`PROGRESS.md`](PROGRESS.md).
> Session log: [`SESSIONS.md`](SESSIONS.md). Project arc: [`HISTORY.md`](HISTORY.md).

---

## 1. State snapshot (2026-11-01)

- **Ship state:** v0.2.4 released (2026-11-02, `6c256bf` — stamps +
  changelog + notes); repo public. Desktop
  + CLI + web bundle + deb all cut from `xtask dist`. The web bundle
  is live at <https://firecraft-web.pages.dev> (Cloudflare Pages,
  direct upload — deploy door in §6).
- **Engine registry:** 245 commands (`engine::catalog::ids()` — the
  live authority; point-in-time counts in the docs trail it), pinned
  equal to the fuzz call table by `the_call_table_covers_the_catalog`.
- **CS6 DOM verb-surface parity:** **268 C / 3 P / 0 M / 85 N** — the
  M table has been empty since 2026-10-14. The 3 P rows are terminal
  at today's evidence: `setAnimInstanceStartFrame` and
  `setSliceAutonaming` are permanent (no wire node exists — refusing
  is the faithful behavior), and `setFillNColorNTexture` is
  evidence-gated (no observed `TXFv` shape, no mintable texture
  rasters). Do not reopen without new evidence.
  (`docs/planning/generated/dom-parity.md`, regenerate per §5.)
- **Where the frontier is:** the *engine* is complete; the *shell* is
  not. A judge-panel audit (2026-10-31) found the engine's 243
  commands reach only a fraction of the interface — **8 of 289 CS6
  menu items still render `Action::Inert`** (derived —
  `cargo test -p app inert_worklist -- --nocapture` prints them, and
  `inert_count_never_rises` prints the count —
  and gated by `menus::tests::inert_count_never_rises`) — so the active program is
  **reachability and canvas competence**, i.e. `docs/NEXT.md`
  Phases 19–21 (19.1–19.5 landed: transform handles, path-node
  editing, gradient editor, command palette, grid/guides/snap; 19.6
  inert-menu burn-down in flight). The quality ratchet (Phases
  14–18) is the parallel backlog. WebOS packaging (`docs/WEBOS.md`)
  is the standing primary-target goal.
- **Clean-room invariants (never violate):** `source/` is Adobe's
  copyrighted material — git-ignored, never committed, never copied
  from. Adobe-derived text extracts are uncommitted working notes
  under `/var/tmp/`. Adobe-authored corpus files are test oracles
  only. The `AMT/` tree is out of scope entirely. Assets need a
  `docs/ATTRIBUTION.md` row in the same change.

## 2. Where everything lives

| Path | What |
|---|---|
| `AGENTS.md` | Operating manual: commands, gates, commit rules, day-to-day method |
| `docs/PLANNING.md` | Master plan: format spec (§6), architecture (§7), method |
| `docs/planning/firecraft/` | `ROADMAP.md` + tickets M0–M15 (all closed; reference for how tickets are written) |
| `docs/decisions/` | Decision records 0001–0067 — **write one per family/batch** |
| `docs/planning/generated/` | `parity.md`, `scorecard.md` (xtask-owned) + `dom-parity.md` (generator-owned, see §5) |
| `docs/research/fireworks-png-nodes.md` | The node dictionary — the format Rosetta stone (observational; corpus wins on conflict) |
| `docs/WEBOS.md` | The web embedding contract + the smoke host page (load-bearing — see the warning there) |
| `log/devlog.md` | Detailed session log (gitignored) — **append an entry to close every task** |
| `crates/engine/src/catalog/` | The command registry, one module per family |
| `crates/engine/src/fuzzdrive.rs` | Random driver: `CALLS` table + payload builders (pinned to the catalog) |
| `crates/app/src/shell/` | The egui shell (desktop + wasm), split into modules (`mod`, `glyphs`, `filters_ui`, `dialogs`, `panels`, `canvas_ui`); `eframe::App` impl in `mod.rs` — carries the wasm-only `as_any_mut` and the stashed repaint context |
| `web/src/main.rs` | The wasm entry: trunk bootstrap, hidden file input (browser Open), postMessage bridge, downloads (browser Save) |
| `corpus/fixtures/` | Committed Fireworks fixtures (round-trip/golden oracles) |

## 3. How to work here (the family loop)

1. **Pick the next phase** from §4. For parity rows, read the CS6 API
   text for its verbs (regeneration recipe: `PLANNING.md` §5.4; the
   extract lives at `/var/tmp/fw_ext_full.txt`).
2. **Write the decision doc first** (`docs/decisions/00NN-<topic>.md`):
   wire/model design, CS6 API mapping, deviations, consequences,
   recorded debts. Look for existing types before inventing new ones.
3. **Implement model → reader/writer → delta → commands → tests** in
   that order; run per-crate gates as you go (`cargo test -p <crate>`,
   clippy `-p`). Hooks deny workspace-wide cargo in ad-hoc shells.
4. **Grow the fuzzer in the same change**: append new command names to
   `CALLS` in `fuzzdrive.rs` + a payload builder arm each —
   `the_call_table_covers_the_catalog` fails otherwise.
5. **Update the parity matrix** (when rows move): flip the family's
   rows in `/var/tmp/domp/gen.py` (edit via a `python3` heredoc — the
   Edit tool is cwd-bounded and hooks deny `| python3`), regenerate,
   then `cargo run -p xtask -- parity scorecard floor layers`.
6. **Gate, commit, push, verify CI** (runbook §6), append the devlog
   entry, update `PROGRESS.md`/`SESSIONS.md`. For shell/web work,
   **prove the browser path** before pushing: §6.2's playwright
   pattern, against the served `dist/web`, end to end.

**Standing engineering lessons (each cost a real failure once):**

- **Capture before mutating.** A delta holds *prior* state; capturing
  after the mutation silently undo-writes the post-state. Every
  `before = …clone()` line goes above the first mutation of that
  slice.
- **Never trust an RNG stream across catalog growth.** Fuzz-derived
  tests that *force* a call class must draw-until-landed **and** seed
  deterministic state first — the generator deliberately over-draws
  indices and injects junk fields for refusal coverage.
- **Fixture-first, corpus-second** (quality rules): validate each
  layer separately — wire round-trip, then model equality, then
  composition. An end-to-end pass alone hides which layer regressed.
- **No corpus oracle → the dictionary stands** and a synthetic
  write→read→byte-equal round-trip is the gate; record the missing
  oracle in the decision doc.
- **A verdict check must not be able to pass on its own inputs** (the
  vacuous-smoke lesson): the bridge smoke matched its "green" string
  inside the host page's own `<script>` source and was green for
  months while the bridge panicked at boot. Strip `<script>` bodies
  before any DOM-string assertion (`strip_script_bodies` in
  `xtask/src/dist.rs`), and require the failure marker's absence too.
- **eframe wasm shells: `as_any_mut` + repaint.** `WebRunner::app_mut`
  panics unless the `App` overrides the wasm-only `as_any_mut`
  default; and mutations arriving outside the frame loop (bridge
  messages, file input) must `ctx.request_repaint()` or the canvas
  shows the stale frame until the next input event. Neither is
  reachable from desktop tests — only a real browser run catches them
  (see the playwright pattern, §6.2).
- **The `web` crate IS a workspace member** (`members = ["crates/*",
  "xtask", "web"]` in the root `Cargo.toml`, since `3295ef0`,
  2026-10-10). `cargo fmt --all`, `clippy --workspace`, and
  `test --workspace` all see it; the dedicated web steps in
  `.gitforge.yml` are belt-and-braces fast-fail steps, not the only
  web coverage. (This corrects an earlier note here that called the
  crate workspace-excluded.)

## 4. The plan — phases and atomic tasks

Each task follows the house ticket shape: **objective / inputs /
authority / stopping condition / verification**. A phase = one
decision doc + one batch + gates + one commit (unless size says
otherwise). Tasks inside a phase are atomic and independently
verifiable.

### Closed campaigns (for context — do not reopen without new evidence)

- **M0–M15** (2026-10-10 → 10-13): workspace bootstrap → format
  read/write → compositor → catalog + shell → vector depth → text +
  filters → import/export → scripting → symbols → web layer → parity
  hardening → shell parity → live filters → tool surface. Tickets in
  `docs/planning/firecraft/`.
- **The DOM verb-surface campaign** (2026-10-13 → 10-14, tasks
  #86–#134): 356 CS6 verbs swept into `dom-parity.md`; 57 missing
  verbs closed family-by-family to **268 C / 3 P / 0 M**; registry
  105 → 241. The full family table with commits lives in
  `PROGRESS.md`; the design rationale lives in `docs/decisions/
  0001–0063`. Phase 9's recorded debts are all closed; the one
  evidence-gated hold is in §4 Phase 12.

### Phase 19–21 — The daily-driver expansion (the active front, 2026-10-31 →)

**The engine is complete; the shell is not.** A judge-panel audit
against Fireworks CS6 and Photopea found the structural gap: 243
registered commands, **8 of 289 CS6 menu items still rendering
`Action::Inert`** (derived via `menus::tests::inert_worklist`, gated
by the ceiling test). The program is reachability and canvas
competence, not repair.

The full plan, with the landed/open state of every item, lives in
[`NEXT.md`](NEXT.md) — **read that file for the work list**, not this
one. Summary:

- **19.1–19.5 LANDED** (2026-10-31 → 11-01): transform handles +
  Numeric Transform, Subselect path-node editing, gradient fill
  editor, command palette, grid/guides/snap. Inert count 181 → 171.
- **19.6 IN FLIGHT**: the inert-menu burn-down, in family batches
  (selection/mask → page/frame → symbol/animation → text). Batch 1
  (selection/mask) landed 2026-11-02 (`e9ece7d`, with the context
  menu and Lucide icons): inert 171 → 144; batch 2 (canvas dialogs,
  Duplicate/Clone) + 19b.1 (path-node add/delete) landed 2026-11-03
  (`bf1b547`): inert → 129 of 289; batch 3 (Window-menu panel
  toggles — Hide Panels/Tab, Main toolbar, 7 docked panels) landed
  2026-11-03: inert → 120; batch 4 (page/frame — Insert ▸ State,
  Trim/Fit Canvas, canvas rotations via the new `Action::Document`
  arm) landed 2026-11-03; batch 5 (symbol/animation — Convert to
  Symbol and Animate Selection via `symbol.create`, Tween Instances
  via `symbol.tween`) landed 2026-11-03: inert → 111; batch 6
  (View menu — Fit Selection, Gamma 1.8, Tooltips, Show All, Rulers,
  Smart Guides, 9-Slice guides, Edges) and batch 7 (Text menu —
  Plain, Align, Smaller/Larger, Orientation, Detach from Path,
  Reverse Direction, Convert to Paths) landed 2026-11-03: inert →
  85; batch 8 (Attach In Path / Attach to Path via
  `text.attachPath`, the rail derived from a selected path object)
  landed 2026-11-03: inert → **83 of 289**. The Text vertical-align
  rows (Top / Centered Vertically / Bottom / Justified Vertically)
  are **evidence-gated**: the node dictionary documents no text
  vertical-justification node (`JSTi` is horizontal-only), so wiring
  them would invent model state — the same posture as the 3 P parity
  rows. Next batch: the Modify and File families.
  Batch 9 (Modify ▸ Transform: Free Transform and Distort set the
  handle mode — Distort drags a corner independently via
  `object.setQuadrangle` with drag-start fixed corners; Split via
  `path.splitPaths`; Remove Transformations via `object.transform`
  with each stored `MTXv`'s inverse) landed 2026-11-03: inert →
  **67 of 289**. Batch 10 (same day): the Window family — Tools
  folds the left dock; Color Mixer, Swatches, and Info dock as
  toggled right-dock panels; Duplicate Window/Cascade/Tile report
  the single-document shell honestly. Batch 11 (same day): Edit ▸
  Insert bitmap rows — Empty Bitmap (`bitmap.addImage` spanning the
  canvas), Bitmap Via Copy/Cut (`bitmap.addImageViaCopy`/`Cut` on a
  selected bitmap), Crop Document (extents, `grow: false`). Batch 12
  (same day): Modify ▸ Animation ▸ Remove Animation via
  `symbol.breakApart` on the selected instance. Batch 13 (same day):
  the File family — Close/Close All start a fresh document (refusing
  to discard unsaved edits), Save All saves the one open document,
  Revert reloads the last-saved file, File Info reports
  pages/layers/objects/canvas, and the export rows open the export
  dialog on the named format. Inert → **42 of 289** (batch 14).
  Batch 15 (2026-11-04): the Edit remainder + Text rows — Insert ▸
  New Button (a button-definition `library.add`), Hotspot /
  Rectangular / Polygon Slice (`hotspot.create` / `slice.create`
  over the selection bounds, canvas center when empty; the polygon
  variant traces the rect's corners), and honest `ReportGap` messages
  for Find and Replace, Copy as Vectors, Copy HTML Code, Paste
  Attributes, Crop Selected Bitmap, Preferences, Keyboard Shortcuts,
  Text ▸ Size ▸ Other, the four vertical-align rows (still
  evidence-gated — no documented wire node), and both spelling rows.
  Inert → **24 of 289**. Batch 16 (same day): the Modify remainder —
  Edit Symbol / Edit in Place enter the selected instance's
  definition scope (`enter_symbol`, view state); Hard / Anti-Alias /
  Feather Fill drive `path.setFillEdgeMode` on every selected path
  (feather rides the panel's radius); the animation/JPEG Settings,
  Swap Symbol, Group as Mask, the three Pop-up Menu rows, and Snap
  To Pixel report their gap plainly. Inert → **11 of 289**. Batch 17
  (same day): the CS6 web panels — Auto Shapes, URL, and Behaviors
  become real panels (`shell::panels_web`, decision 0069) whose every
  action is one engine command (`object.createPath` / `autoshape.*`,
  `slice.setLink`, `slice.addBehavior` / `slice.removeBehavior` /
  `slice.setData`). Inert → **8 of 289** — the remaining rows are the
  scripting family (Commands ▸ Manage Saved Commands, Filters ▸
  Repeat Filter), the Text Font placeholder (a verbatim CS6 resource
  artifact), and the Window panel rows pending their features
  (Optimize / Common Library / CSS Properties / Find and Replace).
- **19b**: path-node add/delete **DONE** (`bf1b547`); the
  wasm-budget re-pin **DONE** (2026-11-03: measured 21,235,599 B,
  ceiling moved to 21,500,000 with the dated cause — 19b.4). The
  reachability gate and inert-drift test landed 2026-11-02
  (`menus::tests::inert_count_never_rises` + the derived census).
- **20 / 21 queued**: daily-driver features (in-canvas text, Optimize
  panel, animation timeline, mask stack, tabs, history rewind, slice
  behaviors) and differentiators (autosave, HiDPI, palette v2,
  preferences, in-app scripting).

The shell was split out of one 5.9k-line `shell.rs` into `shell/`
modules (`32a110d`) before this work — six canvas surfaces were about
to accrete onto it.

### Phase 10 — Browser shell surface — LANDED 2026-10-15 (commit `67532fa`, v0.2.1)
The v0.2.0 web bundle could not open, import, or save anything: every
bridge call panicked in `WebRunner::app_mut` (`Shell` never overrode
the wasm-only `eframe::App::as_any_mut` default, which returns `None`),
and the smoke that should have caught it matched its verdict string
inside the host page's own script source (always green). A second
defect hid behind the first: bridge mutations never repainted, so even
after loading, the canvas showed the stale untitled frame. All three
are fixed and the deploy is verified end to end (open → draw → save in
a real browser). **Carry-forward lessons are in §3 — read them before
touching `web/` or the smoke.**

### Phase 11 — Image support enhancements (the standing "more image support" direction)

- **11.1 Lossy WebP — decide first.** The `image` crate's lossless
  VP8L encoder is pure Rust (landed, decision 0063); lossy VP8 needs
  libwebp-sys (C) — a wasm-build breaker unless feature-gated
  native-only. Decision doc must record the feature-gate design
  *before* code. Stopping: native lossy WebP export behind a cargo
  feature, wasm unaffected, or a recorded refusal.
- **11.2 AVIF decode.** No pure-Rust AVIF decoder is production-grade;
  the honest doors are OS decoders (desktop) or wasm-friendly
  fallback (decode failure naming the format). Investigate
  `image`'s avif-native feature state first; record findings in a
  decision doc before any dependency lands. Import-only is acceptable;
  export stays out of scope until an encoder exists without C.
- **11.3 HEIC — likely refusal.** No maintainable open encoder/decoder
  without patents-or-C problems. Expected outcome: a decision doc that
  records the refusal and the import path (OS decode on desktop
  builds only, if anywhere). Do not ship a half-decoder.
- **11.4 Import surface parity with the export matrix.** The CLI
  `import` door takes jpg/png/webp; `io::raster::decode` sniffs more
  (bmp/tiff/gif). Align the CLI accept-lists and error strings with
  what the decoder actually supports, one test per format.

### Phase 12 — Evidence-gated holds (parked, not lost)

- **Out-of-bounds effect growth** (decision 0040): the live render
  grows (compose kernels canvas-spanning since M14); the destructive
  bake clips by settled law. Reopening needs a CS6 filter/flatten
  bounds oracle (a Wine corpus run). **Setup recipe if attempted:**
  `source/FireworksPortable/` exists to run CS6 under Wine; the corpus
  it would generate is an Adobe-authored test oracle — usable locally,
  never committed.
- **`setFillNColorNTexture`** (parity P): needs an observed `TXFv`
  shape. Same oracle: author a textured fill in CS6 under Wine, read
  the file. Without it, the row stays P.
- **`setAnimInstanceStartFrame` / `setSliceAutonaming`**: permanent —
  the wire nodes do not exist; refusing is faithful. Closed.

### Phase 13 — Release engineering hygiene (small, high-leverage)

- **13.1 WebOS packaging dry-run.** `docs/WEBOS.md` sketches the
  `appinfo.json`; build a real one + operator-supplied icons
  (ATTRIBUTION rows in the same change) and land a `host.html` in
  `dist/web` staging so the packaged artifact is the proven one.
- **13.2 Platform matrix recipes.** `packaging/` carries AppImage, macOS
  dmg, and Windows scripts that this repo has never executed
  (TOOLCHAINS.md records why). Each one that gets run on real hardware
  should produce its artifact for the next release; until then the
  release notes keep the honest "built and verified on Linux x86_64"
  line.
- **13.3 Release notes in-repo?** `dist/release-notes.txt` is
  gitignored staging. Consider `docs/releases/vX.Y.Z.md` (tracked) so
  notes survive in history; keep the SHA block generated.
- ~~**13.4 Smoke budget.**~~ **Decided (0066): keep the constant.**
  Virtual time covers only JS execution; wasm fetch/compile are wall
  clock and outside the budget, so the 60000 ceiling cannot mask a
  slow-loading bundle. The load-side budget is the wasm size ceiling
  (`enforce_wasm_budget`) — tighten that if the bundle grows, not the
  virtual-time constant.

### Suggested order

**Current front:** 19.6 (inert burn-down) → 19b gates → Phase 20 →
Phase 21 — see [`NEXT.md`](NEXT.md) for the live order and per-item
state; it supersedes the ordering below, which dates from the
Phases 11–13 era (Phases 11.1–11.4 and 13.1–13.4 are closed; Phase 12
stays parked unless the Wine oracle materializes). Re-verify the
deployed site after any `web/` change (§6.2) — the browser is the
primary target and the only place its failure modes live.

## 5. Parity-matrix regeneration recipe

The generator is a working note, not repo code (it embeds Adobe-derived
verb text):

```bash
# 1. Command surface from the fuzz CALLS table (authoritative):
sed -n '527,/^];/p' crates/engine/src/fuzzdrive.rs \
  | grep -oE '"[a-z]+\.[A-Za-z]+"' | tr -d '"' | sort -u \
  > /var/tmp/firecraft_commands_full.txt
# 2. Edit rows in /var/tmp/domp/gen.py (python3 heredoc script; the
#    Edit tool cannot reach /var/tmp), then:
python3 /var/tmp/domp/gen.py   # writes docs/planning/generated/dom-parity.md
```

Inputs: `/var/tmp/fw_api_verbs.json` (CS6 verb list) +
`/var/tmp/firecraft_commands_full.txt`. Regeneration recipes for the
other extracts: `PLANNING.md` §5.4. Reading `dom-parity.md` from
`/var/tmp/domp` with a relative path reads a STALE shadow copy — use
the absolute repo path.

## 6. CI runbook (GitForge)

### 6.1 Pipeline

- **Push order:** `git push gitforge main && git push origin main`
  (tags ride `--tags` when releasing).
- **Find the run:** `GET /api/pipeline-runs?limit=15` — match
  `commit_hash` (field name is `commit_hash`, *not* `sha`; both are
  `None` in the list view — fetch jobs to identify a run by its 7-job
  shape).
- **Wait + jobs:** `GET /api/pipeline-runs/{id}` until status is
  terminal; `GET /api/pipeline-runs/{id}/jobs` for the 7 jobs
  (fmt-layers, clippy-test, wasm-web, corpus-golden, perf-budgets,
  fuzz, generated-docs). All must be `succeeded`. ~15–18 min.
- **Job logs:** `GET /api/jobs/{job_id}/logs` — walk
  `data.logs[].chunk` or `data.receipt.step_results[].stdout/stderr`
  (helper: `/var/tmp/getjoblog.py`).
- **Known runner quirks:**
  - *"mkdir: file exists"* infra failure = sshfs race on the fedora
    runner → retrigger via an empty commit in a quiet window (no
    retry API).
  - *Silent mid-compile death* = offline-refusal from a stale image
    cache → check the job-log ledger FIRST. Re-ship the image
    (`infrastructure/docker/ci-rust.Dockerfile`; `docker save |
    ssh <host> docker load` to BOTH hosts) whenever an **external**
    crate version in either lockfile moves — workspace-only version
    bumps leave `cargo fetch` output unchanged (verified v0.2.0).
- **Local gates before any push:** the full gate is
  `cargo run -p xtask -- ci` (fmt, clippy `-D warnings`, tests, layers,
  wasm, corpus, goldens, perf, fuzz-quick, parity, scorecard,
  changelog — the `web/` crate is a workspace member and is covered by
  the shared gates). Per-crate commands
  for dev-loop iteration. `git checkout --
  corpus/scripts-report.json` before committing if the corpus run
  touched it (timestamp-only churn).

### 6.2 Proving a web change (do this before pushing any `web/` or shell change)

The desktop test suite cannot see the browser failure modes — both
Phase 10 defects passed every desktop gate. The working pattern
(playwright, headless chromium):

1. `cargo run -q -p xtask -- dist --web` — the bridge smoke must pass
   **honestly** (script bodies stripped before the verdict check).
2. Serve `dist/web` on loopback, then with playwright: load the page,
   install a `message` listener for `source: "firecraft"` replies
   FIRST, then drive. Reply to `ping` proves the downcast; a
   `postMessage openDocument` + **zero input events** + screenshot
   proves the repaint; `expect_file_chooser` + a real `.fw.png` from
   `corpus/fixtures/` proves the menu open; `expect_download` proves
   save. Check `pageerror` after every step (`unreachable` = a wasm
   panic).
3. Redeploy Pages and repeat step 2 against the live URL — check the
   served `MANIFEST.sha256` hash matches `dist/web` before
   diagnosing anything (edge propagation lags a few minutes).

### 6.3 Release + deploy doors

- **Version stamps (8):** workspace `Cargo.toml`, the six member
  `Cargo.toml`s incl. `web/`, the `xtask` `compose` dep pin,
  `packaging/deb/control`. `xtask changelog` verifies consistency.
- **Artifacts:** `cargo run -q -p xtask -- dist --native` (desktop +
  cli tar.gz, deb, MANIFEST.sha256 → `dist/native/`) and
  `dist --web`. Web zip is manual, files at root:
  `cd dist/web && zip -qr ../firecraft-web-<v>.zip .`
- **Publish:** tag, `git push gitforge main --tags`, `git push origin
  main --tags`, `gh release create vX.Y.Z --notes-file
  docs/releases/vX.Y.Z.md <5 assets>`. GitForge has no release-asset
  surface (tags only); GitHub carries the assets. The release notes are
  tracked in `docs/releases/` (13.3, closed) — `dist/release-notes.txt`
  is gitignored staging and can be stale; do not ship it.
- **Pages:** `wrangler pages deploy dist/web --project-name
  firecraft-web` (authenticated via `CLOUDFLARE_API_TOKEN`;
  <https://firecraft-web.pages.dev>, direct-upload project).

## 7. Session-close checklist

1. Devlog entry (`log/devlog.md`) — what landed, commit hash, lessons.
2. `PROGRESS.md` — move rows, update counts.
3. `SESSIONS.md` — one dated entry.
4. Task board state (the session task list mirrors the active phase).
5. Commit docs with the work (or as an immediate follow-up), push,
   verify CI on the final hash.
6. If `web/` or the shell changed: §6.2 against the live URL, then a
   Pages redeploy if the bundle moved.
