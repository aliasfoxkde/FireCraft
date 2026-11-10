# 0073 — The CI image's baked registry is a fast floor, not a wall

Date: 2026-11-09 · Ticket: [T27](../tasks/T27-cross-platform-release.md) · Status: accepted

## Decision

`firecraft-ci-rust:1` sets `CARGO_NET_OFFLINE=false`. The baked cargo
registry stays as a **fast floor** — a crate that is already cached
touches the network not at all — but a crate that is *not* cached costs
one small fetch instead of a dead pipeline.

The image is **not** rebuilt when `Cargo.lock` moves. It is rebuilt only
when a pinned tool version below it moves or the Rust minor moves.

## Why

The image used to set `CARGO_NET_OFFLINE=true`. The image carries the
crate versions of exactly **one** lockfile, so any branch whose
`Cargo.lock` pinned something else died in its first cargo step:

```
error: failed to download `time v0.3.45`
Caused by: attempting to make an HTTP request, but --offline was specified
```

Observed 2026-11-09 across the whole fleet: `main`, PR #8, PR #12,
PR #13 and PR #14 all failed in `fmt-layers` that way. Only the branch
whose lockfile the image had been built from passed. The prescribed
workaround was to re-ship the image after every lockfile move — a
procedure needed repeatedly, where each branch's move broke the
others. That is a tool defect wearing a procedure as a disguise, and
the repo's rule is to fix the tool.

Measured from the runner host on 2026-11-09: the crates.io index answers
in ~0.09 s, a crate downloads in ~0.12 s, and a crate that **is** in the
baked cache touches the network not at all. Verified inside the
container with `CARGO_NET_OFFLINE=false`: `cargo fetch` downloads
`time v0.3.45` successfully; the same fetch in the old image is
refused. The flag was there so a job never hangs on a stalled
crates.io transfer — a risk the measurements show is not real on this
network.

## Consequences

- `infrastructure/docker/ci-rust.Dockerfile` sets
  `CARGO_NET_OFFLINE=false` with `CARGO_NET_RETRY=5`, and its rebuild
  paragraph says explicitly not to rebuild for a lockfile move.
- The rollback artifact `firecraft-ci-rust:1-offline-wall` is retained
  on both fleet members.
- The old procedure ("re-ship the image whenever a lockfile moves") is
  **deleted** from `docs/HANDOFF.md` §6.1 rather than kept as a
  documented workaround.
- The gate is not weakened: `cargo` still resolves `--locked`, and a
  lockfile that names an unavailable crate still fails the build. Only
  the *availability* of the crate changed, not whether the gate
  judges it.

### A separate failure on the same day

The same fleet was also red because the host was at **load 2057**: a
throwaway probe binary written during the `/proc/self/exe`
investigation (decision 0072) re-executed *itself* through
`/proc/self/exe --child` on every run, and since its `main()` ignored
argv, each child re-ran `main()` and spawned two more. The cascade
produced ~5800 processes, ~2100 in uninterruptible sleep, and starved
the GitForge runner's container sandbox — surfacing as
`failed to create sandbox … Timeout error`, which looks like an
infrastructure flake but was not one.

Lesson recorded for the probe's own discipline: a self-referential
executable must branch on its argv before it does anything, and a
throwaway probe must be written to a directory it can never be
re-entered from. The probe's directory was moved aside rather than
deleted, and the cascade was terminated by PID with a per-PID
cmdline-and-exe check (no name-based `pkill`).
