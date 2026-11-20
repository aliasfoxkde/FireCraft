# T27-54 — Generate and verify a machine-readable release manifest

> **RP:** RP-06 · **Priority:** P0 · **Status:** SMOKE TESTED(validation pending)
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-53
> **Decision:** [0085](../decisions/0089-release-manifest-shape.md) · **Schema:** [RELEASE-MANIFEST.md](../release/RELEASE-MANIFEST.md)

**Objective:** publish one machine-readable manifest describing every artifact, so the
release is machine-verifiable rather than only human-checkable.

**Inputs:** the canonical naming scheme and checksum list from T27-53; the release tag
and source SHA; the CI run URL.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §4 and §10 — release notes must state
exact platform support and the manifest must be verifiable.

**Acceptance criteria:** `release-manifest.json` carries the version, tag, source SHA,
target triple, format, size, SHA-256, run URL and verification status for every
artifact; the generator rejects missing and corrupted assets; the manifest verifies
against the actual files.

**Verification:** `jq -r '.version,.artifacts[0].sha256' dist/release-manifest.json`

**Evidence:** `xtask dist --manifest` writes `dist/release-manifest.json` (schema `firecraft.release-manifest/1`) with one entry per published artifact — canonical name, path, size, SHA-256, format, target triple and verification status — plus the run's version, **tag**, source revision, **run URL** and timestamp; `dist --verify-manifest` re-checks it against the files. The rehearsal published the manifest (3,295 B, sha256 `54b8a231…`) and the workflow's aggregate gate validated it against the downloaded assets. **The manifest names its tag and its run**: `tag` `v0.2.5-dryrun.6`, `source_sha` `c396b18afba0a…`, `run_url` `https://github.com/aliasfoxkde/FireCraft/actions/runs/44188107435`, `run_id` `44188107435`, `built_at` `2026-11-17T22:31:44Z`, 8 artifact entries. Both gaps that made that impossible were closed first — the caller now states the tag and `xtask` fails closed when the stated tag does not name the workspace version, and the run URL is reconstructed from `GITHUB_SERVER_URL`, `GITHUB_REPOSITORY` and `GITHUB_RUN_ID` because GitHub Actions exports no `GITHUB_RUN_URL`. **Done:** 2026-11-17: a published manifest that names its tag and its run.

**Blockers:** —

**Stopping condition:** the manifest exists, verifies, and is published alongside the
assets.
