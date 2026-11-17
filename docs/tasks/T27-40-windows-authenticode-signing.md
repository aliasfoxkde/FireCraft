# T27-40 — Configure and verify Authenticode signing

> **RP:** RP-02 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-36

**Objective:** set up Windows code signing so the executables and installer carry a
verifiable, timestamped Authenticode signature.

**Inputs:** the MSVC binaries and installer; `signtool`; the signing infrastructure
(selected provider and secret *names* only).

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.2 and §4 — signing credentials
are never committed, and only the names of secrets are documented.

**Acceptance criteria:** the signing infrastructure is identified; the secret **names**
are documented (never their values); the executables and installer are signed and
timestamped; `signtool verify /pa` succeeds on each; an unsigned or untrusted-publisher
result is never recorded as signed.

**Verification:** `signtool verify /pa <exe>`

**Evidence:** No signing credential configured anywhere

**Blockers:** No Authenticode certificate

**Stopping condition:** the signature verifies on every shipped Windows artifact, or
the row stays open with the missing credential named as the blocker. An ad-hoc or
unsigned build is never recorded as signed.
