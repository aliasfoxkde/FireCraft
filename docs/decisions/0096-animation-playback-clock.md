# 0096 — The animation clock is a core over elapsed time

Date: 2026-11-12 · Ticket: [T8](../tasks/T8-animation-timeline.md) · Issue: [#116](https://github.com/mkinney/firecraft/issues/116) · Status: accepted

## Context

Decision [0094](0094-animation-timeline-shell.md) built the States panel as
the animation timeline and recorded what was still missing:

> There is no playback yet. CS6's `Play Animation` is not wired, so the
> timeline is a place to edit an animation, not yet to watch one.

The missing piece is a timer that advances the active state on the state's
own delay. The naive way to build one — a thread that sleeps, or a
`std::time::Instant` read inside the update path — makes the timing
untestable: the test would have to sleep, and a sleeping test is a flaky
test. It also makes the timing *ambient*: the decision to advance reads the
clock itself, so it cannot be driven with a known elapsed time.

The repository's own rule for this class of problem is the environment-as-data
rule: a decision core takes the environment's value as a parameter and reads
nothing global. The elapsed time of a frame is exactly such an environment
value.

The CS6 authority is in the reference install's strings. `Fireworks.exe`
carries the literal `Play Animation`, next to `Modify::Animation::Settings`,
`Modify::Animation::Animate Selection` and `Modify::Animation::Remove
Animation`; the animation settings carry `{loopCount:%Ai}` and
`{frameLoopingCount:%Ai}`, and the panel's own verbs — `Add State`,
`Duplicate State`, `Delete State`, `Distribute to States` — are the ones
[0094](0094-animation-timeline-shell.md) already wired. So CS6's playback is
a *named operation on the animation*, with a loop count in its settings, not a
mode the document carries into an export.

## Decision

- **The clock is a core over data.** `tick(&mut Playback, elapsed_cs,
  &durations)` is a free function over two values: the elapsed time of the
  frame, and the animation's per-state delays. It never reads the wall
  clock, the environment, or the document. The shell measures the frame's
  elapsed time once, at the edge, and hands it in.
- **The clock's context is recorded when it starts.** `Playback::start`
  records the page, the state count and the journal length — the session
  facts a page switch, a state-count change or a document edit would
  contradict. `Playback::context_holds` compares them, so the stop
  conditions are a pure function of two sides and can be tested without a
  running shell.
- **Playback stops on the conditions CS6 stops on:** a document edit, a page
  switch, a change in the state count, and the end of the last state. The
  first three are facts about the session and are checked by the shell; the
  last one is the clock's own decision.
- **The clock stops at the last state unless it was told to loop.** CS6
  carries a loop *count* in the animation settings; the panel's `Loop` is the
  boolean stand-in for "again from the first state", and it is a preview
  session setting — it is not written into the document.
- **A frame longer than the delay it crosses carries the surplus.** The
  remaining time is added to the next state's countdown rather than
  discarded, so a slow frame makes the animation late by nothing and the
  states keep their recorded intervals.
- **A frame's elapsed time is bounded** (60 s). A backgrounded window or a
  debugger breakpoint must not run the animation through the whole timeline
  in one frame.
- **egui paints on demand, so the shell asks for frames while the clock
  runs.** `Context::request_repaint` is called on every frame the clock is
  playing; otherwise an idle window would freeze the animation between
  input events.

## Consequences

- The timing is tested without sleeping: the unit tests drive the clock
  with an injected duration and assert the state the clock answers with.
- The panel can now *watch* an animation, not only edit one — the timeline's
  last blocking gap is closed.
- The `Loop` checkbox is a preview-session setting, not a document setting.
  CS6's `{loopCount:%Ai}` is a document-level animation setting; carrying it
  would need a new model field and a new engine command, which is separate
  work.
- The clock is not persisted. A document opened again starts stopped, with
  the view on the state the document records.
- The state count the clock started with is compared on every frame; a
  state added while the animation runs stops it rather than desynchronising
  it.
