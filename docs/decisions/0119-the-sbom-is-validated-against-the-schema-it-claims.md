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
2. **The inventory was never compared.** The SBOM was generated from
   `cargo metadata --locked`, and the generator assumed that the
   lockfile and the metadata graph were the same set of packages. They
   are not, and the assumption was the defect: `cargo metadata`
   reports the *resolved* graph — the packages the current feature
   selection actually activates — while `Cargo.lock` pins the
   *complete* inventory, because Cargo resolves the lockfile
   independently of any one feature selection. Optional and
   platform-specific dependencies that no enabled feature activates
   are pinned but absent from the resolved graph. Measured on this
   workspace: 560 packages resolved, 562 pinned, the difference being
   `glob 0.3.4` and `libwebp-sys 0.14.4` — the webp-codec chain that
   nothing here enables. A generator that took the metadata graph as
   the inventory therefore shipped a document that omitted two crates
   the lockfile names, and nothing *checked* it, so a generator change
   that dropped or renamed a package would equally have shipped a
   plausible, schema-shaped, wrong document.

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
- **The inventory is the lockfile's, and it is a superset of the
  build.** The gate reads `Cargo.lock`'s `[[package]]` sections and
  takes that as the inventory: every locked package becomes a
  component, whether or not the current feature selection resolves it.
  A package the lockfile pins but the resolved graph omits is listed
  as a component with an **empty dependency edge** — it is pinned, not
  linked — and carries the archive digest the lockfile records for it.
  A build on another platform, or under another feature set, links it,
  so a document that omitted it would understate what can end up in a
  shipped binary.
- **The inventory is compared against the lockfile.** The gate then
  compares the SBOM's components with the lockfile as sets of (name,
  version). A package the lockfile pins that the SBOM does not list,
  or a component the SBOM lists that the lockfile does not pin, fails
  the gate and names both sides. The comparison is a real assertion
  only because the inventory is the lockfile's; it is a tautology
  against a resolved graph, which is where the two-package omission
  hid.

## Consequences

- The SBOM is now a claim the gate can falsify: it is a document that
  conforms to the published schema and whose inventory is the
  lockfile's, checked before it is written.
- The document names 562 components for a 560-package resolved graph.
  That is deliberate: two of them are pinned but not linked here, and
  the point of a supply-chain document is the set of things that can
  be built, not the set that this machine happened to activate.
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
