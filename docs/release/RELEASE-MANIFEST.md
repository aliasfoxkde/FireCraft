# The FireCraft release manifest

`release-manifest.json` is the machine-readable description of a FireCraft
release: one JSON document that says what the release contains, what every
file in it hashes to, and which build produced it.

- **Producer:** `cargo run -p xtask -- dist --manifest`
  (`xtask::dist::manifest`, in `xtask/src/dist.rs`)
- **Verifier:** `cargo run -p xtask -- dist --verify-manifest`
  (`xtask::dist::verify_manifest`)
- **Where it lands:** the release output directory, `dist/`, next to the
  artifacts it describes — `dist/release-manifest.json`
- **Published as:** a release asset, attached to the draft release beside
  the artifacts
- **Schema identifier:** `firecraft.release-manifest/1`

A consumer that does not recognise the schema string must refuse the
document rather than guess at its shape.

## Top level

| field | type | meaning |
|---|---|---|
| `schema` | string | `firecraft.release-manifest/1`. |
| `version` | string | the workspace version the artifacts carry (`0.2.5`). |
| `tag` | string or null | the release tag the build was cut from; `null` when the commit carries no tag. |
| `source_sha` | string | the git revision the artifacts were built from. |
| `run_url` | string or null | the CI run URL; `null` outside CI. |
| `run_id` | string or null | the CI run id; `null` outside CI. |
| `built_at` | string | the UTC build timestamp (`2026-11-10T12:34:56Z`). Build-time data. |
| `artifacts` | array | one entry per published asset, sorted by `name`. |

## Artifact entries

| field | type | meaning |
|---|---|---|
| `name` | string | the canonical published name — the asset's basename, which is how a release names it. |
| `path` | string | the path relative to the release output directory (`native/firecraft-0.2.5-linux-x86_64.tar.gz`). |
| `size` | number | the file's size in bytes. |
| `sha256` | string | the file's SHA-256 digest, lowercase hex. |
| `format` | string | the package format: `tar.gz`, `deb`, `zip`, `json`, `sha256`. |
| `target` | string | the target triple the artifact was built for; `any` for the artifacts that are not platform-specific (the source archive, the SBOM, the checksum lists). |
| `source_sha` | string | the revision this artifact was built from. |
| `verification` | string | `verified` when a recorded checksum list corroborates the digest, `computed` when the generator computed it and no recorded list covers it. |

## The asset set

The manifest describes the published release asset set — the eight files the
release attaches beside the manifest itself. The set is a fixed table in the
generator, not a scan of the
output directory: the release names every asset explicitly (a glob such as
`firecraft-web-*.zip` picks up older versions left in `dist/`), and a scan
would let a stale file into the manifest.

| `name` | `path` | `format` | `target` |
|---|---|---|---|
| `firecraft-<v>-linux-x86_64.tar.gz` | `native/firecraft-<v>-linux-x86_64.tar.gz` | `tar.gz` | `x86_64-unknown-linux-gnu` |
| `firecraft-cli-<v>-linux-x86_64.tar.gz` | `native/firecraft-cli-<v>-linux-x86_64.tar.gz` | `tar.gz` | `x86_64-unknown-linux-gnu` |
| `firecraft_<v>_amd64.deb` | `native/firecraft_<v>_amd64.deb` | `deb` | `x86_64-unknown-linux-gnu` |
| `MANIFEST.sha256` | `native/MANIFEST.sha256` | `sha256` | `x86_64-unknown-linux-gnu` |
| `firecraft-web-<v>.zip` | `firecraft-web-<v>.zip` | `zip` | `wasm32-unknown-unknown` |
| `firecraft-<v>-source.tar.gz` | `source/firecraft-<v>-source.tar.gz` | `tar.gz` | `any` |
| `firecraft-<v>-sbom.cdx.json` | `sbom/firecraft-<v>-sbom.cdx.json` | `json` | `any` |
| `RELEASE_SHA256.txt` | `RELEASE_SHA256.txt` | `sha256` | `any` |

`<v>` is the workspace version.

The manifest is written **after** every asset exists — including the web zip
and `RELEASE_SHA256.txt`, which the release workflow produces — so it
describes the whole release and not a part of it.

## Determinism

The manifest bytes are a pure function of the artifact set and the run facts:

- object keys are sorted (`serde_json`'s map is a `BTreeMap` here), and the
  artifact array is sorted by `name`;
- there is no timestamp other than `built_at`, which is documented above as
  build-time data;
- the environment, the git checkout and the clock are read once, at the edge
  (`manifest_context`), and handed to the generator as data — the generator
  itself never touches process-global state.

The same artifact set and the same run facts therefore produce the same
bytes. `built_at`, `run_url` and `run_id` are the fields that differ between
two runs of the same tree; everything else is stable.

## Fail-closed generation

The generator refuses to describe a release it cannot prove. It fails when

- an artifact is **missing** from the output directory;
- an artifact is **empty**;
- a recorded checksum list does not parse;
- a file's digest **contradicts** a recorded list (`RELEASE_SHA256.txt`,
  `native/MANIFEST.sha256`, `web/MANIFEST.sha256`) — that is a corrupted or
  mislabeled artifact;
- the release tag and the workspace version disagree.

The verifier (`dist --verify-manifest`) re-reads the manifest against the
files on disk: every listed artifact must exist with the recorded size and
digest, every published artifact must be listed, and nothing else may be
listed.

## Verifying a published release

The release workflow's aggregate validation does the same check against the
downloaded assets before the draft release is created. To check a manifest by
hand:

    jq -r '.version,.artifacts[0].sha256' dist/release-manifest.json
    jq -r '.artifacts[] | "\(.sha256)  \(.name)"' dist/release-manifest.json | sha256sum -c -

## Known gap

The manifest's own digest is not covered by `RELEASE_SHA256.txt`: the
checksum list is written before the manifest exists, and a list that covered
the manifest could not be covered by the manifest in turn. Signing
(board T27-56 and T27-57) is what closes that gap.
