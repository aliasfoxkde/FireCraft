# 0069 — The CS6 web panels drive the engine, not a panel-side model

- **Date:** 2026-11-04
- **Status:** Accepted
- **Supersedes:** nothing
- **Relates to:** `docs/NEXT.md` Phases 19.6 and 22.1, `docs/HANDOFF.md`,
  `docs/PROGRESS.md`, decision 0017

## Context

Phase 22.1 turned the CS6 panel inventory into real windows: one egui
`Window` per panel, docked in a right-hand column by default, each
toggleable from the Window menu. The panel *set* was CS6's, but three
of the panels the Window menu names — Auto Shapes, URL, and Behaviors —
still rendered their menu rows as `Action::Inert`, because no panel
content existed for them and the engine verbs they would need were
never exercised from a surface.

That leaves three choices, and the third is the one taken here.

The first is to leave the rows inert and wait for the features that
would justify them. But the verbs *do* exist: `object.createPath` and
the ten `autoshape.*` commands create the shapes; `slice.setLink` sets
a slice's anchor URL and alt text; `slice.addBehavior` /
`slice.removeBehavior` carry the behaviors; `slice.setData` carries the
export-facing parameters. The panels were the missing surface, not the
missing feature — deferring them deferred *reachable* behavior.

The second is to build a panel-side model — a list of shapes, a link
field, a behavior list held in the shell — and sync it into the
document. That is what makes a panel a lie: the document is the
authority, a second copy is a second truth, and the two drift the first
time the file is opened, exported, or edited elsewhere.

The third is what this decision records: the panels *are* the engine
commands, with the panel holding only what is genuinely view state.

## Decision

1. **Every panel action is one engine command.** The Auto Shapes
   gallery issues `object.createPath` (the two primitives) or the
   matching `autoshape.*` command (the ten tools). The URL panel
   issues `slice.setLink`. The Behaviors panel issues
   `slice.addBehavior` / `slice.removeBehavior`, and a swap-image
   behavior additionally issues `slice.setData` with the `swapTarget`
   key. No panel writes to the document by any other route.

2. **The panel holds view state only.** The URL panel's draft (the
   link and alt text being typed) and the Behaviors panel's chosen
   swap target are shell-local, and they are re-read from the document
   whenever the selection changes. Everything the panel *displays* is
   copied out of the document read-only before a command runs, so a
   command cannot invalidate a borrow the panel is holding.

3. **Only documented wire semantics are offered.** The behavior kinds
   this build names are the ones the node dictionary and the engine
   catalog document: `BETi` 9 (popup menu) and the swap-image kind 2.
   A stored kind with no documented meaning is *displayed* with its
   raw number and is still removable, rather than being given an
   invented label or silently dropped. The swap-image target rides the
   `swapTarget` export key of decision 0017 — the same key the
   exporter reads.

4. **Placement is a document operation, not a UI guess.** A gallery
   click inserts at the canvas center, sized to fit the canvas. The
   payload builder is the *same* one the creation drag uses
   (`AutoShape::payload_from`), so the circle-bound shapes get a center
   and a radius, the frame-bound shapes a box, and the line-bound
   shapes two endpoints — one rule, two callers, no second geometry
   path to drift.

5. **An empty field clears.** The URL panel sends `null` for a blank
   link or alt text, which the engine already reads as "clear the
   stored value" — so the panel has no separate "erase" gesture and
   cannot leave a stale value behind a blank box.

## Consequences

- The Window menu's Auto Shapes, URL, and Behaviors rows are live, and
  the inert count falls 11 → 8 of 289. The ratchet in
  `menus::tests::inert_count_never_rises` was re-pinned to the derived
  8.
- The panels are tested through the shell, not through the engine
  alone: the insert lands an object and a journal row, the link lands
  on the slice, the behavior list round-trips — so a panel that
  stopped reaching the engine would fail the suite.
- `AutoShape` grew a `payload_in_rect` entry point beside its drag
  path. The drag builder was refactored into a single `payload_from`
  that both call, so the two paths cannot disagree about a shape's
  placement.
- The remaining inert rows are feature-gated rather than reachability
  debt: Optimize, Common Library, CSS Properties, and Find and
  Replace need engine work that does not exist yet, and the Text Font
  placeholder and the scripting family are CS6 artifacts (a verbatim
  resource row and the scripting host) rather than missing surfaces.
