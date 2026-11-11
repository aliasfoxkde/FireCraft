# Contributing

## Before changing code

1. Read [`AGENTS.md`](https://github.com/aliasfoxkde/FireCraft/blob/main/AGENTS.md) for project rules and the clean-room boundary.
2. Check the [parity roadmap](Parity-Roadmap) and the detailed project plan in [NEXT.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md).
3. Read the relevant [decision](https://github.com/aliasfoxkde/FireCraft/tree/main/docs/decisions), source module, and research note before adding a new model or command.

## Engineering rules

- Keep the crate dependency layers valid; register new workspace crates in `xtask/src/layers.rs`.
- Treat file dimensions, lengths, counts, and strings as hostile input; cap proportional allocations and return typed errors.
- No unsafe code or panic-based handling of untrusted files in production code.
- Grow the fuzz anchors whenever adding a hostile-input surface.
- Do not commit `source/`, Adobe-derived scratch extracts, or generated `dist/` artifacts.
- Add asset attribution in the same change as any new asset.

## Validate and close out

Use the gates in [`AGENTS.md`](https://github.com/aliasfoxkde/FireCraft/blob/main/AGENTS.md). Update the task ticket, task board, generated status where needed, and session notes. Any shell or web change requires a real browser check before it is treated as complete.
