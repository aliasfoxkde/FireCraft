# FireCraft v1.0.0-rc External Tester Guide

> **Target:** External testers across Windows (`x86_64`), macOS (`aarch64` / universal), Linux (`x86_64` DEB / AppImage / Flatpak / tarball), and Browser/WASM.

## 1. Before You Begin

- Download the RC build for your platform from the GitHub Draft Release or staging URL and verify its SHA-256 against `SHA256SUMS`.
- **Clean-room & privacy:** Do not attach proprietary/confidential files to public bug reports.

## 2. Core Smoke Checklist (10 minutes)

1. **Launch & Shell:** App opens without white flash, menus (`File`, `Edit`, `View`, `Select`, `Modify`, `Text`, `Commands`, `Filters`, `Window`, `Help`) respond to mouse and `Alt+<key>`.
2. **New Document & Vector:** Create a document, draw a Rectangle + Ellipse + Pen path, apply fill/stroke, undo/redo (`Ctrl+Z` / `Ctrl+Y`).
3. **Bitmap & Text:** Paint with Brush, add a Text object, edit text, toggle layer visibility.
4. **Save / Reopen / Export:** Save as Fireworks PNG (`.png`), close tab, reopen `.png`, export PNG/JPEG/WebP.
5. **WASM (Browser):** Load the web build in Chromium/Firefox/Safari, confirm canvas renders, tools respond, and no console errors appear.

## 3. Reporting Issues

- Use **Help -> Send Feedback...** (or open a GitHub Issue using the Bug Report template).
- Include: OS/browser version, build SHA/version, exact reproduction steps, and expected vs. actual behavior.
