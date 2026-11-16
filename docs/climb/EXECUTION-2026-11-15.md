# Execution plan — 2026-11-15

The state of `main` as measured on 2026-11-15, the gaps that measurement
exposes, and the order to close them. Every number here was produced by a
command in this repo or by `git show origin/main:<path>`; where a number
is quoted from an older recording it says so.

This file supersedes the numbers in `README.md` and `SCOREBOARD.tsv` until
those are re-blessed (W0.3). It does not change any rung's definition —
`P0`…`P10` remain the queue.

## 1. Where `main` actually is

`main` = `44aa5466` (2026-11-14, "wave closeout — floor re-pinned to
96.89 %"), GitHub CI green on the last four runs.

| Metric | Recorded in `SCOREBOARD.tsv` | Measured on `main` | Command |
| --- | --- | --- | --- |
| S1 gate | green | green (GitHub ×4) | `gh run list --branch main` |
| S2 shell reachability | 164 / 253 (64.82 %) | **249 / 257 (96.89 %)** | `.reachability-floor.tsv` |
| S6 tests | 1,810 | **1,885** | `git grep -c '#\[test\]'` |
| S7 god files > 2,000 lines | 18 | **12** (largest `shell/mod.rs` 7,993) | line count per `.rs` |
| S8 wasm bundle | 21,271,259 B | 21,608,249 B *(2026-11-10 recording; cap 25 MB warn / 50 MB hard, summit 16 MB)* | `parity.md` |
| S9 deploy traceable | no | **no** — `version.json` → `200 text/html`; missing path → `200 text/html` | `curl` |
| S11 open issues | 70 | **71** | `gh issue list` |
| S12 remote branches | 34 | **150** | `git branch -r` |
| S13 coverage | 81.1 % lines | floors 81.1 / 78.7; measured 82.1 / 79.3 on rustc 1.99.0 (decision 0102) | `coverage.rs` |

Repo shape: 103 decision files, 71 task tickets, 60 in-repo worktrees, 12
tags (three `v0.2.5-dryrun.*`), 5,015 lines across ten planning docs
(`SESSIONS` 2,010; `NEXT` 812).

Two register-level facts matter more than any single number:

* **`docs/climb/` does not exist on `main`.** The whole queue — 174 rungs
  — lives on `docs/audit-handoff-2026-11-13`. `AGENTS.md` on `main` still
  sends readers to `docs/TASKS.md`.
* **`xtask climb` does not exist anywhere.** Nothing emits the table
  above, which is why it disagrees with `main` by 85 commands.

## 2. Gap register

| # | Gap | Evidence | Sev | Closes with |
| --- | --- | --- | --- | --- |
| G1 | Plan off-trunk | `git ls-tree origin/main docs/` has no `climb` | High | W0.3 |
| G2 | Baseline fiction | S2 164 vs 249; S12 34 vs 150; C0.02–C0.05 "blocked" but #142/#143/#144/#113 merged | High | W0.3 |
| G3 | No altimeter | no `xtask climb`; metrics hand-copied | High | W1.1 |
| G4 | Local gate ≠ shipping gate | `run_ci` never calls `panic-hunt` (0 hits); `.gitforge.yml:194` comment claims the drivers come from `xtask ci` | High | W1.2 |
| G5 | No dependency audit | no `cargo audit`/`deny` in `xtask ci` or `.gitforge.yml` | High | W1.2 |
| G6 | Deploy untraceable, 404s disguised | live `curl` returns the SPA shell for every path | Med | W2 |
| G7 | Reachability story stale in three places | `GAPS.md` §16 "172 bound / 85 unreachable"; `TASKS.md` "8 of 85 left"; issues #136/#137/#139/#140/#141 | Med | W0.4 |
| G8 | One page, two claims | `TASKS.md` T18 queued `RECONCILED — REPUBLISH PENDING` beside the in-flight row saying the live bundle matches no release | Med | W0.4 |
| G9 | Duplicate decision numbers on `main` | `0094`×2, `0096`×2, `0101`×2 | Low | W0.2 |
| G10 | Phantom layer crates on `main` | `layers.rs:26–37` registers `ops`, `gpu`, `automation`, `testkit`; none exist | Low | W0.2 |
| G11 | Physical debt | 150 remote branches, 60 worktrees, 3 dry-run tags | Med | W5 |
| G12 | Plan-doc sprawl | 5,015 lines / 10 docs, `SESSIONS` 2,010 | Low | W1.3 |
| G13 | Size ceiling vs summit | 21.6 MB against a 16 MB summit target; soft cap already at 25 MB | Strategic | C10 lane |

Doc-level residue detected alongside: only 1 of 6 "Frozen 2026-11-13"
banners exists; CodeRabbit markers remain in `NEXT`, `HANDOFF`,
`CS6-PARITY`, `PROGRESS`; `164 / 64.82 / 59.59` still appear in
`HANDOFF`, `PRODUCTION-READINESS-PLAN`, `SESSIONS`, decision 0087 and
`wiki/Automation.md`.

## 3. Root causes

1. **Measurement is manual, so the board rots silently.** G1–G3 are one
   failure. Fix the command, not the numbers: a board that cannot be
   recomputed will drift again within a wave.
2. **The plan was stacked on a branch.** Work was queued against PRs that
   merged without the queue being updated, and the plan is invisible from
   the branch everyone works on.
3. **"Green" has two meanings.** The engine panic fixed in #147 was
   reachable only through `panic_hunt`; `cargo audit` runs nowhere. A local
   green gate does not currently imply a shippable tree.

## 4. The backlog, counted

Extracted from the `###` headers of `P0`…`P10` (`log/rungs.tsv` holds the
generated table):

* **174 rungs** parsed — 173 open, 1 done (`C1.04`).
* **147 rungs are strictly blocked** by another open rung (needs graph).
* **26 are dependency-ready**: 24 agent-executable, 2 maintainer-gated
  (`C0.11`, `C1.06`).
* **18 rungs require `[MAINTAINER]`**, 73 require a `[HOST:browser]` proof.
* Size mix: 40 XS, 75 S, 51 M, 8 L — **382 effort units** (XS 1 · S 2 ·
  M 3 · L 5).

Lane totals (open): C0 12, C1 6, C2 9, C3 18, C4 20, C5 22, C6 30, C7 16,
C8 11, C9 22, C10 7.

The depth is the finding: four fifths of the queue cannot start. Anything
that unblocks — trunk truth, the altimeter, the deploy — is worth more per
hour than any individual panel.

## 5. Waves

### W0 — trunk truth (agent; ~2 days)

| Step | Work | Exit check |
| --- | --- | --- |
| W0.1 | Fresh `main` worktree. The two existing scans are 25 and 16 commits behind, so nothing can be measured in them. | `git rev-list --count HEAD..origin/main` is 0 |
| W0.2 | Retarget #147 to `main`: keep the `breakApart` fix + pinned seed, the `io` lint restoration, the phantom-layer removal, the decision renumbering, `reachability --list`; **drop** the S2→168 doc hunk (superseded by 96.89 %). | gate green on `main`; `reachability --list` prints 8 ids |
| W0.3 | Land `docs/climb/` on `main`; re-baseline `SCOREBOARD.tsv` from W0.1's measurement; mark C0.02–C0.05 `[x]` with merge SHAs. | every number in the board reproducible by one command |
| W0.4 | Contradiction sweep: `GAPS.md` §16, `TASKS.md` "8 of 85", the T18 row, issues #136–#141, the five missing frozen banners, the CodeRabbit markers, the 164/59.59 quotes. | `git grep -n '164/253\|64.82\|59.59'` empty outside history docs |

### W1 — measurement spine (agent; ~1–2 days)

1. `xtask climb`: one verb prints S1–S13. `climb --check` runs inside
   `ci` and fails on any metric below the committed baseline.
2. `ci` gains `panic-hunt` and `cargo audit`; fix the stale `.gitforge.yml`
   comment in the same change. New finding surfaces are pinned in the
   return shape.
3. `SESSIONS.md` archived; `NEXT.md` shrinks to the open queue (G12).

Status 2026-11-15: the verb exists in the audit worktree — `climb`
measures S2–S8/S12 from the tree, `climb --append` records a row, and
`climb --check` ratchets them. Its first run reports `S12_branches`
34 → 123 against the recorded baseline, so the ratchet is already
finding G11. It is deliberately not wired into `ci` yet: the baseline it
compares against is still the 164 board (W0.3).

**Exit:** deliberately lowering a metric fails `ci`; a vulnerable advisory
fails `ci`; `SESSIONS.md` is no longer a second ledger.

### W2 — deploy truth (agent; ~half day)

Commit `version.json` (build SHA, tag, date), `_headers`, and an SPA
fallback that returns real 404s for missing assets (C3.01/C3.02).

**Exit:** `curl …/version.json` returns provenance JSON; a nonexistent
path returns 404; the `site/` page and `dist/web` load in a browser
(HANDOFF §6.2).

### W3 — the ready rungs (agent; ordered by value per hour)

Non-browser, cheap, immediately useful:

1. `C0.01` confirm the pinned toolchain (XS) — required before W0.2's gate
   is meaningful.
2. `C9.01` bring `ARTIFACT-MATRIX` up to date (XS).
3. `C7.04` coalesce rapid edits into one undo step (S) — undo is the most
   used interactive surface.
4. `C6.02` Crop Selected Bitmap, `C6.20` selection history,
   `C6.26` custom text size + vertical alignment, `C6.28` preview browser
   selection (all S/XS, no browser proof).

Browser-gated, batch by panel so one Playwright run covers several:

5. `C6.03` Path panel, `C6.04` Image Editing panel, `C6.05` Special
   Characters, `C6.06` Info pixel statistics — one panel-proof session.
6. `C6.13` Open Recent, `C6.17` Manage Saved Commands, `C6.19` Edit
   Guides + Snap To Pixel, `C6.27` Print and Page Setup.
7. `C6.01` Path Scrubber, `C6.08` Common Library, `C6.12` Inspector
   remainder (#88), `C6.18` Preferences tabs, `C6.21` Paste Attributes /
   Copy as Vectors, `C6.25` spell check, `C7.09` touch gestures.
8. `C5.11` mask commands (#140, 4 remaining).

### W4 — lanes, once W0–W2 are green

* **C4 export (20 rungs, none started)** — the largest user-visible gap and
  the blocker behind the Optimize panel's residuals. Begin `C4.01`–`C4.06`
  (PNG8/24/32, Adam7) and `C4.07`–`C4.09` (selective JPEG).
* **C6 remainder (30 total)** — finish the inspector/window surface.
* **C7 (16)** — undo, history, gestures, document lifecycle.
* **C3 web (18)** — bridge origin checks, IndexedDB autosave, FS Access,
  PWA; unblocked by W2.
* **C5 (22)**, **C8 (11)**, **C9 (22)**, **C2 (9)** — quality and release
  depth; C10.01 (summit review, S) is ready now but only meaningful last.

### W5 — maintainer queue (prepare, do not execute)

C0.09 prune (150 branches, 60 worktrees, 3 dry-run tags), C0.11 marketing
deploy, C1.06 release scope (decision 0075), C3.03 Cloudflare secrets,
C9.17 signing identities. These are 18 of the rungs and every one is a
`[MAINTAINER]` tag — they gate C9 and C3 almost entirely.

## 6. Sequencing rules

1. Nothing starts before W0.3; a rung landed against a fictional baseline
   makes the next comparison meaningless.
2. One rung per PR, with its `docs/TASKS.md` row and the board touched in
   the same change (AGENTS §Workflow 2).
3. A lane whose rungs are all `[MAINTAINER]` does not get started by an
   agent; it gets a decision request.
4. Browser rungs batch per panel: one served `dist/web` + Playwright run
   per group, not per rung.
5. `climb --check` (W1.1) must fail the build before W4 begins, or W4 will
   reproduce G2 by hand.

## 7. Open questions carried, not answered

* Whether `panic_hunt` is green at `44aa5466` — it is not in `xtask ci`, so
  no local run has asserted it since the merge wave.
* The current wasm size at `main` (last recording 2026-11-10); it needs a
  `trunk` build, not a re-read of `parity.md`.
* The identity of the 8 unbound commands — `reachability --list` prints
  them, and that lands with W0.2.
