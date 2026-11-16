# T27-59 — Keep the execution board and the ticket ledger truthful as rows close

> **RP:** RP-00 · **Priority:** P1 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-07

**Objective:** keep the board and its tickets in one-to-one correspondence and keep
every status no stronger than the evidence behind it.

**Inputs:** `docs/release/EXECUTION-BOARD.md`; `docs/tasks/T27-*.md`; `docs/TASKS.md`.

**Authority:** the board's own rules in §10; the campaign's requirement that status
states are distinct and never inferred from a script or workflow definition.

**Acceptance criteria:** every closed row carries a completion date and a cited
evidence link; no row claims a status stronger than its evidence; the board's task-ID
set and the set of `docs/tasks/T27-*.md` files match one-to-one; no ticket file is
orphaned and no board row lacks a ticket.

**Verification:** `ls docs/tasks/T27-[0-9]*.md | wc -l` and
`grep -c '^| T27-' docs/release/EXECUTION-BOARD.md`

**Evidence:** the board's §6 table and the 61 ticket files now state the same status, evidence, blockers and completion date for every row; `scripts/verify_release_ledger.py` reports `rows=61 tickets=61 problems=0`. The reconciliation is a tracked tool, not a one-off script: `scripts/reconcile_release_ledger.py` writes the board's cells into their tickets (the board is the only source of truth), and the verifier is the gate that keeps them in step. The four rows whose verification cell carries an escaped pipe inside a code span — T27-08, T27-22, T27-41 and this one — keep the escape, so each row is ten cells and not eleven. **Done:** 2026-11-15: the board and its 61 tickets reconciled and the verifier landed with a CI step.

**Blockers:** —

**Stopping condition:** the two counts match and every non-NOT STARTED row's status is
justified by a cited file, command output, digest or job id.
