# 0094 — The States panel is the animation timeline

Date: 2026-11-11 · Ticket: [T8](../tasks/T8-animation-timeline.md) · Status: accepted

## Context

Decision [0081](0081-animation-onion-skinning.md) built the onion
compositor and the per-state delay command, and left the shell half
unbuilt. Its own Consequences said so: *"The shell has no onion toggle
yet … Until the shell wires the toggle, the compositor's only callers
are the tests."* The `States` panel listed the page's states as plain
rows — one `selectable_label` per state, named "State N" — and nothing
else: no delay, no onion control, no way to add, duplicate, delete or
distribute a state from the panel.

Two defects sat under that thin panel:

1. **The strip counted the wrong thing.** It took the tallest layer's
   cell count as the number of states. The frame commands take the
   page's own `FRCi` count (`PageInfo::frame_count`) — the count the
   format carries and the count `frame.setDuration`, `frame.delete`
   and `frame.duplicate` accept. In a document whose layers and page
   disagree (a corrupt file, or a page whose cells were edited
   outside the frame commands) the panel offered states the engine
   would refuse.
2. **Insert ▸ State jumped to the wrong state.** The action inserted a
   frame *after the current one* and then set the view to the *last*
   state of the page. The comment above the code said what CS6 does
   ("one empty frame right after the current one, which becomes
   active on insert"), and the code did the other thing.

The CS6 authority for the panel's shape is in the reference install's
strings: `Window::States` names the window; `Add State`, `Add States`,
`Delete State`, `Delete States`, `Duplicate State`, `Copy to States`
and `Distribute to States` name the operations it offers;
`setOnionSkinning`, `onionSkinBefore` and `onionSkinAfter` name the
DOM verb and its two counts; `Modify::Animation::Settings` and
`{delay:%Ai}` name the animation settings and the per-state delay
field CS6 writes into an exported GIF.

## Decision

- **The strip shows the page's own state count.** The panel reads
  `PageInfo::frame_count` — the same number every frame command
  accepts — and never the layer cell lists. The engine is the
  authority on how many states exist; the panel mirrors it.
- **The timeline lives inside the existing `States` panel.** The
  panel inventory is a pinned gate (`crates/app/panels.snapshot`: 19
  right-dock panels), so a new panel kind would have to be admitted
  there for no gain: CS6's animation surface is the States window,
  and the States window is where the strip belongs.
- **The delay field writes `frame.setDuration`.** The delay lands in
  the page's `frame_durations` record — the same record the GIF
  exporter reads — so an animation timed in the panel exports with
  those delays. The default delay (ten hundredths of a second) is one
  constant, `doc::DEFAULT_FRAME_DURATION`, shared by the engine's
  command, the exporter, and the panel; the three can no longer
  drift apart. (Before this change the same literal was written three
  times, in three crates.)
- **The onion toggle writes `page.setOnionSkinning`, and the canvas
  reads the same two counts.** When the page's counts are non-zero,
  the canvas calls `compose::render_onion` instead of the plain
  render; when they are zero, the plain render is unchanged. The
  compositor reads the counts from the page (0081), so the panel
  cannot set a count the canvas ignores, nor the canvas ghost states
  the panel says are off.
- **The panel offers CS6's verbs**: Add State, Duplicate State,
  Delete State and Distribute to States, each through its engine
  command (`frame.add`, `frame.duplicate`, `frame.delete`,
  `frame.distributeLayer`). The last one is what turns a tweened run
  of instances into an animation: the items of the active layer's
  current state move one per state.
- **A numeric edit is applied when the field loses the focus**, so a
  drag is one journal entry rather than one per drag frame — the
  convention the Properties panel already uses.
- **Insert ▸ State activates the state it inserted.** The new state
  lands at `current + 1`, and the view goes there.

## Consequences

- The compositor has a caller outside the tests, and the delay record
  has a writer outside the exporter.
- The shell binds four commands it did not name before
  (`frame.delete`, `frame.setDuration`, `frame.distributeLayer` and
  `page.setOnionSkinning`); the panel also uses `frame.add` and
  `frame.duplicate`, which the shell already named elsewhere. The
  reachability measurement therefore rises, and the floor is restated
  in the same change — to **168 of 253 registry commands bound by the
  shell (66.40%)**, 85 unreachable. (The rest of the rise over the
  146/245 measured on `main` is the train's keymap batch, not this
  change.)
- The panel draws numbers, not thumbnails. CS6's States window shows
  a picture of every state; rendering one per state per frame is a
  separate piece of work, and the strip is usable without it.
- There is no playback yet. CS6's `Play Animation` is not wired, so
  the timeline is a place to edit an animation, not yet to watch one.
- The onion ghost opacity is fixed at 50% (0081). CS6 carries
  `OnionSkin_BeforeOpacity` / `OnionSkin_AfterOpacity` strings in the
  reference install, so a per-side opacity is a plausible later
  extension of the same model.
- The state count the panel shows and the state count the GIF
  exporter writes are now the same number, by construction.
