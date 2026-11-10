# 0076 — The xtask gates resolve the workspace root at run time

Date: 2026-11-09 · Ticket: [T27](../tasks/T27-cross-platform-release.md) · Status: accepted

## Context

`xtask`'s `util::workspace_root()` returned
`PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")` — the path baked
into the **binary** at compile time. Every gate verb resolves its tree
through it: the Aegis scan and baseline, `dist`, `coverage`, `floor`,
`corpus`.

The binary and the tree it acts on are usually the same thing, because
CI builds the xtask from the checkout it is validating. They are not the
same thing when a developer shares one `CARGO_TARGET_DIR` across
worktrees: if the xtask sources in two worktrees are byte-identical,
cargo does not rebuild, and the binary executed in worktree B still
carries worktree A's `CARGO_MANIFEST_DIR`.

Observed on 2026-11-09 while porting the content-keyed Aegis gate onto
the marketing-site branch: `cargo run -p xtask -- aegis --bless`
reported "465 key(s) written (0 added, 0 removed)" while that worktree's
`.aegis-baseline.tsv` was untouched — the run had blessed PR #18's
tree. The command succeeded and reported a clean result on the wrong
tree. For a secret-scanning gate, that is a silent false pass.

## Decision

`workspace_root()` resolves the tree at **call time**, from the current
working directory, and falls back to the compile-time path only when the
invocation is outside any workspace.

The root is the nearest ancestor directory holding both the workspace
manifest (`Cargo.toml`) and the `xtask` crate (`xtask/Cargo.toml`). The
second condition matters: the `web/` crate carries its own `[workspace]`
table, so a manifest alone would let a nested crate claim to be the
root.

## Consequences

- The gates act on the tree they are invoked in, so a stale binary in a
  shared target directory can no longer bless, scan, or package another
  worktree.
- CI is unaffected: the checkout root is both the working directory and
  the compile-time root.
- The invocation directory is now load-bearing. `xtask` must be run from
  inside the repository (or a subdirectory of it); a run from outside
  falls back to the compile-time tree, which is the previous behaviour.
- Five unit tests cover the resolution: the live root, a nested
  directory, a directory outside any workspace, a manifest without the
  xtask crate, and the nearest-qualifying-ancestor rule.

See also decision 0074 (the content-keyed Aegis baseline this makes
safe to re-bless from any worktree).
