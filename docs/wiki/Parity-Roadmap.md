# Fireworks CS6 Parity Roadmap

This page summarizes FireCraft's current user-visible parity work. It does not claim that every undocumented Fireworks behavior has been reproduced. The detailed, evidence-bearing inventory is [CS6-PARITY.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md).

**Status reviewed:** 2026-11-11 against `main` at `b61f198`.

## How to read parity

Keep three independent measures separate:

- **DOM/API parity:** 268 carried, 3 partial, 0 missing of 356 verbs. This describes scripting/API compatibility.
- **Shell command reachability:** 164 of 253 registered engine commands are bound in the CS6-visible shell surface (64.82%, measured 2026-11-11). The command palette exposes the full command registry, but that does not mean every operation has a polished contextual GUI.
- **User-visible feature parity:** each menu, tool, panel, inspector field, workflow, import/export path, canvas interaction, keymap and selection state is classified in the [parity matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md).

A DOM command being carried does not prove that its GUI surface exists. A row appearing in a menu does not prove that the behavior is complete. Each claim needs a working UI path and evidence.

## Current order of work

| Workstream | State at 2026-11-11 |
|---|---|
| User-visible parity inventory | Complete and published (T26); update it as real behavior lands |
| Phase 22 — CS6 shell fidelity | In progress; panel inventory, tool strip, adaptive context menus and icon pack landed; inspector/chrome follow-ups remain |
| Phase 20 — daily-driver editor workflows | Partially delivered: text editing and multi-document/history/slice workflows on main; Optimize/timeline in PR #118; mask stack in PR #113 |
| Phase 21 — modern additions | User-facing deliverables landed: autosave/recovery, HiDPI, command palette, preferences, accessibility work and scripting |
| Phase 19.6 — menu wiring | Complete: 2 inert of 296 rows; eight explicit feature/evidence gates remain |

## Remaining parity and quality work

1. **Inspector completeness:** bitmap transform and fill/stroke behavior remain constrained by the document model and recorded decision 0019; do not fake unsupported fields.
2. **CS6 chrome and toolbar:** the main-toolbar contents and remaining presentation details are tracked in Phase 22 and the parity matrix.
3. **Open Phase 20 PRs:** #118 contains the Optimize panel and timeline; #113 contains the mask-stack extension. They are not shipped until merged and verified.
4. **Evidence and regressions:** maintain screenshot/interaction receipts, round-trip and malformed-file tests, undo/redo checks, and the generated DOM and shell-reachability gates.

## Authoritative references

- [CS6 user-visible parity matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md)
- [Live task board](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TASKS.md)
- [Detailed phased roadmap](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md)
- [Progress and dated receipts](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/PROGRESS.md)
- [Generated DOM parity report](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/planning/generated/dom-parity.md)

The 2026-11-04 screenshot audit remains a dated capture receipt; it is not evidence that every UI surface or workflow has since been certified.
