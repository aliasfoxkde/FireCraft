# 0100 — A release tag is classified before it can cut a release, the aggregate gate is a tested script, and a rehearsal is a prerelease titled `TEST`

Date: 2026-11-12 · Ticket: [T27-57](../tasks/T27-57-workflow-hardening.md) · Issue [#97](https://github.com/aliasfoxkde/FireCraft/issues/97) · Status: accepted

## Context

[T27-57](../tasks/T27-57-workflow-hardening.md) asks for two things the release
workflow did not have.

1. **A gate that fails on a skipped required platform, and a safe test-release
   mode.** The aggregate validation existed, but it was a hundred lines of
   inline YAML in `.github/workflows/release.yml` that ran once, on a tag, with
   nothing around it to tell it apart from a run that was merely quiet. It
   checked that the nine required artifacts were present and non-empty, that
   their digests agreed with `RELEASE_SHA256.txt`, and that the release
   manifest described them. It did **not** check that the artifact directory
   held *only* the published set — a stale artifact from an earlier version
   rode along unnoticed — and it had no notion of a platform at all, so a
   skipped build was indistinguishable from a build that was never asked for.

   There was no test-release mode either. The rehearsal that
   [issue #99](https://github.com/aliasfoxkde/FireCraft/issues/99) records was
   performed by hand on the disposable tag `v0.2.5-dryrun.3`: it produced a
   draft release whose title was `0.2.5-dryrun.3` and whose `isPrerelease`
   flag was **false**. Nothing in the tooling marked it as a rehearsal; the
   only thing separating it from a release was the word `dryrun` in the tag,
   which the workflow never looked at.

2. **A rule that lives in one place.** The workflow's artifact names, its
   checksum list and the manifest's version were all derived from the *tag*
   (`${TAG#v}`), even though the artifacts are named by the *workspace
   version* that `xtask dist` actually builds. The two agree on a tag push —
   `xtask`'s manifest generator refuses to build a tag that disagrees with the
   tree — but they are different facts, and a rehearsal cannot use the tag
   for both.

## Decision

- **A release tag is classified, and an unrecognised tag is an error.**
  `scripts/classify_release_mode.sh` maps a tag to exactly one mode:
  `v<MAJOR>.<MINOR>.<PATCH>` is a production release,
  `v<MAJOR>.<MINOR>.<PATCH>-dryrun.<n>` is a rehearsal, and **everything else
  is refused** — a two-component version, a tag without the `v` prefix, an
  unknown prerelease suffix (`-rc.1`), a `-dryrun` without a number, a
  non-numeric number, a trailing hyphen, and (when the tree's version is
  supplied) a tag that names another version. The workflow calls it before it
  creates anything, and the release job fails closed when the classifier
  refuses.
- **A manual run may only rehearse.** The release workflow gained a
  `workflow_dispatch` trigger that requires the operator to name a disposable
  rehearsal tag. A manual run whose tag classifies as a production release is
  refused: cutting a production release requires a tag push, which is the
  reviewable act.
- **A rehearsal is unmistakably a rehearsal.** `packaging/release-assets.sh`
  takes a mode. In `production` mode the release is an ordinary draft whose
  title is the bare version and whose description must be the tracked release
  notes under `docs/releases/` — a release whose description is a checksum
  dump is not a release. In `test` mode the title is `TEST <version>`, the
  release is a prerelease, and the notes carry a banner that says it must not
  be published. The script refuses an unknown mode, a missing notes file, an
  empty asset list and a missing staged asset before `gh` is ever called.
- **A rehearsal's artifacts bind to the rehearsal tag.** A rehearsal tag does
  not exist in the repository yet, and `gh release create` would create it at
  the default branch — a tag that does not point at the commit that built the
  artifacts. The workflow therefore creates the disposable tag at the exact
  commit the build ran on and pushes it, refusing to overwrite an existing
  tag. The tag carries the `-dryrun.<n>` marker, so it cannot be mistaken
  for a release tag.
- **The artifacts are named by the version, not by the tag.** The build job
  derives the version once from `cargo metadata` and every artifact name,
  checksum entry and gate check uses it. The tag names what is being
  released; the version names what was built.
- **The aggregate gate is a script with a self-test.**
  `scripts/validate_release_aggregate.sh` holds the gate, and
  `scripts/validate_release_aggregate-selftest.sh` exercises it against a
  synthetic artifact set with exactly one thing broken per case: a missing
  artifact, an empty artifact, an unexpected extra artifact, a checksum
  mismatch, a manifest version mismatch, a manifest digest mismatch, an
  asset the manifest does not list, an unknown manifest schema, a skipped
  required platform, and a platform the manifest does not represent. The
  platform check is discriminating on its own: the last case has every
  artifact present and every digest in agreement, and fails only because
  nothing carries the wasm platform's target triple.

## Consequences

- The release path has three tested seams instead of none: the classifier, the
  gate and the `gh` argument vector. All three self-tests run in the backup
  CI's `changes` job, so a pull request that breaks the release path fails
  before it can be merged.
- The gate is now stricter in two ways it was not: an artifact the release
  does not publish fails the gate, and a required platform with no artifact
  fails the gate. Both were silent before.
- The version, not the tag, names the artifacts. On a tag push the two agree
  and nothing changes; on a rehearsal the tag is a disposable name and the
  artifacts still describe the version they were built from.
- The rehearsal path has not yet been run end to end through
  `workflow_dispatch`: it needs this workflow on `main` first. The manual
  rehearsal recorded in issue #99 predates the `TEST` title and the
  prerelease flag, so it does not yet demonstrate this mode.
- The three self-tests are in the GitHub backup CI only. Mirroring them into
  the GitForge pipeline waits for the `:2` pipeline shape (PR #123, PR #129),
  the same reason the pin check is not there yet.
- A rehearsal pushes a disposable tag to the repository. It is a tag that no
  release ever names, it is refused if it already exists, and the campaign
  permits disposable test tags explicitly — but it is still a write to the
  remote, and the reusable rehearsal tags from the earlier manual run remain
  on the remote until a maintainer authorises their removal.
