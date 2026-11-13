#!/usr/bin/env python3
"""Fail-closed check that every GitHub Action is pinned to a commit SHA.

A workflow that names an action by tag or branch (`actions/checkout@v4`,
`dtolnay/rust-toolchain@stable`) resolves to whatever the upstream tag
points at on the day the job runs. That is a mutable reference: the
release workflow — the one that builds and uploads the published
artifacts — would execute code that can change without a review of this
repository, and the change lands with no diff to review.

The rule this script enforces:

  * every `uses:` that names an action is `owner/repo@<40-hex-sha>`
  * a local action (`./path`) is allowed — it is this repository's code
  * anything else (a tag, a branch, a `docker://` image) is rejected

Usage: python3 scripts/validate_workflow_pins.py .github/workflows
Exit 0 prints a summary; exit 1 lists the violations. stdlib-only.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

USES_RE = re.compile(r"^\s*(?:-\s+)?uses:\s*(\S+)\s*(?:#.*)?$")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
ACTION_RE = re.compile(r"^(?P<repo>[\w.\-]+/[\w.\-]+)@(?P<ref>.+)$")


def action_ref(line: str) -> str | None:
    """Return the `owner/repo@ref` value on a `uses:` line, or None."""
    match = USES_RE.match(line)
    if not match:
        return None
    value = match.group(1)
    action = ACTION_RE.match(value)
    return action.group("ref") if action else None


def check_file(path: Path) -> list[str]:
    """Return the violations in one workflow file."""
    violations: list[str] = []
    for number, line in enumerate(path.read_text(encoding="utf-8").split("\n"), start=1):
        match = USES_RE.match(line)
        if not match:
            continue
        value = match.group(1)
        if value.startswith("./"):
            # A local action: this repository's own code, reviewed in the diff.
            continue
        action = ACTION_RE.match(value)
        if not action:
            violations.append(
                f"{path}:{number}: uses `{value}` — a local action must start with `./`, "
                "and every other action must be pinned to a commit SHA"
            )
            continue
        ref = action.group("ref")
        if not SHA_RE.match(ref):
            violations.append(
                f"{path}:{number}: uses `{value}` — `{ref}` is a mutable reference; "
                "pin the action to its 40-character commit SHA"
            )
    return violations


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else ".github/workflows")
    if not root.is_dir():
        print(f"workflow directory not found: {root}", file=sys.stderr)
        return 1
    files = sorted(list(root.glob("*.yml")) + list(root.glob("*.yaml")))
    if not files:
        print(f"no workflow files under {root}", file=sys.stderr)
        return 1
    violations: list[str] = []
    pinned = 0
    for path in files:
        violations.extend(check_file(path))
        pinned += sum(
            1
            for line in path.read_text(encoding="utf-8").split("\n")
            if (ref := action_ref(line)) is not None and SHA_RE.match(ref)
        )
    if violations:
        print(f"✗ {len(violations)} unpinned action reference(s):", file=sys.stderr)
        for violation in violations:
            print(f"  - {violation}", file=sys.stderr)
        return 1
    print(f"✓ {pinned} action reference(s) across {len(files)} workflow file(s) are pinned to commit SHAs")
    return 0


if __name__ == "__main__":
    sys.exit(main())
