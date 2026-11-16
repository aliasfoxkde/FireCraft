# T27-51 — Build and verify the standalone CLI archive

> **RP:** RP-05 · **Priority:** P0 · **Status:** IN PROGRESS
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

**Evidence:** the matrix records the CLI tar.gz as built and manifested; rehearsal `v0.2.5-dryrun.3` (run 42937560976) published `firecraft-cli-0.2.5-dryrun.3-linux-x86_64.tar.gz` (8,109,224 B, sha256 `f646feea…`) as a draft asset and the extracted binary launches. **The archive cannot attest its own build**: the CLI had no `--version`, so the receipt had nothing to say which build it was. `firecraft-cli --version` now prints the stamped version — the archive must be rebuilt for this row to close. **Done:** 2026-11-11: rehearsal receipt written; the rebuilt archive is outstanding.

**Blockers:** —

**Stopping condition:** the archive builds, the round trip passes, and the digest is
recorded.
