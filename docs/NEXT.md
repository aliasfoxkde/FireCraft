# FireCraft — Next-agent plan (2026-10-18)

> Start at `docs/HANDOFF.md` first. This file is the live backlog
> and the expanded plan. HANDOFF §4 has the phase descriptions.
> Updated 2026-10-18 with quality assessment; Aegis/coverage/ci workflows added.

---

## Honest state of the repo (2026-10-18 assessment)

### What's solid
- **CI/CD**: GitForge pipeline (7 jobs) + GitHub Actions backup (`.github/workflows/ci.yml`)
  + GitHub Actions release workflow (`.github/workflows/release.yml`). Local gate:
  `cargo run -p xtask -- ci` (aegis → coverage → fmt → clippy → test →
  layers → wasm → corpus → perf → fuzz → parity → scorecard → changelog).
- **Clippy**: all crates clean — zero warnings, zero errors across workspace.
- **Fmt**: all crates green including the workspace-excluded `web/` (explicit
  check in the fmt gate).
- **Tests**: 681 test functions, 77 test modules across the workspace.
  Corpus round-trip: 21/21 files verified.
- **Aegis**: in xtask ci chain — aegis 0.6.3 production profile, 1316 findings
  in baseline (all in research docs). Gate: `cargo run -p xtask -- aegis`.
- **Coverage gate**: in xtask ci chain — llvm-cov workspace-wide, 60% thresholds.
  Gate: `cargo run -p xtask -- coverage [--bless|--measure]`.
- **Wasm build**: fwpng wasm32 release compiles cleanly.
- **CHANGELOG**: auto-generated from 211 commits, version-stamp verified.
- **Lockfiles**: clean, no external crate version drift.
- **Documentation**: current — HANDOFF, PROGRESS, HISTORY, SESSIONS, README,
  CLAUDE.md, AGENTS, WEBOS all aligned to v0.2.1 / post-parity reality.
- **Clean-room**: `source/` untouched, AMT/ out of scope, no git-unsafe ops.

### What's missing or behind

#### CI gaps (all resolved)
1. ~~No GitHub Actions backup~~ — `.github/workflows/ci.yml` added.
2. ~~No Aegis in xtask ci~~ — integrated as the first gate step.
3. ~~No coverage gate~~ — `coverage.rs` module added with 60% thresholds.
4. **Wasm gate slow** — `cargo build --target wasm32-unknown-unknown` takes
   >5 min even with incremental. The gate compiles full workspace for wasm.
   The `web/` crate uses trunk (not workspace) so it builds separately.
   Current behavior is correct but the timeout is tight.

#### Coverage gaps
4. **`io` crate (6015 lines)**: zero `#[cfg(test)]` modules, zero
   `*_test.rs` files. The corpus round-trip gate (21 files) covers the
   read/write paths at the integration level, but unit-level correctness
   (error paths, edge cases in each codec) is not individually verified.
   Priority: bmp, jpeg, tiff, wbmp codecs have no unit tests.
5. **`text` crate (853 lines)**: zero test modules. Text layout is
   deterministic (contract enforced by font pinning) but error paths and
   edge cases in TextEngine and glyph contour generation are untested.
6. **`scripting` crate (2113 lines)**: has integration tests (44 tests
   across 5 files: dom, files, hostile, mutate, widget) but no unit
   tests in `#[cfg(test)]` modules inside the src tree. The integration
   tests cover the DOM surface, hostile input, and mutation. Acceptable
   but incomplete.
7. **Coverage not measured**: llvm-cov (cargo-llvm-cov 0.9.0) is available
   but no coverage gate exists in xtask ci. The backend-fixed repo has
   99.1% line / 98.8% function / 98.9% region thresholds. FireCraft
   does not measure coverage.
8. **web crate**: workspace-excluded (trunk). No clippy, no test, no
   coverage for `web/src/`. The fmt gate catches formatting. The browser
   smoke (playwright against dist/web) is the functional proof.

#### Documentation gaps
9. **`RELEASING.md`**: accurate but could reference the full v0.2.1 release
   recipe (8 stamps, wrangler deploy, etc.) — currently only covers the
   native set partially.
10. **`TOOLCHAINS.md`**: exists but may not reflect current state of
    platform build capabilities.
11. **No `docs/releases/`**: release notes are gitignored staging
    (`dist/release-notes.txt`). Tracked history would survive.
12. **`docs/planning/firecraft/`**: milestone tickets M0–M15 still present
    as reference. Useful but needs a note that they are historical.

#### Architecture gaps
13. **`webexport` (3 modules, 15 inline tests)**: has `rasterize` returning
    `Result` but the only `unwrap()` in the module is inside a `#[cfg(test)]`
    block. Production code path is panic-free. ✓
14. **No GitHub release automation**: releases are manual (gh CLI). No
    GitHub Actions triggered on tag. Consider a workflow that runs
    `xtask dist` and creates a draft release on tag.

---

## Prioritized task list

### Tier 1 — CI/CD hardening (DONE)
- **#136** ✅ Aegis scan in xtask ci — first gate step, aegis 0.6.3 production profile.
  Gate: `cargo run -p xtask -- aegis`.
- **#137** ✅ llvm-cov coverage gate — 60% conservative thresholds, `--lcov` parsing.
  Gate: `cargo run -p xtask -- coverage`.
- **#138** ✅ GitHub Actions backup CI (`.github/workflows/ci.yml`) + release workflow
  (`.github/workflows/release.yml`).
  - Auth via `GITHUB_TOKEN`; no additional secrets needed.

### Tier 2 — Test coverage (target 90%+)

- **#139** Test `io` crate (6015 lines — highest priority)
  - Add `#[cfg(test)]` modules to each codec: bmp, jpeg, tiff, wbmp,
    png, gif, svg, svgimport, psd, expv, webp.
  - Test error paths: malformed headers, truncated data, dimension caps.
  - Test raster export/import round-trip per format.
  - The corpus gate covers the happy path; unit tests cover the edge cases.

- **#140** Test `text` crate (853 lines)
  - Test TextEngine: empty string, single glyph, multi-run.
  - Test auto_block: zero-size, very large, negative coords.
  - Test glyph_contours: ensure determinism (same input → same output).

- **#141** Unit tests for `scripting` crate
  - Add `#[cfg(test)]` modules for host, dom, marshal, widget
    (separate from the integration tests in `tests/`).
  - Test error handling in host functions.

### Tier 3 — Documentation hygiene

- **#131** Expand `RELEASING.md` with full v0.2.1+ recipe
  - Document the 8-stamp version bump, xtask changelog, dist native+web,
    zip at root, SHA block, tag, push, gh release, wrangler deploy.
  - Reference HANDOFF §6.3 for the authoritative recipe.

- **#142** ✅ Create `docs/releases/` and archive v0.2.0 + v0.2.1 notes.
  `docs/releases/v0.2.0.md` and `v0.2.1.md` committed.

- **#147** Audit `TOOLCHAINS.md` for accuracy
  - Verify platform build capabilities reflect current state.
  - Add results from any platform recipe testing (#134).

### Tier 4 — Architecture improvements

- **#143** ✅ GitHub Actions release workflow committed
  (`.github/workflows/release.yml`).

- **#130** Smoke timeout budget decision doc (HANDOFF #130)
  - Document the `--virtual-time-budget=60000` constant and its origin.
  - Define the threshold for when to increase it and by how much.

### Recurring (run before any web/shell change)

- **#126** Browser smoke against live Pages URL
- **#127** Cloudflare Pages redeploy after web/shell change

### Staged work (HANDOFF tasks #121–#134, still valid)

These remain the primary feature direction. See their descriptions in
HANDOFF §4. Priority order: #121 (quick) → #123 (decision first) →
#122 → #124 → #125 → #131 → #130 → #132/#133 (evidence-gated, parked) →
#129 → #134.

---

## What NOT to do

- **Do not add aegis baseline without scanning first.** The baseline is
  already committed (0 findings). Re-generate only if aegis version changes.
- **Do not run `cargo test --workspace` directly** — use the job manager
  or `cargo run -p xtask -- ci`. Hooks deny workspace-wide test in ad-hoc
  shells.
- **Do not touch `source/`** — Adobe's binaries are reference-only.
- **Do not commit `dist/`** — it is gitignored. Artifacts are local.
- **Do not push `--force`** — the git hooks block it.
- **Do not use `rm -rf` compounds** — deny-listed.

---

## Quick reference

| Gate | Command | Timeout |
|---|---|---|
| Full CI | `cargo run -p xtask -- ci` | ~20 min |
| Aegis | `cargo run -p xtask -- aegis` | ~10s |
| Coverage | `cargo run -p xtask -- coverage [--bless\|--measure]` | ~5 min |
| Wasm only | `cargo run -p xtask -- wasm` | ~5 min |
| Perf | `cargo run -p xtask -- perf` | ~2 min |
| Fuzz | `cargo run -p xtask -- fuzz-quick` | ~2 min |
| Dist | `cargo run -p xtask -- dist --native && dist --web` | ~5 min |

| Aegis findings (baseline) | 1316 (all in research docs) |
| Clippy warnings | 0 |
| Fmt violations | 0 |
| Tests | 681 functions |
| Test modules | 77 |
| Untested crates | io (6015 lines), text (853 lines) |
| Integration-tested crates | scripting (44 tests) |
| Web bundle | wasm32, 12.7 MB, bridge smoke green |
| Live site | https://firecraft-web.pages.dev (v0.2.1) |
