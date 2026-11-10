# 0079 — The web bundle size budget: a shipped-bytes budget, not a falling-only floor

Date: 2026-11-10 · Status: **accepted**

## Decision

The `xtask dist --web` wasm size ceiling is **a budget on the bytes the
shell ships**. It moves in **both** directions, always from a dated
measurement, and the gate's strictness never changes.

This corrects a stale claim. The repository carried two statements that
were wrong about this one budget:

- the failure message in `xtask/src/dist.rs` said *"ceilings only move
  down"*, and
- the generated budget table in `xtask/src/status.rs` filed the wasm
  size row under **"Performance ceilings (falling-only)"**.

Both were true of the **twelve timing and memory budgets** in
`xtask/src/perf.rs` — those measure a property of the host and stay
falling-only. Neither was true of the wasm size budget, which measures
the size of the shipped artifact. The repository had already re-pinned
the wasm ceiling **upward** on 2026-11-03 (`docs/GAPS.md`, the 19b.4
observation: 14,000,000 → 21,500,000), so the prose contradicted the
repository's own history. The two rules are now stated separately:

| Budget | Subject | Direction |
|---|---|---|
| 12 timing / memory budgets (`xtask perf`) | a property of the host | falling-only |
| wasm bundle size (`xtask dist --web`) | the bytes the shell ships | both, from a dated measurement |

The gate's strictness is unchanged by this correction: **any bundle
above the pinned number still fails the build**, and the number is
re-pinned from a *measurement* — never widened to make a build pass.
This record is the re-pin's receipt, not a waiver.

## The measurements

Every figure below is a `dist web` report line read from the
authoritative GitForge store
(`sudo sqlite3 "file:/var/lib/gitforge/gitforge.db?mode=ro"`), on the
exact SHA named — not from a watcher, not from a remembered log.

| Date | SHA | Branch | wasm bytes | Receipt |
|---|---|---|---|---|
| 2026-11-10 | `31fffb8b` | `ci/web-smoke-real-v2` (= `main` + the release-workflow change; no shell code) | 21,258,126 | run `3dac214f-09c2-4089-8b84-c99a89d1b552`, bridge smoke green |
| 2026-11-10 | `aefe5ac1` | `feat/release-workflow-repair` | 21,275,700 | run `9acd8b5a-eb15-4e81-9053-6e783b6e0495` |
| 2026-11-10 | `fb3a92f9` | `feat/dock-inspector-parity-matrix` | 21,469,527 | run `3befbb1b-84fc-405f-83c1-7135ea5d64b0` |
| 2026-11-10 | `8c68b0f` | `feat/phase-21-daily-driver` | **21,608,249** | run `2bf47590-3f50-445c-9948-f0bc3587b483` — over the 21,500,000 B ceiling |

The last row is the one that forced this record: the Phase 21
daily-driver bundle is 21,608,249 B, i.e. **108,249 B over the old
ceiling**, so PR #15's `release` job failed with

```
wasm bundle /workspace/dist/web/firecraft-web-31978c11c66df807_bg.wasm:
21608249 B exceeds the 21500000 B ceiling
```

## The new pin

`WASM_SIZE_CEILING` moves **21,500,000 → 22,000,000 B**.

- The observation it is pinned from is **21,608,249 B** — the largest
  measured intended-feature bundle in flight (`8c68b0f`, the row
  above).
- The headroom is **+1.81 %** over that observation. The previous pin
  carried **+1.24 %** over its own observation (21,235,599 B →
  21,500,000 B), so the new pin is the same discipline applied to a
  larger bundle, not a loosened gate.
- The headroom also has to cover the **merged** `main` bundle, not one
  branch: the Phase 21 daily-driver surface and the Phase 22 shell
  surface (right dock column, per-kind inspector) are merging
  together, and `main` after both lands carries both deltas.

The next pin is taken from a measurement of the merged `main`, the same
way this one was: the bundle-size budget is re-pinned whenever the
shipped bytes change, in the direction the bytes moved.

## What was *not* the cause (recorded so it is not re-litigated)

The first hypothesis was that PR #15's `eframe` **`accesskit`** feature
added the ~353 KB. It did not, and the reason is structural rather than
incidental:

- `eframe`'s `accesskit` feature is `accesskit = ["egui-winit/accesskit"]`
  (eframe 0.36.2's `Cargo.toml`).
- `egui-winit` is a **desktop-only** dependency of `eframe`:
  `[target.'cfg(not(target_arch = "wasm32"))'.dependencies.egui-winit]`.
- `egui-winit`'s own `accesskit` feature is `["dep:accesskit_winit"]` —
  the `accesskit_winit` adapter, which is winit-specific.
- The workspace resolves with `resolver = "3"` (edition 2024), so
  features are resolved per target. For a `wasm32` build `egui-winit`
  is not in the dependency graph at all, so `eframe/accesskit` selects
  nothing and `accesskit_winit` is never compiled into the bundle.

Enabling `accesskit` therefore cannot add bytes to a wasm bundle; it
enables the **desktop** screen-reader adapter, which is what PR #15's
21.5 desktop arm wants. The isolating probe
(`perf/wasm-accesskit-probe`, `c1a58a0` = `main` + the one-line
feature) is queued on GitForge as the empirical confirmation of the
same claim; the re-pin does not depend on it, because the re-pin
follows the measured bundle size either way.

## Consequences

- `xtask/src/dist.rs` — the constant's doc comment now records the
  2026-11-10 observation, the two-direction rule, and the pointer to
  this record; the failure message names the real rule instead of
  "ceilings only move down".
- `xtask/src/status.rs` — the budget table is retitled **"Performance
  and size budgets"**, the twelve falling-only budgets are stated
  separately from the size budget, and the wasm row carries the new
  ceiling and its observation.
- `docs/planning/generated/parity.md` — regenerated from
  `xtask/src/status.rs` (the `generated-docs` job byte-compares it).
- `docs/GAPS.md` — the "Web bundle" row carries the new measurement
  and ceiling.
- `docs/NEXT.md` — the 19b.4 note no longer claims the ceiling only
  moves down.
- `docs/HANDOFF.md` — the 19b note records the second re-pin.

## Not decided here

- Whether the Phase 21/22 bundle should be *reduced* rather than
  budgeted for. The 353 KB is feature surface (multi-doc tabs,
  autosave and recovery, HiDPI, command palette v2, preferences, the
  right dock column, the per-kind inspector), not an accidental
  dependency; no size work is claimed by this record.
- The **perf** timing and memory budgets, which stay falling-only.
