# 0128 — The repository pins the toolchain it measures with

Date: 2026-11-16 · Issues:
[#146](https://github.com/aliasfoxkde/FireCraft/issues/146) ·
Status: accepted

## Context

Decision [0102](0102-coverage-floor-is-measured-with-a-pinned-toolchain.md)
pinned the *workflows* to rustc 1.99.1 and pinned the CI image to
`rust:1.99.1-slim-bookworm`. The third side was left floating: the
repository carried no `rust-toolchain.toml`, so a developer's box ran
whatever its rustup default was.

On this box the default was `stable` — rustc 1.100.0 on 2026-11-16, and
1.99.0 when the issue was filed. The compilers disagree about the same
source, and the disagreement is not academic:

- clippy 1.99.1 fires `chunks_exact_to_as_chunks` on constant chunk
  sizes; 1.99.0 does not. The same commit is green on a 1.99.0 box and
  red on CI, and the fix has to be made without a local signal that
  the change was needed.
- the coverage floor is a claim about a compiler as much as about the
  source. 1.100.0 reads the same tree about 4.9 points of lines lower
  than 1.99.1 does.

The failure mode is the same either way: a contributor has a local gate
that CI ignores, or a local gate that CI rejects, purely from the
default toolchain. `xtask coverage` already names the compiler it
measured with and warns when it is not the compiler the floors were
measured with — but a warning about a mismatch is a procedure worked
around, not a fix.

## Decision

- **The repository pins the toolchain.** A root `rust-toolchain.toml`
  names `channel = "1.99.1"` — the compiler the coverage floors were
  measured with, the compiler the workflows install, and the compiler
  the CI image carries. A box that runs `cargo` inside this tree runs
  that compiler.
- **The pin is the one the CI image already has.** `1.99.1` is
  installed in the image (`rustup component add rustfmt clippy`,
  `rustup target add wasm32-unknown-unknown`), so a pinned build on the
  runner resolves the toolchain it already has and reaches for the
  network for nothing.
- **The pin moves with the floors, in one change.** Bumping
  `rust-toolchain.toml` and the workflows' `toolchain:` is one commit;
  re-blessing the coverage floor is a separate, measured act. A pin
  bump that silently moved the floor would be the defect decision 0102
  closed, reopened one layer down.
- **The file carries no components or targets.** A `components` or
  `targets` key makes rustup reach for the network on a box that does
  not have them, which is a different failure from the one this closes.
  A missing component is the box's own business; a missing *compiler
  version* is what the pin is for.

## Evidence

- `rustc --version` in the tree before this change: 1.100.0 (and
  1.99.0 when issue #146 was filed). After: 1.99.1, read from the
  file by rustup, with no workflow edit.
- The lint that separates the two compilers: `crates/io/src/jpeg.rs`
  fails clippy's `chunks_exact_to_as_chunks` on 1.99.1 and passes on
  1.99.0 — the same source, two verdicts.
- The CI image's toolchain: `infrastructure/docker/ci-rust.Dockerfile`
  builds `FROM rust:1.99.1-slim-bookworm` and adds `rustfmt`, `clippy`
  and `wasm32-unknown-unknown` to it.

## Consequences

- `cargo run -p xtask -- ci` means the same thing on a laptop and on
  the runner, which is the property the release audit recorded as
  finding F-10 ("the release procedure assumes a reproducible
  toolchain").
- A contributor whose box lacks 1.99.1 gets a rustup download at the
  first `cargo` — the intended cost of a pin, and one that happens
  once.
- The wasm target and the clippy/rustfmt components are the box's
  responsibility now, as they always were; the file does not pretend
  otherwise.
- When the floors are re-measured, this file moves in the same change
  that moves the workflows' `toolchain:`, and the decision that
  re-blesses the floor says which compiler measured it.

See also decision [0102](0102-coverage-floor-is-measured-with-a-pinned-toolchain.md)
— the same pin, one layer up.

*Follow-up (2026-11-17):* the one-change rule above is now a measured
check — the `toolchain` gate step (`cargo run -p xtask -- toolchain`)
reads `rust-toolchain.toml` as the source of truth and fails closed,
naming `file:line`, when any workflow `toolchain:` input or CI image
`FROM rust:` base disagrees (issue #146).
