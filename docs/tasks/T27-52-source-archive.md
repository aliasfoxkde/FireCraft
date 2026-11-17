# T27-52 — Generate the source archive from the exact tagged commit and clean-build it

> **RP:** RP-05 · **Priority:** P0 · **Status:** IN PROGRESS
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

**Evidence:** the generator is implemented and wired into `release.yml`; **exercised — and it failed**. The rehearsal's archive (`FireCraft-0.2.5-dryrun.3-source.tar.gz`, asset 733682287) was downloaded from the draft and extracted: `cargo metadata --locked` exits **101** ("cannot update the lock file … because --locked was passed to prevent this"). The manifests are stamped; the lock's dependency *references* and the path dependency's version requirement were not. Fixed by the `xtask stamp` verb and the `xtask stamps` drift gate; the row closes when a rebuilt archive clean-builds. **Done:** 2026-11-11: defect reproduced from the published bytes; fix implemented, verification outstanding.

**Blockers:** —

**Stopping condition:** the archive builds from a clean extraction and its digest is
recorded.
