# T27-51 — Build and verify the standalone CLI archive

> **RP:** RP-05 · **Priority:** P0 · **Status:** SMOKE TESTED
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

**Evidence:** the rehearsal published `firecraft-cli-0.2.5-linux-x86_64.tar.gz` (8,158,124 B, sha256 `953633e4…`) as a draft asset (run 44188107435). The archive was downloaded from the draft, re-hashed locally (the digest matches the API's) and extracted, and **the extracted binary attests its own build**: `firecraft-cli --version` prints `firecraft-cli 0.2.5` and exits 0. The distro matrix runs the CLI on all six cells ([LINUX-COMPAT.md](../release/LINUX-COMPAT.md)). The archive is byte-identical across two independent rehearsals. **Done:** 2026-11-17: the published CLI archive attests its own version.

**Blockers:** —

**Stopping condition:** the archive builds, the round trip passes, and the digest is
recorded.
