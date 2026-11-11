# T27-46 — Re-read the Android feasibility report and record the gate decision

> **RP:** RP-04 · **Priority:** P2 · **Status:** READY
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-04

**Objective:** re-read the Android feasibility report against the current dependency
graph and record a written approve-or-defer decision, so the port is either
authorized or explicitly declined on the record.

**Inputs:** `docs/research/android-feasibility.md`; the current `Cargo.lock`
(eframe 0.36.2, winit 0.30.13, android-activity 0.6.1, glow 0.17.0, glutin 0.32.3,
boa_engine 0.20.0); `docs/decisions/`.

**Authority:** `docs/tasks/T27-cross-platform-release.md` — "Android is either verified
as a real native app on an ARM64 device or explicitly deferred with a feasibility
report"; the plan's non-goal that a native Android claim must not be made for a web
wrapper.

**Acceptance criteria:** the report is re-read against the current dependency graph and
the `accesskit` + `android-native-activity` conflict is re-confirmed or resolved; a
written approve/defer decision is recorded with its date, its rationale and its
conditions.

**Verification:** `test -f docs/research/android-feasibility.md`

**Evidence:** the report concludes "feasible with bounded work" — the GPU stack is
glow-backed (EGL), not wgpu, and `accesskit` with `android-native-activity` forces
GameActivity. Its disposition is "experimental"; it is a feasibility finding, not a
gate decision.

**Blockers:** none — this row can start now.

**Stopping condition:** a decision record exists, and T27-47 is unblocked (approve) or
formally closed (defer).
