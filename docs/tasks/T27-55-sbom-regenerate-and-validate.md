# T27-55 — Regenerate and validate the CycloneDX SBOM

> **RP:** RP-06 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-06

**Objective:** regenerate the SBOM from the locked dependency graph and prove it is
complete and correct, rather than merely well-formed JSON.

**Inputs:** `xtask/src/dist.rs` (`sbom()` — CycloneDX 1.5, source-appropriate
`bom-ref`s, resolved dependency edges, workspace root); the locked `Cargo.lock`.

**Authority:** `docs/tasks/T27-cross-platform-release.md` — SBOM and provenance are
published; `docs/RELEASE-ENGINEERING-PLAN.md` §4.

**Acceptance criteria:** the SBOM regenerates from the locked graph; it validates
against the CycloneDX 1.5 schema; it is compared component-by-component against
`Cargo.lock`; it is published and the download verifies.

**Verification:** `cargo run -p xtask -- dist --sbom && jq .metadata.component dist/sbom/firecraft-0.2.5-sbom.cdx.json`

**Evidence:** commit `00fdd61` marks the updated SBOM implementation as
verification-pending; the earlier artifact was only JSON-validated on the fleet.

**Blockers:** none beyond T27-06.

**Stopping condition:** the schema validation and the `Cargo.lock` component comparison
both pass, and the published file verifies.
