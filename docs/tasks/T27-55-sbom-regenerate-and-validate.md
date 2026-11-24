# T27-55 — Regenerate and validate the CycloneDX SBOM

> **RP:** RP-06 · **Priority:** P0 · **Status:** BUILD VERIFIED
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

**Evidence:** the CycloneDX 1.5 schema and the SPDX sibling it points at are vendored under `xtask/schema/`, and `xtask dist --sbom` validates the document it builds against them *before* writing it and compares its component inventory with `Cargo.lock`'s `[[package]]` sections, naming both sides of any difference (decision [0119](../decisions/0119-the-sbom-is-validated-against-the-schema-it-claims.md)). The validator refuses a schema that uses a keyword it does not implement, so a partial implementation cannot report a bad document as valid. The generator's licence field was wrong for every crate whose licence is an SPDX expression — `MIT OR Apache-2.0` is an expression, not an identifier — and the identifier list is now read from the schema itself. **Outstanding**: publishing the newly validated SBOM and verifying the download belongs to the next release rehearsal **Done:** 2026-11-16: schema validation and the `Cargo.lock` comparison implemented and verified.

**Blockers:** —

**Stopping condition:** the schema validation and the `Cargo.lock` component comparison
both pass, and the published file verifies.
