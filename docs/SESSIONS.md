# FireCraft — Session Log

> One entry per working session, newest last. **Append an entry every
> session** (with [`PROGRESS.md`](PROGRESS.md)) — this file is committed;
> the detailed per-task log is `log/devlog.md` (gitignored, local only).
> Narrative arc and lessons: [`HISTORY.md`](HISTORY.md).

Format: date — what the session did — outcome (commit / gate state).

---

## 2026-10-07 — Repository created
Project inception; planning documents authored (PLANNING.md, roadmap,
format research, vendored node dictionary).

## 2026-10-10 — Bootstrap sprint: M0–M5
- M0 workspace bootstrap (xtask gates: layers/wasm/ci/panic_hunt/
  corpus/parity/scorecard; AGENTS.md; license + NOTICE).
- M1 foundations (geom depth, dirty tiles, fuzz anchors); corpus
  harness landed with a no-Wine fallback.
- M2 fwpng read path + pinned corpus.
- M3 doc model + fwmap byte-exact round-trip.
- M4 compositor + golden-image gate + bench baseline.
- M5 catalog v1 (task #42), app-cli info/convert/apply (#43), app
  shell v1 + web trunk (#44/#45), gates closed (#46).
- Infra: GitForge CI pipeline wired.
- Outcome: workspace live, all gates green.

## 2026-10-11 — Vector depth + text + imports: M6–M8 (part)
- M6 Auto Shape library (#48), freeform/reshape + align/distribute
  (#49), surfaces + gates (#50).
- M7 text + text-on-path + surfaces + gates (#51.7–9).
- M8 tasks 3–6: SVG export + resvg golden loop, SVG import (usvg),
  PSD spike + reader (decision 0006), PSD writer.

## 2026-10-12 — The close-out day: M8–M12 complete
- M8 tasks 7–8: animated GIF ↔ states; EXPv both ways + batch + export
  dialog (M8 closed).
- M9 tasks 1–8: scripting crate (sandbox, prelude), read-side DOM
  projection, mutating host API, .jsf loading, corpus run harness,
  History replay + Save Steps, CLI run-script + control channel + MCP,
  gates/fuzz/floors — **M9 closed**.
- CI battle: M9 CI died mid-compile; diagnosed via the job-log ledger —
  cgroup kill suspicion disproven; root cause was offline-cache refusal
  (stale image after lockfile move). Image re-ship rule recorded.
- M10 tasks 5–7: rich symbols (Widget.opCode), library import/export +
  Common Library, gates — **M10 closed**.
- M11 tasks 1–6: slice model/wire, web-layer commands, webexport crate
  (HTML+Images, Images Only, CSS+Images, CSS Sprites, To Files,
  states→GIF, CSS Properties), shell + CLI surfaces, gates
  (decision 0017) — **M11 closed**.
- M12 tasks 1–6: perf gate, memory ceiling (2,000-object anchor),
  `xtask dist --web` + bridge smoke, WEBOS.md, `xtask dist --native`,
  release chore (`xtask changelog`, RELEASING.md) — **M12 closed;
  roadmap M0–M12 complete.**

## 2026-10-13 — Release + M13–M15 + the parity campaign begins
- **v0.1.0 released; repo public.**
- M13 closed (interaction parity + scorecard); M14 closed (live-filter
  parity); M15 tasks 5–6 closed (gradient fills end to end, style
  model + Styles panel + .stl import).
- M15 recorded gaps closed: canvas background (#82, 77f55af), SVG
  gradient paint-server import (#83, 93e0778), path.setFillColor
  collapse (#84), pattern fills end to end (#85).
- Task #86: the **DOM verb-surface parity matrix** — the CS6 DOM guide
  swept against the registry (`dom-parity.md`), 356 verbs classed
  C/P/M/N. The remaining work is now the M families.
- Tasks #87–#89: object.group/ungroup (96aa656), page guide/grid/onion
  (32dd03a), frame management (8187375).
- Task #91: object mask family (faa738d) — parity 162C/45P/67M.

## 2026-10-13/14 — Structure families (#92, #93)
- #92 layer/page structure (decision 0021): page.setName/reorder/
  setMaster/unsetMaster/addMasterLayer + layer.reorder/setLocked;
  PageBody/Pages deltas; sharing trio deferred with the typed-model
  requirement. e830802, parity 169C/45P/60M.
  - CI leg: the app-cli symbol journal test forced symbol calls by
    name; CALLS growth 138→145 shifted seed 31's stream onto an
    all-refuse run. Probe (deleted after) proved the generator's
    deliberate junk-field/index over-draw; fix = draw-until-landed +
    deterministic seed state (535340f).
  - Lesson banked: three test failures from capture-after-mutate in
    the same batch — the capture-order rule is now standing law
    (HANDOFF §3).
- #93 shared-layer model (decision 0022): LSMv/LSEv/LSAv typed,
  reader/writer wired, synthetic round-trip gate; layer.setSharing /
  shareToPages / detachShared; Delta::SharedLayers. 25fedbc,
  parity **172C/45P/57M**, CI 7/7.

## 2026-10-14 — Documentation + handoff (this session)
- Wrote the committed doc set for future agents: HANDOFF.md (phased
  atomic plan + runbooks), PROGRESS.md (status board), HISTORY.md
  (arc + decisions + lessons), SESSIONS.md (this file).
- Cleanup: scratch scripts tidied, doc pointers added to CLAUDE.md and
  AGENTS.md.

## 2026-10-14 — Gap-family campaign day (#94–#102)
- #94 committed doc set (HANDOFF/PROGRESS/HISTORY/SESSIONS) + campaign
  plan; devlog consolidated.
- #95 canvas color + resolution setters (4b813ef) · #96 object-selection
  model + selection.* basics (cc85956) · #97 hierarchy selects +
  bitmap selectSimilar (135cb6b) · #98 selection-to-frames fans
  (decision 0026, ba60816) · #99 primitive params + autoshape redraw
  (decisions 0027/0028, 93c59b4 + 614bc64) · #100 web-object point
  append (decision 0029, 3516d89) · #100b crop/fill-edge/tween
  (decision 0030, df8fe42) · #101 flatten family via compositor bakes
  (decision 0031, 235e4d4, render_subset) · #102 knife + splitPaths
  (decision 0032, new vector::knife module).
- Parity 172C/57M → **201C/26M** across the day; registry 148 → 176.
- Infra: t5500 runner offline since 2026-10-11 (no route to host);
  fedora-docker carries the CI image; re-shipped on lockfile moves per
  the lockfile-bytes rule.
  (continued) #102 landed 3baf178, CI 7/7; #103 convertToPaths with a
  pixel-identity golden (decision 0033), 56c6f2f, CI 7/7.
  #104 document.rotate (decision 0034): page-scoped 90/180/270 with a
  full geometry re-bake — integer linear part (trig fails exactness),
  image assemble→permutate→re-tile via compose::matte_premultiplied +
  flatten::next_tile_id made shared, guides/grid swap, byte-exact
  undo. Parity **203C/45P/24M**, registry 178. #105 view flags
  (decision 0035): no wire node exists (dictionary + .jsf evidence);
  setShowGrid family hosted as scripting-session state
  (changeCurrentPage pattern), rows flipped to N, a363f0f CI 7/7.
  #106 button symbols (decision 0036): the four `MSTv` booleans
  ATSb/IDSb/DVLb/IODb typed (auto-slice, include-Down,
  show-Down-on-load, include-Over-While-Down — the first three fourccs
  spell their behavior, the fourth by elimination); slice `MSNi` link
  typed (`SliceObject::symbol_number`); five `symbol.setButton*`
  commands; no corpus fixture carries the nodes — synthetic
  byte-equal + wire-order round-trip gate. Parity **208C/45P/15M**,
  registry 183. CI 7/7 on 3d74723.
  #107 place-image trio (decision 0037): `bitmap.addImage` /
  `addImageViaCopy` / `addImageViaCut` over the rotate machinery's
  loops, now factored as shared `engine::bitmap::{assemble_rgba,
  retile_rgba}` helpers (rotate_image deduplicated onto them); fresh
  zero tiles (not mattes) so placed bitmaps are paintable; coverage
  scaling in the premultiplied domain; cut rebuilds the source over
  fresh tile ids. `AddNewImage` rides both case spellings — four
  parity rows flipped. Parity **212C/45P/12M**, registry 186.
  #108 saved selections (decision 0038): `selection.save` /
  `restore` / `deleteSaved` over `Document.saved_selections`
  (editing state, never file content — the `selection`/`jpeg_mask`
  class). Save requires an active pixel selection and folds into an
  existing name (base = stored); restore inverts on request then
  folds with the active selection as base, a collapsed fold leaving
  no selection; CS6's "new or replace" spelling accepted alongside
  "replace". The CI fuzz campaign on 7ea2288 caught a real defect
  first — a refused `bitmap.addImage` (placement cell out of range)
  leaked its minted bank tiles and registry slot; fixed in 000557d
  with an immutable `access::cell`/`layer` pre-flight before any
  minting, plus a bank+registry byte-equality regression test.
  Parity **215C/45P/8M**, registry 189.
  #109 clipboard surface (decision 0039): `Document.clipboard`
  (editing state — the writer never reads it) holds a subtree-closed
  capture (`elements`/`ids`/`roots`; stale selection members skip).
  `clipboard.copy` deep-captures; `clipboard.cut` captures then
  deletes through the object.delete machinery; `clipboard.paste`
  pre-flights the placement cell before minting, re-mints bottom-up
  rewriting group children via the old→new map, and pushes Created
  deltas in **chronological mint order** — the registry's
  `retract_insert` only retracts the newest insertion, so a
  LIFO-sorted undo mismatches generations and fails. `clipPasteAsMask`
  keeps the CS6 exactly-one-selected throw, maps mask kind to the
  `GRTi` type (image beats vector by kind, as the manual states),
  and `replace` swaps the existing mask child in place;
  `clipPasteInside` delegates with defaults. `clipPasteFromChannelTo`
  `Channel` overlays a channel plane on a single selected image, or
  mints a grayscale image (`RGB = byte`) when none is selected.
  Parity **221C/45P/5M**, registry 195.
  2026-10-14 (cont.) — CI fuzz on 1d1c8dc caught the second
  refused-call mutation of the campaign: `clipboard.cut` wrote the
  clipboard and cleared the selection, then `detach()` failed on a
  live-but-unattached object (its owner group was cut earlier in the
  same call — only the group's registry entry is removed, so the
  child rides live). Fixed in 2ae190a: a read-only `find_owner`
  pre-flight refuses the whole cut before any mutation; descendants
  orphaned mid-loop skip silently, having left with the captured
  subtree. Regression test cuts the group then tries the orphan plus
  its healthy sibling, byte-comparing the refusal.
  #110 filterSelection bake family (decision 0040): the three CS6
  front doors — `filter.selection` (named, optional category, optional
  param bag), `filter.selectionByID` (raw `EffectMoaID` GUID), and
  `filter.selectionByName` (the manual's `filterSelectonByName` typo
  rides the same CANON carrier) — resolve through one `bake_core`:
  identity → `compose::plan` (unknown GUID/name refuses; `skipped
  != 0` refuses, no passthrough) → per-image `assemble_rgba` →
  `apply_stack` premultiplied → pixel-mask blend (255 keeps the
  original byte) → `retile_rgba` in place → per-image `Mutated` plus
  one `Bank` delta in a `Group`. `compose` publishes the 19-entry
  `FILTER_CATALOG` (Adjust Color/Blur/Noise/Sharpen/Other) and
  re-exports `apply_stack`/`Surface`, so engine gained no Cargo edge.
  Non-image selection members refuse whole (CS6 rasterizes first —
  debt); out-of-bounds effect results clip to the image surface.
  Fixture laws recorded the hard way: a box mean preserves affine
  gradients exactly (test fixtures must be non-linear), and
  premultiply rounding breaks invert involution on semi-transparent
  pixels (exactness requires alpha 255). Parity **225C/45P/1M**,
  registry 198.
  #111 `document.setImageSize` (decision 0041) — **the M table is
  empty**. Corpus authority first: BatchTemplate.jst's Scale workflow
  passes a left/top-anchored rect holding exact target dimensions, so
  the rect is the new document bounds (the manual's "fit in" is loose
  prose), and resolution rides `{pixelsPerUnit, units}` unchanged.
  Content maps through `T(rect.origin)·S(sx, sy)`; the 6.4 resample
  debt landed inside this batch as `bitmap::resample_rgba` — bilinear,
  edge-clamped, 1/256 fixed-point weights, premultiplied domain — with
  the rotate machinery renamed to the affine-generic
  `map_point`/`map_contours` and its inline image preflight factored
  into `preflight_page_images`. Resample output is capped at the
  editable 1..=8192 grid so minted tiles stay gatherable;
  `currentPageOnly` defaults true (CS3 spelling), false scales every
  page by its own canvas ratio; deltas `[Info, PageBody×pages, Bank,
  Object×n]`, byte-exact undo retracting minted tiles. Parity
  **226C/45P/0M**, registry 199 — document-mutation coverage 271/271.
  Remaining surface work: 45 P rows + Phase 9 debts.

## 2026-10-14 — convolve + motionBlur bake commands (#112, decision 0042)
- Two P rows became real bake commands riding the 0040 core:
  `filter.convolveSelection {kernel_width, kernel_height,
  kernel_values, affects_alpha}` and `filter.motionBlurSelection
  {blur_type: linear/radial/zoom, angle, distance, samples}` — both
  build a one-op `compose::Plan` and call `bake_core` (empty-selection
  refuse, per-image pre-flight, pixel-mask gating, byte-exact undo).
- `compose::KernelOp::Convolve {width, height, values,
  affects_alpha}` — the generic kernel behind the presets
  (`convolve3` now delegates). Edge-clamped, center anchor, raw i64
  accumulation, **no normalization**: the dom passes raw integer
  coefficients, so their scale is the caller's contract (zero-sum
  kernel → black, documented). `affects_alpha` extends the pass.
- `filters::motion_blur_taps(surface, angle, distance, taps)` —
  `samples` ("the number of times the original image is cloned") is
  the tap count; taps spread evenly over 0..=distance px. The effect
  carrier sets `taps = distance + 1`, whose step is exactly one
  pixel — **Live Effect output byte-identical**; cache hash gained
  the taps field. KernelOp gained `taps` on MotionBlur + the Convolve
  variant (cache tag "CV", svg label "convolve").
- Engine gates: kernel sides 1..=31, `values.len == w*h`, angle
  finite 0..=360, distance 0..=400, samples 0..=100, blur_type one of
  the three; everything refuses `InvalidState` before any mutation.
  Radial/zoom take distance as sweep-degrees/pinch-percent through the
  kernels' own carrier clamps (recorded substitution); angle stands
  unused there.
- Tests: identity kernel byte-equal + undo, laplacian zeros a solid
  (incl. edge-clamped corners), 1×1 ×2 alpha gate, samples≤1 exact
  copy, linear smear + undo, radial + zoom reach their kernels, nine
  refusal shapes byte-stable. Fuzz: CALLS + arms (identity/laplacian/
  double kernels + degenerate sizes/mismatched counts; blur_type
  incl. "spiral", angle −5..365, distance 0..410, samples 0..110).
- Parity flip: gen.py convolveSelection + motionBlurSelection P→C,
  command surface refreshed → 201. dom-parity: **228C/43P/0M/85N**,
  unresolved []. Registry 199→201.

## 2026-10-14 — web/group setter batch (#113, decision 0043) + setImageSize preflight fix
- Five P rows became commands: `hotspot.setRectangle` (frame rewrite,
  contours ride the 0041 old-frame→new-rect affine; copy mode inserts
  a duplicate after the original and selects it), `hotspot.setShape`
  (rectangle/oval/polyline filtered by hotspots/slices/both; unknown
  strings refuse, filter mismatches skip, empty result `Noop`),
  `hotspot.copyToHotspot` (single-union or per-object areas from
  `content_bounds`; `makeRectangular: false` donates polygon contours
  from slice/path sources; `whatIfMultipleSelected "ask user"`
  refuses — no headless answer), `object.setGroupType` ("normal" (or
  empty) normalizes `GRTi` mask groups to regular; mask-to-image/path
  spellings refuse — `mask.*` owns masks), `page.setWebObjectsVisibility`
  (lazy Web Layer provision; one `Layer` delta flips `display`).
- Shared-geometry law from 0041 extended: `content_bounds`,
  `map_contours`, `map_point` now `pub(crate)` catalog-wide.
- **CI catch (run eee56916):** the batch's fuzz CALLS shift exposed a
  latent `document.setImageSize` 000557d violation — its pre-flight
  loop called `scale_image` per page, minting bank tiles for page N
  before page N+1's resample cap refused ("failed call … mutated the
  document", CI seed 15305450055185247510, local seed
  11436121107453012762). Fix: `scale_image` now takes the assembled
  buffer; pre-flight validates cap + assembly for EVERY affected page
  (buffers held, the `bake_core` pattern) before the first bake runs.
  Regression pinned by
  `image_size_refusal_after_another_page_bakes_touches_nothing`.
  Local sweep 256×48 clean; the 8eae611 batch itself was sound.
- Registry 201→206. dom-parity: **233C/38P/0M/85N**, unresolved [].

## 2026-10-14 — anim instance parameter family (#114, decision 0044)
- Six `symbol.setAnimInstance*` verbs over the already-round-tripping
  `ANNi` carrier: NumFrames (anchor), LoopCount (FW3 spelling → same
  field per Adobe's own deprecation note), OffsetDist (ANXf/ANYf),
  RotationAmount (ANRi; fractional degrees round — the field is
  integral), ScaleAmount (ASCf, finite > 0), StartEndOpacity (0–100 →
  per-mille ASOi/AEOi).
- One selection walker serves all six (`edit_animation_instances`):
  non-instance members skip ("the selected instances of multi-frame
  image symbols"), plain instances materialize the carrier with
  Objects-panel defaults (1 frame, no shift, 100%, no rotation/ramp —
  the setFillEdgeMode materialize-when-absent precedent), empty
  selection refuses, instance-less selection `Noop`.
- `setAnimInstanceStartFrame` stays P: the FW4 format has no
  start-frame node (the dictionary's ANNi family is complete) — the
  verb refuses pointing at the documented substitute (instance
  placement). Inventing a wire node would fabricate format.
- Tests: six-field write + reverse-undo byte-exact (incl. carrier
  materialization + retraction), ten refusal shapes byte-stable,
  non-instance skip + Noop + empty-selection refuse.
- Registry 206→212. dom-parity: **239C/32P/0M/85N**, unresolved [].

## 2026-10-14 — text rectangle trio (#115, decision 0045)

Batch: `text.setTextRectangle`, `text.setTextRectangleAuto`,
`text.setTextRectangleAutoFromPoint` — the P→C flip of the CS6
text-frame verbs (8.11).

- `setTextRectangle` is a pure frame write: runs and `transform`
  untouched, re-flow is layout-time (the text crate wraps at
  `frame.width()` on every measure). Honors the dom promise "the text
  is not scaled or transformed".
- `setTextRectangleAuto` fits the smallest box via the new
  `TextEngine::auto_block`: unconstrained layout (no wrap), reduce
  widest glyph advance (kern × index, ×hscale, + glyph width·hscale)
  and deepest `line_top + line_height`. Keeps the frame's top-left
  corner.
- `setTextRectangleAutoFromPoint` pins the anchor by justification:
  left/default `x0 = anchor.x`, center `x0 = anchor.x − width/2`
  (documented), right `x1 = anchor.x` (undocumented symmetric mirror,
  recorded). Top-anchored in all modes.
- `ATEb` untouched — one-shot fits vs the persistent auto-enlarge
  mode (`text.setAttrs auto_enlarge`); recorded non-equivalence.
- Shared `fit_frame` tail: measure → gate → capture → frame write →
  `Mutated`. Zero-extent/inverted rects and empty runs refuse
  ("lays out to no box"); non-finite edges cannot survive JSON
  (serde_json maps them to null → payload decode error first), so
  the finite gate is fuzzer-covered, not JSON-test-covered.
- Tests: rewrite + undo byte-exact; auto-fit re-measure equality
  (`engine.auto_block` cross-check) + undo; justification anchoring
  via real `text.setAttrs` deltas; refusal byte-stability across
  two-selected / path-selected / zero-extent / inverted / empty-text.
- Registry 212→215. dom-parity: **242C/29P/0M/85N**, unresolved [].

## 2026-10-14 — canvas trim + delete-all (#116, decision 0046)

Batch: `document.setDocumentCanvasSizeToDocumentExtents`,
`document.setDocumentCanvasSizeToSelection`,
`document.deleteAllInDocument` (8.12).

- Trim semantics: the canvas rect (XLCf/YLCf origin + WIDf/HITf size)
  moves to hug the content bounds; objects keep their world
  coordinates (nothing scales — that is setImageSize's job). The
  whole PageInfo is captured for undo.
- FW3's "document" is one page: both verbs take an explicit page
  index and compute that page's extents via `content_bounds` (group
  and instance recursion included). `union_bounds` became pub(crate).
- `bGrowCanvas` false clamps width/height per dimension to the
  current size; the origin still moves. Selection trim always fits.
- Hygiene: zero-extent bounds floor at 1px/side; >65535 refuses;
  empty page = `Delta::Noop` (documented no-op); empty selection
  refuses.
- `deleteAllInDocument`: one pre-flight sweep collects every
  top-level element id (000557d), then each goes through exact
  `object.delete` semantics (detach + registry remove + `Removed`
  inverse), folded into one `Delta::Group`. Group children ride the
  standing group-delete contract. Empty document = no-op.
- Tests: extents hug + origin move + undo byte-exact; grow=false
  clamp; empty-page Noop; oversize/missing-page refusals untouched;
  selection trim + empty-selection refuse; delete-all across pages +
  undo byte-exact + double-apply Noop; nested-group fold + undo.
- Registry 215→218. dom-parity: **245C/26P/0M/85N**, unresolved [].
- CI note: 12b5e40/d98764b perf-budgets failures were infra (fleet
  load 1.48× over a 3×-headroom ceiling; runner_lost_while_running) —
  8f2919d verified 7/7 green on the same code paths, local
  `xtask perf` 12/12 budgets ok.

## #117 — 2026-10-14 — loose-ends setters (decision 0047)

Batch: `filter.setEffectName`,
`slice.addSwapImageBehaviorFromPoint`,
`text.replaceButtonTextStrings`,
`text.replaceButtonTextStringsInInstances` (8.13).

- `setEffectName`: selection walker over `(CATs, INMs)` exact
  matches. Stale ids skip (animation-walker rule), spine-less kinds
  skip, non-matching skip — only real renames produce deltas, so a
  sweep matching nothing is `Delta::Noop`, not an error. Empty
  `new_name` refuses before any capture (000557d). New shared
  `object_effects` spine accessor.
- `addSwapImageBehaviorFromPoint`: gates in order — finite `where`,
  exactly one selected slice/hotspot, then the point hit. Hit test
  is topmost across all pages/layers/cells; rect/ellipse use the
  frame, polygon gets a real even-odd contour test (the frame is
  only its bounding box). A miss refuses — CS6's documented `false`
  return. The hit writes kind 2 (swap image, decision 0017 wire
  shape) plus `DTAv swapTarget` = source name (internal_name
  fallback; unnamed source writes the behavior with no key).
- Button-text pair: shared tail filters `SymbolType::Button` in the
  tail so both variants are safe by construction (a graphic instance
  in the selection never mutates its symbol's text). Walk = topmost
  text per definition frame (cells in order, elements reversed).
  Rewrite collapses runs to one run (single-run model —
  `uniformAttrs` recorded no-op, mirroring `text.setText`); empty
  text empties runs. All-variant sweeps `document.symbols`;
  instances variant reads `symbol_number` from the selection (empty
  selection refuses). Test fixture: button 7 / graphic 8 defs with
  one text each.
- Test-writing note: `instance_of` returns the FIRST instance in the
  registry — when two instances exist, select by scanning for the
  wanted `symbol_number`, not by helper order.
- fuzzdrive: 4 CALLS entries + arms (text rewrites mostly-set text
  with `uniform_attrs` flips, empty-string tail; filter rename wanders
  cat/old/new so most calls miss; swap point wanders on/off the
  field). Local fuzz 256×48 clean.
- Registry 218→222. dom-parity: **250C/21P/0M/85N**, unresolved []
  (five flips — the `AddSwapImageBehaviorFromPoint` CS6 alias carries
  with the lowercase verb).
- `setSliceAutonaming` stays P: the autonaming flag has no wire node
  (FILs carries only the naming label); fabricating one would
  violate no-format-fabrication.

## #118 — 2026-10-14 — on-path setters (decision 0048)

Batch: `text.setTextOnPathMode`, `text.setTextOnPathOffset`,
`text.reversePathTextDirection` (8.14).

- All three are CS6 selection verbs ("the selected text-on-a-path
  items", plural) over state the model already carries (TAMi / CHOf /
  PTHv — decision 0013's attach payload accepted both since the
  start). The P rows were model debt, not format debt.
- Shared `on_path_selection_walk`: guided text items participate;
  non-text, guide-less, stale skip; edit closures
  compare-before-write so re-applying current state is `Delta::Noop`;
  empty selection refuses.
- Mode gates 0..=3 (wire is the integer; the CS6 words map at the UI
  layer). Offset gates finiteness; negatives legal (glyphs drop off
  the rail end).
- Reverse rewrites the rail: contour points reverse, each point's
  primary/secondary swap — curve preserved exactly, travel flipped.
  Offset untouched (CS6 documents no compensation; the reversed
  start IS the effect).
- Tests: 3 (setters + undo byte-exact + same-state Noop; reverse
  anchors/handles asserted + undo; gates: empty selection, no-guide
  Noop + byte-stability, mode 7 refusal, direct NaN payload).
- fuzzdrive: 3 CALLS + arms (mode 0..5 crosses the legal range into
  refusal; offset ±100; reverse empty). Local fuzz 256×48 clean.
- Registry 222→225. dom-parity: **253C/18P/0M/85N**, unresolved [].

## #119 — 2026-10-14 — pixel-mask pair (decision 0049)

Batch: `bitmap.moveSelectionMaskBy`, `bitmap.adjustPixelSel` (8.15).

- Both verbs are pure `SelectionMask` plane ops — no image pixels
  move, no tiles load, undo rides `selection_delta` like the other
  `bitmap.select*` commands.
- `moveSelectionMaskBy`: byte re-read from (x−dx, y−dy), vacated area
  clears. Gates: no plane refuses (CS6 throws), zero delta refuses,
  |d| > 65535 refuses.
- `adjustPixelSel`: 3-4 chamfer distance field (two passes,
  O(pixels), radius-independent thresholding at 3×amount).
  expand/contract/border are threshold moves; smooth = opening then
  closing (specks go, pinholes fill). `whatToDo` is a serde enum of
  the four CS6 words (junk fails decode). Amount gates 1..=65535.
- Recorded choices: the image edge is NOT a selection boundary (a
  plane with no unselected pixels is contract's identity); the binary
  projection (`alpha != 0`) is what adjusts, so feather softness
  collapses to hard coverage.
- Test trap: the checker fixture is a 4×4 image and SelectRect spans
  the whole plane — a full plane has NO in-plane unselected pixels,
  so contract-1 is the identity (my first "ring" premise was wrong;
  probed, then rebuilt the smooth test around the identity + pinhole
  fill).
- Tests: 4 (translate + undo; move gates; expand/contract roundtrip
  + undo with chamfer-diagonal corner assert; border-empty-on-full
  plane + smooth identity + pinhole fill + kind/amount/no-plane
  gates). fuzzdrive: 2 CALLS + arms (kinds cycled, amounts cross the
  gate). Local fuzz 256×48 clean.
- Registry 225→227. dom-parity: **255C/16P/0M/85N**, unresolved [].

## #120 — 2026-10-14 — fuzz-found defect fixes

CI's fuzz job caught two refuted-calls-mutate-nothing violations that
local pre-push sweeps had missed (the local sweep base differed from
CI's).

- **Clipboard panic (d482b5d's CI seed)**: `capture_selection_with`
  pushed a root slot BEFORE capturing, but `capture_into` skips stale
  selection ids — roots and elements went out of alignment and
  `single_root` indexed an empty list. Fix: push the root only when
  the capture produced an element (index-aligned by construction);
  `single_root` reads through `.get()` and refuses on the impossible
  miss instead of panicking.
- **Animation-walk refusal-mutates (1024-seed sweep, seed
  5846161706420016621)**: `edit_animation_instances` resolved each
  selection id with a mid-walk `access::object(...)?` — a stale id
  errored AFTER an earlier member had already mutated and pushed its
  delta. Fix: the walker pattern (clone-before resolve, stale skips,
  then `get_mut`); no fallible resolution after any mutation. Audit
  of every other mid-walk `?` site (bake, convertToPaths, web,
  deleteAllInDocument) confirmed all pre-flight their fallible steps.
- **panic_hunt repro message corrected**: the CLI is
  `[seeds] [base-seed]` (seed = base ^ i*0x9E3779B97F4A7C15), and CI
  runs base 0x4A2DC0DE — the old message's `[seeds] [steps] [seed]`
  shape sent repro attempts into the wrong seed space, which is why
  the first CI fuzz failure initially "didn't reproduce". Local
  sweeps must use base 1242750446 to cover CI's space.
- Verification: 1024-seed CI-base sweep clean (both repro seeds
  included); engine suite green; clippy -D warnings clean; fmt clean.

## #121 — 2026-10-14 — paragraph setter trio (decision 0050)

Batch: `text.setTextParaIndent`, `text.setTextParaSpacingBefore`,
`text.setTextParaSpacingAfter` (8.16).

- The three dom verbs are pure model debt: `PINf`/`PSBf`/`PSAf`
  already map into the `TFSv` `TextStyle` on both sides (reader folds,
  writer emits in dictionary order); the `TXTv`-level mirrors stay
  bagged verbatim like the `FONs`/`PTSf` copies.
- One `paragraph_setter!` macro generates all three commands
  (id-addressed like `text.setAttrs`; `MissingTarget` for stale,
  `InvalidState` for non-text). Gate is finiteness only — negative
  (outdent/overlap) and zero (clear) are expressible pixels, recorded
  in decision 0050. Style slot materializes on first write; undo rides
  the whole-object capture.
- Test traps: JSON cannot carry NaN/inf (null slot = `BadPayload`) so
  the finite gate asserts on the typed surface (`Apply::apply` direct);
  a stale id raises `MissingTarget`; and `create()` reports
  `elements[0]` — the second create's id must be read off the cell
  tail before removing it.
- rustfmt oscillated on the multi-line `concat!` doc inside the macro
  (indent drifts every run) — collapsed to a single line.
- Tests: 2 (write+invert across all three; typed NaN/inf gate, not-
  text + stale refusals, negative/zero accepted). fuzzdrive: 3 CALLS
  + arms (pixel range straddles negative/zero/positive).
- Registry 227→230. dom-parity: **258C/13P/0M/85N**, unresolved [].

## #122 — 2026-10-14 — multi-run text model + markup verbs (decision 0051)

Batch: `text.setTextRuns`, `text.applyCharacterMarkup`,
`text.removeCharacterMarkup`, `text.applyFontMarkup`,
`text.removeFontMarkup` (8.17). The recorded M15 text gap closes.

- The model caught up to the wire: `TFSv` already documented the
  marker stream (style nodes between `TRNs` runs = the next run's
  changes; `UNDb` true begins / false ends). `TextRun` gained
  `changes: TextStyle` (`serde(default)` keeps old journals
  decodable).
- Reader fold: markers before the first run extend the base; between
  runs they accumulate into the next run's group; trailing markers
  (no following run) bag verbatim at their original child index and
  splice back byte-exact. Writer: each group re-emits before its
  `TRNs`. Marker-free files emit the same bytes as before — corpus
  round-trips untouched; a two-cycle byte-stability test pins the
  divergent shape.
- Restyle machinery (`overlay_changes` / `style_delta` /
  `restyle_runs`): groups restate, never unset — "inherit" is the
  absence of a node. `fwplain` is the only verb that clears the base.
  `removeFontMarkup` drops the field from the base and lets the runs
  re-diff to inherit (default = unset, recorded).
- `applyFontMarkup` enforces the dom's size grammar ("14pt"; a bare
  number refuses). `setTextRuns` pre-flights all attr gates before
  the first mutation; empty run list clears the text.
- Layout still reads the base style (recorded rendering stage, same
  as the para attrs) — wire and model carry divergence now.
- Test traps: the `create()` helper reports `elements[0]` (second
  create's id must be read off the cell tail — again); fwplain's run
  groups come out EMPTY (base cleared first → minimal stream); the
  placeholder-marker lint hook rejects "XXXpt" in prose — decision
  docs say "14pt" instead.
- Tests: +4 engine (markup streams+inverts, font markup grammar,
  remove clears base+runs, setTextRuns replace+gates) +1 doc
  (divergent round-trip, two-cycle byte-stable). fuzzdrive: 5 CALLS +
  arms (tags include an unknown one; size values cross the pt-suffix
  and positivity gates; setTextRuns builds 1-4 runs with occasional
  changed_attrs/initial_attrs).
- Registry 230→235. dom-parity: **263C/8P/0M/85N**, unresolved [].
  `xtask ci` all gates green locally; xtask also restamped the
  changelog (186 commits).
