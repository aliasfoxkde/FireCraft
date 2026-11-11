# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

FireCraft: a clean-room reimplementation of Adobe Fireworks CS6 in pure Rust (Apache-2.0), modeled on the StoryTold/ArtCraft projects (github.com/storytold — photocraft et al.). Primary release target is a **WASM build for the user's WebOS** (live at https://firecraft-web.pages.dev), plus a wide desktop/CLI release matrix. The reference application is Adobe Fireworks CS6 (v12.0).

**Status (2026-11-07 PR branch):** M0–M15 and the DOM verb-surface campaign are complete. The latest tag is v0.2.5; `main` remains at the documented 5fac7a2 baseline while PR #8 carries the current shell/editor work. The registry has 245 commands. The live menu census is **2 inert rows of 296**; 8 additional rows remain feature- or evidence-gated and must not be wired without a model/reference basis. The 19.6 wiring campaign is complete. Phase 22.1 (panel inventory) and 22.2 (tool strip) are complete; 22.5 (CS6 chrome) remains partial. Phase 20 is in progress (20.1 landed); Phase 21 is queued. Use `docs/TASKS.md` for the live task ledger, `docs/NEXT.md` for phase criteria, and `docs/PROGRESS.md` for the dated snapshot. Start with `AGENTS.md`, then the live task ledger:

1. `AGENTS.md` — repository rules, clean-room policy, and workflow
2. `docs/TASKS.md` and `docs/NEXT.md` — current work and detailed roadmap
3. `docs/HANDOFF.md` — agent orientation and operational runbooks
4. `docs/PLANNING.md` — format spec, architecture, and decisions

Update `docs/TASKS.md` and its ticket as work changes. Session history belongs in `docs/SESSIONS.md`; append local details to `log/devlog.md`.

Do not assume global Harness profiles or commands from another repository apply here. This workspace already exists; this repository's gate is `cargo run -p xtask -- ci` (formatting, clippy with `-D warnings`, tests, layer enforcement, and wasm build).

## Repository layout

- `docs/README.md` — documentation map; `docs/TASKS.md` — live task ledger
- `docs/HANDOFF.md` — agent orientation and CI runbook
- `docs/PROGRESS.md` — dated status · `docs/SESSIONS.md` — session log · `docs/HISTORY.md` — project arc
- `docs/PLANNING.md` — master planning document (see above)
- `docs/planning/firecraft/` — ROADMAP.md + per-milestone tickets (objective / inputs / authority / stopping condition / verification)
- `docs/planning/generated/` — `parity.md` / `scorecard.md` (xtask-owned) + `dom-parity.md` (CS6 verb-surface matrix; regen recipe in HANDOFF §5)
- `docs/decisions/` — decision records; write one per family/batch
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
