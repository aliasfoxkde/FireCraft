# T27-52 — Generate the source archive from the exact tagged commit and clean-build it

> **RP:** RP-05 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-06

**Objective:** produce the canonical source archive and prove it is buildable from a
clean extraction.

**Inputs:** `xtask/src/dist.rs` (`source()` — git archive, clean-extract,
`cargo metadata --locked`); the tag SHA; the workspace's `Cargo.lock`.

**Authority:** `docs/tasks/T27-cross-platform-release.md` — source archives must extract
cleanly and build with documented prerequisites.

**Acceptance criteria:** the archive is generated from the tag SHA; it contains the
sources, licenses and build inputs; it excludes build outputs, the gitignored `source/`
tree and any secret; a clean extraction runs `cargo metadata --locked` and
`cargo build --locked`; the digest is recorded.

**Verification:** `cargo run -p xtask -- dist --source`

**Evidence:** the generator is implemented and wired into `release.yml`, but it has
never been exercised on a real tag.

**Blockers:** none beyond T27-06.

**Stopping condition:** the archive builds from a clean extraction and its digest is
recorded.
