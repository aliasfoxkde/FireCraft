#!/usr/bin/env python3
"""Offline structural validation of .gitforge.yml against the GitForge parser.

The GitForge API has no dry-run/validate endpoint, and creating a pipeline
requires authentication — so this script enforces the same contract locally
that `PipelineDefinition::parse` (crates/gitforge-ci/src/pipeline.rs) will
enforce server-side, plus the graph invariants the runner expects:

- required pipeline fields: name, version, trigger_on, environment, jobs
  (environment is REQUIRED — no serde default; a missing key fails every
  run with "missing field environment")
- required job fields: name, image, steps; optional: needs, env, timeout,
  retry
- required step fields: name, run; optional: env, working_directory,
  condition (working_directory is parse-only: the runner executes steps
  verbatim via `sh -c` and never applies it — prefer cd inside run)
- timeout strings parse as <int><s|m|h>
- `needs` references must name jobs in this pipeline
- the graph must stay a single chain (all jobs of one run share ONE
  per-run workspace; the runner has capacity 2 — fan-out re-introduces the
  dsc run 907dc1bf class of race where sibling jobs mutate the same tree)

Exit 0 prints a summary; exit 1 lists violations. stdlib-only.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

try:
    import yaml  # type: ignore[import-untyped]
except ModuleNotFoundError:  # pragma: no cover - exercised only without PyYAML
    print("PyYAML is required: python3 -m pip install pyyaml", file=sys.stderr)
    sys.exit(1)

TIMEOUT_RE = re.compile(r"^\d+(s|m|h)$")


def validate(path: Path) -> list[str]:
    errors: list[str] = []
    doc = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(doc, dict):
        return [f"{path}: top level must be a mapping"]

    for field in ("name", "version", "trigger_on", "environment", "jobs"):
        if field not in doc:
            errors.append(f"pipeline missing required field: {field}")
    if errors:
        return errors

    jobs = doc["jobs"]
    if not isinstance(jobs, list) or not jobs:
        return ["pipeline must declare at least one job"]

    names: list[str] = []
    indegree: dict[str, int] = {}
    for job in jobs:
        name = job.get("name")
        if not name:
            errors.append("job without a name")
            continue
        if name in names:
            errors.append(f"duplicate job name: {name}")
        names.append(name)

        for field in ("image", "steps"):
            if not job.get(field):
                errors.append(f"job {name}: missing required field {field}")
        needs = job.get("needs", [])
        if not isinstance(needs, list):
            errors.append(f"job {name}: needs must be a list")
        else:
            indegree[name] = len(needs)
        timeout = job.get("timeout")
        if timeout is not None and not TIMEOUT_RE.match(str(timeout)):
            errors.append(f"job {name}: timeout {timeout!r} must match <int><s|m|h>")

        steps = job.get("steps") or []
        if isinstance(steps, list):
            for step in steps:
                if not isinstance(step, dict) or not step.get("name") or not step.get("run"):
                    errors.append(f"job {name}: every step needs name and run")

    for ref in (n for job in jobs for n in job.get("needs", []) or []):
        if ref not in names:
            errors.append(f"needs reference to unknown job: {ref}")

    roots = [n for n in names if indegree.get(n, 0) == 0]
    if len(roots) != 1:
        errors.append(
            "pipeline must be a single chain: expected exactly one root job, "
            f"found {len(roots)} ({', '.join(roots) or 'none'})"
        )
    for job in jobs:
        if len(job.get("needs", []) or []) > 1:
            errors.append(
                f"job {job['name']} joins {len(job['needs'])} parents; "
                "single-chain policy forbids joins (shared workspace race)"
            )

    reachable = 0
    if roots:
        nxt = {n: [] for n in names}
        for job in jobs:
            for ref in job.get("needs", []) or []:
                nxt[ref].append(job["name"])
        cursor = roots[0]
        while cursor is not None:
            reachable += 1
            children = nxt[cursor]
            cursor = children[0] if len(children) == 1 else None
        if reachable != len(names):
            errors.append(
                f"pipeline is not fully connected: {reachable}/{len(names)} jobs reachable"
            )

    return errors


def main() -> int:
    path = Path(sys.argv[1] if len(sys.argv) > 1 else ".gitforge.yml")
    if not path.is_file():
        print(f"pipeline definition not found: {path}", file=sys.stderr)
        return 1
    errors = validate(path)
    if errors:
        print(f"✗ {path} failed validation:")
        for error in errors:
            print(f"  - {error}")
        return 1
    doc = yaml.safe_load(path.read_text(encoding="utf-8"))
    jobs = doc["jobs"]
    summary = {
        "name": doc["name"],
        "version": doc["version"],
        "trigger_on": doc["trigger_on"],
        "jobs": [j["name"] for j in jobs],
        "chain": [j["name"] for j in jobs] == [j["name"] for j in sorted(jobs, key=lambda x: 0)]
        or None,
    }
    print(f"✓ {path} is structurally valid")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
