# Taskbar and launcher context menu research

**Reviewed:** 2026-11-25. This note concerns the app's taskbar/Dock launcher context menu, not a notification-area system tray icon.

## Platform findings

- **Windows:** Microsoft's `ICustomDestinationList` API builds the taskbar Jump List from destinations and static tasks. Recent/frequent/custom destinations are document-like entries; the user can pin or remove destinations. Static tasks are app-wide actions and are not pin-able/removable. When an app customizes the list, it owns repopulating it and must honor removed destinations and handle missing paths. See [ICustomDestinationList](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-icustomdestinationlist) and [Windows taskbar extensions](https://learn.microsoft.com/en-us/windows/win32/shell/taskbar-extensions).
- **macOS:** a Dock secondary-click menu is provided by AppKit's `applicationDockMenu(_:)` hook. Apple recommends concise, high-value commands and lists of open/recent windows/documents; the same actions should also remain available elsewhere. See [Apple's Dock menu guidance](https://developer.apple.com/design/human-interface-guidelines/dock-menus) and [AppKit Dock menu hook](https://developer.apple.com/documentation/AppKit/NSApplicationDelegate/applicationDockMenu(_:)).
- **Linux:** desktop launchers can expose static `Actions` groups in a `.desktop` file, but desktop environments vary in how they present them. Recent documents use desktop-environment integration and need testing across target shells. The [freedesktop Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry/latest-single/) explicitly allows implementations to ignore unsupported actions, so these cannot be treated as a universal dynamic menu equivalent to Windows/macOS.

## FireCraft baseline and experiment observation

The current FireCraft menu has a single inert `File → Open Recent → Recent File` row. `packaging/desktop/firecraft.desktop` has no launcher `Actions`. The current FireCraft shell source has no taskbar Jump List adapter. The separate experiment repo has a Windows-only `shell_windows.rs` implementation that sets an AppUserModelID, adds recent document destinations and New/About tasks. It uses a module-local `allow(unsafe_code)`, which conflicts with FireCraft's workspace policy (`unsafe_code = forbid`) and is not suitable for direct transfer. Its recent list is populated from in-memory file paths; audit persistence, stale entries and honoring user-removed destinations before considering its behavior.

## Proposed product model

Maintain recent and pinned document lists separately in a versioned, privacy-aware shell preference. Store normalized paths and display names, bound list size, and skip missing/unreadable files. Expose platform-neutral actions with stable launch arguments; each adapter maps them to native shell conventions. For Windows, let the OS own pin/remove state for Jump List destinations. For Linux, keep static desktop actions and recently used documents distinct. For macOS, use the Dock menu hook and include current documents where appropriate.

The taskbar feature must share the GUI's dirty-document confirmation and document-open path. A launcher request while FireCraft is already running should activate the existing instance or open a new tab according to the documented single-instance policy. Never silently discard dirty documents.

## System tray is separate

A tray icon is not the taskbar/Dock application icon. It has separate discoverability, lifecycle, event-loop, accessibility and platform-menu behavior. Keep tray support optional and independently scoped; do not substitute it for launcher menu integration.
