# T27-19 — Exercise the whole draft-release flow on a disposable test tag

> **RP:** RP-07 · **Priority:** P0 · **Status:** SMOKE TESTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-17, T27-18

**Objective:** prove the complete tag → build → aggregate → draft → upload → verify
path works, using a tag that cannot be mistaken for a production release.

**Inputs:** the tag-driven `.github/workflows/release.yml`; a disposable test tag;
`gh release download`.

**Authority:** campaign §20 — a disposable test tag is permitted; a production publish,
tag, signature, deployment or merge is not.

**Acceptance criteria:** a disposable (non-production) tag produces a draft release
whose assets download successfully and every downloaded file matches
`RELEASE_SHA256.txt`; the test tag and its release are removed afterwards.

**Verification:** `gh release download <test-tag> --dir artifacts && (cd artifacts && sha256sum -c RELEASE_SHA256.txt)`

**Evidence:** the disposable tag `v0.2.5-dryrun.3` → annotated tag `9009d404…` → commit `384687f…` ("Disposable rehearsal tag (not a release)") produced draft release 441136236 with 8 assets, all `state: uploaded`. The downloaded `RELEASE_SHA256.txt` matches the API digests of all 7 assets it covers. The source archive was downloaded and extracted too — and it does **not** clean-build (T27-52). Receipt: [RECEIPT-0.2.5-DRYRUN-3.md](../release/RECEIPT-0.2.5-DRYRUN-3.md). The row's own removal step — deleting the draft release and the three disposable `v0.2.5-dryrun.*` tags — has been carried out: on 2026-11-16 the releases API answered 404 for the draft release 441136236 and for the tag `v0.2.5-dryrun`, `gh release list` listed no dryrun release, and `git ls-remote --tags origin` listed no dryrun tag. The `test/disposable-release-dryrun` branch is gone from `origin` as well, so the rehearsal commits are no longer reachable from the remote, and issue #99's receipt — the run id, the asset list and its digests — is what remains of the rehearsal. Issue #99 recorded the removal as deferred pending maintainer authorization and carries no authorization note, so this row records the remote's verified state rather than an authorized step. **Done:** 2026-11-11: rehearsal `v0.2.5-dryrun.3`, run 42937560976.

**Blockers:** —

**Stopping condition:** a successful disposable-tag run is recorded with the tag name,
the run id, the downloaded asset list and the checksum verification output.
