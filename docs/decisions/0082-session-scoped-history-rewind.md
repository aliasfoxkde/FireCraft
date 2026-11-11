# 0082 — History rewind is a session-scoped command, not a catalog command

Date: 2026-11-10 · Ticket: T20.6 (daily-driver expansion) · Status: accepted

## Context

The History panel listed the session's edit steps but could only undo
one step at a time. CS6's History panel is clickable: clicking a row
rewinds the document to the state that row recorded.

Rewinding is a state operation on the *session*, not a mutation of the
document: it undoes the steps above a depth and drops them. That places
it outside the engine's command registry, whose handlers receive a
`&mut Document` and nothing else.

Putting it in the registry anyway would have been the smaller change,
and it would have broken the catalog's core invariant. The fuzz
property `every_catalog_id_carries_a_payload_template` and the
integration test that every successful catalog command undoes and
redoes *exactly* both assume a catalog command's effect lives entirely
in the document. A rewind's effect lives in the undo stack, so its
undo/redo pair could not be expressed, and the catalog test would fail
on a command that is not wrong.

## Decision

`history.rewind` is a **session-scoped command id**, handled by
`Session::apply` before the registry lookup.

- `engine::session::REWIND_ID` is the id; `catalog::session_ids()`
  lists it and `catalog::all_ids()` lists it alongside the catalog.
  `catalog::ids()` — the registry catalog — does not contain it, so
  the fuzz properties are untouched.
- The payload is `{"depth": n}`, with `deny_unknown_fields`: `n` is the
  undo depth to restore. Depth `0` is the document as opened.
- `Session::rewind_to(depth)` is the engine API; `Session::apply`
  routes the id to it, so the command palette and the MCP bridge drive
  it the same way they drive any other command.
- `Session::history_entries()` returns the History panel's rows: the
  document as opened at depth `0`, then one row per recorded edit.

Two behaviours are deliberate:

- **A rewind clears the redo stack.** Redoing a dropped step would
  replay it against a document that no longer holds the state it was
  captured in.
- **A rewind is journaled.** The journal is the session's record of
  what was applied, and a restarted session must land on the document
  the user sees — the rewound one, not the pre-rewind one. The replay
  path therefore handles the `history.rewind` entry: it rewinds the
  history it has rebuilt so far instead of applying a command.

A `depth` past the recorded steps is refused
(`EngineError::InvalidState`) rather than clamped: a surface asking
for a history that does not exist is asking for something wrong, and
silently rewinding to the beginning would destroy work.

## Consequences

- The catalog stays honest: every id in `catalog::ids()` is a
  document command with an exact inverse. A surface enumerating
  commands must use `catalog::all_ids()` to see the session-scoped
  ones.
- The journal grows with rewinds. A long session that rewinds often
  records both the original commands and the rewind entries; replay
  reproduces the same document, but the journal is no longer a
  minimal description of the state.
- The app surface (clickable History rows, a palette entry) is a
  separate change: the engine half landed first so the surface has a
  contract to build against.

See also decision 0076 (the runtime-resolved workspace root) for the
reason the engine half was validated in its own worktree.
