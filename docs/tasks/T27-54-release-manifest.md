# T27-54 — Generate and verify a machine-readable release manifest

> **RP:** RP-06 · **Priority:** P0 · **Status:** BUILD VERIFIED(validation pending)
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

**Evidence:** implemented on `feat/release-manifest` (PR #106, merged in the train as `658f015`) per [decision 0089](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/decisions/0089-release-manifest-shape.md): `xtask dist --manifest` writes `dist/release-manifest.json` (schema `firecraft.release-manifest/1`) for the eight published assets — canonical name, path, size, SHA-256, format, target triple and verification status — plus the run's version, tag, source revision, run URL and timestamp; `dist --verify-manifest` re-checks it against the files; `release.yml` publishes it as a ninth asset and validates it against the downloads. Unit tests cover the shape, the determinism and every rejection path. **No CI receipt and no published manifest yet** — the row closes when a rehearsal publishes one. **Done:** 2026-11-11: implementation merged (PR #106 → #107).

**Blockers:** T27-53's canonical naming is still `NOT STARTED`; the manifest adopts the names the release already enforces, and its asset table moves with T27-53 (decision 0089)

**Stopping condition:** the manifest exists, verifies, and is published alongside the
assets.
