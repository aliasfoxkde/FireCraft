# PM-03 — Format capability registry and fidelity contract

> **Parent workstreams:** IO-01 and IO-03 (POST-MVP-ROADMAP §5.7).
> **Priority:** P0 — honest compatibility is a trust precondition.
> **Status:** `NOT STARTED` — gated on Gate 0.
> **Owner:** unassigned. **Created:** 2026-11-23.
> **Dependencies:** none; feeds EXPORT-01, DEV-01, and Help (UX-01).

## 1. User problem

The supported-format list lives in prose, dialogs, and CLI help. A user cannot tell
what stays editable, what is flattened, and what is rejected before committing to
an export, and no per-format fidelity matrix exists.

## 2. Current implementation and evidence

- Import: PNG/JPEG/GIF/BMP/TIFF/WebP/WBMP and `.fw.png` (`crates/io`, `crates/fwpng`).
- Export: PNG/JPEG/BMP/TIFF/WBMP/WebP/GIF, SVG, layered PSD, and the web workflows
  (`crates/webexport`, `Shell` export dialog).
- Fidelity decisions exist for PSD (decision 0006) and the native node tree
  (unknown-node preservation in `crates/doc/src/fwmap`).
- There is **no machine-readable registry**; `EXPORT_FORMATS` in the shell dialog
  and CLI match arms are the de facto list. No AVIF/JXL/PDF/OpenRaster/APNG support
  was found.
- **UNVERIFIED:** per-format round-trip results at this SHA; the existing corpus
  (`corpus/fixtures`, 32 entries) is native-format heavy.

## 3. Scope

In scope: one static registry (extension, media type, import/export, feature map,
editability class, color/bit-depth support, animation/page/layer behaviour,
metadata/profile handling, known limitations, fixture coverage, dependency gates);
exposure in the file dialogs, Help, CLI (`--formats`, JSON), and docs; a
compatibility report shown before lossy export; fixtures and round-trip tests per
format where meaningful.

**Non-goals:** adding new codecs, promising fidelity for unpublished semantics, or
claiming a format editable because a flattened preview exists.

## 4. Files and surfaces likely affected

New `crates/io/src/capability.rs` (or a small `format` module inside `io`; no new
workspace crate unless layering justifies it), `crates/app/src/shell/dialogs.rs`
(`EXPORT_FORMATS`), `crates/app-cli/src/main.rs`, `docs/FEATURES.md`,
`docs/CS6-PARITY.md` §F, generated docs, `xtask` if the registry is validated.

## 5. Decisions required

- ADR: *Format capability and fidelity contract* — the exact editability classes
  (`editable`, `transformed`, `flattened`, `reference-only`, `rejected`), how the
  report is worded, and which claims require fixtures.
- Whether the registry is a `const` table validated by a compile-time
  exhaustiveness rule or a runtime-loaded manifest (prefer `const`).

## 6. Implementation steps

1. Enumerate the real import/export paths from source, not from docs.
2. Define the registry type and one row per format; make the dialog and CLI read it.
3. Add a `format-report` CLI verb and a pre-export compatibility summary.
4. Add per-format fixtures and both-direction tests where meaningful.
5. Assert every registry row has at least one fixture or an explicit `no-fixture`
   reason, and that every CLI/dialog format is present in the registry.
6. Publish the matrix and link it from FEATURES/Help.

## 7. Acceptance criteria

- Every format offered anywhere in the UI/CLI appears exactly once in the registry.
- An export that flattens or drops content shows the losses before writing.
- An unsupported combination is refused with a typed reason, never silently
  approximated.
- The docs matrix is generated or validated from the registry.

## 8. Test plan

- Unit: registry completeness/exhaustiveness, report wording for each class.
- Integration: round trip per editable format; known-loss assertions for
  transformed/flattened formats.
- `cargo test -p io -p app -p app-cli`, `cargo run -p xtask -- corpus`,
  `cargo run -p xtask -- ci`.

## 9. Manual verification

Open each format offered by the dialog from a real file; confirm the report before
a lossy export; confirm the Help matrix matches the dialog.

## 10. Performance, security, portability

The registry is static data (no WASM cost beyond bytes). Import diagnostics must
not echo full paths or raw file content.

## 11. Risks and rollback

Risk: registry and real behaviour drift. Mitigation: both the dialog and CLI read
the registry, and a test fails when a format is added to a match arm without a row.
Rollback is a single module revert.

## 12. Stopping condition and closure evidence

Stop if a format's semantics are unknown and a row would require guessing; mark it
`rejected`/`reference-only` with the reason instead. Close with the registry, the
matrix, fixture receipts, and the pre-export report exercised on one lossy format.
