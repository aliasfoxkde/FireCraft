# T10 — 20.6 History rewind as a named command

**Objective:** the session-scoped `history.rewind` becomes a command
every surface can *name*: the command palette offers it, the MCP
bridge's discovery list advertises it, and a palette run takes back the
step the user meant rather than a fuzz depth.

**Inputs:** decision 0082 (the engine half — `history.rewind` as a
session-scoped id, `catalog::session_ids()`, `catalog::all_ids()`);
the shell's command palette and its persisted recents; the MCP bridge's
`firecraft.listCommands` / `firecraft.apply` tools; the engine's
fuzzdrive payload-template table.

**Authority:** decision 0082's design — a rewind is a session
operation, so it stays out of the registry catalog and is enumerated
through `all_ids()`. CS6's History panel remains the clickable
surface; this ticket adds the *command* surface beside it.

**Stopping condition:**

- The palette lists `history.rewind` beside the registry's ids, and its
  "N of M" counter counts the same list.
- The palette's payload for the rewind is grounded to the live history
  (one step below the current depth), and running it takes that step
  back.
- The shell's persisted recents keep a recent `history.rewind` across a
  restart.
- `firecraft.listCommands` lists the session-scoped id, and
  `firecraft.apply` runs it.
- The fuzzdrive payload table carries an arm for the id *without*
  entering it in `CALLS` (that table is asserted to equal
  `catalog::ids()` exactly).
- The reachability floor does not move: the gate counts registry ids,
  and this id is not one.

**Verification:** `cargo test -p app -p app-cli -p engine`; the
palette unit test
`palette_runs_the_session_scoped_rewind`; the integration test
`the_mcp_surface_lists_and_runs_the_rewind`; the full gate
(`cargo run -p xtask -- ci`) before commit.

**Status — LANDED (2026-11-11):** the code, tests, decision 0096 and
the ledger rows are in the tree. The gate receipts are attached to the
pull request.
