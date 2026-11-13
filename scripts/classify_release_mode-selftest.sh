#!/usr/bin/env bash
# Self-test for scripts/classify_release_mode.sh — the rule that decides
# whether a release tag is a production release or a rehearsal.
#
# The rule is the last thing standing between a rehearsal and a release
# that looks like one, so every input it does not recognise must fail
# closed, and each failure mode is demonstrated here with one deliberately
# broken input.
#
# Usage: bash scripts/classify_release_mode-selftest.sh
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
script="$root/scripts/classify_release_mode.sh"

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

# expect <exit> <label> <expected-stdout> <args...>
expect() {
  local want_exit=$1 label=$2 want=$3
  shift 3
  local out status
  out=$(bash "$script" "$@" 2>&1)
  status=$?
  if [ "$status" -ne "$want_exit" ]; then
    bad "$label" "exit $status, expected $want_exit" "$out"
    return
  fi
  if [ "$want_exit" -eq 0 ] && [ "$out" != "$want" ]; then
    bad "$label" "printed '$out', expected '$want'"
    return
  fi
  if [ "$want_exit" -ne 0 ] && [ -z "$out" ]; then
    bad "$label" "failed without a diagnosis"
    return
  fi
  ok "$label"
}

# --- the two accepted shapes --------------------------------------------
expect 0 "a plain version tag is a production release" production v0.2.6
expect 0 "a dryrun tag is a rehearsal" test v0.2.6-dryrun.1
expect 0 "a rehearsal may number itself higher" test v0.2.6-dryrun.12

# --- a version constraint ------------------------------------------------
expect 0 "a tag whose base matches the tree is accepted" production v0.2.6 0.2.6
expect 1 "a tag that names another version is refused" "" v0.2.7 0.2.6

# --- every refused shape -------------------------------------------------
expect 1 "an empty tag is refused" "" ""
expect 1 "a tag without the v prefix is refused" "" 0.2.6
expect 1 "a two-component version is refused" "" v0.2
expect 1 "a four-component version is refused" "" v0.2.6.1
expect 1 "a non-numeric version component is refused" "" v0.x.6
expect 1 "an unknown prerelease suffix is refused" "" v0.2.6-rc.1
expect 1 "a dryrun suffix without a number is refused" "" v0.2.6-dryrun
expect 1 "a dryrun suffix with a non-numeric number is refused" "" v0.2.6-dryrun.x
expect 1 "a tag ending in a hyphen is refused" "" v0.2.6-

# --- usage ---------------------------------------------------------------
out=$(bash "$script" 2>&1)
status=$?
if [ "$status" -eq 2 ] && printf '%s' "$out" | grep -q 'usage:'; then
  ok "no arguments prints the usage"
else
  bad "no arguments prints the usage" "exit $status" "$out"
fi

if [ "$failures" -ne 0 ]; then
  printf '%s of %s self-test case(s) failed\n' "$failures" "$((passed + failures))" >&2
  exit 1
fi
printf 'all %s self-test cases passed\n' "$passed"
