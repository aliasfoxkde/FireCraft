# T27-33 — Test the DEB and tar.gz on a second distro family and check X11/Wayland, desktop integration and fonts

> **RP:** RP-01 · **Priority:** P1 · **Status:** SMOKE TESTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-21

**Objective:** widen Linux coverage beyond the single Debian 12 container the DEB was
smoked in, across distro families and both display servers.

**Inputs:** the DEB and desktop tar.gz from T27-21; Ubuntu and Fedora-family images;
Xvfb for X11; a Wayland session for the Wayland leg.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1 — the release requires testing
across distributions, not on a single image.

**Acceptance criteria:** the DEB and the tar.gz install and launch on an Ubuntu image
and on a Fedora-family image; X11 and Wayland are each exercised; desktop integration
and font/resource loading are each checked; every failure is recorded with the
distribution and version.

**Verification:** `docker run <distro-image> bash -lc 'apt install ./firecraft_*.deb && firecraft --version'`

**Evidence:** Six cells, all passing, on 2026-11-17: Ubuntu 24.04.5 LTS, Debian 12 (bookworm) and Fedora 45, each under X11 and under Wayland. The DEB installs with apt on both Debian-family cells and passes the repository's own eight-stage package verification — metadata, desktop entry accepted by `desktop-file-validate`, icon installed, CLI, CLI document round trip (`firecraft-cli export` writes a 97 B PNG with the signature `89504e470d0a1a0a`), GUI launch under Xvfb with exit 124, uninstall and reinstall. The desktop tar.gz launches on all six cells and `fc-list` enumerates fonts on every cell. The harness is in-tree: `packaging/linux/leg-debian-family.sh`, `packaging/linux/leg-fedora-family.sh`, `packaging/linux/leg-wayland.sh` and [`scripts/release-matrix.sh`](https://github.com/aliasfoxkde/FireCraft/blob/main/scripts/release-matrix.sh). The Fedora cells record that a `.deb` is not that family's package format, so the tar.gz is the artifact under test there. The results are published in [LINUX-COMPAT.md](LINUX-COMPAT.md). **Done:** 2026-11-17: six-cell matrix green on Ubuntu 24.04, Debian 12 and Fedora 45.

**Blockers:** —

**Stopping condition:** a matrix of distro × display results exists, and the gaps are
listed explicitly rather than inferred.
