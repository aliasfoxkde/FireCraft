# Decision 0013 — Machine surfaces share one registry; typed replies only

Date: 2026-10-12 · Milestone: M9 task 7/8 · Status: accepted

## Context

M9 ships three machine surfaces next to the GUI: `firecraft-cli
run-script`, the loopback JSON control channel (`serve` verb), and an
MCP server (`mcp` verb). Each could grow its own mutation path — a
direct model tweak here, a special case there — and each could fail
differently under hostile input. The M9 design line instead requires
one registry and a fuzz-hard contract.

## Decision

**1. One registry door.** Every surface funnels into
[`engine::Session::apply`] — GUI (`Editor::apply`), CLI journal
replay (`Session::reopen`), the control channel, MCP `firecraft.apply`,
and the scripting drain (Decision 0011). No surface mutates a
document any other way; the parity battery
(`crates/app/tests/automation.rs`) pins byte-equal documents for one
command id through GUI, channel, and MCP.

**2. Transport-free dispatch, thin loops.** `app_cli::channel::handle`
and `app_cli::mcp::handle` are pure functions over `(session,
request) → reply`; the binaries are newline-delimited stdin loops
around them. Tests drive exactly the code the transport runs.

**3. Typed replies, never panics — the fuzz contract.**
- Channel: every reply is `{ok: bool, journal?}` or
  `{ok: false, error: string}`; a refused request never touches the
  journal.
- MCP: protocol failures are JSON-RPC errors (-32601 unknown method,
  -32602 malformed frame, -32700 unparsable frame); tool-level
  failures (registry refusal, missing script, limit halt) are
  `isError` tool RESULTS — a client that can ask a question must get
  an answer, not a transport fault. `notifications/*` take no reply.
- Both dispatchers are driven by deterministic fuzz drivers
  (`fuzz/src/bin/control_channel.rs`, floors in the parity table);
  malformed, truncated, and junk-JSON inputs are answered typed with
  the journal untouched.

**4. Script-host outcomes on surfaces.** `run-script` and MCP
`firecraft.runScript` go through `ops::run_script`: limits fire
pre-drain (document untouched, `LimitExceeded`), drain failure keeps
the applied prefix and names the failing intent (Decision 0011 §4).
Surfaces report the typed outcome verbatim; none retries or partially
re-queues.

**5. Native-only control channel.** The stdin loops are
desktop-only (PLANNING §7.4): the wasm shell embeds the engine
directly, so there is no control-channel attack surface on wasm.
app-cli's `scripting` feature (default on, dropped by
`--no-default-features`) gates `run-script`, the runScript tool, and
the tools/list entry — the advertised table never lies about what the
build can do.

**6. Floors.** `SCRIPT_MUTATION_FLOOR` (8192 cases/sweep) and
`CONTROL_CHANNEL_FLOOR` (4096 sweeps) are rising-only parity floors
enforced by `xtask fuzz-quick`; the scripted-corpus floor (19/89
shipped scripts running headless) by `xtask scripts`.

## Consequences

- A new surface (socket channel, WASM bridge, future RPC) must bind
  to `Session::apply` and the two dispatchers — reusing the fuzz
  contract by construction — or argue a new decision.
- Hostile-input handling is testable without a transport: the battery
  and drivers run in-process.
- The MCP surface is intentionally minimal (two tools). Expanding it
  means expanding `tools_value()` and this document together.
