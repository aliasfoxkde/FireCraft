# T27-20 — Prove the DEB glibc-floor check fails on a too-low floor and passes on an adequate one

> **RP:** RP-01 · **Priority:** P0 · **Status:** IN PROGRESS
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-06

**Objective:** demonstrate that the automated libc6 floor check actually discriminates
— that it rejects an understated floor and accepts a correct one — instead of merely
being present in the code.

**Inputs:** `xtask/src/dist.rs` (`verify_deb_glibc_floor`, `build_deb`);
`packaging/deb/control`; the built binaries' GLIBC symbol versions (read via
`objdump`).

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1 (Linux DEB requirements — the
declared dependency must match the binaries); the repo rule that a check which has
never been shown to fail is not a check.

**Acceptance criteria:** with the declared floor lowered below the binaries' real
requirement, the build fails with a message naming both versions; with the floor at
the actual requirement, the build succeeds.

**Verification:** `cargo run -p xtask -- dist --native`

**Evidence:** `b63a040` added `verify_deb_glibc_floor`; control now floors `libc6 (>= 2.35)`; decision 0091 adds the run-time library set (`libx11-6`, `libxcursor1`, `libxrandr2`, `libxi6`, `libxkbcommon0`, `libxkbcommon-x11-0`, `libgl1`, `libgcc-s1`) with a build gate that refuses a control file that drops one

The floor is only half of "the declared dependency must match the binaries". The
desktop binary `dlopen`s the X11 and GL stack, so neither the ELF dynamic section
nor `dpkg-shlibdeps` can see those dependencies, and the package has to name them by
hand. Decision 0091 declares that set and adds a build gate that refuses a control
file that drops one.

**Blockers:** —

**Stopping condition:** both the failing and the passing cases are recorded with their
outputs.
