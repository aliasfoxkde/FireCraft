# PM-04 — Searchable in-app Help Center

> **Parent workstream:** UX-01 (POST-MVP-ROADMAP §5.2).
> **Priority:** P1 — discoverability is the cheapest multiplier on existing features.
> **Status:** `NOT MISSING?` → `MISSING` (see §2). Gated on Gate 0 only for shared
> shell files.
> **Owner:** unassigned. **Created:** 2026-11-23.
> **Dependencies:** PM-05 (command metadata registry) is a soft dependency; Help can
> start with static topics and add command deep links when PM-05 lands.

## 1. User problem

Help is six menu rows: *Online Help* opens the hosted wiki, *Keyboard Shortcuts*
shows a read-only table. There is no offline help, no search by task language, no
contextual help from a panel or error, and no single source that keeps command
names, labels, and shortcuts from drifting.

## 2. Current implementation and evidence

- `Shell` Help menu and `S/panels.rs::help_panel` (read-only bindings table).
- Help rows verified in CS6-PARITY §A (6 rows, implemented).
- `rg -i 'help center' crates` → 0 files. No help-topic corpus exists.
- Existing user docs: `docs/USAGE.md`, `docs/FEATURES.md`, `docs/wiki/`, CS6-PARITY.

## 3. Scope

In scope: bundled offline topics covering getting started, workspace, tools, menus,
shortcuts, document model, vector/bitmap, typography/resources, formats and
limitations, export/batch, prototyping/animation, CLI/scripting/MCP, troubleshooting,
accessibility, release notes, and community; F1/`Help` notes; a Help panel with
search (synonyms + task language), topic navigation, and deep links; contextual
help entry points from panels/dialogs/errors; version-aware notes; offline marker.

**Non-goals:** a hosted documentation site, localization (separate stream), or
auto-generating prose from source.

## 4. Files and surfaces likely affected

`crates/app/src/shell/panels.rs` (or new `shell/help.rs`), `crates/app/src/menus.rs`,
new `docs/help/` topic sources (Markdown compiled into the binary or embedded),
`build.rs`/`include_str!` strategy, `xtask` link check, `docs/USAGE.md` links.

## 5. Decisions required

- Topic source format and embedding strategy (prefer Markdown + a tiny static index;
  no runtime network).
- How command IDs are referenced from topics so PM-05 can validate deep links.
- Whether topics are compiled from `docs/help/` at build time (no duplication with
  USAGE/FEATURES; topics may link to them).

## 6. Implementation steps

1. Author the topic set (start with 12 core topics; every shipped tool, menu, major
   panel, and supported format gets an entry or an explicit `no-topic` reason).
2. Build a static search index (token + synonym table) and a deterministic scorer.
3. Add the Help panel: search field, result list, topic view, breadcrumb, back.
4. Add contextual entry points (`?` on panel headers, error text → topic).
5. Add a coverage test: catalog/tool/panel/format → topic or allowance.
6. Add an xtask link/coverage check to CI.

## 7. Acceptance criteria

- `Help ▸ Help Center` (F1) opens a searchable, keyboard-operable panel offline.
- Searching "make a button", "export sprite", "shortcut undo" returns the right topic.
- Contextual help from a panel opens its topic.
- Every registry command appears in a topic or is explicitly exempt.
- Topic links and anchors pass CI.

## 8. Test plan

- Unit: index build, synonym scoring, deterministic ordering.
- Golden: a fixed query set → expected topic IDs and order.
- Coverage: command/tool/panel/format inventory vs topics.
- `cargo test -p app`, `cargo run -p xtask -- ci`.

## 9. Manual verification

Desktop and served `dist/web` (real browser) keyboard-only walkthrough: open Help,
search, navigate, return to canvas with focus intact.

## 10. Performance, security, portability

Index and topics are embedded static data; measure added WASM bytes against the
budget. No network fetch on open; online links open only on explicit click.

## 11. Risks and rollback

Risk: topic drift. Mitigation: coverage check keyed to the registry; regeneration
is part of the feature checklist. Rollback is a module + data revert.

## 12. Stopping condition and closure evidence

Stop if a topic would require claiming unverified behaviour; write the limitation
instead. Close with the coverage report, the golden query set, and the browser
receipt.
