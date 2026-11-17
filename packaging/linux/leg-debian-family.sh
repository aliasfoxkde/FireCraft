#!/usr/bin/env bash
# The Debian-family leg of the Linux compatibility matrix (T27-33).
#
# It installs the DEB with apt — so apt itself resolves the package's
# declared dependencies — and then runs the eight-stage package
# verification, and separately unpacks the desktop tar.gz and launches
# it under X11.
#
# Run inside a container with:
#   /artifacts   the release artifacts (read-only)
#   /repo        the repository checkout (read-only)
#
# Usage (inside the container):
#   bash /repo/packaging/linux/leg-debian-family.sh
set -euo pipefail

echo "== image: $(grep PRETTY_NAME /etc/os-release)"

apt-get update -qq >/dev/null
apt-get install -y -qq --no-install-recommends \
    xvfb xauth libgl1 libx11-6 libxcursor1 libxrandr2 libxi6 \
    libxkbcommon0 libxkbcommon-x11-0 desktop-file-utils \
    fonts-dejavu-core ca-certificates fontconfig >/dev/null

echo "== install the DEB (apt resolves the declared dependencies)"
apt-get install -y -qq /artifacts/firecraft_*.deb >/dev/null

echo "== the eight-stage package verification"
bash /repo/packaging/deb/verify-install.sh \
    /artifacts/firecraft_*.deb "" /repo/corpus/fixtures/composed.fw.png

echo "== the desktop tar.gz, unpacked and run"
mkdir -p /opt/firecraft-tar
tar xzf /artifacts/firecraft-[0-9]*-linux-x86_64.tar.gz -C /opt/firecraft-tar
find /opt/firecraft-tar -maxdepth 3 -type f -name 'firecraft*' | head -10
TAR_BIN="$(find /opt/firecraft-tar -maxdepth 3 -type f -name 'firecraft' | head -1)"
[ -n "$TAR_BIN" ] || { echo "FAIL: the tar.gz carries no firecraft binary" >&2; exit 1; }
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

echo "== fonts: the desktop entry's font dependency"
if command -v fc-list >/dev/null 2>&1; then
    fc-list >/dev/null
    echo "PASS: fontconfig enumerates fonts"
else
    echo "NOTE: fc-list is unavailable"
fi

echo "== all stages passed"
