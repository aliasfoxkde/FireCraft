# Phase 4 — Binary & WASM Size Optimization, Name Stripping, JS Minification & Brotli/Gzip (`T31.1`–`T31.2`, `0090`)

> **Goal:** Systematically shrink FireCraft's WASM and native binaries using **measured, same-SHA before/after experiments** (one lever per commit!) — honoring your request for name/debug stripping, JS minification/mangling, Brotli + Gzip compression, and dropping runtime `resvg` WASM bloat while **keeping 100% crisp Phosphor Icons**!
> **Rule:** Never guess. Every task card appends an observed byte row to `docs/planning/generated/size-baseline.md` and proves `xtask dist --web` bridge smoke stays green.

---

### `L4.1` `[LUNA]` Reproducible WASM + Native Size & `twiggy` Baseline (`T31.1`)

- **Files:** `docs/planning/generated/size-baseline.md` (new)
- **Pre-check:**
  ```bash
  ssh fedora 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"; which trunk wasm-opt twiggy brotli gzip 2>&1 || true'
  ```
- **Do:**
  1. Ensure `twiggy` and `brotli` are installed on the web build host (`cargo install twiggy --locked` if absent).
  2. From one clean commit, run `cargo run -p xtask -- dist --web` and `cargo build --release -p app -p app-cli`.
  3. Run `twiggy top -n 40 dist/web/*_bg.wasm` and `twiggy dominators -r 25 dist/web/*_bg.wasm`.
  4. Measure exact bytes for raw `.wasm`, `.wasm.gz` (level 9), `.wasm.br` (quality 11), `.js`, `.js.gz`, `.js.br`, `index.html`, and native `firecraft` / `firecraft-cli`.
  5. Write `docs/planning/generated/size-baseline.md` with commit SHA, tool versions, byte table, and top 25 `twiggy` contributors.
- **Verify:**
  ```bash
  test -s docs/planning/generated/size-baseline.md && grep -q "twiggy" docs/planning/generated/size-baseline.md
  ```
- **Done when:** Baseline report is committed with exact byte counts and top 25 `twiggy` dominators.

---

### `L4.2` `[LUNA]` Web-Only Cargo Profile Tuning & `wasm-opt -Oz` Name/Debug Stripping

- **Files:** `xtask/src/dist.rs`, `web/index.html`, `docs/planning/generated/size-baseline.md`
- **Pre-check:**
  ```bash
  grep -n "CARGO_PROFILE_RELEASE\|data-wasm-opt" xtask/src/dist.rs web/index.html
  ```
- **Do:**
  1. Benchmark web-only env overrides in `xtask/src/dist.rs::stage_web()` (leaving native profile unchanged):
     - `CARGO_PROFILE_RELEASE_OPT_LEVEL=z`
     - `CARGO_PROFILE_RELEASE_LTO=fat`
     - `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`
     - `CARGO_PROFILE_RELEASE_PANIC=abort`
     - `CARGO_PROFILE_RELEASE_STRIP=symbols`
  2. Add Binaryen debug/name/producer stripping flags in `web/index.html` (`data-wasm-opt-params="--strip-debug --strip-dwarf --strip-producers --dce --vacuum"`, verified against pinned `trunk` CLI flags).
  3. Keep ONLY flags that reduce bytes AND pass `xtask dist --web` bridge smoke; append before/after row to `size-baseline.md`.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p xtask && cargo run -p xtask -- dist --web'
  ```
- **Done when:** Stripped WASM shrinks measurably and passes `xtask dist --web` bridge smoke.

---

### `L4.3` `[LUNA]` Remove Runtime `resvg`/`usvg`/`fontdb` From WASM (Using Phase 2's Embedded `Phosphor-Regular.ttf`)

- **Files:** `crates/app/src/shell/icons.rs`, `crates/app/Cargo.toml`, `web/Cargo.toml`, `docs/planning/generated/size-baseline.md`
- **Pre-check:**
  ```bash
  grep -n "egui_extras" crates/app/Cargo.toml web/Cargo.toml
  ```
- **Do:**
  1. Read `L4.1`'s `twiggy` report to record exact bytes consumed by `resvg` + `usvg` + `tiny-skia` + `fontdb` + `egui_extras/svg`.
  2. Confirm every GUI icon is now rendered via Phase 2's embedded `Phosphor-Regular.ttf` font (and optional subsetting if desired to drop unused codepoints from the 488 KB font).
  3. Remove `egui_extras/svg` (and runtime `resvg`/`usvg`/`fontdb`) from `crates/app/Cargo.toml` / `web/Cargo.toml` while preserving crisp, tintable Dark/Light Phosphor icons at 1x/2x DPI!
  4. Verify `xtask icon-survey` stays green, visually inspect desktop & browser screenshots to confirm Phosphor Icons stay razor-sharp, and record exact multi-MB WASM savings in `size-baseline.md`!
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo test -p xtask && cargo run -p xtask -- dist --web'
  ```
- **Done when:** `resvg`/`usvg`/`fontdb` are absent from the WASM dependency tree, Phosphor Icons look identical at 1x/2x DPI, and WASM savings are recorded.

---

### `L4.4` `[LUNA]` Embedded Font Deduplication Audit (`default_fonts` vs `LiberationSans`)

- **Files:** `crates/text/src/lib.rs`, `web/Cargo.toml`, `crates/app/Cargo.toml`, `docs/planning/generated/size-baseline.md`
- **Pre-check:**
  ```bash
  ls -lh crates/text/assets/ && grep -n "default_fonts" crates/app/Cargo.toml web/Cargo.toml
  ```
- **Do:**
  1. Check `twiggy` bytes for `epaint_default_fonts` vs `crates/text/assets/LiberationSans-{Regular,Bold}.ttf` (824 KB).
  2. Test sharing `LiberationSans` cleanly as the UI sans-serif font (dropping unused duplicate font faces only if all text tests and visual screenshots stay crisp with zero missing glyphs!).
  3. Record before/after bytes in `size-baseline.md`. If visual quality degrades, keep the sharper font set and document why.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p text -p app && cargo run -p xtask -- dist --web'
  ```
- **Done when:** Font duplication is either eliminated cleanly or documented with exact byte cost and visual proof.

---

### `L4.5` `[LUNA]` JS Glue & Site Script Minification + Local Identifier Mangling

- **Files:** `xtask/src/dist.rs`, `docs/planning/generated/size-baseline.md`
- **Pre-check:**
  ```bash
  which esbuild terser npx 2>&1 || true
  ```
- **Do:**
  1. In `xtask/src/dist.rs::stage_web()`, add a deterministic minification + local-variable mangling pass (`esbuild --minify` or `terser --compress --mangle --module`) over staged `dist/web/*.js` glue (preserving `wasm-bindgen` entry exports and `postMessage` WebOS bridge symbols).
  2. Minify staged `dist/web/index.html` whitespace/comments safely without touching accessible fallback text or `<link>` asset paths.
  3. Verify `xtask dist --web` bridge smoke passes on the minified JS+HTML and record raw/`.gz`/`.br` savings in `size-baseline.md`.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p xtask && cargo run -p xtask -- dist --web'
  ```
- **Done when:** Staged JS glue is minified/mangled cleanly and headless Chromium bridge smoke passes.

---

### `L4.6` `[LUNA]` Brotli (`.br` Quality 11) + Gzip (`.gz` Level 9) Pre-Compression & Header Verification

- **Files:** `xtask/Cargo.toml`, `xtask/src/dist.rs`, `web/_headers` (new), `docs/planning/generated/size-baseline.md`
- **Pre-check:**
  ```bash
  grep -n "is_compressible_web_asset\|flate2" xtask/src/dist.rs xtask/Cargo.toml
  ```
- **Do:**
  1. Add pure-Rust `brotli` crate (or fail-closed pinned `brotli` check) to `xtask` so `stage_web()` deterministically writes BOTH `<file>.gz` (level 9) AND `<file>.br` (quality 11) for every `.wasm`, `.js`, and `.html` file.
  2. Add unit tests in `xtask/src/dist.rs` verifying `.gz` and `.br` round-trip byte-identically to the raw files.
  3. Add `web/_headers` configuring immutable cache headers for hashed `.wasm`/`.js` assets and correct `application/wasm` MIME type; verify local server header responses.
  4. Record final `.br` and `.gz` transfer bytes in `size-baseline.md`.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p xtask && cargo run -p xtask -- dist --web && ls -lh dist/web/*.wasm*'
  ```
- **Done when:** Every compressible staged web file ships verified `.gz` and `.br` sidecars and `size-baseline.md` records exact transfer savings.

---

### `L4.7` `[LUNA]` Native Desktop Binary Strip & Size Benchmark (Linux & Windows)

- **Files:** `docs/planning/generated/size-baseline.md` (and `Cargo.toml` ONLY if separate debuginfo policy is approved)
- **Pre-check:**
  ```bash
  ls -lh /var/tmp/firecraft-linux-check/firecraft /nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe
  ```
- **Do:**
  1. Measure native `firecraft` and `firecraft-cli` sizes with default release vs `CARGO_PROFILE_RELEASE_STRIP=symbols` (preserving optional separate `.dwp`/`.pdb` debug symbols so crash reports remain diagnosable).
  2. Confirm native release speed/frame-time budgets are not regressed.
  3. Record native before/after byte sizes in `docs/planning/generated/size-baseline.md`.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo run -p xtask -- aegis'
  ```
- **Done when:** Native release strip measurements are documented with zero regression.

---

### `L4.8` `[HUMAN GATE]` Maintainer Size Optimization Report Sign-Off

- **Do:** Present the full before/after table from `docs/planning/generated/size-baseline.md` (raw `.wasm`, `.gz`, `.br`, JS glue, native binaries, and bridge smoke receipts) to the maintainer before starting Phase 5 modular folder splits.
- **Done when:** Maintainer approves Phase 4 optimization results.
