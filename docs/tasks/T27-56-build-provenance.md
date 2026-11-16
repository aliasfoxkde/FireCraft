# T27-56 — Generate and independently verify build provenance

> **RP:** RP-06 · **Priority:** P1 · **Status:** BUILD VERIFIED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-53

**Objective:** produce attestations that bind each published artifact's digest to the
source SHA, the workflow and the runner, and prove they verify independently.

**Inputs:** the checksum list from T27-53; the GitHub Actions OIDC identity
(**absent** — see Blockers); `gh attestation`; the artifact digests.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §4 — provenance and signatures are
published and verified where credentials exist; `docs/tasks/T27-cross-platform-release.md`
lists provenance among the RP-06 deliverables.

**Acceptance criteria:** a provenance format is defined; each attestation binds an
artifact digest to the source SHA, the workflow and the runner; attestations verify
independently with `gh attestation verify`; the attestations are published.

**Verification:** `cargo run -p xtask -- dist --provenance && cargo run -p xtask -- dist --verify-provenance`
— the statement's own fail-closed verification. The independent
`gh attestation verify <digest> --repo aliasfoxkde/FireCraft` cannot run until the
attestation identity exists (see Blockers); the release workflow runs the pair on
every build, so a release that cannot state its provenance fails there.

**Evidence:** the format is defined and implemented: `xtask/src/provenance.rs` writes `dist/provenance.intoto.json`, an in-toto Statement v1 (https://in-toto.io/Statement/v1) whose predicate is SLSA provenance v1 — the subjects are every published asset's basename and SHA-256, and the predicate carries the build type (the release workflow), the source URI and commit digest, the workflow path/ref/repository, the runner environment, the builder id, the run URL and the build timestamp. `dist --verify-provenance` re-reads the statement and re-hashes the artifacts it names; it fails closed on a digest mismatch, an asset no subject covers, a subject that is not a published asset, and a source revision that is not the one being built. `release.yml` generates and verifies the statement after the checksum list and before the manifest, so the manifest lists it, and `scripts/validate_release_aggregate.sh` names it a required asset — the release fails without it. The decision cores take their inputs as data, so no test touches the git checkout, the environment or the clock. **The statement is unsigned**: it binds the digests to the build, but it carries no signature, so it can only be checked against the release that published it, not verified independently. **Done:** 2026-11-16: the statement generator and the fail-closed verifier are implemented and wired into the release workflow; the signature and the independent verification remain blocked on the attestation identity.

**Blockers:** the release build job has no attestation identity — GitHub's OIDC attestation needs `id-token: write` and `attestations: write`, which this repository has not been granted — so the statement cannot be signed and `gh attestation verify` cannot pass; signing (artifact-matrix row 17) is likewise unconfigured

**Stopping condition:** every published artifact has a verifiable attestation, or the
row names the exact missing credential.
