# Handoff: wasm32 Build Fix (2026-11-21)

## Branch

`feat/wasm-cdn-deploy` — 80 commits ahead of `main`'s `b61f198`, 194 files changed.

## What was done

The wasm32 build was failing with 67 compilation errors due to native-only code paths not being gated for the wasm32 (WebAssembly) target. The fix gates all native-only modules, types, and code paths behind `#[cfg(feature = "native")]` and provides wasm32-compatible stubs.

## Files changed

| File | Change |
|------|--------|
| `crates/app/src/shell/mod.rs` | Gate `dialogs`, `file_operations`, `html_preview` modules. Remove cfg gates from struct field declarations. Gate all native-only ShellAction match arms. Add stub types (ExportDialog, CanvasSizeDialog, etc.) and stub Dialogs impl. Replace cfg-on-expressions with `cfg!()` blocks. Gate Editor::open/save calls. |
| `crates/app/src/shell/panels.rs` | Gate DisplayState destructuring. Fix cfg-on-pattern. Gate save_steps call. |
| `crates/app/src/editor.rs` | Add wasm32 stubs for Editor::open and Editor::save. Gate export_library/import_library. |
| `crates/app/src/shell/dialogs.rs` | Gate all app_cli-dependent functions. |
| `crates/app/src/shell/file_operations.rs` | Gate preview_in_browser. |
| `crates/app/src/shell/html_preview.rs` | Gate build_cache. |
| `crates/engine/src/platform.rs` | Add NoOpFs implementation for wasm32 filesystem stub. |

## Verification

- **wasm32 build: PASS (0 errors, 0 warnings)** on rustc 1.100.0 — clean build verified twice with clean target directories.
- **Native build: PENDING** — cannot verify on this machine (build system denies /nas storage). Requires local machine build.

## Native build verification steps (run on local machine)

```bash
cd /nas/Temp/repos/FireCraft
git checkout feat/wasm-cdn-deploy

# Native check (local machine)
cargo check -p app

# Full native build (local machine)
cargo build -p app

# wasm32 check (to confirm fix still works)
HOST_BUILD_LEASE_SKIP=1 cargo build --target wasm32-unknown-unknown -p firecraft-web

# xtask ci gate (requires native rustc that works)
cargo run -p xtask -- ci
```

## Key design decisions

1. **Stub types, not conditional compilation on usage sites:** The struct fields in `Shell` are un-gated — they exist for both native and wasm32. For wasm32, the stub types serve as zero-cost placeholders. This avoids having to sprinkle cfg gates at every usage site.

2. **`cfg!()` over `#[cfg]` for expression-level gating:** Rust doesn't support `#[cfg]` on expressions (only items). Where native-only assignments needed gating (e.g., `self.export_dialog = Some(...)`), they were converted to `if cfg!(feature = "native") { ... }` blocks.

3. **`Editor::open`/`save` have wasm32 stubs:** The wasm entry point calls these directly. The stubs return `EngineError::InvalidState` pointing users to `load_document_bytes` instead. This is intentional — wasm32 documents load through the wasm bridge, not direct file I/O.

4. **No breaking changes to native code path:** All `#[cfg(feature = "native")]` gates are additive. The native build sees the same code as before. The `#[cfg(not(feature = "native"))]` blocks are never compiled in native.

## Pending

- Native build verification (requires local machine)
- xtask ci gate (requires native build)
- Merge to main (requires maintainer review)
