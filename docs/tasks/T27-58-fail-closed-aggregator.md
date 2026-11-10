# T27-58 — Make the aggregator fail closed on every invalid artifact shape

> **RP:** RP-07 · **Priority:** P0 · **Status:** READY
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-17

**Objective:** prove the release aggregate refuses to publish a bad artifact set — it
must reject missing, mislabeled, empty, duplicate, wrong-version, wrong-commit and
checksum-mismatched artifacts, and it must fail when a required platform job was
skipped.

**Inputs:** the aggregate validation step in `.github/workflows/release.yml`; the
8-artifact expectation list; `RELEASE_SHA256.txt`.

**Authority:** `docs/tasks/T27-cross-platform-release.md` — "Release aggregation fails
closed on missing, mislabeled, mismatched, or corrupt assets"; `docs/RELEASE-ENGINEERING-PLAN.md` §10.

**Acceptance criteria:** each invalid shape is demonstrated to fail the aggregate, and
the valid shape is demonstrated to pass; a skipped required platform job is treated as
a failure, not as an omission.

**Verification:** run the `release.yml` aggregate step with one artifact deliberately
removed, renamed, emptied, duplicated and corrupted in turn.

**Evidence:** commit `8fc3694` added the fail-closed aggregate validation; it was
rehearsed locally with negative and positive cases. It has not been exercised through
a real tag run.

**Blockers:** T27-17 — the aggregate must reach `gh release create` at all before its
downstream behaviour is observable.

**Stopping condition:** every invalid shape has a recorded failing run and the valid
shape has a recorded passing run.
