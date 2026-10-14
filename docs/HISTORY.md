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

**Where it goes.** The 57 missing verbs, the partial-class debts, and
the evidence-gated questions (view flags, button-symbol wire shape) are
phased into atomic tasks in [`HANDOFF.md`](HANDOFF.md) §4.

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
