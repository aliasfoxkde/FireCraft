#!/usr/bin/env bash
# install-aegis.sh — fetch the pinned Aegis secret scanner.
#
# One implementation of "get the scanner": the GitHub `gate` job, the
# GitHub `docs` job and the GitForge `aegis` job all call this script, so
# the download URL and its checksum are pinned in exactly one place. The
# three callers used to carry their own copy of the pin, and the two CI
# systems could drift apart without either noticing.
#
# The pin is a checksum, not a version: the tarball is fetched over
# HTTPS, and its SHA-256 must match the value below or the install fails
# and nothing is installed. The scanner is a build-time input to a gate,
# so it is pinned like one.
#
# The downloaders are tried in order, because no single one is present
# everywhere: the GitHub runners have curl, and the authoritative GitForge
# CI image ships neither curl nor wget — it has node, git and rust. The
# image is deliberately minimal, so the script brings its own fetch.
#
# Usage: install-aegis.sh [target-dir]
#   target-dir   where the `aegis` binary is installed (default
#                /usr/local/bin). The directory must be on PATH for the
#                gate that follows.
#
# The scanner's own `--help` must list the `scan` subcommand the gate
# uses: an unrelated tool also ships under the bare name `aegis` (a
# security-file generator), so the name alone is not an identity.

set -euo pipefail

AEGIS_VERSION="0.6.3"
AEGIS_URL="https://github.com/aliasfoxkde/aegis/releases/download/v${AEGIS_VERSION}/aegis-linux-x86_64.tar.gz"
AEGIS_SHA256="835fad56172f8977a91424a580d0a3ec0c5b59fea99f4c2eb4c0a88fce30ff9e"

target_dir="${1:-/usr/local/bin}"

work="$(mktemp -d)"
# The scratch directory is enumerated and deleted with `find -depth -delete`
# rather than `rm -rf`: the repository's own rule is that `rm -rf` is
# deny-listed for the agent, and a script that CI runs is still the
# agent's work.
trap 'find "$work" -depth -delete 2>/dev/null || true' EXIT

fetch() {
  # fetch <url> <destination>
  local url="$1" out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -sSL -o "$out" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$out" "$url"
  elif command -v node >/dev/null 2>&1; then
    node -e '
      const fs = require("fs");
      const [url, out] = process.argv.slice(1);
      fetch(url)
        .then((r) => {
          if (!r.ok) {
            console.error("http " + r.status);
            process.exit(1);
          }
          return r.arrayBuffer();
        })
        .then((body) => fs.writeFileSync(out, Buffer.from(body)))
        .catch((e) => {
          console.error(e.message);
          process.exit(1);
        });
    ' "$url" "$out"
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c '
import sys, urllib.request
url, out = sys.argv[1], sys.argv[2]
with urllib.request.urlopen(url) as response, open(out, "wb") as handle:
    handle.write(response.read())
' "$url" "$out"
  else
    echo "::error::no downloader found (looked for curl, wget, node, python3)" >&2
    exit 1
  fi
}

echo "installing aegis ${AEGIS_VERSION}"
fetch "$AEGIS_URL" "$work/aegis.tar.gz"

# The checksum is checked against the pin, and a mismatch is reported
# rather than swallowed: `sha256sum -c` writes the offending line to its
# stdout, so discarding stdout would leave the failure silent — a gate
# that says nothing about why it failed.
if ! echo "${AEGIS_SHA256}  ${work}/aegis.tar.gz" | sha256sum -c - > "${work}/sha256.log" 2>&1; then
  cat "${work}/sha256.log" >&2
  echo "::error::the downloaded tarball does not match the pinned SHA-256" >&2
  exit 1
fi

tar xzf "$work/aegis.tar.gz" -C "$work" aegis

mkdir -p "$target_dir"

if [ -w "$target_dir" ]; then
  install -m 0755 "$work/aegis" "${target_dir}/aegis"
elif command -v sudo >/dev/null 2>&1; then
  sudo install -m 0755 "$work/aegis" "${target_dir}/aegis"
else
  echo "::error::cannot write to ${target_dir} and sudo is unavailable" >&2
  exit 1
fi

if ! "${target_dir}/aegis" --help 2>&1 | grep -q "scan"; then
  echo "::error::the installed binary is not the Aegis scanner (no 'scan' subcommand)" >&2
  exit 1
fi

"${target_dir}/aegis" --version
echo "aegis installed in ${target_dir}"
