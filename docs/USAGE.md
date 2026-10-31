# FireCraft — Usage

> Verified against v0.2.3. CLI examples run as written; GUI claims
> come from the menus and shortcuts tables in the source
> (`crates/app/src/menus.rs`).

## Building

```sh
cargo run -p xtask -- ci        # the full gate: fmt, clippy, tests, layers, wasm
cargo build --release -p app    # the desktop shell
cargo build --release -p app-cli
cargo run -p xtask -- dist --web     # the browser bundle under dist/web/
cargo run -p xtask -- dist --native  # Linux tar.gz + deb + CLI archives
```

Release artifacts and platform recipes: [`TOOLCHAINS.md`](TOOLCHAINS.md).

## CLI (`firecraft-cli`)

Every verb operates on `.fw.png` documents (or any raster, for
`import`):

```sh
firecraft-cli info doc.fw.png
firecraft-cli convert in.fw.png out.fw.png       # canonical re-emit
firecraft-cli apply doc.fw.patch.steps --out next.fw.png
firecraft-cli export in.fw.png out.png            # png|jpg|bmp|tiff|wbmp|webp|svg|psd|gif
firecraft-cli export in.fw.png out.jpg --quality 82 --progressive --matte white
firecraft-cli import photo.jpg doc.fw.png "Photo" # any raster -> a fresh .fw.png
firecraft-cli batch ./pages --format webp --out ./exports
firecraft-cli export-web in.fw.png ./site --workflow css --page 0
firecraft-cli run-script export-all.jsf doc.fw.png --out done.fw.png
firecraft-cli serve doc.fw.png                    # JSON-lines command server
firecraft-cli mcp doc.fw.png                      # MCP framing over the same registry
```

Notes:

- `export` reads the document's stored `EXPv` block first; explicit
  flags win over stored settings.
- Animated GIFs import as states (one state per frame); single-frame
  GIFs import as plain rasters.
- `batch` stamps only the option fields you provide, so per-file
  stored settings survive where the batch leaves `None`.
- Every mutation goes through the engine registry — the CLI and the
  GUI are the same surface.

## The GUI

### Documents

`Ctrl+N` new · `Ctrl+O` open · `Ctrl+S` save ·
`Ctrl+Shift+S` save as · `Ctrl+Shift+R` export ·
`Ctrl+Shift+X` export preview. Open accepts `.fw.png` documents and
raster images (a raster opens as a fresh document).

### Editing

`Ctrl+Z` / `Ctrl+Shift+Z` undo/redo · `Ctrl+X`/`C`/`V` cut/copy/paste ·
`Delete` or `Backspace` clears the selection · `Ctrl+A` select all ·
`Ctrl+D` deselect · `Ctrl+G`/`Ctrl+U` group/ungroup ·
`Ctrl+Alt+E` merge down · `Ctrl+J` join paths (union).

### Tools (one-key selects)

| Key | Tool | Key | Tool |
|---|---|---|---|
| `V` | Pointer | `B` | Paintbrush |
| `R` | Rectangle | `N` | Pencil |
| `E` | Ellipse | `Y` | Eraser |
| `O` | Line | `G` | Paint bucket |
| `T` | Text | `D` | Gradient |
| `Z` | Zoom | `I` | Eyedropper |
| `H` | Hand | `M` | Marquee |
| `L` | Lasso | `P` | Polygonal marquee |
| `W` | Magic wand | | |

Pixel-selection tools compose with modifiers (add/subtract/intersect)
while dragging.

### View

`Ctrl+=` zoom in · `Ctrl+-` zoom out · `Ctrl+Alt+0` fit all. The
View menu carries the full zoom ladder (6%–6400%).

### Arrange and text

`Ctrl+F` bring to front · `Ctrl+Shift+F` forward ·
`Ctrl+Shift+B` backward · `Ctrl+B` send to back ·
`Ctrl+Alt+Shift+B/I/U` bold/italic/underline.

### Help

`F1` opens the keyboard-shortcuts panel — the live binding map, from
the same table the dispatcher runs. Plain keys don't fire while a
text field has focus.

## The browser build

The wasm bundle at <https://firecraft-web.pages.dev> is the same
shell. Serve `dist/web/` over HTTP(S) (`file://` will not work —
wasm needs `application/wasm` streaming). Embedding contract, iframe
sizing, and the full `postMessage` API (`openDocument`, `apply`,
`exportPng`, `exportDocument`, `ping`): [`WEBOS.md`](WEBOS.md).

```js
const frame = document.querySelector('iframe');
frame.contentWindow.postMessage(
    { type: 'apply', name: 'object.createPath', payload: { /* ... */ } },
    '*',
);
```

## Further reading

- [`FEATURES.md`](FEATURES.md) — the full feature surface and its gates
- [`WEBOS.md`](WEBOS.md) — the embedding contract
- [`TOOLCHAINS.md`](TOOLCHAINS.md) — release artifacts and platforms
- [`PLANNING.md`](PLANNING.md) — the master plan and the format spec
