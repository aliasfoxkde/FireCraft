# T27-51 — Build and verify the standalone CLI archive

> **RP:** RP-05 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-06

**Objective:** build the headless CLI archive from the candidate commit and verify that
it is a working, self-contained tool.

**Inputs:** `crates/app-cli`; `xtask/src/dist.rs` (`native()` staging); the CLI
subcommands (`import`, `export`, `convert`).

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §10 — source and CLI archives must
extract cleanly and function.

**Acceptance criteria:** the CLI archive builds from the candidate commit; the
architecture, `--version` and `--help` output are correct; an import → export round
trip produces a correct file; the archive extracts cleanly; the checksum is recorded.

**Verification:** `firecraft-cli --version && firecraft-cli import … && firecraft-cli export …`

**Evidence:** the matrix records the CLI tar.gz as built and manifested, but not
re-verified against the candidate commit.

**Blockers:** none beyond T27-06.

**Stopping condition:** the archive builds, the round trip passes, and the digest is
recorded.
