# 0123 — The release-workflow branch is superseded by `main` and must not be merged

Date: 2026-11-16 · Tickets: T27-59 (the board-ledger hygiene row),
issue #72 · Status: accepted

## Context

The T27 release campaign was developed on two long-lived branches:
`feat/release-workflow-repair` (`bc0aef1f`) and, on top of it,
`docs/release-execution-board-sync` (`803c28a5`, "sync release
execution board onto the repaired workflow"). PR #12 carried them and
was closed unmerged; the work then reached `main` by another route — the
release train (#108), the ledger wave closeouts (#148, #150, #151) and
the panel-dispatch wave (#152).

That leaves the branches in the awkward state the ledger is supposed to
prevent: they look like pending work, but their content is *older* than
what is already on `main`. Merging one would regress the release
documentation and the release tooling.

The measurement that settles it is a merge, not an opinion. Merging
`origin/main` into `docs/release-execution-board-sync` conflicts in 33
paths; comparing the two sides of every conflict by their unique line
sets gives the shape of the divergence:

| Path | lines only on the branch | lines only on `main` |
|---|---|---|
| `xtask/src/dist.rs` | 41 | 731 |
| `docs/release/ARTIFACT-MATRIX.md` | 21 | 102 |
| `docs/tasks/T27-57-workflow-hardening.md` | 6 | 83 |
| `packaging/release-assets.sh` | 10 | 61 |
| `.github/workflows/release.yml` | 65 | 132 |
| `docs/HANDOFF.md` | 157 (stale 2026-11-01 wording) | 152 |
| `docs/release/EXECUTION-BOARD.md` | 14 | 14 (same set, older rows) |

`main` is the superset on every path. The branch's four commits that
are not in `main`'s history were checked one by one against `main`:

| Branch commit | Present on `main`? |
|---|---|
| `bc0aef1f` — collapse the glibc-floor parse into a let-chain | yes: `xtask/src/dist.rs` uses the let-chain form |
| `6e8c3f3a` — the release title is the bare version | yes: `xtask/tests/release_workflow.rs` asserts it |
| `5c0fda16` — restore the Aegis adjudication the merge dropped | yes: `xtask/src/aegis.rs` carries `CLASS_NOTES` |
| `803c28a5` — sync the execution board | superseded: `main`'s board carries the rehearsal receipts and the newer statuses |

The branch's own headline commit is the one that is most misleading: it
syncs the board onto the repaired workflow, but it syncs the board **as
it stood before the rehearsal**, so its T27-17/18/19/51/52/53/57 rows
read `NOT STARTED`/`READY` where `main`'s read `BUILD VERIFIED`,
`IN PROGRESS` and `SMOKE TESTED` with the `v0.2.5-dryrun.3` receipts.
It is the very drift issue #72 was opened for.

The same conclusion holds for the release tooling the branch added:
`scripts/validate_release_aggregate.sh` and its self-test,
`scripts/classify_release_mode.sh` and its self-test,
`scripts/validate_workflow_pins.py` and its self-test,
`packaging/build-flatpak.sh`, `packaging/windows/firecraft.iss`,
`packaging/release-assets.sh` and the CI image's offline-wall fix are
all on `main`.

## Decision

- **The two branches are not merged.** Their content is on `main`; a
  merge would take the release documentation backwards, to the
  pre-rehearsal snapshot, and would take the release tooling with it.
- **The branches are closed, not deleted.** Deleting a remote branch is
  an outward-facing act with no way back if the closure is wrong, so
  the disposition is recorded here and on the board rather than
  executed by the agent. `git branch -d` on the local copies is enough
  to keep the tree from offering them again.
- **What the branch was for is still open, and it is open on `main`.**
  The campaign's *implementation* landed; what remains is
  *verification*: an end-to-end tag run of the repaired workflow with
  the run id attached as the RP-07 receipt, the draft-to-published
  transition (a maintainer act), the per-platform builds (no Windows
  and no macOS host on this fleet), and the signature. Those are the
  campaign's real blockers and none of them is a code-merge problem.
- **The board's ledger hygiene work continues on `main`,** where the
  board and the 61 tickets live. The count is 61 rows and 61 tickets,
  the id sets match one to one with no orphan on either side, and the
  one closed row that lacked a completion date (T27-47, `DEFERRED` by
  decision 0121) now carries it.

## Consequences

- A future agent that finds `docs/release-execution-board-sync` and
  tries to merge it finds this record instead of a 33-path conflict
  and a documentation regression.
- The campaign's open work is stated in terms of what it needs — a
  Windows host, a macOS host, a maintainer publication, an OIDC
  signing identity — rather than in terms of a branch to merge.
- `docs/TASKS.md` row 27 now links the board, so the ledger named by
  the issue is reachable from the board the issue is about.
