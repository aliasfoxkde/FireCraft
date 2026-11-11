# T27-54 — Generate and verify a machine-readable release manifest

> **RP:** RP-06 · **Priority:** P0 · **Status:** IMPLEMENTED (validation pending)
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

**Evidence:** implemented on `feat/release-manifest` — `xtask dist --manifest` writes
`dist/release-manifest.json` (schema `firecraft.release-manifest/1`) describing the eight
published assets with their canonical name, path relative to `dist/`, size, SHA-256,
format, target triple, source revision and verification status, plus the version, tag,
source revision, run URL, run id and build timestamp of the run. `xtask dist
--verify-manifest` re-reads it against the files on disk. The generator is fail-closed:
a missing artifact, an empty artifact, an unparseable checksum list, or a digest that
contradicts a recorded list fails the run. Unit tests cover the document's shape, its
determinism (the same inputs produce the same bytes, keys and entries sorted), the
verification status, and every rejection path. `release.yml` generates the manifest
after the other assets exist, uploads it as a ninth release asset, and its aggregate
validation checks the manifest against the downloaded assets. **Not yet validated against
a real release run** — the CI receipt is outstanding, and no published release carries
a manifest.

**Blockers:** none for the implementation. T27-53's canonical naming is still
`NOT STARTED`; the manifest adopts the names the release already enforces, and its
asset table moves with T27-53 when that lands ([decision 0089](../decisions/0089-release-manifest-shape.md)).

**Stopping condition:** the manifest exists, verifies, and is published alongside the
assets.
