# Getting Started

## Choose a way to use FireCraft

- **Browser:** open the [FireCraft web app](https://firecraft-web.pages.dev). The latest published release is v0.2.5; current `main` may contain newer work not yet published.
- **Desktop:** the current published native release includes Linux x86_64 desktop and CLI archives plus a Debian package. Other packaging scripts are not proof of a published, tested binary. Check [Release Status](Project-Status) before selecting a platform.
- **Command line:** build or install `firecraft-cli` to inspect, convert, import, export, batch-process or script documents.

## First commands

```sh
cargo run -p xtask -- ci
cargo run -p firecraft-cli -- info document.fw.png
cargo run -p firecraft-cli -- convert input.fw.png output.fw.png
cargo run -p firecraft-cli -- export input.fw.png output.webp
```

These examples assume a Rust workspace checkout. See the full [Usage Guide](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/USAGE.md) for imports, GUI shortcuts and release builds.

## Learn the editor

Continue to [Editor and Layout](Editor-and-Layout) for the canvas, tools, panels and current parity gaps. For file support, see [Documents and Formats](Documents-and-Formats); for CLI/MCP and scripting, see [Automation](Automation).
