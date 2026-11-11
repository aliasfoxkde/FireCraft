# T27-42 — Audit the DMG script and verify both macOS architectures build

> **RP:** RP-03 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-04

**Objective:** audit the macOS packaging script and establish that both Apple Silicon
and Intel binaries build, and decide whether a universal2 artifact is achievable.

**Inputs:** `packaging/build-macos-dmg.sh`; `docs/TOOLCHAINS.md`; a macOS host with
Xcode CLT; `lipo`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.3 (macOS requirements, including
the audit and the both-architecture verification).

**Acceptance criteria:** the script is audited and its environment assumptions
recorded; `aarch64` and `x86_64` targets both build; universal2 feasibility is decided
and written; `lipo -info` confirms the slices actually present in the artifact.

**Verification:** `lipo -info <binary>`

**Evidence:** recipe-only; never built.

**Blockers:** no macOS host.

**Stopping condition:** both slices are built and verified, and the universal2
decision is recorded.
