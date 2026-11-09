# T27-13 — Fix the genuine Aegis findings, suppress the false positives narrowly, regenerate the baseline

> **RP:** RP-00 · **Priority:** P0 · **Status:** IMPLEMENTED
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

**Evidence:** baseline held 520 entries; 36 findings were not in it and 2 had moved
lines.

**Blockers:** none — T27-12 is complete.

**Stopping condition:** the scan is clean, the baseline diff shows only the adjudicated
changes, and `xtask ci` passes.

## Outcome

T27-12 classified every finding as a verified false positive, so there was no genuine
finding to fix in code. The baseline was re-pinned by `scripts/aegis_repin.py`, which:

- reads a **full** scan (`--output-file` on an unfiltered scan), never a filtered one,
  so no coverage is silently dropped;
- adjudicates every finding against the table in T27-12, and **refuses to write any
  finding that no row covers**, so an unreviewed entry cannot enter the baseline;
- matches a finding to a row by rule name and by the file the row is about, **not by
  line number** — the baseline keys on `pattern:file:line:content-digest`, so a
  line-keyed table would be invalidated by every row added to it, and the ticket's own
  table is itself a source of findings. Matching by rule and file makes the table
  self-consistent and extendable without a re-pin;
- keeps the committed entries in place, in the committed order, with their committed
  `id` values — the `id` field is a per-scan nonce while `stable_id` is the stable
  key, so replacing every entry would churn all 520 for the sake of a few real changes;
- preserves the 76,440-unit inspection ledger, because regenerating the whole file
  produces a 440,980-line deletion diff (the local scan environment differs from
  CI's) and a ledger that no longer describes the tree the gate runs against;
- re-scans against the baseline it just wrote and exits non-zero if any finding
  survives, so a green run is a verified claim rather than a written one.

No rule class was disabled and no scanner coverage was reduced.

Verification:

```
python3 scripts/aegis_repin.py
  → Verification: the scan reports no findings.
```

Baseline diff: 36 entries added (the adjudicated findings, including the ticket's own),
2 entries removed (the `xtask/src/dist.rs` findings whose lines moved when the file was
reformatted), and nothing else. The inspection ledger is byte-identical.

Completed 2026-11-09.
