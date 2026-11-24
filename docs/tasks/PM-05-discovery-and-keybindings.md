# PM-05 — Universal command discovery and keybinding editor

> **Parent workstreams:** UX-02 and UX-03 (POST-MVP-ROADMAP §5.2).
> **Priority:** P1 — the registry already carries 260 commands; finding and binding
> them is the bottleneck.
> **Status:** `NOT STARTED` — gated on Gate 0 for the shell files it shares with the
> local UI work.
> **Owner:** unassigned. **Created:** 2026-11-23.
> **Dependencies:** PM-04 consumes the same metadata for Help deep links; PM-03
> provides format entries for search.

## 1. User problem

Ctrl+P searches commands only. There is no search over tools, panels, help topics,
fonts, styles, symbols, assets, or settings; no aliases or synonyms; no explanation
of why a result ranks where it does. Shortcuts are a fixed read-only table with five
keymap sets and no per-command rebinding, conflict detection, or profile import.

## 2. Current implementation and evidence

- `Shell` command palette (Ctrl+P) over `engine::catalog::all_ids()`; fuzzy scoring,
  recents, saved steps (CS6-PARITY §G/§J; decision for palette v2).
- 260 command IDs across 23 catalog modules; 1,811 `#[test]` functions.
- `crates/app/src/keymap.rs`: `KeymapSet { Cs6, Illustrator, Photoshop, FreeHand,
  WebStandard }` with per-set overrides over the CS6 table; tests assert one
  resolved binding per verb and no dropped bindings.
- **Documentation drift found:** CS6-PARITY §H says the four alternative keymap sets
  are "missing", but `keymap.rs` implements them. The matrix must be corrected
  during Gate 0.
- No rebinding UI, conflict resolver, import/export, or shortcut editor was found.

## 3. Scope

In scope: one command metadata registry (ID, label, description, aliases, category,
shortcut, parameter schema, context, capability, destructive risk, help link);
search over commands/tools/panels/settings/shortcuts/help/fonts/styles/symbols/
assets; explainable deterministic ranking; guided parameter form for required
parameters; disabled/unavailable/destructive states; pinned commands; per-command
rebinding with conflict detection and precedence; workspace profiles; profile
import/export; reset to defaults; generated shortcut tables from the dispatch
authority.

**Non-goals:** semantic/embedding search (optional later), remote search, a second
command registry, or changing existing bindings.

## 4. Files and surfaces likely affected

`crates/app/src/shell/palette.rs`, `crates/app/src/keymap.rs`,
`crates/app/src/menus.rs`, `crates/engine/src/registry.rs` (read-only metadata),
`crates/engine/src/catalog/*` (descriptions/aliases/risk), `xtask/src/shortcuts.rs`,
`docs/CS6-PARITY.md` §H, `docs/USAGE.md`.

## 5. Decisions required

- ADR: *Command metadata and keybinding profile format* — metadata fields, ranking
  formula, conflict precedence, profile schema/version, and reset/rollback.
- Where metadata lives so it cannot drift from the registry (prefer attaching it to
  registration, with a compile/test exhaustiveness rule).

## 6. Implementation steps

1. Extend registration with metadata (description, aliases, category, risk) and a
   test that every ID has metadata.
2. Build the multi-domain index and deterministic scorer; expose a debug view that
   explains the score.
3. Add tool/panel/help/font/style/symbol/asset providers as read-only adapters.
4. Add guided parameter entry for commands with required payload fields.
5. Build the keybinding editor: per-command rebind, conflict list, precedence,
   profile import/export, reset.
6. Generate F1/help shortcut tables from the resolved keymap.
7. Add tests for focus contexts (text fields own the keystroke) and OS conventions.

## 7. Acceptance criteria

- Exact, prefix, fuzzy, synonym, and contextual queries have pinned expected
  orderings.
- No result advertises a command unavailable in the current build/context.
- Rebinding a command updates menus, tooltips, palette, and Help; a conflict is
  refused or explicitly resolved, never silently accepted.
- A bad profile cannot lock the user out of Preferences; reset works.
- A user can recover the CS6 profile in one action.

## 8. Test plan

- Unit/golden: ranking sets, metadata completeness, conflict detection, precedence.
- Integration: rebind → dispatch → menu label update; profile round trip.
- kittest/egui focus tests for text-field and canvas contexts.
- `cargo test -p app`, `cargo run -p xtask -- shortcuts`, `cargo run -p xtask -- ci`.

## 9. Manual verification

Keyboard-only browser test on the served dist/web bundle: search, rebind, reload,
profile persists. Desktop: macOS-style modifiers if available, otherwise record the
limitation.

## 10. Performance, security, portability

Index built once; incremental update on metadata change. No filesystem scan for
fonts at startup (provider is lazy and platform-gated). Profile files are
size-capped and schema-validated.

## 11. Risks and rollback

Risk: metadata duplication. Mitigation: registration-owned metadata + exhaustiveness
test. Rollback is a module revert; profiles are additive.

## 12. Stopping condition and closure evidence

Stop if command payload schemas are not introspectable without a registry redesign;
record the redesign as an ADR instead of guessing. Close with golden ranking sets,
conflict tests, and the browser receipt.
