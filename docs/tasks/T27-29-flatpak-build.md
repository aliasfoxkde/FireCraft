# T27-29 — Build a Flatpak bundle from the manifest

> **RP:** RP-01 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-06

**Objective:** produce a real `.flatpak` bundle, proving the manifest is buildable
rather than only parseable.

**Inputs:** `packaging/build-flatpak.sh`;
`packaging/flatpak/dev.firecraft.FireCraft.yml`; `flatpak-builder`; the
`org.freedesktop.Sdk//24.08` SDK.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1 — the Flatpak must be built from
an identified source revision, not from whatever happens to be in the working tree.

**Acceptance criteria:** `flatpak-builder` produces a `.flatpak` bundle from the
manifest; the bundle's source is an identified revision (the manifest currently uses
`type: dir, path: ..`, which points at the local tree and must be repointed or its
SHA asserted); the bundle's digest is recorded.

**Verification:** `packaging/build-flatpak.sh && flatpak build-bundle`

**Evidence:** Manifest is YAML-parse-checked by `dist --native`; never built

**Blockers:** Flatpak SDK 24.08 absent on this fleet

**Stopping condition:** a `.flatpak` bundle exists with a recorded digest and a
recorded source revision.
