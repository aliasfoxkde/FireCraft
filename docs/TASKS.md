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

## Closed (this board's lifetime)

| Task | Landed |
|---|---|
| Aegis baseline regeneration + `.aegisignore` self-exclusion | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 6 — View menu (10 inerts) | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 7 — Text menu (14 inerts) | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 8 — Attach In Path / Attach to Path (2 inerts); vertical-align rows evidence-gated | 2026-11-03 (`a8c86af`) |
