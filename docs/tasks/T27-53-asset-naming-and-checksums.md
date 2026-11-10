# T27-53 — Define canonical asset naming and generate every checksum and the release-wide list

> **RP:** RP-06 · **Priority:** P0 · **Status:** NOT STARTED
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

**Evidence:** `release.yml` generates `RELEASE_SHA256.txt` for 7 files — it does not
yet cover provenance or signature files, which do not exist.

**Blockers:** none beyond T27-06.

**Stopping condition:** the naming scheme is documented and the checksum list verifies
across the full published set.
