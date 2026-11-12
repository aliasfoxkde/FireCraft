# Contributing

## Before changing code

1. Read [AGENTS.md](https://github.com/aliasfoxkde/FireCraft/blob/main/AGENTS.md) for project rules and the clean-room boundary.
2. Check the live [task board](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TASKS.md), [CS6 parity matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md), and detailed [roadmap](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md).
3. Read the relevant decision, source module, research note and tests before adding a new model or command.
4. Check existing PRs and issues first; do not start a duplicate audit or a competing implementation of work already in review.

## Engineering rules

- Keep crate dependency layers valid; register new workspace crates in `xtask/src/layers.rs`.
- Treat file dimensions, lengths, counts and strings as hostile input; cap proportional allocations and return typed errors.
- No unsafe code or panic-based handling of untrusted files in production code.
- Grow fuzz anchors whenever adding a hostile-input surface.
- Do not commit `source/`, Adobe-derived scratch extracts or generated `dist/` artifacts.
- Add asset attribution in the same change as any new asset.
- Keep FireCraft standalone; integrations and the future SDK are optional consumers, not product prerequisites.
- Adopt shared crates/workflows only after checking licenses, target compatibility, feature footprint and measurable benefit.

## Validate and close out

Use the gates in [AGENTS.md](https://github.com/aliasfoxkde/FireCraft/blob/main/AGENTS.md). Run focused tests during iteration and the full gate at integration/release boundaries; avoid duplicate full builds on an unchanged tree. Update the task ticket, live ledger, generated status where applicable and session notes. Any shell/web change requires a real browser check before it is treated as complete.
