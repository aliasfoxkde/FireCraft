# T27-37 — Build the Inno Setup installer from the tag SHA

> **RP:** RP-02 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-36

**Objective:** compile the Windows installer and confirm its identity and metadata are
correct, since a required artifact currently has no built form at all.

**Inputs:** `packaging/windows/firecraft.iss`; the MSVC binaries from T27-36; `ISCC`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.2; the artifact matrix, which
records the installer as "recipe-only, never built".

**Acceptance criteria:** the installer compiles with the correct product name, version,
publisher, install scope, program files destination and shortcut set; the installed
files match the ZIP's contents.

**Verification:** `packaging\build-windows.ps1` on a Windows host with `ISCC.exe` on
`PATH`. The `.iss` is compiled **through the recipe**, not on its own: it carries the
`__APPVERSION__` and `__LICENSEFILE__` stand-ins, which the recipe substitutes and
which the compiler cannot resolve by itself (decision 0101).

**Evidence:** parse-checked only; never built.

**Blockers:** `ISCC` on a Windows host.

**Stopping condition:** a `.exe` installer exists with a recorded digest and its
compiled metadata is captured.
