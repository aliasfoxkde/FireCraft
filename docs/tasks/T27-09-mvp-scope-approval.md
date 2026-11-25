# T27-09 — Obtain the maintainer's approval of the MVP v1 scope

> **RP:** RP-00 · **Priority:** P0 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §3](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-07

**Objective:** obtain a written maintainer decision naming the approved release scope.
The decision exists: [decision 0075](../decisions/0075-release-scope-mvp.md) is accepted,
so the approved scope is the MVP candidate scope in that record and a row may
name that matrix.

**Inputs:** the MVP candidate list in `docs/RELEASE-ENGINEERING-PLAN.md` §1.1 and in
`docs/release/EXECUTION-BOARD.md` §3; the campaign's MVP list.

**Authority:** campaign §5 P1.3 — "Record the maintainer's approved scope. Do not treat
a recommendation as approval." `docs/decisions/0075-release-scope-mvp.md` — the
release-scope authority, now accepted (Michael Kinney, 2026-11-17);
`docs/RELEASE-ENGINEERING-PLAN.md` §1.1 — the approved scope cut, which the
decision now points at.

**Acceptance criteria:** a decision record names the approved MVP scope, the date, and
the approver; the board's §3 scope split is updated to point at that record and is
marked approved rather than recommended.

**Verification:** `grep -n 'Approved by' docs/decisions/00XX-*.md`

**Evidence:** [decision 0075](../decisions/0075-release-scope-mvp.md) **Done:** 2026-11-17.

**Blockers:** —

**Stopping condition:** a decision record exists and the board cites it. The record
exists and the board cites it, so T27-10 is unblocked; T27-60 and T27-61 remain
blocked on their own evidence, not on the scope.
