# Troubleshooting

## The browser app does not load

- Serve the bundle over HTTP or HTTPS; `file://` does not provide the wasm MIME and fetch behavior expected by the app.
- Confirm that the server provides `application/wasm` and that the JavaScript and wasm files named by `index.html` are present.
- Check [Project Status](Project-Status) before assuming the hosted app matches the current source.

## A command or menu item is unavailable

Some registry commands are not exposed through a polished menu or panel yet. The [Parity Roadmap](Parity-Roadmap) describes the known UI and feature backlog; the CLI and command palette may expose operations earlier than a dedicated panel.

## A file will not open or export

Check the CLI error for the detected format and unsupported feature. Use [Documents and Formats](Documents-and-Formats), the [feature matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/FEATURES.md), and [release status](Project-Status) to distinguish a format limit from a stale hosted build.

## Build or test failures

Use the toolchain notes in [`TOOLCHAINS.md`](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TOOLCHAINS.md), recreate the `target/` symlink if missing, and follow the exact `xtask` command from [`AGENTS.md`](https://github.com/aliasfoxkde/FireCraft/blob/main/AGENTS.md). Avoid running concurrent workspace builds into the shared target directory.
