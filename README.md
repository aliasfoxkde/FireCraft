<h1 align="left"><img src="docs/img/logo.svg" alt="FireCraft logo" width="40" align="top" /> FireCraft</h1>

FireCraft is a clean-room, open-source reimplementation of Adobe Fireworks CS6 in Rust. It edits native `.fw.png` documents and runs as a browser app, desktop app, or headless CLI.

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:06ef3fb450bd08a6a2b6b281f3bff32256e992572e6459415ee63c40a68bbef2 -->

> FireCraft is not affiliated with, sponsored by, or endorsed by Adobe Inc. Adobe and Fireworks are trademarks of Adobe Inc.; names are used only to describe compatible workflows and formats.

## Project status

The latest tagged release is **v0.2.5**. Work on `main` has continued since that tag. The editor has 245 registered commands and 8 of 289 menu rows still report an unsupported or evidence-gated action. The current UI priorities are completing CS6 shell fidelity and improving panel behavior at normal browser sizes. A recent optimized web distribution and browser smoke are recorded in the [status page](docs/PROGRESS.md); the public Pages manifest currently differs from that local build, and generated artifacts are ignored by Git.

The [app screenshot](docs/img/app-main.png) is a 1280×800 capture of the current shell with a vector object selected, taken from the optimized web bundle built on 2026-11-04. It is evidence for that build and state, not a general parity claim — see the [screenshot audit](docs/tasks/T22-screenshot-ui.md) for the captures, measurements, and their limits.

## Start here

- **Try the app:** [usage guide](docs/USAGE.md)
- **See what works and what is incomplete:** [feature map](docs/FEATURES.md) · [current gaps](docs/GAPS.md)
- **Browse all project docs:** [documentation index](docs/README.md)
- **Current work and next tasks:** [task board](docs/TASKS.md)
- **Contributing or handing off work:** [agent handoff](docs/HANDOFF.md) · [repository rules](AGENTS.md)

## Quick commands

```sh
cargo run -p xtask -- ci
cargo run -p app-cli -- info document.fw.png
cargo run -p app-cli -- convert input.fw.png output.fw.png
```

<!-- /coderabbit-ai-generated:v1 -->

See [USAGE.md](docs/USAGE.md) for CLI, GUI shortcuts, and browser embedding. See [RELEASING.md](docs/RELEASING.md) for artifact and deployment procedures.

## License

Dual-licensed under MIT or Apache-2.0. The full terms are in [LICENSE](LICENSE).
