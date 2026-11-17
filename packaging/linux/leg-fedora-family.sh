#!/usr/bin/env bash
# The Fedora-family leg of the Linux compatibility matrix (T27-33).
#
# The DEB is a Debian package format: no package manager on this family
# can install it. That is a fact about the format, not a gap in the
# test, so the leg says so out loud and then covers the *tar.gz* — the
# format-agnostic artifact — on this family instead.
#
# Run inside a container with:
#   /artifacts   the release artifacts (read-only)
#   /repo        the repository checkout (read-only)
#
# Usage (inside the container):
#   bash /repo/packaging/linux/leg-fedora-family.sh
set -euo pipefail

echo "== image: $(grep PRETTY_NAME /etc/os-release)"

dnf install -y -q \
    xorg-x11-server-Xvfb mesa-libGL libXcursor libXrandr libXi \
    libxkbcommon libxkbcommon-x11 mesa-dri-drivers dejavu-sans-fonts \
    fontconfig tar gzip >/dev/null

echo "== the DEB is not installable on this family"
if command -v dpkg >/dev/null 2>&1; then
    echo "NOTE: dpkg is present but a .deb is not this family's package format"
else
    echo "NOTE: no dpkg in this image; a .deb cannot be installed on this family"
fi

echo "== the desktop tar.gz, unpacked and run"
mkdir -p /opt/firecraft-tar
tar xzf /artifacts/firecraft-[0-9]*-linux-x86_64.tar.gz -C /opt/firecraft-tar
find /opt/firecraft-tar -maxdepth 3 -type f -name 'firecraft*' | head -10
TAR_BIN="$(find /opt/firecraft-tar -maxdepth 3 -type f -name 'firecraft' | head -1)"
[ -n "$TAR_BIN" ] || { echo "FAIL: the tar.gz carries no firecraft binary" >&2; exit 1; }
TAR_CLI="$(find /opt/firecraft-tar -maxdepth 3 -type f -name 'firecraft-cli' | head -1)"
if [ -n "$TAR_CLI" ]; then
    "$TAR_CLI" --help | head -3
    echo "PASS: the tar.gz CLI runs"
fi
set +e
timeout 25 xvfb-run -a "$TAR_BIN" >/tmp/tar-gui.log 2>&1
TAR_STATUS=$?
set -e
if [ "$TAR_STATUS" = "124" ]; then
    echo "PASS: the tar.gz desktop binary launched and stayed up"
else
    cat /tmp/tar-gui.log >&2 || true
    echo "FAIL: the tar.gz desktop binary exited with status $TAR_STATUS" >&2
    exit 1
fi

if command -v fc-list >/dev/null 2>&1; then
    fc-list >/dev/null
    echo "PASS: fontconfig enumerates fonts"
else
    echo "NOTE: fc-list is unavailable"
fi

echo "== all stages passed"
