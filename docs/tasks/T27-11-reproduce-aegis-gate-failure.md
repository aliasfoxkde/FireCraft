# T27-11 — Reproduce the PR #9 Aegis gate failure and enumerate the offending findings

> **RP:** RP-00 · **Priority:** P0 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** none

**Objective:** reproduce the red `gate` job locally with the pinned Aegis version and
enumerate every finding that is not in the committed baseline, with file and line.

**Inputs:** `aegis-0.6.3` (pinned by SHA in `.github/workflows/ci.yml`); the command
`xtask` runs: `aegis --format json -c production scan . --baseline .aegis-baseline.json -q`;
`.aegis-baseline.json` (520 entries); `.aegisignore`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5; the repo rule that a gate result
is read from the store, not from a remembered watcher.

**Acceptance criteria:** the failure reproduces on the candidate tree and every
offending finding is enumerated with its rule id, file and line.

**Verification:** `aegis-0.6.3 --format json -c production scan . --baseline .aegis-baseline.json -q`

**Evidence:** reproduced 2026-11-08 — 5 findings absent from the 520-entry baseline:
`gdpr-personal-data` (`docs/RELEASE-ENGINEERING-PLAN.md:155`), `pci-cardholder-data`
(`docs/research/android-feasibility.md:62`), `ssrf-localhost` (`xtask/src/dist.rs:1040`),
`zip-code` (`xtask/src/dist.rs:468`, `xtask/src/dist.rs:960`).

**Blockers:** none.

**Stopping condition:** the finding list exists with file/line for each entry.
Completed 2026-11-08.
