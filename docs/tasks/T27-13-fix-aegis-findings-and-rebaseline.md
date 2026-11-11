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

> **Note (2026-11-10):** the baseline named above was the retired line-keyed
> `.aegis-baseline.json`. Decision 0074 content-keys the gate; the committed
> baseline is now `.aegis-baseline.tsv` and the current gate command is
> `cargo run -p xtask -- aegis`. The re-pin recorded below is the historical one,
> taken on 2026-11-09 against the baseline of that day.

**Evidence:** baseline held 520 entries; 36 findings were not in it and 2 had moved
lines.

**Blockers:** none — T27-12 is complete.

**Stopping condition:** the scan is clean, the baseline diff shows only the adjudicated
changes, and `xtask ci` passes.

## Outcome

T27-12 classified every finding as a verified false positive, so there was no genuine
finding to fix in code. The baseline was re-pinned by the repo's own gate:

```
cargo run -p xtask -- aegis --bless
```

which:

- reads a **full** scan, never a filtered one, so no coverage is silently dropped;
- keys every record on the **content key** `(pattern, file, content digest)` — not on a
  line number — so a finding that moved lines is not new, while a new literal, a new
  file, or an extra copy of a known literal still is, and still fails the gate;
- records, above each rule class's rows, the reason those occurrences were adjudicated
  as false positives, so the reason travels with the rows it covers and a reviewer can
  read the baseline without leaving the file;
- keeps every rule class armed: nothing is switched off and no scanner coverage is
  reduced;
- prints every key it adds and every key it drops, and each addition must be verified
  as a false positive against T27-12's table before the rewrite is committed.

No rule class was disabled and no scanner coverage was reduced.

Verification:

```
cargo run -p xtask -- aegis
  → [xtask] aegis ok
```

Baseline diff: the adjudicated findings were added (including this ticket's own rows,
whose rule identifiers and quoted literals trip the classes they explain), the
`xtask/src/dist.rs` findings whose lines moved when the file was reformatted are not
new and are not re-admitted, and nothing else changed.

**Superseded (2026-11-10).** This outcome originally recorded the re-pin as a helper
script, `scripts/aegis_repin.py`, against the retired line-keyed
`.aegis-baseline.json`. Decision 0074 content-keys the gate and moved the committed
baseline to `.aegis-baseline.tsv`, so that script no longer had a baseline to read or
write. The tool-level fix supersedes the procedure, so the script was removed in the
same change that took it, and this ticket now names the tool.

Completed 2026-11-09.
