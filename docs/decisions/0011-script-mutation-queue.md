# Decision 0011 — Script mutations queue and drain through the registry

Date: 2026-10-12 · Milestone: M9 task 3 · Status: accepted

## Context

Shipped Fireworks scripts mutate the document mid-run and read the
result back (`dom.addNewText(...)` then `fw.selection[0]`). The
M9 design line requires that scripted mutations ride the same
command registry as GUI/CLI mutations — journal, undo, and replay
semantics for free — and that a failing script leaves the document
untouched.

boa 0.20's safe native-function path
(`from_copy_closure_with_captures`) only captures `Trace` types, so
a native host call cannot borrow `&mut Session` from outside the
interpreter. Mid-run re-entry into the session is therefore not
reachable without `unsafe`.

## Decision

Mutating host calls **queue** `(command id, payload JSON)` intents
in GC-held host state; [`run_in_session`] drains the queue through
`Session::apply_delta` — the same door GUI and CLI use — only when
the run completes.

1. **Failure is transactional.** A throwing or limit-aborted run
   drops its queue: nothing applied, journal untouched. This is
   stricter than CS6 (which leaves earlier statements applied) and
   is the M9 design line's own promise.
2. **Revert is exact.** `fw.setUpRevert` marks the queue position;
   `fw.revert` discards queued commands after the mark during drain
   resolution — before any application, so document, journal, and
   history show no residue. (CS6's live-apply revert would need
   undo; ours never applies, so it simply truncates.)
3. **Order is queue order.** The drain applies survivors in the
   order the script issued them, so `script run ≡ journal replay ≡
   direct invocation` byte-for-byte (pinned per mutator in
   `crates/scripting/tests/mutate.rs`).
4. **A drain-time command failure stops the drain**, keeps what did
   apply, and reports `Outcome::Threw` — the script's declared
   effects did not land.
5. **View state is not document state.** `fw.selection`
   (element list), `currentLayerNum`, and `changeCurrentPage` are
   script-host state: they never journal, and re-attach re-projects
   them from the engine (selection also from the drain's
   created-object bookkeeping — `addNew*` selects what it created,
   matching Fireworks).
6. **Read staleness is scoped.** Between queue and drain, reads
   project the pre-mutation snapshot. The shipped corpus's mutating
   scripts either finish with their mutations or read back through
   the next `run_in_session` attach; the corpus run harness (M9
   task 5) is the gate that this model carries the shipped scripts.

## Consequences

- Session gains `apply_delta` (returns the delta so the drain can
  select created ids); `apply` delegates.
- Catalog gains `layer.add` / `layer.delete` (corpus: `addNewLayer`,
  `deleteLayer`) with whole-list snapshot deltas, last-layer delete
  refused, fuzz coverage required by the existing catalog-coverage
  test.
- Payload ids are projected onto elements as `__fwId`
  (`{index, generation}`) — exactly the payload serde shape, so JS
  builds command payloads without a second id dialect.
- The projection gained the corpus's real access shapes discovered
  while wiring this: `layer.frames[f].elements`, settable
  `currentLayerNum` (default: top layer), `isMasterPageLayer` /
  `masterPageNum`, `selectNone`.
- Smart Shape handler dispatch (`smartShape.GetCurrentContour` & co
  → `autoshape.*`) is deferred to the corpus report (task 5), which
  enumerates the handlers the shape library actually needs; the
  refusals name task 5.
