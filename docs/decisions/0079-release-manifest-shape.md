# 0079 — The release manifest is a fixed asset table, generated last

Date: 2026-11-10 · Ticket: [T27](../tasks/T27-cross-platform-release.md) · Status: accepted

## Context

The campaign requires one machine-readable description of a release
(`docs/RELEASE-ENGINEERING-PLAN.md` §4, board row T27-54). The plan
lists what it must carry: version, tag, commit SHA, build timestamp,
artifact filename, target triple, package format, byte size, SHA-256,
the workflow run URL/ID, and a verification status.

Three things about the current release path made the shape of that
document a decision rather than a transcription.

**The canonical naming is not settled.** T27-53 owns the canonical asset
names and the release-wide checksum list, and it is still `NOT
STARTED`. The manifest cannot adopt a convention that does not exist
yet, so it adopts the names the release already enforces: the
`expected=(...)` array in `.github/workflows/release.yml` and the rows
of `docs/release/ARTIFACT-MATRIX.md`.

**The output directory cannot be scanned.** The release workflow names
each digested file explicitly and says why: `firecraft-web-*.zip` picks
up older versions left in `dist/`, and `firecraft-*.tar.gz` also
matches `firecraft-cli-*.tar.gz`. A manifest built by scanning `dist/`
would inherit exactly that defect — a stale file from an older build
would appear in the document that claims to describe this release.

**The document cannot be byte-identical across runs and carry a run
URL at the same time.** A CI run URL is a different string on every
run. "Deterministic" and "records the run" pull in opposite
directions unless the build-time facts are kept separate from the
artifact description.

There is also a structural circularity: the release-wide checksum list
and the manifest each describe the other's file set. Whichever is
written first cannot be covered by the other.

## Decision

`cargo run -p xtask -- dist --manifest` writes
`dist/release-manifest.json`, into the release output directory next to
the artifacts it describes, and the file is published with the release
as a ninth asset.

- **The asset set is a fixed table**, `release_specs()` in
  `xtask/src/dist.rs`, not a scan. It is the eight published assets;
  the table changes when the published set changes.
- **The generator is fail-closed.** A missing artifact, an empty
  artifact, a checksum list that does not parse, a digest that
  contradicts a recorded list, or a tag that disagrees with the
  workspace version all fail the run. A manifest that claims a release
  the build cannot prove is worse than no manifest.
- **The build-time facts are data.** `manifest_context()` reads the
  git revision, the tag, the run URL, the run id and the clock once,
  at the edge; the generator takes them as a value and never touches
  process-global state. The artifact description is therefore a pure
  function of the artifact set and those facts, and the bytes are
  sorted — keys sorted, artifact array sorted by name — so the same
  inputs produce the same document. `built_at`, `run_url` and `run_id`
  are documented as build-time data; nothing else varies between two
  runs of one tree.
- **The manifest is generated last**, after every asset exists —
  including the web zip and `dist/RELEASE_SHA256.txt`, which the
  workflow produces, not `xtask dist`.
- **The workflow validates the manifest against the assets.** The
  aggregate step checks that it carries the tag's version, lists every
  published asset, and describes each one with the digest and the size
  the asset actually has. The manifest's file name is exempt from the
  "the name carries the tag version" rule because its version is
  checked inside the document instead — a content check, not a hole.

## Consequences

- A consumer can check a release with one command:
  `jq -r '.version,.artifacts[0].sha256' dist/release-manifest.json`,
  and `sha256sum -c` over the manifest's digest lines.
- The published asset set is now stated in two places: the generator's
  table and the workflow's `expected=(...)` array. The aggregate
  validation compares them — it requires every expected asset to be
  listed in the manifest — so a drift between the two fails the
  release instead of shipping.
- `dist --manifest` requires the complete release set, so it cannot
  run on a partial tree. The GitForge release job builds only the web
  and native sets, and therefore produces no manifest; the GitHub
  release workflow is the path that publishes one.
- The manifest's own digest is **not** in `RELEASE_SHA256.txt`: the
  list is written before the manifest exists, and a list that covered
  the manifest could not be covered by the manifest in turn. That gap
  is signing's to close (board T27-56 and T27-57), and it is recorded
  in `docs/release/RELEASE-MANIFEST.md` rather than left implicit.
- T27-53 is still open. When it settles the canonical names, the
  generator's table moves with them — the manifest inherits the
  convention instead of inventing one.
