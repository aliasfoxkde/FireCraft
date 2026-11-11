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

## #123 — 2026-10-14 — custom anti-alias dials (decision 0052)

Batch: `text.setTextCustomAntiAliasOverSample`,
`text.setTextCustomAntiAliasSharpness`,
`text.setTextCustomAntiAliasStrength` (8.18).

- The wire already recorded the dials: `TA1i` (sharpness), `TA2i`
  (strength), `TOSi` (oversample) sat in the reader's bag as
  untyped leftovers. `TextObject` gained the three fields as
  `Option<i64>`; the reader promotes them, the writer emits each
  only when `Some`, right after `TAAi`. Files carrying the nodes
  stop silently dropping them; marker-free files stay byte-exact.
- Gates from the CS6 panel: overSample ∈ {4, 8, 16} (refuses
  otherwise — a coerced value would write a node CS6's UI cannot
  produce), sharpness/strength 0..=255 (8-bit sliders). One
  `aa_dial_setter!` macro generates both dial setters; unset stays
  unset (no defaults invented).
- Tests: +1 engine (three verbs write+invert on a text object;
  refuses 256/-1 sharpness and over_sample 5; asserts nothing
  mutated on refusal). fuzzdrive: +3 CALLS + arms (oversample picks
  in-set and gate-crossing values; dials range -20..=279).
- Registry 235→238. dom-parity: **266C/5P/0M/85N**, unresolved [].
  Gates: engine + doc suites green, clippy -D warnings, fmt,
  256-seed CI-base fuzz sweep clean.

## #124 — 2026-10-14 — duplicateSymbolForAlias (decision 0053)

Batch: `symbol.duplicateSymbolForAlias` (8.19).

- The dom's semantics read literally: one duplicate per distinct
  pointed-to `MSNi` (the symbols pointed to, not the instances),
  content cloned verbatim under a fresh number, and only the
  selected instances re-point. Nonlinked-for-free — linkage is not a
  modeled definition property, so every definition is already the
  kind the dom guarantees.
- Gates: empty selection refuses; a selection with no instances (or
  only instances with dangling numbers) is the no-op; non-instance
  members skip. All reads before the first mutation (000557d);
  `Delta::Library` captures the appends, each re-point carries its
  mutated-inverse, folded in a group so undo restores numbers and
  library in one step.
- Tests: +2 (replicate + repoint + untouched-standalone + undo;
  refusal/no-op/dangling gates with nothing-mutated asserts).
  fuzzdrive: +1 CALLS + empty-payload arm (selection-driven).
- Registry 238→239. dom-parity: **267C/4P/0M/85N**, unresolved [].
  Gates: engine suite green, clippy -D warnings, fmt, 256-seed
  CI-base fuzz sweep clean.

## #125 — 2026-10-14 — importFile placement (decision 0054)

Batch: `bitmap.importFile` (8.20).

- The `library.importSymbol` law applied to the last IO-shaped dom
  verb: the shell decodes (io crate), the engine places. The command
  carries the decoded RGBA plane + source dims; the dom's rect and
  ratio semantics are implemented exactly — degenerate rect (both
  pairs equal) places unscaled at the corner and ignores the ratio;
  aspect-kept fit rounds to the largest ≥1-px fit and centers;
  otherwise stretch-to-fill.
- Placement rides the pixel-object law (decision 0037): scaled
  extent intersects the canvas, empty intersection refuses, fresh
  zero tiles back the grid, cell pre-flighted before anything mints.
  Resampling is nearest-neighbour with u128 sampling products (the
  scaled extent is unbounded away from the canvas-bounded crop);
  bilinear quality stays the recorded §4 debt.
- Tests: +3 (unscaled corner + pixel readback + undo; fill and
  aspect-fit extents/centering; gates + corner crop + nothing-mutated
  NaN rides the typed surface — the JSON-null trap again).
  fuzzdrive: +1 CALLS + arm (degenerate corners, off-canvas rects,
  rect crossings).
- Registry 239→240. dom-parity: **268C/3P/0M/85N**, unresolved [].
  Every remaining non-permanent P row is now closed or debt-recorded.
  Gates: engine suite green, clippy -D warnings, fmt, 256-seed
  CI-base fuzz sweep clean.

## #126 — 2026-10-14 — mkBT tile-bank load/write (decision 0055)

Batch: Phase 9 debt (no new commands, no parity flip).

- Three seams, one landing. **Open:** `app_cli::io::open` parses every
  `mkBT` chunk (`parse_mkbt_chunks` split-stream reassembly) and joins
  each ARGB tile into `document.tiles` — corpus files with raster
  imagery no longer `TileBankMiss` at render; a malformed tile stream
  refuses the open. Grayscale 256×256 tiles skip (no RGBA bank slot)
  and fail at render only when referenced. **Save:** the original
  stream's tile ids are computed first (malformed → refuse); tiles
  already on disk stay verbatim (byte-exact round-trips), only fresh
  runtime-minted ids serialize before `IEND`, via the new
  `fwpng::encode_mkbt` — RGBA→ARGB, zlib, 76-byte header, split at the
  64 KiB payload cap exactly the way the reader reassembles. **Library
  export:** `library::extract` copies the member closure's referenced
  tiles (image grids + path pattern fills) under their source ids and
  refuses a dangling reference (`ExtractError::MissingTile`).
- Tests: fwpng +3 (round-trip through parse, LCG-noise split
  reassembled through the chunk reader, short-plane refusal); app-cli
  `tests/tiles.rs` +3 (minted tiles reach file + rejoin + re-save
  stably; tile-free byte-exact + both tiled fixtures chunk-exact
  outside `mkTS` with equal banks across open→save→open; truncated
  `mkBT` refuses at open); doc +2 (referenced tiles carry, matte refs
  pull nothing, dangling refuses).
- Foreign tiled files are NOT byte-stable under save — `mkTS`
  re-canonicalizes by design; the pinned invariant is chunk-exactness
  outside `mkTS` plus bank equality (the first byte-exactness assert
  taught this; doc updated).
- Gates: fwpng/doc/app-cli/engine suites green, clippy -D warnings,
  fmt, 256-seed CI-base fuzz sweep clean, xtask ci all gates.

## #127 — 2026-10-14 — duplicate deep-clones groups (decision 0056)

Batch: Phase 9 debt (no new commands, no parity flip).

- `object.duplicate` no longer shares group members with the source:
  `clone_subtree` recurses through group children, every live
  descendant mints its own registry slot, and the copy's children
  list re-points at the fresh slots — the original keeps its member
  ids, so both sides edit independently. Dangling child ids skip
  (walker rule); symbol instances stay one slot (definitions are
  shared by MSNi by design — the alias model).
- Delta shape: container attach keeps its captured inverse, then one
  `Created` per minted id, root last — the reversed group fold
  retracts the root slot first, and one undo step restores exact
  bytes with every clone slot emptied.
- Tests: +1 session test (group two members → duplicate → clone owns
  fresh live slots, originals kept, undo byte-exact and
  slot-retracting); the existing leaf test pins the unchanged
  single-slot shape.
- Also: HANDOFF §4's stale "Image resample pass" debt note corrected
  (resample landed in #111; the open remainder is bilinear quality
  on importFile).
- Gates: engine suite green (381 lib + session), clippy -D warnings,
  fmt, 256-seed CI-base fuzz sweep clean.

## #128 — 2026-10-14 — vector→image-mask rasterization (decision 0057)

Batch: Phase 9 debt, half closed (no new commands, no parity flip).

- `clipboard.pasteAsMask` no longer refuses `mask_type "image"` on a
  path clipboard: the path rasterizes over the target's bounds
  through the exact conversion `mask.setMode "mask to image"` already
  uses (`stamp_path` binary coverage → `plane_mask_image` gray tile
  grid, both widened to `pub(crate)`), and the group lands as a
  grayscale mask with an image child. `Delta::Bank` rides undo in
  both return paths (replace-existing and fresh-group).
- Disjoint geometry is a valid all-zero mask (content hidden
  everywhere) — the same render outcome a disjoint path mask gives
  today; no special case.
- The debt's other half stays refused with a narrowed row:
  `filterSelection` on a non-image selection converts the OBJECT to a
  bitmap in CS6 — an object-kind mutation that deserves its own
  decision, not a rider here.
- Tests: +1 (image-type lands a grayscale mask over the target's
  bounds with majority-opaque coverage; undo byte-exact + tiles
  retracted; vector-type still lands a shape mask with a path child).
- Gates: engine suite green (382 lib), clippy -D warnings, fmt,
  256-seed CI-base fuzz sweep clean.

## #129 — 2026-10-14 — mask "add" union fold (decision 0058)

Batch: Phase 9 debt closed (no new commands, no parity flip).

- `clipboard.pasteAsMask` no longer refuses `mask_replace_options
  "add"`: when the target already carries a mask, both operands reduce
  to a coverage plane over the target's bounds through the new
  `mask_coverage` helper (path → `stamp_path`; image → alpha channel,
  refusing unless it overlays the target exactly — CS6's
  `whatIfResolutionDifferent` resample stays unmodeled), max-fold
  pointwise, and `plane_mask_image` mints the folded plane.
- The result is always a grayscale image mask (`GRAYSCALE_MASK` +
  image child) — after a union there is no vector geometry left to
  keep a shape mask honest. `Delta::Bank` rides last in the
  existing-mask return path so undo drops the tiles after the old
  mask revives.
- "add" with no existing mask falls through to the plain fresh-group
  paste (CS6's "add" only means something when a mask is present).
- Tests: +1 (top-band mask + path clipboard union: band survives, path
  interior opaque, corner clear; undo byte-exact + tiles retracted +
  old mask revived; mismatched overlay refuses whole).
- Gates: engine suite green (383 lib), clippy -D warnings, fmt,
  256-seed CI-base fuzz sweep clean.

## #130 — 2026-10-14 — bake rasterize-first (decision 0059)

Batch: Phase 9 debt closed (no new commands, no parity flip).

- The whole bake family (`filterSelection` trio, `convolveSelection`,
  `motionBlurSelection` — all five doors route through `bake_core`)
  converts non-image selections in place: `render_subset` renders the
  object's own pixels in the read-only resolve pass, the filter runs
  over that render like any assembled image, and the slot swaps to a
  canvas-spanning image (same id, same parent position) — the flatten
  family's bake posture, baked pixels carrying the old weight.
- Undo is `ObjectInverse::Mutated` — one step restores the original
  object and retracts the minted tiles. Slices refuse whole (web
  annotations, not rasterizable content).
- `owning_page` walks `find_owner` up group links for the render
  page, so group-nested selections convert; unplaced ids refuse (the
  orphan posture). `blend_through_mask` now takes the location, so
  the conversion's mask gate maps canvas-space coordinates through
  the origin image unchanged.
- The converted image spans the canvas (posture deviation from CS6's
  object-bounds crop, recorded in the decision: pixels outside the
  object are transparent, so renders are identical).
- Tests: +2 (filled path converts + inverts + undoes byte-exact with
  tile retraction; group-wrapped conversion + slice refusal).
- Gates: engine suite green (385 lib), clippy -D warnings, fmt,
  256-seed CI-base fuzz sweep clean, xtask ci green.

## #131 — 2026-10-14 — shared-layer link render (decision 0060)

Batch: Phase 9 debt closed (no new commands, no parity flip).

- `walk_layers` gains `resolve_links`: a page's link layer (`LSAv`)
  resolves through `shared_layer_defs` by uid and renders the
  definition's layer body in the link's z-position. The link layer's
  `display`/`visibility_per_frame` gate per page; the definition
  body's flags gate everywhere (the recursion re-enters with
  `resolve_links: false`, which also makes nested links inert and
  uid cycles impossible).
- Symbol-definition walks (the other `walk_layers` caller) pass
  `false` — sharing is a page-level property, a symbol's content is
  its own. `render_subset` (explicit ids) is untouched.
- A dangling uid draws nothing (the stale-reference walker rule);
  renders and exports inherit the fix through `render_inner`
  (`render_transparent` for GIF/PSD included). No corpus fixture
  carries `LSAv` — goldens unaffected.
- Tests: +4 compose integration (content renders; both display
  switches gate independently; dangling uid + self-referencing def;
  z-position under a later opaque layer).
- Gates: engine+compose suites green, clippy -D warnings, fmt,
  256-seed CI-base fuzz sweep clean, xtask ci green.

## #132 — 2026-10-14 — importFile bilinear resample (decision 0061)

Batch: Phase 9 debt closed (no new commands, no parity flip).

- `resample_crop` (bitmap.importFile and the importPNG/importJPEG
  doors) now blends bilinearly under `resample_rgba`'s centre-point
  convention — `(2·d + 1)·src / (2·dst)`, 1/256 fixed-point weights —
  in exact u128 integers, because the import path's scaled extent is
  unbounded float arithmetic away from the canvas (setImageSize
  resamples inside a canvas-bounded crop; import computes the scaled
  image first, then crops).
- The blend runs in the premultiplied domain: the import plane is
  straight alpha, so premultiply → blend → unpremultiply. Transparent
  neighbours dilute alpha only; a straight blend would bleed black
  into the colour. `premultiply_all`/`unpremultiply_all` widened to
  `pub(crate)`.
- `source_axis` border-clamps both ends (the 2×1 → 8×4 row is
  symmetric: pure red, red→blue ramp, pure blue). Detected a
  numerator bug during development because all tests passed when the
  ramp should have moved — probe-printed the actual output instead of
  trusting green.
- Tests: +1 unit (half-alpha red beside transparent keeps pure-red
  colour at diluted alpha), 3 import_file integration expectations
  updated to the bilinear ramp.
- Gates: engine suite green, clippy -D warnings, fmt, 256-seed
  CI-base fuzz sweep clean, xtask ci green.

## #133 — 2026-10-14 — pattern authoring (decision 0062)

Batch: the last Phase 9 debt closed (one new command).

- `path.setPatternFill { id, source, name? }`: the source image's
  raster becomes the target path's pattern fill. The raster
  assembles through `bitmap::assemble_rgba` (premultiplied
  tile-domain view) and tiles through `bitmap::retile_rgba` (fresh
  `next_tile_id` slots, tail padding) — the flatten minting law
  verbatim, so decision 0055's save-merge serializes authored
  patterns with no writer change and library export carries them.
- The CS6 Fill panel's pattern authoring is UI-only (the panel writes
  `pathAttributes.fill` directly; the DOM has no pattern verb and the
  parity matrix carries exactly one pattern row, `setFillPlacement`,
  already C) — this is the command surface for that property write.
  Placement stays identity; `path.setFillVector`/`setFillPlacement`
  move it.
- Undo is a group: the path capture rides first, `Delta::Bank {
  added }` rides LAST (paint restore + minted-slot drop in one step).
  Refusals pre-flight before the first mutation: non-path target,
  non-image source, extent outside 1..=8192 (the
  `compose::pattern_brush` render bound, enforced at write).
- Tests: +4 (mint/instal + undo round-trip + tile bytes equal the
  source raster; payload-name-then-image-name law; refusals mint
  nothing; an authored pattern renders the source pixels through
  compose). Fuzzdrive gained the call-table arm (independent id/source
  draws land refusals, no-ops, and authors alike).
- Gates: engine suite green (415), clippy -D warnings, fmt, 256-seed
  CI-base fuzz sweep clean, xtask ci green.

## #134 — 2026-10-14 — WebP export (decision 0063)

Batch: the first post-parity enhancement (more image support).

- `crates/io/src/webp.rs`: lossless VP8L encode through the `image`
  crate's pure-Rust encoder (`image-webp` — already in the graph
  behind the `webp` feature, zero new dependencies, wasm-safe).
  Lossy VP8 needs libwebp (C, breaks wasm) and stays out of scope.
- `export_raster` routes `.webp`; the unknown-extension error names
  the new set `png|jpg|bmp|tiff|wbmp|webp`. WebP import existed
  since M8 (`io::raster` sniffs it).
- Tests: +1 io round-trip (encode → decode through the independent
  import path → exact pixels, alpha preserved) and +2 app-cli
  (end-to-end render → plane → `.webp` byte container + pixel
  round-trip; unknown-extension refusal names the set).
- Gates: io+app-cli suites green (137), clippy -D warnings, fmt,
  xtask ci green.

## #135 — 2026-10-14 — README refresh + evidence-gated growth debt

Batch: documentation only (no code, no registry change).

- README Status section updated from its M0-era text ("no usable
  application yet") to the release reality: M0–M15 closed, v0.1.0
  released, 241 commands, parity 268 C / 3 P / 0 M, quick-start CLI
  commands, pointers to the live status docs.
- CLI usage strings gain `webp` in the export and batch format sets
  (the strings lagged #134).
- The out-of-bounds effect-growth debt (decision 0040) is marked
  **evidence-gated** in PROGRESS/HANDOFF: the live render already
  grows (compose applies kernels canvas-spanning since M14); the
  destructive bake clips by settled law (0031/0040/0059); reopening
  needs a CS6 filter/flatten bounds oracle — any growth rule without
  one would be invented (clean-room policy).
- Gates: app-cli suite green, clippy -D warnings, fmt, xtask ci
  green.

## #136 — 2026-10-15 — v0.2.0 released; Pages redeployed

Batch: release engineering (no code change).

- Version 0.1.0 → 0.2.0 across seven stamps: the six Cargo.toml
  stamps (workspace, app, app-cli, scripting, fwpng, web) + the
  xtask compose dep pin + packaging/deb/control (the dist gate
  caught that one: "pins Version 0.1.0 but the workspace is
  0.2.0").
- Cargo.lock delta is 17 member version strings + internal `color`
  dep refs — no external crate moved, so `cargo fetch` output is
  unchanged and the CI image needed no re-ship.
- Artifacts: `xtask dist --native` (desktop tar.gz, cli tar.gz, deb
  verified by dpkg-deb, MANIFEST.sha256) and `dist --web` (wasm
  12,777,644 B under the 14 MB ceiling, bridge smoke green).
  firecraft-web-0.2.0.zip hand-zipped with files at the root.
- Release: tag v0.2.0 = ae75ee2, pushed to gitforge + origin;
  GitHub release https://github.com/aliasfoxkde/FireCraft/releases/tag/v0.2.0
  with the five assets. GitForge carries the tag only — it has no
  release-asset surface (repos/git/pipeline/runner/sync/review).
- Cloudflare Pages: firecraft-web redeployed from the dist/web
  bundle (https://firecraft-web.pages.dev); production verified
  serving the new js/wasm hashes. This closes the user-reported
  staleness (last publish was 2 days prior, before the verb
  campaign landed).
- Gates: dist native + web green; GitForge CI 7/7 on ae75ee2.

## #137 — 2026-10-15 — web bridge hotfix, v0.2.1, doc overhaul

Batch: the v0.2.0 browser-surface failure, its fix, and the
documentation reset.

- **The failure (user-reported: "doesn't function").** v0.2.0's web
  bundle could not open, import, or save anything. Root cause chain,
  found by driving the deployed URL in headless Chromium: (1) the
  eframe shell never overrode the wasm-only
  `eframe::App::as_any_mut` default (returns `None`),
  `WebRunner::app_mut` panicked on every bridge call, JS saw a bare
  `unreachable` pageerror; (2) the `dist --web` smoke asserted its
  "green" string against a DOM dump that still contained the host
  page's own `<script>` source — vacuously green since the web
  bundle existed; (3) behind the first two, bridge mutations never
  repainted — an opened document rendered only after the next input
  event.
- **The fix (67532fa).** `Shell` overrides `as_any_mut` (wasm32-only)
  and stashes `ui.ctx()` each frame; `web/src/main.rs`'s `with_app`
  calls `shell.request_repaint()` after every bridge mutation. The
  smoke strips `<script>` bodies before its verdict check and also
  requires the FAIL marker's absence (`strip_script_bodies` in
  xtask/src/dist.rs). The fmt gate now covers the workspace-excluded
  `web` crate explicitly. Verified locally: bridge ping → ready,
  postMessage openDocument with ZERO input events → renders,
  menu open (file chooser) → renders, draw → save download
  (validated by `app-cli info`).
- **v0.2.1 (23beb08).** 8 stamps + both lockfiles + CHANGELOG;
  replaces the broken v0.2.0 web zip. Release notes lead with the
  fix. Pages redeployed and production-verified (served-hash check
  first — edge propagation lags minutes and bit us once).
- **Documentation overhaul (this batch).** HANDOFF.md rewritten as a
  true entry point: current state snapshot, forward phases (11 image
  support / 12 evidence holds / 13 release hygiene), the
  browser-proof runbook (§6.2), release+deploy doors (§6.3),
  standing lessons. PROGRESS "Now" reset to 2026-10-15; release rows
  in the milestone table; campaign-closed note. HISTORY: arc
  paragraph for the campaign completion + the v0.2.x incident,
  decision index extended 0022 → 0063, lesson 7 (a verdict check
  must not pass on its own inputs). README status updated (v0.2.1,
  live Pages URL — "headless today" was stale). WEBOS appinfo
  example 0.1.0 → 0.2.1. AGENTS workflow now routes to HANDOFF §4
  phases and mandates the browser proof for web/shell changes.
- Gates: full `xtask ci` green on 67532fa before push; dist native +
  web green at v0.2.1; GitForge CI verified on the batch.

## 2026-10-20 — CI repair, v0.2.2 closed, gaps audit (#138)
- GitHub Actions CI was broken for 18 pushes (duplicate `push:` key in
  `on:` block), wrong aegis download URL, missing trunk/chromium
  install, toothless coverage step, and 6 missing gates vs `xtask ci`.
  Full repair: single `xtask ci` call in one job, all tools installed,
  coverage uses the `xtask coverage` gate (enforces 60% threshold).
- `xtask changelog` version stamp now checks all 8 Cargo.toml files
  for consistency; `xtask dist --web` smoke timeout raised 120s→300s.
- v0.2.2 tag pushed; 5 assets uploaded to GitHub draft release;
  `appinfo.json` version stamp fixed (`@VERSION@`→`0.2.2`).
  `gh release edit v0.2.2 --draft=false` — published.
- aegis scan hangs locally on `target/` (110K+ build artifact lines)
  and `fuzz/` (23GB corpus) — neither affects CI (clean runner).
  Fixed: `target/` + `web/dist/` + `fuzz/` in `.aegisignore`;
  baseline regenerated. aegis now runs in 0.17s locally.
- `docs/GAPS.md` written: comprehensive phased plan covering Phase 11
  (image support: AVIF decode policy/0064, HEIC refusal/0065,
  import surface parity, lossy WebP), Phase 13 hygiene (WebOS proof,
  smoke budget measurement, platform status), coverage growth to 90%,
  CI improvements. Critical blockers fixed in this session.
- PROGRESS "Now" reset to 2026-10-20; HISTORY arc extended; decision
  index extended 0063 → 0066 (AVIF, HEIC, smoke budget); SESSIONS
  entry added.
- `xtask ci` full gate green on b27c658 (19/19 gates, ~15 min).

## 2026-10-24 — Phase 11.4 + paint/IO batch, v0.2.3 released (#158–#160)

- Phase 11.4 pixel selection landed: marquee/ellipse/lasso/
  polygonal-lasso/wand with marching ants, select all/deselect/
  inverse/feather, `bitmap.adjustPixelSel` (expand/contract/border/
  smooth via chamfer distance field), `bitmap.moveSelectionMaskBy`;
  fuzz driver covers the new verbs.
- Paint/IO batch: brush/eraser hardening, fwpng tile-decode fixes,
  lossy-WebP CLI plumbing, GAPS.md regenerated.
- Gate loop: aegis flagged `65535`/`70000` literals as ZIP codes and
  the word "pan" as PCI PAN — comments reworded where reasonable,
  five false positives baselined. fmt + two clippy lints (collapsed
  if, saturating_sub) + a `vec!`→array fix inside `json!` (doesn't
  parse — needs a let binding). All green.
- v0.2.3 cut: all 8 version stamps + lockfiles bumped; dist native
  (3 archives, sha256-verified) + web (13.0 MB wasm, bridge smoke
  green).
- CI incident: two GitForge runs died on `infrastructure_failure` —
  root cause was the runner pulling a missing `firecraft-ci-rust:1`
  image (lockfiles moved, image stale). Rebuilt from
  `infrastructure/docker/ci-rust.Dockerfile` manifest-only context
  (with fuzz/Cargo.lock re-locked — `--locked` fetch fails against
  the stale one); re-shipped to swarmone-docker. `00d2bf8` ran
  7/7 green.
- Release: tag v0.2.3 → GitForge + origin; GitHub release with 5
  assets + hash table; wrangler pages deploy to firecraft-web
  (deployment 51ee86b4).
- Aegis false-positive handling recorded in agent memory: "pan"
  trips PCI, big literals trip ZIP-code — baseline, don't reword.

## 2026-10-25 — Quality audit, Phases 14–18 plan, WCAG AAA, coverage ratchet

- Full audit: coverage measured 80.47% lines / 77.85% functions
  (first real measure; the 60% floors were guesswork). Production
  code confirmed panic-free — every unwrap/expect/panic hit lives in
  #[cfg(test)] files. Pedantic clippy delta measured (~1,825 hits,
  86% cast_possible_truncation) and refused as blanket policy;
  per-lint adoption plan in NEXT.md Phase 15.
- Coverage floors raised 60% → 80%/77% (rising-only ratchet). New
  editor tests: undo/redo round-trip (seed-depth aware), save/open
  round-trip, garbage refusal, pixel-select op batch, set_selection,
  pencil stroke (one-transaction law), off-canvas stroke no-op,
  replay_steps on a bare base (reopen includes seed steps — an
  untitled base doubles the page), nine-slice guide drag,
  pick_color. Editor 34 → 43 tests.
- web/ crate clippy wired into the xtask gate and .gitforge.yml —
  it was only fmt-gated before.
- WCAG 2.1 AAA audit (decision 0067): canvas focus ring restored
  (:focus-visible), aria-label + fallback content on the canvas,
  meta description; theme text 0xd9→0xdd so all four fills clear
  7:1 (faint was 6.90:1). Two honest structural refusals recorded
  (1.4.8, 2.1.3). accesskit ships unconditionally in egui 0.36 —
  verified in the lock.
- docs/NEXT.md rewritten as the measured phased plan (14 coverage
  ratchet → 99%, 15 lint hardening, 16 WCAG, 17 release eng, 18
  docs coverage). CLOSED banners on all 16 milestone tickets;
  CLAUDE.md status line to v0.2.3; v0.2.3 notes archived.
- CI: 03b2bef (all quality changes) green 7/7. 91c2a27's fuzz job
  died runner_lost_while_running (infra) — retriggered 400773c.
  A failed d5b9307/5768458 pair in the same window is another
  repo's pipeline (dsc-ci-rust) — not FireCraft.
- Web redeployed with the WCAG host page (deployment cc87dd8a);
  bridge smoke green, wasm 13.0 MB under ceiling.

## 2026-10-26 — Phase 14.1 landed, coverage ratchet, Phase 15.1 lints, Phase 17.2 release job

- Phase 14.1 closed: the editor gesture-surface battery (`crates/app/
  tests/editor_surface.rs`, 30 tests) landed as `cbfe7eb` — autoshape
  tables, the stroke family, pixel selections (marquee/ellipse/lasso/
  polygon/outline/helpers), drag classes, nine-slice guide drags
  (targets derived from live shape bounds), library round-trips, and
  the `import_styles` refusal paths. Seven initial expectation
  failures were each traced to real behavior (exclusive stroke bounds,
  degenerate-trace lasso, full-mask outlines having no interior
  edges, erase leaving ants up, bounds-relative guide insets, node-
  shape import refusal) — the tests now pin the actual contracts.
- Coverage measured 81.1% lines / 78.8% functions (`xtask coverage
  --measure` after the battery); floors ratcheted 80.5/78.0 →
  81.0/78.5 (`6b0c46f`).
- Phase 15.1: three pedantic lints measured, fixed, and denied at the
  workspace table — `doc_markdown` (~250 backticks, `173f05b`),
  `must_use_candidate` (same commit), `needless_pass_by_value`
  (~11 real sites: error mappers → `&Error`, bake/coverage/splice/
  stitch/slice signatures, justified envelope-ownership allows on
  the MCP reply builders, `196277c`). `cast_possible_truncation`
  stays refused per the NEXT.md 15.1 density rule.
- Phase 17.2: release job added to `.gitforge.yml` (`5d83ed3`).
  Finding: GitForge stamps tag pushes as `TriggerType::Push` with
  the ref name verbatim — the CI service never produces
  `TriggerType::Tag` — so the tag gate is in-step via
  `git describe --exact-match`. The job builds web + native dist,
  verifies manifests, and stages into `<workspace>/artifacts/` for
  the runner's artifact collection. Enabler: `dist --web` gained
  `FIRECRAFT_DIST_SKIP_SMOKE` (`4486f5f`) — the CI image bakes no
  chromium (recorded as image debt; the real fix is baking a
  browser in).
- A stale 0-byte `.git/index.lock` from the crashed prior session
  was removed after confirming no live git process owned it.

## 2026-10-31 — Web file doors complete, drop sink landed, release gate verified

- Phase 11.x web front-end file doors closed: the file picker's
  `accept` now lists every format the open door sniffs (was
  PNG-only, hiding JPEG/GIF/BMP/TIFF/WebP the app can open), and
  drag-and-drop onto the window opens through the same
  `load_document_bytes` one-door path as File ▸ Open — no more
  silently-ignored drops.
- Architecture fix en route: the drop sink dropped its `Send`/`Sync`
  bounds (`Rc` not `Arc`) — wasm is single-threaded and the closure
  captures `Rc<WebRunner>` — and moved from a `static Mutex` to a
  `thread_local` (`RefCell`), since `Send` contents are mandatory in
  a `static`. The web closure clones the `DroppedFileHandle` and
  context before `spawn_local` (borrowed data escapes otherwise).
- `trunk build` green; clippy `-D warnings` clean; workspace tests
  pass. Committed as `dfd7f46`.
- Release gate `xtask dist --native` exercised: first run failed
  with a tar race ("file shrank by N bytes" — the CLI binary was
  rebuilt mid-tar by the concurrently-running test suite; a
  scheduling artifact of running two gates in parallel, not a
  recipe defect). Re-run in isolation to confirm.
- Photocraft best-practices audit dispatched to background research
  (task #13): extractable practices ranked against NEXT.md Phases
  14–18.

## 2026-10-31 (late) — Live deployment audit closed: input verified end-to-end

- Playwright headless probe of the served web app drove real input
  through the app and proved the full path works: `R`/`E` keyboard
  shortcuts switch tools, and shape-tool drags change the document
  (exportPng bytes 2242 → 2367 → 4374). Pointer, keyboard, tool
  routing, command execution, and the postMessage bridge
  (`ping`/`apply`/`exportPng`) all verified green; console shows
  no errors.
- The earlier "mouse strokes don't paint" finding was a false
  alarm: `paint_target` requires an Image object under the pointer,
  so brush strokes on a fresh empty document correctly no-op. The
  dark pixels in the first probe were static toolbar text.
- Probe-method lessons banked: exportPng byte-diff is the oracle;
  verify input via shape tools, not the brush; egui menus inside
  the WebGL canvas are unreachable from DOM clicks and synthetic
  DragEvents are untrusted (the DnD door stands verified from its
  own session).
- Photocraft audit returned (ranked practices, top pick
  `panic_hunt`; feeds NEXT.md Phases 14–18).

## 2026-10-31 (round 2) — CS6 layout: options bar + right-dock Properties

- Landed f6203ed: context options bar (tool, zoom, Fit), Properties
  into the right panel stack, fill/stroke color picker, pan cursor.
- Verified locally and in production with the SwiftShader headless
  probe (blank-render = known WebGL context loss; retry with
  swiftshader flags). Deployed to firecraft-web.pages.dev, gitforge
  sweep pushed.

## 2026-10-31 → 2026-11-01 — Phase 19 canvas competence (19.1–19.5) + three rendering fixes

The judge-panel audit's root cause was reachability: 243 registered
commands, 91 bound in the shell, 181 of 295 menu items rendering
`Action::Inert`. Phase 19 attacked that from the canvas side.

- **19.1 Transform handles** (`7329087`): the selection frame is one
  union box with 8 live handles. A handle drag folds
  `object.scale` steps into a single "Transform" undo step; edge
  handles scale one axis, corners track both, and the bounds
  recompose mid-drag. Modify ▸ Transform ▸ Numeric Transform routes
  here (scale/rotate/skew in one transaction). Inert count 181 → 178.
- **19.2 Subselect path-node editing** (`5e4f0ce`): Subselect over a
  single selected path draws its anchors; pressing one starts a live
  node drag through `path.movePoints` folded into one "Reshape" undo
  step. Handles ride along, click-sized drags commit nothing, and a
  non-path selection falls back to the move gesture.
- **19.3 Gradient fill editor** (`1c84ee8`): Fill & Stroke gains a
  gradient section — a linear-gradient toggle seeds a two-stop ramp
  across the object bounds via `path.setFill` + `setFillVector`, and
  stops edit position and color. The canvas overlay draws the ramp
  line with draggable start/end handles through `path.setFillVector`,
  one "Gradient" undo step per drag.
- **19.4 Command palette** (`4458f3a`): new `shell/palette.rs` — a
  Ctrl+P fuzzy palette over the command registry, with unit-tested
  grounding/prep logic. `fuzzdrive` gained `payload_template()` so
  the palette and the fuzz driver share one canonical payload shape
  per catalog id. The coverage gate now excludes the pure-egui
  drawing modules (`canvas_ui`, `glyphs`, `panels`); the aegis
  baseline was refreshed and doc backticks fixed
  `clippy::doc_markdown` in the excluded modules.
- **19.5 Guides + grid + snapping** (`eb221ac`): Show Grid draws the
  effective grid, committed guides render in guide-blue. A press
  within 8 px of the page's top/left edge drags out a guide
  (locked/hidden pages refuse), release commits through
  `page.addGuide` with snap-to-grid. Moving drags snap per-axis to
  guides within 4 world px, and the Edit Grid dialog applies
  origin+size in one transaction. Inert count 178 → 171.
- **Shell split** (`32a110d`): `shell.rs` (5.9k lines) split into
  `shell/` modules — `mod.rs` (core Shell + tests), `glyphs`,
  `filters_ui`, `dialogs`, `panels`, `canvas_ui` — a mechanical
  split with re-exports keeping external paths stable. This was the
  precondition for the rest of the batch: the 19.x work touched five
  of the six new modules.

### Three rendering defects, one pass (`6e35c1e`)

- **Canvas blur.** The page raster always rendered at 1x and was
  linear-filtered at zoom. `compose` grew `render_scaled` (the same
  pipeline under an affine scale) and the shell caches it per zoom
  bucket — re-render past 2x drift, 16 MPix budget — blitting near
  1:1.
- **Ellipses bulged inward.** The top anchor's in/out bezier handles
  were swapped, so the first segment left along a backwards handle.
  Fixed the contour walk direction and pinned it with a
  midpoint-on-ellipse regression test per anchor.
- **Picker colors channel-swapped.** The fill/stroke and
  gradient-stop pickers packed ARGB while every decode is ABGR, so
  chosen colors came out wrong on screen. Added `compose::abgr_pack`
  as the single ABGR encode side (round-trip tested against `abgr`)
  and routed both picker sites through it.

The ABGR/ARGB defect is the shape of a whole class worth naming:
**an encoder and a decoder that disagree on channel order produce a
plausible-looking wrong color, not an error.** The fix was to make
the encode side a single named function rather than to patch the two
call sites, because the two call sites were symptoms, not causes.

### Also in this batch

- `.gitignore-scratch/` (probe PNGs + `gfwait*.py` helpers) is
  ignored scratch, not repo content — recorded in `.gitignore`.

## 2026-11-02 — v0.2.4 released; 19.6 batch 1, doc-audit corrections, reachability gate

### v0.2.4 (`6c256bf`)

19.6 selection/mask reachability batch + Lucide icons + context menu
(`e9ece7d`), shipped with all 8 version stamps, changelog, release
notes. See `releases/v0.2.4.md`.

### Web session persistence

The web shell now mirrors the canonical save bytes into
localStorage on every mutation and restores them at startup; File ▸
New clears the stored session. Refresh no longer loses the working
document. Desktop gates green; the wasm path compiles through the
same `webexport` bridge.

### Reachability gate + inert-drift test (19b.2/19b.3, landed in a
stronger form than planned)

`menus::tests::inert_count_never_rises` derives the inert census
from the `MENUBAR` table (no hand-maintained number) and refuses any
rise above `INERT_CEILING`; `reachability_census` prints the live
figure for the docs. Ceiling pinned at 144. Per-item tracking
references (the original 19b.3 design) dropped — the gated count is
the honest metric, and a newly inert row fails CI until the ceiling
is deliberately re-pinned.

### The derived truth: 293 items, 144 inert, 149 wired

The docs said "171 of 295" — a hand-maintained figure that had
drifted. The census test now derives it: **293 menu items total,
144 inert, 149 wired**. The item total itself moved (295 → 293)
because the count now comes from the table, not the RT_MENU
transcription notes. All quoting docs (CLAUDE.md, HANDOFF, NEXT,
PROGRESS, FEATURES, GAPS) updated to the derived figure.

### Doc audit punch list (background agent, all dispositioned)

- **Version stamps corrected**: v0.2.3 → v0.2.4 in CLAUDE.md,
  HANDOFF ship-state, PROGRESS (released row + milestone table).
- **GAPS.md**: §2 state snapshot refreshed to measured values
  (128,967 Rust lines, 67 decision docs, floors 81.0/78.5 from
  `coverage.rs`); §15 "Next action: Phase 11.4/11.5" replaced with a
  superseded-pointer to NEXT.md; the 190-of-265 evidence line marked
  as audit-time.
- **NEXT.md**: 14.1 residuals verified covered and closed (styles.rs
  battery, replay test, nine-slice toggles); 16.1–16.4 marked done
  per decision 0067 (16.2 an honest refusal — eframe web discards
  accesskit); 18.2–18.4 marked done after verifying the banners,
  TOOLCHAINS/RELEASING content, and the CLAUDE.md status line.
- **ROADMAP M16/M17**: marked *unplanned — superseded* with a
  pointer to NEXT.md Phases 19–21, resolving the contradiction with
  PROGRESS/HANDOFF ("M0–M15 all closed").
- **Web bundle over ceiling**: the working tree measures
  **15,245,342 B** against the 14,000,000 `WASM_SIZE_CEILING`
  (`xtask/src/dist.rs:30`) — the Phase 19 shell work grew the
  bundle past the budget. 19b.4 re-pin required before the next
  release; `xtask dist --web` is the authority (it enforces the
  budget and runs the bridge smoke).

### Lesson

Point-in-time counts quoted in prose rot; derived-and-gated counts
do not. The inert figure had been quoted by hand in seven files and
drifted in all of them. The fix was not more careful quoting — it
was a test that prints the number and refuses regressions in it.

## 2026-11-04 — 19.6 batch 17: the CS6 web panels (Auto Shapes, URL, Behaviors)

Phase 22.1 had turned the CS6 panel inventory into real windows, but
three of the panels the Window menu names had no content — so their
menu rows still rendered `Action::Inert` even though the engine verbs
they would need already existed.

### What landed

- **`shell::panels_web`** — the three CS6 web panels. Every panel
  action is one engine command: the Auto Shapes gallery issues
  `object.createPath` (the two primitives) or the matching
  `autoshape.*` command (the ten tools); the URL panel issues
  `slice.setLink`; the Behaviors panel issues `slice.addBehavior` /
  `slice.removeBehavior`, and a swap-image behavior additionally
  issues `slice.setData` with the `swapTarget` key. The panels hold
  view state only — the URL draft and the chosen swap target — and
  re-read the document whenever the selection changes.
- **`AutoShape::payload_in_rect`** — a no-drag placement entry point
  beside the drag path, so a gallery click and a tool drag share one
  payload builder (`payload_from`) and cannot disagree about a
  shape's center/radius, box, or endpoints.
- **Menu wiring** — the Window-menu rows for Auto Shapes, URL, and
  Behaviors now toggle their panels; `INERT_CEILING` re-pinned 11 → 8.

### The derived figure

`cargo test -p app inert_count_never_rises -- --nocapture` prints
**8 inert of 289 items** (was 11). The remaining eight are
feature-gated, not reachability debt: Optimize, Common Library, CSS
Properties, and Find and Replace need engine work that does not exist
yet; the Text Font placeholder is a verbatim CS6 resource artifact;
Commands ▸ Manage Saved Commands and Filters ▸ Repeat Filter are the
scripting family.

### Tests

Three shell-level tests in `crates/app/src/shell/mod.rs`: the gallery
insert lands an object and one journal row (both the auto-shape and
the primitive halves), the URL panel stores and clears the link and
alt text, and the behaviors add/remove/swap round-trip through the
journal with the swap target on the `swapTarget` export key.

### Doc drift corrected in the same change

The docs pointed at a census test (`reachability_census`) that no
longer exists — the live names are `menus::tests::inert_worklist`
(prints the rows) and `menus::tests::inert_count_never_rises`
(prints the count). Every quoting file (CLAUDE.md, HANDOFF, NEXT,
PROGRESS, FEATURES, GAPS, this log) now names the test that
actually runs. Decision 0069 records why the panels drive the engine
rather than keeping a panel-side model.

### Browser proof (§6.2)

`xtask dist --web` green (wasm 21,274,323 B under the 21,500,000 B
ceiling, bridge smoke honest — script bodies stripped before the
verdict). Served `dist/web` on 127.0.0.1:8391 and drove it with
playwright + system Chrome (`channel` chrome; no Playwright browsers
are downloaded on this host):

- `ping` → `ready` ok=true — the downcast holds.
- `postMessage openDocument` with `corpus/fixtures/composed.fw.png`
  and **zero input events** → `opened` ok=true, screenshot taken.
- The batch-17 engine commands over the same bridge: `object.createPath`,
  `autoshape.rect`, `autoshape.star` → all `applied` ok=true;
  `exportPng` 140 B, `exportDocument` 1,122 B.
- `Ctrl+O` → `expect_file_chooser` accepted the real `.fw.png` →
  second `opened` ok=true (the browser open door works).
- `Ctrl+S` → `expect_download` fired: `untitled.fw.png`, 612 B.
- `pageerror`: **none** (`unreachable` would be a wasm panic). The
  only console entry is the loopback server's missing `favicon.ico`
  (404), not an app error.

The three new panels were also proved to *render*: the CS6 right dock
stacks its twelve panel windows 260 pt apart, so the Auto Shapes /
URL / Behaviors windows (roster slots 9–11) fall below a 900 px
viewport. At a 3200 px tall viewport, with the persisted
`shell_layout` seeding the dock open, all three windows draw; hiding
the three by name and re-shooting changes **95,559 px** in the dock
column (Auto Shapes 66,952 px, URL 25,080 px, Behaviors 3,527 px) —
the panels are on screen, not merely registered.

### Scan gate: the scratch dir was being scanned

The secret-scan step failed the gate on the proof's own helper
scripts: they live under `.gitignore-scratch/`, are git-ignored, and
are rewritten on every run, so their findings could never stick to
the line-keyed baseline. `.gitignore-scratch/` is now excluded in
`.aegisignore` alongside `temp/` and `log/` — untracked working
files, not source. The baseline was then regenerated from a full
scan: 519 → 520 findings, the one addition being the loopback
address quoted in this entry above.

### Lesson

A panel is not a feature. The three inert rows looked like missing
features, but the verbs were already there — what was missing was the
surface that called them. Checking "does the engine already own this
data?" before "does this need new engine work?" turned three
feature-sized items into a panel-sized change.

The same lesson applies to the proof: "the panel is in the inventory"
and "the panel is on screen" are different claims. Only the pixel
diff distinguishes them.

## 2026-11-04 — Batch 17 shipped: CI repaired and PR merged

### The CI image was the CI failure

The first GitForge sweep on `56ee1bf` failed at `fmt-layers` with
`no matching package named 'egui_extras' found ... offline mode`. The
image cache was stale after the lockfile change. The image was rebuilt
from the current lockfile and re-shipped; retry `ce94c14f` passed all
eight jobs. Rebuild and re-ship `firecraft-ci-rust:1` whenever
`Cargo.lock` moves.

### PR #7 merged

The direct push to `main` was refused by the Safeguards ruleset, so the
branch went through PR #7. After `gate` passed, the PR merged by squash
as `5fac7a2` to preserve linear history.

### UI evidence and follow-up

The batch 17 browser proof recorded in the local devlog used a 3200 px
viewport to show every panel in the right-side stack. That proves the
panel content renders, but also exposed a usability gap at ordinary
window sizes: lower panels need a better navigation/docking treatment.
The tracked `docs/img/app-main.png` predates this merge and must not be
used as a current screenshot. Follow-up is tracked in
[`tasks/T22-screenshot-ui.md`](tasks/T22-screenshot-ui.md).

The previously empty GitHub Wiki landing page was replaced with user
and contributor navigation (`Home.md`, `_Sidebar.md`), pushed to the
wiki repository at `4fd9c7a`. The source copy is under `docs/wiki/`.

The public Pages manifest was fetched read-only at 20:01 local time.
Its `appinfo.json` and `host.html` hashes match local `dist/web`, but
the JavaScript, wasm, and index hashes differ. The deployed source
revision is unknown; T18 now tracks reconciliation rather than assuming
the live site matches the local build.

## 2026-11-04 — CS6 shell dock, per-kind inspector, and the T22 screenshot audit

### The right dock replaces the floating panel windows

The panel group was a stack of independently positioned egui windows
along the right edge. As floating windows they covered the canvas, and
their fixed vertical step put the lower panels below the fold at
ordinary sizes. The group is now one right-side `egui::Panel` — a
column of stacked, collapsible panels under CS6-style headers — so the
dock *reserves* its width, the canvas shrinks instead of being covered,
and the column scrolls when the stack outgrows the window.

### The inspector grew per-kind field sets

`properties_panel` used to answer "selection has no editable properties
yet" for anything that was not a path or a text object. It now dispatches
every object kind the document model has — path, text, symbol instance,
group, bitmap, slice — and the group and bitmap kinds reach the live
effect list as well. A bitmap selection shows its pixel size, tile
count, location and offset; a slice shows its type, shape, geometry and
its link and alt text.

The slice link editor is now one shared component used by both the URL
panel and the inspector, so the two cannot disagree: a single draft is
reseeded whenever the selected object or the stored value changes
underneath it.

The bitmap's fill and stroke editors are **not** UI work — the `IMGv`
node carries no fill/stroke node, `object.setFill`/`object.setStroke`
are path-only, and `object.setBounds` refuses bitmaps (decision 0019).
CS6's bitmap tools live in a separate *Image Editing* command panel,
evidenced in the CS6 install. That gap is recorded as evidence-gated
rather than faked.

### The screenshot audit, and an invalid first round

T22's first capture round was invalid: the bundle in `dist/web` predated
the inspector work, so the captures showed the pre-change fallback text.
Rebuilding the bundle and recapturing produced the real field sets —
the path's X/Y/W/H/opacity/blend/visible/locked, and the bitmap's
`bitmap 32x32 px, 1 tile(s) of 128 px`, `at (100.0, 100.0), offset (0.0,
0.0)`, name, display controls and effect list.

Six captures (empty, vector, bitmap at 1280×800 and 1920×1080) were
taken against the fresh optimized bundle, with the objects created
through the page's own command surface and the WebGL context alive in
every one. Measured at 1280×800: the tool strip ends at x=70, the canvas
runs 71–961, the dock occupies 962–1279, and the dock's drawn content
ends at row 682 of 800 — every open panel inside the viewport. At a
deliberately short 1280×400 viewport a wheel event over the dock moves
the content, so panels stay reachable when the stack is taller than the
window.

The tracked `docs/img/app-main.png` was replaced with the 1280×800
vector capture and labelled for its build and state.

### Outcome

`cargo run -p xtask -- ci` green (exit 0). The tracked screenshot, the
T22 ticket's receipts, the parity matrix's sampling note, and the task
board are reconciled with what the captures actually showed.

## 2026-11-05 — 22.2: the CS6 tool strip, as data

### The evidence, and where it stops

The strip's shape is CS6's, and the evidence for it is three sources
that agree where they overlap:

- **The CS6 help set** (a text extract of the help PDFs) gives the six
  categories in order, the group triangle and the cycling rule, the
  Modify trio, the pixel-selection family, the retouch trio and its
  Alt toggle, the Colors section's four buttons, and the three view
  modes.
- **The book's Table 1-1 "Fireworks Tools"** gives the tool names and
  the one-key shortcuts, and it is the only source for `O` (Freeform)
  and `2` (Hide/Show Slices).
- **The install's own `Fireworks.xml`** gives the *shape* of CS6's
  tool-shortcut table: 37 `<one_tool>` blocks, 40 entries, decimal
  `MODS VK`, every one plain. The blocks carry **bare GUIDs and no tool
  names**, so the GUID→tool mapping is not recoverable. That is
  recorded as an open evidence gap, not guessed; the strip's names and
  keys come from the help and the book.

The multiplicity in the keymap's VK histogram (S×3, Q×4, B×2, L×2, M×2,
U×2, R×2) corroborates the group sizes the help describes in prose.

### What changed

`crates/app/src/toolstrip.rs` is new: the strip as data — six
categories in CS6's order, twenty-four groups with their key and their
members, CS6's labels, and the flyout memberships. The shell renders
from it, so nothing about the order is FireCraft's own.

CS6's group-cycling rule is implemented: a group has **one** key, and
repeated presses advance within the group. The Hide/Show Slices
control is the one exception — it keeps the book's `2` but *acts* on
the control rather than becoming the active tool, because it is one of
CS6's panel buttons.

New tools in the strip: Skew and Distort (behind the Scale button on
`Q`), Blur, Sharpen, Smudge, Dodge, Burn, Rubber Stamp, Red-eye
Removal and Replace Color, Knife, Freeform and Reshape Area, and the
Web section's hotspot and slice variants. The Colors section's four
controls and the two color boxes are panel buttons — they act on click
and never become the tool.

### Four bugs found on the way

1. **The path paint was written as ARGB, but the engine reads it as
   ABGR** — red and blue swapped in "Set Default Stroke/Fill Colors".
   The shell had one color helper; the engine has two conventions by
   design (bitmap/retouch ARGB, path paint ABGR). The shell now has
   `argb_raw` and `abgr_raw` and each call site names the one it means.
2. The hotspot payload sent the shape name `"oval"`; the engine's
   `WebShape` is `"rect"` / `"ellipse"` / `"polygon"`.
3. **`Editor::commit_polygon` was never called from anywhere** — the
   polygonal marquee could be started and never finished. The
   properties panel now carries the in-progress shape's panel (point
   count, finish, cancel), and Enter/Escape finish or abandon a plotted
   path on the canvas.
4. The icon table had been regenerated by appending rather than
   replacing: 117 entries for 73 icons, every lookup silently keeping
   the first copy. It is now one entry per icon.

### The icons

29 new Lucide SVGs were vendored for the new tools. Every new file was
diffed against upstream `lucide-static@1.69.0` and matches byte for
byte after the license comment and the `class` attribute are
stripped; the 43 pre-existing files were checked the same way. The one
file that is **not** Lucide — `slice-knife.svg`, an original drawing,
since Lucide has no such icon — now has its own row in
`docs/ATTRIBUTION.md` (it also had a stale comment naming a different
project), and the Lucide row's count is 72.

### Outcome

`cargo clippy -p app -p engine --all-targets -- -D warnings` clean;
the app crate's suite green at 151 unit tests plus the integration
suites, including nine new tests over the strip's own invariants and a
test that every tool the strip shows has an embedded icon.

**The full workspace gate is green** — `cargo run -p xtask -- ci`
finished `[xtask] ci: all gates green` (`XTASK_EXIT=0`) with all
twenty steps `ok`: aegis, coverage, fmt, clippy (workspace and
`web/`), test, layers, wasm, corpus, floor, scripts, golden,
svg-golden, svg-roundtrip, web-golden, perf, fuzz, parity, scorecard,
changelog. The perf step enforced its 12 budgets (worst headroom
1.4×, on `open_scripted_hwm_kib`).

### The gate that was already broken

The gate failed at its **first** step, Aegis — and it had been failing
there since the last baseline regeneration, before this change was in
play. The committed `.aegis-baseline.json` keys each finding on
`file:line:content-digest`; this change rewrote `menus.rs`,
`document.rs` and four docs, which shifted every finding below the
edit and invalidated 307 baseline entries. All 307 were the known
false-positive classes (292 `zip-code` on numeric literals, 8
`pci-cardholder-data` on the word "pan", 5 `sql-query` on the words
"select from a tool group", 1 each of `ssrf-localhost` and
`bearer-token-url`). The baseline was regenerated over the whole tree
with the documented recipe — `aegis --format json -c production scan .
--output-file .aegis-baseline.json -q` — and the scan is now clean
(exit 0, zero new findings).

The lesson worth keeping: the rebaseline is not paperwork, it is the
step that makes the gate mean anything. A gate that fails on noise is
a gate nobody runs, and a gate that is quietly red is worse than no
gate at all.

## 2026-11-06 — The Aegis gate, rebuilt on rule classes and content digests

> **Superseded 2026-11-09.** The mechanism this entry describes —
> `.aegis-pattern-state.json` plus `.aegis-allowlist.json`, with the
> gate filtering the scanner's output itself — was **not** merged. It
> was superseded by decision 0074, the content-keyed baseline
> (`.aegis-baseline.tsv`), which keeps every rule class armed and
> adjudicates each occurrence individually. The entry is kept as the
> record of what was measured and why; the mechanism it describes is
> gone from the tree.

### The question that started it

Two questions were asked about the release machinery rather than about
Fireworks: *what exactly is the 21.5 MB ceiling, and could the WASM
just be loaded from GitHub instead?* and *is Aegis (Rust) or the older
Atheon-Enhanced scanner (Go) the faster and more reliable of the two?*

Both were answered from measurement, and both produced changes.

### The 21.5 MB number is ours, not the platform's

`xtask/src/dist.rs` pins `WASM_SIZE_CEILING: u64 = 21_500_000` and
fails the build past it. That is FireCraft's own regression tripwire —
not a WASM 3.0 limit, not GitHub's, not Cloudflare's — and it is
re-pinned deliberately whenever the bundle grows. The binding external
limit is Cloudflare Pages' **25 MiB per-asset cap**; the shipped
bundle is 21,271,259 B raw and 7,376,731 B gzipped, so it sits ~4.94 MB
under the cap.

Loading the WASM from GitHub is possible but worse: GitHub's raw host
serves `.wasm` as `application/octet-stream`, which makes
`WebAssembly.instantiateStreaming` throw, and its URLs are ref-mutable
rather than immutable. Decision 0071 records the measurements and the
answer — Pages stays; if the bundle ever outgrows 25 MiB the move is
object storage behind a Worker that serves `application/wasm`.

### Aegis versus Atheon-Enhanced

Same corpus, same box, three interleaved runs each:

| scanner | median wall clock | findings |
|---|---|---|
| Aegis 0.6.3 (Rust) | **12.12 s** | **373** |
| Atheon-Enhanced (Go) | **216.3 s** | **18,021** |

Atheon-Enhanced is 17.8× slower and 48× noisier, and its rules
mis-attribute (terraform, azure and dockerfile rules firing on Rust
lines). On a planted corpus both missed `ghp_` and `xoxb_` tokens;
Atheon-Enhanced caught a Stripe key Aegis missed, which does not
outweigh 48× the noise. Aegis stays the scanner.

### What the comparison exposed, and the gate it broke

Aegis has two suppression channels and only one of them works. Its
`--baseline` keys a finding as `pattern:file:line:content-digest`, so
a suppression dies with the line it names; a JSON profile's
`disabled_patterns` field is silently ignored (measured: 7
`eval-usage` findings with and without it); the only working per-rule
switch is the state file at `$HOME/.config/aegis/pattern-state.json`,
which is keyed on the *rule name* and survives every edit.

That distinction is why the gate failed on 2026-11-06: the 22.2
receipt prose quotes the rule names it describes, so the receipt trips
the rules it is about — and because the baseline is line-keyed, an
unrelated edit above a finding invalidates its entry. The recorded
repair was to regenerate a 19.4 MB baseline over the whole tree, which
takes a full-tree scan. A gate whose routine response to an unrelated
edit is a fifteen-minute regeneration is a gate that gets skipped.

### What changed

The baseline is gone; 644 line-keyed entries in 19.4 MB are replaced by
12 rule names and 8 content digests.

- **`.aegis-pattern-state.json`** suppresses whole rule classes that
  cannot fire in this tree — `zip-code` (523 hits: every 5-digit
  constant), `pci-cardholder-data` (52: the word "pan" in "pan-zoom"),
  `phone-number` (21: `4294967295` and CRC values), `eval-usage` (15:
  Rust `.eval(t)`), `ssrf-localhost` (8: the workspace has no HTTP
  client), `sql-query` (7: English prose), the JavaScript-shaped
  `path-traversal` rules, and three digit-run PII rules. The evidence
  for each was in decision 0070, whose rule-class suppression
  `main` superseded with decision 0074; the same evidence now rides
  with each baselined row in `.aegis-baseline.tsv` and in the
  `CLASS_NOTES` table in `xtask/src/aegis.rs`. The five `credit-card-*`
  rules stay
  **armed**, so a real card number is still caught.

- **`.aegis-allowlist.json`** suppresses the ten remaining
  per-finding false positives, keyed `pattern:file:content-digest` —
  the line is deliberately omitted, so an entry survives every edit
  that does not touch the matched content, and dies when the content
  itself changes, which is exactly when a human should look again.
  Each entry carries its own note saying why it is a false positive.
  This file, and `.aegis-pattern-state.json` above, were both
  superseded and deleted when `main` adopted the content-keyed
  baseline (decision 0074); the branch no longer carries either.
- **`xtask`** installs the pattern state into a private `HOME` (so the
  developer's own Aegis configuration is never touched) and filters the
  scanner's findings itself; `cargo run -p xtask -- aegis --report`
  prints the uncovered findings with the key an allowlist entry would
  need. The gate never writes the allowlist: suppressing a finding is a
  human decision that requires seeing the finding first.

### Receipts

- The gate is green in the new form: `[xtask] aegis ok`, exit 0.
- The gate still fails when it should. A planted AWS example access-key
  id (the key AWS publishes in its own documentation) in the tree
  produced `aws-access-key aegis-gate-selftest.txt:1 [critical]` and
  exit 1; the file was removed immediately afterwards.
- The `.aegis-baseline.json` file is deleted from the repository.

### Superseded

The class-level suppression above was replaced before merge by
decision 0074's content-keyed baseline. The measurements in this
entry stand; the mechanism does not. What survives is the reason the
gate was rebuilt at all: a line-keyed baseline turns an unrelated doc
edit into a gate failure, and the remedy for that must not be a
procedure that encodes the defect.

The lesson is the one the previous session recorded, now with the tool
fixed rather than the procedure: the rebaseline was a procedure
compensating for a line-keyed baseline, and a procedure that encodes a
defect survives the defect's fix.

## 2026-11-06 — 22.5: the screen-mode cycle, the Alt-hold retouch swap, and the Window arrangement trio

Ticket 22.5's evidence-backed items landed as one batch. Three of
them came straight out of the reference extracts; the fourth was a
comment that asserted the opposite of the layout XML.

### The three-mode screen cycle (fwref 1178–1185)

The screen-mode control in the View section of the tool strip used to
alias `ToggleHidePanels` — a Tab press wearing a monitor icon. CS6's
help names three view modes: *Standard Screen mode* (the default),
*Full Screen with Menus mode* ("A maximized document window view set
against a gray background. Menus, toolbars, scroll bars, and panels
are visible"), and *Full Screen mode* ("A maximized document window
view set against a black background. No menus, toolbars, or title
bars are visible"). `ScreenMode` now cycles those three in order; the
layout in `Shell::ui` folds chrome by mode, the canvas backdrop
switches to CS6's gray (0x999999) / black in the two full-screen
modes, and the document tab strip drops in Full Screen (no title
bars). The cycle is view state: the journal never moves.

### The Alt-hold retouch swap (fwref 3555, 3580)

CS6's help twice: "Hold down Alt (Windows) or Option (Mac OS) to
change from one tool behavior to the other" (the Blur/Sharpen/Smudge
section), and "To temporarily switch between the Dodge tool and the
Burn tool, hold down Alt (Windows)". `toolstrip::alt_variant` is the
pure pair-swap (Blur↔Sharpen, Dodge↔Burn; Smudge has no variant),
and `Editor::paint_dab` resolves the effective tool at the dab edge
through a shell-supplied `alt_held` flag — the editor never reads
the window system, and the active tool never changes. The swap is
an involution gated by
`toolstrip::tests::the_alt_swap_is_an_involution_on_two_pairs` and
proven end-to-end by `editor_surface::alt_hold_swaps_the_retouch_pair_at_the_dab`
(journal command names differ under Alt).

### The Window arrangement trio (57650/57651/57652)

The transcription carried 57650 *Cascade* and 57652 *Tile* and no
57651 — `ID_WINDOW_TILE_HORZ` from `MENU_DESCRIPTION_MAP`. The
"Tile Horizontally" row is now transcribed between them and 57652 is
relabeled *Tile Vertically* (the resource's `ID_WINDOW_TILE_VERT`);
`menus::tests::the_window_arrangement_trio_matches_the_resource`
pins all three. The pinned item census rose 289 → 290: the row was
always part of the CS6 surface; the transcription was short one. The
live doc quotes (CLAUDE.md, GAPS, HANDOFF, NEXT) moved with it; the
dated batch history in NEXT keeps its period counts. The §A open
evidence item in CS6-PARITY closed on this evidence — no bar-resource
decode was needed.

### The wrong comment

`Shell::ui` claimed "the CS6 and Photopea arrangement puts it [the
Properties block] in the right dock". CS6's own `Expanded Mode.xml`
puts Properties (palette app-data 35586) in the **bottom** dock at
923×105. The comment is replaced by the screen-mode layout note; the
placement itself is 22.1's item (recorded, not silently kept).

### Receipts

- `cargo test -p app --lib toolstrip::tests::the_alt_swap_is_an_involution_on_two_pairs` — pass
- `cargo test -p app --lib menus::tests::the_window_arrangement_trio_matches_the_resource` — pass
- `cargo test -p app --lib shell::tests::the_screen_mode_cycles_through_the_three_view_modes` — pass
- `cargo test -p app --test editor_surface alt_hold_swaps_the_retouch_pair_at_the_dab` — pass
- Full workspace gate: `/var/tmp/fc-ci-225.log` (`cargo run -p xtask -- ci`)
- Deferred from 22.5 with reasons: dock tab-groups and the Properties
  bottom dock (22.1's structure), Text ▸ Font submenu and *Repeat
  Filter* (rows 52/54, rank 14), the F1 grouping mismatch (row 255),
  the 76 unbound keymap rows (row 257), the 3D tool (deliberately not
  carried), the Main toolbar's contents (row 164).
