# T27-53 — Define canonical asset naming and generate every checksum and the release-wide list

> **RP:** RP-06 · **Priority:** P0 · **Status:** SMOKE TESTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-06

**Objective:** establish one canonical asset-naming scheme and one checksum list that
covers every published file, so the aggregate can be checked deterministically.

**Inputs:** `xtask/src/dist.rs` (`MANIFEST.sha256` generation);
`.github/workflows/release.yml` (the `RELEASE_SHA256.txt` generation step); the
artifact matrix's artifact-name column.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §4 — every published asset has a
verified SHA-256 and a release-manifest entry; the aggregate fails closed on
mislabeled or mismatched assets.

**Acceptance criteria:** the naming scheme is written down; every asset carries a
SHA-256; one release-wide checksum list covers every published file, including the
provenance and signature files when they exist; the list verifies with `sha256sum -c`.

**Verification:** `cd dist && sha256sum -c RELEASE_SHA256.txt`

**Evidence:** [`release.yml`](https://github.com/aliasfoxkde/FireCraft/blob/main/.github/workflows/release.yml) generates `RELEASE_SHA256.txt` for 7 files (no provenance/signature files). The rehearsal published it (718 B, sha256 `6ae766e7…`); downloaded from the draft, its 7 entries match the API digests of the 7 assets it covers — 7/7. **Naming is not yet canonical**: the source archive still ships as `FireCraft-<version>-source.tar.gz` while every other asset is lowercase; the rename to `firecraft-<version>-source.tar.gz` is implemented and lands with this row's next rebuild **Done:** 2026-11-11: checksum list verified against the published digests (rehearsal `v0.2.5-dryrun.3`).

**Blockers:** —

**Stopping condition:** the naming scheme is documented and the checksum list verifies
across the full published set.
