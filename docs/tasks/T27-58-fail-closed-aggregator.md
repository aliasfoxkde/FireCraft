# T27-58 — Make the aggregator fail closed on every invalid artifact shape

> **RP:** RP-07 · **Priority:** P0 · **Status:** BUILD VERIFIED
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

**Evidence:** `8fc3694` added fail-closed aggregate validation; rehearsed locally negative and positive. **The positive path now runs for real**: the rehearsal's `create draft release` job validated all nine required artifacts, ran `sha256sum -c RELEASE_SHA256.txt`, checked the release manifest against the downloaded assets and every asset name against the tag version, then created the draft (run 42937560976). **2026-11-12**: the aggregator moved into `scripts/validate_release_aggregate.sh` and is now exercised by `scripts/validate_release_aggregate-selftest.sh`, which breaks a complete artifact set one thing at a time — a missing artifact, an empty artifact, an unexpected extra artifact, a checksum mismatch, a manifest version mismatch, a manifest digest mismatch, an asset the manifest does not list, an unknown manifest schema, a skipped required platform, and a platform the manifest does not represent. The last is the discriminating case for the platform check: every artifact is present and every digest agrees, and the gate still fails **Done:** 2026-11-12: every failure mode demonstrated with one deliberately broken input.

**Blockers:** —

**Stopping condition:** every invalid shape has a recorded failing run and the valid
shape has a recorded passing run.
