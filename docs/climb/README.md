# FireCraft Hill Climb — the ordered path to v1.0

> **Created 2026-11-13** from a full audit of `main` @ `1ac3dbc` (see
> [`AUDIT-2026-11-13.md`](AUDIT-2026-11-13.md)). This directory is the
> **single ordered work queue** for FireCraft from here to a production
> v1.0 at Fireworks CS6 parity plus modern features. It replaces the
> overlapping plan documents (NEXT, GAPS §-phases, HANDOFF §4,
> PRODUCTION-READINESS-PLAN, RELEASE-ENGINEERING-PLAN, EXECUTION-BOARD) as
> the place to pick work from. `docs/TASKS.md` stays the live status board;
> `AGENTS.md` stays the operating manual and its rules still bind you.

## What a "hill climb" means here

The project is a hill. Its height is measured by the **scoreboard** (below):
a fixed set of numbers, each with one command that measures it. Work comes
in **rungs**: small, ordered, independently verifiable tasks. Every rung
names the scoreboard metric it moves and the command that proves it moved.

**The one law: the scoreboard never goes down.** A rung that improves its
target metric while worsening another one is not done. Fix it or revert it.
This is the repo's existing "floors are rising-only" rule
(`.reachability-floor.tsv`, `parity.md`, the menu-census tests) applied to
the whole project.

## The climbing loop (do exactly this, every rung)

1. **Sync.** `git checkout main && git pull --ff-only`. Read this file,
   then the phase file you are working in.
2. **Measure.** Run the scoreboard. Use `cargo run -p xtask -- climb` once
   rung C0.08 has landed; until then, use the manual commands in the
   *Scoreboard* table. Note the values.
3. **Pick.** Take the **lowest-numbered open rung (`[ ]`)** in the
   **lowest open phase of your lane** (see *Lanes*). Pick only a rung that
   meets all three conditions:
   - every rung listed under **Needs** is `[x]`;
   - it carries no unmet `[MAINTAINER]` tag;
   - no open PR already claims it. Check with
     `gh pr list --search "C<id>" --state open`.
4. **Claim.** Make a branch `climb/C<id>-<slug>`, e.g.
   `climb/C4.02-png8-encoder`. Change the rung's box to `[~]` and add your
   PR number once the PR exists.
5. **Do.** Follow the rung's **Steps**. Make the smallest diff that meets
   **Done when**. Do one rung per PR. Don't refactor, rename, or "improve"
   anything outside the rung.
6. **Verify.**
   - Run the rung's **Verify** commands, then `cargo run -p xtask -- ci`;
     both must be green.
   - For any `web/` or shell change, also do the browser proof in
     `docs/HANDOFF.md` §6.2 (playwright against the served `dist/web`).
7. **Re-measure.** Run the scoreboard again.
   - The rung's **Moves** metric must have moved as stated.
   - No other metric may have dropped.
   - If either check fails, do not open the PR as ready. Fix the problem or
     mark the rung blocked (step 9).
8. **Record.** In the same PR:
   - Tick the box `[x]` in the phase file.
   - Append one row to [`SCOREBOARD.tsv`](SCOREBOARD.tsv).
   - Update `docs/TASKS.md` if a tracked item changed.
   - Bless any ratchet the rung raised, e.g.
     `cargo run -p xtask -- reachability --bless`.
   - Commit as `type(scope): summary (C<id>)` with the Co-Authored-By
     trailer. Push to `gitforge` first, then `origin` (AGENTS.md
     workflow §4).
9. **Blocked?** Set the box to `[!]`. Under the rung, add one line:
   `BLOCKED <date>: <reason> — <what would unblock>`. Commit only that, then
   pick the next rung. **Never fake progress.** No placeholder UI, no
   `ReportGap` swapped for a no-op, no lowered floor, no skipped test.

### Status boxes

`[ ]` open · `[~]` claimed (PR #) · `[x]` done · `[!]` blocked (see note) ·
`[-]` dropped by a recorded decision.

### Tags on rungs

| Tag | Meaning |
|---|---|
| `[MAINTAINER]` | Needs a human action: approval, secrets, merge rights, deletes, closing issues, or a production deploy. **Prepare everything** (exact commands, text, evidence), post it as a PR or issue comment, and stop. Do not do the action yourself. |
| `[HOST:linux]` | Needs the Linux build host or the CI image `firecraft-ci-rust:2`. All cargo work needs it; this Windows workstation has no Rust toolchain. |
| `[HOST:browser]` | Needs Playwright/Chromium against `dist/web`. |
| `[HOST:gha-win]` / `[HOST:gha-mac]` | Runs on GitHub-hosted runners (`windows-latest` / `macos-14`) through a workflow. No local hardware is needed. |
| `[EVIDENCE]` | The CS6 behaviour must be established first, from `.jsf` scripts, help text or observed behaviour (clean-room). If you can't establish it, mark the rung `[!]` and write down what evidence is missing. Never invent behaviour. |
| `[DOCS]` | Docs-only. Any host will do; the gate's docs steps still apply. |

## Lanes (parallel tracks)

Agents can climb different lanes at the same time. Inside a lane, go in
order.

| Lane | Phases | Notes |
|---|---|---|
| **trunk** | P0, P1, P2 | Everyone waits on P0. P1 and P2 unblock the other lanes. |
| **web** | P3 | WebOS/browser production. Starts after P0. |
| **export** | P4 | Fireworks' core output workflow. Starts after P0. |
| **shell** | P5, P6 | Command reachability, then CS6 surface. Starts after P2's split rungs, to avoid merge conflicts in god-files. |
| **modern** | P7 | Starts after P3. |
| **quality** | P8 | Starts after P2. Repeatable rungs. |
| **release** | P9 | Starts after C1.06 (scope approval). |
| **summit** | P10 | Last. Everything else at target. |

## Phase map

| Phase | File | Goal (deliverable) | Rungs |
|---|---|---|---|
| P0 | [P0-base-camp.md](P0-base-camp.md) | Clean trunk: open PRs landed or fixed, stale branches/issues pruned, CI release job off branch pushes, the `xtask climb` scoreboard exists | 12 |
| P1 | [P1-one-source-of-truth.md](P1-one-source-of-truth.md) | One status board, frozen history docs, no stale facts, release scope approved | 7 |
| P2 | [P2-structural-health.md](P2-structural-health.md) | God-files split, lint holes closed, toolchain pinned, so many agents can work in parallel | 9 |
| P3 | [P3-web-production.md](P3-web-production.md) | Web build deployed from a tag by CI with provenance; secure versioned WebOS bridge; PWA; real file and clipboard I/O | 18 |
| P4 | [P4-export-parity.md](P4-export-parity.md) | PNG8/24/32 + interlace, selective JPEG, @2x, presets, PDF, modern HTML/CSS, HTML Setup/Update, Batch Process GUI | 20 |
| P5 | [P5-reachability.md](P5-reachability.md) | All 253 registry commands reachable from the designed UI, or recorded as script-only by decision | 22 |
| P6 | [P6-cs6-surface.md](P6-cs6-surface.md) | Missing CS6 tools/panels/dialogs, chrome, inspector, animation/mask residuals, shortcut editor, preferences, ReportGap burn-down | 30 |
| P7 | [P7-modern-features.md](P7-modern-features.md) | Light theme, accessibility, undo coalescing, i18n, ICC, OpenType, touch, design tokens, scripting API docs | 16 |
| P8 | [P8-quality-ratchet.md](P8-quality-ratchet.md) | Coverage floors, missing_docs, panel snapshots, wasm size trim, load-time budget, fuzz anchors | 14 |
| P9 | [P9-release-matrix.md](P9-release-matrix.md) | Every in-scope artifact built on CI, smoke-tested with receipts, checksummed, SBOM + provenance | 22 |
| P10 | [P10-summit.md](P10-summit.md) | v1.0.0 release certification against the summit criteria | 7 |

## Scoreboard

Baseline measured **2026-11-13 on `main` @ `1ac3dbc`**. History lives in
[`SCOREBOARD.tsv`](SCOREBOARD.tsv). ↑ = may only rise, ↓ = may only fall.

| # | Metric | Dir | Baseline | Summit target | How to measure (until `xtask climb` exists) |
|---|---|---|---|---|---|
| S1 | Gate green on `main` | = | green (GitHub backup CI, last 15 runs) | green | `cargo run -p xtask -- ci` |
| S2 | Shell-bound registry commands | ↑ | 168 / 253 (66.40 %) | 253 minus script-only-by-decision = 100 % | `cargo run -p xtask -- reachability` (stderr line) |
| S3 | `ReportGap(` sites in menus | ↓ | 52 | each remaining one has a decision row | `grep -c 'ReportGap(' crates/app/src/menus.rs` |
| S4 | Inert menu rows | ↓ | 2 of 296 | 0 | `cargo test -p app menus::tests::inert_count_never_rises` (pinned value in test) |
| S5a | CS6-PARITY rows **implemented** | ↑ | 82 | ≥ 115 | `grep -oE '\| \*\*implemented\*\*' docs/CS6-PARITY.md \| wc -l` |
| S5b | CS6-PARITY rows **partial** | ↓ | 25 | ≤ 5 (each with decision) | same grep with `partial` |
| S5c | CS6-PARITY rows **missing** | ↓ | 11 | 0 (others → *not applicable* by decision) | same grep with `missing` |
| S6 | `#[test]` functions | ↑ | 1,810 | rising | `grep -r '#\[test\]' crates xtask web --include=*.rs \| wc -l` |
| S7 | `.rs` files > 2,000 lines | ↓ | 18 | ≤ 8 (catalog data files only) | `find crates web xtask -name '*.rs' \| xargs wc -l \| awk '$1>2000 && $2!="total"' \| wc -l` |
| S8 | Release wasm, raw bytes | ↓ | 21,271,259 (last recorded, SESSIONS.md) | ≤ 16,000,000 | `cargo run -p xtask -- dist --web` then `stat -c %s dist/web/*.wasm` |
| S9 | Live web deploy traceable to a commit | = | **no** (#30) | yes | `curl -s https://firecraft-web.pages.dev/version.json` (after C3.01) |
| S10 | Release artifacts with a SMOKE TESTED receipt | ↑ | 0 certified; Linux tar/CLI/deb/web published | all in-scope (see P9) | `docs/release/ARTIFACT-MATRIX.md` |
| S11 | Open issues | ↓ | 70 | only post-v1 enhancements | `gh issue list --state open --limit 500 --json number \| jq length` |
| S12 | Remote branches besides `main` | ↓ | 34 | only open-PR branches | `git branch -r \| grep -vc 'main$\|HEAD'` |
| S13 | Workspace line coverage | ↑ | 81.1 % line / 78.7 % fn (gate floors in `xtask/src/coverage.rs`, measured 2026-10-30) | ≥ 85 % line, plus per-crate floors | `cargo run -p xtask -- coverage` |

## Numbering map (old schemes → climb)

Several numbering schemes collide in the old docs. Refer to work by its
**climb ID** from now on, and cite the old ID only for traceability.

| Old scheme | Where | Climb home |
|---|---|---|
| M0–M15 milestones | `planning/firecraft/ROADMAP.md` | Closed history. M12's unmet macOS/Windows gate → P9 |
| "Phase 14" (coverage) / "Phase 15" (missing_docs) | NEXT.md | P8 |
| "Phase 14/17" (symbols/Optimize) | GAPS.md, a **different** meaning | P4 / P6 |
| "Phase 13" (release hygiene) | HANDOFF.md §4 | P9 |
| Phase 20.x (daily driver), 22.x (shell fidelity) | NEXT.md, TASKS.md | P6 |
| Phase 21 (modern) | NEXT.md | done; follow-ons in P7 |
| T2, T3, T7, T8, T18, T20, T22, T23, T26 | `docs/tasks/` | named in each rung |
| T27-01…61, RP-00…09, WP-00…07 | release docs | P9 (one rung per real deliverable) |
| Issues #136–#141 | GitHub | P5 |

## Ground rules that cheap agents most often break

- **Clean-room.** Never open, copy or commit anything from `source/`. Never
  decompile. Base behaviour on `.jsf` scripts, help text and observation, and
  write the evidence into a `docs/decisions/NNNN-*.md` record (the next free
  number is above **0112**; check with `ls docs/decisions | tail -3`).
- **No `unwrap`/`expect`/`panic!`/`todo!`** in non-test code. Hostile
  sizes take caps and return `Result` (AGENTS.md "Numbers are hostile").
- **Never break wasm.** No C dependencies or threads in core crates.
  `xtask ci` checks `wasm32-unknown-unknown`.
- **No versioned files** (`foo_v2.rs`). No duplicate implementations. Reuse
  what exists. Search first with `git grep`.
- **Every non-code asset** (icons, templates, fonts, sample art) gets a row in
  `docs/ATTRIBUTION.md` in the same change. Use only originals or
  permissive licences.
- **Git:** no `git stash`, no `reset --hard`, no `push --force`, no
  `rebase -i`. To update a PR branch, `git merge origin/main` into it.
- **Protected files:** do not edit `CLAUDE.md` or `.claude/` unless the
  maintainer unlocks them (rung C1.05).
- **Ratchets:** run `--bless` only when your rung *raised* a floor. Bless
  verbs refuse to lower, and you must not work around that.

## Maintainer decision queue (humans only)

Agents prepare these; the maintainer decides. Each one blocks the rungs
listed.

| Decision | Blocks | Rung |
|---|---|---|
| Merge the three open PRs (#143 → #113 → #144; #142 landed as `ddf19fb4`) | P5, P6 | C0.02–C0.05 |
| Approve deleting 30 stale branches + 3 dry-run tags | S12 | C0.09 |
| Approve closing/re-scoping ~15 issues | S11 | C0.10 |
| Add Cloudflare secrets (`CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`) | site deploy, P3 deploy | C0.11, C3.03 |
| Approve release scope (decision 0075) | all of P9 | C1.06 |
| Unlock `CLAUDE.md` for its status paragraph | — | C1.05 |
| Production web deploy from a tag | S9 | C3.04 |
| Code-signing identities (Apple Developer ID, Windows Authenticode) | signed P9 artifacts | C9.17 |
| Proposal #1 (incubation under artcraft-candidates) | — | C0.10 |
