#!/usr/bin/env bash
# Self-test for scripts/verify_release_ledger.sh — the gate that keeps the
# release execution board and its 61 tickets saying the same thing.
#
# The gate is the only thing standing between a stale board and a release
# plan that reports progress it never made, so every failure it is
# supposed to catch is demonstrated here with one deliberately broken
# ledger copy.
#
# It is written in bash+awk, like the gate it tests, so the self-test
# runs wherever the gate runs — including the GitForge CI image, which
# ships no python3.
#
# Usage: bash scripts/verify_release_ledger-selftest.sh
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
script="$root/scripts/verify_release_ledger.sh"

work="$(mktemp -d)"
trap 'find "$work" -type f -delete; find "$work" -type d -empty -delete' EXIT

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

# A fresh copy of the ledger, broken one way.
reset_tree() {
  find "$work/tree" -type f -delete 2>/dev/null
  find "$work/tree" -type d -empty -delete 2>/dev/null
  mkdir -p "$work/tree/docs"
  cp -r "$root/docs/release" "$work/tree/docs/"
  cp -r "$root/docs/tasks" "$work/tree/docs/"
}

board_file() {
  printf '%s/docs/release/EXECUTION-BOARD.md\n' "$work/tree"
}

ticket_file() {
  find "$work/tree/docs/tasks" -name "$1-*.md" | head -1
}

# Rewrite one board row's cell, keeping every other byte of the row.
# The row's cells are separated by pipes that are not escaped; a `\|`
# inside a cell is part of that cell.
set_board_cell() {
  local id=$1 idx=$2 value=$3 board
  board="$(board_file)"
  awk -v want="$id" -v idx="$idx" -v value="$value" '
    function rawsplit(line, arr,   i, c, n, cur) {
      n = 0; cur = ""
      for (i = 1; i <= length(line); i++) {
        c = substr(line, i, 1)
        if (c == "\\" && substr(line, i + 1, 1) == "|") {
          cur = cur c substr(line, i + 1, 1); i++; continue
        }
        if (c == "|") { arr[++n] = cur; cur = ""; continue }
        cur = cur c
      }
      arr[++n] = cur
      return n
    }
    {
      if (substr($0, 1, 6) != "| T27-" || index($0, "| " want " ") == 0) { print; next }
      n = rawsplit($0, parts)
      if (idx + 1 > n) { print; next }
      parts[idx + 1] = value
      s = ""
      for (j = 1; j <= n; j++) s = s parts[j] "|"
      print s
      next
    }
  ' "$board" > "$board.new" && mv "$board.new" "$board"
}

# --- the mutations, one per failure the gate must catch -------------------

mutate_status() {
  sed -i 's/\*\*Status:\*\* BUILD VERIFIED/**Status:** CERTIFIED/' "$(ticket_file T27-01)"
}

mutate_evidence() {
  sed -i 's/\*\*Evidence:\*\*/**Evidence:** something else. /' "$(ticket_file T27-01)"
}

mutate_blockers() {
  sed -i 's/\*\*Blockers:\*\* —/**Blockers:** something else/' "$(ticket_file T27-01)"
}

mutate_orphan_row() {
  find "$work/tree/docs/tasks" -name 'T27-01-*.md' -delete
}

mutate_orphan_ticket() {
  printf '# T27-99\n\n**Status:** READY\n' > "$work/tree/docs/tasks/T27-99-orphan-ticket.md"
}

mutate_no_date() {
  set_board_cell T27-01 10 "—"
}

mutate_no_blocker() {
  set_board_cell T27-61 9 "—"
}

mutate_unknown_status() {
  set_board_cell T27-01 4 "ALMOST DONE"
}

mutate_malformed_row() {
  local board
  board="$(board_file)"
  awk '{
    if (substr($0, 1, 6) == "| T27-" && index($0, "| T27-01 ") > 0) { print $0 " | an extra cell"; next }
    print
  }' "$board" > "$board.new" && mv "$board.new" "$board"
}

mutate_duplicate_row() {
  local board
  board="$(board_file)"
  awk '{
    print
    if (substr($0, 1, 6) == "| T27-" && index($0, "| T27-01 ") > 0) print
  }' "$board" > "$board.new" && mv "$board.new" "$board"
}

# expect <label> <grep-pattern> <mutation>
expect_fails() {
  local label=$1 pattern=$2 mutation=$3
  reset_tree
  "$mutation"
  local out status
  out=$(bash "$script" "$work/tree" 2>&1)
  status=$?
  if [ "$status" -eq 0 ]; then
    bad "$label" "the verifier passed a broken ledger"
    return
  fi
  if ! printf '%s' "$out" | grep -q "$pattern"; then
    bad "$label" "the failure did not name $pattern" "$out"
    return
  fi
  ok "$label"
}

expect_passes() {
  local label=$1
  local out status
  out=$(bash "$script" "$root" 2>&1)
  status=$?
  if [ "$status" -ne 0 ]; then
    bad "$label" "exit $status" "$out"
    return
  fi
  ok "$label"
}

# --- the ledger as committed ---------------------------------------------
expect_passes "the committed ledger has no problems"

# --- one broken thing at a time -------------------------------------------

expect_fails "a ticket whose status disagrees with the board is caught" \
  "STATUS T27-" mutate_status

expect_fails "a ticket whose evidence disagrees with the board is caught" \
  "EVIDENCE T27-" mutate_evidence

expect_fails "a ticket whose blockers disagree with the board is caught" \
  "BLOCKERS T27-" mutate_blockers

expect_fails "a board row with no ticket is caught" \
  "ORPHAN ROW" mutate_orphan_row

expect_fails "a ticket with no board row is caught" \
  "ORPHAN TICKET" mutate_orphan_ticket

expect_fails "an advanced status with no completion date is caught" \
  "NO DATE" mutate_no_date

expect_fails "a blocked row with no named blocker is caught" \
  "NO BLOCKER" mutate_no_blocker

expect_fails "a status outside the board's vocabulary is caught" \
  "UNKNOWN STATUS" mutate_unknown_status

expect_fails "a row whose cell count is wrong is caught" \
  "MALFORMED ROW" mutate_malformed_row

expect_fails "two rows claiming one id are caught" \
  "DUPLICATE ROW" mutate_duplicate_row

if [ "$failures" -ne 0 ]; then
  printf '%s of %s self-test case(s) failed\n' "$failures" "$((passed + failures))" >&2
  exit 1
fi
printf 'all %s self-test cases passed\n' "$passed"
