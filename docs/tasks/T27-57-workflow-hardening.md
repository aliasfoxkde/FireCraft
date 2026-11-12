# T27-57 — Harden the release workflows

> **RP:** RP-06 · **Priority:** P1 · **Status:** IN PROGRESS
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-18

**Objective:** harden the CI and release workflows so a compromised or drifted
dependency cannot silently alter a release.

**Inputs:** `.github/workflows/ci.yml`; `.github/workflows/release.yml`; the Aegis
version pin (already by SHA); the GitForge pipeline.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5 and §10; the campaign's requirement
that all artifacts bind to one tag SHA and that the gate fails closed.

**Acceptance criteria:** third-party actions are pinned to full commit SHAs;
permissions are least-privilege; build, sign and publish are separate jobs; all
artifacts bind to one tag SHA; the gate fails on missing outputs, a version mismatch,
a checksum mismatch, unexpected extra artifacts and a skipped required platform; a
safe test-release mode exists.

**Verification:**

```bash
python3 scripts/validate_workflow_pins.py .github/workflows
bash scripts/validate_workflow_pins-selftest.sh
bash tests/release-assets-selftest.sh
bash scripts/classify_release_mode-selftest.sh
bash scripts/validate_release_aggregate-selftest.sh
```

**Evidence (2026-11-11):** decision
[0099](../decisions/0099-workflow-actions-are-pinned.md) — all 19 action references
in `ci.yml`, `release.yml` and `deploy-site.yml` name a 40-hex commit SHA (the
version is kept in a trailing comment). `scripts/validate_workflow_pins.py` fails
the workflow on an unpinned reference and runs in the backup CI's `changes` job —
the job every push and pull request runs, and the job `fmt` and `gate` depend on.
`scripts/validate_workflow_pins-selftest.sh` demonstrates each failure mode with one
deliberately broken input (a mutable tag, a mutable branch, a `docker://` image, a
short SHA) and the two accepted inputs (a pinned SHA, a local action). The deploy
trigger no longer lists the workflow's own file, so a workflow edit is not a
production publication.

**Evidence (2026-11-12):** decision
[0100](../decisions/0100-release-mode-classification-and-a-tested-release-gate.md) —
the two remaining acceptance criteria are implemented, and each is demonstrated by a
self-test that runs in the same CI step:

* *The gate fails on a skipped required platform, and on unexpected extra
  artifacts.* The aggregate validation moved out of the workflow into
  `scripts/validate_release_aggregate.sh` (it was a hundred lines of inline YAML
  that ran once, on a tag, with nothing around it). It now also rejects an
  artifact the release does not publish, and requires every required platform
  (`x86_64-unknown-linux-gnu`, `wasm32-unknown-unknown`) to be represented in the
  release manifest. `scripts/validate_release_aggregate-selftest.sh` builds a
  complete, self-consistent artifact set and breaks it one thing at a time:
  a missing artifact, an empty artifact, an unexpected extra artifact, a checksum
  mismatch, a manifest version mismatch, a manifest digest mismatch, an asset the
  manifest does not list, an unknown manifest schema, a skipped required platform,
  and a platform the manifest does not represent — the last being the
  discriminating case, where every artifact is present and every digest agrees and
  only the platform check can fail.
* *A safe test-release mode exists.* `scripts/classify_release_mode.sh` maps
  `vX.Y.Z` to a production release and `vX.Y.Z-dryrun.N` to a rehearsal, and
  refuses every other shape (two-component version, no `v` prefix, unknown
  suffix, a `-dryrun` without a number, a non-numeric number, a trailing hyphen,
  a tag that names another version). The release workflow gained a
  `workflow_dispatch` trigger that may only rehearse. A rehearsal release is a
  prerelease titled `TEST <version>` whose notes carry a banner saying it must
  not be published, and its disposable tag is created at the exact commit that
  built the artifacts so the artifacts and the tag bind to one SHA.
  `scripts/classify_release_mode-selftest.sh` demonstrates each refused shape,
  and `tests/release-assets-selftest.sh` drives `packaging/release-assets.sh`
  against a stub `gh` that records its argument vector — asserting the
  production shape (a draft titled by the version, no `--prerelease`, tracked
  notes) and the rehearsal shape (`TEST ` title, `--prerelease`, the banner in
  the notes), and refusing a checksum dump as a production description, a missing
  notes file, an empty asset list, an unknown mode, an empty source tree, a
  stale destination and a duplicate basename.

**Already satisfied before this change:** least-privilege permissions (`contents: read`
at workflow level, `contents: write` only for the draft-release job); build and
publish are separate jobs; the artifacts bind to the tag SHA through the checkout;
the aggregate gate fails closed on a missing, empty, misnamed or digest-mismatched
artifact and on a version mismatch.

**Outstanding:** the rehearsal path has not been run end to end through
`workflow_dispatch` — it needs this workflow on `main` first, and the manual
rehearsal recorded in issue #99 predates the `TEST` title and the prerelease
flag, so it does not yet demonstrate this mode. The three self-tests and the pin
check are in the GitHub backup CI; mirroring them into the GitForge pipeline
waits for the `:2` pipeline shape (PR #123, PR #129).

**Blockers:** none. The remaining work is the end-to-end rehearsal run and the
GitForge mirror.

**Stopping condition:** every `uses:` is a full commit SHA, the permission set is
least-privilege, and the failure-mode checks are demonstrated with one deliberately
broken input each.
