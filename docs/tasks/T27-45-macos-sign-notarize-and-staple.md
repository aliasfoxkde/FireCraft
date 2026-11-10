# T27-45 — Sign, notarize, staple and assess the macOS artifact

> **RP:** RP-03 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-43

**Objective:** apply Developer ID signing with the hardened runtime, notarize, staple,
and confirm Gatekeeper accepts the artifact.

**Inputs:** the `.app` and `.dmg` from T27-43; `codesign`; `xcrun notarytool`;
`stapler`; `spctl`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.3 and §4 — signatures are verified
where signing credentials are available, and ad-hoc signing is never recorded as
notarization.

**Acceptance criteria:** Developer ID signing succeeds with the hardened runtime and
the correct entitlements; the notarization is accepted; the ticket is stapled to the
artifact; `spctl -a -vvv` accepts it; an ad-hoc signature is never recorded as
notarization.

**Verification:** `codesign -dv --verbose=4 <app> && spctl -a -vvv <app>`

**Evidence:** no Developer ID credential is configured anywhere; notarization and
signing are listed as not configured in the artifact matrix.

**Blockers:** no Developer ID credential.

**Stopping condition:** `spctl` accepts the artifact and the notarization log is
attached, or the row stays open naming the missing credential.
