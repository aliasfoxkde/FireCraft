# 0072 — The gate's re-exec resolves through `/proc/self/exe`, not `current_exe()`

Date: 2026-11-09 · Ticket: [T27](../tasks/T27-cross-platform-release.md) · Status: accepted

## Decision

When the gate re-execs itself (the `perf` memory probe), it resolves the
executable through **`/proc/self/exe` first**, falling back to
`std::env::current_exe()`. The candidate order is a pure function of its two
inputs (`probe_exe_order`), and a candidate counts only when the child both
spawned *and* exited clean.

## Why

The CI gate's `perf` step failed with:

```
spawn perf-child: No such file or directory (os error 2)
```

The cause is mechanical, not incidental. `std::env::current_exe()` resolves
through `/proc/self/exe` and therefore reports the **inode's current path**,
not the path the process was launched from. The gate runs nested `cargo`
steps (coverage, test) that re-uplift `target/debug/xtask` while the gate is
still running. The running inode is unlinked, `current_exe()` gains a
`" (deleted)"` suffix, and spawning that path fails with `ENOENT` — the
memory probe could not run at all, and the gate failed on a measurement that
had nothing to do with the measurement.

Two independent probes reproduce the shape in **both** link topologies
(hardlink, and the sole-link copy that is cargo's exact uplift shape):

| After the unlink | Result |
|---|---|
| `current_exe()` | `/path/xtask (deleted)` |
| `metadata(current_exe())` | fails |
| `Path::exists("/proc/self/exe")` | true |
| spawn `/proc/self/exe` | succeeds |
| spawn `current_exe()` | `No such file or directory (os error 2)` |

`/proc/self/exe` names the inode directly, so it survives the replacement.
It is also the honest answer: the probe must measure the code this gate is
running, not a build that landed underneath it mid-run.

## Consequences

- `xtask/src/perf.rs` splits the resolution into a pure core
  (`probe_exe_order`) that takes its inputs as values, per the repo's rule
  that a decision takes its environment as data. Four unit tests cover the
  ordering, the duplicate collapse, the fallback, and the empty case.
- `xtask/tests/perf_selfexec.rs` reproduces the CI shape deterministically:
  the gate binary is run from a link of its own, the probe removes that link
  (the running inode survives), and the measurement must still complete. The
  test **fails on the pre-fix code with the exact CI error**, so it
  discriminates the repair rather than the probe.
- The `perf-selfexec` probe refuses to delete what it runs from without the
  `FIRECRAFT_PERF_SELFEXEC=1` opt-in, and refuses outright when the running
  binary has no other name — the probe can only remove a link, never the last
  name of a binary.
- The gate is not weakened: the `perf` step still measures, and a budget
  breach still fails. Only the *resolution* of the child process changed.
