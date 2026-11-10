# 0077 — The DEB's libc6 floor follows the build host's glibc

Date: 2026-11-09 · Ticket: [T27](../tasks/T27-cross-platform-release.md) · Status: accepted

## Context

The v0.2.5 release shipped `firecraft_0.2.5_amd64.deb` with
`Depends: libc6 (>= 2.31)`. The package installs on Debian 11 and
Ubuntu 20.04. The binaries inside it do not run there: they were
linked on a host whose glibc is newer than 2.31, so the dynamic
loader cannot resolve the versioned symbols they reference.

Measured on 2026-11-09 from the published artifacts:

| Binary | Highest GLIBC symbol version | Referencing symbols |
|---|---|---|
| `usr/bin/firecraft` | **2.39** | `pidfd_spawnp`, `pidfd_getpid` (weak) |
| `usr/bin/firecraft-cli` | **2.35** | `hypot`, `hypotf` |

`objdump -T` on the extracted payload, not a remembered table.

`xtask`'s `verify_deb_glibc_floor` was supposed to catch this before
the package was built. It never could:

```rust
.split_whitespace()
.filter_map(|tok| tok.strip_prefix("GLIBC_"))
```

Modern binutils prints the version **parenthesized** —
`(GLIBC_2.34)` — not as a bare token. `strip_prefix("GLIBC_")` never
matched a single symbol, the fold stayed at its `f64::MIN` seed, and
the check compared "nothing needed" against the floor. It passed
every time, including on the package that shipped broken. The check
was vacuous.

Two further facts shaped the fix, both established by experiment:

1. **The requirement comes from the build host's glibc, not from Rust
   std.** A program that calls `std::process::Command` and is built on
   Debian 12 (glibc 2.36) keeps `pidfd_spawnp`/`pidfd_getpid` as weak
   undefined symbols with **no version tag** — the maximum version it
   references is 2.34. The same program built on Ubuntu 24.04
   (glibc 2.39) carries `(GLIBC_2.39)` on those same weak symbols.
   Rust std declares them weakly so an older glibc resolves them to
   null; the *version tag* is what the build host's linker stamps.
2. **A symbol version is not a float.** `2.2.5` (the historic base
   series) and `2.10` both appear in real output, and `2.9 < 2.10`
   inverts under a float comparison.

## Decision

- The native artifacts are built on **Debian 12** (glibc 2.36), in
  the same base the CI image uses, and `packaging/deb/control` declares
  `libc6 (>= 2.36)`. GitHub Actions' `release.yml` builds them inside
  a `rust:1.99-slim-bookworm` container for that reason; the
  ubuntu-24.04 runner's glibc is 2.39 and produced the broken
  package.
- `verify_deb_glibc_floor` parses the version column as it is
  printed — parenthesized `(GLIBC_2.34)` and bare `GLIBC_2.2.5` are
  both accepted — compares versions component-wise rather than as
  floats, ignores `GLIBC_PRIVATE`, and **fails loudly** when the
  binary has a dynamic symbol table but no GLIBC version could be
  read. A check that cannot measure the requirement must not report
  success.
- The release workflow installs the built package on a clean
  `debian:12-slim` container and runs `firecraft-cli`, which links
  against libc only. That turns "the floor is adequate" into a
  statement about the shipped artifact rather than about a parser.

The published v0.2.5 package is historical and is not replaced. Its
declared floor (2.31) stays wrong on purpose: it records what was
published. The corrected floor applies to the next build.

## Consequences

- Building native artifacts elsewhere now **fails the release** with a
  message naming the required version, instead of silently producing a
  package that cannot install on its declared platform.
- The 2.36 floor means the DEB installs on Debian 12 and newer, and
  not on Debian 11 (glibc 2.31). That is the intended support
  statement.
- The tar.gz artifacts get the same treatment for free: they are built
  from the same binaries, so they now run on Debian 12 as well.
- The `DEB` still declares only `libc6`. The desktop binary also
  needs the X11/GL stack at run time, which the control file does not
  declare — a separate packaging gap, tracked independently of the
  floor.

See also decision 0073 (the CI image's baked registry is a floor, not a
wall) and 0076 (the gates resolve their tree at run time).
