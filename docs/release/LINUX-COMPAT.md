# Linux platform compatibility

> The authoritative statement of what FireCraft is **tested** on, what
> is only **built**, and what is **deferred**. Read it before assuming a
> Linux artifact works on a given machine.
>
> Authority: [decision 0075](../decisions/0075-release-scope-mvp.md)
> (release scope, accepted) and the artifact inventory in
> [ARTIFACT-MATRIX.md](ARTIFACT-MATRIX.md). Execution ledger:
> [EXECUTION-BOARD.md](EXECUTION-BOARD.md) rows T27-33 and T27-34.
>
> A cell is listed as *tested* only when the artifact was installed (or
> unpacked) on that exact distribution **and** the desktop binary was
> launched there. *Built* means the bytes exist but were never exercised
> on that platform. *Deferred* means a recorded decision took it out of
> scope. Nothing is implied: an absent row is an untested combination,
> and every untested combination is listed.

## What was exercised

**Date:** 2026-11-17 · **Host:** Fedora Linux 44 (Workstation Edition),
x86_64, Docker 29.7.2 · **Artifacts built from:** `a8a59b35` on the same
host, inside `rust:1.99-slim-bookworm` (Debian 12), so the DEB's glibc
floor is the real one.

**These are the published bytes.** The three artifacts tested here are
byte-identical to the assets the release workflow published in the
rehearsal `v0.2.5-dryrun.5` (draft release 449261624): the digests
below are the digests the GitHub API reports for those assets. The
matrix is therefore a receipt for the *published* artifacts, not for a
local rebuild that merely resembles them.

| Artifact | Bytes | sha256 |
|---|---|---|
| `firecraft_0.2.5_amd64.deb` | 16,228,104 | `7928369626375dfca80516551870d0ab7c98b3f3021ff5c82fc36a86dc39fa42` |
| `firecraft-0.2.5-linux-x86_64.tar.gz` | 24,389,908 | `cb55a52507477c9af327c35be8bb0e6d7dfc16a767e34bfde1aab0d92f346a3d` |
| `firecraft-cli-0.2.5-linux-x86_64.tar.gz` | 8,158,124 | `953633e40c2021858e7001f9691a47ad4ed4491ec752b38ccd716f8e99339093` |

The DEB declares `Version: 0.2.5`, `Architecture: amd64` and
`Depends: libc6 (>= 2.36), libgcc-s1, libgl1, libx11-6, libxcursor1,
libxrandr2, libxi6, libxkbcommon0, libxkbcommon-x11-0`.

### The tested matrix

| Distribution | Version | Arch | DEB | tar.gz (desktop) | tar.gz (CLI) | X11 | Wayland |
|---|---|---|---|---|---|---|---|
| Ubuntu | 24.04.5 LTS | x86_64 | **tested** | **tested** | **tested** | **tested** | **tested** |
| Debian | 12 (bookworm) | x86_64 | **tested** | **tested** | **tested** | **tested** | **tested** |
| Fedora | 45 | x86_64 | n/a — format | **tested** | **tested** | **tested** | **tested** |

"n/a — format" is a fact about the artifact, not a gap in the test: a
`.deb` is a Debian package format, and no package manager on a
Fedora-family system can install it. The Fedora cells therefore test
the format-agnostic artifact, the desktop tar.gz.

### Formats that have no tested row

| Format | State | Why |
|---|---|---|
| AppImage | **never built** | the recipe exists (`packaging/build-appimage.sh`) but has never been executed — B-8 (`linuxdeploy` absent, no pinned copy vendored), B-10 (the recipe ignores `CARGO_TARGET_DIR`), B-9 (the icon is in-tree but `linuxdeploy` has still never run) |
| Flatpak | **never built** | the recipe and manifest exist (`packaging/build-flatpak.sh`, `packaging/flatpak/dev.firecraft.FireCraft.yml`) but have never been executed — B-11 (`flatpak-builder` absent), plus the manifest's module source is the local working tree, so a bundle built from it would not be reproducible from a tag SHA |
| RPM | **deferred** | [decision 0070](../decisions/0070-rpm-deferred.md), re-validated 2026-11-16 (T27-31): no issue on the tracker asks for an RPM; Fedora-family users are served by the tar.gz |

## What each cell actually did

The matrix is reproducible: `scripts/release-matrix.sh <artifacts-dir>`
on any Docker host, with the leg scripts in `packaging/linux/`.

**Debian-family cells** (Ubuntu 24.04, Debian 12) — install the DEB with
`apt`, so apt itself resolves the package's declared dependencies, then
run the repository's own eight-stage package verification
(`packaging/deb/verify-install.sh`):

1. metadata — `Version: 0.2.5`, `Architecture: amd64`
2. desktop entry — installed, names `firecraft`/`firecraft`, and
   `desktop-file-validate` accepts it
3. icon — installed at `/usr/share/icons/hicolor/256x256/apps/firecraft.png`
4. CLI — `firecraft-cli` runs
5. CLI document round trip — `firecraft-cli info` reads the document,
   `firecraft-cli export` writes a 97 B PNG with the signature
   `89504e470d0a1a0a`
6. GUI launch under Xvfb — the process stayed up for the 25 s launch
   window (exit status 124)
7. upgrade — **not exercised**: no previous package was supplied
8. uninstall / reinstall — removal took the binaries, the desktop entry
   and the icon with it; reinstall restored the payload and the CLI runs

Then the desktop tar.gz is unpacked and launched under Xvfb (exit 124),
and `fc-list` enumerates fonts.

**Fedora-family cells** (Fedora 45) — no `dpkg`, so the DEB is recorded
as not applicable; the desktop tar.gz is unpacked, `firecraft-cli
--help` runs, the desktop binary launches under `xvfb-run` (exit 124),
and `fc-list` enumerates fonts.

**Wayland cells** (all three families) — weston's headless backend
serves `wayland-0`, the desktop binary is launched with
`WAYLAND_DISPLAY=wayland-0` and stays up for the 25 s launch window
(exit 124), and `fc-list` enumerates fonts.

All six cells passed on 2026-11-17. The cell logs are the receipts.

The matrix was run twice on that date. The first run exercised copies of
the leg scripts; the second run — the one the receipts above describe —
used the repository's own `scripts/release-matrix.sh` and
`packaging/linux/leg-*.sh` at commit `8f9a602c`, with the same six
cells and the same artifacts. The second run exists because the first
run's copies were not the tracked scripts, and because the tracked
Wayland leg had a real defect the copies did not have: it ran
`apt-get install` on a fresh image whose package lists were never
fetched, so the Debian and Ubuntu Wayland cells answered "Unable to
locate package weston" and failed. The leg now refreshes the lists
first, and all six cells pass with the tracked scripts.

## What is NOT tested

These are gaps, not endorsements. Nothing in this table claims support
for a combination that is not in the tested matrix above.

**Distributions and versions** — every other release of every family:
Ubuntu 22.04 LTS, 20.04 LTS and every earlier release; Debian 13, 11,
10 and every earlier release; Fedora 44 down to 40; RHEL / CentOS
Stream / AlmaLinux / Rocky; Arch and derivatives; openSUSE / SLES;
Alpine; NixOS; Clear Linux; Gentoo; Void; Solus; any non-Debian,
non-Fedora family.

**Architectures** — every architecture other than x86_64: aarch64/arm64
(assessed feasible-later, [T27-32](../tasks/T27-32-linux-aarch64-assessment.md),
deferred), 32-bit x86, RISC-V, ppc64le, s390x.

**Real hardware** — every cell ran in a container with software
rendering (llvmpipe/swrast). No cell used a real GPU, a vendor
OpenGL/Vulkan driver, a Wayland compositor with hardware acceleration,
or a display manager.

**Real desktop sessions** — X11 was Xvfb (a virtual framebuffer) and
Wayland was weston's headless backend. No cell ran GNOME, KDE Plasma,
Xfce, Cinnamon or a systemd user session. Desktop integration was
checked at the *package* level only: the entry is installed and
validates, and the icon is installed. It was **not** checked at the
*session* level: no application menu appearance, no file association,
no thumbnailer, no MIME registration, no icon-theme resolution beyond
the file's presence.

**Interaction and I/O** — no cell exercised audio, clipboard, drag and
drop, file dialogs, HiDPI scaling, multi-monitor layouts, GPU
acceleration paths, or the Vulkan backend.

**The DEB's upgrade path** — stage 6 of the package verification needs a
previous package to upgrade from, and none was supplied, so the upgrade
transition is untested.

**The published v0.2.5 release assets** — the artifacts tested here are
built from `a8a59b35` in this repository, and they are byte-identical
to the assets the rehearsal published. The *production* `v0.2.5`
release's own assets were published out of band (B-21): they were
downloaded and checksum-verified on 2026-11-09, but their clean
container receipts were taken against the **v0.2.3** package, not
against those bytes, so the production DEB still carries the open
defects below.

## Known defects on these platforms

- **B-7 — the published DEB's glibc floor.** The DEB tested here
  declares `libc6 (>= 2.36)` and the build gate
  (`xtask dist --native`) fails the build when the declared floor is
  below the binaries' real maximum GLIBC symbol version, read back with
  `objdump`. The **published** v0.2.5 package under-declares its floor;
  that is an open defect on the published asset, not on this build.
- **B-24 — the published DEB has no desktop integration.** The published
  v0.2.5 package carries only the two binaries and the copyright file:
  no desktop entry, no icon. The DEB tested here has both (stages 2 and
  3 pass), but the published asset still does not.
- **Issue #39 — the published DEB declares only `libc6`.** The DEB
  tested here declares the full X11/GL runtime set
  (`libgl1`, `libx11-6`, `libxcursor1`, `libxrandr2`, `libxi6`,
  `libxkbcommon0`, `libxkbcommon-x11-0`), so the Fedora-family cells
  install their own runtime stack. The published package does not, so
  it installs cleanly and then fails to start on a system without that
  stack already present.

## Reproducing this

```sh
cargo run -p xtask -- dist --native          # produces dist/native/
scripts/release-matrix.sh dist/native         # six cells, one per row
```

The matrix needs a Docker host. On a SELinux-enforcing host the bind
mounts carry `:z`; without it the containers cannot read the mount and
every cell fails with "Permission denied" before it runs anything.

## Relationship to the other release documents

- [ARTIFACT-MATRIX.md](ARTIFACT-MATRIX.md) — one row per artifact, with
  its evidence state and blockers.
- [EXECUTION-BOARD.md](EXECUTION-BOARD.md) — the per-unit ledger;
  T27-33 is the matrix, T27-34 is this document.
- [RECEIPT-0.2.5-DRYRUN-6.md](RECEIPT-0.2.5-DRYRUN-6.md) — the
  rehearsal receipt for the release workflow path, and the run that
  published the artifacts this table tests.
- [RECEIPT-0.2.5-DRYRUN-3.md](RECEIPT-0.2.5-DRYRUN-3.md) — the first
  rehearsal receipt; it found the source-archive and naming defects
  this table's artifacts no longer carry.
