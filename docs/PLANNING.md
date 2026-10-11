# FireCraft — Planning

> **Status:** Planning complete (2026-10-10). This is the master planning document.
> The executable roadmap lives in [`docs/planning/firecraft/ROADMAP.md`](planning/firecraft/ROADMAP.md);
> milestone tickets sit next to it. Research findings this plan is built on are
> summarized in §12 with sources.

---

## 1. Original brainstorm (2026-10-10)

Preserved verbatim from the first revision of this file:

> FireCraft, a clean-room reimplementation of Adobe Fireworks in pure Rust.
>
> Look at the following GitHub account and repositories: https://github.com/storytold
> They have built several adobe products creating a "clean-room reimplementation of Adobe {X} in pure Rust".
> I would like to do the same for a no longer support Adobe product called Fireworks (my favorite app of all time but the old version I have is showing it's age), but I am also including the application itself, I would like you to decompile it and use the references (StoryTold / ArtCraft), the source code and Ghidra (a reverse engineering framework) to build a failful replica of the application with better performance, modern standards (can handle memory better), improved support for HVEC, WebP, and other formats, and so on.
>
> It's probably best to audit the documentation and AGENTS.md for various {app}craft projects to understand how they are structured, and you can likely borrow a good chunk of the code, since they are all based on Adobe Products, see for reference examples:
> https://github.com/storytold/photocraft/blob/main/AGENTS.md
> https://github.com/storytold/photocraft/blob/main/ATTRIBUTION.md
>
> Ghidra is a software reverse engineering (SRE) framework created and maintained by the National Security Agency Research Directorate. This framework includes a suite of full-featured, high-end software analysis tools that enable users to analyze compiled code on a variety of platforms including Windows, macOS, and Linux:
> https://github.com/NationalSecurityAgency/ghidra
>
> I am largely interested in a WASM build/release, so that I can include it with my WebOS, but it would be useful to include a wide range of releases, much like done by ArtCraft and their various projects:
> https://github.com/storytold/photocraft/releases
>
> See application code reference:
> * Full install ./source/Adobe Fireworks CS6/
> * Portable Version: ./source/FireworksPortable

---

## 2. Mission

FireCraft is a clean-room reimplementation of **Adobe Fireworks CS6 (v12.0)** in
pure Rust: a hybrid vector + bitmap design editor with pages/states/layers,
live filters, slices and web export, and scripting — with the Fireworks PNG
(`.fw.png`) document format as a first-class citizen, read **and** written with
full fidelity.

Success looks like: a designer's existing `.fw.png` files open in FireCraft
with layers, vectors, text, filters and symbols intact, render identically to
CS6's output, and save back out without loss.

**Delivery targets (in priority order):**

1. **WebAssembly first.** The user's primary interest is a WASM build that
   embeds in their **WebOS**: a client-side-only static bundle (no backend),
   iframe/postMessage-friendly, running on WebGPU with a WebGL2 fallback.
   The wasm target is a **first-class gate from M0** — photocraft's "never
   break wasm" rule — not a later port.
2. **Wide release matrix** like photocraft's: Linux (tar/AppImage/deb),
   macOS (universal dmg), Windows (installer + portable zip), **web (static
   wasm bundle)**, plus a headless CLI for servers/CI.

**Modernizations over CS6:** native 64-bit memory handling; GPU-accelerated
canvas with a deterministic CPU reference renderer; modern codec support
(WebP, AVIF, JPEG XL decode, HEIC via OS decoders); true cross-platform with
HiDPI done right; scriptable headless (CLI + control channel) for batch and
CI; runs in the browser.

**Parity target:** Fireworks CS6 12.0.1 behavior. Where CS6 and modern
expectations conflict (e.g. Flash-era exports), modern wins — see non-goals.

---

## 3. Ground truth: the product being replicated

Researched facts (sources in §12), condensed:

- **Lineage:** Macromedia Fireworks 1.0 (1998) → MX 2004 (v7) → v8 (2005) →
  CS3 (v9) → CS4 (v10) → CS5 (v11) → **CS6 = v12.0, GA 2012-05-07**, final
  patch 12.0.1 (2013-06-17). Discontinued 2013-05-06; Adobe EOL matrix: core
  support ended 2013-05-07, extended 2015-05-07. Our local reference binary
  is `12.0.0.236` (CS6 RTM).
- **Document model:** Pages → States (frames) → Layers → Objects. Per-page
  canvas size/color; Web Layer always topmost; shared layers across
  states/pages; master page; state delays + onion skinning.
- **Vector:** pen/path tools, freeform & reshape, path ops (union, intersect,
  punch, crop, simplify, expand stroke, inset), **13 Auto Shape tools** and a
  library of ~27 scripted Auto Shapes — all implemented as `.jsf` scripts
  driving `smartShape` control points (the scripts ship in our local tree and
  double as behavioral specs).
- **Bitmap:** marquee/lasso/wand selections, retouch tools, **live filters**
  (levels/curves/hue-sat/blur family/sharpen/shadow & glow/bevel/emboss/noise/
  find edges/convert-to-alpha + Photoshop Live Effects), **27 blend modes**,
  vector and bitmap masks.
- **Text:** Adobe Text Engine, auto-size vs fixed-width blocks, kerning/
  tracking/leading/baseline shift, anti-alias presets, **text on a path**.
  No paragraph/character styles (object styles only) — parity means parity.
- **Symbols:** graphic / button / animation symbols; rich ("component")
  symbols scripted by `.jsf` (`setDefaultValues()` / `applyCurrentValues()`);
  document library with import/export; **9-slice scaling guides**.
- **Web layer:** slices (rect/polygon/HTML), hotspots, image maps, behaviors
  (simple rollover, swap image, nav bar, pop-up menus), slice auto-naming,
  export modes (Images Only, HTML+Images, CSS+Images, **CSS Sprites**,
  Pages/States/Layers To Files), image optimization (palettes, dithering,
  selective JPEG), 2-Up/4-Up preview.
- **Scripting:** embedded **SpiderMonkey "JavaScript-C 1.5 pre-release 3"**
  (ES3); globals `fw` + `fw.getDocumentDOM()` (**117 `fw.*` functions, 342
  `dom.*` methods** — counts extracted from the official "Extending
  Fireworks" PDF); `.jsf` command files; Flash(SWF) command panels via
  `MMExecute`; History-panel step replay ("Save Steps As Command"); batch
  processing that itself saves `.jsf` scripts; a loopback **RPC server on
  TCP 127.0.0.1:12124** (custom null-terminated UTF-8 XML fragments).
- **Formats:** native = Fireworks PNG (§6). In: PNG, PSD, AI, EPS(flattened),
  JPEG, GIF (incl. animated), WBMP, HTML(first table), clipboard. Direct
  save: PNG, GIF, JPEG, BMP, WBMP, TIFF, SWF, AI, PSD. Export adds: PDF, FXG,
  CSS+Images, CSS Sprites, MXML (deprecated). **No native SVG either way**
  (community extensions only) — FireCraft will do SVG natively as a
  modernization.

---

## 4. Clean-room policy (non-negotiable)

Following the photocraft/ArtCraft method (their AGENTS.md §3):

1. **FireCraft is written from public specifications, observed behavior, and
   original engineering.** Never from Adobe source code, decompiled code,
   disassembly pasted into code, shaders, or assets. Ghidra/decompilation is
   used to *understand* behavior and file formats; findings are recorded as
   notes and specs, then implemented fresh.
2. **No Adobe-copyrighted bytes ever enter the repo.** `source/` (the local
   CS6 install) is git-ignored forever (`.gitignore`). This includes
   "just a small icon", fonts, patterns, textures, symbol libraries, sounds,
   and the SpiderMonkey source copies shipped inside CS6.
3. **Adobe-derived extracts (API name lists, exe strings, help-PDF text) stay
   out of git** — they live uncommitted under `/var/tmp/` (see §5.4) with
   regeneration recipes recorded here.
4. **Third-party material must be permissively licensed, attributed in
   [`docs/ATTRIBUTION.md`](ATTRIBUTION.md) in the same change it lands, with
   its license file adjacent.** Vendored research:
   `docs/research/fireworks-png-nodes.md` (MIT, from davidje13/png-analyser).
5. **Assets (icons, fonts, demo images, symbol libraries) are original or
   permissively licensed** — the Common Library is re-created from scratch,
   never copied. Fonts live in a separate optional repo (photocraft's
   `craft-fonts` pattern), never committed.
6. **Naming/trademark:** product name is **FireCraft** (distinct). README
   carries the Adobe non-affiliation disclaimer: "Adobe and Fireworks are
   trademarks of Adobe Inc.; FireCraft is an independent open-source project,
   not affiliated with, sponsored by or endorsed by Adobe Inc.; these names
   are used only to describe compatibility."
7. **License:** the repo currently ships Apache-2.0; M0 adds **`MIT OR
   Apache-2.0`** dual-licensing to match the wider Rust ecosystem (photocraft
   convention), plus `NOTICE` and the trademark disclaimer.

The AMT/activation layer of the reference install is **out of scope
entirely** — never analyzed as part of feature work, never reimplemented.

---

## 5. Reference base

### 5.1 Local, untracked (`source/` — never commit)

| Path | What it is | Use for FireCraft |
|---|---|---|
| `source/Adobe Fireworks CS6/` | Full CS6 install (v12.0.0.236), ~350 MB | Binary/behavioral reference; corpus generation host |
| `…/Configuration/Auto Shapes/`, `Auto Shape Tools/` | **89 `.jsf` scripts total in tree** — complete Auto Shape implementations in ES3 JS, with control-point behavior documented in comments | Primary behavioral spec for Auto Shapes — port logic, rewrite as Rust + our scripting layer; **do not copy the files** |
| `…/Configuration/Commands/` (Web, Text, Document, Selection, Creative, jQuery Mobile Theme…) | Shipped `.jsf` commands | Behavioral spec for command surface + scripting API usage patterns |
| `…/Configuration/HTML Code/*.htt`, `SLICES.XTT`, `mm_menu.js` | HTML export templates + pop-up menu runtime | Spec for HTML/CSS export behavior (re-implement, don't copy) |
| `…/Configuration/Third Party Source Code/JavaScript Interpreter/` | SpiderMonkey 1.5 sources as `.txt` (MPL, 2001) | Confirms exact JS engine (ES3 + quirks); **do not copy into repo** |
| `…/Plug-Ins/*.x32` | BMP/Targa/TIFF/WBMP/Palette/TWAIN format handlers | Confirms format support matrix |
| `…/Configuration/Libraries/*.png` | 5 Common Library files (Animations, Bullets, Buttons, Halo Lite, Themes) | Reference for what symbol libraries *contain*; re-created as original assets |
| `…/FireworksPortable/` | PortableApps launcher | Runs under Wine → **corpus generation harness** |

### 5.2 External documentation (all live/archived URLs verified 2026-10-10)

- **"Extending Fireworks" (JS API, CS5/CS6 gen, 333 pp) — still live:**
  https://help.adobe.com/archive/en_US/fireworks/cs5/fireworks_cs5_extending.pdf
  (CS4 variant: `…/cs4/fireworks_cs4_extending.pdf`). Cached:
  `/var/tmp/fw_cs5_extending.pdf`, extracted text `/var/tmp/fw_ext_full.txt`.
- **CS6 help / reference manual (289 pp), archived:**
  https://web.archive.org/web/20140502065935id_/http://helpx.adobe.com/pdf/fireworks_reference.pdf
  (cached `/var/tmp/fw_cs6_help.pdf` + `.txt`).
- **Fireworks PNG format:** Stack Overflow byte-level write-up
  https://stackoverflow.com/questions/4242402 (answer 51683285 + 2023 comments);
  **davidje13/png-analyser** `docs/Fireworks.md` + `docs/FireworksNodes.md`
  (MIT) https://github.com/davidje13/png-analyser — the only serious public
  format parser; pngcheck chunk naming
  https://github.com/pnggroup/pngcheck; format wiki
  https://fileformats.archiveteam.org/wiki/Fireworks_PNG.
- **JS quirks catalog:** https://github.com/fwextensions/fwjs-errata ;
  community extension ecosystem: johndunning.com/fireworks,
  fwextensions/fwlib (GitHub), senocular (GitHub).
- Devnet article archive (746 URLs harvested): `/var/tmp/fw_devnet.txt`
  (uncommitted; regenerate via Wayback CDX).
- **Release engineering reference:** https://github.com/storytold/photocraft/releases
  (per-OS desktop bundles + web build — the matrix FireCraft mirrors).

### 5.3 Vendored research (committed, attributed)

- `docs/research/fireworks-png-nodes.md` — ~500 FourCC node dictionary
  (MIT © davidje13, png-analyser). The format Rosetta stone. See
  [`docs/ATTRIBUTION.md`](ATTRIBUTION.md).

### 5.4 Uncommitted local caches (`/var/tmp`, 30-day aging — regenerate as needed)

| File | Content | Regeneration |
|---|---|---|
| `fw_cs6_help.pdf/.txt` | CS6 manual | Wayback URL in §5.2 |
| `fw_cs5_extending.pdf`, `fw_ext_full.txt` | JS API reference | Live URL in §5.2 |
| `fw_api_strings.txt` | API FourCC/function strings from `Fireworks.exe` | `strings` over local exe |
| `fw_funcs.txt`, `dom_funcs.txt` | 117 `fw.*` / 342 `dom.*` API inventory | Extract from the Extending PDF |
| `fw_devnet.txt` | Devnet URL archive | Wayback CDX query |
| `FireworksNodes.md` | — vendored to repo — | github.com/davidje13/png-analyser |

**Rule:** Adobe-derived text extracts are working notes, never committed.

### 5.5 Tooling on this host

- **Ghidra** (Linux-native, Java) for PE analysis of `Fireworks.exe` — target
  functions of interest: the PNG writer (`CMkbStructureOutputter`, `CDbText`,
  `CWriteDictInfo` RTTI classes), chunk serializers, filter/effect pipeline.
- **Wine + the portable CS6 install** as a **corpus oracle**: run scripted
  `.jsf` batch scripts in real Fireworks to author deterministic `.fw.png`
  documents + their flattened exports for golden-image and round-trip tests.
- **png-analyser web app** (https://davidje13.github.io/png-analyser/) as a
  differential-testing reference for our parser.
- **REA** (https://github.com/morluto/rea, MIT) — optional accelerator, not a
  dependency: an MCP server/CLI that lets an agent drive Ghidra/Hopper/IDA
  (pseudocode, strings, symbols) plus ELF/Electron/APK analysis. Useful for
  the §5.5 Ghidra passes over `Fireworks.exe` (e.g. the
  `CMkbStructureOutputter` writer path) from inside an agent session. Clean
  room unchanged: it extracts *evidence*; nothing it emits is ever pasted
  into FireCraft code, and it never touches `source/` placement or the
  no-committed-derivatives rule.

---

## 6. The native format: Fireworks PNG

An ordinary PNG plus **five private ancillary chunk types** (big-endian
throughout; the flattened fallback image lives in the normal IDAT):

| Chunk | Count | Content |
|---|---|---|
| `prVW` | 1 | zlib; magic `CA FE BE EF`, u16 w/h, RGB palette + `00 00 00` terminator, then **16384 B = 128×128 palette-index thumbnail** (padding is uninitialized memory — ignore it) |
| `mkBF` | 1 | 72–84 B; magic `FA DE CA FE`; mostly zeroes; purpose unknown |
| `mkTS` | 1 | zlib; **the document**: ASCII FourCC node tree (`MKBv` root, one `PDCv` per page) |
| `mkBS` | 1 | Same encoding; legacy simplified copy (document node only, no pages) |
| `mkBT` | N | magic `FA CE CA FE`, u24 tile index, grayscale flag, then zlib → **65536 B = 128×128 ARGB tile** (or 256×256 8-bit texture); streams >64 KB split across consecutive chunks |

**Node-tree encoding** (mkTS/mkBS): each node = 4 ASCII letters; the 4th
letter is the type — `i` int (hex ASCII), `f` float (decimal ASCII), `b`
bool (`0`/`1`), `s` string (BE u16 length + BE UTF-16), `v` container
(children in `{}` braces). Example: `MKBv{FRCi{1}XLCi{0}LYLv{LAYv{…}}}`.
~500 node FourCCs documented in `docs/research/fireworks-png-nodes.md`
(`PTHv` paths/`PBPv` béziers, `TXTv`/`TFSv` rich text runs, `EFDv` effect
dictionaries, `NGSv` nine-slice, `EXPv` export options, `MSKv` masks, …).

**Known unknowns (format risks):** `mkBF` semantics; long-name guesses in the
node dictionary ("inferred", per its author); exact filter-parameter encodings
inside `EFDv`; color-int layouts vary by context (RGBA/ABGR/ARGB + string
forms); unexplained IDs (`FRHN`, `HMKB`, `IUMF`). These are attacked by:
corpus differential-testing vs png-analyser, and Ghidra passes over the
`CMkbStructureOutputter` writer path when the corpus stalls.

**Verification strategy:** (a) fuzz the parser — no panics on arbitrary
bytes; (b) unknown nodes/fields **preserved verbatim** (lossless round-trip
even before full semantics); (c) round-trip floors: re-serialize parsed
corpus files and require byte-identical private-chunk payloads at a rising
`FLOOR` percentage; (d) render oracle: compose output vs CS6's own flattened
export, pixel-diff floors; (e) compile-time completeness: adding a field to a
document-model struct without mapping it into the format **fails the build**
(photocraft §8 trick).

**WASM constraint on the format stack:** `fwpng` and every L0–L3 crate stay
**pure Rust, no C FFI** so the whole document pipeline builds for
`wasm32-unknown-unknown`. C-FFI deps (libwebp, libheif, mozjpeg, libjxl) are
native-only cargo features, default-off for wasm.

---

## 7. Architecture

### 7.1 Workspace layout (photocraft pattern: layer-numbered, xtask-enforced)

```
crates/
  L0  geom        kurbo-based geometry, transforms, béziers (no deps beyond kurbo/glam)
      color       pixel formats, 27 blend modes, color models, premultiply
      raster      copy-on-write tiled bitmap store (128×128 tiles — matches mkBT)
      fwpng       standalone publishable crate: PNG + private chunks + node-tree
                  parse/serialize. ZERO workspace deps (like photocraft's `psd`)
  L1  doc         pure-data document model: pages/states/layers/objects/symbols;
                  compile-fails if any field is unmapped to the format
  L2  vector      fills, strokes, gradients, patterns, path ops, auto-shape logic
      filters     live filters: CPU kernels first (resvg/Graphite pattern), GPU later
      text        cosmic-text layout + Fireworks text attributes (leading, tracking…)
      ops         history: delta command stack, transactions, coalescing
  L3  compose     CPU compositor — the deterministic reference oracle
      gpu         wgpu/vello_gpu fast path (same output, benchmarked vs compose)
      engine      session, command registry ("everything is a command"),
                  scripting host, platform-services traits (fs/dialogs/clipboard
                  injected — engine never touches std::fs directly)
  L4  io          import/export pipeline (raster codecs, PSD, SVG, PDF, FXG, HTML/CSS)
  L5  app         egui/eframe desktop+web shell (thin, data-driven UI)
      app-cli     headless CLI: convert / render / batch / run-script / commands / MCP
      automation  control channel + MCP server (same command ids as UI/CLI)
  cross  testkit   corpus harness, golden-image tools, fuzz anchors
web/             trunk-based wasm build of the same shell (photocraft-web pattern)
apps/            (later: packaging, installers)
xtask/           layers | wasm | ci | parity | scorecard | perf | corpus | fuzz anchors
docs/ plan/ log/ assets/ packaging/
```

Workspace rules (photocraft golden rules, adopted verbatim where applicable):
`edition = "2024"`; `unsafe_code = "forbid"` workspace-wide (if a crate ever
needs unsafe — tablet input, platform FFI — it becomes ONE isolated crate with
mandatory `SAFETY:` comments); **no `unwrap`/`expect`/`panic!`/`todo!` in
non-test code** — input-derived numbers are hostile (`get()`, `checked_*`,
allocation caps); every crash fix ships a regression test; **never break
wasm** — `cargo xtask wasm` runs in every finish checklist; MSRV pinned at M0
(edition 2024 ⇒ ≥1.85; expect 1.9x, matching ecosystem reality).

### 7.2 WebAssembly target (primary, per §2)

- **Shell:** eframe/egui compiles to wasm with wgpu backend (WebGPU;
  WebGL2 fallback) — the same `app` crate drives desktop and browser
  (photocraft-web/Graphite pattern: one editor core, two thin frontends).
- **wasm-specific services:** file open/save via browser File System Access
  API with download fallback; clipboard via async navigator APIs; no
  threads assumption (wasm threads where available, single-threaded
  fallback); persistence via OPFS where supported.
- **Bundle:** client-side-only static files (trunk), cache-friendly, no
  server logic — embeddable in WebOS via iframe/postMessage; a thin
  postMessage bridge exposes load-document/render/export to the host OS.
- **Dependency policy:** pure-Rust-first. Every C-FFI dependency is a cargo
  feature, default-off on wasm: scripting engine (see §7.4), libwebp,
  libheif, mozjpeg. CI gate: `cargo xtask wasm` builds the full workspace
  + runs the headless test subset in-browser (wasm-bindgen-test).

### 7.3 Render strategy

**CPU first, GPU second, always identical.** `compose` (vello_cpu 0.3 /
tiny-skia-class rasterization + our filter kernels) is the deterministic
oracle used by every test; `gpu` (wgpu 30 + vello_gpu 0.3) is the interactive
fast path — and on wasm, `vello_gpu`'s WebGL2 backend is the canvas.
Filters (blur/shadow/bevel…) are implemented as CPU separable kernels with
gamma-aware premultiply (Graphite `filter.rs` pattern; resvg is the reference
implementation for SVG-style effects). Live filters compose lazily with
caching keyed on content hashes. The famous compute-shader `vello 0.11` is
**research status — do not build on it**.

### 7.4 Document model & history

- Flat node registry + derived indexes (Graphite pattern): layer tree as
  index over a flat store, cached bounds/hit-targets per node. No recursive
  struct soup, cheap reparenting, cheap serialization.
- **Undo = inverse deltas from day one**, gesture-scoped transactions with
  coalescing (Graphite's RFC explicitly documents why snapshot undo fails at
  scale; photocraft's `ops` crate is the same lesson).
- **The document format IS Fireworks PNG** — no second native format in early
  milestones (revisit a zip container only if tile-heavy docs demand it).
- Persistence hygiene: write-to-sibling + atomic rename (`tempfile`) on
  desktop; OPFS + download fallback on web; rotating autosave generations;
  crash-recovery journal of the command stack.

### 7.5 Scripting & automation

- Engine: **decided by an M1 wasm spike** — primary `rquickjs` (QuickJS-NG,
  ES2020 superset of Fireworks' ES3, production-grade; C code — must prove a
  `wasm32` build) with fallback **`boa_engine`** (pure Rust, builds for wasm
  trivially, self-described "experimental"). Whichever wins, the Fireworks
  API (`117 fw.*`, `342 dom.*` from the "Extending Fireworks" PDF) is
  implemented as **host objects**, with ES3 quirk shims informed by
  fwjs-errata. Sandboxing: instruction/alloc limits.
- `.jsf` command loading (app + user dirs; virtual FS on web),
  `smartShape`/`Widget` globals for Auto Shapes and rich symbols — this is
  how CS6's own shipped Auto Shapes keep working (their scripts are behavior
  specs we port to).
- Automation surfaces all dispatch the **same command registry**: GUI,
  CLI (`firecraft-cli convert/render/batch/run-script`), loopback JSON
  control channel, and an MCP server — photocraft's "everything is a
  command". The CS6 TCP-12124 XML RPC is emulated late (T3, desktop only)
  for legacy tooling.

### 7.6 Stack decisions (verified 2026-10-10; alternatives + risk)

| Area | Choice | Alternative | Risk note |
|---|---|---|---|
| Windowing | `winit` 0.30 (desktop); eframe web entry (wasm) | SDL3 FFI | standard |
| UI toolkit | `egui` 0.36 + `egui-wgpu` (eframe for web) | iced 0.14; gpui (Zed) | immediate mode ⇒ scene lives in our state; proven for canvas apps (Rerun) and wasm (eframe); Slint rejected (GPL/commercial tri-license) |
| GPU | `wgpu` 30 (WebGPU + WebGL2 on wasm) | — | quarterly majors ⇒ breakage budget |
| 2D render | `vello_cpu`/`vello_gpu` 0.3 | `tiny-skia` 0.12 baseline | vello 0.11 compute = research, avoid; vello_gpu has WebGL2 backend for wasm |
| Geometry | `kurbo` 0.13 (+`glam`) | `lyon_tessellation` 1.0 | path booleans are bespoke work (spike in M6) |
| Text | `cosmic-text` 0.19 (wasm32-capable) | `parley` 0.12 | cosmic-text has editor/caret APIs |
| PNG | `png` 0.18 + **custom chunk layer** | — | `png` crate has no public generic chunk writer — fwpng wraps it for private chunks |
| JPEG | `zune-jpeg` dec + `jpeg-encoder` | mozjpeg FFI (native-only feature) | pure-Rust pair keeps wasm green |
| GIF | `gif` 0.14 | — | incl. animated → states |
| WebP | `image-webp` 0.2 | lossy enc: `libwebp-sys` (native-only) | lossless-only encode in pure Rust |
| AVIF | `ravif`+`rav1e` | libaom FFI | AV1 royalty-free — safe |
| JPEG XL | `jxl-oxide` (decode) | — | **`jxl-encoder` is AGPL — do not use**; encode via libjxl FFI or skip |
| TIFF/BMP/WBMP | `tiff` 0.11 / `image` / `wbmp` | — | `wbmp` tiny — wrap/vendor if stalled |
| HEIC/HEIF | **OS decoders only** (VideoToolbox/MediaFoundation via libheif-rs platform backends) | pure-Rust `heic` crate | **no bundled HEVC decode** — patent pools; browsers take the same stance; native-only feature (§10) |
| PSD | `ag-psd` 0.3 (read+write, layers) | `psd` 0.3.5 (stale) | young crate — spike early, be ready to fix forward |
| SVG | `usvg`/`resvg`/`svgtypes` 0.48 | — | resvg = filter-effects reference |
| PDF | `pdf-writer` 0.15 + `lopdf` 0.45 | printpdf | typst-style split |
| FXG | none exists — write from Adobe's public FXG spec | — | XML subset; scope to T3 |
| Scripting | M1 spike: `rquickjs` 0.14 vs `boa_engine` 0.22 | — | wasm build is the deciding test (§7.5) |
| Undo | bespoke delta stack | `undo` 0.52 | command objects + transactions |
| Persistence | `tempfile` atomic rename; serde+ron where internal; OPFS on web | — | autosave rotation |
| Clipboard/dialogs | `arboard` 3.6 / `rfd` 0.17 (desktop); browser APIs (web) | — | behind platform-services traits |
| i18n / dirs | `fluent` 0.17 / `dirs` 7 (desktop) | — | — |
| wasm toolchain | `trunk` + `wasm-bindgen`; `wasm-pack` for fwpng publish | — | photocraft-web pattern |
| Testing | `insta` / `goldenfile` / `image-compare`; `cargo-fuzz` parsers; `criterion`; `wasm-bindgen-test` | — | `pixeldiff` does not exist; `image-compare` is real |

---

## 8. Scope: feature tiers

| Tier | Contents |
|---|---|
| **T0 — MVP** | Open/render/save `.fw.png` (flat docs): primitives + pen, fills/strokes/gradients, layers/pages/states, select/subselect/transform, undo, guides/rulers, PNG/JPEG/GIF export — **running both native and in-browser** |
| **T1 — Core parity** | Auto Shapes (13 tools + library), path ops, text engine + text-on-path, 27 blend modes, masks, core live filters, smart guides, graphic symbols, 9-slice, styles panel |
| **T2 — Bitmap + formats** | Selections/retouch tools, full filter set, PSD import/export (fidelity-tested), SVG in/out, TIFF/BMP/WBMP, WebP/AVIF, slices + export workflows, batch, `.jsf` commands + history replay |
| **T3 — Web/animation/scripting** | Button/animation/rich symbols + common library (original assets), behaviors (rollover/swap), states animation → animated GIF, CSS+Images / CSS Sprites export, CSS extraction panel, FXG, PDF export, HTML export, RPC emulation (desktop), MCP |
| **T4 — Modern/polish** | Perf budgets enforced, WebOS embedding polish (postMessage bridge, OPFS autosave), plugin surface, i18n, HEIC via OS decoders, JXL decode, packaging/installers, release matrix |

**Non-goals (modern wins over CS6):** Flash-era exports — SWF, AIR, Flex
skinners/MXML, Director, Device Central, Lotus Domino (all deprecated even in
CS6); Flash/SWF command panels (FireCraft panels are native); Extension
Manager `.mxp/.zxp` packaging (native extension story instead); Photoshop
`.8bf` plugin hosting (stretch, T4+); PDF *import* (CS6 never had it);
bundled HEVC (patents — OS decoders only).

---

## 9. Roadmap (summary — canonical: [`planning/firecraft/ROADMAP.md`](planning/firecraft/ROADMAP.md))

| Milestone | Theme | Gate (abridged) |
|---|---|---|
| **M0** | Bootstrap: workspace, xtask gates (incl. `wasm`), AGENTS.md, ATTRIBUTION.md, licensing | `cargo xtask ci` green on scaffold, native + wasm |
| **M1** | L0 foundations: geom, color, raster tiles + **scripting-engine wasm spike** | property tests + fuzz clean; engine decision recorded |
| **M2** | `fwpng`: chunk layer, node-tree parser, `prVW`/`mkBT` | corpus parses panic-free; cross-checks vs png-analyser; wasm build of fwpng |
| **M3** | `doc` model + format mapping + round-trip writer | byte-level round-trip floors rising; compile-fail completeness |
| **M4** | CPU compositor | golden-image floors vs CS6 exports |
| **M5** | Editor shell (native + **web build**): commands, undo, canvas, basic tools | open→edit→save `.fw.png` in both targets; `panic_hunt` fuzz |
| **M6** | Vector depth: path ops, Auto Shapes | scripted shapes parity vs CS6 geometry |
| **M7** | Bitmap + full live filters | filter golden images vs CS6 |
| **M8** | Import/export pipeline | per-format round-trip + oracle suites |
| **M9** | Scripting/automation | shipped `.jsf` corpus runs headless |
| **M10** | Symbols/libraries/components | symbol round-trip + rich-symbol scripts |
| **M11** | Web export: slices/behaviors/animation/CSS sprites | export-oracle suites vs CS6 HTML |
| **M12** | Parity hardening, perf, packaging, **release matrix** | scorecard `enforce = true`; installers + wasm bundle releases |

---

## 10. Risks & mitigations

| # | Risk | Mitigation |
|---|---|---|
| 1 | **mkTS node semantics partially unknown** (dictionary is observational; filter params, color layouts) | corpus differential-testing vs png-analyser; unknown nodes preserved verbatim so round-trip never waits on semantics; targeted Ghidra passes on `CMkbStructureOutputter` when corpus stalls |
| 2 | **HEVC/HEIC patents** — no open license; shipping decoders is where pools assert | policy: OS-provided patent-licensed decoders only (Firefox/Chrome stance); pure-Rust decode documented as user's legal decision; HEIC is T4, native-only feature |
| 3 | `ag-psd` young (0.3) for PSD layer read+write | early spike (M8 pre-work); fallback: vendor + fix forward (photocraft ships its own `psd` crate — precedent) |
| 4 | Vello/wgpu churn (quarterly majors; vello restructured 2026) | CPU `compose` oracle is renderer-independent — tests never depend on GPU stack; pin versions; budget upgrade windows |
| 5 | **wasm constrains dependencies** (no threads assumption, C FFI risk, bundle size) | pure-Rust-first policy + feature-gated FFI deps; `xtask wasm` gate in every finish checklist; M1 spike settles the scripting engine; WebGL2 fallback for WebGPU gaps |
| 6 | **Asset copyright** (icons, libraries, patterns, fonts) | originals only; ATTRIBUTION-in-same-change rule; fonts in separate optional repo |
| 7 | Trademark/naming friction | distinct name; standard Adobe disclaimer; no Fireworks-derived branding |
| 8 | Scope blow-up (CS6 is enormous) | tier gates + rising-only `FLOOR`s; honest parity/scorecard docs; non-goals list is binding |
| 9 | Solo dev + agent sessions lose context | photocraft method: gitignored `log/devlog.md` handoff, AGENTS.md finish checklist, session task lists mirror current milestone |
| 10 | ES3/SpiderMonkey quirks vs QuickJS/Boa | fwjs-errata as quirk ledger; compatibility shims tested against shipped `.jsf` corpus |
| 11 | Corpus capture requires Wine+CS6 on this host | degrades gracefully: harvested real-user files + png-analyser cross-checks; Wine harness is a force multiplier, not a dependency |

---

## 11. Method & process

1. **AGENTS.md as operating manual** (M0 deliverable): workspace map with
   layers, golden rules (§7.1), work-picking order (roadmap → parity →
   scorecard → focus), **finish checklist** (test → clippy → `xtask layers`
   → `xtask wasm` → `xtask parity` → fuzz → devlog entry), parallel-agent
   etiquette (per-agent `CARGO_TARGET_DIR`, own your files, atomic Cargo.toml
   writes).
2. **`xtask` verification gates** (photocraft pattern): `layers` (dependency
   direction enforced), `wasm` (full workspace builds + headless tests in
   browser), `parity` (generated missing-feature doc, rising-only floor),
   `scorecard` (measured checklist, `enforce = true` budgets),
   `corpus`/`test-corpus` (pinned+sha256, fetched never committed),
   `panic_hunt` (fuzz every command with adversarial params).
3. **Tracking:** milestones + tickets in `docs/planning/firecraft/`
   (objective / inputs / authority / stopping condition / verification per
   ticket); `plan/` and `log/` are gitignored; session task lists mirror the
   active milestone; ATTRIBUTION.md rows land in the same change as assets.
4. **Corpus:** `firecraft-corpus` — CS6-authored `.fw.png` docs generated by
   `.jsf` scripts under Wine + user's real documents (with permission) +
   public sample files; every file sha256-pinned; byte-level round-trip and
   golden-image floors derived from it. Never commit Adobe-authored files to
   this repo.
5. **Releases (photocraft matrix):** tagged releases carry Linux
   (tar/AppImage/deb), macOS (universal dmg), Windows (installer + portable
   zip), headless CLI archives, and the **web bundle** (static wasm, also
   deployable to any static host / WebOS shelf). Changelog generated from
   conventional commits.
6. **Definition of done for any feature:** tests green, clippy `-D warnings`
   clean, layers + wasm checks clean, parity doc regenerated, devlog
   appended, scorecard row updated.

---

## 12. Research appendix (condensed, sourced 2026-10-10)

**Fireworks PNG format** — chunks `prVW`/`mkBF`/`mkTS`/`mkBS`/`mkBT`
(pngcheck source; TrID definition; fileformats.archiveteam.org wiki):
prVW = 128×128 palette thumbnail (`CA FE BE EF` header); mkBT = 128×128 ARGB
tiles (`FA CE CA FE` + u24 index, 64 KB zlib spans concatenated); mkTS/mkBS =
big-endian ASCII-FourCC node trees, 4th letter = type (i/f/b/s/v), zlib.
Document root `MKBv` (canvas `WIDf/HITf/RESf/RSUi/BGCi`, `FRCi` frame count,
`LYLv` layers), `PDCv` per page, `PTHv` paths (`PBPv` béziers, `CPTv`
control points), `TXTv`/`TFSv`/`TRNs` rich text, `EFDv`/`DCEv` effects,
`MSTv`/`ALSv` symbols, `NGSv` nine-slice, `EXPv` export options, `GDSv`
guides, `FGLv` gradients. Writer class `CMkbStructureOutputter` in exe RTTI.
`prWM` chunk: **refuted** (does not exist). Full dictionary: vendored
`docs/research/fireworks-png-nodes.md`. Best public parser: png-analyser
(MIT); Photopea reads FW PNGs (closed source); ExifTool/PolyFile/BANG
recognize chunks without decoding.

**Timeline** — FW 1.0 1998 (webdesignmuseum.org); version map v1→CS6=v12
(Wikipedia: Adobe Fireworks); Macromedia acquisition closed 2005-12-03
(Adobe press, archived); CS6 GA 2012-05-07, 12.0.1 2013-06-17 (Adobe blogs,
archived); discontinuation post 2013-05-06 ("The future of Adobe Fireworks",
archived); EOL core 2013/ext 2015 (helpx.adobe.com/support/programs/eol-matrix.html).
The "EOL Feb 2021" claim found in secondary sources: **refuted**.

**Scripting** — engine = SpiderMonkey "JavaScript-C 1.5 pre-release 3"
(source shipped in local tree, MPL; quirks at fwextensions/fwjs-errata);
globals `fw` / `fw.getDocumentDOM()` / `Files` / `Dialogs` / `pngText` /
`Errors` / `Find` / `System`; context globals `smartShape`, `Widget`
(`opCode` 1=setDefaultValues 2=applyCurrentValues); 117 `fw.*` + 342 `dom.*`
("Extending Fireworks" PDF, live at help.adobe.com archive URL — §5.2);
CS6 additions incl. `fw.exportPages` (headless conversion), `fw.saveJsCommand`,
`fw.exportCSSSpriteForJQuery`; no `fw.batchProcess()` (batch drives the UI
via `fw.openDocuments` + progress globals); RPC = TCP 127.0.0.1:12124,
null-terminated UTF-8 XML fragments (`<get>/<set>/<func>/<release>`), one
request per connection, documented since MX 2004 (devnet, archived).

**Feature surface** — enumerated against the archived CS6 reference PDF
(§5.2) and cross-checked against the local install: 13 Auto Shape *tools*
(14 `.jsf` entries locally incl. Arrow Line), ~27 Auto Shape library shapes,
27 blend modes, live-filter catalog, Pages/States/Layers/Web-Layer model,
slice auto-naming (8 ordered elements), optimization palettes
(Adaptive/WebSnap/Web216/Exact/Win/Mac/Grayscale/B&W/Uniform/Custom `.act`),
CS6 additions = CSS Properties panel, CSS Sprites export, jQuery Mobile
themes, `.fw.png` default naming; deprecated-in-CS6 list (§8 non-goals).
Local install carries 5 Common Library PNGs — the fuller CS6 library lists
circulating online are **unverified** against our tree.

**StoryTold method** — photocraft (39.8k★, MIT OR Apache-2.0): 24
layer-numbered crates (`xtask layers` enforced), everything-is-a-command
registry (UI/CLI/JSON/MCP dispatch same ids), golden rules (never crash >
features; `unsafe_code = "forbid"` + one exception crate; hostile input
numbers; tests are the gate; **never break wasm**), finish checklist +
gitignored devlog for agent-session handoff, parity/scorecard generated docs
with rising `FLOOR`s, `panic_hunt` command fuzzing, compile-fail
unmapped-field trick in the format crate, corpus of real-app-authored
oracles (photocraft-corpus: 256 PSDs) fetched pinned+sha256 never committed,
ATTRIBUTION rows in-same-change, fonts/demos/assets separation, Adobe
trademark disclaimer verbatim, wide release matrix incl. web build. Org:
51 repos, `craft-*` infra prefix, app ids `ai.storyteller.<app>`.

**Rust stack** — every crate verified on crates.io 2026-10-10 (versions in
§7.6 table). Notable: vello restructured 2026 (compute renderer now
"research"; vello_cpu/vello_gpu are the supported pair; vello_gpu ships a
WebGL2 backend); wgpu on quarterly majors; `png` lacks public generic chunk
writer (custom chunk layer needed); `image-webp` lossless-encode only;
`jxl-encoder` AGPL (avoid); HEIC = OS decoders (patent pools; GNOME 2024
thread; imazen `heic` crate AGPL + backend-policy pattern); cosmic-text,
parley, boa build for wasm32; Graphite read at commit `7477cd2`
(2026-10-10): editor/GUI separation, dispatcher message bus, delta-undo RFC,
flat registry document model, content-addressed resources, CPU filter
kernels — patterns adopted in §7.
