# T27-32 — Assess a Linux aarch64 target

> **RP:** RP-01 · **Priority:** P2 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-04

**Objective:** produce a written verdict on whether a Linux aarch64 build is feasible
for this codebase, and what it would cost — without starting the port.

**Inputs:** the dependency graph (eframe, winit, glow, glutin); the DEB control file's
`Architecture: amd64`; the available cross-compilation toolchain; the artifact matrix's
aarch64 row.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1 (RP-01 includes the aarch64
assessment).

**Acceptance criteria:** a written assessment covers the aarch64 toolchain, native
dependencies (notably the GPU stack), the packaging path for arm64, the CI cost, and
a clear feasible / not-feasible / feasible-later verdict with its conditions.

**Verification:** `rustup target list --installed`

**Evidence:** Matrix records aarch64 as "unassessed"

**Blockers:** —

**Stopping condition:** the assessment is committed and the matrix's aarch64 row is
updated from "unassessed" to the verdict. **Met 2026-11-16** — the assessment is below,
and the matrix's Linux aarch64 row now carries the verdict.

---

## Assessment

**Verdict: FEASIBLE-LATER.** The port is a build-matrix change, not a code change: the
default native build is pure Rust and architecture-agnostic. What defers the target is
the absence of an ARM64 test environment and the scope decision — not a technical
obstacle.

### 1. The aarch64 toolchain

- Rust's aarch64 standard library is already present: `rustup target list --installed`
  lists `aarch64-unknown-linux-gnu` and `aarch64-unknown-linux-musl` for the stable
  toolchain (rustc 1.100.0). A cross build needs no `rustup target add` on a
  developer machine.
- There is **no aarch64 C toolchain or sysroot on this host**: `dpkg
  --print-architecture` is `amd64`, `/usr/aarch64-linux-gnu` does not exist, and no
  `aarch64-linux-gnu-gcc` is on `PATH`. A GNU-target link still needs a libc, so
  `rust-lld` on its own is not enough.
- `zig` is installed (`/home/mkinney/.local/bin/zig`) and supplies both the C compiler
  and the linker for `aarch64-linux-gnu` from its bundled sysroot; `cargo-zigbuild` is
  the conventional wrapper. Debian's `gcc-aarch64-linux-gnu` plus
  `libc6-dev-arm64-cross` is the alternative. Either way the toolchain is a package
  install away — this is the smallest of the four conditions below.
- The CI image does not carry the target: `infrastructure/docker/ci-rust.Dockerfile:94`
  adds `wasm32-unknown-unknown` and nothing else, so an aarch64 CI job needs a new
  image revision — the same rule the campaign already follows for any toolchain
  change.

### 2. Native dependencies and the GPU stack

The shipped desktop build is pure Rust. Nothing in the default feature set compiles C,
and nothing links a system graphics library at build time:

| Layer | Crate | How it reaches the system |
|---|---|---|
| Windowing | `winit` (features `x11`, `wayland`) | — |
| X11 | `x11-dl` | `dlopen` at run time (`x11-dl` depends only on `libc` and `once_cell`) |
| Wayland | `wayland-sys`, `smithay-client-toolkit` | `dlopen` at run time (`wayland-sys` depends on `dlib`) |
| GL context | `glutin` | `dlopen` of `libGL` and `libEGL` |
| GL loader | `glow` | function pointers from `glutin`'s `get_proc_address`; no `libGL` at link time |
| Vulkan | `wgpu`, `naga`, `ash` | present in the lock file but **not enabled** — `crates/app/Cargo.toml` selects eframe's `glow` feature, not `wgpu` |
| Compression | `flate2` | built with `rust_backend`, so no zlib |
| Lossy WebP | `libwebp-sys` | the only C library actually compiled — vendored libwebp built by the `cc` crate — behind `io`'s **non-default** `lossy-webp` feature, which `app-cli` exposes as its own non-default feature; the default desktop and CLI builds do not compile it |

Consequences:

- No X11, Wayland or GL development packages are needed to build, and the runtime
  needs only the target distro's own `libX11`, `libwayland-client` and `libGL`.
- Turning `lossy-webp` on for an aarch64 build would need an aarch64 C compiler — the
  same toolchain as above — so it adds no condition of its own.
- The runtime GPU stack on ARM is the distro's Mesa or a vendor EGL; `glow` speaks the
  same GL/EGL ABI it does on x86_64.

### 3. The packaging path for arm64

Every published Linux artifact name and the DEB's control field are pinned to x86_64:

| Place | The pin | What an arm64 build needs |
|---|---|---|
| `xtask/src/dist.rs:79` | `dist --native` **refuses** any host that is not Linux x86_64 | a target parameter, or a second verb — the refusal is correct as written and must not be relaxed silently |
| `xtask/src/dist.rs:117,123,130` | `firecraft-{v}-linux-x86_64.tar.gz`, `firecraft-cli-{v}-linux-x86_64.tar.gz`, `firecraft_{v}_amd64.deb` | the architecture in the names, and `arm64` in the DEB's file name |
| `xtask/src/dist.rs:1482-1503` | the release manifest's `target` fields | per-target entries |
| `packaging/deb/control:5` | `Architecture: amd64` | `arm64` — a per-architecture control file, or a generated field |
| `packaging/deb/verify-install.sh:62` | reads the architecture back from the installed package (`deb_field Architecture`) | nothing — already architecture-agnostic |
| `packaging/build-appimage.sh:16,36,85` | `linuxdeploy-x86_64.AppImage`, `…-linux-x86_64.AppImage` | the aarch64 `linuxdeploy` binary and an architecture parameter |
| `packaging/build-flatpak.sh:39` | `…-linux-x86_64.flatpak` | an architecture parameter |
| `packaging/flatpak/dev.firecraft.FireCraft.yml` | no `branches:` key, so the manifest builds for the builder's own architecture | `branches: [x86_64, aarch64]` to publish both |

The Flatpak manifest itself is architecture-neutral: it builds from source with the
Rust SDK extension, and freedesktop publishes the 26.08 runtime and SDK for aarch64
as well.

### 4. The CI cost

- One extra job per release: cross-build the desktop and the CLI for
  `aarch64-unknown-linux-gnu`. Its wall-clock is close to the native build's.
- Two one-time costs: a new CI image revision (the aarch64 target plus a cross
  toolchain) and the parameterization of the packaging recipes above — roughly a day
  of work.
- The expensive part is not the build but the **verification**: the runner fleet is
  x86_64 docker containers with no qemu-user in the image, and the fleet has no ARM64
  host. An aarch64 artifact could be *built* but not *run*, and qemu-user would not
  help with the desktop smoke, because it emulates the CPU and not the GL/EGL stack.
  The campaign's ladder requires an install, launch and smoke receipt before an
  artifact rises past "built", so the artifact would stop at rung 3
  (`BUILD VERIFIED`) with no way to advance.

### 5. Verdict and conditions

**FEASIBLE-LATER.** Technically this is among the cheapest platform targets in the
matrix: no code changes, no C library to port, no architecture-specific Rust. The
build is a cross compile, the packaging recipes need an architecture parameter, and
the only true blocker is somewhere to run the result.

All four conditions must hold before an aarch64 artifact is published:

1. **An ARM64 test environment** — an ARM64 host or VM with a real desktop session
   (X11 or Wayland). Without it the artifact can be built but never certified, and the
   campaign's ladder holds it at rung 3.
2. **A scope decision** — [decision 0075](../decisions/0075-release-scope-mvp.md)
   deferred Linux aarch64 as extended scope; reopening it is a maintainer decision,
   not an engineering one.
3. **A CI image revision** carrying `aarch64-unknown-linux-gnu` and a cross toolchain
   (zig or `gcc-aarch64-linux-gnu`).
4. **Architecture-parameterized packaging** — the `dist` verb, the DEB control, the
   AppImage recipe, the Flatpak recipe and the release manifest.

What is *not* needed: no code changes, no architecture-specific Rust, and no C library
porting (unless `lossy-webp` is enabled, which needs the same cross C toolchain).

**Not verified by this assessment:** nothing has been cross-compiled and nothing has
been run on aarch64. Every statement above is read from the locked dependency graph,
the packaging recipes, the CI image definition and this host's toolchain inventory.
