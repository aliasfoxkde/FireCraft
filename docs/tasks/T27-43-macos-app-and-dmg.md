# T27-43 — Build the `.app` bundle and the DMG with correct metadata

> **RP:** RP-03 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-42

**Objective:** produce the macOS application bundle and disk image with the metadata a
signed macOS release requires.

**Inputs:** the built binaries from T27-42; `hdiutil`; the bundle's `Info.plist`,
icons and license files.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.3 and §4.

**Acceptance criteria:** the bundle has a correct `Info.plist` — bundle identifier,
version, minimum system version, icons, embedded licenses and the required
`NSHighResolutionCapable`/`LSMinimumSystemVersion` keys; the DMG is built and passes
`hdiutil verify`; the artifact is checksummed.

**Verification:** `hdiutil verify <dmg> && shasum -a 256 <dmg>`

**Evidence:** none yet.

**Blockers:** a macOS host.

**Stopping condition:** the `.app` and `.dmg` exist, the DMG verifies, and the bundle's
metadata is captured.
