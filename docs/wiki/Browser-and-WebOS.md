# Browser and WebOS

## Browser app

FireCraft's web shell runs as WebAssembly. Open the [hosted web app](https://firecraft-web.pages.dev) over HTTPS; browser file access and downloads go through the host UI. The deployed artifact can differ from the latest source build, so check [Project Status](Project-Status) before relying on its version.

## Embedding

The host uses an iframe-friendly canvas and a `postMessage` bridge for opening documents, applying commands, and exporting results. The authoritative message schema and host requirements are in [WEBOS.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/WEBOS.md).

## Building locally

Use the repository's `xtask dist --web` workflow to build the optimized bundle and run the bridge smoke. Keep `wasm32-unknown-unknown` dependencies browser-safe. Full commands and the release recipe are in [TOOLCHAINS.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TOOLCHAINS.md) and [RELEASING.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/RELEASING.md).
