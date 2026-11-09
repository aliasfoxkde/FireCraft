#!/usr/bin/env python3
"""Re-pin the Aegis baseline against a written adjudication table.

The baseline keys a finding on `pattern:file:line:content-digest`, so any
edit to a file that holds findings invalidates its entries. That is a
problem for the release-engineering tickets: the adjudication table in
docs/tasks/T27-12 names the rule identifiers and the phrases that the
rules match, so the table itself is a source of findings, and every row
added to the table moves the lines of the rows already in it.

This script therefore does not key adjudications by line. It reads the
adjudication table out of the ticket and matches a finding to a row by
rule name and by the file the row is about. A finding that no row
covers aborts the run, so an unreviewed entry can never enter the
baseline, and a row that moves does not need a new adjudication.

Matching is by file name, not by path, because the table cites a file
with a path prefix that varies between rows.

Usage:
    python3 scripts/aegis_repin.py [--table PATH] [--dry-run]

Exit status is 0 when the baseline is clean and 1 when it is not.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile


TABLE = "docs/tasks/T27-12-adjudicate-aegis-findings.md"
BASELINE = ".aegis-baseline.json"
SCAN_BINARIES = ("aegis-0.6.3", "aegis")


def emit(message):
    sys.stdout.write(message + "\n")


def rule_rows(table_path):
    """Return [(rule_name, row_text)] for every data row of the table.

    A row is a pipe-delimited markdown line whose first cell names a
    rule. The header and the separator carry no rule name and are
    skipped.
    """
    rows = []
    with open(table_path, encoding="utf-8") as handle:
        for line in handle:
            stripped = line.strip()
            if not stripped.startswith("|"):
                continue
            cells = [cell.strip() for cell in stripped.strip("|").split("|")]
            if len(cells) < 2:
                continue
            name = cells[0].strip().strip("`").strip()
            if not name or set(name) <= set("-: "):
                continue
            if name.lower() == "rule":
                continue
            rows.append((name, stripped))
    return rows


def covering_row(rows, rule, file_name):
    """Return the first row that adjudicates `rule` in `file_name`.

    A row covers a finding when its first cell is the rule name and its
    text names the file the finding is in. Both conditions are needed:
    the rule name alone would let a row about one file silently excuse
    the same rule elsewhere.
    """
    for name, text in rows:
        if name != rule:
            continue
        if file_name in text:
            return text
    return None


def scan_binary():
    for candidate in SCAN_BINARIES:
        found = shutil.which(candidate)
        if found:
            return found
    raise SystemExit("no aegis binary on PATH")


def run_scan(binary, output_path, baseline=None):
    command = [
        binary,
        "--format",
        "json",
        "-c",
        "production",
        "scan",
        ".",
        "--output-file",
        output_path,
    ]
    if baseline:
        command.extend(["--baseline", baseline])
    command.append("-q")
    subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--table", default=TABLE)
    parser.add_argument("--baseline", default=BASELINE)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    rows = rule_rows(args.table)
    if not rows:
        emit("REFUSING: the adjudication table has no rule rows.")
        return 1

    with tempfile.TemporaryDirectory(prefix="aegis-repin-") as work:
        fresh_path = os.path.join(work, "fresh.json")
        run_scan(scan_binary(), fresh_path)
        if not os.path.exists(fresh_path):
            emit("REFUSING: the scan produced no output file.")
            return 1
        with open(fresh_path, encoding="utf-8") as handle:
            fresh = json.load(handle)

        with open(args.baseline, encoding="utf-8") as handle:
            base = json.load(handle)

        fresh_by_fingerprint = {f["fingerprint"]: f for f in fresh["findings"]}
        added = [f for f in fresh["findings"]
                 if f["fingerprint"] not in {b["fingerprint"] for b in base["findings"]}]
        removed = [b for b in base["findings"]
                   if b["fingerprint"] not in fresh_by_fingerprint]

        uncovered = []
        for finding in added:
            rule = finding["pattern"]
            file_name = os.path.basename(finding["location"]["file"])
            if covering_row(rows, rule, file_name) is None:
                uncovered.append(finding)

        if uncovered:
            emit("REFUSING: findings with no row in the adjudication table:")
            for finding in uncovered:
                location = finding["location"]
                emit("  %-20s %s:%s" % (finding["pattern"],
                                        location["file"],
                                        location["line"]))
            return 1

        emit("Adjudicated by table (%d new):" % len(added))
        for finding in added:
            location = finding["location"]
            emit("  %-20s %s:%s" % (finding["pattern"],
                                    location["file"],
                                    location["line"]))
        emit("")
        emit("Dropped (moved or gone) (%d):" % len(removed))
        for finding in removed:
            location = finding["location"]
            emit("  %-20s %s:%s" % (finding["pattern"],
                                    location["file"],
                                    location["line"]))

        # Keep the committed entries in place, in the committed order,
        # with their committed identifiers. The identifier field is a
        # per-scan nonce; the stable identifier is the durable key, so
        # replacing every entry would churn the whole file for the sake
        # of the few real changes.
        retained = [f for f in base["findings"] if f["fingerprint"] in fresh_by_fingerprint]
        kept = {f["fingerprint"] for f in retained}
        retained.extend(f for f in fresh["findings"] if f["fingerprint"] not in kept)
        base["findings"] = retained

        stats = base["stats"]
        stats["finding_count"] = len(base["findings"])
        severity = {}
        category = {}
        for finding in base["findings"]:
            severity[finding["severity"]] = severity.get(finding["severity"], 0) + 1
            category[finding["category"]] = category.get(finding["category"], 0) + 1
        stats["findings_by_severity"] = severity
        stats["findings_by_category"] = category

        if args.dry_run:
            emit("")
            emit("Dry run: the baseline was not written.")
            return 0

        with open(args.baseline, "w", encoding="utf-8") as handle:
            json.dump(base, handle, indent=2)
            handle.write("\n")

        ledger_units = len(stats.get("inspection_ledger", {}).get("units", []))
        emit("")
        emit("Baseline holds %d findings; the inspection ledger keeps %d units."
             % (len(base["findings"]), ledger_units))

    # Verify against the baseline that was just written: the gate must
    # see no finding at all.
    verify_path = tempfile.mkstemp(prefix="aegis-verify-", suffix=".json")[1]
    try:
        run_scan(scan_binary(), verify_path, baseline=args.baseline)
        with open(verify_path, encoding="utf-8") as handle:
            verify = json.load(handle)
        remaining = verify["findings"]
        if remaining:
            emit("")
            emit("FAIL: the scan still reports %d findings." % len(remaining))
            for finding in remaining[:20]:
                location = finding["location"]
                emit("  %-20s %s:%s" % (finding["pattern"],
                                        location["file"],
                                        location["line"]))
            return 1
        emit("Verification: the scan reports no findings.")
    finally:
        if os.path.exists(verify_path):
            os.unlink(verify_path)

    return 0


if __name__ == "__main__":
    sys.exit(main())
