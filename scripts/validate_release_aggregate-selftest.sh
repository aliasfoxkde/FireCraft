#!/usr/bin/env bash
# Self-test for scripts/validate_release_aggregate.sh — the gate that
# decides whether a release may be created.
#
# The gate is the last thing between a build and a published release,
# and in the workflow it runs once, on a tag, with nothing around it to
# tell it apart from a run that is merely quiet. So it is exercised here
# against a synthetic artifact set, and every failure mode it claims to
# catch is demonstrated with one deliberately broken input.
#
# Usage: bash scripts/validate_release_aggregate-selftest.sh
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
script="$root/scripts/validate_release_aggregate.sh"

work="$(mktemp -d)"
trap 'find "$work" -type f -delete; find "$work" -type d -empty -delete' EXIT

version=0.2.6
failures=0
passed=0

ok() {
  printf 'ok   %s\n' "$1"
  passed=$((passed + 1))
}

bad() {
  printf 'FAIL %s\n' "$1" >&2
  shift
  for line in "$@"; do
    printf '     %s\n' "$line" >&2
  done
  failures=$((failures + 1))
}

digest() {
  sha256sum "$1" | cut -d' ' -f1
}

# Build a complete, self-consistent artifact set. Every case below is
# this set with exactly one thing broken.
make_set() {
  local dir="$1"
  mkdir -p "$dir"
  printf 'desktop\n' > "$dir/firecraft-${version}-linux-x86_64.tar.gz"
  printf 'cli\n' > "$dir/firecraft-cli-${version}-linux-x86_64.tar.gz"
  printf 'deb\n' > "$dir/firecraft_${version}_amd64.deb"
  printf 'web\n' > "$dir/firecraft-web-${version}.zip"
  printf 'source\n' > "$dir/firecraft-${version}-source.tar.gz"
  printf 'sbom\n' > "$dir/firecraft-${version}-sbom.cdx.json"
  printf 'manifest\n' > "$dir/MANIFEST.sha256"
  printf 'checksums\n' > "$dir/RELEASE_SHA256.txt"

  # The checksum list covers the digested files, by bare name.
  (
    cd "$dir"
    {
      sha256sum "firecraft-${version}-linux-x86_64.tar.gz"
      sha256sum "firecraft-cli-${version}-linux-x86_64.tar.gz"
      sha256sum "firecraft_${version}_amd64.deb"
      sha256sum "firecraft-web-${version}.zip"
      sha256sum "firecraft-${version}-source.tar.gz"
      sha256sum "firecraft-${version}-sbom.cdx.json"
      sha256sum "MANIFEST.sha256"
    } > "RELEASE_SHA256.txt"
  )

  # The release manifest describes every published asset but itself.
  local name sha size
  {
    echo '{'
    echo '  "schema": "firecraft.release-manifest/1",'
    echo "  \"version\": \"${version}\","
    echo '  "tag": null,'
    echo '  "artifacts": ['
    local first=1
    for entry in \
      "firecraft-${version}-linux-x86_64.tar.gz:x86_64-unknown-linux-gnu" \
      "firecraft-cli-${version}-linux-x86_64.tar.gz:x86_64-unknown-linux-gnu" \
      "firecraft_${version}_amd64.deb:x86_64-unknown-linux-gnu" \
      "MANIFEST.sha256:x86_64-unknown-linux-gnu" \
      "firecraft-web-${version}.zip:wasm32-unknown-unknown" \
      "firecraft-${version}-source.tar.gz:any" \
      "firecraft-${version}-sbom.cdx.json:any" \
      "RELEASE_SHA256.txt:any"; do
      name=${entry%%:*}
      local target=${entry#*:}
      sha=$(digest "$dir/$name")
      size=$(stat -c%s "$dir/$name")
      [ "$first" -eq 1 ] || echo ','
      first=0
      printf '    {"name": "%s", "sha256": "%s", "size": %s, "target": "%s"}' \
        "$name" "$sha" "$size" "$target"
    done
    echo
    echo '  ]'
    echo '}'
  } > "$dir/release-manifest.json"
}

# expect <exit> <label> <dir> [expected-message]
expect() {
  local want_exit=$1 label=$2 dir=$3
  shift 3
  local want_message=${1:-}
  local out status
  out=$(bash "$script" "$dir" "$version" 2>&1)
  status=$?
  if [ "$status" -ne "$want_exit" ]; then
    bad "$label" "exit $status, expected $want_exit" "$out"
    return
  fi
  if [ -n "$want_message" ] && ! printf '%s' "$out" | grep -qF "$want_message"; then
    bad "$label" "the diagnosis did not mention '$want_message'" "$out"
    return
  fi
  if [ "$want_exit" -ne 0 ] && [ -z "$out" ]; then
    bad "$label" "failed without a diagnosis"
    return
  fi
  ok "$label"
}

# --- a complete, self-consistent set passes ------------------------------
good="$work/good"
make_set "$good"
expect 0 "a complete artifact set passes the gate" "$good"

# --- a missing artifact --------------------------------------------------
missing="$work/missing"
make_set "$missing"
rm -f "$missing/firecraft_${version}_amd64.deb"
expect 1 "a missing artifact fails the gate" "$missing" "required artifact missing"

# --- an empty artifact ---------------------------------------------------
empty="$work/empty"
make_set "$empty"
: > "$empty/firecraft-web-${version}.zip"
expect 1 "an empty artifact fails the gate" "$empty" "required artifact empty"

# --- an unexpected extra artifact -----------------------------------------
extra="$work/extra"
make_set "$extra"
printf 'stale\n' > "$extra/firecraft-0.2.5-linux-x86_64.tar.gz"
expect 1 "an unexpected extra artifact fails the gate" "$extra" "unexpected artifact"

# --- a checksum mismatch --------------------------------------------------
mismatch="$work/mismatch"
make_set "$mismatch"
printf 'tampered\n' > "$mismatch/firecraft-web-${version}.zip"
expect 1 "a checksum mismatch fails the gate" "$mismatch"

# --- a version mismatch in the manifest -----------------------------------
wrongversion="$work/wrongversion"
make_set "$wrongversion"
sed -i "s/\"version\": \"${version}\"/\"version\": \"0.2.5\"/" "$wrongversion/release-manifest.json"
expect 1 "a manifest version mismatch fails the gate" "$wrongversion" "describes version 0.2.5"

# --- a digest mismatch in the manifest -------------------------------------
wrongdigest="$work/wrongdigest"
make_set "$wrongdigest"
sed -i "s/\"sha256\": \"[0-9a-f]\{64\}/\"sha256\": \"$(printf '0%.0s' {1..64})\"/" \
  "$wrongdigest/release-manifest.json"
expect 1 "a manifest digest mismatch fails the gate" "$wrongdigest"

# --- an asset the manifest does not list -----------------------------------
unlisted="$work/unlisted"
make_set "$unlisted"
sed -i "/firecraft-cli-${version}-linux-x86_64.tar.gz/d" "$unlisted/release-manifest.json"
expect 1 "an asset missing from the manifest fails the gate" "$unlisted" "is not listed in the release manifest"

# --- an unknown manifest schema -------------------------------------------
schema="$work/schema"
make_set "$schema"
sed -i "s/firecraft.release-manifest\/1/firecraft.release-manifest\/2/" "$schema/release-manifest.json"
expect 1 "an unknown manifest schema fails the gate" "$schema" "unknown schema"

# --- a skipped required platform -------------------------------------------
skipped="$work/skipped"
make_set "$skipped"
# The wasm bundle never arrived: its artifact and its manifest entry are
# both gone — the shape a skipped platform build leaves behind.
rm -f "$skipped/firecraft-web-${version}.zip"
sed -i "/firecraft-web-${version}.zip/d" "$skipped/release-manifest.json"
expect 1 "a skipped required platform fails the gate" "$skipped" "required platform wasm32-unknown-unknown"

# --- a platform the manifest does not represent ----------------------------
# The discriminating case for the same defect: every artifact is
# present and every digest agrees, but nothing carries the wasm
# platform's target triple — so only the platform check can catch it.
unrepresented="$work/unrepresented"
make_set "$unrepresented"
sed -i 's/"target": "wasm32-unknown-unknown"/"target": "any"/' \
  "$unrepresented/release-manifest.json"
expect 1 "a platform the manifest does not represent fails the gate" "$unrepresented" \
  "required platform wasm32-unknown-unknown"

# --- a missing artifact directory ------------------------------------------
expect 1 "a missing artifact directory fails the gate" "$work/does-not-exist" \
  "the artifact directory does not exist"

if [ "$failures" -ne 0 ]; then
  printf '%s of %s self-test case(s) failed\n' "$failures" "$((passed + failures))" >&2
  exit 1
fi
printf 'all %s self-test cases passed\n' "$passed"
