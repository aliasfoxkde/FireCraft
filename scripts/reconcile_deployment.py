#!/usr/bin/env python3
"""Compare a live web deployment against a release bundle, by digest.

The web release ships a bundle with a `MANIFEST.sha256` beside it:
every file of the bundle, with its sha256. A deployment publishes the
same bundle to a URL. This tool fetches the deployment's manifest and
every file it names, hashes the bytes it actually received, and
compares the two inventories file by file.

The output is a machine-readable report that says `match` or names
every difference: a file whose bytes differ, a file the deployment
does not serve, a file the deployment serves that the bundle does not
name. A deployment that corresponds to no release is not a
near-miss — it is a deployment nobody can trace to a commit, and the
report says so in the same words the ticket does.

The report is written even when the comparison fails: a failing
comparison is the useful output, not a reason to produce nothing.

Usage:
    python3 -I scripts/reconcile_deployment.py --bundle <dir> --url <base-url>
                                          [--out <report.json>] [--deployment-id <id>]
                                          [--require-match]

`--require-match` makes a difference a non-zero exit; without it the
tool exits 0 on a completed comparison whatever the result, because the
report — not the exit code — is the deliverable.
"""

import argparse
import datetime
import hashlib
import json
import pathlib
import sys
import urllib.error
import urllib.request

SCHEMA = "firecraft.deployment-reconciliation/1"
MANIFEST = "MANIFEST.sha256"
TIMEOUT = 60


def parse_manifest(text):
    """Read a `sha256sum`-shaped manifest into {name: digest}.

    A manifest line is a digest, two spaces, and the file name.
    Comment lines (a leading `#`) are ignored, as `sha256sum -c`
    ignores them.
    """
    entries = {}
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 1)
        if len(parts) != 2:
            continue
        digest, name = parts
        name = name.lstrip("*").strip()
        entries[name] = digest.strip()
    return entries


def fetch(url):
    """Fetch a URL and return its bytes."""
    request = urllib.request.Request(url, headers={"User-Agent": "firecraft-release-tools"})
    with urllib.request.urlopen(request, timeout=TIMEOUT) as response:
        return response.read()


def fetch_response(url):
    """Fetch a URL and return its bytes with the content type served."""
    request = urllib.request.Request(url, headers={"User-Agent": "firecraft-release-tools"})
    with urllib.request.urlopen(request, timeout=TIMEOUT) as response:
        return response.read(), response.headers.get("Content-Type", "")


def fetch_text(url):
    return fetch(url).decode("utf-8", errors="replace")


def digest_of(payload):
    return hashlib.sha256(payload).hexdigest()


def looks_like_html(path):
    """True when the bundle's own file for this name is an HTML page.

    A deployment that answers with a page instead of the file is a
    missing file, not a differing one — but only when the bundle's file
    was not itself a page.
    """
    if not path.is_file():
        return False
    head = path.read_bytes()[:64].lstrip().lower()
    return head.startswith(b"<!doctype html") or head.startswith(b"<html")


def compare(bundle_dir, base_url, deployment_id=None, bundle_label=None):
    """Compare the bundle at `bundle_dir` with the deployment at `base_url`."""
    bundle = pathlib.Path(bundle_dir)
    manifest_path = bundle / MANIFEST
    if not manifest_path.is_file():
        raise SystemExit(f"the bundle has no {MANIFEST}: {manifest_path}")
    bundle_manifest_text = manifest_path.read_text(encoding="utf-8", errors="replace")
    bundle_entries = parse_manifest(bundle_manifest_text)

    base = base_url.rstrip("/")
    deployed_manifest_text = fetch_text(f"{base}/{MANIFEST}")
    deployed_entries = parse_manifest(deployed_manifest_text)

    files = []
    for name in sorted(bundle_entries):
        expected = bundle_entries[name]
        local = bundle / name
        row = {
            "name": name,
            "expected_sha256": expected,
            "actual_sha256": None,
            "bytes": None,
            "status": "missing",
        }
        if local.is_file():
            payload = local.read_bytes()
            row["bytes"] = len(payload)
            row["actual_sha256"] = digest_of(payload)
            row["status"] = "match" if row["actual_sha256"] == expected else "differ"
            if row["status"] == "differ":
                # The bundle's own bytes disagree with its manifest: the
                # bundle is internally inconsistent, which is a finding
                # of its own.
                row["note"] = "the bundle's bytes disagree with its own manifest"
        try:
            served, content_type = fetch_response(f"{base}/{name}")
        except (urllib.error.URLError, OSError) as error:
            row["status"] = "not_served"
            row["error"] = str(error)
        else:
            row["served_sha256"] = digest_of(served)
            row["served_bytes"] = len(served)
            row["served_content_type"] = content_type
            if row["served_sha256"] != expected:
                row["status"] = "differ"
                if "html" in content_type.lower() and not looks_like_html(local):
                    # The deployment answered with a page instead of the
                    # file: the name is not deployed at all, and saying
                    # "differ" alone would hide that.
                    row["note"] = (
                        "the deployment served an HTML page for this name, "
                        "so the file is not deployed under it"
                    )
        files.append(row)

    only_in_bundle = sorted(set(bundle_entries) - set(deployed_entries))
    only_in_deployment = sorted(set(deployed_entries) - set(bundle_entries))
    differences = [row["name"] for row in files if row["status"] != "match"]
    differences.extend(only_in_bundle)
    differences.extend(only_in_deployment)

    report = {
        "schema": SCHEMA,
        "generated": datetime.datetime.now(datetime.timezone.utc).strftime(
            "%Y-%m-%dT%H:%M:%SZ"
        ),
        "bundle": {
            "label": bundle_label or str(bundle.resolve()),
            "manifest_sha256": digest_of(bundle_manifest_text.encode("utf-8")),
            "files": len(bundle_entries),
        },
        "deployment": {
            "url": base,
            "manifest_sha256": digest_of(deployed_manifest_text.encode("utf-8")),
            "files": len(deployed_entries),
            "deployment_id": deployment_id or "unknown",
            "deployment_id_reason": (
                "the deployment id comes from the Cloudflare Pages API, which needs a "
                "CLOUDFLARE_API_TOKEN; without it the report names the URL only"
                if not deployment_id
                else "supplied on the command line"
            ),
        },
        "inventory": {
            "only_in_bundle": only_in_bundle,
            "only_in_deployment": only_in_deployment,
        },
        "files": files,
        "result": "match" if not differences else "differences",
        "differences": sorted(set(differences)),
    }
    return report


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--bundle", required=True, help="the extracted release bundle")
    parser.add_argument("--bundle-label", help="how the bundle is named in the report")
    parser.add_argument("--url", required=True, help="the deployment's base URL")
    parser.add_argument("--out", help="where to write the JSON report")
    parser.add_argument("--deployment-id", help="the deployment id, when it is known")
    parser.add_argument(
        "--require-match",
        action="store_true",
        help="exit non-zero when the deployment does not match the bundle",
    )
    args = parser.parse_args(argv)

    report = compare(args.bundle, args.url, args.deployment_id, args.bundle_label)
    text = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.out:
        out = pathlib.Path(args.out)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)

    if args.require_match and report["result"] != "match":
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
