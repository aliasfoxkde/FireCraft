# PM-06 — Accessibility inspector and accessible authoring baseline

> **Parent workstreams:** A11Y-01 and A11Y-02 (POST-MVP-ROADMAP §5.12).
> **Priority:** P1/P2 — an honest, tested baseline before any conformance claim.
> **Status:** `NOT STARTED` — gated on Gate 0.
> **Owner:** unassigned. **Created:** 2026-11-23.
> **Dependencies:** PM-05 metadata improves focus/command coverage; PM-03 formats
> feed alt-text/export guidance.

## 1. User problem

FireCraft can check neither the accessibility of the artwork it produces nor its
own UI beyond a keyboard walkthrough. The web canvas has no reachable accessibility
tree (decision 0067), and there is no object navigator, contrast report, or
suppressible-warning workflow.

## 2. Current implementation and evidence

- `crates/app/Cargo.toml` enables egui's `accesskit` feature; decision 0067 records
  that the web canvas tree is unreachable.
- F1 lists 167 bindings in 40 sections; tool one-key selects and CS6 keymap are
  gated by tests (CS6-PARITY §H).
- `rg -i 'a11y|accessibility' crates` → 0 files; no contrast/touch-target/focus
  inspector exists.
- Desktop screen-reader behaviour is **UNVERIFIED** (no receipt).

## 3. Scope

In scope: an accessibility inspector for documents with contrast (documented
luminance basis), text-size/legibility, touch targets, focus order/visible focus,
keyboard reachability and traps, accessible names, alt-text vs decorative, semantic
grouping, reduced-motion/color-only warnings, language/direction metadata, and an
exportable severity report with suppression reasons; an object navigator/list as a
semantic alternative to canvas interaction; a criterion-by-criterion conformance
report.

**Non-goals:** claiming WCAG conformance from a contrast check; hosted audit
services; auto-fixing content without user action.

## 4. Files and surfaces likely affected

New `crates/engine/src/a11y.rs` or `crates/doc` read model; new panel under
`crates/app/src/shell/`; `crates/compose` contrast sampling; `crates/io` export
reports; decision 0067 update; `docs/FEATURES.md` accessibility section.
Surfaces: desktop inspector, CLI report verb, web (limited, documented).

## 5. Decisions required

- ADR: *Accessibility model and conformance reporting* — contrast basis (relative
  luminance vs profile-aware), report schema, suppression model, and the explicit
  web limitation.
- How the object navigator maps document order/hierarchy to focus order.

## 6. Implementation steps

1. Define the finding type (severity, object, criterion, reason, remediation).
2. Implement contrast/text-size/touch-target checks over the document model.
3. Add alt-text/decorative and language/direction metadata verbs + inspector fields.
4. Add the object navigator with keyboard focus that mirrors the layer selection.
5. Build the report panel and CLI `report --a11y` JSON.
6. Add criterion-by-criterion conformance table and suppression with reasons.
7. Re-audit decision 0067; keep the web limitation until an end-to-end browser AT
   test passes.

## 7. Acceptance criteria

- Automated tests cover labels, keyboard navigation, contrast math, focus, and
  report generation.
- A finding names the object and a remediation; suppression requires a reason.
- The object navigator can select and reveal any object without pointer use.
- The report distinguishes "checked" from "not testable here" per criterion.
- No blanket WCAG claim is rendered anywhere in the UI or docs.

## 8. Test plan

- Unit: contrast fixtures (known WCAG pairs), touch-target thresholds, focus order.
- Golden: a fixture document → expected findings JSON.
- Manual AT receipts: OS, screen reader, version, host, workflows (desktop only,
  recorded when available).
- `cargo test -p engine -p app -p app-cli`, `cargo run -p xtask -- ci`.

## 9. Manual verification

Keyboard-only desktop pass over menus, panels, inspector, palette, and navigator;
browser pass with the recorded limitation.

## 10. Performance, security, portability

Contrast sampling is bounded per object; reports never include raw document content
by default. WASM: all checks are pure Rust; no threads.

## 11. Risks and rollback

Risk: false authority from partial checks. Mitigation: per-criterion reporting and
explicit "not testable" states. Rollback is additive (new panel/verb).

## 12. Stopping condition and closure evidence

Stop if the platform AT path cannot be exercised in this environment; deliver the
inspector/navigator with the limitation recorded. Close with the report schema, the
golden document findings, and any AT receipt naming host/version/date.
