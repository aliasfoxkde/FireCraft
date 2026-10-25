# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

FireCraft: a clean-room reimplementation of Adobe Fireworks CS6 in pure Rust (Apache-2.0 OR MIT), modeled on the StoryTold/ArtCraft projects (github.com/storytold — photocraft et al.). Primary release target is a **WASM build for the user's WebOS** (live at https://firecraft-web.pages.dev), plus a wide desktop/CLI release matrix. The reference application is Adobe Fireworks CS6 (v12.0).

**Status: M0–M15 complete; v0.2.3 released (2026-10-24). The CS6 DOM verb-surface parity campaign is complete (268 C / 3 P / 0 M, registry 241); Phase 11 (image support) is complete. Active work = the quality ratchet (docs/NEXT.md Phases 14–18): coverage to 99%, lint hardening, WCAG 2.1 AAA audit, release automation.** Start at `docs/HANDOFF.md` — it holds the state snapshot, the phased/atomic plan, and the runbooks. Read before doing anything:

1. `docs/HANDOFF.md` — agent handoff: state, phased plan, CI runbook, lessons (START HERE)
2. `docs/PLANNING.md` — master plan: clean-room policy, Fireworks PNG format spec (§6), crate architecture (§7), stack decisions (§7.6), tiers, risks, method, RE tooling (§5.5), sourced research appendix
3. `docs/PROGRESS.md` — live status board; `docs/SESSIONS.md` — session log; `docs/HISTORY.md` — arc, decisions, lessons
4. `AGENTS.md` — day-to-day operating manual (commands, gates, commit rules)

Session task lists should mirror the active phase in HANDOFF.md §4. RE evidence work (Ghidra/REA over `source/` binaries) records derived specs only — raw Adobe-derived dumps stay uncommitted in scratch.

The global harness commands (backend-fixed's cargo/just/local-ci) do not exist here. Once M0 creates the workspace, the project's own gate is `cargo run -p xtask -- ci` (fmt, clippy `-D warnings`, tests, layer enforcement, wasm build).

## Repository layout

- `docs/HANDOFF.md` — agent handoff: state snapshot, phased/atomic plan, CI runbook
- `docs/PROGRESS.md` — live status board · `docs/SESSIONS.md` — session log · `docs/HISTORY.md` — arc, decisions, lessons
- `docs/PLANNING.md` — master planning document (see above)
- `docs/planning/firecraft/` — ROADMAP.md + per-milestone tickets (objective / inputs / authority / stopping condition / verification)
- `docs/planning/generated/` — `parity.md` / `scorecard.md` (xtask-owned) + `dom-parity.md` (CS6 verb-surface matrix; regen recipe in HANDOFF §5)
- `docs/decisions/` — decision records 0001–0022; write one per family/batch
- `docs/research/fireworks-png-nodes.md` — vendored MIT reverse-engineering dictionary of the Fireworks PNG node system (from davidje13/png-analyser) — the format Rosetta stone
- `docs/ATTRIBUTION.md` — asset-attribution ledger; **add a row in the same change as any non-code asset**
- `source/` — proprietary Adobe reference material. **Analysis input only — never commit.**
  - `source/Adobe Fireworks CS6/` — full CS6 install (~350MB, ~2100 files): `Fireworks.exe`, DLLs, Plug-Ins (`.x32`), Configuration tree (89 `.jsf` scripts = readable behavioral specs)
  - `source/FireworksPortable/` — PortableApps launcher; intended to run under Wine as the corpus-generation oracle

## source/ rules (non-negotiable)

`source/` holds Adobe's copyrighted binaries and resources; this repo's license covers only FireCraft's own code and docs. `source/` is git-ignored and must stay that way: never `git add` it, never copy its binaries or asset files into commits, releases, or published artifacts.

Adobe-derived text extracts (help-PDF text, exe strings, API inventories) are working notes under `/var/tmp/` — uncommitted, regeneration recipes in PLANNING.md §5.4.

The `AMT/` tree and `amtlib*.dll` are Adobe activation/licensing machinery — out of scope entirely; don't analyze or reimplement them as part of feature work.

## Clean-room method

- Code is written from public specs, observed behavior, and corpus evidence — not from decompiled code pasted in. Ghidra is the RE tool for the PE binaries (primary targets: the `CMkbStructureOutputter` PNG-writer path). Behavioral specs without disassembly: the CS6 `Configuration/` tree's Extendscript `.jsf` files and `HTML Code/` export templates.
- The format ground truth lives in PLANNING.md §6 and `docs/research/fireworks-png-nodes.md`; where the dictionary (observational) disagrees with corpus evidence, corpus wins and record the discrepancy in `docs/decisions/`.
- Follow photocraft's engineering discipline once the workspace exists: layered crates enforced by `xtask layers`, everything-is-a-command registry, no `unwrap`/`panic!` in non-test code, `unsafe_code = "forbid"`, never break the wasm build, rising-only test floors, devlog entry (`log/devlog.md`, gitignored) closes every session.
- Asset rule: originals or permissively-licensed only, with a `docs/ATTRIBUTION.md` row in the same change. Fonts/demo imagery/libraries are never copied from the reference install.
