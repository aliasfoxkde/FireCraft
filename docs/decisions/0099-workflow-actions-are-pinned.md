# 0099 — Every workflow action is pinned to a commit, and a production deploy is triggered by content, not by its own workflow file

Date: 2026-11-11 · Ticket: [T27-57](../tasks/T27-57-workflow-hardening.md) · Issue [#97](https://github.com/aliasfoxkde/FireCraft/issues/97) · Status: accepted

## Context

The release campaign's rule is that a release must fail closed: the artifacts
published under a tag are the artifacts the repository's own code produced, and
nothing in the build path may change without a review of this repository. Two
holes in that rule sat in the GitHub Actions workflows.

1. **Every third-party action was named by a mutable reference.**
   `actions/checkout@v4`, `actions/cache@v4`, `actions/upload-artifact@v4`,
   `actions/download-artifact@v4`, `cloudflare/wrangler-action@v3` and
   `dtolnay/rust-toolchain@stable` all resolve, at job-run time, to whatever
   the upstream tag or branch points at that day. A tag can be moved; a branch
   moves on its own. The release workflow — the one that builds the desktop,
   CLI, web and source artifacts, writes the SBOM, and creates the draft
   release — therefore executed code that can change without a diff to review
   here. The Aegis scanner was already SHA-pinned; the actions were not, and
   the ticket that asks for the hardening (`T27-57`) names exactly this as its
   first acceptance criterion.
2. **Editing the deploy workflow published to production.** `deploy-site.yml`
   triggered on a push to `main` that touched `site/**`, `wrangler.toml` *or
   `.github/workflows/deploy-site.yml`*. The deploy step publishes `site/`; the
   workflow's own source is not part of the Pages deployment. Listing it in the
   trigger paths meant that every edit to the workflow file — including a
   one-line pin — was a production publication of the marketing site. The
   campaign keeps production publication gated for explicit maintainer
   approval, so a workflow edit must not silently become a deploy.

## Decision

- **Every `uses:` that names an action is `owner/repo@<40-hex-sha>`.** The
  version the SHA corresponds to is recorded in a trailing comment, so the
  pin is auditable without a network call.
- **A local action (`./path`) is allowed.** It is this repository's own code
  and it is reviewed in the diff.
- **The rule is enforced by a script, not by review.** `scripts/validate_workflow_pins.py`
  walks `.github/workflows/*.yml`, and fails with the file and line of every
  unpinned reference. It runs in the backup CI's `changes` job — the job every
  push and pull request executes, and the job the `fmt` and `gate` jobs depend
  on — so an unpinned action fails the workflow before any job builds or
  publishes anything. The script is stdlib-only, like the pipeline validator
  beside it.
- **The failure modes are demonstrated, not asserted.**
  `scripts/validate_workflow_pins-selftest.sh` feeds the validator one
  deliberately broken input per failure mode — a mutable tag, a mutable
  branch, a `docker://` image, a short SHA — plus the two inputs it must
  accept (a pinned SHA, a local action), and checks the exit code each time.
  It runs in the same CI step as the validator itself.
- **A production deploy is triggered by the content it publishes.** The
  `deploy-site.yml` trigger paths are `site/**` and `wrangler.toml`; the
  workflow's own file is no longer one of them. A deliberate redeploy is
  `workflow_dispatch`.

## Consequences

- The release workflow's code path is fixed at review time. A new action
  version is a deliberate commit: bump the SHA and the version comment.
- `dtolnay/rust-toolchain` is pinned to the SHA of its `stable` branch at the
  time of the change. The action's code is fixed; the Rust toolchain it
  installs still follows its own default, so the toolchain version can move
  under a pinned action. Pinning the action is the part that closes the
  supply-chain hole; pinning the toolchain version is a separate decision
  that would need the CI image rebuilt with it.
- The pin check runs in the GitHub backup CI. It is not yet in the GitForge
  pipeline (the primary CI), because `.gitforge.yml` is being restructured in
  flight (PR #123, PR #129); mirroring the step there is a one-line change
  once that settles. Both CIs must be green before a merge, so the rule is
  still enforced on every pull request.
- The self-test uses `mktemp -d` and removes its fixtures; it writes nothing
  outside the temporary directory.
- The deploy workflow no longer redeploys the site when only the workflow
  changes. If a future change to `deploy-site.yml` must reach production, it
  is a `workflow_dispatch` run — a deliberate act rather than a side effect
  of a merge.
