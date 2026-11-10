# 0081 — Onion skinning composites the neighbouring states as flat 50% ghosts

Date: 2026-11-10 · Ticket: [T8](../tasks/T8-animation-timeline.md) · Status: accepted

## Context

CS6's onion skinning draws the states adjacent to the current one as
translucent ghosts *behind* the current state, so an animator can see
where the motion comes from and where it goes. The DOM verb
`page.setOnionSkinning` was already carried as a class-C command
(`docs/planning/generated/dom-parity.md`, row `setOnionSkinning`), and
the settings it writes are page state: the `OSBi` / `OSAi` nodes, which
the model carries as `PageInfo::onion_before` / `PageInfo::onion_after`.
The verb's own note in the matrix reads "`OSBi`/`OSAi`; 0/0 disables
per CS6" — the settings are two counts of states on each side of the
current one, and nothing else.

Nothing consumed those settings. The compositor drew exactly one state
per call, so the shell could set the counts and the canvas would not
change — the setting was carried but inert. T8's stopping condition
asks for "onion skin renders (toggleable)", so the compositor needed
the onion path.

Two constraints shaped the design:

1. **The compose crate is the deterministic oracle** (decision 0002):
   the same document must always produce the same bytes, single
   threaded, with no clock and no randomness. An onion composite is
   therefore a pure function of the document, the page, the current
   state and the scale — it must hold no state of its own and must not
   read anything the document does not carry.
2. **The frame delay had no writer.** The GIF exporter reads a
   per-state delay out of the page's `frame_durations` record
   (`crates/io/src/gif.rs`), but no command set one: a delay could only
   exist in a document that had been read from a file. The unit is
   hundredths of a second — the GIF delay field's own unit, shared by
   the `FNDi` node (decision 0009) — and the field's ceiling is 65535.

## Decision

- **The composite order is CS6's:** the canvas matte first (opaque),
  then the ghosts, then the current state at its own alpha. The
  current state is composited last and opaque, so where it is opaque
  the pixels are exactly the plain render's pixels — a ghost can never
  tint the state the animator is editing.
- **Ghosts are drawn from their state's content, without the matte.**
  The matte belongs to the canvas, not to a state: a ghost that
  carried its own matte would paint the canvas colour over its
  neighbours instead of letting them show through.
- **The ghost alpha is a flat 50%** — a numerator of 128 over 256,
  applied to the ghost's colour and its coverage alike. The DOM verb
  carries two counts and no opacity, so an opacity would have to be a
  value nothing else reads; a flat factor keeps the composite a pure
  function of the document.
- **The onion settings are read from the page** (`OSBi` / `OSAi`,
  `PageInfo::onion_before` / `PageInfo::onion_after`) inside the
  compositor. The shell only sets them; the compositor never caches
  them, so the document alone determines the render and the two can
  never disagree.
- **`before` and `after` are independent counts.** A page may show two
  states of history and none of the future. The ghost list is ordered
  by increasing distance from the current state, and the compositor
  walks it backwards, so the nearest ghost lands on top of the farther
  ones.
- **Out-of-range frames are dropped and the current state is never a
  ghost.** A page with five states cannot ghost a sixth, and the
  current state is drawn opaque, not translucent.
- **The compositor exposes two doors:** `render_onion` (the page's own
  settings) and `render_onion_with` (an explicit ghost list — the seam
  for a host that wants to preview a different ghost set).
- **`frame.setDuration` records a state's delay** in the page's
  `frame_durations` record, keeping the record positional: a record
  shorter than the page's frame count is padded with the `FNDi` default
  of ten hundredths of a second. A delay outside `0..=65535` and a
  frame outside the page are both refused, and the undo restores the
  page's own record. A page claiming more frames than the format
  allows — 4096, the bound `frame.add` uses — is refused as well: the
  duration record is sized from the page's frame count, so a corrupt
  document must not be able to inflate it.

## Consequences

- The onion view is reproducible: the same document and the same
  settings produce the same bytes, so the compose crate's oracle
  property holds for the onion path as well.
- There is no onion-skinning opacity control, in the document or in
  the shell. A different ghost factor would be a change to the
  compositor, not a setting.
- The page's `frame_durations` record becomes positional the moment
  one delay is set: the states that had no explicit delay take the
  default of 10 — the GIF exporter's own default — so the exported
  bytes do not change.
- `frame.setDuration` is a FireCraft-side command: CS6's DOM has no
  frame-delay verb (the delay is a GIF-export option), so the matrix
  in `docs/planning/generated/dom-parity.md` has no row for it. It is
  grounded in the `FNDi` node and decision 0009 instead. The matrix is
  produced by an external sweep and no gate checks it against the
  command registry, so a command without a row is a gap in the
  document, not a build failure — the same situation as
  `symbol.setScript`.
- The shell has no onion toggle yet: `crates/app/**` is outside this
  change. Until the shell wires the toggle, the compositor's only
  callers are the tests.

## Alternatives rejected

- **Carry a per-ghost opacity in the document.** Nothing else would
  read it, and it would put a rendering knob in the model that CS6
  itself does not have.
- **Give each ghost its own matte.** The matte is a canvas property;
  painting it per ghost would hide the neighbouring ghosts behind the
  canvas colour rather than behind each other.
- **Blend the ghosts into the current state's own render.** The
  current state would then carry a tint that the exporters and the
  undo stack would have to undo; keeping the composite in the
  compositor keeps the model clean and the plain render untouched.
- **A dedicated delta variant for one frame's delay.** The page's
  `PageInfo` delta already restores the whole record, and undo is
  last-in-first-out, so the coarse delta restores exactly what it
  should — a narrower variant would be a second way to say the same
  thing.
- **Read the onion settings from the shell instead of the page.** They
  are page state, written by the DOM verb into `OSBi` / `OSAi`; a
  shell-side copy would be a second source of truth for a value the
  document already carries.
