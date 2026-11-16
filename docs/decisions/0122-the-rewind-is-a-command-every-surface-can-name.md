# 0096 — The rewind is a command every surface can name

Date: 2026-11-11 · Ticket: [T10](../tasks/T10-history-rewind-command.md) · Status: accepted

## Context

Decision 0082 landed the engine half of the history rewind:
`history.rewind` is a **session-scoped** command id — handled by
`Session::apply` before the registry lookup, because a rewind acts on
the undo stack rather than on the document, and a catalog command is
required to have an exact document-level inverse.

That record named the app half as a separate change and warned about
the trap it leaves behind:

> A surface enumerating commands must use `catalog::all_ids()` to see
> the session-scoped ones.

The app surface was still enumerating `catalog::ids()`. The History
panel's rows were clickable (the panel rewinds through
`Session::rewind_to` directly), but the command palette offered only
the registry's ids — so `history.rewind` was a command the engine
could run, the palette could not name, and the MCP bridge's
`firecraft.listCommands` could not advertise. An agent that discovers
its commands by asking could not find the rewind; a user who knew the
id could not type it.

## Decision

Every surface that enumerates commands enumerates the commands it can
*run*, not the subset the registry owns.

- `crates/app/src/shell/palette.rs` filters `engine::catalog::all_ids()`
  — the registry's ids plus the session-scoped ones — and the palette's
  counter ("N of M") counts the same list.
- The shell's persisted *recents* are restored against the same list,
  so a recent `history.rewind` survives a restart (a name the build
  cannot run is still dropped).
- `crates/app-cli/src/mcp.rs` — `firecraft.listCommands` lists
  `session.command_names()` plus `catalog::session_ids()`.
  `firecraft.apply` already routed the id through `Session::apply`, so
  the command was runnable but invisible; discovery now matches
  capability.
- `crates/engine/src/fuzzdrive.rs` carries a payload arm for
  `history.rewind` (`{"depth": n}`), so the template table that the
  palette and the fuzz driver share can build the command's payload.
  The arm is deliberately **not** in `fuzzdrive::CALLS`: that table is
  asserted to equal `catalog::ids()` exactly, and the session-scoped
  id is not a registry id.

The palette's payload for the rewind is **grounded to the live
history**: one step below the depth the document currently sits at.
The fuzz template's depth is a random number in `0..4` — right for a
fuzz driver, wrong for a user, who means "undo the last thing".

## Consequences

- The palette's catalogue grows by one command. The reachability
  measurement does not move: the gate counts the *registry* ids the
  shell binds, and `history.rewind` is not a registry id, so
  168/253 (66.40%) stands and the floor is untouched.
- The palette's "covers every command" property now covers the
  session-scoped id too, and the property is stated against
  `all_ids()` so it cannot silently drift back to the registry subset.
- `catalog::ids()` remains the registry's own list, and the fuzz
  properties that depend on it are unchanged.
- A future session-scoped command (a session save, a session reset)
  becomes nameable in the palette and in the MCP discovery list by
  being added to `catalog::session_ids()` — the enumeration sites do
  not need to change again.

See also decision 0082 (the engine half this completes) and decision
0076 (the runtime-resolved workspace root that lets the gate measure
whichever worktree it runs in).
