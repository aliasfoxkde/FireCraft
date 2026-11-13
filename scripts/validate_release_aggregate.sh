#!/usr/bin/env bash
# validate_release_aggregate.sh — the fail-closed gate between "the
# artifacts were built" and "a release exists".
#
# The gate answers one question: is the set of files in the artifact
# directory exactly the published release for one version, with every
# digest agreeing with the manifest and the checksum list? It refuses,
# loudly and specifically, when:
#
#   * a required artifact is missing;
#   * a required artifact is empty;
#   * an artifact is present that the release does not publish (a stale
#     file from an earlier version, an artifact nobody declared);
#   * a digested file disagrees with RELEASE_SHA256.txt;
#   * the release manifest carries an unknown schema;
#   * the release manifest describes a different version than the build;
#   * an artifact's digest or size disagrees with the manifest;
#   * a published asset is missing from the manifest;
#   * an artifact's name does not carry the version;
#   * a required platform produced no artifact (a skipped build).
#
# Every check is reported; the script exits 1 if any of them failed, so
# the caller never creates a release from a partial set.
#
# Usage: validate_release_aggregate.sh <artifacts-dir> <version>
#
# The version is the version the artifacts carry — the workspace
# version, not the tag: a rehearsal's tag is a disposable name and the
# artifacts still describe the version they were built from.

set -uo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: validate_release_aggregate.sh <artifacts-dir> <version>" >&2
  exit 2
fi

artifacts=$1
version=$2

[ -d "$artifacts" ] || {
  echo "::error::the artifact directory does not exist: $artifacts"
  exit 1
}
[ -n "$version" ] || {
  echo "::error::the version must not be empty"
  exit 1
}

fail=0

# The manifest's digest list is written to a scratch file for
# `sha256sum -c`. The runner's own temp directory is used when it
# exists; otherwise a temporary directory is made and removed.
scratch=$(mktemp -d)
trap 'find "$scratch" -type f -delete; find "$scratch" -type d -empty -delete' EXIT

fail_with() {
  echo "::error::$1"
  fail=1
}

# The published asset set for this version. This is a fixed table, not a
# scan of the directory: the release names every asset explicitly.
expected=(
  "firecraft-${version}-linux-x86_64.tar.gz"
  "firecraft-cli-${version}-linux-x86_64.tar.gz"
  "firecraft_${version}_amd64.deb"
  "firecraft-web-${version}.zip"
  "firecraft-${version}-source.tar.gz"
  "firecraft-${version}-sbom.cdx.json"
  "MANIFEST.sha256"
  "RELEASE_SHA256.txt"
  "release-manifest.json"
)

# The platforms this release must ship. A platform whose build was
# skipped must fail the gate: a release that quietly ships one platform
# is not the release the manifest claims to describe.
required_targets=(
  "x86_64-unknown-linux-gnu"
  "wasm32-unknown-unknown"
)

# --- every required artifact is present and non-empty ---------------------
for name in "${expected[@]}"; do
  path="$artifacts/$name"
  if [ ! -f "$path" ]; then
    fail_with "required artifact missing: $name"
  elif [ ! -s "$path" ]; then
    fail_with "required artifact empty: $name"
  fi
done

# --- nothing else is in the directory -------------------------------------
# Without nullglob an unmatched glob expands to the literal pattern,
# which would be validated as a file name.
shopt -s nullglob
for path in "$artifacts"/*; do
  name=${path##*/}
  known=0
  for wanted in "${expected[@]}"; do
    if [ "$name" = "$wanted" ]; then
      known=1
      break
    fi
  done
  if [ "$known" -eq 0 ]; then
    # A file the release does not publish. The common case is a stale
    # artifact from an earlier version that the upload carried along.
    fail_with "unexpected artifact in the release set: $name"
  fi
done

# --- the digests agree with the checksum list ----------------------------
if [ -f "$artifacts/RELEASE_SHA256.txt" ]; then
  (cd "$artifacts" && sha256sum -c RELEASE_SHA256.txt) || fail=1
else
  fail_with "the checksum list is missing: RELEASE_SHA256.txt"
fi

# --- the manifest agrees with the assets it describes ----------------------
manifest="$artifacts/release-manifest.json"
if [ -f "$manifest" ]; then
  if ! command -v jq >/dev/null 2>&1; then
    fail_with "jq is required to validate the release manifest"
  else
    if ! schema=$(jq -r '.schema // ""' "$manifest"); then
      fail_with "the release manifest is not valid JSON"
    elif [ "$schema" != "firecraft.release-manifest/1" ]; then
      fail_with "the release manifest carries an unknown schema: $schema"
    fi
    if ! described=$(jq -r '.version // ""' "$manifest"); then
      fail_with "the release manifest is not valid JSON"
    elif [ "$described" != "$version" ]; then
      fail_with "the release manifest describes version $described, the build produced $version"
    fi
    # Every artifact the manifest lists must be present with the recorded
    # digest and the recorded size.
    if jq -r '.artifacts[] | "\(.sha256)  \(.name)"' "$manifest" > "$scratch/manifest.sha256" 2>/dev/null; then
      (cd "$artifacts" && sha256sum -c "$scratch/manifest.sha256") || fail=1
    else
      fail_with "the release manifest lists no artifacts"
    fi
    while read -r name size; do
      if [ -z "$name" ]; then
        continue
      fi
      actual=$(stat -c%s "$artifacts/$name" 2>/dev/null || echo "")
      if [ "$actual" != "$size" ]; then
        fail_with "the release manifest records $size B for $name, the asset is ${actual:-missing} B"
      fi
    done < <(jq -r '.artifacts[] | "\(.name) \(.size)"' "$manifest")
    # Every published asset must be listed in the manifest: a manifest
    # that omits an asset is as wrong as one that misdescribes it.
    for name in "${expected[@]}"; do
      if [ "$name" != "release-manifest.json" ]; then
        if ! jq -e --arg n "$name" '.artifacts | any(.name == $n)' "$manifest" >/dev/null; then
          fail_with "$name is not listed in the release manifest"
        fi
      fi
    done
    # Every required platform must be represented. A platform whose build
    # was skipped leaves no artifact behind, and a release that ships one
    # platform is not the release this manifest describes.
    for target in "${required_targets[@]}"; do
      if ! jq -e --arg t "$target" '.artifacts | any(.target == $t)' "$manifest" >/dev/null; then
        fail_with "required platform $target produced no artifact"
      fi
    done
  fi
fi

if [ "$fail" -ne 0 ]; then
  echo "::error::aggregate validation failed — refusing to create the release"
  exit 1
fi
echo "aggregate validation ok: ${#expected[@]} required artifacts present, digests verified"
