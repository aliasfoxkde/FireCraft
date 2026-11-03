<h1 align="left"><img src="docs/img/logo.svg" alt="FireCraft logo" width="40" align="top" /> FireCraft</h1>

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

**v0.2.5 released (2026-11-03).** All roadmap milestones (M0–M15) are
closed and the post-roadmap CS6 DOM parity campaign is complete: the
document model with full `.fw.png` read/write fidelity, a
deterministic CPU compositor with golden-image gates, the vector and
bitmap tool surface (including the paint family and pixel selection),
text, symbols, scripting (`.jsf`), and import/export (SVG, PSD, GIF,
WebP, and the rest of the raster matrix). The command registry
carries **245 commands** (`engine::catalog::ids()` is the live
authority); the CS6 DOM parity matrix stands at **268 carried / 3
partial (evidence-gated) / 0 missing**.

### What works today

- **Documents**: full Fireworks PNG (`.fw.png`) read/write, PNG/JPEG/
  WebP/AVIF/GIF/TIFF/BMP import, SVG and PSD import, export to the
  full raster matrix plus SVG.
- **Canvas**: GPU-accelerated rendering with a deterministic CPU
  reference renderer gated by golden-image tests; vector shapes
  (rectangle, ellipse, polygon, line, pen), bitmap painting, pixel
  selection, text, symbols.
- **Editing**: transform handles with a Numeric Transform dialog,
  Subselect path-node editing (add/delete node), gradient fill editor
  with on-canvas handles, guides/grid with drag-out and snapping,
  duplicate/clone, context menu, command palette.
- **Shell**: CS6-style menubar and panels (Properties, Layers, History,
  Optimize, and the Window-menu panel toggles), Lucide tool icons,
  options bar with fill/stroke picker.
- **Automation**: 245-command registry (everything-is-a-command),
  `.jsf` scripting support, headless CLI verbs, `postMessage` browser
  embedding, and a full MCP server (`firecraft-cli mcp`) with
  document inspection, export, undo/redo, and registry-discovery
  tools for AI agents.

### Roadmap — daily-driver expansion (docs/NEXT.md, Phases 19–21)

1. **Reachability** (in flight): the engine binds 245 commands but the
   shell still leaves ~114 of 289 CS6 menu items inert; the burn-down
   (`menus::tests::inert_count_never_rises` is the gate) continues with
   page/frame verbs next.
2. **Daily-driver features**: live text editing on canvas, Optimize
   panel with export preview, animation timeline (frames/states), mask
   stack.
3. **Differentiators**: autosave/crash recovery, HiDPI polish.

### Next steps

- The quality ratchet runs in parallel: coverage toward 99%, lint
  hardening (`deny(missing_docs)` on geom/color is in), a WCAG 2.1 AAA
  audit, and release automation.
- See [`docs/NEXT.md`](docs/NEXT.md) for the full phased plan and
  [`docs/PROGRESS.md`](docs/PROGRESS.md) for the live status board.

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
