# FireCraft — Live task board

> Working document (not a historical record). Canonical planning:
> [`PLANNING.md`](PLANNING.md) + [`planning/firecraft/ROADMAP.md`](planning/firecraft/ROADMAP.md).
> Per-task tickets: [`tasks/`](tasks/) (one file per task, same
> objective/inputs/authority/stopping-condition/verification shape as the
> milestone tickets). Session-level history: [`SESSIONS.md`](SESSIONS.md);
> arc-level history: [`HISTORY.md`](HISTORY.md).
>
> **Refresh rule:** update this board in the same change as any task
> lands, starts, or is re-scoped. The board mirrors the agent session
> task list; if the two disagree, this file is the authority to reconcile
> from.
>
> **Release-task status vocabulary.** Every release-relevant row below
> uses the cross-platform release campaign's status words
> (campaign §3.2): `NOT STARTED` · `READY` · `IN PROGRESS` · `BLOCKED` ·
> `IMPLEMENTED` · `BUILD VERIFIED` · `INSTALLED` · `LAUNCHED` ·
> `SMOKE TESTED` · `PUBLISHED` · `CERTIFIED` · `DEFERRED` · `REJECTED`.
> These are not interchangeable labels. A row may claim `INSTALLED`,
> `LAUNCHED`, `SMOKE TESTED`, `PUBLISHED` or `CERTIFIED` only when it
> carries a receipt naming what was observed, on what host, and on what
> date. A row with no receipt is `IMPLEMENTED` or `BLOCKED`.
>
> **Scope authority** for release work:
> [decision 0075](decisions/0075-release-scope-mvp.md) — currently
> *recommended — awaiting maintainer approval*, so the approved release
> scope is **unset**. **Evidence baseline** for release work:
> [`release/PHASE-0-AUDIT.md`](release/PHASE-0-AUDIT.md) (PR #16,
> dated 2026-11-09 — the link resolves once that PR merges).
> Per-artifact evidence state:
> [`release/ARTIFACT-MATRIX.md`](release/ARTIFACT-MATRIX.md).

## In flight

| # | Task | Ticket | Status | Owner | Next action |
|---|---|---|---|---|---|
| 5 | 19.6 inert-menu burn-down — Modify/File families (83 of 289; batch 8 = Attach to Path landed 2026-11-03) | [tasks/T5-inert-burndown.md](tasks/T5-inert-burndown.md) | `IN PROGRESS` | UI/verb lane | continue the per-family burn-down; each batch carries its own evidence gate |
| 194 | Aegis gate: content-keyed baseline, so a doc edit no longer fails the gate (decision 0074) | [decisions/0074-aegis-content-keyed-baseline.md](decisions/0074-aegis-content-keyed-baseline.md) | `IMPLEMENTED` — the content-keyed gate and its decision record are in the tree; the gate has been exercised on this branch's tree | CI/tooling lane | the gate is content-keyed on this branch but the baseline format is **not yet on `main`**; port it and attach a run id before claiming the fleet-wide state |
| 27 | Cross-platform release engineering: Windows, macOS, Linux package matrix, Android feasibility, source, SBOM/provenance, signing, CI aggregation and certification | [tasks/T27-cross-platform-release.md](tasks/T27-cross-platform-release.md) | `IN PROGRESS` — implementation is ahead of verification; **no target is `CERTIFIED`** and nothing is signed | release-workflow lane | execute the cross-workstream [production-readiness plan](PRODUCTION-READINESS-PLAN.md), preserve and sequence the active PR stack, then run a disposable tag through build, aggregation, download, and checksum verification |

## Queued (Phase 20 — daily-driver features)

| # | Task | Ticket |
|---|---|---|
| 20 | 20.1 In-canvas text editing (egui TextEdit overlay; kittest focus tests) | [tasks/T20-text-editing.md](tasks/T20-text-editing.md) |
| 7 | 20.2 Optimize panel (palette/bitrate over `ExportOptions` + live preview) | [tasks/T7-optimize-panel.md](tasks/T7-optimize-panel.md) |
| 8 | 20.3 Animation timeline (frames strip, timing, onion skinning, `symbol.tween`) | [tasks/T8-animation-timeline.md](tasks/T8-animation-timeline.md) |
| — | 20.4 Mask stack UI (add/remove/mode/link/move) | — |
| 9 | 20.5 Multi-document tabs + Window-menu panel toggles | — |
| 10 | 20.6 History panel click-to-rewind (journal replay to seq N) | — |
| 11 | 20.7 Slice/hotspot behavior editing + HTML preview | — |

## Queued (Phase 21 — modern differentiators)

| # | Task | Ticket |
|---|---|---|
| 12 | 21.1 Autosave + crash recovery (journal snapshot sidecar; restore prompt) | — |
| 13 | 21.2 HiDPI `pixels_per_point` + render-stamp→dirty-counter migration | — |
| 14 | 21.3 Command palette v2 (fuzzy scoring, recents, save-steps-as-command) | — |
| 15 | 21.4 Preferences dialog + platform-storage persistence (0067 follow-up) | — |
| 16 | 21.5 accesskit desktop + web re-audit (0067 records the web blocker) | — |
| 17 | 21.6 In-app scripting (`.jsf`/`.fwsteps` from the Commands menu) | — |

## Queued (release + quality)

| # | Task | Ticket | Status | Owner | Next action |
|---|---|---|---|---|---|
| 18 | Ship v0.2.5: release assets + web deploy | [tasks/T18-release.md](tasks/T18-release.md) | `BLOCKED` — blocked on T23's asset breadth. The v0.2.5 release exists as published artifacts, but the **live web deployment serves a bundle that corresponds to no identifiable release** (B-6), and the published set was published out of band rather than by the release workflow (B-21) | release-workflow lane | reconcile the deployed bundle against a tagged release by hashing the deployed assets individually and recording the producing commit; do not re-cut v0.2.5 — B-19 means the next artifact must be rebuilt and re-measured, not re-tagged |
| 23 | Broaden release assets to photocraft breadth (dmg, AppImage, Flatpak, win installer) | [tasks/T23-release-assets.md](tasks/T23-release-assets.md) | `IMPLEMENTED` for the recipe set, `NOT STARTED` for every off-fleet artifact — the AppImage, Flatpak, Windows and macOS recipes exist and are parse-checked (or existence-checked) by the gate, but **none has ever been produced**: `linuxdeploy`, `flatpak`/`flatpak-builder`, `pwsh`/`ISCC`, and `lipo`/`hdiutil`/Xcode CLT are all absent from the fleet (B-8, B-11, B-12) | release-workflow lane | either provision a host with each SDK and produce the artifact, or keep the recipes recipe-only and let the scope decision in 0075 exclude those targets — do not advertise them as supported |
| 19 | Quality ratchet: coverage → 99%, `deny(missing_docs)` on geom/color, WCAG AAA | — | `NOT STARTED` | quality lane | schedule after the release lane; the coverage floor is a workspace-wide change |
| 21 | egui_kittest snapshot gates per panel/toolbar | — | `NOT STARTED` | quality lane | needs a stable per-panel harness |
| 22 | Screenshot-driven frontend audit + systematic refinement | — | `IN PROGRESS` | frontend lane | audit against the CS6 shell-parity directive; record screenshots per panel |
| 25 | Responsive/adaptive layout audit + fixes | — | `NOT STARTED` | frontend lane | follows the responsive audit in the current UI work |

### Production readiness and T27 work-package detail

The cross-cutting cleanup, WASM optimization, QA, UI polish, agent ownership, and integration sequence are defined in [PRODUCTION-READINESS-PLAN.md](PRODUCTION-READINESS-PLAN.md).


The full implementation-versus-verification table for T27 — per work
package, the implementation state, the verification state, the owner and
the next action — lives in
[`tasks/T27-cross-platform-release.md`](tasks/T27-cross-platform-release.md).
The per-artifact evidence state (one row per artifact, each with a status
word, an evidence-ladder rung and a receipt) lives in
[`release/ARTIFACT-MATRIX.md`](release/ARTIFACT-MATRIX.md). The board
does not restate either; where the two disagree, the ticket and the
matrix win.

Two conditions are recorded here because they are easy to misread as
progress:

- **Nothing is `CERTIFIED`.** No artifact is signed and no provenance or
  attestation exists on any branch (B-14), so no artifact can reach
  evidence-ladder rung 9.
- **The release scope is not approved.** Decision 0075 records the MVP
  candidate and the extended campaign scope, and its status is
  *recommended — awaiting maintainer approval*. Until the maintainer
  approves it, the plan's full target matrix remains the standing scope
  and no cut is authorized against a narrowed one.

## Closed (this board's lifetime)

| Task | Landed |
|---|---|
| Aegis baseline regeneration + `.aegisignore` self-exclusion | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 6 — View menu (10 inerts) | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 7 — Text menu (14 inerts) | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 8 — Attach In Path / Attach to Path (2 inerts); vertical-align rows evidence-gated | 2026-11-03 (`a8c86af`) |
| Gate `perf` step repair — the memory probe re-execs through `/proc/self/exe` so it survives the gate's own binary being replaced mid-run ([decision 0072](decisions/0072-gate-reexec-resolution.md)) | 2026-11-09 (`7252648`) |
| CI-image offline fix — the baked registry is a fast floor, not a wall; the fleet-wide red from a moved `Cargo.lock` was a tool defect, not a stale image ([decision 0073](decisions/0073-ci-image-fast-floor-not-wall.md)) | 2026-11-09 (`02c1c9d`) |
| Aegis baseline re-pin for the release-workflow tree — the baseline was line-keyed, so PR #14's prose edits shifted two documented false positives and the gate failed on noise. Superseded by the content-keyed gate below; the re-pin was a procedure that existed only to route around the tool defect | 2026-11-09 (`7e7f8ad`) |
| Aegis gate is content-keyed on this branch — a finding's identity is `(pattern, file, content digest)` rather than Aegis's line-keyed fingerprint, so a prose edit that moves a documented false positive no longer fails the gate. No rule class is switched off; each class's rows carry the reason they were adjudicated ([decision 0074](decisions/0074-aegis-content-keyed-baseline.md)) | 2026-11-09 |
| Host fork-bomb from a throwaway `/proc/self/exe` probe — ~5800 processes at load 2057 starved the GitForge runner's sandbox; terminated by per-PID cmdline+exe verification and the probe directory moved aside | 2026-11-09 |
| Task-board tracking scaffold — `docs/TASKS.md` plus the `docs/tasks/` per-task ticket set (one file per task, same objective/inputs/authority/stopping-condition/verification shape as the milestone tickets). The scaffold is this board and the `docs/tasks/` directory; the row it carried in the queued table is closed by this reconciliation | 2026-11-09 |
