#!/usr/bin/env bash
# Self-test for scripts/validate_workflow_pins.py.
#
# The ticket that asks for the workflow hardening asks for the failure
# modes to be *demonstrated* — one deliberately broken input each — so
# this script feeds the validator the inputs it must reject and the
# inputs it must accept, and checks the exit code each time.
#
# Usage: bash scripts/validate_workflow_pins-selftest.sh
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
validator="$root/scripts/validate_workflow_pins.py"
work="$(mktemp -d)"
trap 'find "$work" -type f -delete; find "$work" -type d -empty -delete' EXIT

failures=0

expect() {
  # expect <wanted-exit> <label> <file-content>
  local wanted="$1" label="$2" content="$3"
  local dir="$work/$label"
  mkdir -p "$dir"
  printf '%s\n' "$content" > "$dir/wf.yml"
  set +e
  output="$(python3 -I "$validator" "$dir" 2>&1)"
  got=$?
  set -e
  if [ "$got" -ne "$wanted" ]; then
    printf 'FAIL %s: expected exit %s, got %s\n%s\n' "$label" "$wanted" "$got" "$output" >&2
    failures=$((failures + 1))
  else
    printf 'ok   %s (exit %s)\n' "$label" "$got"
  fi
}

sha="11d5960a326750d5838078e36cf38b85af677262"

expect 0 "pinned-action" "jobs:
  a:
    steps:
      - uses: actions/checkout@$sha # v4.4.0"

expect 0 "local-action" "jobs:
  a:
    steps:
      - uses: ./scripts/local-action"

expect 1 "mutable-tag" "jobs:
  a:
    steps:
      - uses: actions/checkout@v4"

expect 1 "mutable-branch" "jobs:
  a:
    steps:
      - uses: dtolnay/rust-toolchain@stable"

expect 1 "docker-image" "jobs:
  a:
    steps:
      - uses: docker://alpine:3.20"

expect 1 "short-sha" "jobs:
  a:
    steps:
      - uses: actions/checkout@11d5960"

if [ "$failures" -ne 0 ]; then
  printf '%s self-test case(s) failed\n' "$failures" >&2
  exit 1
fi
printf 'all self-test cases passed\n'
