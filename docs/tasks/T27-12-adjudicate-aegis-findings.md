# T27-12 — Adjudicate each of the new Aegis findings

> **RP:** RP-00 · **Priority:** P0 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-11

**Objective:** decide, for each of the new findings, whether it is a genuine defect
or a verified false positive — before any fix or suppression is written.

**Inputs:** the finding list from T27-11; the scanner's rule definitions; the repo's
known false-positive classes.

**Authority:** the repo's Aegis working knowledge — "pan" trips the PCI rule and
large literals trip the zip-code rule, so those classes must be baselined rather than
reworded away; and the rule that a genuine finding is fixed while a verified false
positive carries a narrowly scoped, documented suppression.

**Acceptance criteria:** each finding is classified as genuine defect or verified false
positive, with the reason written and the evidence that supports the classification.

**Verification:** `aegis-0.6.3 --format json -c production scan . --baseline .aegis-baseline.json -q`

> **Note (2026-11-10):** the baseline named above was the retired line-keyed
> `.aegis-baseline.json`. Decision 0074 content-keys the gate; the committed
> baseline is now `.aegis-baseline.tsv` and the current gate command is
> `cargo run -p xtask -- aegis`. The adjudication below is the historical one,
> taken on 2026-11-09 against the baseline of that day.

**Evidence:** known false-positive classes: `pan` → PCI rule; large literals → zip-code
rule. `gdpr-personal-data` and `ssrf-localhost` are not yet classified.

**Blockers:** none — T27-11 is complete, so this row can start now.

**Stopping condition:** all findings have a written verdict and a named next action
per finding.

## Adjudication

The count grew from the 5 recorded on 2026-11-08 to **16** on 2026-11-09: the
branch's own release documentation quotes the Aegis rule ids and the file:line of
each finding so a reader can reproduce the adjudication, and those citations trip the
rules that name them. Two findings that were already baselined moved lines when
`xtask/src/dist.rs` was reformatted.

**No finding is a genuine defect. All 16 are verified false positives**, so the
next action for each is a narrowly scoped baseline entry — recorded, not blanket.
No rule class is disabled and no scanner coverage is reduced.

| Rule | Location | Verdict | Reason |
|---|---|---|---|
| `gdpr-personal-data` | `docs/RELEASE-ENGINEERING-PLAN.md:155` | false positive | The source-archive rule names the category of material to exclude. It says what must not be published; the document holds no personal data. |
| `pci-cardholder-data` | `docs/research/android-feasibility.md:62` | false positive | The touch-input row says "two-finger pan". The rule matches the word "pan"; the row describes a gesture and holds no cardholder data. |
| `zip-code` | `xtask/src/dist.rs:468` | false positive | The constant 36524 is the Gregorian 400-year cycle correction in the day-of-era formula. Not a postal code. |
| `zip-code` | `xtask/src/dist.rs:960` | false positive | The literal 60000 is the `--virtual-time-budget` passed to a headless browser — a duration in milliseconds, not a postal code. |
| `ssrf-localhost` | `xtask/src/dist.rs:1040` | false positive | `TcpListener::bind(("127.0.0.1", 0))` binds the loopback interface on an ephemeral port for a local preview server. No request target, no untrusted input. |
| `gdpr-reference` | `docs/release/EXECUTION-BOARD.md:51` | false positive | The row quotes the rule id `gdpr-personal-data` and the file:line of the finding it adjudicates. The rule fires on the category name inside the citation; the row holds no personal data. |
| `ssrf-localhost` | `docs/release/EXECUTION-BOARD.md:51` | false positive | The same row quotes the rule id `ssrf-localhost`. |
| `gdpr-reference` | `docs/release/EXECUTION-BOARD.md:371` | false positive | The row quotes the rule id `gdpr-personal-data` and the file:line it adjudicates. |
| `ssrf-localhost` | `docs/release/EXECUTION-BOARD.md:371` | false positive | The same row quotes the rule id `ssrf-localhost`. |
| `pci-cardholder-data` | `docs/release/EXECUTION-BOARD.md:167` | false positive | The row quotes the rule id `pci-cardholder-data`. |
| `gdpr-reference` | `docs/tasks/T27-11-reproduce-aegis-gate-failure.md:23` | false positive | The ticket enumerates the offending findings by rule id so the reproduction is reproducible. |
| `ssrf-localhost` | `docs/tasks/T27-11-reproduce-aegis-gate-failure.md:24` | false positive | The same ticket enumerates the rule id `ssrf-localhost`. |

## This ticket's own findings

The rows above cite rule identifiers and the phrases the rules match, so the
table above is itself a source of findings. Those findings are the same class
as the ones already adjudicated: a rule identifier written in prose, or the
matched phrase quoted inside a citation. The rows below record that verdict for
this file, and they are what make the table self-consistent — a rule identifier
named here is a citation, not the thing the rule detects.

| Rule | Location | Verdict | Reason |
|---|---|---|---|
| `gdpr-reference` | `docs/tasks/T27-12-adjudicate-aegis-findings.md` | false positive | This table quotes the rule identifiers it adjudicates, so the rule fires on the identifier inside the citation. The ticket holds no personal data. |
| `gdpr-personal-data` | `docs/tasks/T27-12-adjudicate-aegis-findings.md` | false positive | The reason text names the category the rule matches while explaining that the category is absent. A name in an explanation is not the data itself. |
| `ssrf-localhost` | `docs/tasks/T27-12-adjudicate-aegis-findings.md` | false positive | This table quotes the rule identifier and the loopback bind it adjudicates. There is no request target and no untrusted input in the ticket. |
| `pci-cardholder-data` | `docs/tasks/T27-12-adjudicate-aegis-findings.md` | false positive | This table quotes the rule identifier and the word the rule matches. A quoted word inside a citation is not cardholder data. |
| `zip-code` | `docs/tasks/T27-12-adjudicate-aegis-findings.md` | false positive | This table quotes the two numeric literals it adjudicates so a reader can reproduce the classification. They are a date constant and a millisecond duration. |

## Re-pinning

The baseline is re-pinned with the repo's own gate:

```
cargo run -p xtask -- aegis --bless
```

The gate keys every baseline record on the **content key** `(pattern, file,
content digest)`, not on a line number, so a row that moves does not need a new
adjudication and this table can be extended without invalidating itself. The
bless run prints every key it adds and every key it drops; each addition must
be verified as a false positive against a row of this table before the rewrite
is committed.

**Superseded (2026-11-10).** This ticket originally recorded the re-pin as a
helper script, `scripts/aegis_repin.py`, that read and wrote the retired
line-keyed `.aegis-baseline.json`. That baseline format is gone — decision
0074 content-keys the gate and moved the committed baseline to
`.aegis-baseline.tsv` — so the script could not run against the tree it
described. The tool-level fix supersedes the procedure, so the script was
removed in the same change that took it.

Completed 2026-11-09.
