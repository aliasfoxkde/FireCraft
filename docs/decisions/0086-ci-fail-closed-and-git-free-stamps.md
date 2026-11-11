# 0086 — CI is fail-closed, and the stamp checks do not need git

Date: 2026-11-11 · Status: accepted

## Context

The `generated-docs` job in `.gitforge.yml` looked green on every
run. It was not green: two of its checks were not running, and a
third was comparing against a stale file.

**The masked failure.** A job step's `run:` block is one `sh -c`
invocation (`gitforge-runner/src/executor.rs`: `let cmd = vec!["sh",
"-c", &step.run];`). Without `set -e`, the block's exit status is the
**last** command's status. The `generated-docs` block ended in two
`cmp` calls, so every earlier command's status was discarded. The
step was:

```sh
cp docs/planning/generated/parity.md /tmp/parity.committed
cp docs/planning/generated/scorecard.md /tmp/scorecard.committed
cargo run --locked -p xtask -- parity
cargo run --locked -p xtask -- scorecard
cargo run --locked -p xtask -- changelog
cmp /tmp/parity.committed docs/planning/generated/parity.md
cmp /tmp/scorecard.committed docs/planning/generated/scorecard.md
```

**The dead gate.** `xtask changelog` regenerates `docs/CHANGELOG.md`
from `git log` (`xtask/src/changelog.rs`: `generate()` spawns
`git log --no-merges --format=%h%x09%s`). The CI image
`firecraft-ci-rust:1` ships **no git** — its Dockerfile says so
explicitly ("No git: no CI step talks to git"). So the verb failed on
every run with `spawn git log: No such file or directory (os error
2)`, and because of the masking, the job stayed green. The receipt of
a *SUCCEEDED* `generated-docs` job (commit `0b0de2c`) carries
`[xtask] changelog FAILED` in its stderr.

What died with it were the two release stamp checks. `changelog::run`
verified the license claim and the workspace version stamps **after**
the generation step, so neither ever ran anywhere in CI: the license
claim was checked only by a local `xtask ci`, and the version stamps
likewise. The gate engaged and produced no observable metric.

**The stale artifact.** Commit `f06e2b2` ("docs(release): renumber the
wasm budget decision to 0079") updated `xtask/src/status.rs`, the
decision file, `docs/GAPS.md`, `docs/PROGRESS.md` and
`xtask/src/dist.rs` — but not the committed
`docs/planning/generated/parity.md`, which the `PARITY` constant
generates byte-for-byte. The constant pointed at
`docs/decisions/0079-wasm-size-budget.md`; the committed file still
pointed at `0078-...`. The `cmp` failed on the renumber's own
commit.

**The lying report.** The `release` job's first step asked
`git describe --exact-match --tags HEAD` whether the run was a tag
push. With no git in the image, `git describe` failed, the `if` took
its else branch, and the step printed "not a tag push; building the
release path anyway" — on **every** run, including real tag pushes.

## Decision

- **The pipeline is fail-closed.** Every multi-command `run:` block in
  `.gitforge.yml` sets `set -e`. A step's status is the block's
  status; with `set -e` that status is the first failure, not the
  last command's.
- **The stamp checks are their own verb.** `xtask stamps` verifies the
  license claim and the workspace version stamps and needs no git:
  `verify_licenses` reads the license file, `verify_versions` reads
  `cargo metadata --no-deps`. `xtask changelog` keeps the generation
  and then calls `stamps`, so the local gate is unchanged. The
  `generated-docs` job calls `stamps`.
- **The tag report says what it can prove.** The `release` job's
  report no longer asks git for the ref. It states that the image
  ships no git, and points at where the ref *is* authoritative: the
  pipeline run (`pipeline_runs.ref_name`) and the release workflow's
  arguments on the host. The release path itself still builds on every
  push, which is the intent.
- **The committed generated doc is regenerated, not hand-edited.**
  `docs/planning/generated/parity.md` is written from the `PARITY`
  constant, exactly as `xtask parity` writes it.

## Consequences

- The stamp checks now run in CI for the first time. They are
  expected to pass on this commit: the license file still carries the
  dual claim and every workspace crate carries the workspace version.
- The `changelog` verb can still not run in CI. `docs/CHANGELOG.md`
  stays a release-time artifact (refreshed by `RELEASING.md`), which
  is what the job's comment said all along; only the claim that the
  verb "verifies licenses + versions ... on the runner" was false.
- `set -e` may surface failures that the masking hid. That is the
  point, and any such failure is a real defect to fix, not to
  re-mask.

## Debt

The CI image ships no git. That is a property of the image, not of
the pipeline: adding `git` to
`infrastructure/docker/ci-rust.Dockerfile` and re-shipping the image
(`docker save firecraft-ci-rust:1 | ssh <host> docker load`) would
restore in-container tag detection and let `xtask changelog` run on the
runner. **Owner:** the release campaign's maintainer. **Not done here**
because the image is built on a host whose root NVMe is constrained
and the only online runner is on it, so a rebuild-and-ship cycle
belongs to a deliberate release step, not to a bug-fix commit.
**What "fixed" looks like:** the Dockerfile installs git, the image is
re-shipped to every runner host, the `release` job's report asks git
for the ref again, and the `generated-docs` job can run
`xtask changelog` as well as `xtask stamps`.
