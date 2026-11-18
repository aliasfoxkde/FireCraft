# T27-60 — Produce the release certification report for the approved scope

> **RP:** RP-08 · **Priority:** P0 · **Status:** SMOKE TESTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-19, T27-50

**Objective:** write the single document that states exactly what is being released,
what was verified, how, and what remains unverified.

**Inputs:** the approved scope decision (T27-09); every row's evidence; the artifact
matrix; the deployment reconciliation report (T27-50); the draft-release test run
(T27-19).

**Authority:** `docs/tasks/T27-cross-platform-release.md` — close T27 only when RP-08's
certification checklist is complete for every required target, or when the maintainer
explicitly approves a documented reduced support matrix; a green workflow with skipped
targets is not completion.

**Acceptance criteria:** the report names the tag, the source SHA, the workspace
version, the approved matrix, every artifact with its size, SHA-256, runner and
toolchain, the install/launch/smoke evidence, the signature/SBOM/provenance status,
the deployment reconciliation, the CI links, the limitations, the unsupported targets,
the residual risk and the independent verification results.

**Verification:** `test -f docs/release/CERTIFICATION.md`

**Evidence:** the report exists: [[CERTIFICATION.md](../release/CERTIFICATION.md)](../release/CERTIFICATION.md). It names the tag, the source SHA, the workspace version, the approved matrix, every artifact with its size, digest, runner and toolchain, the install/launch/smoke evidence, the signature/SBOM/provenance status, the deployment reconciliation, the CI gates, the limitations, the unsupported targets, the residual risk and the independent verification results. The independent checks are real: `RELEASE_SHA256.txt` downloaded from the draft matches 7 of 7 API digests; three downloaded assets re-hash 3 of 3; the source archive clean-builds on a second host; the native artifacts are byte-identical across two independent rehearsals. **The report certifies nothing beyond that**: nothing is signed and no provenance exists, so no artifact reaches rung 9 of the evidence ladder. **Done:** 2026-11-17: the certification report published in the repository.

**Blockers:** B-14 — no signing identity and no provenance generator exist, so no artifact can be `CERTIFIED`; and approved-scope item 8 (the verified production deployment) is blocked by B-11/B-12

**Stopping condition:** the report exists and every claim in it is backed by a cited
receipt. Unsupported targets are listed as unsupported, not omitted.
