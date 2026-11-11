# 0090 — Web bundle size policy: 25 MB warning, 50 MB hard ceiling

Date: 2026-11-10 · Status: **accepted by maintainer instruction**

Relates to: [decision 0079](0079-wasm-size-budget.md), the
[production-readiness plan](../PRODUCTION-READINESS-PLAN.md),
[xtask/src/dist.rs](../../xtask/src/dist.rs), and
[xtask/src/status.rs](../../xtask/src/status.rs).

## Decision

Apply two independent guardrails to the raw WebAssembly module produced
by the production web build:

| Guardrail | Value | Behavior |
|---|---:|---|
| Investigation warning | 25,000,000 B (25 MB) | Emit a warning; build continues |
| Hard ceiling | 50,000,000 B (50 MB) | Fail the build above this value |

All values use decimal megabytes (1 MB = 1,000,000 bytes). The hard
ceiling is an explicit maintainer policy limit, not a measurement-based
target. A 49 MB bundle is not considered good merely because it passes
the cap. Crossing the 25 MB warning requires a measured optimization
review and explanation before further size growth is accepted.

The latest measured raw Wasm module is **21,608,249 B**, observed
2026-11-10 on the Phase 21/22 feature branch. Decision 0079 retains
the complete historical measurements and dependency-graph analysis.
The 50 MB cap gives agents room to work while systematic size reduction
is performed; it is not permission to double the shipped bundle.

## Build optimization

The web build now has two size-oriented optimization stages:

1. Trunk's asset declaration selects Binaryen's size-oriented
   wasm-opt=z pass rather than optimization level 4, which is
   speed-oriented.
2. xtask dist --web sets CARGO_PROFILE_RELEASE_OPT_LEVEL=z only for
   the child Trunk build. The native desktop release profile is not
   changed by this override.

The bridge smoke still runs against the staged release bytes. A smaller
bundle is not accepted if it breaks startup, document open, command
application, PNG export, document export, drag/drop, or WebOS messaging.

## Required size report

For the same source SHA and pinned toolchain, agents must record:

- Raw Wasm bytes and section/function breakdown.
- JavaScript glue bytes, HTML/CSS, and total uncompressed bundle bytes.
- Gzip and Brotli size for every served asset.
- Rust, Cargo, Trunk, wasm-bindgen, Binaryen, and optimization versions.
- Build command, source SHA, cache condition, and CI run.
- Cold fetch, compile/instantiate, first frame, interactive-ready time,
  and representative memory/CPU observations.
- Top size contributors from a Wasm profiler such as Twiggy.
- Dependency and feature graph before/after.
- Exact test and browser-smoke receipts.

Do not claim a reduction without a same-SHA before/after comparison.

## Future policy

- Below 25 MB: retain size reports as part of the release build.
- At or above 25 MB: update the optimization issue with contributors,
  experiments, and owner before accepting more size growth.
- Above 50 MB: hard failure. A higher cap needs a new decision, measured
  cause, impact review, and explicit maintainer approval.
- Update the measured baseline after meaningful size changes; never
  confuse the hard cap, warning threshold, and observed size.
- Keep Rust source identifiers readable. Renaming Rust locals does not
  reduce optimized Wasm in the ordinary case because source names are
  not shipped as source text. Strip nonessential name/debug sections
  only after checking diagnostics and stack traces.
- Minify generated JavaScript only if profiling shows it is material.
  Keep source maps for development and compare raw, gzip, and Brotli.
- Do not apply JavaScript obfuscation as a substitute for minification
  or dependency reduction. It can damage diagnostics and compression.
- Reusable Rust helpers are not automatically smaller: generic
  monomorphization and inlining can increase code. Measure the emitted
  result before retaining a refactor for size reasons.

The generated parity document is derived from xtask/src/status.rs;
update the source and generated output together.
