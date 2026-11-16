#!/usr/bin/env python3
"""Sync the release execution board's rows into their tickets.

The board (`docs/release/EXECUTION-BOARD.md` §6) is the ledger.  Each
row is one logical change, and each row has exactly one ticket under
`docs/tasks/`.  This script writes the board's `Status`, `Evidence`,
`Blockers` and `Done` cells into that ticket, so the two files cannot
state different things.

The direction is one-way on purpose.  A ticket is never read as
authority: a second source of truth is how the two ledgers drifted
apart in the first place.  To advance a row, edit the board, then run
this script.

`scripts/verify_release_ledger.sh` checks the result, and CI runs it.

Usage:
    python3 -I scripts/reconcile_release_ledger.py <root> [--dry-run]

`<root>` is a directory holding `docs/release/EXECUTION-BOARD.md` and
`docs/tasks/`.

The writer is python because writing markdown with python's regex is
short and clear; the CHECKER is bash+awk (`scripts/verify_release_ledger.sh`)
because the authoritative GitForge CI image ships no python3, and a gate
that only runs on a maintainer's laptop is not a gate.
"""

import pathlib
import re
import sys

BOARD = "docs/release/EXECUTION-BOARD.md"
TASKS = "docs/tasks"

# The board's §6 row shape: | ID | Title | Pri | Status | Depends on |
# Acceptance criteria | Verification | Evidence | Blockers | Done |
COLUMNS = 10

ID, TITLE, PRI, STATUS, DEPENDS, CRITERIA, VERIFY, EVIDENCE, BLOCKERS, DONE = range(COLUMNS)


def split_markdown_row(line):
    """Split one table row into its cells, honouring `\\|` escapes."""
    body = line.strip().strip("|")
    cells = re.split(r"(?<!\\)\|", body)
    return [cell.replace("\\|", "|").strip() for cell in cells]


def repair_row(line):
    """Escape a bare `|` that sits inside a code span in a table cell."""
    out = []
    in_code = False
    index = 0
    while index < len(line):
        char = line[index]
        if char == "`":
            in_code = not in_code
            out.append(char)
            index += 1
            continue
        if char == "|" and in_code and not (index > 0 and line[index - 1] == "\\"):
            out.append("\\|")
            index += 1
            continue
        out.append(char)
        index += 1
    return "".join(out)


def load_board(root):
    """The board's §6 rows, keyed by task id."""
    rows = {}
    path = root / BOARD
    for line in path.read_text().splitlines():
        if not line.startswith("| T27-"):
            continue
        cells = split_markdown_row(repair_row(line))
        if len(cells) != COLUMNS:
            raise SystemExit(
                f"{BOARD}: row {cells[0]!r} has {len(cells)} cells, expected {COLUMNS}"
            )
        rows[cells[ID]] = {
            "title": cells[TITLE],
            "status": cells[STATUS],
            "evidence": cells[EVIDENCE],
            "blockers": cells[BLOCKERS],
            "done": cells[DONE],
        }
    return rows


def ticket_path(root, task_id):
    matches = sorted((root / TASKS).glob(f"{task_id}-*.md"))
    if not matches:
        return None
    return matches[0]


def set_field(text, label, value):
    """Replace a `**Label:** ...` paragraph, or insert it before `**Blockers:**`."""
    pattern = re.compile(
        rf"(\*\*{re.escape(label)}:\*\*\s*).+?(?=\n\n|\Z)", re.S
    )
    if pattern.search(text):
        return pattern.sub(lambda m: m.group(1) + value, text, count=1)
    anchor = re.search(r"\*\*Blockers:\*\*", text)
    if anchor:
        insert = f"**{label}:** {value}\n\n"
        return text[: anchor.start()] + insert + text[anchor.start() :]
    return text.rstrip() + f"\n\n**{label}:** {value}\n"


def sync_ticket(text, row):
    """Apply one board row to one ticket's text."""
    out = text
    # The status also appears in the ticket's header blockquote.
    out = re.sub(
        r"(\*\*Status:\*\*\s*)[A-Z][A-Z ]*",
        lambda m: m.group(1) + row["status"],
        out,
        count=1,
    )
    evidence = row["evidence"]
    if row["done"] and row["done"] != "—":
        evidence = f"{evidence} **Done:** {row['done']}."
    out = set_field(out, "Evidence", evidence)
    out = set_field(out, "Blockers", row["blockers"])
    return out


def main():
    args = [a for a in sys.argv[1:] if a != "--dry-run"]
    dry_run = "--dry-run" in sys.argv[1:]
    if len(args) != 1:
        raise SystemExit(__doc__)
    root = pathlib.Path(args[0])

    rows = load_board(root)
    changed = []
    for task_id in sorted(rows):
        row = rows[task_id]
        path = ticket_path(root, task_id)
        if path is None:
            print(f"ORPHAN ROW    {task_id}: no ticket under {TASKS}/")
            changed.append(task_id)
            continue
        original = path.read_text()
        updated = sync_ticket(original, row)
        if updated != original:
            changed.append(task_id)
            if not dry_run:
                path.write_text(updated)

    print(
        f"board rows: {len(rows)}, tickets synced: {len(changed)}"
        + (" (dry run — nothing written)" if dry_run else "")
    )
    return 1 if dry_run and changed else 0


if __name__ == "__main__":
    sys.exit(main())
