# T27-08 — Publish one atomic child ticket per board row

> **RP:** RP-00 · **Priority:** P0 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-07

**Objective:** create one self-contained ticket per board row under `docs/tasks/`, so
that an agent picking up any row can execute it without reconstructing the campaign
conversation.

**Inputs:** the row table in `docs/release/EXECUTION-BOARD.md` §6; the ticket shape
used by `docs/tasks/T18-release.md`, `docs/tasks/T23-release-assets.md` and
`docs/tasks/T27-cross-platform-release.md`.

**Authority:** the repo ticket convention — bold-label paragraphs `**Objective:**`,
`**Inputs:**`, `**Authority:**`, `**Stopping condition:**`, `**Verification:**`.

**Acceptance criteria:** every board row has exactly one ticket at
`docs/tasks/T27-NN-<slug>.md`, each carrying the RP, priority, status, dependencies,
acceptance criteria, verification command, evidence, blockers and stopping condition —
and each readable standalone.

**Verification:** `ls docs/tasks/T27-*.md | wc -l`

**Evidence:** 61 tickets, one per board row (this change).

**Blockers:** none.

**Stopping condition:** the ticket count equals the board row count and no ticket
describes more than one logical change or more than one acceptance result.
Completed 2026-11-08.
