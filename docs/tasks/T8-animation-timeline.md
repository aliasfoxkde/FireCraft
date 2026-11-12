# T8 — 20.3 Animation timeline

**Objective:** the frames strip with per-frame timing, onion skinning,
and tween instance playback (`symbol.tween`).

**Inputs:** the page/frame model (states), `symbol.tween` command, the
shell's frame navigation UI, CS6 States/Animation panel behavior.

**Authority:** CS6 observed behavior; the engine's frame/animation
command surface (already parity-complete).

**Stopping condition:** strip navigates states, timing edits persist in
the model, onion skin renders (toggleable), tween instances animate;
tests cover state transitions; full gate green.

**Verification:** `cargo test -p app`, manual canvas check (rendered
frames), full gate before commit.

## Status — PARTIAL (2026-11-11)

**Landed** (decision
[0094](../decisions/0094-animation-timeline-shell.md), on the engine
and compositor half of
[0081](../decisions/0081-animation-onion-skinning.md)):

- **The States panel is the timeline.** The strip shows one row per
  state, the active one marked; clicking a row moves the view. The
  state count is the page's own `FRCi` — the count every frame command
  accepts — not the tallest layer's cell list.
- **Per-state timing.** The delay field writes `frame.setDuration`, so
  the delay lands in the page's `frame_durations` record: the record
  the GIF exporter reads. The default delay (ten hundredths of a
  second) is one constant now — `doc::DEFAULT_FRAME_DURATION` —
  shared by the engine's command, the exporter, and the panel.
- **Onion skinning renders, and it is toggleable.** The panel writes
  the two counts through `page.setOnionSkinning`; the canvas calls
  `compose::render_onion` when they are non-zero, and the plain
  render when they are zero. The compositor reads the same counts
  from the page, so the panel and the canvas cannot disagree.
- **CS6's state verbs are in the panel.** Add State, Duplicate State,
  Delete State and Distribute to States, each through its engine
  command.
- **Insert ▸ State activates the state it inserted.** The new state
  lands right after the current one; the view goes there (the action
  used to jump to the last state of the page).
- **Tests.** `crates/app/tests/animation_timeline.rs` covers the
  workflow the panel drives — the delay reaching the page's record
  and undo taking it back, the onion toggle ghosting the neighbouring
  state and switching off again, the add/duplicate/delete state
  transitions, and Distribute to States spreading a layer's items one
  per state. `crates/app/src/shell/timeline.rs` covers the strip's
  own reading of the page.

**Open:**

- **No playback.** CS6's `Play Animation` is not wired: the timeline
  is a place to edit an animation, not yet to watch one. The missing
  piece is a timer that advances the active state on the state's own
  delay.
- **The strip shows numbers, not thumbnails.** CS6's States window
  shows a picture of every state; rendering one per state per frame
  is separate work.
- **The onion ghost opacity is fixed at 50%** (decision 0081). CS6
  carries `OnionSkin_BeforeOpacity` / `OnionSkin_AfterOpacity`
  strings, so a per-side opacity is a plausible extension of the
  same model.
- No screenshot acceptance yet — the panel is unstyled relative to the
  rest of the dock, and the Phase 22 screenshot audit has not covered
  it.
