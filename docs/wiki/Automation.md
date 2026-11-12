# Automation

FireCraft's GUI, CLI, scripting host, journal, MCP server and browser bridge are designed to use the same engine command service. This makes repeatable edits, batch export, document inspection, replay and agent-driven workflows possible without implementing a second document mutation model.

## Command coverage

The engine currently registers **253 command IDs**. The latest shell-reachability measurement is **164/253 (64.82%)** in the CS6-visible UI. The command palette exposes the full registry; the reachability metric deliberately measures the designed shell surface, not the palette. DOM/API parity is tracked separately at 268 carried / 3 partial / 0 missing of 356 verbs.

These figures must not be described as complete GUI parity. The [CS6 user-visible matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md) records menus, tools, panels, inspector fields, workflows and evidence-gated gaps.

## Command line and scripts

The native `firecraft-cli` supports document inspection and conversion, imports/exports, command application, batch operations, journal/step replay, and JSON-lines control. The scripting host runs a supported subset of Fireworks `.jsf` automation; the app also replays `.fwsteps` journals. See [USAGE.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/USAGE.md), the [script corpus report](https://github.com/aliasfoxkde/FireCraft/blob/main/corpus/scripts-report.json), and current decisions for exact limits.

## MCP and browser bridge

The native MCP server exposes document inspection, command application, undo/redo, export and command discovery. The browser host uses a documented `postMessage` bridge. The same command semantics should be preserved across GUI, CLI, MCP, and the future WebOS SDK adapter; browser apps use a host bridge, not an in-browser TCP listener.

- [Browser/WebOS protocol](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/WEBOS.md)
- [FireCraft platform integration and SDK direction](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/WEBOS-PLATFORM-INTEGRATION.md)
- [Automation usage guide](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/USAGE.md)

Automation coverage does not imply that every operation has an equivalent polished menu, panel or inspector workflow.
