# T27-56 — Generate and independently verify build provenance

> **RP:** RP-06 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-53

**Objective:** produce attestations that bind each published artifact's digest to the
source SHA, the workflow and the runner, and prove they verify independently.

**Inputs:** the checksum list from T27-53; the GitHub Actions OIDC identity;
`gh attestation`; the artifact digests.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §4 — provenance and signatures are
published and verified where credentials exist; `docs/tasks/T27-cross-platform-release.md`
lists provenance among the RP-06 deliverables.

**Acceptance criteria:** a provenance format is defined; each attestation binds an
artifact digest to the source SHA, the workflow and the runner; attestations verify
independently with `gh attestation verify`; the attestations are published.

**Verification:** `gh attestation verify <digest> --repo aliasfoxkde/FireCraft`

**Evidence:** not implemented — provenance is listed as "planned" in the artifact
matrix, and no signing or attestation identity is configured.

**Blockers:** T27-53; an attestation identity must exist.

**Stopping condition:** every published artifact has a verifiable attestation, or the
row names the exact missing credential.
