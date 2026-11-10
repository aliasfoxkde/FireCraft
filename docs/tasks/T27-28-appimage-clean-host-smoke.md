# T27-28 — Functionally smoke the AppImage on a clean Linux host and record its checksum

> **RP:** RP-01 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-27

**Objective:** prove the AppImage is not merely launchable but usable — a real
open → edit → save/export round trip — and record the artifact's digest.

**Inputs:** the AppImage from T27-27; a clean Linux host or container with a display;
the headless input-probe procedure (shape-tool drag with an export-bytes diff).

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §10 (exit criteria — a native package
must be installed, launched and functionally smoke-tested on its target OS).

**Acceptance criteria:** open → edit → save/export succeeds from the AppImage on a
clean host; the output differs from the input in the expected way; the SHA-256 and a
receipt are recorded.

**Verification:** `sha256sum dist/firecraft-*-linux-x86_64.AppImage`

**Evidence:** none yet.

**Blockers:** a clean host with a display or an equivalent headless path.

**Stopping condition:** a successful round trip with a recorded digest.
