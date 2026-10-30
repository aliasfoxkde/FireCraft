# FireCraft — Feature surface

> Audited 2026-10-30 against the v0.2.3 tree. Every claim here is
> backed by a gate in the repository: the command registry
> (`engine::catalog::ids()`), the parity matrix
> (`docs/planning/generated/dom-parity.md`), the menus
> (`crates/app/src/menus.rs`), and the CLI (`crates/app-cli`).
> Where a feature is a *recipe* rather than a built artifact, it says
> so.

FireCraft is a clean-room reimplementation of Adobe Fireworks CS6 —
the hybrid vector + bitmap web-design tool Adobe discontinued in 2013
— in pure Rust. It reads and writes Fireworks PNG (`.fw.png`)
documents natively, and speaks the CS6 DOM verb surface through one
command registry that the GUI, the CLI, the journal, the ExtendScript
host, and the browser bridge all drive.

## Document format

| Area | State |
|---|---|
| `.fw.png` read/write | Complete — the `mkTS` node tree round-trips; `crates/fwpng`, `crates/doc/src/fwmap` |
| Chunks | `mkTS` (node tree), `mkBT` (tile rasters), `IEND`, plus unknown-chunk preservation |
| Tiles | 128×128 ARGB and 256×256 grayscale `mkBT` banks, `TIDi`/`TMCi` references |
| Unknown-node preservation | Every unrecognized node rides the save path byte-exactly |

## The command registry

One registry serves every surface. The live count is
`engine::catalog::ids()` — 243 as of 2026-10-29 (a point-in-time
number; the call is the authority). The CS6 DOM parity matrix stands
at **268 carried / 3 partial (evidence-gated) / 0 missing** across 356
verbs.

Surfaces that drive it:

- **GUI** — the shell; menus map CS6 menu ids to actions
  (`crates/app/src/menus.rs`).
- **CLI** (`firecraft-cli`) — every verb drivable headless.
- **Journal** — every mutation is a journal line; "Save Steps As
  Command" and replay are exact inverses by construction.
- **ExtendScript host** — the CS6 `.jsf` corpus runs headless
  (`crates/scripting`).
- **Browser bridge** — `postMessage` over the wasm bundle
  (`web/src/main.rs`); every mutation goes through the same registry.

## Editing surfaces

### Vector

Paths with contours and curve handles, the primitive source kept live
(rect, ellipse, line, polygon, star, spiral, smart polygon), stroke
and fill (solid, linear/radial/conical/satin/pinch gradients, pattern
fills), boolean path operations (union, intersect, punch, crop),
simplify, align and arrange, z-order.

### Bitmap

Bitmap objects on the canvas, the paint family (`bitmap.brush`,
`bitmap.pencil`, `bitmap.erase`, `bitmap.fill`), pixel selection
(marquee, lasso, polygonal marquee, magic wand with add/subtract/
intersect/replace modes), gradients, and the retouch tools.

### Text

Text objects with font, size, style, color, and alignment; text
styles; rich text attributes. The paragraph-level surface
(character markup, text-on-path, text rectangles) remains the largest
unexposed family.

### Symbols

Symbol definitions and instances, symbol types (graphic, button,
movie clip), nested editing, nine-slice guides with draggable
insets, symbol instance transforms.

### Layers, pages, states

Layers with opacity/blend/lock/visibility, pages with their own export
presets, animation states per layer with per-state fill.

### Effects and filters

Live effects (drop shadow, glow, bevel variants, emboss, inner
shadow) render in the compositor. Thirty filters are exposed through
the menus (blur, sharpen, color correction, noise, and the rest).

## Import / export

| Direction | Formats |
|---|---|
| Import (raster) | PNG, JPEG, GIF (static and animated), BMP, TIFF, WebP, WBMP |
| Import (Fireworks) | `.fw.png` (native), `.fw.stl` style libraries |
| Export (raster) | PNG, JPEG (baseline + progressive, matte + quality), BMP, TIFF, WBMP, WebP, GIF (animated) |
| Export (vector/other) | SVG (layered), PSD (layered, merged composite), GIF |
| Export (web) | `export-web` workflows: images, HTML, CSS, sprites, CSS properties, pages, states, layers, animated GIF |

Quality and matte settings resolve from the document's stored `EXPv`
block; explicit CLI flags win over stored values.

## Scripting

The CS6 ExtendScript dialect runs headless — the `.jsf` corpus from
the reference installation executes under `crates/scripting`, with
`Files.*` and `runScript` available. Script runs are sandboxed by
limits (statement count, time), and a failing script leaves the
document untouched.

## Applications

- **Web (wasm32)** — the primary release target. Live at
  <https://firecraft-web.pages.dev>; the same shell the desktop build
  runs, compiled to WebAssembly and driven over a `postMessage`
  bridge. See [`WEBOS.md`](WEBOS.md) for the embedding contract.
- **Desktop** — a native Linux/Windows/macOS shell (egui + wgpu).
- **CLI** — headless automation: inspect, convert, export, import,
  batch, replay, script, and an MCP server mode.

## Release artifacts

| Artifact | Built by | Verification |
|---|---|---|
| `dist/web/` bundle | `xtask dist --web` | wasm size budget + bridge smoke |
| Linux x86_64 tar.gz, `.deb`, CLI archive | `xtask dist --native` | sha256 manifest, `dpkg-deb` read-back, recipe parse-check |
| Linux AppImage | `packaging/build-appimage.sh` | operator-run recipe (needs linuxdeploy) |
| macOS universal `.dmg` | `packaging/build-macos-dmg.sh` | operator-run recipe (needs macOS + Xcode CLT) |
| Windows portable zip / Inno Setup installer | `packaging/build-windows.ps1` | operator-run recipe (needs Windows + MSVC) |

The repo never carries an artifact it did not build, and never claims
a platform is supported beyond what a gate exercised. Toolchain
requirements live in [`TOOLCHAINS.md`](TOOLCHAINS.md).

## Accessibility

The web host is a single egui canvas with a readable fallback for
screen readers. egui's accesskit integration is compiled in, but the
accessibility tree is **not** reachable on the web build (decision
0067) — the honest position is that the canvas is keyboard-operable
(F1 lists every binding) but not screen-reader-navigable. WCAG
conformance is audited criterion by criterion rather than claimed
blanket; see [`decisions/0067-*`](decisions/) and
[`NEXT.md`](NEXT.md) Phase 16.

## Licensing

Dual-licensed MIT OR Apache-2.0. FireCraft is independent and not
affiliated with Adobe Inc.; Adobe names appear only to describe
compatible workflows and file formats. Vendored material is tracked
in [`ATTRIBUTION.md`](ATTRIBUTION.md).
