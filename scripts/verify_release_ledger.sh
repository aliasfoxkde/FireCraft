#!/usr/bin/env bash
# verify_release_ledger.sh — the gate that keeps the release execution
# board and its tickets saying the same thing.
#
# The board (`docs/release/EXECUTION-BOARD.md` §6) is the ledger: one row
# per logical change, with a status, the evidence for that status, the
# blockers, and the date it was reached. Each row has exactly one ticket
# under `docs/tasks/`, and the ticket restates the row. This gate refuses,
# loudly and specifically, when:
#
#   * a board row has no ticket (ORPHAN ROW);
#   * a ticket has no board row (ORPHAN TICKET);
#   * a row's cell count is not the ledger's ten columns (MALFORMED ROW);
#   * two rows claim the same id (DUPLICATE ROW);
#   * a row carries a status outside the board's own vocabulary
#     (UNKNOWN STATUS);
#   * a ticket's status disagrees with its row (STATUS);
#   * a ticket's evidence disagrees with its row (EVIDENCE);
#   * a ticket's blockers disagree with its row (BLOCKERS);
#   * an advanced status carries no completion date (NO DATE);
#   * an advanced status cites no evidence link (NO LINK);
#   * a blocked row names no blocker (NO BLOCKER).
#
# Every problem is reported; the script exits 1 if any were found, so a
# board that claims progress its tickets do not record can never be
# published.
#
# It is written in bash+awk on purpose: the authoritative GitForge CI
# image ships no python3, and a gate that only runs on a maintainer's
# laptop is not a gate.
#
# Usage: verify_release_ledger.sh [repo-root]
#
# The root defaults to the repository this script lives in. A root is
# accepted so the self-test can point it at a deliberately broken copy.

set -uo pipefail

root="${1:-}"
if [ -z "$root" ]; then
  root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fi

board="$root/docs/release/EXECUTION-BOARD.md"
tasks="$root/docs/tasks"

if [ ! -f "$board" ]; then
  echo "::error::the release execution board is missing: $board" >&2
  exit 2
fi
if [ ! -d "$tasks" ]; then
  echo "::error::the task directory is missing: $tasks" >&2
  exit 2
fi

shopt -s nullglob
tickets=("$tasks"/T27-*.md)
shopt -u nullglob

if [ "${#tickets[@]}" -eq 0 ]; then
  echo "::error::no T27 tickets under $tasks" >&2
  exit 2
fi

# The board's own vocabulary (§1). These are distinct states and are never
# collapsed into one another.
advanced="IMPLEMENTED|BUILD VERIFIED|INSTALLED|LAUNCHED|SMOKE TESTED|PUBLISHED|CERTIFIED|"
open="NOT STARTED|READY|BLOCKED|DEFERRED|REJECTED|IN PROGRESS|"

awk_program=$(cat <<'AWK'
function trim(s) {
  gsub(/^[ \t\r]+/, "", s)
  gsub(/[ \t\r]+$/, "", s)
  return s
}

function collapse(s) {
  gsub(/[ \t\r\n]+/, " ", s)
  return trim(s)
}

# The ledger's rows are markdown tables whose cells may carry an escaped
# pipe (`\|`) inside a code span. Split on the pipes that are not escaped.
function cells(line, arr,   tmp, n, i) {
  tmp = line
  sub(/^[ \t]+/, "", tmp)
  sub(/[ \t]+$/, "", tmp)
  while (substr(tmp, 1, 1) == "|") tmp = substr(tmp, 2)
  while (length(tmp) > 0 && substr(tmp, length(tmp), 1) == "|") tmp = substr(tmp, 1, length(tmp) - 1)
  gsub(/\\\|/, "\001", tmp)
  n = split(tmp, arr, "|")
  for (i = 1; i <= n; i++) {
    gsub(/\001/, "|", arr[i])
    arr[i] = trim(arr[i])
  }
  return n
}

function inlist(s, list) { return index(list, s "|") > 0 }

FILENAME == board {
  if (substr($0, 1, 6) != "| T27-") next
  n = cells($0, c)
  if (n != 10) {
    printf("MALFORMED ROW %s:%d: %d cells, expected 10\n", board, FNR, n) > "/dev/stderr"
    problems++
    next
  }
  id = c[1]
  if (id in r_status) {
    printf("DUPLICATE ROW %s:%d: %s\n", board, FNR, id) > "/dev/stderr"
    problems++
  }
  r_status[id] = c[4]
  r_evidence[id] = c[8]
  r_blockers[id] = c[9]
  r_done[id] = c[10]
  r_order[++rorder] = id
  next
}

{
  base = FILENAME
  sub(/.*\//, "", base)
  if (base !~ /^T27-[0-9]+/) next
  match(base, /^T27-[0-9]+/)
  id = substr(base, RSTART, RLENGTH)

  if (FNR == 1) {
    if (!(id in t_seen)) {
      t_seen[id] = 1
      t_order[++torder] = id
    }
    mode = ""
    buf = ""
    t_status[id] = ""
    t_evidence[id] = ""
    t_blockers[id] = ""
  }

  # A field runs from its label to the next blank line.
  if (mode != "") {
    if ($0 ~ /^[ \t]*$/) {
      if (mode == "ev") {
        t_evidence[id] = collapse(buf)
        have_evidence[id] = 1
      } else {
        t_blockers[id] = collapse(buf)
        have_blockers[id] = 1
      }
      mode = ""
    } else {
      buf = buf " " $0
    }
    next
  }

  if (!have_status[id] && index($0, "**Status:**") > 0) {
    rest = substr($0, index($0, "**Status:**") + length("**Status:**"))
    gsub(/^[ \t\r\n]+/, "", rest)
    if (match(rest, /^[A-Z][A-Z ]*/)) t_status[id] = trim(substr(rest, RSTART, RLENGTH))
    have_status[id] = 1
  }
  if (!have_evidence[id] && index($0, "**Evidence:**") > 0) {
    mode = "ev"
    buf = substr($0, index($0, "**Evidence:**") + length("**Evidence:**"))
    next
  }
  if (!have_blockers[id] && index($0, "**Blockers:**") > 0) {
    mode = "bl"
    buf = substr($0, index($0, "**Blockers:**") + length("**Blockers:**"))
    next
  }
}

END {
  for (i = 1; i <= rorder; i++) {
    id = r_order[i]
    if (!(id in t_seen)) {
      printf("ORPHAN ROW    %s: no ticket under docs/tasks/\n", id) > "/dev/stderr"
      problems++
      continue
    }
    st = r_status[id]
    if (!inlist(st, advanced) && !inlist(st, open)) {
      printf("UNKNOWN STATUS %s: '%s'\n", id, st) > "/dev/stderr"
      problems++
    }
    if (t_status[id] != st) {
      printf("STATUS %s: board='%s' ticket='%s'\n", id, st, t_status[id]) > "/dev/stderr"
      problems++
    }
    expected = r_evidence[id]
    if (r_done[id] != "" && r_done[id] != DASH) expected = expected " **Done:** " r_done[id] "."
    if (t_evidence[id] != trim(expected)) {
      printf("EVIDENCE %s: board='%s' ticket='%s'\n", id, substr(r_evidence[id], 1, 60), substr(t_evidence[id], 1, 60)) > "/dev/stderr"
      problems++
    }
    if (t_blockers[id] != r_blockers[id]) {
      printf("BLOCKERS %s: board='%s' ticket='%s'\n", id, r_blockers[id], t_blockers[id]) > "/dev/stderr"
      problems++
    }
    if (inlist(st, advanced) && r_done[id] == DASH) {
      printf("NO DATE %s: status=%s\n", id, st) > "/dev/stderr"
      problems++
    }
    # A closed row must cite its evidence as a link — a markdown link or
    # a URL — so a reader can follow the receipt instead of trusting a
    # prose path. The board's own acceptance criterion says "a cited
    # evidence link".
    if (inlist(st, advanced) && index(r_evidence[id], "](") == 0 && index(r_evidence[id], "https://") == 0) {
      printf("NO LINK %s: status=%s\n", id, st) > "/dev/stderr"
      problems++
    }
    if (st == "BLOCKED" && r_blockers[id] == DASH) {
      printf("NO BLOCKER %s: status=BLOCKED\n", id) > "/dev/stderr"
      problems++
    }
  }
  for (i = 1; i <= torder; i++) {
    id = t_order[i]
    if (!(id in r_status)) {
      printf("ORPHAN TICKET %s: docs/tasks/%s-*.md\n", id, id) > "/dev/stderr"
      problems++
    }
  }
  printf("rows=%d tickets=%d problems=%d\n", rorder, torder, problems) > "/dev/stderr"
  if (problems > 0) exit 1
}
AWK
)

awk -v board="$board" \
  -v DASH="—" \
  -v advanced="$advanced" \
  -v open="$open" \
  "$awk_program" "$board" "${tickets[@]}"
