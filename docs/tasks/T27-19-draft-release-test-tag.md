# T27-19 — Exercise the whole draft-release flow on a disposable test tag

> **RP:** RP-07 · **Priority:** P0 · **Status:** NOT STARTED
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

**Evidence:** none — no end-to-end tag run has ever executed.

**Blockers:** T27-17, T27-18; disposable-tag authorization.

**Stopping condition:** a successful disposable-tag run is recorded with the tag name,
the run id, the downloaded asset list and the checksum verification output.
