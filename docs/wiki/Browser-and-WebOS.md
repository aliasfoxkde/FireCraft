# Browser and WebOS

## Standalone browser application

FireCraft's web shell runs as WebAssembly and is independently usable over HTTPS. Browser file access and downloads go through the host UI. The deployed build is versioned separately from the latest source commit; see [Project Status](Project-Status) and [release notes](https://github.com/aliasfoxkde/FireCraft/tree/main/docs/releases).

## Embedding and host integration

The current host contract uses an iframe-friendly canvas and a `postMessage` bridge for document opening, command application, liveness checks and export. The canonical message schema and host requirements are in [WEBOS.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/WEBOS.md).

FireCraft is both a standalone product and an embeddable application component. WebOS is one integration target, not a runtime requirement. The longer-term versioned command/capability SDK and shared-library strategy are described in [WEBOS-PLATFORM-INTEGRATION.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/WEBOS-PLATFORM-INTEGRATION.md) and [RELEASE-ENGINEERING-PLAN.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/RELEASE-ENGINEERING-PLAN.md).

## Building locally

Use `cargo run -p xtask -- dist --web` to build the optimized bundle and run the bridge smoke. Keep `wasm32-unknown-unknown` dependencies browser-safe. Full commands and release evidence rules are in [TOOLCHAINS.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TOOLCHAINS.md) and [RELEASING.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/RELEASING.md).
