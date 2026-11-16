# T27-36 — Build the MSVC desktop, CLI and portable ZIP on a Windows runner

> **RP:** RP-02 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-35

**Objective:** produce the Windows release artifact from the release-path toolchain —
the MSVC build — not from the fleet's GNU cross-build.

**Inputs:** `packaging/build-windows.ps1`; the tag SHA; a Windows runner with the MSVC
toolchain; `docs/TOOLCHAINS.md` pinning rules.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.2 — every required artifact is
built from the exact release tag SHA with locked dependencies and pinned toolchains.

**Acceptance criteria:** the desktop and CLI binaries build for
`x86_64-pc-windows-msvc` from the tag SHA; the ZIP has a stable top-level layout, the
required runtime DLLs and the license files; the artifact's digest and size are
recorded.

**Verification:** `cargo build --release --locked --target x86_64-pc-windows-msvc`

**Evidence:** the 2026-11-06 fleet build is a `x86_64-pc-windows-gnu` cross-build
(cargo 19m39s) producing a 72 MB zip (`0cd17a87…710f72`). It is preview evidence only:
the artifact matrix itself names the MSVC operator build as the release-path
authority.

**Open decision for the operator (2026-11-12, from the T27-35 audit).** An MSVC build
links the Microsoft C runtime dynamically, so the binaries need `vcruntime140.dll` (and
friends) on the machine that runs them. Two honest options, both recorded rather than
chosen silently: install the VC++ 2015-2022 redistributable on the smoke VM (T27-38),
or build with `-C target-feature=+crt-static` so the binaries carry the runtime
themselves. Whichever is taken, the receipt names it — the clean-VM smoke is the row
that proves it.

**Blockers:** a Windows runner with the MSVC toolchain.

**Stopping condition:** an MSVC-built ZIP exists with a recorded digest, and the
distinction from the GNU preview build is stated in the matrix.
