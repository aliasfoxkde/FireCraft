# T27-59 — Keep the execution board and the ticket ledger truthful as rows close

> **RP:** RP-00 · **Priority:** P1 · **Status:** READY
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

**Evidence:** 61 board rows and 61 tickets at creation (2026-11-08).

**Blockers:** `docs/TASKS.md` is owned by another agent in this session; its owner must
add the cross-reference from T27 to this board.

**Stopping condition:** the two counts match and every non-NOT STARTED row's status is
justified by a cited file, command output, digest or job id.
