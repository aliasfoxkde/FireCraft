# FireCraft

<p align="center">
  <img src="docs/img/logo.svg" alt="FireCraft logo" width="128"/>
</p>

An open-source, clean-room reimplementation of Adobe Fireworks in pure Rust.

FireCraft aims to replicate Adobe Fireworks CS6 — the beloved hybrid vector +
bitmap web-design tool Adobe discontinued in 2013 — as fast, modern,
memory-safe Rust: native 64-bit memory handling, GPU-accelerated canvas with a
deterministic CPU reference renderer, modern codec support (WebP, AVIF, HEIC
via OS decoders), and first-class Fireworks PNG (`.fw.png`) document support
with full read/write fidelity. A WebAssembly build is a primary release
target, alongside native desktop and headless CLI releases.

<p align="center">
  <a href="https://firecraft-web.pages.dev"><img src="docs/img/app-main.png" alt="FireCraft running in the desktop shell — the demo document with a gradient brand band, vector shapes, and type" width="880"/></a>
</p>
<p align="center"><em>FireCraft running the demo document
(<code>docs/img/firecraft-demo.fw.png</code>, regenerable with
<code>cargo run -p xtask -- hero-gen</code>) — gradient fills, vector
shapes, and live type over the CS6-style graphite chrome.</em></p>

> **Trademark notice:** Adobe, Fireworks, Photoshop, Illustrator, and
> Macromedia are trademarks or registered trademarks of Adobe Inc. FireCraft
> is an independent, open-source project and is not affiliated with,
> sponsored by, or endorsed by Adobe Inc.; these names are used only to
> describe the workflows and file formats FireCraft is compatible with.

## Status

**v0.2.4 released (2026-11-02).** All roadmap milestones (M0–M15) are
closed and the post-roadmap CS6 DOM parity campaign is complete: the
document model with full `.fw.png` read/write fidelity, a
deterministic CPU compositor with golden-image gates, the vector and
bitmap tool surface (including the paint family and pixel selection),
text, symbols, scripting (`.jsf`), and import/export (SVG, PSD, GIF,
WebP, and the rest of the raster matrix). The command registry
carries **245 commands** (`engine::catalog::ids()` is the live
authority); the CS6 DOM parity matrix stands at **268 carried / 3
partial (evidence-gated) / 0 missing**.

The WebAssembly build is live at <https://firecraft-web.pages.dev>
(open, draw, and save documents in the browser), alongside desktop
and CLI releases on the [releases page](https://github.com/aliasfoxkde/FireCraft/releases).
Active work: the quality ratchet — coverage toward 99%, lint
hardening, a WCAG 2.1 audit, release automation (see
[`docs/NEXT.md`](docs/NEXT.md)).

## Using FireCraft

```sh
cargo run -p xtask -- ci                      # the full gate
cargo run -p firecraft-cli -- info doc.fw.png # inspect a document
cargo run -p firecraft-cli -- convert in.fw.png out.fw.png
cargo run -p firecraft-cli -- export in.fw.png out.webp
cargo run -p firecraft-cli -- import photo.jpg doc.fw.png
```

## Documentation

| Doc | What it covers |
|---|---|
| [`docs/FEATURES.md`](docs/FEATURES.md) | The feature surface, backed by gates |
| [`docs/USAGE.md`](docs/USAGE.md) | CLI verbs, GUI keys, embedding pointer |
| [`docs/WEBOS.md`](docs/WEBOS.md) | The browser embedding contract (`postMessage`) |
| [`docs/TOOLCHAINS.md`](docs/TOOLCHAINS.md) | Release artifacts and platform recipes |
| [`docs/PLANNING.md`](docs/PLANNING.md) | Master plan, format spec (§6), architecture (§7) |
| [`docs/HANDOFF.md`](docs/HANDOFF.md) | Agent handoff: state, phased plan, runbooks |
| [`docs/PROGRESS.md`](docs/PROGRESS.md) · [`docs/NEXT.md`](docs/NEXT.md) | Live status board · active backlog |
| [`docs/ATTRIBUTION.md`](docs/ATTRIBUTION.md) | Vendored-material attribution ledger |

## License

Dual-licensed under either of:

Dual-licensed (MIT OR Apache-2.0); the full texts of both live in
[LICENSE](LICENSE) — use this project under either, at your option.
