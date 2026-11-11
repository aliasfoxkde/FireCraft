# T27-33 — Test the DEB and tar.gz on a second distro family and check X11/Wayland, desktop integration and fonts

> **RP:** RP-01 · **Priority:** P1 · **Status:** NOT STARTED
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

**Evidence:** only Debian 12 has been exercised for the DEB.

**Blockers:** no Fedora-family image is available locally.

**Stopping condition:** a matrix of distro × display results exists, and the gaps are
listed explicitly rather than inferred.
