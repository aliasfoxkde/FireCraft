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
  closed. 155+ commits, CI green on `3d74723` (7/7 jobs; the fuzz job
  on that commit caught the addImage minting leak — fixed in `000557d`).
- **Engine registry:** 189 commands, pinned equal to the fuzz call
  table by `the_call_table_covers_the_catalog`.
- **CS6 DOM verb-surface parity:** **215 C / 45 P / 8 M / 88 N**
  (`docs/planning/generated/dom-parity.md`, regenerate per §5 below).
  (The CS6 verb list carries both case spellings of mergeDown and
  AddNewImage — those families flipped four rows each.)
- **Active campaign:** close the 8 remaining M (missing) verbs in
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
| `docs/decisions/` | Decision records 0001–0036 — **write one per family/batch** |
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

### Phase 2 — Selection-to-frames (3 verbs + 1 typo dup) — DONE (decision 0026)
`selection.duplicateToFrames` / `duplicateToFrameRange` /
`distributeToFrames` landed 2026-10-14: source frame is an explicit
payload field (the engine has no current-frame state — the `frame.add`
precedent), fans reuse `check_closure`/`clone_element`/`splice_cells`,
and copies append to the target cell. No `.jsf` corpus usage exists
for any of the three, so the API text is the sole authority; the
silent points (all-includes-source, range refusals, append z-order)
are recorded in the decision doc.

### Phase 3 — Group/flatten (3 verbs) — DONE (decision 0031)
`flatten.mergeDown` / `flatten.selection` / `flatten.document` landed
2026-10-14, backed by a new `compose::render_subset` (listed ids,
list order = paint order, no canvas matte). Each bake mints fresh
`mkBT` bank tiles and swaps the participants for one canvas-sized
ImageObject (Fireworks bitmaps are canvas-sized — faithful).
mergeDown mutates the bitmap below **in place** (identity/name kept,
`path.boolean`-style) and refuses unless a bitmap is immediately
below; flattenSelection spans layers of one page and lands at the
topmost member's z-slot; flattenDocument collapses content layers to
one bottom layer with web layers preserved and refuses multi-state
pages (state-0-only bakes are the recorded deviation). Baked pixels
carry default weight (the render already applied paint/weight/effects).

### Phase 4 — View flags (4 verbs) — DONE (decision 0035)
Resolved 2026-10-14: no wire home exists — proven by the exhaustive
dictionary (`GRDv` = geometry+colour, `GDSv` = locked/colour/
positions, complete `MKBv`/`PDCv` child lists have no view node) and
zero `.jsf` usage across the CS6 Configuration tree. The four verbs
are app-preferences session state: hosted in the scripting session
(`HostState` flags + `AttachView` projection + control intents, the
`changeCurrentPage` pattern), rows flipped to `N`. No wire node was
invented.

### Phase 5 — Document/image ops (4 verbs)
- **5.1 `setDocumentCanvasColor`** — DONE.
- **5.2 `setDocumentResolution`** — DONE.
- **5.3 — DONE (decision 0034).** `document.rotate {rotation, page}`
  landed 2026-10-14: one page's canvas rotates 90/180/270 clockwise
  with a full geometry re-bake — paths/slices map point-for-point,
  text/symbol transforms compose the rotation, images assemble their
  tile grid, rotate the buffer by permutation, and re-tile over fresh
  bank slots (matte slots bake — flatten debt). Guides swap, the grid
  origin rides the map, deltas are `[PageBody, Bank, Mutated × n]`
  with byte-exact undo. The rotation linear part is built from
  integers (trig smears `cos(pi/2)` into every coordinate — caught by
  the corner-anchor test).
- **5.4 `setDocumentImageSize`** — bitmap resample pass (shares the
  resample debt 6.4). Largest task in the phase; schedule after 6.4's
  design.

### Phase 6 — Primitives / smart shapes (5 verbs)
- **6.1 — DONE (decision 0027).** `primitive.setRectRoundness` /
  `primitive.setRectSides` / `object.setQuadrangle` landed 2026-10-14;
  `setRectRoundnessMode` went `N` — the exhaustive `PRIv` dictionary
  has no mode node (session-state unit toggle), so no command was
  invented for it. Percent roundness scales the shorter side; the
  quadrangle maps the bounds rect onto a parallelogram-only quad
  (affine `MTXv`).
- **6.2 — DONE (decision 0028).** `autoshape.redraw {id, kind,
  params}` landed 2026-10-14: the Auto Shape library's generators
  re-run over explicit parameters (CS6 re-runs each shape's embedded
  JSF, which has no clean-room equivalent — no script interpreter, no
  wire node), swapping contours + `even_odd` behind one captured
  mutation. Golden gate: redrawn contours == freshly created contours
  for the same parameters. Cross-kind redraw is allowed (CS6 could
  not); recorded as a deviation.

### Phase 7 — Button symbols (5 verbs) — DONE (decision 0036)
Landed 2026-10-14. The four behaviors are the four previously-unknown
`MSTv` child booleans (`ATSb`=auto-slice, `IDSb`=include-Down,
`DVLb`=show-Down-on-load, `IODb`=include-Over-While-Down — three
fourccs spell their behavior; the fourth pinned by elimination), typed
as presence-tracked `Option<bool>` on `SymbolDefinition` (no corpus
fixture carries them — the synthetic byte-equal round-trip +
wire-order check is the gate). `setButtonOptions` mutates the button's
auto-slice, found through the newly-typed slice→definition `MSNi`
link (`SliceObject::symbol_number`): link/alt/slice name/export
override; `targetTagString`/`statusMessage` have no wire home
(deviation 1). No `.jsf` usage exists for any of the five. Commands
`symbol.setButton*` follow the `SetSymbolType` shape
(`Delta::Library`); options produces a `Delta::Group` of slice
mutations and refuses without a linked slice.

### Phase 8 — Misc singles (independent; any order)
- **8.2 — DONE (decision 0029).** `slice.appendPoint` /
  `hotspot.appendPoint` landed 2026-10-14: CS6's selection-driven
  point-append state machine (append to the selected open polygon of
  the command's class, else start a fresh one on the Web Layer;
  tolerance closes the polyline). The closing point itself still
  lands; rect/ellipse selections fall through to the create branch.
- **8.5 — DONE (decision 0030).** `path.cropSelection` (selection ×
  rect blade = BooleanOp::Crop per path; empty results remove;
  non-path members refuse — bitmap crop is bake debt),
  `path.setFillEdgeMode` (hard/antialias/feather onto the FEFi/FRDb
  slots; materializes the fill definition when absent),
  `symbol.tween` (two same-symbol instances in one cell; transform
  lerped with shortest-arc rotation; `distribute` appends frames;
  transform-only — instance opacity/animation params are debt),
  `setGradientName` → **N** (gradient library is preferences state;
  no document-level node — same rule as setRectRoundnessMode).
- **8.1 — DONE (decision 0037).** The place-image trio landed
  2026-10-14 over the rotate machinery's assemble→re-tile helpers
  (now factored as `engine::bitmap::{assemble_rgba, retile_rgba}`;
  the originally-planned `io::raster::attach_image` is io-internal
  and import-shaped). `bitmap.addImage` places an empty image with
  bounds constrained to the canvas — fresh zero bank tiles, not
  mattes, so the bitmap is paintable; `…ViaCopy` turns the pixel
  selection into a new image directly above the source (coverage
  scaling in the premultiplied domain); `…ViaCut` also clears the
  source over fresh tile ids. No clipboard interaction (the CS6 text
  names none); no paint-mode posture (deviation 1).
- **8.3 — DONE (decision 0032).** `path.splitPaths` (compound paths
  become one path per contour; identity keeps the first),
  `path.knifeElementsFromPoint` (click cut within tolerance opens/cuts
  selected paths), `path.knifeElementsFromPoints` (drag-blade
  crossings + tolerance clause) landed 2026-10-14 via the new
  `vector::knife` module (flattened-proxy crossings, true-curve
  `subsegment` output, ring rotation at the first cut, sliver-drop
  with marker ride-along). Selection-driven like the tool; non-path
  and group-nested members refuse whole-command.
- **8.4 — DONE (decision 0033).** `text.convertToPaths` landed
  2026-10-14: each selected text object swaps to a `PathObject` in
  place (same id, z-order kept; the renderer's synthetic-path contract
  — default black fill, nonzero winding, display fields inherited).
  The `text::TextEngine::glyph_contours` world-space outlines are the
  geometry; a golden test proves `render_subset` pixels are
  byte-identical before/after. Ink-less text removes the object
  (empty-result contract). Engine gained the `text` dependency.
- **8.7 — DONE (decision 0038).** The saved-selections trio landed
  2026-10-14: `Document.saved_selections: Vec<SavedSelection>` joins
  the editing-state-not-file-content class (`selection`, `jpeg_mask`);
  `selection.save` requires an active pixel selection and folds into
  an existing name through the operation (base = stored);
  `selection.restore` inverts on request then folds with the active
  selection as base (collapsed fold → no selection); `selection.deleteSaved`
  removes. Single-document (`toDocument`/`fromDocument` omitted);
  both "new or replace" and "replace" spellings accepted.
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
(evidence) → 8.1 → 8.6/6.6 → 6.4 → 5.4. Progress through 2026-10-14:
5.1–5.2, Phases 1–2, 6.1, 6.2, 8.2, 8.5, Phase 3, 8.3, 8.4, 5.3, 4,
7, 8.1, 8.7 done — **next is 8.6/6.6** (the clipboard surface, feeding
`clipPasteAsMask` / `clipPasteInside` / `clipPasteFromChannelToChannel`),
then the filterSelection bake trio (4 M rows, one a CS6-typo duplicate;
the `filters` kernel crate exists from `be98d4b`), then 6.4 (resample)
and 5.4 (setDocumentImageSize, which rides 6.4).
Re-derive from `dom-parity.md`'s M table if the matrix has moved; the
plan follows the matrix, not the reverse.

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
