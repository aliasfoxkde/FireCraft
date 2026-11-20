#!/usr/bin/env bash
# The Linux compatibility matrix (T27-33) — three distro families × two
# display servers, six cells, each in its own container.
#
#   Usage: scripts/release-matrix.sh <artifacts-dir> [results-dir]
#
# <artifacts-dir> must hold the DEB and the desktop tar.gz, e.g. the
# output of `cargo run -p xtask -- dist --native`.
#
# The leg scripts and the package verifier come from the checkout, so
# the matrix tests the repository's own verification, not a copy of it.
#
# The `:z` on every bind mount is not decoration: on a SELinux-enforcing
# host an unlabelled mount from /var/tmp is unreadable inside the
# container, and the cell fails with "Permission denied" before it can
# run anything.
#
# Exit status is 0 only when every cell passed.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ART="${1:?usage: release-matrix.sh <artifacts-dir> [results-dir]}"
RESULTS="${2:-/var/tmp/firecraft-matrix/results}"
mkdir -p "$RESULTS"
: >"$RESULTS/matrix.tsv"

run_cell() {
    local label="$1" image="$2" script="$3" bin="$4"
    local log="$RESULTS/$label.log"
    echo "== cell: $label ($image)"
    local status=0
    docker run --rm \
        -v "$ART:/artifacts:ro,z" \
        -v "$REPO_ROOT:/repo:ro,z" \
        -e FIRECRAFT_BIN="$bin" \
        "$image" bash "/repo/packaging/linux/$script" >"$log" 2>&1 || status=$?
    if [ "$status" = 0 ]; then
        echo "   PASS  $label  (log: $log)"
    else
        echo "   FAIL  $label  exit=$status  (log: $log)"
        tail -40 "$log" | sed 's/^/      | /'
    fi
    printf '%s\t%s\t%s\n' "$label" "$image" "$status" >>"$RESULTS/matrix.tsv"
    return 0
}

# --- Debian family: the DEB installs with apt, plus the tar.gz --------
run_cell ubuntu-24.04-x11     ubuntu:24.04   leg-debian-family.sh /usr/bin/firecraft
run_cell debian-12-x11        debian:12-slim  leg-debian-family.sh /usr/bin/firecraft
run_cell ubuntu-24.04-wayland ubuntu:24.04   leg-wayland.sh        /usr/bin/firecraft
run_cell debian-12-wayland    debian:12-slim  leg-wayland.sh        /usr/bin/firecraft

# --- Fedora family: the DEB is not this family's package format, so the
#     tar.gz is the artifact under test there.
run_cell fedora-x11           fedora:latest   leg-fedora-family.sh ""
run_cell fedora-wayland       fedora:latest   leg-wayland.sh        ""

echo "== matrix"
column -t -s $'\t' "$RESULTS/matrix.tsv"

if grep -qv $'\t0$' "$RESULTS/matrix.tsv"; then
    echo "== at least one cell failed" >&2
    exit 1
fi
echo "== every cell passed"
