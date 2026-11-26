# T30 — Native taskbar/Dock launcher menu

**Status:** Planned — baseline has no taskbar adapter; Windows Jump List behavior exists only in the separate experiment repository and is not yet reconciled or policy-compliant for direct porting.

## Objective

Provide useful right-click actions on FireCraft's taskbar/Dock/desktop launcher, including New/Open, recent and pinned documents where the platform supports them, current documents where appropriate, and safe activation of an already-running app. Keep notification-area system tray support out of scope.

## Inputs

- [Product/UX roadmap Phase 6.1](../PRODUCT-UX-ROADMAP.md#phase-61--taskbar-and-launcher-integration-p1).
- [Platform research](../research/taskbar-context-menus.md).
- [Task board](../TASKS.md) and repository rules in [`../../AGENTS.md`](../../AGENTS.md).
- Experiment-only Windows sample: `/nas/Temp/repos/FireCraft-CraftAnything-Experiment/app/src/shell_windows.rs`; reference behavior only, not implementation authority.

## Work sequence

### T30.1 — common document history and favorites

- Replace the inert `File → Open Recent` row with a real most-recent-first menu and Clear Recent action.
- Add a separate pinned/favorites document model with stable user-defined ordering and pin/unpin behavior.
- Persist through a versioned settings schema; cap counts, retain display names, avoid storing document contents, handle stale/deleted paths, and document privacy/clear controls.
- Route selections through the existing file and dirty-document workflows. Add tests for persistence, deduplication, order, missing files and safe close/open decisions.

### T30.2 — shared launcher command model

- Define stable, platform-neutral actions and command-line activation arguments for New, Open, open a path, show/focus, and Quit.
- Handle URI/path quoting, Unicode, multiple startup arguments, repeated activation and already-running instances.
- Route launch requests to an existing instance or documented tab policy without bypassing unsaved-document confirmation.

### T30.3 — Windows Jump List

- Set a stable AppUserModelID before creating the desktop window.
- Add New/Open static tasks and recent/pinned destinations using a supported safe abstraction consistent with `unsafe_code = forbid`.
- Honor shell-removed destinations; reject missing paths gracefully; avoid stale history; update after open/save.
- Test Windows 11 packaged and unpackaged installs and launches. Do not mark the menu complete from a Linux cross-build alone.

### T30.4 — macOS Dock menu

- Add the AppKit Dock menu integration seam and populate current/recent document actions and app-wide tasks.
- Verify bundle identity, application delegate ownership, menu selection routing, activation, and close/reopen behavior on macOS.

### T30.5 — Linux launchers

- Add static `.desktop` actions for New/Open using correct `Exec` field-code quoting.
- Integrate opened documents with the desktop's recent-document mechanism where supported.
- Verify GNOME and KDE; record Wayland/X11 and launcher-specific differences. Do not promise a dynamic right-click recent/pin menu where the shell does not expose one.

### T30.6 — UX and release evidence

- Ensure the same important actions remain in File/Help menus and are keyboard-accessible.
- Add installation/upgrade/uninstall tests for desktop entry changes and app identity.
- Record screenshots or manual receipts per supported OS and environment, including app closed/open, missing documents, removed/pinned entries and dirty-document flows.

## Acceptance

- No placeholder entry remains in Open Recent; recent and pinned semantics are distinct and persisted.
- Windows 11 right-click exposes stable tasks and valid recent destinations, honoring user removal/pinning.
- macOS Dock context menu and supported Linux launcher actions work with correct activation arguments.
- Missing or inaccessible paths do not crash; a launch request never silently discards dirty content.
- Common menu model has unit coverage; Windows/macOS/Linux native behavior has real platform receipts; docs accurately describe unsupported launchers.
- Workspace unsafe-code policy remains intact; no direct copy of the experiment implementation.

## Deferred follow-up

An optional notification-area/system-tray icon with similar actions is a separate task because click conventions, event-loop delivery and accessibility differ by platform.
