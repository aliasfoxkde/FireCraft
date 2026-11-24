# 0119 — The SBOM is validated against the schema it claims, and against the lockfile it names

Date: 2026-11-16 · Tickets: [T27-55](../tasks/T27-55-sbom-regenerate-and-validate.md) ·
Status: accepted

## Context

`xtask dist --sbom` writes a CycloneDX 1.5 document describing the
locked Cargo graph: the package inventory as `components`, the
resolved dependency edges as `dependencies`, and the workspace itself
as the metadata component. The row that owns it,
[T27-55](../tasks/T27-55-sbom-regenerate-and-validate.md), asks for
three things the generator did not do:

- the SBOM **validates against the CycloneDX 1.5 schema**;
- it is **compared component-by-component against `Cargo.lock`**;
- it is **published and the download verifies**.

Until now the only check was that the file was well-formed JSON. A
well-formed JSON document is not a CycloneDX document: every field
name, type and licence identifier in the file was the generator's
opinion rather than the schema's rule, and nothing compared the
inventory in the file with the inventory in the lockfile the file
claims to describe. The two defects that made this reachable:

1. **The licence field was wrong for most crates.** The generator wrote
   every Cargo licence as `{"license": {"id": "<string>"}}`. The
   schema's `license.id` is `$ref: "spdx.schema.json"` — an enum of
   SPDX *identifiers* — so `"MIT OR Apache-2.0"`, which is an SPDX
   *expression*, is not a valid identifier. The field was wrong for
   every crate whose licence is an expression, and no check said so.
2. **The inventory was never compared.** The SBOM is generated from
   `cargo metadata --locked` and the lockfile is what `--locked` means,
   so the two come from the same source — but nothing *checked* it, so
   a generator change that dropped or renamed a package would have
   shipped a plausible, schema-shaped, wrong document.

## Decision

- **The published schema is the authority, and it is vendored.** The
  CycloneDX 1.5 schema and its SPDX sibling are vendored under
  `xtask/schema/` (Apache-2.0, `docs/ATTRIBUTION.md` carries the rows)
  and compiled into the gate. The gate validates the document it just
  built against them, and fails before the file is written — a
  non-conforming artifact never reaches `dist/`.
- **The validator refuses a schema it does not fully understand.**
  `xtask/src/jsonschema.rs` implements the draft-07 keywords the
  CycloneDX schema uses, and `Validator::new` walks every schema
  position and *fails* on any keyword the validator does not implement.
  A validator that ignored an unknown keyword would report a document
  as valid when the schema says otherwise, which is the one failure
  mode a gate must not have.
- **The schema's own sibling is resolved by name.** The schema points
  at `spdx.schema.json` for the SPDX identifier list; the validator
  takes sibling schemas by file name (`Validator::with_siblings`) and
  resolves that `$ref` to them.
- **A licence is an identifier when the schema says it is one.** The
  generator reads the SPDX identifier list out of the vendored schema
  and writes `{"license": {"id": ...}}` only for a string the schema
  lists; every other licence — an expression such as
  `"MIT OR Apache-2.0"` — goes in the schema's `expression` branch.
- **The inventory is compared against the lockfile.** The gate reads
  `Cargo.lock`'s `[[package]]` sections and compares them with the
  SBOM's components as sets of (name, version). A package the
  lockfile pins that the SBOM does not list, or a component the SBOM
  lists that the lockfile does not pin, fails the gate and names both
  sides.

## Consequences

- The SBOM is now a claim the gate can falsify: it is a document that
  conforms to the published schema and whose inventory is the
  lockfile's, checked before it is written.
- The licence defect above is fixed by construction rather than by
  enumeration: the set of identifiers comes from the schema, so a new
  SPDX identifier needs no change here, and a licence string that is
  an expression can no longer be written as an identifier.
- The gate carries a small JSON Schema implementation. That is a cost,
  accepted deliberately: the alternative was a schema-validation crate
  in a release gate, and the keywords the CycloneDX schema uses are
  known and few. The refusal to run against an unimplemented keyword
  is what makes the small implementation safe.
- "Published and the download verifies" remains a property of the
  release rehearsal, not of this gate; the row's stopping condition
  names it.

See also decision
[0102](0102-coverage-floor-is-measured-with-a-pinned-toolchain.md) —
the same principle one layer up: a claim about a measurement is only
as good as the thing it is measured against.
