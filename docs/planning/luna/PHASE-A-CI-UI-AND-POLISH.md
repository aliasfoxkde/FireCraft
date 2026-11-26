# Phase A — PR #189 Green CI, Remaining CS6 UI Polish, WASM Browser Proof, Feedback & Open Recent

> **Authority:** [`docs/planning/luna/README.md`](README.md) · [`docs/tasks/T29-experiment-integration-and-polish.md`](../../tasks/T29-experiment-integration-and-polish.md) · [`docs/tasks/T30-taskbar-launcher-menu.md`](../../tasks/T30-taskbar-launcher-menu.md) · [`docs/PRODUCT-UX-ROADMAP.md`](../../PRODUCT-UX-ROADMAP.md)
> **Worktree:** `/var/tmp/firecraft-int-1.0` (`integration/firecraft-1.0`) until PR `#189` merges into `main`; thereafter branch cleanly from `origin/main` (e.g., `feat/phase-a-ui-polish`).

---

### `L-A1` `[HUMAN GATE]` Verify PR `#189` Full Cross-Platform CI Green & Merge to `main`

- **Goal:** Confirm every required CI workflow on PR `#189` (`integration/firecraft-1.0`) passes across GitForge and GitHub Actions (`fmt`, `wasm32`, `gate`, `macOS`, `Windows`, `CodeQL`), then obtain maintainer merge approval.
- **Pre-check:**
  ```bash
  gh pr view 189 --json state,headRefOid,statusCheckRollup
  ```
- **Steps:**
  1. Inspect all checks on PR `#189` (`gh pr checks 189`).
  2. If any check is red, read the failing job log (`gh run view <run-id> --log-failed`), fix the exact root cause in `/var/tmp/firecraft-int-1.0`, verify on `ssh fedora`, commit, and push `gitforge` then `origin`.
  3. Re-run `gh pr checks 189` until every check reports `pass` (zero `fail`, zero unexplained `skip`).
  4. **STOP at `[HUMAN GATE]`:** Present PR `#189` URL, final commit SHA, CI run URLs, Linux desktop screenshots, and Windows review `.exe` SHA-256 to the maintainer for merge into `main`.
- **Verify:**
  ```bash
  gh pr checks 189 --required
  ```
- **Exit:** All checks green + maintainer merges PR `#189` (and reconciles local/gitforge `main`).

---

### `L-A2` `[LUNA]` Real-Browser Playwright Proof of CS6 Studio WASM Build (`P2.10`)

- **Goal:** Prove the newly integrated CS6 Studio UI compiles via `trunk build --release` (`data-wasm-opt="z"`), generates real `.gz` sidecars, passes `xtask dist --web` headless Chromium bridge smoke, and renders cleanly in Playwright at `1440x900` and `960x640` with zero console/page errors.
- **Pre-check:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; which trunk wasm-opt chromium chromium-browser google-chrome node npx 2>&1 || true'
  ```
- **Steps:**
  1. On `ssh fedora` (or the host with `trunk` + Chromium installed), run `cargo run -p xtask -- dist --web`.
  2. Confirm `dist/web/` contains `index.html`, `*.js`, `*_bg.wasm`, AND matching `.gz` sidecars (`*.wasm.gz`, `*.js.gz`, `index.html.gz`) produced by our fixed `is_compressible_web_asset`.
  3. Serve `dist/web/` locally (`python3 -m http.server 8089 --directory dist/web`) and run `web/test.spec.ts` plus a Playwright screenshot script capturing `1440x900` and `960x640` into `/var/tmp/firecraft-wasm-shots/`.
  4. **Visual inspection:** Open BOTH PNG screenshots with your image viewer tool and verify the dark charcoal CS6 chrome, crisp icons, canvas showcase, non-clipping tools footer at `960x640`, and zero JS console errors.
  5. Record exact raw `.wasm`, `.wasm.gz`, `.js`, and `.js.gz` byte sizes in `PROGRESS.md` (feeds directly into Phase B `L-B1`).
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && cargo run -p xtask -- dist --web && ls -lh dist/web/'
  ```

---

### `L-A3` `[LUNA]` Unify & Integrate Phosphor Icons Across the Entire GUI

- **Goal:** Replace mixed icon rendering (`icons.rs` Phosphor SVGs in flyouts vs `vector_icons.rs` programmatic glyphs in toolbar/rail/panels) with one cohesive, razor-sharp **Phosphor Icons (MIT)** system across EVERY chrome surface: left Tools rail, tool flyouts, Main Toolbar, Header bar, Document Tab view modes (`Original`/`Preview`/`2-Up`/`4-Up`), Canvas Footer (`Snap`/`Rulers`/`Guides`/Transport), Panel Dock / Iconic Mode rail, Align/Path panels, and Welcome Screen.
- **Pre-check:**
  ```bash
  ls /var/tmp/firecraft-int-1.0/assets/icons-phosphor/*.svg | wc -l && grep -rn "icons::paint\|vector_icons::draw" /var/tmp/firecraft-int-1.0/crates/app/src/shell/ | head -n 30
  ```
- **Steps:**
  1. Inventory all icons currently drawn via `vector_icons::draw` (`chrome.rs`, `panel_windows.rs`, `welcome_overlay.rs`, `panels.rs`) against the 57 shipped Phosphor SVGs in `assets/icons-phosphor/` (`crates/app/src/shell/icons.rs`).
  2. Vendor any missing MIT Phosphor SVG icons (Regular/Duotone consistent stroke weight) into `assets/icons-phosphor/`, update `crates/app/src/shell/icons.rs` + `docs/ATTRIBUTION.md`, and run `cargo run -p xtask -- icon-survey` (re-pinning the survey table in the same commit).
  3. Wire `chrome.rs`, `panel_windows.rs`, `welcome_overlay.rs`, and `panels.rs` to render through the unified Phosphor icon helper (keeping crisp tinting for Dark/Light themes, active/hover/disabled states, and exact HiDPI sizing, with clean programmatic fallback only for bespoke geometry glyphs like stroke/fill swatch chips).
  4. **Fix 3 visually-verified polish defects captured on 2026-11-26 (`shot-workspace.png` & `shot-min-960x640.png`):**
     - Replace missing-glyph Unicode boxes (`□`) in the Layers panel action row (`panels.rs`) and bottom-left Tools footer (`chrome.rs` swap/default/screen-mode buttons) with crisp Phosphor icons.
     - Shorten the top-right theme selector button text in `chrome.rs` (`Dark` / `Light` / `System`) so it never clips at the right edge of `1440px` or `960px` viewports.
     - Make the Canvas Footer (`chrome.rs::cs6_canvas_footer_bar`) responsive below `640px` canvas width (hide verbose format/DPI labels first) so state transport `1/1` never overlaps `Rulers | Guides` at `960x640`.
  5. Verify zero visual blur/clipping at `1440x900` (Dark + Light) and `960x640` by capturing and inspecting real `xvfb-run` PNG screenshots!
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo test -p xtask && cargo clippy -p app -p xtask --all-targets -- -D warnings && cargo run -p xtask -- icon-survey && cargo run -p xtask -- aegis'
  ```

---

### `L-A3b` `[LUNA]` Floating Panel Drag Ghost, Vertical Stack Grouping & Edge Snapping (`P2.6`)

- **Goal:** Complete the last remaining Experiment panel-management behavior in `crates/app/src/shell/panel_windows.rs` without changing `crates/app/panels.snapshot` or breaking existing `panel_windows` tests.
- **Pre-check:**
  ```bash
  grep -n "floating_pos\|stack_parent\|snap" /var/tmp/firecraft-int-1.0/crates/app/src/shell/panel_windows.rs | head -n 30
  ```
- **Steps:**
  1. Read `/nas/Temp/repos/FireCraft-CraftAnything-Experiment/app/src/panels/mod.rs` (read-only) for drag ghost preview, vertical stack snapping (`stack_parent`), and viewport/dock edge snapping constants.
  2. Implement clean-room equivalents in `crates/app/src/shell/panel_windows.rs` (zero `unwrap`/`expect`/`panic`/`unsafe`):
     - Translucent rounded drag ghost overlay while dragging a floating group header.
     - Vertical stack attach/detach (`stack_parent` follows parent drag delta).
     - Edge snapping to left tools rail, right dock edge, top toolbar, and bottom inspector dock (within 12 px snap radius).
  3. Add unit tests covering snap geometry, vertical stack chaining, and cycle prevention in `panel_windows.rs`.
  4. Capture a real `xvfb-run` screenshot with a floating panel group visible and inspect it visually.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot'
  ```

---

### `L-A4` `[LUNA]` Build In-App Feedback Dialog UI (`P8.2` + `P8.7`)

- **Goal:** Wire the already-tested `crates/app/src/diagnostic.rs` (`firecraft-diagnostic/v1` PII scrubber + zero-token prefilled GitHub Issue URL builder) into a real `Help ▸ Send Feedback / Report Bug...` modal dialog usable on both native desktop and WASM.
- **Pre-check:**
  ```bash
  grep -n "DiagnosticReport\|prefilled_issue_url" /var/tmp/firecraft-int-1.0/crates/app/src/ -r
  ```
- **Steps:**
  1. Add a clean modal state struct (`FeedbackDialogState`) in `crates/app/src/shell/dialogs.rs` (or `feedback_dialog.rs`) with:
     - Report type selector (`Bug Report`, `Feature Request`, `UX / Parity Issue`).
     - Summary & reproduction steps text inputs.
     - Explicit **opt-in** checkboxes (default OFF for optional diagnostics; NEVER include document artwork, paths, or usernames).
     - Live read-only preview of the exact scrubbed `firecraft-diagnostic/v1` payload that will be copied/submitted.
     - Actions: `Copy Report JSON`, `Save Report (.json)` (native), and `Open Prefilled GitHub Issue` (opens canonical `https://github.com/aliasfoxkde/FireCraft/issues/new?...` URL via `ctx.open_url`).
  2. Wire `Help ▸ Send Feedback / Report Bug...` in `crates/app/src/menus.rs` and `crates/app/src/shell/chrome.rs` (keep `menus::tests` pinned counts reconciled).
  3. Add unit tests verifying scrubbing, empty-summary validation, and opt-in defaults.
  4. Verify both native (`cargo test -p app`) and `wasm32-unknown-unknown` (`cargo check --target wasm32-unknown-unknown -p firecraft-web`) pass, and capture + inspect a screenshot of the open modal!
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo check --target wasm32-unknown-unknown -p firecraft-web && cargo run -p xtask -- aegis'
  ```

---

### `L-A5` `[LUNA]` Upgrade Command Palette Discovery (`PRODUCT-UX-ROADMAP` Phase 2)

- **Goal:** Upgrade `Ctrl+P` Command Palette from raw ID-only subsequence matching to user-language search over readable names, descriptions, aliases (e.g., `"make text bold"`, `"export transparent png"`, `"show layers"`), categories, shortcuts, and recent/favorite commands.
- **Pre-check:**
  ```bash
  grep -rn "palette" /var/tmp/firecraft-int-1.0/crates/app/src/shell/ | head -n 30
  ```
- **Steps:**
  1. Inspect `crates/app/src/shell/palette.rs` (or wherever `CommandPalette` lives) and `crates/engine/src/catalog/`.
  2. Add a deterministic metadata/alias lookup table (`title`, `category`, `description`, `aliases`, `shortcut`) over `engine::catalog::ids()` without removing exact command-ID matching.
  3. Add recent-commands MRU (capped at 10) shown as empty-state suggestions when query is empty, plus keyboard Up/Down/Enter/Esc navigation.
  4. Add unit tests proving user-language queries rank the expected command first and exact ID queries still hit exact matches.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- reachability'
  ```

---

### `L-A6` `[LUNA]` Persisted `File ▸ Open Recent`, Favorites/Pinned & Launcher CLI Args (`T30.1` + `T30.2`)

- **Goal:** Replace the placeholder `File ▸ Open Recent` flow with a real persisted MRU + Pinned/Favorites list (wired to both `File ▸ Open Recent` and the Welcome Screen's Recent Files list!) plus stable CLI launch flags (`--new`, `--open <path>`).
- **Pre-check:**
  ```bash
  grep -rn "recent_files\|Open Recent" /var/tmp/firecraft-int-1.0/crates/app/src/
  ```
- **Steps:**
  1. Create a versioned JSON persistence model (`schema_version: 1`, `recent: Vec<RecentEntry>`, `pinned: Vec<RecentEntry>`, capped at 12 recent / 12 pinned, storing path + display name + timestamp — NEVER document contents).
  2. Wire record-on-open/save, pin/unpin toggle, `Clear Recent`, and missing-file graceful warning into `Shell`, `File ▸ Open Recent` menu, and `welcome_overlay.rs`.
  3. Add platform-neutral CLI argument handling in `crates/app/src/main.rs` (`--new`, `--open <path>`, Unicode/space-safe path args, dirty-document prompt protection).
  4. Add unit tests for deduplication, MRU ordering, cap enforcement, pin/unpin, stale/missing file handling, and schema round-trip.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo check --target wasm32-unknown-unknown -p firecraft-web'
  ```

---

### `L-A7` `[LUNA]` Responsive/Theme/A11y Screenshot Suite & Fresh Windows Review Build (`P2.11` + `T29` Gate D)

- **Goal:** Capture and visually verify dated screenshots across `1440x900` (Dark + Light), `960x640` (minimum viewport — zero tool footer clipping), Welcome modal, and Feedback modal; rebuild `dist/windows-review/firecraft.exe` on `ssh t5500` and record exact SHA-256.
- **Pre-check:**
  ```bash
  which xvfb-run import && ls -lh /nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe
  ```
- **Steps:**
  1. Build Linux release binary on `fedora`, fetch to `/var/tmp/firecraft-linux-check/firecraft`, and capture `1440x900` (Dark, Light, Welcome) and `960x640` screenshots under `xvfb-run`.
  2. Visually inspect EVERY screenshot with your image viewer tool; verify WCAG contrast, zero clipped widgets, readable text, crisp icons.
  3. Refresh `docs/img/app-main.png` and `site/assets/app-main.png` with the best `1440x900` Dark Studio capture.
  4. Push verified branch to `gitforge` then `origin`, build `firecraft.exe` on `ssh t5500`, copy to `/nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe`, verify PE GUI subsystem (`file` / `objdump`), and record exact byte size + `sha256sum` in `PROGRESS.md`.
- **Verify:**
  ```bash
  file /nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe && sha256sum /nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe
  ```

---

### `L-A8` `[HUMAN GATE]` Maintainer Phase A Visual & Windows 11 Functional Sign-Off (`P2.12`)

- **Goal:** Stop and present Phase A screenshots, WASM Playwright results, and the fresh Windows review binary (`dist/windows-review/firecraft.exe` + commit SHA + SHA-256) for maintainer launch/edit verification on Windows 11 before advancing to Phase B.
- **Exit:** Maintainer approves Phase A and authorizes Phase B.
