# 0129 — The export counts lines found and lines with counters; the ratio is the signal

Date: 2026-11-15 · Tickets: [T27-57](../tasks/T27-57-workflow-hardening.md) ·
Amends: [0103](0103-coverage-export-counts-a-file-once-per-build.md) ·
Status: accepted

## Context

Decision [0103](0103-coverage-export-counts-a-file-once-per-build.md)
told the coverage gate to print, per file, the export's `DA` record
count and the file's on-disk line count beside its counters. It also
named the check that would close it:

> The check that closes this decision is the next pinned run: `LF` must
> equal the `DA` count again for `shell/mod.rs`, and the file's on-disk
> count must bound both.

The pinned run happened (rustc 1.99.1, head `7766d175`, GitHub run
`43785126332`). Its numbers for `crates/app/src/shell/mod.rs`:

```
[coverage] measured with rustc 1.99.1
[coverage] lines 81.9% (threshold 81%) ✓
[coverage] functions 79.1% (threshold 79%) ✓
[coverage] inventory: 83 profile file(s) merged, 171 file(s) measured, 3 excluded file(s) (5280 line(s))
  .../crates/app/src/shell/mod.rs: lines 4172/5304, funcs 281/365, DA 5167 record(s), 7993 line(s) on disk
```

The on-disk count bounds both: 7993 ≥ 5304 ≥ 5167. Good.

`LF` does **not** equal `DA`: 5304 against 5167. And that is not a
defect, and never was a defect — it is what the export does to every
file. `LF` is the number of lines the export *found*; `DA` is the
number of lines it wrote a *counter* for, and LLVM writes a record
only for the lines that carry one. Across the 171 measured files of
the pinned run, `LF > DA` for 138 of them, and the largest ordinary
ratio was 1.33 — a six-record file, where one extra line found moves
the ratio by a sixth. The tree as a whole sat at 1.03.

The export that 0103 describes — the 1.100.0 one — read differently:

```
  .../crates/app/src/shell/mod.rs: lines 2996/8138, funcs 201/559
  [diagnose] shell/mod.rs: 6309 line(s) on disk
  [diagnose] shell/mod.rs: section with 5297 DA record(s), lines 2996/8138
```

`LF` 8138 against `DA` 5297 is a ratio of 1.54, and the file was 6309
lines long. **The export claimed 1829 more lines than the file had.**
That is the impossibility, and the impossibility is the defect: not
`LF > DA`, which is the norm.

So 0103's closing check had two halves, and the second half was never
an invariant of the format. The gate as first written tested
`DA > on-disk` for impossibility — the claim that could not be true was
the one it did not check, because the export's own line count is the
number that broke.

The same mistake was in the gate's marker. "The export counted this
file more than once" was implemented as `LF > DA`, which fired on 138
of 171 files: the ordinary case, dressed as a defect.

## Decision

- **The ratio is the marker, not the inequality.** A file the export
  counted once per build reports roughly *twice* its own lines. The
  gate reports it when `LF >= 1.5 × DA`: an ordinary export measures
  1.00–1.33, a doubled one lands near 2.0, and the cut sits between.
- **The gate fails on the impossibility.** An export that claims more
  lines than the file has is a broken measurement: the gate fails when
  `LF > on-disk` or `DA > on-disk`, and names the file.
- **0103's closing check is replaced** by the one the measurement can
  actually close: the export's line count must not exceed the file's
  on-disk count, and no file's ratio may sit in the double-count
  range.

## Evidence

- The pinned run (`43785126332`, head `7766d175`): 171 measured files,
  **0** with `LF > on-disk`, **0** with `DA > on-disk`, largest ratio
  1.33, `shell/mod.rs` at 1.03. The export counts the measured tree
  once per file.
- The 1.100.0 export (`43292826338`, head `7ac5afc`): `shell/mod.rs`
  at ratio 1.54, claiming 8138 lines for a 6309-line file. Both
  halves of the marker fire on it, and the impossibility is
  decisive.
- Local exports of the same format (`/var/tmp/fc-final.lcov`,
  `/var/tmp/fc-cat-full.lcov`) show the ordinary gap on files that
  are certainly compiled once — `LF` 734 against 730 `DA` records,
  `LF` 192 against 192, `LF` 2278 against 2230 — so `LF > DA` is a
  property of the writer, not of the tree.

## Consequences

- The gate's marker stops crying wolf: it names the files that are
  really counted once per build, and only those.
- The gate now fails on the claim 0103's diagnostic actually made —
  more lines than the file has — which it previously only detected
  for the `DA` count.
- Decision 0103 stands: its four decisions (print the breakdown, print
  the profile inventory, do not re-bless the floor, name the measuring
  compiler) are unchanged and implemented. Only its closing check is
  replaced, because the check as written cannot be met by any
  ordinary export.

See also decision [0103](0103-coverage-export-counts-a-file-once-per-build.md)
— the same measurement, one step earlier; and
[0102](0102-coverage-floor-is-measured-with-a-pinned-toolchain.md) —
the same defect one layer up: the floor was a claim about a compiler
nobody named.
