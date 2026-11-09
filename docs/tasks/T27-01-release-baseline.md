# T27-01 — Record the repository release baseline

> **RP:** RP-00 · **Priority:** P0 · **Status:** IN PROGRESS
> **Board row:** [EXECUTION-BOARD.md §2](../release/EXECUTION-BOARD.md)
> **Depends on:** none

**Objective:** produce one dated, verifiable statement of the repository's release
baseline — `main` SHA, workspace version, toolchain inventory, tag/release inventory,
and the true state of every artifact — so every later row is anchored to a known point.

**Inputs:** `git log --oneline origin/main`; `git tag`; `gh release list`;
`gh release view v0.2.5 --json assets`; `docs/TOOLCHAINS.md`;
`docs/release/ARTIFACT-MATRIX.md`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §7 RP-00; campaign §15 RP-00
completion criteria. The campaign's stated baseline is `main` at
`5fac7a2267ebc6b61432d5302ab6b02a1ff15c0f`, version `0.2.5` — re-inspect, do not
assume it still holds.

**Acceptance criteria:** a dated baseline report states the `main` SHA, workspace
version, fleet and operator toolchains, every tag mapped to its commit, every
published asset with its size and URL, and each artifact's true state — with the
command that produced each fact.

**Verification:** `git rev-parse origin/main && cargo run -p xtask -- ci`

**Evidence:** `docs/release/EXECUTION-BOARD.md` §2 (baseline snapshot, verified
2026-11-08): `main` = `5fac7a2`; tag `v0.2.5` → `3a4832b`; 5 published assets.

**Blockers:** the canonical `xtask ci` gate has not yet been re-run on the release
branch — see T27-05.

**Stopping condition:** the baseline report exists in `docs/release/EXECUTION-BOARD.md`
§2 with a verification date, and every fact carries the command that produced it.
