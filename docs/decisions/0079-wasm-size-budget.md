# 0079 — The web bundle size budget: a shipped-bytes budget, not a falling-only floor

> **Historical decision:** the 22,000,000 B observation-based ceiling recorded here was superseded by [decision 0080](0080-wasm-bundle-size-policy.md), which establishes the current 25 MB warning and 50 MB hard ceiling. The 21,608,249 B measurement and analysis below remain the baseline.

Date: 2026-11-10 · Status: **accepted**

Relates to: decision **0071** (the bundle stays on Cloudflare Pages; the
size figure is FireCraft's own budget, not a platform limit), decision
**0074** (the content-keyed Aegis gate), `xtask/src/dist.rs` (the
ceiling), `xtask/src/status.rs` (the budget table), and the Phase 21/22
feature surface.

0071 already said the right thing — the ceiling is *"FireCraft's own
regression tripwire… deliberately re-pinned each time the bundle grows"*,
and that it is *"re-pinned deliberately, and the re-pin is recorded in
the release notes"*. The **code and the generated table** did not say it:
the failure message and the budget table's heading both said the opposite.
This record makes the two statements agree, and pins the number.

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
21.5 desktop arm wants.

### The graph evidence (measured locally, not argued)

The structural argument above was checked against the resolved
dependency graph, on both sides of the one-line change, with:

```
cargo metadata --format-version 1 \
  --filter-platform wasm32-unknown-unknown --offline
```

| Graph | wasm32 packages |
|---|---|
| `main` (`2cfdeee`) | 336 |
| probe (`c1a58a0` = `main` + `eframe/accesskit`) | 338 |

The two crates the probe adds to the wasm32 graph are `futures-macro
0.3.34` and `tracing-attributes 0.1.31` — and **both are
`proc-macro = true`**. A proc-macro crate is compiled on the *host*
and never linked into the wasm artifact, so it contributes **zero**
bytes to the bundle. Nothing else is added or removed.

`egui`'s resolved feature set is identical in both graphs —
`bytemuck, default, default_fonts, persistence, ron, serde` — so the
feature does not even change egui's own compile. In the wasm32 graph
`accesskit 0.24.1` is present on **both** sides, and its only dependent
is `egui`, which depends on it **unconditionally** (egui 0.36.2's
`Cargo.toml` has a plain `[dependencies.accesskit]`, not an optional
one). `egui-winit` is absent from the wasm32 graph entirely. The
`accesskit` *runtime* was therefore already in the bundle before
PR #15 and its feature flag changes nothing about it.

This is the receipt that replaces the queued-probe wording an earlier
draft of this record carried. The probe branch itself
(`perf/wasm-accesskit-probe`) is still re-run as the end-to-end
confirmation of the same claim — its first run failed at `fmt-layers`
with `cannot update the lock file ... because --locked was passed`,
because the feature line was committed without the lockfile it implies,
not because the hypothesis was wrong; the lockfile is corrected in
`7dba077`. The re-pin does not depend on the probe either way, because
the re-pin follows the measured bundle size regardless of the cause.

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
