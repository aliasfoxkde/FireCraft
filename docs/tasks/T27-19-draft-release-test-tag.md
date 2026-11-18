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

**Evidence:** the disposable tag `v0.2.5-dryrun.6` → commit `c396b18afba0a…` produced draft release 449279892 with 9 assets, all `state: uploaded`; the draft is `draft: true`, `prerelease: true`, titled `TEST 0.2.5-dryrun.6`, with `published_at: null`. The downloaded `RELEASE_SHA256.txt` matches the API digests of all 7 assets it covers, and the source archive, the checksum list and the manifest were each re-hashed locally — 3 of 3 match. The tag is **lightweight** (`GET /git/ref/tags/…` reports an object of type `commit` and `GET /git/tags/…` answers 404), so it carries no message of its own; the rehearsal's do-not-publish intent lives in the draft state, the `TEST` title and the notes banner. Receipt: [RECEIPT-0.2.5-DRYRUN-6.md](../release/RECEIPT-0.2.5-DRYRUN-6.md). The row's own removal step was carried out on 2026-11-16 for the rehearsal-3 artifacts (the releases API answered 404 for draft 441136236 and for the tag `v0.2.5-dryrun`); **the three later rehearsals are still on the remote** — drafts 449261624 (`v0.2.5-dryrun.5`) and 449279892 (`v0.2.5-dryrun.6`), the rehearsal-4 draft, and their three disposable tags. **Done:** 2026-11-17: rehearsal `v0.2.5-dryrun.6`, run 44188107435, draft 449279892.

**Blockers:** the removal of the rehearsal-4, -5 and -6 drafts and tags is outstanding and stays maintainer-gated, as the rehearsal-3 removal was

**Stopping condition:** a successful disposable-tag run is recorded with the tag name,
the run id, the downloaded asset list and the checksum verification output.
