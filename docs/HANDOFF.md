# FireCraft — Agent Handoff

> **Start here.** This is the entry point for any agent (or human)
> picking up FireCraft work. It answers: where things stand, where
> everything lives, how to work here, and exactly what to do next.
> Day-to-day rules: [`../AGENTS.md`](../AGENTS.md). Master plan:
> [`PLANNING.md`](PLANNING.md). Live status: [`PROGRESS.md`](PROGRESS.md).
> Session log: [`SESSIONS.md`](SESSIONS.md). Project arc: [`HISTORY.md`](HISTORY.md).

---

## 1. State snapshot (2026-10-15)

- **Ship state:** v0.2.1 released (v0.2.0's web bundle shipped a dead
  browser bridge — replaced within the day; see §4 Phase 10 for what
  was wrong). Repo public. Desktop + CLI + web bundle + deb all cut
  from `xtask dist`. The web bundle is live at
  <https://firecraft-web.pages.dev> (Cloudflare Pages, direct upload —
  deploy door in §6). CI green 7/7 per batch (runbook §6).
- **Engine registry:** 241 commands, pinned equal to the fuzz call
  table by `the_call_table_covers_the_catalog`.
- **CS6 DOM verb-surface parity:** **268 C / 3 P / 0 M / 85 N** — the
  M table has been empty since 2026-10-14. The 3 P rows are terminal
  at today's evidence: `setAnimInstanceStartFrame` and
  `setSliceAutonaming` are permanent (no wire node exists — refusing
  is the faithful behavior), and `setFillNColorNTexture` is
  evidence-gated (no observed `TXFv` shape, no mintable texture
  rasters). Do not reopen without new evidence.
  (`docs/planning/generated/dom-parity.md`, regenerate per §5.)
- **Where the frontier is:** the parity campaign reached its evidence
  limits; active work is **application surface and enhancements** —
  §4 Phase 10 (browser shell, landed), Phase 11 (image support:
  AVIF/HEIC/lossy-WebP), Phase 12 (evidence holds), Phase 13
  (release engineering hygiene). WebOS packaging
  (`docs/WEBOS.md`) is the standing primary-target goal.
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
| `docs/decisions/` | Decision records 0001–0063 — **write one per family/batch** |
| `docs/planning/generated/` | `parity.md`, `scorecard.md` (xtask-owned) + `dom-parity.md` (generator-owned, see §5) |
| `docs/research/fireworks-png-nodes.md` | The node dictionary — the format Rosetta stone (observational; corpus wins on conflict) |
| `docs/WEBOS.md` | The web embedding contract + the smoke host page (load-bearing — see the warning there) |
| `log/devlog.md` | Detailed session log (gitignored) — **append an entry to close every task** |
| `crates/engine/src/catalog/` | The command registry, one module per family |
| `crates/engine/src/fuzzdrive.rs` | Random driver: `CALLS` table + payload builders (pinned to the catalog) |
| `crates/app/src/shell.rs` | The egui shell (desktop + wasm); `eframe::App` impl at the bottom — carries the wasm-only `as_any_mut` and the stashed repaint context |
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
- **The `web` crate is workspace-excluded (trunk).** `cargo fmt
  --all`, `clippy --workspace`, and `test --workspace` never see it.
  The fmt gate checks it explicitly (added 2026-10-15 after an
  unformatted `web/src/main.rs` slipped through); remember it for
  clippy/test when touching `web/` — run them with
  `--manifest-path web/Cargo.toml`.

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
- **13.4 Smoke budget.** The bridge smoke is chromium-headless inside
  `dist --web`; it currently needs `--virtual-time-budget=60000`. If
  the wasm grows past the boot budget on slow machines, the smoke
  fails spuriously — budget the timeout off the measured load time,
  not a constant.

### Suggested order

11.4 (quick) → 13.1 → 11.1 (decision first) → 13.2/13.3 → 11.2 →
11.3 → Phase 12 only if the Wine oracle actually materializes. Phase
10 is closed. Re-verify the deployed site after any `web/` change
(§6.2) — the browser is the primary target and the only place its
failure modes live.

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
  `cargo run -p xtask -- ci` (fmt incl. the workspace-excluded `web/`
  crate, clippy `-D warnings`, tests, layers, wasm, corpus, goldens,
  perf, fuzz-quick, parity, scorecard, changelog). Per-crate commands
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
  dist/release-notes.txt <5 assets>`. GitForge has no release-asset
  surface (tags only); GitHub carries the assets.
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
