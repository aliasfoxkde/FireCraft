# 0124 — The branch count is measured from the remotes, not from the clone's refs

Date: 2026-11-16 · Tickets: C8.04 (the climb plan's missing-docs campaign),
`docs/climb/README.md` S12 · Status: accepted

## Context

`xtask climb` measures every scoreboard number from the tree, and
`climb --check` fails when a ratcheted metric moves the wrong way. S12 —
"remote branches besides `main`" — was the one number that was not
measured at all: it was **transcribed**. The scoreboard's last three rows
carry 143 and 145, and the note on the first of them says "recorded
post-push" — the number a human wrote down after pushing, not a number
any command produced.

The measurement that existed in the plan — `git branch -r | grep -vc
'main$\|HEAD'` — is not a measurement of the repository. It counts the
**clone's remote-tracking refs**, which are whatever the clone's last
fetch happened to bring. Measured across the clones this repository
actually has:

| Clone | `git branch -r \| grep -vc 'main$\|HEAD'` |
|---|---|
| the CI runner's current checkout | 0 |
| older CI checkouts | 23 and 78 |
| a full clone | 154 |
| this worktree | 153 (79 `origin` refs + 77 `gitforge` refs) |

Three separate defects in one number:

1. **It measures the clone, not the repository.** The CI runner's
   checkout carried 0 remote branches — the metric read "no remote
   branches exist" in the environment where the ratchet is supposed to
   be meaningful, and 154 in a clone that had fetched everything.
2. **It double-counts.** This worktree has two remotes; a branch that
   exists on both appears once per remote. 79 + 77 = 156 refs for 77
   distinct branches.
3. **It is not reproducible.** The same commit, measured from two
   clones, gave 0 and 154. The number no environment can reproduce is
   worse than an honest gap: it makes the ratchet fire on the clone
   that fetched most recently and stay silent on the one that did not.

The measurement also could not run in CI at all. The runner's checkout
has `origin` set to a **host path**
(`/nas/Temp/repos/GitForge/repos/<uuid>`) that the job's container does
not mount, so `git ls-remote` there fails — which is why the number was
transcribed in the first place.

## Decision

- **S12 is measured from the remotes.** `xtask climb` runs
  `git ls-remote --heads <remote>` over every configured remote and
  unions the branch names it returns, dropping `main` and `HEAD`. The
  result is the number of **distinct remote branch heads besides
  `main`** — a property of the remotes, not of the clone that measures
  it. Measured on 2026-11-16: `gitforge` carries 76 heads, `origin`
  carries 10, and the union is **77** distinct branches.
- **The metric is `n/a` when no remote answers**, like the other
  measurements that need state outside the tree (S9–S11, S13). The CI
  image is offline by contract and the runner's checkout's remote is a
  host path its container does not mount, so S12 measures `n/a` in CI.
- **An unmeasured metric does not move, so it is not compared.** The
  ratchet applies the `n/a` → number rule: `S12_branches` is compared
  only when the measured value parses as a number *and* the baseline
  parses as a number. An `n/a` measurement is not a regression, and an
  `n/a` measurement is not a rise either. This is the same rule S8
  already used, and it is stated in the code next to the comparison.
- **The transcribed numbers are not carried forward.** The board's S12
  column keeps the values that were recorded (143, 145) until a row
  measured by this method replaces them; a recorded number from a
  different method is not comparable with a measured one, so the
  comparison simply skips until both sides are measured the same way.

## Consequences

- The metric means the same thing in every clone and in every
  environment: the number of branches the remotes carry. The
  double-count is gone (the union is by name), and the clone's fetch
  history no longer moves the number.
- The ratchet stays meaningful where it can be: a developer with
  reachable remotes sees a real branch count, and CI records `n/a`
  rather than a number that means "this clone never fetched".
- The board's S12 history is discontinuous across this change (145
  under the old reading, 77 under the new one). The discontinuity is
  recorded here rather than papered over, and the ratchet compares
  only like with like.
- The `docs/climb/README.md` S12 row now names `xtask climb` as the
  measurement and carries the method, so the next reader does not
  re-derive it from the old shell command.
