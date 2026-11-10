# T27-30 — Install the Flatpak into a test installation and exercise it under the sandbox

> **RP:** RP-01 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-29

**Objective:** prove the Flatpak installs and works under its sandbox, that its
permissions are minimal and documented, and that the document portal round trip works.

**Inputs:** the bundle from T27-29; `flatpak` and `flatpak-builder`; a test
installation (not the system one); the manifest's `finish-args`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1 — the Flatpak must install, run
under the sandbox, open and save through the document portal, and uninstall cleanly.

**Acceptance criteria:** the app installs into a test installation and runs via
`flatpak run` (not via an unsupported `--repo`); open → edit → export through the
document portal succeeds; removal works; each `finish-args` permission is justified in
writing and the set is minimal.

**Verification:** `flatpak run dev.firecraft.FireCraft`

**Evidence:** the manifest declares `--device=dri --share=ipc --socket=x11
--socket=wayland --socket=fallback-x11 --filesystem=xdg-documents
--talk-name=org.freedesktop.portal.Desktop`; none of it has been exercised.

**Blockers:** a Flatpak SDK/runtime host.

**Stopping condition:** the install → run → portal round trip → remove sequence is
recorded with a per-permission justification.
