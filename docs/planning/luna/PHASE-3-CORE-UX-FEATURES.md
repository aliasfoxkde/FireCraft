# Phase 3 — Core UX Features, Full Experiment `v0.3.0` Feature Parity & Windows Review

> **Goal:** Complete every high-value desktop & web UX feature from the 14-domain Experiment `v0.3.0` audit ([`2026-11-26-experiment-deep-audit.md`](../../integration/2026-11-26-experiment-deep-audit.md)), `T29`, `T30.1–T30.2`, `P8.2`, and `PRODUCT-UX-ROADMAP.md` Phase 2, then deliver a freshly verified Windows review build for human Windows 11 sign-off.
> **Clean-room rule:** Zero `unsafe`, zero `unwrap`/`expect`/`panic!` in non-test code, zero Adobe trademark strings.

---

### `L3.1` `[LUNA]` Floating Panel Windows, Full Translucent Drag Ghost, 1-Click Collapse Chevron & `»` Tab Overflow (`P2.6` Part 1)

- **Files:** `crates/app/src/shell/panel_windows.rs`, `crates/app/src/shell/mod.rs`
- **Experiment Ref (read-only):** `app/src/layout.rs:354-543,821-891`, `app/src/panels/mod.rs:86-173,1022-1377`
- **Pre-check:**
  ```bash
  grep -n "floating_pos\|floating_width\|stack_parent" crates/app/src/shell/panel_windows.rs
  ```
- **Do:**
  1. Wire floating `PanelGroup` rendering via `egui::Area` (`Order::Middle`) with movable title bars, Undock/Pin toggle, and Panel Options menu (`Expand/Collapse`, `Undock to Floating`, `Pin to Primary/Inner-Right/Left Dock`, `Ungroup Active Tab Below`, `Group With…`, `Collapse/Expand All`).
  2. Draw Experiment's translucent full-panel drag ghost facsimile (`draw_widget_drag_ghost` at `Order::Tooltip`: header, tabs, mock rows, drop hint) while dragging a tab or whole panel group, committing new `floating_pos` clamped to viewport bounds on release.
  3. Apply Experiment's 1-click collapse chevron fix (register background double-click BEFORE chevron or hit-test `interact_pointer_pos()` against chevron rect so 1st click ALWAYS toggles!), add per-group bottom resize splitter (`72..520 px`), always-visible scrollbars, and `»` tab overflow menu when a dock column narrows.
  4. Add unit tests for viewport clamping, 1-click collapse chevron hit-testing, and tab overflow.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app panel_windows && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Floating panels render with translucent drag ghosts, 1-click collapse chevrons, resize splitters, and `»` tab overflow.

---

### `L3.2` `[LUNA]` Pure Panel Geometry Module (`geometry.rs`), Magnetic Snap/Stacking, Grip Bars & 6 Workspace Presets (`P2.6` Part 2)

- **Files:** `crates/app/src/shell/panel_geometry.rs` (new), `crates/app/src/shell/panel_windows.rs`, `crates/app/src/shell/chrome.rs`
- **Experiment Ref (read-only):** `app/src/panels/geometry.rs:1-188`, `app/src/layout.rs:166-279,916-1205,1235-1427`
- **Pre-check:**
  ```bash
  grep -n "apply_workspace_preset" crates/app/src/shell/chrome.rs
  ```
- **Do:**
  1. Port Experiment's pure `panels/geometry.rs` math (`SNAP_THRESHOLD = 12.0`, `COLUMN_SNAP = 28.0`, `snap_pos`, `stacked_below`, `shifted_for_window_above`, `group_height`) + all 7 unit tests into `crates/app/src/shell/panel_geometry.rs`.
  2. Wire `stack_parent` vertical window stacking (dragging root moves whole column; `Detach from Window Stack` splits cleanly; reject cycles) and draggable dotted grip bars on Tools & Panels docks (drag to float or snap Left/Right; double-click to collapse; single state-aware `▾`/`▸` dock collapse-all toggle).
  3. Add all 6 CS6 workspace presets: `Expanded Mode`, `CS6 Dual-Column` (`RightOuter` + `RightInner`), `All Panels Floating`, `Iconic Mode` (36 px strip + icon-anchored flyouts), `Iconic Mode With Panel Names` (94 px), and `Netbook Mode`.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot'
  ```
- **Done when:** Magnetic edge snapping, vertical stacking, floating Tools/Panels grip bars, and all 6 presets work with `panels.snapshot` intact.

---

### `L3.3` `[LUNA]` Build In-App Feedback Dialog UI Over `diagnostic.rs` (`P8.2` + `P8.7`)

- **Files:** `crates/app/src/shell/dialogs.rs`, `crates/app/src/shell/chrome.rs`, `crates/app/src/menus.rs`
- **Pre-check:**
  ```bash
  grep -n "pub struct DiagnosticReport\|prefilled_issue_url" crates/app/src/diagnostic.rs
  ```
- **Do:**
  1. Build a clean modal (`Help ▸ Send Feedback / Report Bug...`) using standardized dialog header/footer styling, with report kind (`Bug`, `Feature`, `UX/Parity`), Summary input, Repro Steps input, and explicit opt-in checkboxes (default OFF; NEVER include document content/paths/usernames).
  2. Display a live read-only scrubbed preview rendered from `DiagnosticReport::new(...)`.
  3. Wire 3 actions: `Copy Scrubbed JSON`, `Save Report (.json)` (native), and `Open Prefilled GitHub Issue` (`ctx.open_url`).
  4. Add unit tests verifying default-off opt-ins, empty-summary guard, and WASM compilation.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo check --target wasm32-unknown-unknown -p firecraft-web && cargo run -p xtask -- aegis'
  ```
- **Done when:** Feedback modal opens cleanly on native & WASM, previews scrubbed payload, and opens the prefilled GitHub URL.

---

### `L3.4` `[LUNA]` Supercharge Command Palette (`Ctrl+K` / `Ctrl+P`) With 7 Categories, Object Search, Inline Replace & MRU

- **Files:** `crates/app/src/shell/palette.rs`, `crates/app/src/shell/mod.rs`
- **Experiment Ref (read-only):** `app/src/dialogs/mod.rs:579-972`
- **Pre-check:**
  ```bash
  grep -rln "palette_open\|toggle_palette" crates/app/src/shell/
  ```
- **Do:**
  1. Expand `palette.rs` to search all 7 Experiment categories alongside FireCraft's `257` catalog commands (with user-language aliases & descriptions + MRU top-10 when query is empty):
     - `[Replace]` inline `replace:old->new`
     - `[Object]` live canvas object search (`find:<text>` or direct match jumping & selecting on canvas)
     - `[Panel]` all CS6 panels (`Open / Focus <Panel>`)
     - `[Layout]` workspace presets + Reset Workspace + Full Screen
     - `[Filter]` live & bitmap filters
     - `[Widget]` Common Library UI components
     - `[Cmd]` `engine::catalog::ids()` with human title, aliases, category, and keybinding
  2. Support Up/Down/Enter/Esc keyboard selection with category badges.
  3. Add unit tests proving user-language aliases, `find:` object jump, `replace:a->b`, and 100% exact catalog ID reachability.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- reachability'
  ```
- **Done when:** `Ctrl+K` / `Ctrl+P` searches commands, aliases, panels, filters, widgets, canvas objects, and inline replace while preserving 257/257 reachability.

---

### `L3.5` `[LUNA]` Persisted Recent & Pinned Document Model (`T30.1` Model)

- **Files:** `crates/app/src/recent.rs` (new), `crates/app/src/lib.rs`
- **Pre-check:**
  ```bash
  grep -rn "recent_files" crates/app/src/
  ```
- **Do:**
  1. Create `crates/app/src/recent.rs` implementing a versioned JSON schema (`schema_version: 1`, `recent: Vec<RecentDoc>`, `pinned: Vec<RecentDoc>`, cap 12 each, storing path + display name + timestamp only — NEVER document bytes).
  2. Implement `record_open`, `toggle_pin`, `clear_recent`, `remove_missing`, and safe load/save via platform config path (no-op / `localStorage` hook on WASM).
  3. Add unit tests for deduplication, MRU ordering, cap trimming, pin preservation across `clear_recent`, and corrupted JSON graceful fallback.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app recent && cargo clippy -p app --all-targets -- -D warnings && cargo check --target wasm32-unknown-unknown -p firecraft-web'
  ```
- **Done when:** `recent.rs` passes all unit tests on native and compiles clean on `wasm32`.

---

### `L3.6` `[LUNA]` Wire `File ▸ Open Recent`, `HTML Setup` / `Page Setup` / `Print` / `Preview in Browser`, Welcome Recents & CLI Flags (`T30.1–T30.2`)

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/welcome_overlay.rs`, `crates/app/src/shell/file_operations.rs`, `crates/app/src/shell/mod.rs`, `crates/app/src/main.rs`
- **Experiment Ref (read-only):** `app/src/menus.rs:506-608`, `app/src/main.rs:54-196`
- **Pre-check:**
  ```bash
  grep -n "Open Recent\|--no-welcome\|preview_in_browser" crates/app/src/main.rs crates/app/src/shell/chrome.rs crates/app/src/shell/file_operations.rs
  ```
- **Do:**
  1. Wire `File ▸ Open Recent` submenu (pinned + recent + `Clear Recent`) and `welcome_overlay.rs` real recent/pinned files (graceful banner + removal offer if a path is missing).
  2. Wire `File ▸ Save a Copy…`, `Save All`, `Revert`, `Preview in Browser (F12)`, and `Print…` (launching printable preview in system browser/print flow cleanly without `unsafe`).
  3. Extend `crates/app/src/main.rs` CLI args to support `--new`, `--open <path>`, positional `<path>` (with spaces/Unicode), `--size WxH`, `--about`, `--splash`, and `--state <menu|flyout|find-replace>` (with throwaway `persistence_path` when capturing screenshots so persisted window size never overrides `--size`, per Experiment `main.rs:138-143`).
  4. Add CLI argument parsing unit tests.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- aegis'
  ```
- **Done when:** `Open Recent`, Welcome Screen, `File` browser/print commands, and deterministic CLI flags all work end-to-end with tests.

---

### `L3.7` `[LUNA]` Multi-Monitor Floating OS Canvas Windows, Dirty Bulk-Close Queue & Quit Confirm (`allow_close` Gate)

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/dialogs.rs`, `crates/app/src/shell/mod.rs`
- **Experiment Ref (read-only):** `app/src/layout.rs:37-43,551-661`, `app/src/app/files.rs:206-365`, `app/src/dialogs/mod.rs:1074-1261`
- **Pre-check:**
  ```bash
  grep -n "floating_doc_tabs\|close_requested" crates/app/src/shell/mod.rs
  ```
- **Do:**
  1. Wire the dead `floating_doc_tabs: Vec<usize>` field (`shell/mod.rs:407`): dragging a document tab vertically out of the tab strip (or clicking `⧉` / context menu `Float in New Window (Multi-Monitor)`) detaches that document into a native movable OS window via `ctx.show_viewport_immediate(...)` (gated `#[cfg(not(target_arch = "wasm32"))]`) with Original/Preview toggle, Canvas Footer, and `Dock to Main Window` / `Dock All` buttons.
  2. Implement Experiment's non-destructive bulk-close queue (`pending_close_tabs` sorted descending) so `Close Other`/`Left`/`Right`/`All` closes clean tabs immediately and stops on each dirty tab with `Save / Don't Save / Cancel` (continuing cleanly after Save As).
  3. Intercept OS window close & `Ctrl+Q` when any open tab is dirty using `Unsaved Changes` (`Save All & Quit` walking untitled docs one-by-one, `Discard & Quit` arming `allow_close = true` so the OS close interceptor never loops, `Cancel`).
  4. Add unit tests covering bulk-close queue order, dirty tab prompt pause/resume, and `should_intercept_close()` / `allow_close` gate.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app --features native && cargo check --target wasm32-unknown-unknown -p firecraft-web && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Multi-monitor floating canvas windows work natively, WASM stays clean, and dirty tab/quit close flows never silently discard work or loop.

---

### `L3.8` `[LUNA]` Standardized Dialog Grammar, 4-Mode Find & Replace, `HTML/Page Setup`, Startup Splash, `Check for Updates` & Redesigned `About`

- **Files:** `crates/app/src/shell/dialogs.rs`, `crates/app/src/shell/panels.rs`, `crates/app/src/shell/mod.rs`, `crates/app/src/shell/welcome_overlay.rs`
- **Experiment Ref (read-only):** `app/src/dialogs/mod.rs:20-50,313-506,1263-1548`, `app/src/dialogs/find_replace.rs:1-161`
- **Pre-check:**
  ```bash
  grep -n "show_update_dialog\|show_page_setup_dialog\|FindReplaceMode" crates/app/src/shell/mod.rs crates/app/src/shell/constants.rs
  ```
- **Do:**
  1. Introduce shared `dialog_header`, `primary_button`, and `dialog_footer_rule` helpers across all modal dialogs in `dialogs.rs`.
  2. Wire the dead `show_page_setup_dialog` (`mod.rs:414`) & `HTML Setup` dialogs and the dead `show_update_dialog` (`mod.rs:420`, `Help ▸ Check for Updates…` with current `CARGO_PKG_VERSION`, non-blocking latest GitHub Release check on native, `Open Releases Page`, `Copy Version Info`).
  3. Upgrade **Find & Replace** modal (`Ctrl+F`) to support all 4 `FindReplaceMode` variants (`Text & object names`, `Fill color`, `Stroke color`, `Any color` across all pages including hidden objects) with `Match case`, `Use fill`/`Use stroke`, live match-count dot, `Find Next`, and `Replace All` wired through `engine` commands.
  4. Add optional animated semi-transparent Startup Splash (`--splash` / initial desktop launch, auto-skipped on `--no-welcome` & WASM embed, clean-room FireCraft Studio branding — zero Adobe trademarks) and upgrade `About FireCraft` with 64 px flame logo, links, and striped Workflow Shortcuts grid.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo check --target wasm32-unknown-unknown -p firecraft-web && cargo run -p xtask -- aegis'
  ```
- **Done when:** Zero dead dialog stub fields remain in `shell/mod.rs`, 4-mode Find & Replace works across all pages, and all modals share crisp CS6 visual grammar.

---

### `L3.9` `[LUNA]` `Intelligence` Bottom-Dock Tab, Interactive MCP Console & Headless GUI Click / Frame-Budget Test Harness

- **Files:** `crates/app/src/shell/panels.rs`, `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/dialogs.rs`, `crates/app/tests/gui_harness.rs` (new), `scripts/gui-suite.sh` (new)
- **Experiment Ref (read-only):** `app/src/panels/bodies.rs:1182-1312`, `app/src/dialogs/mod.rs:975-1072`, `app/src/harness.rs:1-1017`, `scripts/gui-suite.sh`
- **Pre-check:**
  ```bash
  ls crates/app/tests/ scripts/
  ```
- **Do:**
  1. Add the **`Intelligence`** tab to the bottom Properties dock (`Properties | Symbol Properties | Intelligence`, shortcut `Ctrl+Alt+I`): OpenAI-compatible endpoint/model/API-key settings (reads `FIRECRAFT_AI_API_KEY` without ever mutating env or logging secrets), non-blocking request with real selection/document JSON context, response viewer, and 1-click `Copy MCP config` — gracefully explains when unconfigured and never fabricates output.
  2. Add an interactive `MCP & Headless CLI Console` modal (`Help ▸ MCP / CLI Console…`) letting users test live JSON-RPC requests against FireCraft's real `app-cli` / `engine` command catalog and copy CLI commands.
  3. Create `crates/app/tests/gui_harness.rs` driving real headless `Shell` frames at `1360×860` and `960×640`: click-level collapse chevron test, `Alt+` menu & `Esc` test, panel float/snap/stack test, 4-mode Find/Replace test, and a **500-object `@ 1920×1080` frame-time budget regression test**, plus `scripts/gui-suite.sh` for 6-state Xvfb capture.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app --features native && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- aegis'
  ```
- **Done when:** `Intelligence` tab, MCP Console, and headless GUI click/frame-budget harness all pass cleanly with zero regressions.

---

### `L3.10` `[LUNA]` Capture Phase 3 6-State Screenshot Suite & Build Fresh Windows Review Binary (`P2.11` + `T29` Gate D)

- **Files:** `dist/windows-review/firecraft.exe` (untracked review artifact), `docs/planning/luna/PROGRESS.md`
- **Pre-check:**
  ```bash
  git status -s
  ```
- **Do:**
  1. Push verified branch to `gitforge` and `origin`.
  2. On `ssh t5500`, build BOTH Windows `firecraft.exe` (`x86_64-pc-windows-gnu`) and Debian-glibc Linux `firecraft`.
  3. Copy `firecraft.exe` to `/nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe`; verify PE32+ GUI subsystem (`file`) and record exact byte size + `sha256sum`.
  4. Run `scripts/gui-suite.sh` under `xvfb-run` to capture `1440×900` (Dark, Light, Welcome, Menu, Flyout, Find & Replace, About, Feedback modal) and `960×640`, and visually inspect every PNG.
- **Verify:**
  ```bash
  file /nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe && sha256sum /nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe
  ```
- **Done when:** Fresh GUI `firecraft.exe` + all Phase 3 screenshots are visually verified and logged in `PROGRESS.md`.

---

### `L3.11` `[HUMAN GATE]` Maintainer Windows 11 & Phase 3 UX Sign-Off (`P2.12`)

- **Do:** Stop and present the Phase 3 screenshot suite + `dist/windows-review/firecraft.exe` path, commit SHA, and SHA-256 for maintainer Windows 11 launch/open/edit/save/export review before starting Phase 4.
- **Done when:** Maintainer approves Phase 3 and clears Phase 4.
