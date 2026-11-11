# T27-09 — Obtain the maintainer's approval of the MVP v1 scope

> **RP:** RP-00 · **Priority:** P0 · **Status:** BLOCKED
> **Board row:** [EXECUTION-BOARD.md §3](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-07

**Objective:** obtain a written maintainer decision naming the approved release scope.
Until this exists, no scope is approved and no row may claim an approved matrix.

**Inputs:** the MVP candidate list in `docs/RELEASE-ENGINEERING-PLAN.md` §1.1 and in
`docs/release/EXECUTION-BOARD.md` §3; the campaign's MVP list.

**Authority:** campaign §5 P1.3 — "Record the maintainer's approved scope. Do not treat
a recommendation as approval." `docs/RELEASE-ENGINEERING-PLAN.md` §1.1 — "a recommended
scope cut for maintainer approval, not an automatic change".

**Acceptance criteria:** a decision record names the approved MVP scope, the date, and
the approver; the board's §3 scope split is updated to point at that record and is
marked approved rather than recommended.

**Verification:** `grep -n 'Approved by' docs/decisions/00XX-*.md`

**Evidence:** `docs/RELEASE-ENGINEERING-PLAN.md` §1.1 recommends; **no approval
exists**. This board records the MVP as a recommendation only.

**Blockers:** maintainer decision required — nothing in the repository can substitute
for it.

**Stopping condition:** a decision record exists and the board cites it. Until then
T27-10, T27-60 and T27-61 remain blocked on this row.
