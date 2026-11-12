# 0102 — The coverage floor is measured with a pinned toolchain

Date: 2026-11-12 · Tickets: [T27-57](../tasks/T27-57-workflow-hardening.md) ·
Status: accepted

## Context

On 2026-11-12 the GitHub `gate` job went red on two pull requests whose Rust
source was byte-identical to `main`'s.

PR #134's tree differs from `main` only in `docs/` and `packaging/`. The Rust
diff is empty:

```
git diff --shortstat origin/main origin/chore/windows-recipe-audit \
    -- crates xtask Cargo.toml Cargo.lock
```

Yet #134 measured **77.0 % lines / 76.1 % functions** and failed the
81.1 / 78.7 floor, while `main` measured **81.9 / 78.9** and passed.

The differentiator is the compiler, not the code:

| GitHub run | tree | rustc | measurement |
|---|---|---|---|
| `43157085372` | `main` @ `bc8abfb` | 1.99.1 | 81.9 / 78.9 ✓ |
| `43169461346` | #130 @ `002eb56e` | 1.99.1 | 81.9 / 78.8 ✓ |
| `43284817650` | #134 @ `f570c86` | **1.100.0** | 77.0 / 76.1 ✗ |
| `43284861290` | #134 @ `f570c86` | **1.100.0** | 77.0 / 76.1 ✗ |
| `43287370012` | #134 @ `545e7a5` | **1.100.0** | 77.0 / 76.1 ✗ |

The workflow installs `toolchain: stable`. rustc 1.100.0 was released on
2026-11-09 and reached the runners during 2026-11-12. The GitForge pipeline
did not move: its image is `rust:1.99-slim-bookworm` (rustc 1.99.1), which is
why the two pipelines disagreed about the same commit — GitForge's
`clippy-test` job passed the floor while GitHub's `gate` job failed it.

The build cache is not the explanation. The failing runs restored the same
Cargo.lock-keyed build cache as the passing ones, and the last failing run
(`43287370012`) restored a warm cache and still read 77.0.

So the floor was never a property of the source tree alone. It was a property
of the source tree *and* the compiler that instrumented it — and the workflow
said `stable`.

## Decision

- **The GitHub workflows pin the toolchain.** `ci.yml` (the `fmt` and `gate`
  jobs) and `release.yml` install `toolchain: 1.99.1` — the compiler the
  floors were measured with. A release is built with the compiler the gate
  judged, not with whatever `stable` happened to be that week.
- **The CI image is pinned to the patch.** `rust:1.99-slim-bookworm` becomes
  `rust:1.99.1-slim-bookworm`. The two tags resolve to the same image digest
  today (`sha256:392f2219…`), so the runner's image does not change; the
  explicit patch stops a future `rust:1.99` from moving under the floor.
- **The gate names the compiler that measured it.** The coverage output
  carries `measured with rustc <version>`, and says so plainly when that is
  not the version the floors were measured with. The gate warns; it does not
  fail, because a developer's local compiler is not the CI compiler and a
  local `xtask ci` must not become impossible.
- **The floor is not re-blessed.** 81.1 / 78.7 stands. The measurement moved,
  not the code, and lowering the floor to meet a moving measurement is the
  weakening the gate exists to prevent.

## Evidence

- The run table above: five runs, two toolchains, one Rust tree.
- `rust:1.99.1-slim-bookworm` and `rust:1.99-slim-bookworm` are the same
  digest on Docker Hub (`sha256:392f221964669402bf4ae133d2a46c3ae270f64734133f4b2703d40019235c1a`),
  so the image pin changes no artifact.
- `xtask/src/coverage.rs` names the measuring compiler and carries unit tests
  for the version parser.
- PR #133 (`feat/play-animation`) also failed the floor, at 73.3 / 72.8 —
  and its diagnostic run was on 1.100.0 as well. Its deficit is therefore
  partly the toolchain and partly real: the animation code it adds is not
  covered. That is a separate, content-level shortfall, and this decision
  does not excuse it.

## Consequences

- The GitHub and GitForge pipelines judge the same rustc again, which is what
  `infrastructure/docker/ci-rust.Dockerfile` already asked for ("this pin
  matches backend-ci-rust:1 so both pipelines judge the same rustc").
- A toolchain bump is now an explicit change with a measured consequence,
  instead of an event that silently moves the floor under every open pull
  request.
- The floor's basis is written down: 81.1 / 78.7, measured with rustc
  1.99.1. A future re-bless names the compiler it was measured with.
- The local toolchain (1.99.0 here) is one patch behind the pinned one; the
  gate warns when it measures with anything other than 1.99.1.

See also decision [0099](../decisions/0099-workflow-actions-are-pinned.md)
— the same class of drift, one layer down: an unpinned *action* versus an
unpinned *compiler*.
