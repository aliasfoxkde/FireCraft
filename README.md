# FireCraft

An open-source, clean-room reimplementation of Adobe Fireworks in pure Rust.

FireCraft aims to replicate Adobe Fireworks CS6 — the beloved hybrid vector +
bitmap web-design tool Adobe discontinued in 2013 — as fast, modern,
memory-safe Rust: native 64-bit memory handling, GPU-accelerated canvas with a
deterministic CPU reference renderer, modern codec support (WebP, AVIF, HEIC
via OS decoders), and first-class Fireworks PNG (`.fw.png`) document support
with full read/write fidelity. A WebAssembly build is a primary release
target, alongside native desktop and headless CLI releases.

> **Trademark notice:** Adobe, Fireworks, Photoshop, Illustrator, and
> Macromedia are trademarks or registered trademarks of Adobe Inc. FireCraft
> is an independent, open-source project and is not affiliated with,
> sponsored by, or endorsed by Adobe Inc.; these names are used only to
> describe the workflows and file formats FireCraft is compatible with.

## Status

**v0.2.1 released.** All roadmap milestones (M0–M15) are closed and
the post-roadmap CS6 DOM parity campaign is complete: the document
model with full `.fw.png` read/write fidelity, a deterministic CPU
compositor with golden-image gates, the vector and bitmap tool
surface, text, symbols, scripting (`.jsf`), and import/export (SVG,
PSD, GIF, WebP, and the rest of the raster matrix). The command
registry carries **241 commands**; the CS6 DOM parity matrix stands
at **268 carried / 3 partial (evidence-gated) / 0 missing**.

The WebAssembly build is live at <https://firecraft-web.pages.dev>
(open, draw, and save documents in the browser), alongside desktop
and CLI releases on the [releases page](https://github.com/aliasfoxkde/FireCraft/releases).
Active work: image-support enhancements (lossy WebP, AVIF, HEIC
decisions) and WebOS packaging.

```sh
cargo run -p xtask -- ci                      # the full gate
cargo run -p firecraft-cli -- info doc.fw.png # inspect a document
cargo run -p firecraft-cli -- convert in.fw.png out.fw.png
cargo run -p firecraft-cli -- export in.fw.png out.webp
cargo run -p firecraft-cli -- import photo.jpg doc.fw.png
```

- Master plan: [`docs/PLANNING.md`](docs/PLANNING.md)
- Roadmap and milestone tickets: [`docs/planning/firecraft/ROADMAP.md`](docs/planning/firecraft/ROADMAP.md)
- Live status board: [`docs/PROGRESS.md`](docs/PROGRESS.md) · [`docs/HANDOFF.md`](docs/HANDOFF.md)
- Attribution & licensing of vendored material: [`docs/ATTRIBUTION.md`](docs/ATTRIBUTION.md)

## License

Dual-licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
