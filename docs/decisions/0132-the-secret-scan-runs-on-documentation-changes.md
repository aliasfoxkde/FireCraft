# 0132 — The Aegis secret scan runs on documentation-only changes, in both CI systems

Date: 2026-11-17 · Tickets: T27-24 (the adjudication that found it), T27-62 (the
fix) · Status: accepted

## Context

The Aegis gate is the first leg of `xtask ci` (`xtask/src/main.rs`, the `aegis`
step). It scans the whole repository — including `docs/` — and compares the
findings against the committed, content-keyed baseline `.aegis-baseline.tsv`
(decision 0074). A finding the baseline does not cover fails the gate.

Both CI systems ran that leg **only when the change touched code**:

| System | Where the leg ran | Why a documentation-only change skipped it |
|---|---|---|
| GitHub | the `gate` job, `cargo run --package xtask -- ci-no-fmt` | the job's condition is `code_changed == 'true'`; the `docs-only-check` job that runs instead validates whitespace and merge markers only |
| GitForge | no Aegis job exists at all; `xtask ci`'s own Aegis leg is not part of any pipeline job | every Cargo job in `.gitforge.yml` carries a `skip_if` on `CI_CHANGE_INCLUDE_CODE` |

So "the baseline is current" was a claim nobody checked. The consequence is not
hypothetical. PR #170 (`f1a45bec`) added `docs/tasks/T27-41-windows-arm64-assessment.md`,
whose text quotes four Windows build numbers. The GitHub `gate` job was skipped for
that pull request because the change was documentation-only, so those findings
entered `main` unblessed. The next code change's gate failed for content it never
touched: on `main` `c26adffc` the `gate` job's Aegis leg reported **5 findings the
baseline does not cover** — the Windows OS build `19041` (twice), `22621`, `26100`,
and the Visual Studio build `37710`. The run's log is the evidence.

The failure mode is worth naming precisely: the gate blames the wrong change. The
author of the code change sees a red pipeline whose message points at four numbers
in a Windows assessment they have never read, and the author of the documentation
change has already merged and forgotten it.

The second observation is the same defect seen from the other CI system. Pull
request #154 (`8067b53b`, the icon-resource survey) touches no file under
`docs/tasks/` and has nothing to do with Windows. Its GitForge pipeline run
(`fe161636`) **succeeded** — the GitForge pipeline has no Aegis job at all, so
"the baseline covers the tree" was never asked — while the GitHub `gate` job on
the same SHA **failed** with `aegis found 5 finding(s) the baseline does not
cover`, every one of them in `docs/tasks/T27-41-windows-arm64-assessment.md`,
a file PR #170 had merged and PR #154 had never read. The two systems did not
disagree about the code; one of them had no opinion about the secret scan at all.

## Decision

1. **The Aegis scan runs on every change, in both systems.** The GitHub
   `docs-only-check` job installs the pinned scanner and runs
   `cargo run --package xtask -- aegis`; the GitForge pipeline gains an `aegis`
   job with **no `skip_if`**, placed in the chain between `release-ledger` and
   `fmt-layers` so it runs on every push and every tag.
2. **One script installs the scanner, in both systems.** `scripts/install-aegis.sh`
   holds the release URL, its SHA-256 and the identity check (`aegis --help` must
   list the `scan` subcommand — an unrelated tool also ships under the bare name
   `aegis`). The GitHub `gate` job's inline installer is replaced by a call to it.
   The pin lives in one place, so the two systems cannot drift.
3. **The scan is cheap because it is not the gate that is expensive.** It reads
   the tree and compares digests; it does not compile the workspace. The GitForge
   job compiles `xtask` (seconds of work against a warm per-job cache) and then
   scans.

## Why not the procedure

The obvious alternative is to keep the CI wiring as it is and require a re-bless
after every documentation change — "run `cargo run -p xtask -- aegis --bless`
when you touch `docs/`". That procedure compensates for a defect in the tool: it
asks the author of a change to remember a step that the machine can perform, and
it cannot catch the change nobody remembers. It also grows: every future
documentation change re-learns the same lesson.

The discriminator is the one this repository already uses: *what breaks if the
procedure is skipped?* If the answer is "the gate misbehaves" — a red pipeline
pointing at unrelated content, exactly as on `c26adffc` — then the procedure is a
workaround for a bug, and the bug is what gets fixed.

Adjudicating a new finding and re-blessing the baseline remains a **human** step
and always was: the baseline admits a literal only after someone has checked that
it is a false positive. This decision does not automate that judgement; it makes
sure the judgement is *asked for* at the moment the content lands, instead of
several commits later on someone else's pipeline.

## Consequences

- A documentation-only pull request now installs Rust and the scanner and runs the
  scan. That is a real cost on the backup CI system, and it is the same cost the
  code path already pays.
- The GitForge `aegis` job downloads the scanner on every run. The CI image
  deliberately ships no `curl` and no `wget`; `scripts/install-aegis.sh` brings
  its own fetch (curl, wget, node or python3, in that order) and verifies the
  tarball's SHA-256 before installing anything. The image is not rebuilt for this,
  so no fleet-wide image re-ship is needed.
- The baseline stays a committed artefact that a human blesses. What changes is
  that its currency is checked on every push instead of being assumed.

## References

- `xtask/src/aegis.rs` — the gate and the content-keyed baseline.
- `.gitforge.yml` — the `aegis` job, and the `skip_if` conditions it does not
  carry.
- `.github/workflows/ci.yml` — the `docs-only-check` job's Aegis step.
- `scripts/install-aegis.sh` — the single installer.
- [decision 0074](0074-aegis-content-keyed-baseline.md) — the content-keyed
  baseline this gate reads.
