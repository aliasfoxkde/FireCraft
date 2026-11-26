# Phase 3 — Core UX Features (Drag Ghost, Feedback Modal, Command Palette, Open Recent & Windows Review)

> **Goal:** Complete the high-value interactive desktop & web UX features from `T29`, `T30.1–T30.2`, `P8.2`, and `PRODUCT-UX-ROADMAP.md` Phase 2, then deliver a freshly verified Windows review build for human Windows 11 sign-off.

---

### `L3.1` `[LUNA]` Floating Panel Drag Ghost Preview (`P2.6` Part 1)

- **Files:** `crates/app/src/shell/panel_windows.rs`
- **Pre-check:**
  ```bash
  grep -n "floating_pos\|floating_width" /nas/Temp/repos/FireCraft/crates/app/src/shell/panel_windows.rs
  ```
- **Do:**
  1. Reference `/nas/Temp/repos/FireCraft-CraftAnything-Experiment/app/src/panels/mod.rs` (read-only) for drag ghost geometry and translucency.
  2. While dragging a floating panel group title bar, draw a translucent preview rectangle (`Order::Tooltip`) tracking pointer delta cleanly without jitter.
  3. Commit the new `floating_pos` on pointer release, clamped within viewport bounds.
  4. Add unit test for viewport clamping on release.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app panel_windows && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Floating panel drag shows a live translucent ghost and snaps cleanly inside the viewport on release.

---

### `L3.2` `[LUNA]` Floating Panel Vertical Stack Grouping & Edge Snapping (`P2.6` Part 2)

- **Files:** `crates/app/src/shell/panel_windows.rs`
- **Pre-check:**
  ```bash
  grep -n "stack_parent" /nas/Temp/repos/FireCraft/crates/app/src/shell/panel_windows.rs
  ```
- **Do:**
  1. Implement pure helper `compute_snap_position(candidate: Rect, viewport: Rect, others: &[Rect], snap_px: f32) -> (Pos2, Option<u64>)` snapping within `12.0 px` to viewport/dock edges and to the bottom edge of another floating group.
  2. When attached via `stack_parent`, propagate parent drag deltas to attached children and prevent cycles.
  3. Add unit tests covering edge snap, vertical stack attach/detach, and cycle rejection.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot'
  ```
- **Done when:** Edge snapping and vertical stack chaining pass unit tests without disturbing `panels.snapshot`.

---

### `L3.3` `[LUNA]` Build In-App Feedback Dialog UI Over `diagnostic.rs` (`P8.2` + `P8.7`)

- **Files:** `crates/app/src/shell/dialogs.rs` (or `crates/app/src/shell/feedback_dialog.rs`), `crates/app/src/shell/chrome.rs`, `crates/app/src/menus.rs`
- **Pre-check:**
  ```bash
  grep -n "pub struct DiagnosticReport\|prefilled_issue_url" /nas/Temp/repos/FireCraft/crates/app/src/diagnostic.rs
  ```
- **Do:**
  1. Build a clean modal (`Help ▸ Send Feedback / Report Bug...`) with report kind (`Bug`, `Feature`, `UX/Parity`), Summary input, Repro Steps input, and explicit opt-in checkboxes (default OFF; NEVER include document content/paths/usernames).
  2. Display a live read-only scrubbed preview rendered from `DiagnosticReport::new(...)`.
  3. Wire 3 actions: `Copy Scrubbed JSON`, `Save Report (.json)` (native), and `Open Prefilled GitHub Issue` (`ctx.open_url`).
  4. Add unit tests verifying default-off opt-ins, empty-summary guard, and WASM compilation (`cargo check --target wasm32-unknown-unknown -p firecraft-web`).
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo check --target wasm32-unknown-unknown -p firecraft-web && cargo run -p xtask -- aegis'
  ```
- **Done when:** Feedback modal opens cleanly on native & WASM, previews scrubbed payload, and opens the prefilled GitHub URL.

---

### `L3.4` `[LUNA]` Upgrade Command Palette (`Ctrl+P`) With User-Language Aliases & MRU

- **Files:** `crates/app/src/shell/palette.rs` (check actual filename in pre-check), `crates/app/src/shell/mod.rs`
- **Pre-check:**
  ```bash
  grep -rln "palette_open\|toggle_palette" /nas/Temp/repos/FireCraft/crates/app/src/shell/
  ```
- **Do:**
  1. Add a metadata lookup mapping `engine::catalog::ids()` to readable title, category, short description, user-language aliases (e.g., `"make text bold"`, `"export transparent png"`, `"show layers"`), and keybinding string.
  2. Rank exact ID matches first, then title/alias matches, then subsequence matches; show recent commands (MRU cap 10) when query is empty.
  3. Add unit tests proving user-language queries hit the right command ID and exact command IDs still match 100%.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- reachability'
  ```
- **Done when:** `Ctrl+P` searches by natural-language aliases + descriptions + shortcuts while preserving 257/257 reachability.

---

### `L3.5` `[LUNA]` Persisted Recent & Pinned Document Model (`T30.1` Model)

- **Files:** `crates/app/src/recent.rs` (new), `crates/app/src/lib.rs`
- **Pre-check:**
  ```bash
  grep -rn "recent_files" /nas/Temp/repos/FireCraft/crates/app/src/
  ```
- **Do:**
  1. Create `crates/app/src/recent.rs` implementing a versioned JSON schema (`schema_version: 1`, `recent: Vec<RecentDoc>`, `pinned: Vec<RecentDoc>`, cap 12 each, storing path + display name + timestamp only — NEVER document bytes).
  2. Implement `record_open`, `toggle_pin`, `clear_recent`, `remove_missing`, and safe load/save via platform config path (no-op / `localStorage` hook on WASM).
  3. Add unit tests for deduplication, MRU ordering, cap trimming, pin preservation across `clear_recent`, and corrupted JSON graceful fallback.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app recent && cargo clippy -p app --all-targets -- -D warnings && cargo check --target wasm32-unknown-unknown -p firecraft-web'
  ```
- **Done when:** `recent.rs` passes all unit tests on native and compiles clean on `wasm32`.

---

### `L3.6` `[LUNA]` Wire `File ▸ Open Recent`, Welcome Recent List & Launcher CLI Args (`T30.1–T30.2`)

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/welcome_overlay.rs`, `crates/app/src/shell/mod.rs`, `crates/app/src/main.rs`
- **Pre-check:**
  ```bash
  grep -n "Open Recent\|--no-welcome" /nas/Temp/repos/FireCraft/crates/app/src/main.rs /nas/Temp/repos/FireCraft/crates/app/src/shell/chrome.rs
  ```
- **Do:**
  1. Wire `File ▸ Open Recent` submenu to list pinned docs, recent docs, and `Clear Recent`; show a graceful status banner if a clicked path no longer exists (and offer removal).
  2. Wire `welcome_overlay.rs` to show real recent/pinned files when present (falling back cleanly to Showcase samples when empty).
  3. Support `--new` and `--open <path>` (plus positional `<path>` with spaces/Unicode) in `crates/app/src/main.rs`.
  4. Add CLI argument parsing unit tests in `main.rs` (or a small helper module).
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- aegis'
  ```
- **Done when:** `Open Recent`, Welcome Screen, and `--open`/`--new` CLI flags all work end-to-end with tests.

---

### `L3.7` `[LUNA]` Capture Phase 3 Screenshot Suite & Build Fresh Windows Review Binary (`P2.11` + `T29` Gate D)

- **Files:** `dist/windows-review/firecraft.exe` (untracked review artifact), `docs/planning/luna/PROGRESS.md`
- **Pre-check:**
  ```bash
  git -C /nas/Temp/repos/FireCraft status -s
  ```
- **Do:**
  1. Push verified branch to `gitforge` and `origin`.
  2. On `ssh t5500`, build BOTH Windows `firecraft.exe` (`x86_64-pc-windows-gnu`) and Debian-glibc Linux `firecraft`.
  3. Copy `firecraft.exe` to `/nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe`; verify PE32+ GUI subsystem (`file`) and record exact byte size + `sha256sum`.
  4. Run `xvfb-run` locally to capture `1440x900` (Dark, Light, Welcome, Feedback modal) and `960x640`, and visually inspect every PNG.
- **Verify:**
  ```bash
  file /nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe && sha256sum /nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe
  ```
- **Done when:** Fresh GUI `firecraft.exe` + all Phase 3 screenshots are visually verified and logged in `PROGRESS.md`.

---

### `L3.8` `[HUMAN GATE]` Maintainer Windows 11 & Phase 3 UX Sign-Off (`P2.12`)

- **Do:** Stop and present the Phase 3 screenshots + `dist/windows-review/firecraft.exe` path, commit SHA, and SHA-256 for maintainer Windows 11 launch/open/edit/save/export review before starting Phase 4.
- **Done when:** Maintainer approves Phase 3 and clears Phase 4.
