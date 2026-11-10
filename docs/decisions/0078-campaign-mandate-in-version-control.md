# 0078 — The campaign mandate is a repository document

Date: 2026-11-10 · Ticket: [T27](../tasks/T27-cross-platform-release.md) · Status: accepted

## Context

The cross-platform release engineering campaign was issued as two files
outside version control, in the operator's checkout only:

- `docs/planning/release-compaign/Cross-Platform Releas.txt` — the
  execution mandate (mission, §3.2 status vocabulary, §3.4 evidence
  ladder, the phase rows, the §20 authorization boundary)
- `docs/planning/release-compaign/Planning.md` — the repository
  re-audit that produced the task ledger

`git status` reported the directory as untracked, so neither file was
in any commit, on any branch, and neither was reachable by a clone.

This is audit finding **F-13** (`docs/release/PHASE-0-AUDIT.md` §8),
severity medium: the authority document for the entire campaign is
not under version control. Every row in the campaign's ledger is
adjudicated against that document — the status vocabulary, the
evidence ladder, and the per-row stopping conditions are its
sections — so an untracked mandate means the criteria are not
reviewable, not diffable, and not recoverable. The damage is already
visible: `docs/release/EXECUTION-BOARD.md` (PR #12) carries a
relative link to the mandate that resolves to nothing on `main`,
because the target was never committed.

## Decision

The mandate is tracked in the repository, at the path it was issued
at, under `docs/planning/release-compaign/`, together with the audit
that preceded it and a `README.md` that explains what the two files
are.

The **file names are kept verbatim**, including the two typos
(`release-compaign` for "campaign", and `Releas` for "Release") and
the `.txt` extension on a markdown document. This is deliberate.

## Consequences

- The campaign's criteria are now reviewable in a diff and present in
  every clone. A finding that cites "§3.2" or "§3.4" now has a
  versioned referent, and the board's relative link resolves.
- The typos are preserved because the path is **cited**, not merely
  referenced:

  - `docs/release/PHASE-0-AUDIT.md` names the file in its "Campaign
    authority" line and in the F-13 row itself
  - `docs/release/EXECUTION-BOARD.md` links to
    `../../planning/release-compaign/Cross-Platform%20Releas.txt`
  - `docs/tasks/T27-07-execution-board.md` names it as its input

  Renaming the file would break links in branches that are already
  reviewed, and would silently re-point every numbered citation
  (`§3.2`, `§3.4`, the phase rows) at a differently-named document.
  A link fix is cheap; a broken citation is not. The names can be
  corrected in one sweep later, when every referencing branch has
  merged and the citations can be re-pointed together.
- The `README.md` in the directory records the naming rationale, so
  the next reader does not "fix" the typo and break the links.
- The Aegis scan is clean on the new files: the mandate and the
  audit produce no findings, and the audit file's existing
  `zip-code` finding keeps its baselined content digest and count, so
  the baseline needed no re-bless.

See also the Phase 0 audit (`docs/release/PHASE-0-AUDIT.md`), whose
§8 register and §11 reconciliation carry the finding and its
disposition.
