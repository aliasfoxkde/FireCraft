# T27-52 — Generate the source archive from the exact tagged commit and clean-build it

> **RP:** RP-05 · **Priority:** P0 · **Status:** SMOKE TESTED
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

**Evidence:** the generator is implemented and wired into `release.yml`, and the defect the rehearsal-3 archive exposed — a lockfile whose dependency references did not name the stamped version — was fixed by the `xtask stamp` verb and the `xtask stamps` drift gate. The rehearsal published `firecraft-0.2.5-source.tar.gz` (3,176,024 B, sha256 `0cbee184…`); the workflow's own step reported `clean-extract + cargo metadata --locked verified`, and the archive was then downloaded from the draft and extracted on a **second host**: `cargo metadata --locked` succeeds and `cargo build --locked` finished the dev profile in 53.78s on cargo 1.99.1. The archive's only `.claude/` entry is `.allow_config_edits` — the scratch helper that rehearsal 5 published (`c396b18a`) is gone. The source archive is generated from the tagged commit, carries the sources, the licences and the build inputs, and excludes build outputs and secrets. Receipt: [RECEIPT-0.2.5-DRYRUN-6.md](../release/RECEIPT-0.2.5-DRYRUN-6.md). **Done:** 2026-11-17: the published source archive clean-builds on a second host.

**Blockers:** —

**Stopping condition:** the archive builds from a clean extraction and its digest is
recorded.
