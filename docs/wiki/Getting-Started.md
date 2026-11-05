# Getting Started

## Choose a way to use FireCraft

- **Browser:** open the [FireCraft web app](https://firecraft-web.pages.dev). Its deployed build can differ from the current source; check [Project Status](Project-Status) for the latest manifest comparison.
- **Desktop:** build the `app` workspace package on a supported native system. Platform build requirements are in [TOOLCHAINS.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TOOLCHAINS.md).
- **Command line:** build or install `firecraft-cli` to inspect, convert, import, export, batch-process, or script documents.

## First commands

```sh
cargo run -p xtask -- ci
cargo run -p firecraft-cli -- info document.fw.png
cargo run -p firecraft-cli -- convert input.fw.png output.fw.png
cargo run -p firecraft-cli -- export input.fw.png output.webp
```

These examples assume a Rust workspace checkout. See the full [Usage Guide](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/USAGE.md) for imports, GUI shortcuts, and release builds.

## Learn the editor

Continue to [Editor and Layout](Editor-and-Layout) for the canvas, tools, panels, and known layout gaps. For file support, see [Documents and Formats](Documents-and-Formats).
