#!/usr/bin/env bash
# classify_release_mode.sh — decide whether a release tag is a production
# release or a rehearsal, and refuse anything it does not recognise.
#
# Why this is a script and not a shell expression inside the workflow:
#
#   The release job runs on a tag push. The tag is the only thing that
#   says whether the run is cutting a release or rehearsing one, and the
#   two must not be confused — a rehearsal that looks like a release is
#   the failure that publishes the wrong thing. The rule is small, but
#   "small" is exactly what makes a rule likely to be edited carelessly
#   later, so it lives in one place, is exercised by its own self-test,
#   and fails closed on every input it does not recognise.
#
# The two shapes:
#
#   v<MAJOR>.<MINOR>.<PATCH>            → production
#   v<MAJOR>.<MINOR>.<PATCH>-dryrun.<N>  → test (a rehearsal)
#
# Everything else is an error: a two-component version, a tag without
# the `v` prefix, an unknown prerelease suffix (`-rc.1`), a dryrun
# suffix without a number, a non-numeric dryrun number, and — when a
# version is supplied — a tag whose base version is not that version.
#
# Usage:
#   classify_release_mode.sh <tag> [expected-version]
#
# Prints `production` or `test` on stdout and exits 0, or prints a
# diagnosis on stderr and exits 1.

set -euo pipefail

die() {
  echo "::error::$1" >&2
  exit 1
}

usage() {
  echo "usage: classify_release_mode.sh <tag> [expected-version]" >&2
}

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  usage
  exit 2
fi

tag=$1
expected=${2:-}

[ -n "$tag" ] || die "the release tag is empty"

# The `v` prefix is what every release tag in this repository carries.
case "$tag" in
  v*) ;;
  *) die "the release tag '$tag' does not start with 'v'" ;;
esac

body=${tag#v}

# Split the base version from any suffix. The base is three numeric
# components; anything else is not a version this repository has ever
# released.
base=$body
suffix=""
hyphenated=0
case "$body" in
  *-*)
    hyphenated=1
    base=${body%%-*}
    suffix=${body#*-}
    ;;
esac

case "$base" in
  *.*.*) ;;
  *) die "the release tag '$tag' does not name a MAJOR.MINOR.PATCH version" ;;
esac

IFS=. read -r major minor patch <<< "$base"
for part in "$major" "$minor" "$patch"; do
  case "$part" in
    ''|*[!0-9]*) die "the release tag '$tag' has a non-numeric version component '$part'" ;;
  esac
done

if [ -n "$expected" ] && [ "$base" != "$expected" ]; then
  die "the release tag '$tag' names version $base, the tree builds $expected"
fi

if [ -z "$suffix" ]; then
  if [ "$hyphenated" -eq 1 ]; then
    die "the release tag '$tag' ends in '-' with nothing after it"
  fi
  echo "production"
  exit 0
fi

# The only suffix this repository recognises is the rehearsal marker.
case "$suffix" in
  dryrun.*) ;;
  *)
    die "the release tag '$tag' carries the unknown suffix '-${suffix}' — only '-dryrun.<n>' is a rehearsal"
    ;;
esac

number=${suffix#dryrun.}
case "$number" in
  ''|*[!0-9]*) die "the rehearsal tag '$tag' must number its rehearsal: '-dryrun.<n>'" ;;
esac

echo "test"
