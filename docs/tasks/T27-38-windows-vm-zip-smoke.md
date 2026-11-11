# T27-38 — Install and smoke the portable ZIP on a clean Windows VM

> **RP:** RP-02 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-36

**Objective:** prove the Windows artifact works on a real clean Windows machine — not
under Wine on Linux — including the file paths Windows users actually have.

**Inputs:** the MSVC ZIP from T27-36; a clean Windows VM; the release receipt
procedure.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §10 — every required native package
is installed, launched and functionally smoke-tested on its target OS.

**Acceptance criteria:** on a clean Windows VM the app installs, launches, creates and
opens a document, edits, saves and reopens, and exports a PNG; paths containing spaces
and non-ASCII characters work; the results are recorded.

**Verification:** the Windows runner release receipt.

**Evidence:** the 2026-11-06 Wine smoke covered a CLI `import` → `export` PNG round
trip and a GUI launch showing a real 1024×768 "FireCraft" window on Xvfb. Wine is
supplementary runtime evidence, not the Windows target.

**Blockers:** a clean Windows VM.

**Stopping condition:** the full round trip passes on a clean Windows VM, with the
receipt attached.
