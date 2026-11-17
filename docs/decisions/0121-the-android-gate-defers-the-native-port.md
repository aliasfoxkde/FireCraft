# 0121 — The Android gate defers the native port and keeps Android experimental

Date: 2026-11-16 · Tickets: [T27-46](../tasks/T27-46-android-gate-decision.md),
[T27-47](../tasks/T27-47-android-native-port.md) ·
Status: accepted

## Context

The Android spike
([android-feasibility.md](../research/android-feasibility.md), dated
2026-11-06) concluded that a native Android build is **feasible with
bounded work**: no crate on the app's critical path blocks
`aarch64-linux-android`, and no engine or format change is needed.
What it could not conclude was that the port is *verifiable* here.

[T27-46](../tasks/T27-46-android-gate-decision.md) asks for the
report to be re-read against the dependency graph as it stands now and
for the gate decision to be written down, so the port is either
authorized or explicitly declined on the record. The row's stopping
condition accepts either: approve (and unblock
[T27-47](../tasks/T27-47-android-native-port.md)) or defer (and close
it with the deferral recorded).

The re-read, against `Cargo.lock` and the vendored crate sources on
this host:

| Crate | Pinned | The report said | Now |
|---|---|---|---|
| eframe | 0.36.2 | 0.36.2 | unchanged |
| egui | 0.36.2 | 0.36.2 | unchanged |
| winit | 0.30.13 | 0.30.13 | unchanged |
| android-activity | 0.6.1 | 0.6.1 | unchanged |
| glow | 0.17.0 | 0.17.0 | unchanged |
| glutin | 0.32.3 | 0.32.3 | unchanged |
| glutin_egl_sys | 0.7.1 | 0.7.1 | unchanged |
| boa_engine | 0.20.0 | 0.20.0 | unchanged |
| wgpu | 30.0.1 | 30.0.1 | unchanged |
| accesskit | 0.24.1 | — | unchanged |

The one constraint the report named is re-confirmed in the vendored
source: `eframe-0.36.2/src/lib.rs:148–153` carries

```rust
#[cfg(all(
    target_os = "android",
    feature = "accesskit",
    feature = "android-native-activity"
))]
compile_error!("`accesskit` feature is only available with `android-game-activity`");
```

and the app enables `accesskit` (`crates/app/Cargo.toml`, eframe
features `default-features = false` with `default_fonts`, `glow`,
`persistence`, `wayland`, `x11`, `accesskit`). So GameActivity stays
mandatory, `android-game-activity` stays out of the lockfile, and the
conflict is not resolvable by a dependency bump that the lockfile
offers today.

Nothing in the stack blocks the port. What blocks the *claim* is the
fleet: there is no NDK/SDK CI image and no physical ARM64 device with
`adb` on it. The rule this repository works under is that a platform is
supported only when it has been exercised on that platform; an APK that
builds but has never been installed proves nothing.

## Decision

- **Defer the native Android port.** The gate does not approve the
  port for this release train, and T27-47 is closed with this
  deferral rather than left blocked on a decision that has now been
  made.
- **Keep Android experimental in the artifact matrix.** The row stays
  `DEFERRED`, now citing this decision instead of an unrecorded gate.
  Android is not a *required* target for RP-08, so the certification
  report must not list it among skipped required targets.
- **The deferral is a scheduling decision, not a feasibility verdict.**
  The stack is sound; the verification path is what is missing. The
  work reopens when **both** of these exist: an Android SDK/NDK CI
  image, and a physical ARM64 device reachable with `adb`.
- **The report stays the authority for the shape of the work.** The
  entrypoint, the SAF document adapter, the touch/gesture pass and the
  Gradle/NDK recipe are as the report describes them; nothing in this
  decision changes that scope.

## Consequences

- The board carries an answer instead of a hole: T27-46 is
  `IMPLEMENTED` (this record exists and the re-read is recorded) and
  T27-47 is `DEFERRED` with the conditions named.
- No Android support is claimed anywhere. The web build for Android's
  browser remains what it is — a web build, not a native app — and the
  non-goal that a native Android claim must not be made for a web
  wrapper still holds.
- The conditions are concrete enough to check: the CI image inventory
  and the device list are both named, so the row reopens on evidence
  rather than on intent.
- When the port does resume, the GameActivity constraint and the
  verification plan are already written down, so the work starts from
  the spike rather than from a re-derivation.
