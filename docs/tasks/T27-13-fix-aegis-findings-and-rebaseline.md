# T27-13 — Fix the genuine Aegis findings, suppress the false positives narrowly, regenerate the baseline

> **RP:** RP-00 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-12

**Objective:** clear the Aegis gate without weakening coverage — fix what is broken,
suppress only what is provably a false positive, and regenerate the baseline from a
full scan rather than from a filtered one.

**Inputs:** the adjudications from T27-12; `.aegis-baseline.json`; `.aegisignore`;
`xtask/src/main.rs` (`aegis()`).

**Authority:** the repo's rebaseline rule — regenerate the baseline via
`--output-file` from a full scan **before** `xtask ci`; never seed a baseline from a
filtered scan, because that silently drops coverage.

**Acceptance criteria:** every genuine finding is fixed; every suppressed finding has a
narrowly scoped, documented suppression; the regenerated baseline is a superset of the
previous one except for the adjudicated entries; the scan is clean.

**Verification:** `aegis-0.6.3 --format json -c production scan . --baseline .aegis-baseline.json -q`

**Evidence:** baseline currently holds 520 entries; 5 findings are not in it.

**Blockers:** T27-12.

**Stopping condition:** the scan is clean, the baseline diff shows only the adjudicated
changes, and `xtask ci` passes.
