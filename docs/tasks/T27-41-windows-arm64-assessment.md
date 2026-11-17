# T27-41 — Assess a Windows ARM64 target

> **RP:** RP-02 · **Priority:** P2 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-36

**Objective:** produce a written verdict on whether a Windows ARM64 build is feasible
for this codebase, without starting the port.

**Inputs:** the dependency graph (eframe, winit, glow, glutin, accesskit); the
`x86_64-pc-windows-msvc` build configuration; runner availability.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.2 (RP-02 includes the Windows
ARM64 assessment).

**Acceptance criteria:** a written assessment covers the `aarch64-pc-windows-msvc`
target, the native dependency story, whether an ARM64 Windows runner exists, and a
feasible / not-feasible / feasible-later verdict with its conditions.

**Verification:** `rustup target list | grep aarch64-pc-windows`

**Evidence:** The assessment is the ticket itself ([`docs/tasks/T27-41-windows-arm64-assessment.md`](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/tasks/T27-41-windows-arm64-assessment.md)): the toolchain (both `aarch64-pc-windows-msvc` and `aarch64-pc-windows-gnullvm` install for the pinned 1.99.1 toolchain with prebuilt standard libraries, and the MSVC target takes its C runtime from a Visual Studio installation, so linking needs a Windows host), the native stack (the default build compiles no C, the lock file already carries the `windows_aarch64_msvc` and `windows_aarch64_gnullvm` import libraries, `accesskit_windows` gates explicitly on `any(x86_64, aarch64)`, and no workspace crate is x86_64-specific), the GPU story (FireCraft uses eframe's `glow` backend, so a native ARM64 build draws through Microsoft's OpenGL compatibility pack over Direct3D 12, and eframe's GLES fallback is unavailable natively), the runner picture (the GitForge fleet is Linux-only, but GitHub's hosted `windows-11-arm` runner is free for this public repository and carries the ARM64 MSVC toolset, so unlike Linux aarch64 the build needs no new hardware), the packaging path (the `x86_64`-pinned target triple, artifact names and `x64compatible` installer architecture), and the verdict **feasible-later**: no code port is needed, but the artifact cannot rise past a build receipt without an ARM64 Windows environment that can draw, and the x86_64 Windows row T27-36 must land first. Nothing has been built or run for Windows ARM64. **Done:** 2026-11-16: assessment written.

**Blockers:** —

**Stopping condition:** the assessment is committed and the matrix's Windows ARM64 row
is updated from "unassessed" to the verdict. **Met 2026-11-16** — the assessment is
below, and the matrix's Windows ARM64 row now carries the verdict.

---

## Assessment

**Verdict: FEASIBLE-LATER.** The port is a build-matrix change, not a code change:
the default native build is pure Rust, the Windows dependency graph is
architecture-agnostic, and an ARM64 Windows runner already exists at no cost for this
public repository. What defers the target is the absence of an ARM64 Windows machine
that can *draw* — not a technical obstacle.

### 1. The Windows ARM64 toolchain

`rustup target list | grep aarch64-pc-windows` prints two targets:
`aarch64-pc-windows-msvc` and `aarch64-pc-windows-gnullvm`. **Both install for the
repository's pinned toolchain** — `rustup target add --toolchain 1.99.1 <target>`
succeeded for each and downloaded the prebuilt `rust-std` component, so neither
target needs a hand-built sysroot.

The two differ in where the C runtime comes from, and that difference decides where
the build has to run:

| Target | `linker-flavor` | C runtime | Where it can be linked |
|---|---|---|---|
| `aarch64-pc-windows-msvc` | `msvc` | the Visual Studio CRT — the target's `lib/self-contained/` carries only the `std-*.dll.lib` import library, no CRT objects, because the MSVC CRT is not redistributable | **a Windows host** with the ARM64 MSVC toolset |
| `aarch64-pc-windows-gnullvm` | `gnu-cc` (`linker = aarch64-w64-mingw32-clang`) | the target's `lib/self-contained/` **does** bundle `crt2.o` and `dllcrt2.o` — the mingw-w64 CRT import libraries | a Windows host, or a Linux cross-link once the mingw-w64 ARM64 libraries are present |

The MSVC target is the right one for this project: it is the triple
`packaging/build-windows.ps1` already uses, and it is the target the `windows`
crate's ARM64 import library is built for. Its cost is that linking needs a Windows
host — but an ordinary one, not a special one (§3).

Target-spec facts worth recording: the MSVC target advertises `crt-static-respected:
true`, features `+v8a,+neon` and `max-atomic-width: 128`, so the ARM64 build gets the
NEON vector unit and 128-bit atomics without any source change.

### 2. Native dependencies and the GPU stack

| Layer | Crate | How it reaches Windows | ARM64 status |
|---|---|---|---|
| Windowing | `winit` 0.30.13 | `windows-sys` (Win32) | architecture-agnostic — the `windows-sys` API surface is identical for every target |
| GL context | `glutin` 0.32.3 | WGL backend on `windows-sys` 0.52 | architecture-agnostic |
| GL loader | `glow` 0.17 | function pointers from `glutin` | pure Rust |
| Accessibility | `accesskit_windows` 0.32.1 | the `windows` crate 0.62 | **explicitly ARM64-aware**: `src/subclass.rs:25` and `src/adapter.rs:558` gate on `#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]` |
| Import libraries | `windows_aarch64_msvc` / `windows_aarch64_gnullvm` | — | **already in the lock file** (0.52.6 and 0.53.1 of each); the registry carries `windows.0.53.0.lib` and `libwindows.0.53.0.a` |
| Compression | `flate2` | `rust_backend` | pure Rust — no zlib |
| Lossy WebP | `libwebp-sys` | the only C library this project compiles | behind `io`'s **non-default** `lossy-webp` feature; absent from the default desktop and CLI builds |

Consequences:

- The default build compiles no C and links no system graphics library, so the ARM64
  build needs no cross C toolchain. Turning `lossy-webp` on would need the ARM64 MSVC
  C compiler, which the hosted runner image carries.
- No crate under `crates/` is gated on `x86_64`; the only architecture gates in the
  workspace are `target_arch = "wasm32"`.
- The dependency graph is therefore already ARM64-clean — this is the concrete reason
  the answer is "feasible" rather than "not feasible".

**The GPU stack is the real question, not the compiler.** FireCraft selects eframe's
**`glow`** backend (`crates/app/Cargo.toml:31`); `wgpu` is not enabled. eframe 0.36.2
creates a desktop GL context and, if that fails, retries with a GLES context
(`eframe-0.36.2/src/native/glow_integration.rs:1138-1165`, `ContextApi::Gles(None)`).
`egui_glow` 0.36.2 accepts desktop GL 1.20/1.40 and GLES 1.00/3.00 shaders
(`src/shader_version.rs:58-61`), so the desktop GL 3.3 that Microsoft's OpenGL
compatibility pack exposes is more than the shader path needs.

On native ARM64 Windows the OpenGL story is:

- Microsoft's **OpenGL Compatibility Pack** maps desktop OpenGL 3.3 and earlier onto
  Direct3D for ARM64 applications. It needs OS build 19041.488 or later, an
  up-to-date D3D12 driver, and the MSIX package from the Microsoft Store — it is not
  part of the OS image.
- The GLES-through-WGL path is *not* available natively: an ARM64 client asking for
  a GLES context over WGL gets "WGL_ARB_create_context_es2_profile is unavailable"
  (Fyne #6483). eframe's fallback would therefore not rescue a native ARM64 build
  that lacks the compatibility pack.

So the ARM64 build compiles, and the runtime path is plausible, but it rests on a
Microsoft Store package and a D3D12 driver rather than on anything in this
repository. **That is what makes the launch receipt expensive, not the compile.**

One mitigating fact changes the urgency: **Windows 11 on Arm runs x86/x64
applications transparently** (Prism). Once T27-36 lands, the x86_64 Windows artifact
already reaches ARM64 devices under emulation. A native ARM64 build is a native
performance and battery story, not a reach story.

### 3. Runner availability

- The campaign's own CI cannot build **any** Windows artifact, x86_64 or ARM64: the
  GitForge fleet's runners are docker-only (`swarmone-docker` online;
  `swarmone-docker-legacy-…`, `fedora-docker` and `t5500-docker` offline) and all are
  Linux. There is no Windows runner and no ARM64 runner on it. The repository agrees:
  `ci.yml`, `deploy-site.yml` and `release.yml` all declare `runs-on: ubuntu-*`, and
  `.gitforge.yml` has no Windows job.
- **An ARM64 Windows runner does exist, and it is free for this repository.**
  GitHub-hosted Windows-on-Arm runners (`windows-11-arm`, `windows-11-vs2026-arm`)
  went GA on 2025-08-07 and are free for **public** repositories; private
  repositories need a paid larger runner. This repository is public, so the runner is
  available at no cost and without new hardware.
- The `Windows11-Arm64` image carries Visual Studio Enterprise 2022 (17.14.37710.0)
  with the ARM64 MSVC toolset (14.29.16.11) and the 10.0.19041 / 22621 / 26100 Windows
  SDKs, plus Rust 1.99.0 — the repository's pinned 1.99.1 comes from `rustup`.
- The runner is an Azure VM **without GPU passthrough**. It can compile and package;
  it cannot provide a real OpenGL context, so it cannot produce the launch receipt the
  plan requires.

This is the material difference from the Linux aarch64 case ([T27-32](T27-32-linux-aarch64-assessment.md)):
there the fleet has no ARM64 host at all and a cross toolchain is a package install
away; here an ARM64 **Windows** host exists on demand for free, and the only missing
piece is somewhere to *run* the result.

### 4. The packaging path

| Place | The pin | What an ARM64 build needs |
|---|---|---|
| `packaging/build-windows.ps1` | `$Target = "x86_64-pc-windows-msvc"` | a target parameter |
| `packaging/build-windows.ps1` | `$Portable = "firecraft-$Version-windows-x86_64.zip"`, `$Installer = "firecraft-$Version-windows-x86_64-setup.exe"` | the architecture in the names |
| `packaging/windows/firecraft.iss` | `OutputBaseFilename=firecraft-__APPVERSION__-windows-x86_64-setup` | the architecture in the name |
| `packaging/windows/firecraft.iss` | `ArchitecturesInstallIn64BitMode=x64compatible` | `arm64` — Inno Setup 6 accepts `arm64` in both `ArchitecturesAllowed` and `ArchitecturesInstallIn64BitMode`, and Microsoft's Windows-on-Arm guidance lists Inno Setup as supported |
| `docs/release/ARTIFACT-MATRIX.md` | one Windows x86_64 ZIP row and one installer row | ARM64 rows |
| the release manifest in `xtask/src/dist.rs` | `target:` fields | per-target entries |

Nothing deeper is pinned: the installer script's product identity, shortcut table and
version metadata are architecture-neutral, and the build script's stage layout
(`bin/firecraft.exe`, `bin/firecraft-cli.exe`, `share/licenses/`) does not mention an
architecture.

### 5. Verdict and conditions

**FEASIBLE-LATER.** The port is a build-matrix change: no code changes, no
architecture-specific Rust, no C library to port, no new hardware and no paid runner.
The target's three conditions are:

1. **T27-36 lands first.** The x86_64 Windows row — the MSVC build and the portable
   ZIP — is the row that proves the toolchain, the runner wiring and the receipt
   format. ARM64 is an increment on that recipe, not a separate path. Building ARM64
   first would produce an artifact for a platform whose installer, signing and smoke
   receipts do not exist yet.
2. **A Windows job in the workflows.** GitHub's `windows-11-arm` satisfies this for
   this public repository at no cost; the GitForge fleet cannot and does not need to.
   The unresolved Authenticode story ([T27-40](T27-40-authenticode-signing.md)) is not
   ARM64-specific — no Windows target can be published unsigned.
3. **An ARM64 Windows environment that can draw** — a physical ARM64 PC, or an ARM64
   host with a D3D12 driver and the OpenGL compatibility pack. The hosted runner has
   no GPU: it can build but never certify. Without this the artifact stops at
   "build verified", and `docs/RELEASE-ENGINEERING-PLAN.md` §3.1 forbids the claim:
   "Do not publish ARM64 claims until the native target and dependencies build and
   launch on Windows ARM64." Emulation does not substitute — an x64 binary under Prism
   still needs the same GL stack.
4. **Architecture-parameterized packaging** — `build-windows.ps1`, `firecraft.iss`, the
   artifact names and the release manifest.

And the scope question, which is not an engineering matter:
[decision 0075](../decisions/0075-release-scope-mvp.md) placed ARM64 in extended
scope, and the maintainer has not approved an MVP scope at all.

**Not verified by this assessment:** nothing has been built or run for Windows ARM64.
A `cargo check --locked --target aarch64-pc-windows-msvc -p app -p app-cli` was
queued on this host and the build lease denied it — the campaign's own CI saturates
the machine's CPU and I/O, and the lease refuses to admit a second compiler while it
does — so the compile evidence here comes from the target specifications, the
installed standard libraries and the locked dependency graph, not from a finished
build. No ARM64 binary has been linked, installed or launched, and no OpenGL context
has been created on ARM64. The compatibility-pack and Prism facts come from
Microsoft's documentation and the runners' published image manifest, not from this
fleet.
