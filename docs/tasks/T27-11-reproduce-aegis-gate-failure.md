# T27-11 — Reproduce the PR #9 Aegis gate failure and enumerate the offending findings

> **RP:** RP-00 · **Priority:** P0 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** none

**Objective:** reproduce the red `gate` job locally with the pinned Aegis version and
enumerate every finding that is not in the committed baseline, with file and line.

**Inputs:** `aegis-0.6.3` (pinned by SHA in `.github/workflows/ci.yml`); the command
`xtask` ran at the time: `aegis --format json -c production scan . --baseline .aegis-baseline.json -q`;
`.aegis-baseline.json` (520 entries); `.aegisignore`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5; the repo rule that a gate result
is read from the store, not from a remembered watcher.

**Acceptance criteria:** the failure reproduces on the candidate tree and every
offending finding is enumerated with its rule id, file and line.

**Verification:** `aegis-0.6.3 --format json -c production scan . --baseline .aegis-baseline.json -q`

> **Note (2026-11-10):** the baseline named above was the retired line-keyed
> `.aegis-baseline.json`. Decision 0074 content-keys the gate; the committed
> baseline is now `.aegis-baseline.tsv` and the current gate command is
> `cargo run -p xtask -- aegis`. The reproduction recorded below is the historical
> one, taken on 2026-11-08 against the baseline of that day.

**Evidence:** 5 new findings vs a 520-entry baseline (2026-11-08) — adjudicated in [decision 0074](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/decisions/0074-aegis-content-keyed-baseline.md) **Done:** 2026-11-08.

**Blockers:** —

**Stopping condition:** the finding list exists with file/line for each entry.
Completed 2026-11-08.
