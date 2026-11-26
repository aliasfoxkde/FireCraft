# Fireworks Console benchmark

**Reviewed:** 2026-11-25. **Reference:** [fwextensions/fwconsole](https://github.com/fwextensions/fwconsole). Its README describes a Flex 3/ActionScript panel plus JavaScript bridge that exposes a Firebug-like `console` object to help debug Fireworks JavaScript extensions. The source requires a legacy Flex 3 compiler and external `trace`/JSON dependencies; no release is published on the repository page.

## Transferable product idea

The useful idea is an editor-integrated **script console for inspection and diagnostics**, close to the object being edited. FireCraft already has a scripting engine, `.jsf`/`.fwsteps` workflows, command palette and MCP server, but users may still benefit from a console that can:

- run a selected script or documented command against the current document;
- show structured success, error, warning and timing output;
- expose safe read-only document/selection inspection helpers;
- link an error to the relevant script line where available;
- keep a bounded, copyable session history and allow clear/reset;
- show which capabilities and filesystem/network permissions are available.

This is a better first step than embedding an unrestricted OS shell inside the editor. A system terminal is a separate product decision involving process lifecycle, working directory, command execution and permissions; keep it isolated from the document scripting console.

## FireCraft gap and plan

1. Inventory the existing in-app script runner, command palette, status messages, diagnostics and MCP behavior.
2. Design a small **Automation Console** around existing scripting/command boundaries, not an additional execution authority.
3. Define whether scripts can inspect selection, modify document, read/write files or access network; show these capabilities before execution.
4. Add cancellation, bounded output, clear error reporting and undo/history integration for mutations.
5. Consider a separate optional system-terminal panel only after the editor console and panel resource behavior are stable.

## Limits and clean-room note

The reference is a legacy Flex/ActionScript implementation for Adobe's JavaScript extension host. Its UI, implementation, bridge code and compatibility assumptions should not be copied. This review uses the high-level idea of placing script diagnostics beside the editing workflow. The repository page does not state a license in the reviewed README; do not reuse its code or assets.
