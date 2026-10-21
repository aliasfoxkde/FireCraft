# FireCraft — History

> The project's arc, the decision index, and the engineering lessons
> each paid for. Live status: [`PROGRESS.md`](PROGRESS.md). Session log:
> [`SESSIONS.md`](SESSIONS.md). What to do next: [`HANDOFF.md`](HANDOFF.md).

## The arc

**The mandate.** A clean-room, open-source reimplementation of Adobe
Fireworks CS6 (v12.0) in pure Rust — full-featured, faithful to the
original, modeled on the StoryTold/ArtCraft engineering discipline,
WASM-first for the author's WebOS plus a wide native release matrix,
with enhancements over time (more image support and beyond). Adobe
abandoned Fireworks; this project carries it forward without carrying
anything of Adobe's code or assets.

**One week, twelve milestones (2026-10-07 → 10-12).** The repo went
from planning documents to a released application in seven days by
running the roadmap strictly: M0 bootstrap and method, M1 foundations,
M2 the Fireworks PNG reader, M3 the document model with byte-exact
round-trip, M4 the compositor with golden-image gates, M5 the command
catalog + app shell + web trunk, M6 vector depth, M7 text and bitmap
filters, M8 the import/export matrix (SVG, PSD, GIF, EXPv, batch), M9
scripting (sandboxed ES3-ish host, .jsf loading, History replay), M10
symbols and the library, M11 the web layer and export surfaces, M12
parity hardening and the release matrix (perf gate, memory ceiling,
`xtask dist`).

**Release and the parity turn (2026-10-13).** v0.1.0 shipped and the
repo went public. M13–M15 (shell parity, live filters, tool surface)
closed the same day, then the work redefined itself: task #86 swept the
entire CS6 DOM verb surface (356 verbs, from Adobe's *Extending
Fireworks*) against the engine registry into
`docs/planning/generated/dom-parity.md`. From then on the unit of work
is the **gap family**: one decision doc, one command batch, parity rows
flipped M→C, fuzzer grown in the same change. Families #87–#93 landed
group/ungroup, page guide/grid/onion, frames, object transforms, object
masks, layer/page structure (decision 0021), and the typed shared-layer
model with its sharing commands (decision 0022) — taking the registry
to 148 commands and parity to 172C/45P/57M.

**Campaign complete, release turn two (2026-10-14 → 10-15).** The gap
families ran #95–#125 in one day, taking parity from 174C to
**268 C / 3 P / 0 M / 85 N** (registry 105 → 241); #126–#132 closed
every recorded Phase 9 debt (tile bank, deep clone, mask rasterize,
bake rasterize, shared-layer render, bilinear import), #133–#134 added
pattern authoring and lossless WebP export. v0.2.0 shipped the whole
surface — and its web bundle was dead on arrival: the eframe shell
never overrode the wasm-only `as_any_mut`, so every browser file
operation panicked, and the bridge smoke matched its "green" string
inside the host page's own script source, so every gate stayed green.
A real-browser session against the deployed bundle found it in an
hour; v0.2.1 replaced the bundle the same day (downcast override,
repaint after bridge mutations, honest smoke, web fmt gate).

**Where it goes.** The parity campaign reached its evidence limits;
current work is application surface and enhancements —
[`HANDOFF.md`](HANDOFF.md) §4 Phases 11–13 (image support: AVIF/HEIC
decisions 0064–0065, import surface parity, lossy WebP; WebOS packaging
dry-run; release-engineering hygiene). Evidence-gated holds parked in
Phase 12.

**v0.2.2 and the hygiene turn (2026-10-19 → 10-20).** The GitHub
Actions CI had been failing silently on every push for 18 commits —
duplicate `push:` key in the `on:` block, wrong aegis download URL,
missing trunk/chromium install, a coverage step that would pass at 0%,
and a step list that had drifted 6 gates from `xtask ci`. All repaired
in one session. The aegis PII scan was also found to hang locally on
`target/` (build artifacts) and `fuzz/` (23GB corpus) — neither affects
CI on a clean runner, but it meant local gate validation was skipped.
Fixed: `target/` + `web/dist/` + `fuzz/` in `.aegisignore`; baseline
regenerated. aegis now runs in 0.17s locally vs the previous >120s hang.
The `xtask changelog` version stamp was also extended to verify all 8
Cargo.toml files, and the smoke timeout raised 120s→300s. v0.2.2
published on GitHub with 5 assets.

## Decision index

| # | Decision | One line |
|---|---|---|
| 0001 | scripting engine | Sandboxed ES3-ish host for .jsf automation |
| 0002 | CPU rasterizer | Software scanline/tile raster, no GPU dependency |
| 0003 | path booleans | Own boolean ops for union/subtract/intersect |
| 0004 | filter color order | Pinned color order through the filter stack |
| 0005 | live effect render | Effects as stack-order-preserving passes |
| 0006 | PSD fidelity | PSD reader/writer scoped to Fireworks-observable subset |
| 0007 | text run encoding | Run-split text model |
| 0008 | SVG filter mapping | SVG filters mapped onto the Fireworks effect stack |
| 0009 | animated GIF | States ↔ GIF frames, disposal-exact |
| 0010 | EXPv export options | Typed export options with verbatim fallback |
| 0011 | script mutation queue | Scripts mutate through the command queue only |
| 0012 | ES3 quirk ledger | Documented CS6 scripting quirks, reproduced |
| 0013 | automation surfaces | CLI/MCP/control-channel automation fronts |
| 0014 | symbol render | Symbol instances render through definition resolution |
| 0015 | symbol script carrier | Widget.script/opCode carried on definitions |
| 0016 | library import/export | Common Library re-creation, .stl style sets |
| 0017 | web layer + export | MSLb special layer + HTML/CSS/slice export model |
| 0018 | setFillColor collapse | Solid-fill collapse semantics pinned |
| 0019 | object transforms | object.* transform family incl. refusals pre-resample |
| 0020 | element masks | Mask model (mask.add family) |
| 0021 | layer/page structure | Flat LYLv, IMPb master, PageBody/Pages deltas; sharing deferred |
| 0022 | shared-layer model | LSMv/LSEv/LSAv typed; setSharing/shareToPages/detachShared |
| 0023 | canvas color/resolution | document canvas color & resolution commands |
| 0024 | object selection | selection model & the selection.* commands |
| 0025 | hierarchy selects | hierarchy selects, selectSimilar, and the walk |
| 0026 | selection-to-frames | selection-to-frames fans |
| 0027 | primitive params | rectangle-primitive params and the quadrangle |
| 0028 | autoshape redraw | autoshape.redraw (CS6 redrawSmartShape) |
| 0029 | web point append | web-object point append |
| 0030 | crop/edge/tween | crop selection, fill edge mode, tween |
| 0031 | flatten family | flatten.mergeDown & co. via compositor bakes |
| 0032 | knife/split | Knife + splitPaths (path splitting) |
| 0033 | convert to paths | convertToPaths (text → glyph outlines) |
| 0034 | rotate document | page rotation with a geometry re-bake |
| 0035 | view flags | view flags are session state, not document state |
| 0036 | button symbols | the four behaviors + button options |
| 0037 | place image | the place-image trio + shared assemble/retile helpers |
| 0038 | saved selections | named saved selections |
| 0039 | clipboard | clipboard surface + the paste-mask trio |
| 0040 | filter bake | filterSelection bake family over compose kernels |
| 0041 | image size | document.setImageSize (the last M row) |
| 0042 | convolve/motion blur | convolveSelection / motionBlurSelection bakes |
| 0043 | web/group setters | web/group setter verbs |
| 0044 | anim instance params | symbol.setAnimInstance* over the ANNi carrier |
| 0045 | text rectangle fits | text.setTextRectangle* via TextEngine::auto_block |
| 0046 | canvas trim | canvas trim + delete-all |
| 0047 | loose-ends setters | setEffectName walker, swap behaviors, button text |
| 0048 | text on path | text-on-path setters over a shared selection walk |
| 0049 | pixel selection | pixel-mask move + chamfer-field adjust |
| 0050 | paragraph setters | paragraph attribute setters over PINf/PSBf/PSAf |
| 0051 | multi-run text | multi-run text model + markup verbs |
| 0052 | anti-alias dials | custom anti-alias dial setters |
| 0053 | symbol alias | duplicateSymbolForAlias re-points instances |
| 0054 | import file | importFile placement (the last surface P row) |
| 0055 | tile bank | mkBT tile-bank load/write |
| 0056 | duplicate deep clone | object.duplicate deep-clones groups |
| 0057 | vector image mask | mask_type "image" rasterizes a vector clipboard |
| 0058 | mask union fold | pasteAsMask "add" unions the coverages |
| 0059 | bake rasterize-first | filterSelection rasterizes non-image selections |
| 0060 | shared-layer render | shared-layer links render their definitions |
| 0061 | bilinear resample | importFile resamples bilinearly, premultiplied |
| 0062 | pattern authoring | path.setPatternFill authors a pattern from an image |
| 0063 | WebP export | lossless VP8L export through image-webp, zero new deps |
| 0064 | AVIF decode policy | OS decoder on desktop; "unsupported in browser" in WASM |
| 0065 | HEIC refusal | No maintainable open HEIC encoder/decoder; patent pools |
| 0066 | smoke budget | 60s virtual-time covers app logic; 300s wall-clock backstop |

## Lessons (each paid for in a real failure)

1. **Capture deltas before mutating.** Undo writes *prior* state;
   capturing after the mutation writes the post-state back. Three test
   failures in one batch (#92) bought this rule; it is now checked at
   review time in every command.
2. **Never trust an RNG stream across catalog growth.** Forcing a call
   class in a fuzz-derived test requires draw-until-landed **plus**
   deterministic seed state — the generator deliberately over-draws
   indices and injects junk fields for refusal coverage, so a thin
   landable surface can go dry for a fixed seed (#92's CI leg,
   diagnosed with a deleted-after probe).
3. **Validate premises with execution, not deliberation.** The M9 CI
   "cgroup memory kill" theory died against the job-log ledger; the
   real cause was a stale offline cache after `Cargo.lock` moved.
   Re-ship the image on any lockfile move; check logs first.
4. **Fixture-first, corpus-second; validate layers separately.** A
   30-second fixture gate beats a 35-minute corpus run on a silently
   wrong input mapping (recorded from the strands-2B import campaign;
   applied here as synthetic write→read→byte-equal gates).
5. **No corpus oracle → the dictionary stands, synthetically gated,
   and the gap is recorded** (#93's LSMv/LSAv; the sharing encoding
   has no Adobe-authored sample in the corpus).
6. **Silence is a finding; findings must land in artifacts.** A
   diagnosis that exists only in chat is appeasement — decision docs,
   the parity matrix, PROGRESS.md, and the devlog are where work
   becomes durable.
7. **A verdict check must not be able to pass on its own inputs**
   (v0.2.0, 2026-10-15). The `dist --web` bridge smoke asserted its
   "green" string against a DOM dump that still contained the host
   page's own `<script>` source — it matched the code that would
   produce the outcome, so it stayed green for the web bundle's
   entire life while the bridge panicked on first use, and the
   release could not open or save anything. Strip script bodies
   before any DOM-string assertion; require the failure marker's
   absence too. Paired lesson: both web defects (the `as_any_mut`
   downcast panic, the missing repaint after bridge mutations) were
   invisible to every desktop gate — wasm-only cfgs and
   out-of-frame mutation have no desktop analog. Prove browser
   behavior in a browser (HANDOFF §6.2) before shipping any `web/`
   change.

## Infra archaeology (the runbook's "why")

- GitForge CI (7 jobs) verifies every push; find runs by `commit_hash`
  under `/api/pipeline-runs`, jobs under `/api/pipeline-runs/{id}/jobs`,
  logs under `/api/jobs/{id}/logs`.
- The fedora runner has an sshfs race ("mkdir: file exists") —
  retrigger via an empty commit in a quiet window.
- Stale image caches refuse offline builds ("silent mid-compile death")
  — the ledger in the job logs is the ground truth.
- The NAS is a spinner: batch reads, keep scratch on NVMe (`/var/tmp`),
  multi-GB payloads on `/nas/Temp/tmp` only.
