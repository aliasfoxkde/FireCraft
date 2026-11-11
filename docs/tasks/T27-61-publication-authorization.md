# T27-61 — Obtain publication authorization

> **RP:** RP-08 · **Priority:** P0 · **Status:** BLOCKED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-60

**Objective:** obtain the maintainer's explicit written authorization to publish the
certified release, and only then flip the draft to published.

**Inputs:** the certification report (T27-60); the draft release; the maintainer's
decision.

**Authority:** campaign §20 — no production publish, tag, signing, deployment or merge
without explicit maintainer approval. A disposable test tag is permitted; this row is
not that.

**Acceptance criteria:** the maintainer authorizes publication in writing, naming the
tag; only then is the draft published; the published release's assets and digests are
re-verified against `RELEASE_SHA256.txt` after publication.

**Verification:** `gh release edit <tag> --draft=false`

**Evidence:** no authorization exists. The v0.2.5 release was published previously and
carries 5 assets while the tag workflow now expects 8.

**Blockers:** maintainer authorization; T27-60.

**Stopping condition:** a written authorization is recorded and the published release's
digests are re-verified against the pre-publication list.
