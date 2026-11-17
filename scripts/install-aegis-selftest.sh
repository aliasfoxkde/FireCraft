#!/usr/bin/env bash
# install-aegis-selftest.sh — exercise scripts/install-aegis.sh's failure modes.
#
# The installer is now called by three CI jobs, and it is the only place
# the scanner's URL and checksum are pinned. Its behaviour has to be
# right in the cases that are not the happy path:
#
#   1. a tarball whose checksum does not match the pin installs nothing;
#   2. a tarball that carries the right checksum but the wrong program
#      (an unrelated tool also ships under the bare name `aegis`) is
#      refused by the identity check;
#   3. a machine with no curl, no wget and no python3 — the CI image —
#      still installs the scanner, because the script brings its own
#      fetch.
#
# The self-test copies the installer and rewrites its URL and checksum
# onto a local fixture, so the real pin is never exercised and never
# weakened: the copy is a throwaway in a temporary directory.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
installer="${repo_root}/scripts/install-aegis.sh"

work="$(mktemp -d)"
cleanup() {
  find "$work" -depth -delete 2>/dev/null || true
}
trap cleanup EXIT

fail() {
  echo "SELFTEST FAILED: $*" >&2
  exit 1
}

pass() {
  echo "ok - $*"
}

# --- the fixtures: tarballs that carry an `aegis` member -----------------

# make_fixture <directory> <--help text> <tarball name>
make_fixture() {
  local dir="$1" help_text="$2" tarball="$3"
  mkdir -p "$dir"
  cat > "${dir}/aegis" <<EOF
#!/bin/sh
case "\${1:-}" in
  --help) echo "${help_text}" ;;
  --version) echo "aegis 0.6.3" ;;
esac
exit 0
EOF
  chmod 0755 "${dir}/aegis"
  tar czf "${work}/${tarball}" -C "$dir" aegis
}

# The real installer, with its pin rewritten onto a fixture.
# make_installer <expected checksum> <tarball the server should serve>
make_installer() {
  local expected="$1" tarball="$2"
  fixture_url="${fixture_base}/${tarball}"
  sed -e "s|^AEGIS_URL=.*|AEGIS_URL=\"${fixture_url}\"|" \
      -e "s|^AEGIS_SHA256=.*|AEGIS_SHA256=\"${expected}\"|" \
      "$installer" > "${work}/installer.sh"
  chmod 0755 "${work}/installer.sh"
}

# A local HTTP server for the fixtures: the CI image ships no python3, so
# the server is node, which the image does ship.
start_server() {
  cat > "${work}/server.js" <<'EOF'
const http = require("http");
const fs = require("fs");
const dir = process.argv[2];
const server = http.createServer((req, res) => {
  const path = `${dir}/${req.url.replace(/^\//, "")}`;
  if (!fs.existsSync(path)) {
    res.writeHead(404);
    res.end("not found");
    return;
  }
  res.writeHead(200);
  fs.createReadStream(path).pipe(res);
});
server.listen(0, "127.0.0.1", () => {
  console.log(`port=${server.address().port}`);
});
EOF
  node "${work}/server.js" "${work}" > "${work}/server.log" 2>&1 &
  server_pid=$!
  for _ in $(seq 1 50); do
    port="$(sed -n 's/^port=//p' "${work}/server.log")"
    if [ -n "$port" ]; then
      break
    fi
    sleep 0.2
  done
  [ -n "${port:-}" ] || fail "the fixture server did not start"
  fixture_base="http://127.0.0.1:${port}"
}

cleanup_server() {
  if [ -n "${server_pid:-}" ]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
    server_pid=""
  fi
}

# A PATH with node but without curl, wget or python3 — the CI image's
# PATH, in effect.
node_only_path() {
  local dir="${work}/nodeonly"
  mkdir -p "$dir"
  local tool found
  for tool in node bash sh mktemp rm mkdir install grep sha256sum tar gzip sed cat chmod ls dirname env; do
    found="$(command -v "$tool" || true)"
    [ -n "$found" ] || continue
    ln -sf "$found" "${dir}/${tool}"
  done
  echo "$dir"
}

make_fixture "${work}/good" "aegis 0.6.3
  scan    Scan a directory for secrets" good.tar.gz
good_sha="$(sha256sum "${work}/good.tar.gz" | cut -d' ' -f1)"

make_fixture "${work}/wrong" "aegis 0.6.3
  init    Create a security file" wrong.tar.gz
wrong_sha="$(sha256sum "${work}/wrong.tar.gz" | cut -d' ' -f1)"

start_server
trap 'cleanup_server; cleanup' EXIT

# --- case 1: the checksum matches and the program is the scanner -------

make_installer "$good_sha" good.tar.gz
target="${work}/bin-good"
if ! bash "${work}/installer.sh" "$target" > "${work}/case1.log" 2>&1; then
  cat "${work}/case1.log" >&2
  fail "case 1: a matching checksum did not install"
fi
[ -x "${target}/aegis" ] || fail "case 1: the scanner was not installed"
pass "a matching checksum installs the scanner"

# --- case 2: the checksum does not match -------------------------------

# The checksum of the other fixture: a real digest, and the wrong one for
# the tarball the server is serving.
make_installer "$wrong_sha" good.tar.gz
target="${work}/bin-badsum"
if bash "${work}/installer.sh" "$target" > "${work}/case2.log" 2>&1; then
  fail "case 2: a mismatched checksum installed something"
fi
[ ! -e "${target}/aegis" ] || fail "case 2: a mismatched checksum left a binary behind"
grep -q "FAILED" "${work}/case2.log" || fail "case 2: the failure was not reported"
pass "a mismatched checksum installs nothing and says so"

# --- case 3: the right checksum, the wrong program ---------------------

make_installer "$wrong_sha" wrong.tar.gz
target="${work}/bin-wrong"
if bash "${work}/installer.sh" "$target" > "${work}/case3.log" 2>&1; then
  fail "case 3: a program that is not the scanner was accepted"
fi
grep -q "not the Aegis scanner" "${work}/case3.log" || fail "case 3: the refusal was not explained"
pass "a program that is not the scanner is refused"

# --- case 4: no curl, no wget, no python3 ------------------------------

make_installer "$good_sha" good.tar.gz
target="${work}/bin-nodeonly"
restricted="$(node_only_path)"
if ! env -i PATH="$restricted" HOME="${HOME:-/tmp}" \
  bash "${work}/installer.sh" "$target" > "${work}/case4.log" 2>&1; then
  cat "${work}/case4.log" >&2
  fail "case 4: the installer needs a downloader the CI image does not ship"
fi
[ -x "${target}/aegis" ] || fail "case 4: the scanner was not installed without curl or wget"
pass "the installer's own fetch works without curl, wget or python3"

cleanup_server
echo "install-aegis-selftest: all cases passed"
