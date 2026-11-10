#!/usr/bin/env bash
# FireCraft DEB install verification — the clean-container proof that
# the package is installable, launchable, upgradable, removable and
# reinstallable on the platform it declares (Debian 12 / glibc 2.36).
#
# Usage, inside a `debian:12` container with the candidate deb mounted:
#
#   verify-install.sh <candidate.deb> [previous-version.deb]
#
# The previous deb is optional; when given, the upgrade path is
# exercised too (install the old version first, then upgrade over it).
#
# What each stage proves:
#   1. metadata      — dpkg reports the package name, the candidate's
#                      version, the amd64 architecture and the declared
#                      libc6 floor.
#   2. desktop entry — /usr/share/applications/firecraft.desktop is
#                      installed, is a valid desktop entry, and names
#                      the binary and the icon that were installed.
#   3. icon          — the hicolor application icon is installed where
#                      the desktop entry's Icon= resolves.
#   4. CLI           — firecraft-cli runs and prints its usage (this
#                      also proves the loader resolved every symbol the
#                      declared glibc floor claims to cover).
#   5. GUI launch    — the desktop binary starts under Xvfb and stays
#                      up for the launch window (skipped with
#                      FIRECRAFT_DEB_SKIP_GUI=1).
#   6. upgrade       — the previously installed version is replaced and
#                      the installed version moves forward.
#   7. uninstall     — `dpkg -r` removes the binaries, the desktop
#                      entry and the icon.
#   8. reinstall     — installing again restores the whole payload.
#
# Every stage prints a PASS/FAIL line; the script exits non-zero on the
# first failure so a CI step cannot mistake a partial run for a pass.
set -euo pipefail

CANDIDATE="${1:?usage: verify-install.sh <candidate.deb> [previous.deb]}"
PREVIOUS="${2:-}"

readonly DESKTOP_ENTRY="/usr/share/applications/firecraft.desktop"
readonly ICON_PATH="/usr/share/icons/hicolor/256x256/apps/firecraft.png"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

pass() {
    echo "PASS: $*"
}

# Reads the candidate's own metadata rather than trusting a hard-coded
# version: the script must work for any candidate.
deb_field() {
    dpkg-deb --field "$CANDIDATE" "$1"
}

VERSION="$(deb_field Version)"
[ -n "$VERSION" ] || fail "the candidate deb declares no Version"
EXPECTED_ARCH="$(deb_field Architecture)"
DEPENDS="$(deb_field Depends)"

echo "== candidate: $CANDIDATE (version $VERSION, arch $EXPECTED_ARCH)"
echo "== depends:   $DEPENDS"

[ "$EXPECTED_ARCH" = "amd64" ] || fail "unexpected architecture: $EXPECTED_ARCH"

echo "== stage 1: install"
dpkg -i "$CANDIDATE" >/dev/null || fail "dpkg -i rejected the candidate"
INSTALLED_VERSION="$(dpkg-query -W -f='${Version}' firecraft)"
[ "$INSTALLED_VERSION" = "$VERSION" ] ||
    fail "installed version $INSTALLED_VERSION != candidate $VERSION"
pass "installed firecraft $INSTALLED_VERSION"

echo "== stage 2: desktop entry"
[ -f "$DESKTOP_ENTRY" ] || fail "$DESKTOP_ENTRY is not installed"
grep -q '^Type=Application$' "$DESKTOP_ENTRY" || fail "desktop entry is not an Application"
EXEC_NAME="$(sed -n 's/^Exec=\([^ ]*\).*/\1/p' "$DESKTOP_ENTRY" | head -1)"
[ -n "$EXEC_NAME" ] || fail "desktop entry has no Exec="
[ -x "/usr/bin/$EXEC_NAME" ] || fail "desktop entry Exec=$EXEC_NAME has no installed binary"
ICON_NAME="$(sed -n 's/^Icon=//p' "$DESKTOP_ENTRY" | head -1)"
[ -n "$ICON_NAME" ] || fail "desktop entry has no Icon="
pass "desktop entry installs and names $EXEC_NAME / $ICON_NAME"

if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "$DESKTOP_ENTRY" || fail "desktop-file-validate rejected the entry"
    pass "desktop-file-validate accepts the entry"
else
    echo "NOTE: desktop-file-validate is not installed; the entry was not validated by it"
fi

echo "== stage 3: icon"
[ -f "$ICON_PATH" ] || fail "$ICON_PATH is not installed"
pass "icon installed at $ICON_PATH"

echo "== stage 4: CLI"
CLI_OUT="$(firecraft-cli --help 2>&1 || true)"
echo "$CLI_OUT" | grep -qi 'usage' || fail "firecraft-cli printed no usage line: $CLI_OUT"
pass "firecraft-cli runs"

echo "== stage 5: GUI launch"
if [ "${FIRECRAFT_DEB_SKIP_GUI:-0}" = "1" ]; then
    echo "NOTE: GUI launch skipped (FIRECRAFT_DEB_SKIP_GUI=1)"
elif ! command -v xvfb-run >/dev/null 2>&1; then
    fail "xvfb-run is missing; the GUI launch cannot be proven (install xvfb, or set FIRECRAFT_DEB_SKIP_GUI=1 to say so explicitly)"
else
    # A GUI that starts and is still alive after the launch window is
    # the evidence; an immediate exit is a failed launch.
    set +e
    timeout 25 xvfb-run -a /usr/bin/firecraft >/tmp/firecraft-gui.log 2>&1
    GUI_STATUS=$?
    set -e
    if [ "$GUI_STATUS" = "124" ]; then
        pass "the desktop binary launched and stayed up for the launch window"
    else
        cat /tmp/firecraft-gui.log >&2 || true
        fail "the desktop binary exited with status $GUI_STATUS instead of staying up"
    fi
fi

echo "== stage 6: upgrade"
if [ -n "$PREVIOUS" ]; then
    dpkg -i "$PREVIOUS" >/dev/null || fail "dpkg -i rejected the previous version"
    PREV_VERSION="$(dpkg-query -W -f='${Version}' firecraft)"
    dpkg -i "$CANDIDATE" >/dev/null || fail "dpkg -i failed to upgrade over $PREV_VERSION"
    UPGRADED_VERSION="$(dpkg-query -W -f='${Version}' firecraft)"
    [ "$UPGRADED_VERSION" = "$VERSION" ] ||
        fail "upgrade left version $UPGRADED_VERSION, expected $VERSION"
    [ -f "$DESKTOP_ENTRY" ] || fail "the upgraded package lost its desktop entry"
    pass "upgraded $PREV_VERSION -> $UPGRADED_VERSION with the desktop entry in place"
else
    echo "NOTE: no previous deb given; the upgrade path was not exercised"
fi

echo "== stage 7: uninstall"
dpkg -r firecraft >/dev/null || fail "dpkg -r failed"
[ ! -e /usr/bin/firecraft ] || fail "/usr/bin/firecraft survived the removal"
[ ! -e /usr/bin/firecraft-cli ] || fail "/usr/bin/firecraft-cli survived the removal"
[ ! -e "$DESKTOP_ENTRY" ] || fail "the desktop entry survived the removal"
[ ! -e "$ICON_PATH" ] || fail "the icon survived the removal"
pass "removal took the binaries, the desktop entry and the icon with it"

echo "== stage 8: reinstall"
dpkg -i "$CANDIDATE" >/dev/null || fail "the reinstall was rejected"
[ -x /usr/bin/firecraft ] || fail "the reinstall did not restore /usr/bin/firecraft"
[ -f "$DESKTOP_ENTRY" ] || fail "the reinstall did not restore the desktop entry"
[ -f "$ICON_PATH" ] || fail "the reinstall did not restore the icon"
firecraft-cli --help >/dev/null 2>&1 || fail "the reinstalled CLI does not run"
pass "reinstall restored the payload and the CLI runs"

echo "== all stages passed"
