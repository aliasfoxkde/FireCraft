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
- the graph must have exactly one root and every job must hang off it
- a job must not JOIN two parents: all jobs of one run share ONE
  per-run workspace, so a join means two writers in one tree (the dsc
  run 907dc1bf class of race). A FORK is allowed — the pipeline fans
  perf-budgets and fuzz out of corpus-golden, and each of those jobs
  owns a separate cache directory, so they never share a target tree

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
    cache_sources: set[str] = set()
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

        cache = job.get("cache")
        if cache is not None and not isinstance(cache, dict):
            errors.append(f"job {name}: cache must be a mapping")
        elif isinstance(cache, dict):
            source = cache.get("source")
            if source is None:
                errors.append(
                    f"job {name}: cache must name its own directory (source: "
                    "\"{cache_root}/cargo/{image}-<job>\"); the bare default is "
                    "shared by every job and serializes the fleet"
                )
            elif not isinstance(source, str):
                errors.append(f"job {name}: cache source must be a string")
            else:
                # The runner takes an EXCLUSIVE lock on every writable host
                # directory a job mounts (GitForge GF-11), so two jobs on one
                # cache directory cannot run at the same time. A source shared
                # by every job would serialize the whole fleet into one lane.
                if not source.endswith(name):
                    errors.append(
                        f"job {name}: cache source must end with the job name so each job "
                        f"owns its own cache directory (got {source!r})"
                    )
                if source in cache_sources:
                    errors.append(
                        f"job {name}: cache source {source!r} is already used by another job; "
                        "two jobs on one cache directory serialize each other"
                    )
                cache_sources.add(source)

        steps = job.get("steps") or []
        if isinstance(steps, list):
            runs_cargo = False
            for step in steps:
                if not isinstance(step, dict) or not step.get("name") or not step.get("run"):
                    errors.append(f"job {name}: every step needs name and run")
                    continue
                run = str(step["run"])
                runs_cargo |= re.search(r"\bcargo(?:\s|$)", run) is not None
                if re.search(r"CARGO_TARGET_DIR=/tmp/", run):
                    errors.append(
                        f"job {name}: Cargo target output must use the declared host cache, "
                        "not a disposable container /tmp directory"
                    )
            if runs_cargo and not isinstance(cache, dict):
                errors.append(f"job {name}: Cargo work must declare a cache: block")

    for ref in (n for job in jobs for n in job.get("needs", []) or []):
        if ref not in names:
            errors.append(f"needs reference to unknown job: {ref}")

    roots = [n for n in names if indegree.get(n, 0) == 0]
    if len(roots) != 1:
        errors.append(
            "pipeline must have exactly one root job, "
            f"found {len(roots)} ({', '.join(roots) or 'none'})"
        )
    for job in jobs:
        if len(job.get("needs", []) or []) > 1:
            errors.append(
                f"job {job['name']} joins {len(job['needs'])} parents; "
                "a join is forbidden (the run's single shared workspace "
                "would be written by two jobs at once)"
            )

    # Reachability over the DAG. A fork (two jobs needing the same
    # parent) is allowed; a job that no path from the root reaches is
    # not — such a job never runs.
    nxt = {n: [] for n in names}
    for job in jobs:
        for ref in job.get("needs", []) or []:
            if ref in nxt:
                nxt[ref].append(job["name"])
    seen = set()
    stack = list(roots)
    while stack:
        node = stack.pop()
        if node in seen:
            continue
        seen.add(node)
        stack.extend(nxt.get(node, []))
    if roots and len(seen) != len(names):
        unreachable = sorted(set(names) - seen)
        errors.append(
            f"pipeline is not fully connected: {len(seen)}/{len(names)} jobs reachable "
            f"from the root (unreachable: {', '.join(unreachable)})"
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
        "root": next((j["name"] for j in jobs if not j.get("needs")), None),
    }
    print(f"✓ {path} is structurally valid")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
