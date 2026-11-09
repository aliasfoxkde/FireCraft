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

## In flight

| # | Task | Ticket | Status |
|---|---|---|---|
| 5 | 19.6 inert-menu burn-down — Modify/File families (83 of 289; batch 8 = Attach to Path landed 2026-11-03) | [tasks/T5-inert-burndown.md](tasks/T5-inert-burndown.md) | in progress |
| 194 | Aegis gate: content-keyed baseline, so a doc edit no longer fails the gate (decision 0074) | [decisions/0074-aegis-content-keyed-baseline.md](decisions/0074-aegis-content-keyed-baseline.md) | in progress |

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

| # | Task | Ticket |
|---|---|---|
| 18 | Ship v0.2.5: release assets + web deploy | [tasks/T18-release.md](tasks/T18-release.md) |
| 23 | Broaden release assets to photocraft breadth (dmg, AppImage, Flatpak, win installer) | [tasks/T23-release-assets.md](tasks/T23-release-assets.md) |
| 19 | Quality ratchet: coverage → 99%, `deny(missing_docs)` on geom/color, WCAG AAA | — |
| 21 | egui_kittest snapshot gates per panel/toolbar | — |
| 22 | Screenshot-driven frontend audit + systematic refinement | — |
| 24 | docs/TASKS.md + docs/tasks/ tracking scaffold | — (this change) |
| 25 | Responsive/adaptive layout audit + fixes | — |
| 27 | Cross-platform release engineering: Windows, macOS, Linux package matrix, Android feasibility, source, SBOM/provenance, signing, CI aggregation and certification | [tasks/T27-cross-platform-release.md](tasks/T27-cross-platform-release.md) | in flight — P0; RP-00 matrix + RP-01 Flatpak/RPM-decision + **DEB clean-container smoke passed** (floor fix 2.31→2.35, xtask now verifies the floor) + RP-05 source archive + RP-06 SBOM + RP-07 fail-closed aggregate landed 2026-11-06; Windows/macOS/AppImage/Flatpak actual builds + install smokes hardware-bound; Android spike in research |

## Closed (this board's lifetime)

| Task | Landed |
|---|---|
| Aegis baseline regeneration + `.aegisignore` self-exclusion | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 6 — View menu (10 inerts) | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 7 — Text menu (14 inerts) | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 8 — Attach In Path / Attach to Path (2 inerts); vertical-align rows evidence-gated | 2026-11-03 (`a8c86af`) |
| Gate `perf` step repair — the memory probe re-execs through `/proc/self/exe` so it survives the gate's own binary being replaced mid-run ([decision 0072](decisions/0072-gate-reexec-resolution.md)) | 2026-11-09 (`7252648`) |
| CI-image offline fix — the baked registry is a fast floor, not a wall; the fleet-wide red from a moved `Cargo.lock` was a tool defect, not a stale image ([decision 0073](decisions/0073-ci-image-fast-floor-not-wall.md)) | 2026-11-09 (`02c1c9d`) |
| Aegis baseline re-pin for the release-workflow tree — the baseline is line-keyed, so PR #14's prose edits shifted two documented false positives and the gate failed on noise | 2026-11-09 (`7e7f8ad`) |
| Host fork-bomb from a throwaway `/proc/self/exe` probe — ~5800 processes at load 2057 starved the GitForge runner's sandbox; terminated by per-PID cmdline+exe verification and the probe directory moved aside | 2026-11-09 |
