# FireCraft — Agent Handoff

> **Start here.** This is the entry point for any agent (or human)
> picking up FireCraft work. It answers: where things stand, where
> everything lives, how to work here, and exactly what to do next.
> Day-to-day rules: [`../AGENTS.md`](../AGENTS.md). Master plan:
> [`PLANNING.md`](PLANNING.md). Live status: [`PROGRESS.md`](PROGRESS.md).
> Session log: [`SESSIONS.md`](SESSIONS.md). Project arc: [`HISTORY.md`](HISTORY.md).

---

## 1. State snapshot (2026-10-14)

- **Ship state:** v0.1.0 released, repo public. Roadmap M0–M15 all
  closed. 153+ commits, CI green on `25fedbc` (7/7 jobs).
- **Engine registry:** 148 commands, pinned equal to the fuzz call
  table by `the_call_table_covers_the_catalog`.
- **CS6 DOM verb-surface parity:** **172 C / 45 P / 57 M / 82 N**
  (`docs/planning/generated/dom-parity.md`, regenerate per §5 below).
- **Active campaign:** close the 57 remaining M (missing) verbs in
  family batches — one decision doc + one command batch per family,
  exactly as #87–#93 did. The plan is §4 below.
- **Clean-room invariants (never violate):** `source/` is Adobe's
  copyrighted material — git-ignored, never committed, never copied
  from. Adobe-derived text extracts are uncommitted working notes under
  `/var/tmp/`. Adobe-authored corpus files are test oracles only. The
  `AMT/` tree is out of scope entirely. Assets need a
  `docs/ATTRIBUTION.md` row in the same change.

## 2. Where everything lives

| Path | What |
|---|---|
| `AGENTS.md` | Operating manual: commands, gates, commit rules, day-to-day method |
| `docs/PLANNING.md` | Master plan: format spec (§6), architecture (§7), method |
| `docs/planning/firecraft/` | `ROADMAP.md` + tickets M0–M15 (all closed; reference for how tickets are written) |
| `docs/decisions/` | Decision records 0001–0022 — **write one per family/batch** |
| `docs/planning/generated/` | `parity.md`, `scorecard.md` (xtask-owned) + `dom-parity.md` (generator-owned, see §5) |
| `docs/research/fireworks-png-nodes.md` | The node dictionary — the format Rosetta stone (observational; corpus wins on conflict) |
| `log/devlog.md` | Detailed session log (gitignored) — **append an entry to close every task** |
| `crates/engine/src/catalog/` | The command registry, one module per family |
| `crates/engine/src/fuzzdrive.rs` | Random driver: `CALLS` table + payload builders (pinned to the catalog) |
| `corpus/fixtures/` | Committed Fireworks fixtures (round-trip/golden oracles) |

## 3. How to work here (the loop that produced #87–#93)

1. **Pick the next family** from §4. Read the CS6 API text for its
   verbs (regeneration recipe: `PLANNING.md` §5.4; the extract lives at
   `/var/tmp/fw_ext_full.txt`).
2. **Write the decision doc first** (`docs/decisions/00NN-<topic>.md`):
   wire/model design, CS6 API mapping, deviations, consequences,
   recorded debts. Look for existing types before inventing new ones.
3. **Implement model → reader/writer → delta → commands → tests** in
   that order; run per-crate gates as you go (`cargo test -p <crate>`,
   clippy `-p`). Hooks deny workspace-wide cargo.
4. **Grow the fuzzer in the same change**: append the new command names
   to `CALLS` in `fuzzdrive.rs` + a payload builder arm each —
   `the_call_table_covers_the_catalog` fails otherwise.
5. **Update the parity matrix**: flip the family's rows M→C in
   `/var/tmp/domp/gen.py` (edit via a `python3` heredoc script — the
   Edit tool is cwd-bounded and hooks deny `| python3`), regenerate,
   then `cargo run -p xtask -- parity scorecard floor layers`.
6. **Gate, commit, push, verify CI** (runbook §6), append the devlog
   entry, update `PROGRESS.md`/`SESSIONS.md`.

**Standing engineering lessons (each cost a real failure once):**

- **Capture before mutating.** A delta holds *prior* state; capturing
  after the mutation silently undo-writes the post-state. This produced
  three test failures in #92. Every `before = …clone()` line goes
  above the first mutation of that slice.
- **Never trust an RNG stream across catalog growth.** Fuzz-derived
  tests that *force* a call class must draw-until-landed **and** seed
  deterministic state first — the generator deliberately over-draws
  indices and injects junk fields for refusal coverage, so a thin
  surface can have no landable draw for a fixed seed (#92's CI leg).
- **Fixture-first, corpus-second** (quality rules): validate each layer
  separately — wire round-trip, then model equality, then composition.
  An end-to-end pass alone hides which layer regressed.
- **No corpus oracle → the dictionary stands** and a synthetic
  write→read→byte-equal round-trip is the gate; record the missing
  oracle in the decision doc (#93 did this for `LSMv`/`LSAv`).

## 4. The plan — phases and atomic tasks

Each task follows the house ticket shape: **objective / inputs /
authority / stopping condition / verification**. A phase = one decision
doc + one command batch + one parity flip + one commit (unless size
says otherwise). Tasks inside a phase are atomic and independently
verifiable.

### Phase 1 — Object-selection surface + selection tooling (12 verbs)
*Do first: Phase 2 is blocked on it. The current `Document.selection`
is the raster marquee (`SelectionMask`); there is no object-level
selection.*

- **1.1 Type the object selection.** Objective: a document-level
  object selection (`Vec<ObjectId>`), an ordering rule (z-order), and a
  delta variant (capture `before: Vec<ObjectId>`). Authority: CS6
  `fw.getSelection()` semantics. Stopping: commands can read AND set
  the selection; undo round-trips; scripting projection exposes it.
  Verify: unit tests + fuzz arms for the new commands.
- **1.2 `selection.*` basics.** `selectAll`, `selectInverse`,
  `selectAllOnLayer` (scope), `selectNone` if absent. Stopping: each
  command + undo + refusal (empty document → Noop or InvalidState per
  CS6) + fuzz arms + parity rows C.
- **1.3 Hierarchy selects.** `selectParents`, `selectChildren` — walk
  group containment from the current selection. Stopping: selection of
  a group child promotes/demotes exactly per CS6 text; undo exact.
- **1.4 Attribute selects — CORRECTED 2026-10-14 (decision 0025).**
  The API text showed the original premise wrong: `selectSimilar` is a
  *bitmap-mode* verb (pixels within tolerance of the selection's
  average color — mapped to `bitmap.selectSimilar`),
  `selectSimilarFromPoint` is carried by `bitmap.selectWand`, and the
  `filterSelection*` trio are *filter-application* verbs (permanent
  pixel filters — filter family + the bitmap bake debt, **not**
  selection surface). Landed: `selection.selectParents` /
  `selectChildren` (group hierarchy). The genuinely-unclaimed
  attribute select (fill/stroke/type predicates) has **no CS6 dom
  verb** — drop the idea from this campaign unless corpus evidence
  surfaces one.

### Phase 2 — Selection-to-frames (3 verbs + 1 typo dup) — needs Phase 1
- **2.1** `duplicateSelectionToFrames` + the `cuplicateSelectionToFrames`
  typo row; **2.2** `duplicateSelectionToFrameRange`;
  **2.3** `distributeSelectionToFrames`. All are clone_element fans
  into target cells — reuse `clone_element`/`check_closure` from
  cell.rs (the layer.reorder copy branch is the template). Stopping:
  per-frame fan with closure validation; undo retracts fresh slots.

### Phase 3 — Group/flatten (3 verbs)
- **3.1 `mergeDown`.** Merge the layer/selection into the layer below.
  Vector+vector = composite into one path set is CS6-faithful only as a
  bitmap bake for overlapping content — decide in the decision doc
  (path booleans exist: decision 0003). Stopping: golden-verified
  composite; undo exact.
- **3.2 `flattenSelection`.** Bake selected objects into one bitmap
  object via the compose render path (render → tile bank → ImageObject
  swap). Depends on nothing; shares machinery with debt 6.5.
- **3.3 `flattenDocument`.** Whole-page bake; page-level variant of 3.2.

### Phase 4 — View flags (4 verbs) — evidence-gated
- **4.1 Corpus hunt:** prove or refute a wire home for
  show/snap flags (`setShowGrid/setShowGuides/setSnapToGrid/`
  `setSnapToGuides`) — sweep the dictionary + corpus + CS6 `.jsf`
  Configuration tree for a view-prefs node. Stopping: a written
  finding (decision doc section) either way.
- **4.2** If a node exists: model + commands (page-scoped, like the
  guide/grid family). If not: host-session state — implement at the
  app/scripting layer only and flip the rows to `N` with the evidence
  note. **Do not invent a wire node.**

### Phase 5 — Document/image ops (4 verbs)
- **5.1 `setDocumentCanvasColor`** — `background_raw` is already
  modeled and rendered (77f55af); this is a one-command batch. Do it
  first: cheapest win in the matrix.
- **5.2 `setDocumentResolution`** — `RESf`/`RSUi` are parsed; add the
  setter command.
- **5.3 `rotateDocument`** — 90°-step canvas rotation with a geometry
  re-bake (transforms + canvas swap). Decide multi-page semantics
  (CS6 rotates the current page's canvas) in the decision doc.
- **5.4 `setDocumentImageSize`** — bitmap resample pass (shares the
  resample debt 6.4). Largest task in the phase; schedule after 6.4's
  design.

### Phase 6 — Primitives / smart shapes (5 verbs)
- **6.1** `setRectRoundness`, `setRectRoundnessMode`, `setRectSides`,
  `setQuadrangle` — `PathObject.primitive` is modeled; these are param
  setters that re-derive the contours. One command batch.
- **6.2** `redrawSmartShape` — autoshape params are creation-only today
  (decision: autoshape.* family). Add the param-edit command over the
  Auto Shape library's sources. Stopping: edited autoshape re-renders;
  golden gate.

### Phase 7 — Button symbols (5 verbs) — wire-evidence-gated
- **7.1** Evidence pass: where do button states live on the wire?
  (dictionary `MSTv`/symbol nodes + corpus + `.jsf` button samples.)
- **7.2** Model + commands: `setButtonOptions`,
  `setButtonIncludeDownState`, `setButtonIncludeOverWhileDownState`,
  `setButtonShowDownOnLoad`, `setButtonAutoSlice`. Same shape as the
  9-slice work (decision 0014 territory).

### Phase 8 — Misc singles (independent; any order)
- **8.1 Place-image trio**: `addNewImage` / `…ViaCopy` / `…ViaCut` —
  `io::raster::attach_image` is the import path; the command wraps it
  with placement + registry insert. ViaCopy/ViaCut differ in clipboard
  interaction — coordinate with 6.6.
- **8.2 Web-object point append**: `appendPointToHotspot`,
  `appendPointToSlice` — geometry append onto slice/hotspot rects
  (recreate-vs-append decision needed).
- **8.3 Split/knife**: `splitPaths`, `knifeElementsFromPoint`,
  `knifeElementsFromPoints` — boolean ops exist (decision 0003);
  knife = line-vs-path intersection + split. Largest single here.
- **8.4 `convertToPaths`** (text-to-paths) — glyph outline extraction
  into PathObjects; the text render already flattens glyphs, so the
  outline source exists.
- **8.5** `cropSelection` (crop to selection/mask bounds),
  `setFillEdgeMode`, `setGradientName` (gradients need names in the
  model + wire check), `tween` (instance tweening over the existing
  SymbolAnimation model).
- **8.6 Clipboard mask ops**: `clipPasteAsMask`, `clipPasteInside`,
  `clipPasteFromChannelToChannel` — needs the clipboard surface (6.6).

### Phase 9 — Recorded debts (P-class limitations; no parity pressure)
- **6.1-object-duplicate** deep-clone of groups (today shallow).
- **Pattern-authoring command** (pattern fills landed; authoring UI
  command missing).
- **mkBT tile-bank WRITER** (reader-side bank exists; writer side
  missing — check `fwpng`/`doc::library`).
- **Image resample pass** (feeds 5.4).
- **Clipboard surface** (feeds 8.6).
- **Shared-layer link-resolution render** (decision 0022: link layers
  render empty today).

### Suggested order
5.1 → 5.2 (quick wins, warm up the loop) → Phase 1 → Phase 2 → 6.1 →
6.2 → 8.2 → 8.5 → Phase 3 → 8.3 → 8.4 → 5.3 → 4 (evidence) → 7
(evidence) → 8.1 → 8.6/6.6 → 6.4 → 5.4. Re-derive from
`dom-parity.md`'s M table if the matrix has moved; the plan follows the
matrix, not the reverse.

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
other extracts: `PLANNING.md` §5.4.

## 6. CI runbook (GitForge)

- **Push order:** `git push gitforge main && git push origin main`.
- **Find the run:** `GET /api/pipeline-runs?limit=15` — match
  `commit_hash` (field name is `commit_hash`, *not* `sha`).
- **Wait + jobs:** `GET /api/pipeline-runs/{id}` until status is
  terminal; `GET /api/pipeline-runs/{id}/jobs` for the 7 jobs
  (fmt-layers, clippy-test, wasm-web, corpus-golden, perf-budgets,
  fuzz, generated-docs). All must be `succeeded`.
- **Job logs:** `GET /api/jobs/{job_id}/logs` — walk
  `data.logs[].chunk` or `data.receipt.step_results[].stdout/stderr`
  (helper: `/var/tmp/getjoblog.py`).
- **Known runner quirks:**
  - *"mkdir: file exists"* infra failure = sshfs race on the fedora
    runner → retrigger via an empty commit in a quiet window (no retry
    API).
  - *Silent mid-compile death* = offline-refusal from a stale image
    cache → check the job-log ledger FIRST; re-ship the image whenever
    `Cargo.lock` moved.
- **Local gates before any push:** `cargo fmt --all -- --check`,
  `cargo clippy -p <touched> --all-targets -- -D warnings`,
  `cargo test -p <touched>`, the four xtask verbs (§3.5).
  `git checkout -- corpus/scripts-report.json` before committing if the
  corpus run touched it.

## 7. Session-close checklist

1. Devlog entry (`log/devlog.md`) — what landed, commit hash, lessons.
2. `PROGRESS.md` — move rows, update counts.
3. `SESSIONS.md` — one dated entry.
4. Task board state (the session task list mirrors the active phase).
5. Commit docs with the work (or as an immediate follow-up), push,
   verify CI on the final hash.
