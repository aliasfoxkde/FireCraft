# Receipt — the `v0.2.5-dryrun.3` disposable release rehearsal

> The first end-to-end execution of the tag-driven release workflow
> this repository has ever run. It is a **rehearsal**: a disposable
> tag, a **draft** release, no publication. The production release
> `v0.2.5` is untouched and remains the latest published release.
>
> Every fact below was read from the authoritative store at the time of
> writing — the GitHub API for the tag, the workflow run, the draft
> release and its assets — not from a watcher or from a remembered
> log.

## What was run

| Fact | Value | Where it was read |
|---|---|---|
| Tag | `v0.2.5-dryrun.3` | `GET /git/ref/tags/v0.2.5-dryrun.3` |
| Annotated tag object | `9009d404bba6288192be00afcdf5dee6cc8acf14` | same |
| Tag message | `Disposable rehearsal tag (not a release)` | `GET /git/tags/9009d404…` |
| Tagged commit | `384687fcc956ad19a204323a351fd975ca86124d` | `GET /git/tags/9009d404…` → `.object.sha` |
| Tagger | Michael Kinney, 2026-11-11T01:15:03Z | same |
| Release workflow run | **42937560976** — `push` on `v0.2.5-dryrun.3`, `completed`/`success`, 2026-11-11T01:15:06Z | `GET /actions/runs?head_sha=384687f…` |
| `build artifacts` job | success, 2026-11-11T01:15:09Z → 01:37:28Z | `GET /actions/runs/42937560976/jobs` |
| `create draft release` job | success, 2026-11-11T01:37:30Z → 01:37:42Z | same |
| CI (backup) on the same SHA | run 42937560759, `completed`/`success` | `GET /actions/runs?head_sha=384687f…` |
| Draft release | id **441136236**, `draft: true`, `published_at: null`, created 2026-11-11T01:15:03Z | `GET /releases/441136236` |

The draft was **not** published. Publishing a production release is a
maintainer-only act (campaign §20); a disposable rehearsal tag and a
draft are what this authorization covers.

## The eight assets, as published on the draft

Every asset is `state: uploaded`; the digest is the API's own
`sha256:` field for the asset bytes.

| Asset | Bytes | sha256 |
|---|---|---|
| `firecraft-0.2.5-dryrun.3-linux-x86_64.tar.gz` | 22,577,452 | `a97d9bd3e05b1721316f5e9859968b316662046faf05357f9a9d4b0488b7a69f` |
| `firecraft-cli-0.2.5-dryrun.3-linux-x86_64.tar.gz` | 8,109,224 | `f646feeac13369dbf204b998f733ac0a1bac806db6d2e78cebf583af854ab26e` |
| `firecraft_0.2.5-dryrun.3_amd64.deb` | 15,093,628 | `8f131947a400661382a3bd103274b3a8b9d0670d96cc28fc90910cc67b74ae01` |
| `firecraft-web-0.2.5-dryrun.3.zip` | 7,394,859 | `57054d0613678c8fe3885fb98f3de19668d69086de39cab0ecb4894e613f15ab` |
| `FireCraft-0.2.5-dryrun.3-source.tar.gz` | 2,262,800 | `1caf6e335ad2da526d584647d23ce95d88a54217aac0e6af237515cd6e211f5c` |
| `firecraft-0.2.5-dryrun.3-sbom.cdx.json` | 256,066 | `c4aa8cee0ebb4152b1f01d03a720690c1dacf8c6c1230c7ef70e8678713d163c` |
| `MANIFEST.sha256` | 425 | `e71ce4859fc811b5d3c4289ed3df57f63738add59e7711b3101e99d56219dbba` |
| `RELEASE_SHA256.txt` | 718 | `6ae766e787eaa1f0580c34560f4e73f993a4571cb96529c21a0e658f61190f80` |

`RELEASE_SHA256.txt` was downloaded from the draft and compared with
the API's digests above: **7 of 7 entries match** — the desktop
tarball, the CLI tarball, the DEB, the web ZIP, the source archive,
the SBOM and `MANIFEST.sha256`. The list does not cover itself, which
is why it holds seven entries for eight assets.

## What the rehearsal proved

The tag-driven workflow works end to end for the first time:

- `build artifacts` produced the native set, the web bundle, the
  source archive and the SBOM, and wrote the release-wide checksum
  list — all inside one `sh -c` step whose failures are load-bearing
  (decision 0086).
- `create draft release` ran the fail-closed aggregate validation
  (every required artifact present and non-empty, `sha256sum -c`
  passing, the release manifest agreeing with the assets it
  describes, every asset name carrying the tag version) and then
  created the draft with the assets passed **positionally** — the
  T27-17 defect (`-a` is not a flag of `gh release create`) is fixed
  and exercised.
- The tag is disposable and says so in its own message, and the
  release it produced is a draft. Nothing about it can be mistaken
  for the production `v0.2.5` release.

## What the rehearsal found

**The published source archive cannot be built with `--locked`.**
Reproduced from the asset bytes themselves (asset id `733682287`),
not from the build that produced them:

```
$ cargo metadata --locked --offline --format-version 1
error: cannot update the lock file …/Cargo.lock because --locked was passed to prevent this
exit 101
```

Re-resolving the extracted tree without `--locked` shows exactly what
cargo wanted to change — three lines, all of them the lock's
*dependency references*:

```
94c94
<  "color 0.2.5",
---
>  "color 0.2.5-dryrun.3",
```

The manifests in the archive **are** stamped (`0.2.5-dryrun.3`
throughout). Two places were not:

1. **`Cargo.lock`'s dependency references.** The lock holds two
   versions of `color` — the local crate and a registry one — so
   every reference to the local one carries its version
   (`"color 0.2.5"`). The release workflow stamped the lock's
   `[[package]] version` entries but not the references, so the lock
   stopped describing itself.
2. **The path dependency's version requirement.** `xtask/Cargo.toml`
   pins `compose = { version = "0.2.5", path = "../crates/compose" }`.
   A caret requirement on `0.2.5` does not match the pre-release
   `0.2.5-dryrun.3`, so cargo had to re-resolve — and `--locked`
   refuses to let it.

The stamp was a two-file manual edit (`Cargo.toml` and
`packaging/deb/control`); the lock and the requirements were never in
it. The fix is a tool, not a procedure: `xtask stamp <version>` moves
all six stamps together, and `xtask stamps` fails the release lane
when any of them drifts — including a lock whose references name a
package the lock no longer holds. See
[`xtask/src/stamp.rs`](../xtask/src/stamp.rs) and
[`docs/RELEASING.md`](../RELEASING.md).

## Two naming defects the rehearsal exposed

- The source archive is published as `FireCraft-<version>-source.tar.gz`
  while every other asset is lowercase. The artifact naming is
  canonicalized to `firecraft-<version>-source.tar.gz` (the same
  change that drops the `FireCraft` designation from the release
  name).
- The CLI archive carries no version stamp of its own: `firecraft-cli`
  cannot report which build it is, so the release receipt has nothing
  to attest to. `firecraft-cli --version` now prints the stamped
  version.

## What this receipt does not claim

- **Nothing is `CERTIFIED`.** No artifact is signed, no provenance or
  attestation exists (B-14), so the ladder's ceiling for this
  repository is rung 8 for everything.
- **The draft was never published**, and this receipt is not
  authorization to publish it.
- **The production `v0.2.5` release is unchanged** and remains the
  latest published release. Its artifacts were published out of band
  (B-21) and a rebuild of the tree does not reproduce those bytes
  (B-18).
- **A rehearsal on Linux x86_64 is not the platform ladder.** The
  Windows, macOS, AppImage and Flatpak recipes remain unexecuted
  (B-8, B-10, B-11, B-12).
