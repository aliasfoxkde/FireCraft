# 0068 — CS6 shell fidelity and the icon-asset policy

- **Date:** 2026-11-03
- **Status:** Accepted
- **Supersedes:** nothing
- **Relates to:** `docs/NEXT.md` Phase 22, `docs/PROGRESS.md`, `docs/ATTRIBUTION.md`

## Context

FireCraft's engine carries the CS6 command surface, but the shell that
presents it does not look or behave like CS6. The user directive
(2026-11-03) is explicit: the program must present as CS6 — dockable
panels with the CS6 panel inventory, a CS6-style tool strip, a
right-click context menu that adapts to what is selected, an editable
property inspector — and it must reach modern feature parity on top of
that. Fireworks CS6 was abandoned in 2013, so the value of FireCraft
is that the abandoned application is usable again.

Two constraints shape how that is built.

**Clean-room.** Adobe's assets (icons, bitmaps, panel artwork, splash
screens) are copyrighted and are not copied into this repository. The
reference install under `source/` is analysis input only. Behavior is
derived from the CS6 `Configuration/` Extendscript specs, the exe's
own string tables, and public documentation.

**No oracle.** The CS6 install cannot be launched as a behavioral
oracle. Under Wine it fails its own configuration check ("Error: 1")
before any UI is drawn, and the activation/licensing machinery is
explicitly out of scope for this repository — so no license path is
attempted and none is needed. The reference material that *is*
available: the `Configuration/` tree (89 `.jsf` behavioral specs), the
exe's string tables (for example the `string:contextmenu.*` family of
18 panel/document/tab/drawer verbs), the embedded PE resources, and
observed behavior.

## Decision

1. **The shell targets CS6 fidelity, not a FireCraft-specific
   design.** Scoped as Phase 22 in `docs/NEXT.md`: panel inventory and
   docking (22.1), the tool strip (22.2), the adaptive canvas context
   menu (22.3), the property inspector (22.4), CS6 chrome (22.5), and
   the icon pack integration (22.6).
2. **Icons come from open-license packs, never from Adobe.** The
   survey and its per-pack licensing facts live in
   `docs/planning/generated/icon-resources-survey.md`; each pack we
   actually ship gets a row in `docs/ATTRIBUTION.md` in the same
   change as the assets. A pack whose license forbids embedding, or
   whose copyleft reaches the work, is not used.
3. **Behavior is derived, not transcribed.** The adaptive context
   menu is specified per selection class (nothing selected, bitmap,
   vector path, text, instance, multi-selection, inside a slice) and
   each class's verb set is drawn from the engine's own command
   registry, so every row in it executes.
4. **Honest gaps stay visible.** Where a CS6 capability has no
   implementation — e-mail delivery, for instance — the shell says so
   rather than pretending.

## Consequences

- The wasm bundle grows by the icon payload; the size ceiling must be
  re-pinned with a measurement, not a guess (Phase 22.6).
- Panel layout state becomes persisted application state, which the
  preferences work (Phase 21.4) will own.
- The inert-menu ratchet (19.6) and Phase 22 overlap on the surface:
  Phase 22's context-menu verbs are menu verbs, so both advance the
  same census.

## Alternatives rejected

- **Reuse the CS6 artwork from the install.** Violates the clean-room
  policy and the repository's asset rule.
- **Skip the reference and design a "modern" editor instead.** The
  user's stated goal is the CS6 experience; a different design does
  not serve it.
- **Continue the burn-down and defer the shell to "later".** The
  shell is the visible surface; leaving it un-CS6 while the verbs
  behind it land produces a program that works but does not read as
  Fireworks.
