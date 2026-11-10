# Cross-Platform Release Engineering campaign

This directory holds the **campaign mandate** for FireCraft's cross-platform
release engineering engagement (parent task T27), plus the repository
re-audit that preceded it.

| File | What it is |
|---|---|
| `Cross-Platform Releas.txt` | The execution mandate: mission, status vocabulary, evidence ladder, phase-by-phase rows (`P0.1.1`–`P10.x`), and the §20 authorization boundary. |
| `Planning.md` | The independent repository re-audit (November 8) that produced the task ledger and the implementation campaign. |

## Why these file names look wrong

`release-compaign` and `Cross-Platform Releas.txt` are typos in the original
mandate as issued. They are **kept verbatim on purpose**: the path is quoted
verbatim by documents in several open branches —

- `docs/release/PHASE-0-AUDIT.md` (the audit register, row F-13 and the
  "Campaign authority" line at the top)
- `docs/release/EXECUTION-BOARD.md` — a relative link to
  `../../planning/release-compaign/Cross-Platform%20Releas.txt`
- `docs/tasks/T27-07-execution-board.md` — cites the mandate as its input

Renaming the file would break those links in branches that are already
reviewed and would silently change the identity of the authority document
that every audit row cites by section number (`§3.2` status vocabulary,
`§3.4` evidence ladder). A link fix is cheap; a broken citation to a
numbered section of a renamed document is not. The names can be corrected
in one sweep later, once every referencing branch has merged.

The `.txt` extension is likewise the mandate's own: the file is plain text
with markdown structure, and it was issued that way.

## Status

Tracking this mandate in version control closes audit finding **F-13**
("the campaign mandate is a repository document — `…/Cross-Platform
Releas.txt` is **untracked** in the operator's checkout"). The finding
record and its disposition live in `docs/release/PHASE-0-AUDIT.md` §11.

## Reading order

1. `Cross-Platform Releas.txt` §1–§4 — mission, repository context, status
   vocabulary, evidence ladder, and the Phase 0 rows.
2. `Planning.md` — the audit that established the ledger the mandate
   executes.
3. `docs/release/PHASE-0-AUDIT.md` — the live audit register, including the
   defects (B-*) and findings (F-*) this campaign is closing.
4. `docs/RELEASING.md` and `docs/release/ARTIFACT-MATRIX.md` — how a release
   is actually produced and what it must contain.
