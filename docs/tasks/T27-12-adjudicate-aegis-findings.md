# T27-12 — Adjudicate each of the 5 new Aegis findings

> **RP:** RP-00 · **Priority:** P0 · **Status:** READY
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-11

**Objective:** decide, for each of the 5 new findings, whether it is a genuine defect
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

**Evidence:** known false-positive classes: `pan` → PCI rule; large literals → zip-code
rule. `gdpr-personal-data` and `ssrf-localhost` are not yet classified.

**Blockers:** none — T27-11 is complete, so this row can start now.

**Stopping condition:** all 5 findings have a written verdict and a named next action
per finding.
