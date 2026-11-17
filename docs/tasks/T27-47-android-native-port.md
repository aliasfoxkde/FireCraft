# T27-47 — Implement and device-test the native Android port (only if the gate approves)

> **RP:** RP-04 · **Priority:** P2 · **Status:** DEFERRED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-46

**Objective:** build a real native Android application — a native entrypoint, not a
web wrapper — and prove it on a physical ARM64 device.

**Inputs:** the gate decision from T27-46; the Android NDK and SDK; an `arm64-v8a`
build configuration; a physical ARM64 device with `adb`.

**Authority:** `docs/tasks/T27-cross-platform-release.md` — Android is verified as a
real native app on an ARM64 device or explicitly deferred; the non-goal that a native
Android claim must not be made for a web wrapper.

**Acceptance criteria:** a native entrypoint exists; lifecycle handling, the SAF
document picker and touch mapping are implemented; an `arm64-v8a` APK installs on a
physical ARM64 device and passes import → edit → export.

**Verification:** `adb install <apk> && adb shell am start -n dev.firecraft.FireCraft/.MainActivity`

**Evidence:** Closed with the deferral: [decision 0121](../decisions/0121-the-android-gate-defers-the-native-port.md) — the stack is sound (no crate on the critical path blocks `aarch64-linux-android`), but this fleet has no NDK/SDK CI image and no physical ARM64 device, so the port cannot be exercised and no support is claimed **Done:** 2026-11-16: deferred by decision 0121.

**Blockers:** an Android SDK/NDK CI image; a physical ARM64 device reachable with `adb`

**Stopping condition:** an APK installs and passes the round trip on a physical ARM64
device, or the row is closed with the deferral decision recorded.
