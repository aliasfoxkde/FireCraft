# T27-21 — Build the DEB from the candidate commit, inspect its metadata, install it clean and run a CLI round trip

> **RP:** RP-01 · **Priority:** P0 · **Status:** IN PROGRESS
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-20

**Objective:** produce the DEB from the exact release candidate and demonstrate, on a
clean Debian-family container, that it installs, runs, and does what it claims.

**Inputs:** `packaging/deb/control`; `xtask/src/dist.rs` (`build_deb`, `verify_deb`);
a clean Debian 12 container image.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1; the artifact matrix's DEB row.

**Acceptance criteria:** the DEB builds from the candidate commit; `dpkg-deb --info`
and `--contents` agree with the declared metadata; a clean Debian 12 install succeeds
and a CLI import → export round trip produces a correct output file.

**Verification:** `dpkg-deb --info dist/native/firecraft_0.2.5_amd64.deb && apt install ./firecraft_0.2.5_amd64.deb`

**Evidence:** a clean-container smoke passed 2026-11-06 (Debian 12 image: apt install →
functional CLI import/convert/export round trip → reinstall → remove → purge), which
caught and fixed the 2.31 vs 2.35 libc6 floor.

The pipeline now performs this check on every run
(`packaging/deb/verify-install.sh`, stage by stage). On 2026-11-11 the run for
`4273471` (pipeline run `85502100`) passed the install,
desktop-entry, icon, CLI and document round-trip stages and failed the GUI launch:
the desktop binary aborted with "Library libxkbcommon-x11.so could not be loaded",
because the package declared only `libc6` and the verification image installed only
`libxkbcommon0`. Decision 0091 declares the run-time library set the binary
`dlopen`s, and the verification now installs exactly that set plus its harness.

**Blockers:** a clean Debian 12 environment.

**Stopping condition:** the DEB is built from the candidate commit, installed clean,
the round trip passes, and the artifact's digest is recorded.
