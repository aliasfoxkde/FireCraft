#!/usr/bin/env bash
# The Wayland leg of the Linux compatibility matrix (T27-33).
#
# It runs the desktop binary against a real Wayland compositor (weston's
# headless backend) and proves the window comes up there, not only under
# X11.
#
# The cell is self-contained: it puts a FireCraft binary on the box
# before it launches anything. On a Debian-family image that means
# installing the DEB (the artifact under test); on a Fedora-family image
# it means unpacking the desktop tar.gz, because a .deb is not that
# family's package format.
#
# Run inside a container with:
#   /artifacts   the release artifacts (read-only)
#   /repo        the repository checkout (read-only)
#
# Usage (inside the container):
#   bash /repo/packaging/linux/leg-wayland.sh
set -euo pipefail

echo "== image: $(grep PRETTY_NAME /etc/os-release)"

if command -v apt-get >/dev/null 2>&1; then
    apt-get install -y -qq --no-install-recommends \
        weston wayland-protocols libwayland-client0 libwayland-server0 \
        libxkbcommon0 libxkbcommon-x11-0 libgl1 libx11-6 libxcursor1 \
        libxrandr2 libxi6 fonts-dejavu-core fontconfig >/dev/null
else
    dnf install -y -q weston libwayland-client libwayland-server \
        libwayland-cursor libxkbcommon libxkbcommon-x11 mesa-libGL \
        libXcursor libXrandr libXi dejavu-sans-fonts fontconfig >/dev/null
fi

# --- put a desktop binary on the box ---------------------------------
BIN="${FIRECRAFT_BIN:-}"
if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
    if command -v apt-get >/dev/null 2>&1 && ls /artifacts/firecraft_*.deb >/dev/null 2>&1; then
        echo "== install the DEB (apt resolves the declared dependencies)"
        apt-get install -y -qq /artifacts/firecraft_*.deb >/dev/null
        BIN=/usr/bin/firecraft
    else
        echo "== the tar.gz is this family's artifact; unpack it"
        mkdir -p /opt/firecraft-tar
        tar xzf /artifacts/firecraft-[0-9]*-linux-x86_64.tar.gz -C /opt/firecraft-tar
        BIN="$(find /opt/firecraft-tar -maxdepth 3 -type f -name firecraft | head -1)"
    fi
fi
[ -n "$BIN" ] && [ -x "$BIN" ] ||
    { echo "FAIL: no firecraft desktop binary is available to launch" >&2; exit 1; }
echo "== the binary under test: $BIN"

export XDG_RUNTIME_DIR=/run/firecraft-wayland
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

echo "== start the headless Wayland compositor"
weston --backend=headless-backend.so --socket=wayland-0 --width=1280 --height=800 \
    >/tmp/weston.log 2>&1 &
WESTON_PID=$!
for _ in $(seq 1 30); do
    [ -S "$XDG_RUNTIME_DIR/wayland-0" ] && break
    sleep 1
done
if [ ! -S "$XDG_RUNTIME_DIR/wayland-0" ]; then
    cat /tmp/weston.log >&2 || true
    echo "FAIL: the compositor never created its socket" >&2
    kill "$WESTON_PID" 2>/dev/null || true
    exit 1
fi
echo "PASS: weston is serving $XDG_RUNTIME_DIR/wayland-0"

echo "== launch the desktop binary on Wayland"
set +e
WAYLAND_DISPLAY=wayland-0 timeout 25 "$BIN" >/tmp/wayland-gui.log 2>&1
WAYLAND_STATUS=$?
set -e
kill "$WESTON_PID" 2>/dev/null || true

if [ "$WAYLAND_STATUS" = "124" ]; then
    echo "PASS: the desktop binary launched on Wayland and stayed up"
else
    cat /tmp/wayland-gui.log >&2 || true
    echo "FAIL: the desktop binary exited with status $WAYLAND_STATUS on Wayland" >&2
    exit 1
fi

if command -v fc-list >/dev/null 2>&1; then
    fc-list >/dev/null
    echo "PASS: fontconfig enumerates fonts"
else
    echo "NOTE: fc-list is unavailable"
fi

echo "== all stages passed"
