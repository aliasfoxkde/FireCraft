# 0103 — The coverage export counts a file once per build; the gate prints its inputs

Date: 2026-11-12 · Tickets: [T27-57](../tasks/T27-57-workflow-hardening.md) ·
Status: accepted

## Context

[Decision 0102](0102-coverage-floor-is-measured-with-a-pinned-toolchain.md)
established that the two red gates (#133 and #134) were measured by
rustc 1.100.0 rather than by the compiler the floors were measured
with. It did not say *what* the other compiler measures differently,
and the difference read like a 4.9-point drop in coverage — as if the
new code were uncovered.

A diagnostic run (GitHub `43292826338`, head `7ac5afc`, branch
`chore/coverage-breakdown`) instrumented the coverage gate to print its
own inputs on #133's tree under rustc 1.100.0. What it found is not a
coverage drop. It is a denominator that counts one file more than
once.

The numbers, verbatim from that run:

```
[coverage] breakdown: 170 measured file(s), lines 63284/86563, funcs 4849/6687, 3 excluded file(s) (3402 line(s))
[coverage]   .../crates/app/src/shell/mod.rs: lines 2996/8138, funcs 201/559
[diagnose] shell/mod.rs: 6309 line(s) on disk
[diagnose] shell/mod.rs: section with 5297 DA record(s), lines 2996/8138, DA lines 302..6308
[diagnose] shell/mod.rs: 1 section(s)
```

A file cannot have more lines than it has.
`crates/app/src/shell/mod.rs` has 6309 lines on disk, 5297 of them
instrumented (5297 `DA` records, spanning source lines 302..6308), and
the export reports **8138** lines found for it — 1829 more than the
file has, and 2841 more than it instruments.

The reading that fits every number: the file is compiled **twice** —
once for the app library, and once for the library's unit-test binary,
whose `#[cfg(test)]` module lives at the end of the same file — and the
export merges the two instances. The merge lists each source line once
(5297 `DA` records) but sums the two instances' line counts
(2841 + 5297 = 8138). The app's binary target does not add a third
copy: `crates/app/src/main.rs` calls into the library
(`app::shell::Shell`), so the library build and the library's test
build are the two instances.

The effect on the score is mechanical. The numerator counts a line
once; the denominator counts it twice. So a file's coverage reads
roughly half of what it is: `shell/mod.rs` reads 36.8 % (2996/8138)
where the instrumented lines actually covered are 2996 of 5297 —
56.6 %.

## Decision

- **The gate prints the per-file breakdown beside its verdict.** Each
  row carries the file's `DA` record count and its on-disk line count
  next to its counters, so a reader can see at a glance whether the
  reported line count outruns the file it claims to measure.
- **The gate prints the profile inventory.** The number of `.profraw`
  files the measurement merged is the measurement's other input, and it
  belongs in the log.
- **The floor is not re-blessed, and no file joins the exclusions.** The
  measurement moved; the code did not. Lowering the floor or widening
  `EXCLUDED` until a moving measurement passes is the weakening P2.1
  forbids.
- **The measuring compiler is named** (decision 0102), because the
  export's behaviour is the compiler's.

## Evidence

- The run above: one tree, one compiler, and a file whose reported line
  count exceeds its own length. The `DA`/`LF`/`on-disk` triple is what
  makes the merge visible; without the `DA` count the export's number
  is indistinguishable from a real regression.
- The app crate is the one that compiles twice: `crates/app/src/lib.rs`
  declares `pub mod shell;` and `crates/app/src/main.rs` uses the
  library rather than re-declaring the module, so the shell's modules
  exist in the library build and in the library's unit-test build.
- Decision 0102's run table: five runs, two toolchains, one Rust tree,
  and the same 4.9-point gap.

## Consequences

- Every future coverage run states, per file, whether it measured the
  file or the sum of the file's builds. The check that closes this
  decision is the next pinned run: `LF` must equal the `DA` count again
  for `shell/mod.rs`, and the file's on-disk count must bound both.
- A red coverage gate now names the file and the reason instead of a
  bare percentage.
- Open, and deliberately not folded in here: `cargo-llvm-cov`'s own
  version is not pinned in CI (`cargo install cargo-llvm-cov --locked`
  floats to the newest published release), and the `target` cache key is
  `cargo-build-<os>-<Cargo.lock digest>` — it carries neither the source
  tree nor the toolchain. Neither explains this measurement: the failing
  runs restored the same warm cache as the passing ones and still read
  low. Both make the measurement's inputs harder to see than they need
  to be, so pinning the wrapper version and adding the source digest to
  the cache key is the next tool-level change, on its own branch.

## Receipt

The pinned run (rustc 1.99.1, head `7766d175`, GitHub run
`43785126332`) printed what this decision asked for:

```
[coverage] measured with rustc 1.99.1
[coverage] lines 81.9% (threshold 81%) ✓
[coverage] functions 79.1% (threshold 79%) ✓
[coverage] regions 0.0% (not gated)
[coverage] inventory: 83 profile file(s) merged, 171 file(s) measured, 3 excluded file(s) (5280 line(s))
[coverage] breakdown: 171 measured file(s)
  .../crates/app/src/shell/mod.rs: lines 4172/5304, funcs 281/365, DA 5167 record(s), 7993 line(s) on disk
```

The on-disk count bounds both: 7993 ≥ 5304 ≥ 5167. Across all 171
measured files, none claimed more lines than it had — 0 with `LF`
above its on-disk count, 0 with `DA` above it — and the floor was not
re-blessed (81.1 % / 78.7 % unchanged; measured 81.9 % / 79.1 %).

The closing check above asked for `LF == DA`. The run reads 5304
against 5167: `LF` is the lines the export found, `DA` the lines it
wrote a counter for, and the two differ for 138 of the 171 files —
the ordinary case, not a defect. The check is restated, and the
gate's marker corrected with it, in decision
[0129](0129-the-export-counts-lines-found-and-lines-with-counters.md).

See also decision [0102](0102-coverage-floor-is-measured-with-a-pinned-toolchain.md)
— the same defect one layer up: the floor was a claim about a compiler
nobody named.
