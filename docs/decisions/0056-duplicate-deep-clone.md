# Decision 0056 — object.duplicate deep-clones groups (Phase 9 debt)

**Debt.** `object.duplicate` inserted a verbatim clone of the source
`ObjectData` — for a group that meant a fresh slot whose `children`
list held the *original's* member ids. The copy shared its members
with the source: editing the clone's children edited the original's,
and deleting the original's members dangled the clone. CS6's
Clone is a deep copy.

## Design

- **`clone_subtree` recurses through group children.** Every live
  descendant mints its own registry slot; the copy's `children` list
  re-points at the fresh slots; the source tree is never touched (the
  original keeps its member ids, so both sides edit independently).
  Symbol instances carry no member closure (definitions are
  referenced by `MSNi` number, not by member ids), so a duplicate
  instance stays one slot — matching the CS6 alias model where
  instances share a master by design.
- **Dangling child references skip** per the walker rule (stale ids
  never error): a clone of a group holding a stale id lands without
  it. The clone's member list is the live closure, not a byte-copy
  of the source's decayed one.
- **Undo retracts every minted slot.** Each created id — root copy
  last, so the root slot retracts first under the reversed group
  fold — carries `ObjectInverse::Created`; the container attach
  (cell insert or parent `children.insert`) keeps its captured
  inverse ahead of them. One undo step restores the exact document
  bytes and empties all the clone's slots, test-pinned.
- **All reads before the first mutation** (000557d): the source's
  liveness, the owner lookup, and the parent group's `before`
  capture all precede `clone_subtree`'s first `registry.insert`.
  The parent capture is safe after the clone because the clone only
  adds slots — it never writes existing objects.

## Consequences

- Registry unchanged (no new commands); parity table unchanged.
- Tests: +1 session test — group two members, duplicate the group,
  assert the clone re-pointed (fresh live slots, originals kept by
  the source), and undo retracts every minted slot with byte-exact
  restoration. The existing duplicate test (leaf path) pins the
  unchanged single-slot shape.
- Phase 9 debt `6.1-object-duplicate` closed. Remaining Phase 9
  debts: pattern-authoring command, mask union fold, path→image
  rasterizer hook, shared-layer link render, resample quality
  (bilinear).
