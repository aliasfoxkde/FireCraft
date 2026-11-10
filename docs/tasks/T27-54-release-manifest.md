# T27-54 — Generate and verify a machine-readable release manifest

> **RP:** RP-06 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-53

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

**Evidence:** not implemented — no manifest exists today.

**Blockers:** T27-53.

**Stopping condition:** the manifest exists, verifies, and is published alongside the
assets.
