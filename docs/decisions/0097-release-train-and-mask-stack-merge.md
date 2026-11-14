# 0097 — The release train and the mask stack land as one tree, and the floor follows the merged measurement

Date: 2026-11-11 · Tickets: [T7](../tasks/T7-optimize-panel.md), [T8](../tasks/T8-animation-timeline.md), [T9](../tasks/T9-mask-stack.md) · Status: accepted

## Context

Two feature lines were in flight from the same base.

- The **release train** landed on `main` in three steps: the Optimize
  panel ([T7](T7-optimize-panel.md), decision
  [0095](0095-optimize-panel-honors-only-what-export-honors.md)), the
  animation timeline ([T8](T8-animation-timeline.md), decision
  [0094](0094-animation-timeline-shell.md)), and the document tabs
  (20.5). Each step re-pinned the reachability floor and restated the
  Phase-20 row in the task ledger.
- The **mask stack** ([T9](T9-mask-stack.md), decisions
  [0092](0092-mask-stack-combine-mode.md) and
  [0093](0093-mask-stack-levels-and-folding.md)) grew on its own branch
  from the older base. It added four registry commands, the model's
  mask-stack entry, FWMap persistence for it, the compositor's
  bottom-up fold, and the Properties-panel level list with per-level
  link/unlink.

Both branches had touched the same four files — the doc crate's public
surface and the ledger that records Phase 20 — so the merge had real
conflicts, and both branches had quoted *their own* reachability
measurement in the ledger and in `NEXT.md`, `GAPS.md` and
`PROGRESS.md`. Only one number can be in the merged tree.

## Decision

1. **Merge, do not rebase-and-pick.** The release train is merged into
   the mask-stack branch, and the mask stack's whole tree lands with
   it. Neither line is reimplemented against the other: the release
   train's Optimize panel, animation timeline and document tabs, and
   the mask stack's model, persistence, fold and level panel, are all
   in the merged tree.
2. **The doc crate exports the union.** `crates/doc/src/lib.rs`
   re-exports `model::{…}`; the merged list carries the release
   train's `DEFAULT_FRAME_DURATION` and the mask stack's
   `MASK_STACK_MODE_KEY` and `MaskCombineMode`. Both sides' names are
   genuinely in `model`, and each is used by the half that added it.
3. **One Phase-20 ledger row, naming both landings.** The row reads
   "20.1, 20.2, 20.3, 20.5 landed; 20.4 partial", and the detail
   names the Optimize panel, the animation timeline and the mask
   stack with their tickets. 20.4 stays `PARTIAL` — its screenshot
   acceptance and the remaining add/remove/move affordances are
   open — so the merged ledger does not claim more than landed.
4. **The floor follows the merged measurement.** The reachability gate
   measures the tree it runs in: the merged tree binds **172 of 257
   registry commands (66.93%)**, 85 unreachable. The floor file is
   re-pinned to that measurement, which is the legal direction of the
   one-way ratchet (the release train had already pinned 66.40%). The
   quoted numbers in `NEXT.md`, `GAPS.md` and `PROGRESS.md` are
   restated to the merged measurement so the ledger and the gate
   cannot disagree.

## Consequences

- The floor rises from 66.40% to 66.93%: a shell that binds fewer
  commands than the merged tree does now fails the gate. The pin was
  made from the merged measurement, not from either side's, so the
  ratchet reflects what the merged tree actually reaches.
- Decisions [0092](0092-mask-stack-combine-mode.md),
  [0094](0094-animation-timeline-shell.md) and
  [0095](0095-optimize-panel-honors-only-what-export-honors.md) keep
  the measurements that were true when they were written. They are
  dated records, not live counters; the live figure lives in
  `.reachability-floor.tsv` and is quoted by the docs.
- The mask stack's remaining acceptance — the screenshot and the
  add/remove/move affordances — stays open in
  [T9](T9-mask-stack.md) and in the ledger's 20.4 row. Merging does
  not close it.
- The two branches' Phase-20 items that were never in conflict (the
  Optimize panel's Adam7/JPEG work, the timeline's playback and
  thumbnails, the history rewind, slice behavior) keep their own
  tickets and their own open state.
