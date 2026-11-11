# Automation

FireCraft's CLI, script host, GUI, and browser bridge share the command registry. Automation is useful for repeatable edits, batch export, document inspection, and agent-driven workflows.

## Command line

`firecraft-cli` can inspect and convert documents, apply command steps, import/export, batch-process folders, replay scripts, and serve a JSON-lines or MCP interface. See [USAGE.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/USAGE.md) for the complete command list and argument shapes.

## ExtendScript and steps

The script host runs a supported subset of Fireworks `.jsf` automation in a sandbox. Its exact behavior and gaps are recorded in the [script report](https://github.com/aliasfoxkde/FireCraft/blob/main/corpus/scripts-report.json) and [decision records](https://github.com/aliasfoxkde/FireCraft/tree/main/docs/decisions).

## MCP and browser bridge

The CLI MCP server exposes document inspection, command application, undo/redo, export, and command discovery. The browser host accepts the documented `postMessage` protocol. See [WEBOS.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/WEBOS.md) for message details.

Automation coverage does not imply that every operation has an equivalent polished GUI control. The [Parity Roadmap](Parity-Roadmap) tracks the shell work separately.
