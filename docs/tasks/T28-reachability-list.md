# T28 — Print the reachability gate's unreachable list

## Objective

`xtask reachability` measures the gap between the engine's command
registry and the shell surface, but reports only two counts. Everything
downstream — the six per-family issues (#136–#141), the climb's P5
rungs, and any batch plan — needs the *list*, and hand-transcribing it
from the gate's scan is how the docs drifted in the first place (the
audit of 2026-11-13 found a baseline recorded as 164 that the gate reads
as 168).

`reachability --list` prints the unbound registry ids, sorted, one per
line, on stderr with the one-line summary — the workspace denies
`print_stdout`, and every xtask verb keeps its output on stderr. A
consumer strips the summary line:

```bash
cargo run -q -p xtask -- reachability --list 2>&1 >/dev/null | grep -v '^\[xtask\]'
```

## Inputs

- `xtask/src/reachability.rs` — the existing scan (`bound_commands`,
  `SHELL_SOURCE`) and floor comparison.
- `.reachability-floor.tsv` — the rising-only floor the plain verb
  checks against.

## Authority

- `AGENTS.md` — "xtask prints diagnostics to **stderr** so output stays
  pipeable".
- `docs/GAPS.md` §16 and `docs/climb/P5-reachability.md` — the list is
  quoted in both; both should be derivable from one command.

## Stopping condition

- `cargo run -p xtask -- reachability --list` prints exactly the
  registry ids the existing gate counts as unbound, and nothing else
  except its `[xtask]` summary line.
- The list is derived from the same scan as the floor, never
  hand-maintained.
- The plain verb's behaviour and the floor file are unchanged.

## Verification

```bash
cargo run -p xtask -- reachability --list 2>&1 >/dev/null \
  | grep -vc '^\[xtask\]'                          # equals 253 - bound
cargo run -p xtask -- reachability                   # still green, same counts
cargo test -p xtask reachability                     # the two new unit tests
```
