# Android feasibility — T27 RP-04 spike

Date: 2026-11-06 · Baseline: `5fac7a2` + T27 commits (worktree `t27-release`)
Method: Cargo.lock pins + vendored crate sources in the local cargo
registry. No claim below rests on recall; every version is read from
this workspace's lockfile.

## Pinned versions (Cargo.lock)

| Crate | Pinned | Android relevance |
|---|---|---|
| eframe | 0.36.2 | ships `android-game-activity` / `android-native-activity` features (eframe-0.36.2/Cargo.toml:58–59) |
| egui | 0.36.2 | renderer-agnostic |
| winit | 0.30.13 | `platform/android.rs` present; depends on `android-activity` |
| android-activity | 0.6.1 | already in the graph (via winit); both NativeActivity and GameActivity backends |
| glow | 0.17.0 | the app's chosen renderer |
| glutin | 0.32.3 | declares `aarch64-linux-android` target support; `cfg(target_os = "android")` → `glutin_egl_sys` (glutin-0.32.3/Cargo.toml:42,166) |
| glutin_egl_sys | 0.7.1 | EGL on Android compiled in (lib.rs:4) |
| boa_engine | 0.20.0 | pure-Rust JS engine; no JIT/native-codegen dependency that would fight W^X on Android — interprets |
| wgpu | 30.0.1 | in the graph but **not used by the app** (see renderer below) |

## Renderer

`crates/app/Cargo.toml:28–33`: eframe with `default-features = false`,
features `default_fonts, glow, persistence, wayland, x11`. The shell is
**glow-backed** (OpenGL ES via glutin/EGL), not wgpu. OpenGL ES 3 is
universally available on Android devices that could run this app, and
glutin 0.32.3's EGL path explicitly targets Android. The web crate
(web/Cargo.toml) builds its own eframe feature set separately — the
desktop feature set is independent.

## What the stack already provides

1. **Entrypoint exists upstream**: eframe 0.36.2 `NativeOptions.android_app`
   (`epi.rs:401–402`) and `run.rs:31–41` wire winit's
   `with_android_app`. A new `crates/app/src/main.rs` android path
   (cfg-gated) plus a small `android` crate with `#[android_main]` is
   the standard pattern; no eframe fork needed.
2. **Activity choice is forced**: eframe-0.36.2/src/lib.rs:147–154
   `compile_error!`s if `accesskit` + `android-native-activity` combine
   — the app enables accesskit, so **GameActivity is required**.
   GameActivity needs the AndroidX `games-activity` AAR glued into the
   Gradle project (android-activity 0.6.1 ships the sources under
   `android-games-sdk/`).
3. **Persistence**: eframe `persistence` writes app state via its
   storage abstraction; on Android the `AndroidApp` handle provides
   `internal_data_path()` — the storage dir must be seeded from the
   app handle (small cfg'd change where the app builds `NativeOptions`).
4. **IO plane is clean**: `crates/io` is byte-buffer in/out, no
   `std::fs` (io/src/lib.rs:3). Filesystem touchpoints are the shell
   (one `std::fs::write` in shell/mod.rs:634) and `app-cli` (desktop
   only, stays native). Document open/save on Android must route
   through SAF (Storage Access Framework) via JNI — the engine and
   format crates need **zero changes**.

## Blockers / bounded work

| Item | Severity | Scope |
|---|---|---|
| No `#[android_main]` entrypoint in workspace | work | new `crates/app-android` (~100 lines) + Gradle/NDK scaffold (cargo-apk or xtask recipe) |
| File dialogs are desktop-only patterns | work | SAF picker over JNI; shell command routing already byte-oriented, so it's an adapter |
| Touch input | work | egui/winit 0.30 deliver touch as pointer events; canvas gesture layer (pinch-zoom, two-finger pan) is new UI code, not engine code |
| GameActivity AAR glue + NDK build recipe | work | cargo-ndk + Gradle template; CI needs an Android SDK image (currently absent) |
| accesskit+NativeActivity conflict | constraint | solved by choosing GameActivity (see above) |
| x11/wayland features in app's eframe set | trivial | feature set is already `default-features = false`; add `android-game-activity` per-target; the x11/wayland features no-op off-Android-targets |

No hard blocker found in the pinned dependency graph: every crate on
the app's critical path either supports `aarch64-linux-android`
explicitly (glutin, winit→android-activity) or is platform-agnostic
pure Rust (engine, doc, compose, io, filters, scripting/boa).

## Toolchain

- Target: `aarch64-linux-android` (armv8a; `armv7-linux-androideabi`
  unnecessary — the app's memory profile is desktop-class).
- NDK r26+ with `cargo-ndk` or `cargo-apk`; minimum API level 24
  (GameActivity floor per AndroidX games-activity; also the floor
  where EGL 3 + `AHardwareBuffer` paths settle).
- CI: needs a runner image with Android SDK/NDK — the GitForge image
  has none today (see ARTIFACT-MATRIX toolchain inventory).

## Verdict

**Feasible with bounded work** — approximately: entrypoint + Gradle
scaffold (1–2 days), SAF file adapter (1–2 days), touch/gesture pass
on the canvas (2–4 days), CI image + build recipe (1 day). No engine
or format changes. The gating risk is not the stack, it is verification:
the no-untested-claims rule means an APK is not "supported" until it
installs and passes open/edit/export on a real ARM64 device, which this
fleet does not have. Recommended disposition: keep Android
**experimental**; build the APK once an Android SDK runner + device
test path exist, before any support claim.
