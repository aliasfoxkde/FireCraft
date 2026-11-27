# Phase 5 — Modular Folders (`examples/`, `libraries/`, `extensions/`) & Lazy WASM Modules (`T31.3`–`T31.4`)

> **Goal:** Split runtime assets, sample documents, symbol/style libraries, sandboxed extensions, and optional heavy WASM features into clean, maintainable folders loaded on demand — keeping the core binary lean and distributions organized (`bin/`, `examples/`, `libraries/`, `extensions/`, `dist/web/modules/`).

---

### `L5.1` `[LUNA]` Create Modular `examples/` Folder & Sample `.fw.png` Documents

- **Files:** `examples/manifest.json`, `examples/*.fw.png`, `docs/ATTRIBUTION.md`
- **Pre-check:**
  ```bash
  ls -la /nas/Temp/repos/FireCraft/examples 2>&1 || true
  ```
- **Do:**
  1. Create `examples/` with 4 genuine, clean-room editable `.fw.png` documents generated via `firecraft-cli` / `demo.rs` (zero Adobe files):
     - `studio-showcase.fw.png` (1920×1080 hero showcase)
     - `ui-card-kit.fw.png` (reusable vector buttons/cards)
     - `icon-grid.fw.png` (vector alignment & boolean ops tutorial)
     - `animated-banner.fw.png` (multi-state web banner with slice)
  2. Create `examples/manifest.json` (`schema: "firecraft-examples/v1"`, id, title, description, file, width, height, sha256).
  3. Verify every `.fw.png` opens, renders, and round-trips through `fwpng` cleanly.
- **Verify:**
  ```bash
  ls -lh /nas/Temp/repos/FireCraft/examples/ && python3 -m json.tool /nas/Temp/repos/FireCraft/examples/manifest.json >/dev/null
  ```
- **Done when:** `examples/` contains 4 valid `.fw.png` documents and a schema-valid `manifest.json`.

---

### `L5.2` `[LUNA]` Create Modular `libraries/` Folder (`symbols/`, `styles/`, `swatches/`)

- **Files:** `libraries/manifest.json`, `libraries/styles/`, `libraries/symbols/`, `libraries/swatches/`
- **Pre-check:**
  ```bash
  ls -la /nas/Temp/repos/FireCraft/assets/library /nas/Temp/repos/FireCraft/assets/styles
  ```
- **Do:**
  1. Organize clean-room shipped libraries under top-level `libraries/`:
     - `libraries/styles/` (shipped `.fw.stl` style libraries)
     - `libraries/symbols/` (Common Library symbol packs)
     - `libraries/swatches/` (CS6 web/UI swatch palettes)
  2. Add `libraries/manifest.json` (`schema: "firecraft-libraries/v1"`, entries with id, kind, path, sha256).
  3. Keep any existing `crates/app/tests/styles.rs` and `library` test paths green (update paths cleanly or keep compatibility shim).
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app --test styles && python3 -m json.tool libraries/manifest.json >/dev/null'
  ```
- **Done when:** `libraries/` has a valid manifest and all existing style/library tests pass.

---

### `L5.3` `[LUNA]` Implement Lazy Runtime Resource Loader (`resources.rs`) for Desktop & WASM

- **Files:** `crates/app/src/resources.rs` (new), `crates/app/src/lib.rs`, `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/welcome_overlay.rs`
- **Pre-check:**
  ```bash
  grep -rn "load_demo_showcase" /nas/Temp/repos/FireCraft/crates/app/src/
  ```
- **Do:**
  1. Implement `crates/app/src/resources.rs` to discover `examples/` and `libraries/`:
     - **Desktop:** executable-adjacent `./examples` & `./libraries` -> user data dir -> workspace root (dev mode).
     - **WASM:** async on-demand HTTP fetch of `examples/manifest.json` & `libraries/manifest.json` (ZERO bytes fetched at startup!).
  2. Reject path traversal (`..`, absolute paths, symlinks escaping root) with typed errors.
  3. Wire `File ▸ Open Example ▸ ...` and the Welcome Screen sample rows to open real files from `examples/`.
  4. Add unit tests for manifest parsing, missing-folder fallback, and path-traversal refusal.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app resources && cargo clippy -p app --all-targets -- -D warnings && cargo check --target wasm32-unknown-unknown -p firecraft-web'
  ```
- **Done when:** Desktop and WASM load `examples/` & `libraries/` on demand with path-safety unit tests passing.

---

### `L5.4` `[LUNA]` Define Sandboxed `extensions/` Contract & Discovery Loader (`T31.4` Part 1)

- **Files:** `docs/decisions/0131-extension-package-contract.md`, `extensions/manifest.schema.json`, `crates/app/src/extensions.rs` (new), `crates/app/src/lib.rs`
- **Pre-check:**
  ```bash
  ls -la /nas/Temp/repos/FireCraft/crates/scripting/src/
  ```
- **Do:**
  1. Write `docs/decisions/0131-extension-package-contract.md` specifying versioned sandboxed extensions (`schema: "firecraft-extension/v1"`, id, name, version, api_range, kind `"jsf"` | `"command-pack"`, capabilities allowlist, entry, sha256) — zero unsafe native DLLs!
  2. Implement `crates/app/src/extensions.rs` to discover, validate, enable/disable, and run extensions through `engine` / `scripting` with statement/time caps and explicit capability enforcement.
  3. Add unit tests covering incompatible API rejection, malformed JSON, path traversal, and undeclared capability refusal.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app extensions && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- aegis'
  ```
- **Done when:** Extension loader validates manifests, enforces capabilities, and passes all negative security tests.

---

### `L5.5` `[LUNA]` Ship 2 Real First-Party Extensions Under `extensions/` & Manager UI (`T31.4` Part 2)

- **Files:** `extensions/index.json`, `extensions/css-design-tokens/`, `extensions/batch-social-presets/`, `crates/app/src/shell/dialogs.rs`, `crates/app/src/shell/chrome.rs`
- **Pre-check:**
  ```bash
  grep -rn "Commands" /nas/Temp/repos/FireCraft/crates/app/src/shell/chrome.rs | head -n 15
  ```
- **Do:**
  1. Ship 2 genuinely useful first-party extensions (no placeholders!):
     - `extensions/css-design-tokens/` — reads document fills/strokes/text styles and produces clean CSS custom properties.
     - `extensions/batch-social-presets/` — adds standard web/social artboard pages via engine commands.
  2. Wire `Commands ▸ Manage Extensions...` dialog showing installed extensions, declared capabilities, enable/disable toggle, and `Run` action.
  3. Add integration tests executing both extensions against a real document and asserting exact output/undoability.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo check --target wasm32-unknown-unknown -p firecraft-web'
  ```
- **Done when:** Both extensions install, inspect, run, and undo cleanly on desktop and WASM.

---

### `L5.6` `[LUNA]` Extract One True Lazy WASM Feature Module (`dist/web/modules/`, `T31.3`)

- **Files:** `web/src/lazy.rs` (or worker bridge), `xtask/src/dist.rs`, `web/test.spec.ts`, `docs/planning/generated/size-baseline.md`
- **Pre-check:**
  ```bash
  head -n 50 /nas/Temp/repos/FireCraft/docs/planning/generated/size-baseline.md
  ```
- **Do:**
  1. Pick the #1 heaviest optional non-startup feature from `L4.1`'s `twiggy` dominators report (e.g., optional PSD/TIFF codecs or `.jsf` scripting worker — NEVER split core open/save/edit/undo!).
  2. Build it into `dist/web/modules/<feature>-<hash>.wasm` (with `.gz` + `.br` sidecars) and instantiate it via `WebAssembly.instantiateStreaming` ONLY when the user triggers that feature.
  3. Show explicit loading status and degrade gracefully if offline (core editing continues unaffected).
  4. Add a Playwright network-interception test in `web/test.spec.ts` proving `modules/*.wasm` is **0 requests at startup**, fetches **once on feature trigger**, and stays cached!
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo run -p xtask -- dist --web && npx playwright test web/test.spec.ts'
  ```
- **Done when:** Playwright network waterfall proves 0 startup fetch for the deferred module and working on-demand execution.

---

### `L5.7` `[LUNA]` Stage Modular Folder Hierarchy in All Native & Web Distribution Scripts

- **Files:** `xtask/src/dist.rs`, `packaging/build-windows.ps1`, `packaging/build-appimage.sh`, `packaging/build-macos-dmg.sh`
- **Pre-check:**
  ```bash
  grep -n "stage_archive\|stage_web" /nas/Temp/repos/FireCraft/xtask/src/dist.rs | head -n 20
  ```
- **Do:**
  1. Update `xtask/src/dist.rs` (`stage_web` and `dist --native`) plus Windows/AppImage/macOS scripts to stage:
     ```text
     firecraft-<version>/
       bin/firecraft (+ firecraft-cli)
       examples/
       libraries/
       extensions/
       LICENSE, NOTICE
     ```
  2. Run `xtask dist --native` and `xtask dist --web` on `fedora` and verify archive listings + SHA-256 manifests.
  3. Confirm raw `.wasm` is well under the 25 MB warning threshold (`0090`) and record final per-folder sizes in `size-baseline.md`.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo run -p xtask -- ci && cargo run -p xtask -- dist --native && tar -tzf dist/native/firecraft-*-linux-x86_64.tar.gz | head -n 30'
  ```
- **Done when:** Native archives and `dist/web/` both ship the clean modular folder layout and pass `xtask ci`.

---

### `L5.8` `[HUMAN GATE]` Maintainer Modular Architecture & Lazy-Load Sign-Off

- **Do:** Present final `size-baseline.md`, directory tree of `examples/`, `libraries/`, `extensions/`, `dist/web/modules/`, and Playwright network waterfall proof for maintainer sign-off before Phase 6.
- **Done when:** Maintainer approves Phase 5.
