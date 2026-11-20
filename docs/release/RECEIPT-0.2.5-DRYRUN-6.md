# Receipt — the `v0.2.5-dryrun.6` disposable release rehearsal

> The sixth rehearsal of the tag-driven release workflow, and the one
> whose **published bytes** the Linux compatibility matrix tests. It is
> a **rehearsal**: a disposable tag, a **draft** release, no
> publication. The production release `v0.2.5` is untouched and
> remains the latest published release.
>
> Every fact below was read from the authoritative store at the time of
> writing — the GitHub API for the tag, the workflow run, the draft
> release and its assets, and the workflow's own job logs — not from a
> watcher or from a remembered log.
>
> The earlier receipts are
> [RECEIPT-0.2.5-DRYRUN-3.md](RECEIPT-0.2.5-DRYRUN-3.md) (the first
> end-to-end run) and the rehearsals 4 and 5, whose findings are
> carried forward below.

## What was run

| Fact | Value | Where it was read |
|---|---|---|
| Tag | `v0.2.5-dryrun.6` | `GET /git/ref/tags/v0.2.5-dryrun.6` |
| Tag object type | `commit` — the tag is **lightweight** | same |
| Annotated tag object | none — `GET /git/tags/v0.2.5-dryrun.6` answers 404 | same |
| Tagged commit | `c396b18afba0a81f9589b7f74bd412aaca53d62d` | `GET /git/ref/tags/…` → `.object.sha` |
| Workflow trigger | `workflow_dispatch` | `GET /actions/runs/44188107435` → `.event` |
| Release workflow run | **44188107435** — `completed`/`success`, created 2026-11-17T22:14:43Z, head `c396b18a…` | `GET /actions/runs/44188107435` |
| `build artifacts` job | success — 2026-11-17T22:14:47Z → 22:31:49Z | `GET /actions/runs/44188107435/jobs` |
| `create draft release` job | success | same |
| Release mode | **rehearsal** | the run's own log line: `the workspace builds version 0.2.5 for tag v0.2.5-dryrun.6` |
| Draft release | id **449279892**, `draft: true`, `prerelease: true`, `published_at: null` | `GET /releases/449279892` |
| Draft title | `TEST 0.2.5-dryrun.6` | same |
| Draft notes | the `TEST RELEASE — not for publication.` banner and the seven-entry checksum list | same |

The draft was **not** published. Publishing a production release is a
maintainer-only act (campaign §20); a disposable rehearsal tag and a
draft are what this authorization covers.

## The nine assets, as published on the draft

Every asset is `state: uploaded`; the digest is the API's own
`sha256:` field for the asset bytes.

| Asset | Bytes | sha256 |
|---|---|---|
| `firecraft-0.2.5-linux-x86_64.tar.gz` | 24,389,908 | `cb55a52507477c9af327c35be8bb0e6d7dfc16a767e34bfde1aab0d92f346a3d` |
| `firecraft-cli-0.2.5-linux-x86_64.tar.gz` | 8,158,124 | `953633e40c2021858e7001f9691a47ad4ed4491ec752b38ccd716f8e99339093` |
| `firecraft_0.2.5_amd64.deb` | 16,228,104 | `7928369626375dfca80516551870d0ab7c98b3f3021ff5c82fc36a86dc39fa42` |
| `firecraft-web-0.2.5.zip` | 5,853,417 | `898be851f2e1abac03e0e01a58a3d5f593e74dd44acd572f0d47c4cbfef9d3c4` |
| `firecraft-0.2.5-source.tar.gz` | 3,176,024 | `0cbee1846cb24211fe3be899d286dd1917fd2464d2219f754ae9df7744d5f523` |
| `firecraft-0.2.5-sbom.cdx.json` | 369,745 | `e50516b21fcb5254824a262d73649d746b971a27b1d4a4ffcabe554f0b703174` |
| `MANIFEST.sha256` | 398 | `6a8692ed6806fe9074e8f29762bbbbaabd78ace8bba3e1c5bd5702ead4a5a101` |
| `release-manifest.json` | 3,295 | `54b8a231e2d4ff7e155915867f8d085dcfb86f944815032bd67e06ce2277ee57` |
| `RELEASE_SHA256.txt` | 664 | `af054b99c8c3b864de127597f23bf0f133b4d65f6504de70ba8ede11a61e8381` |

`RELEASE_SHA256.txt` was downloaded from the draft and compared with
the API's digests above: **7 of 7 entries match** — the desktop
tarball, the CLI tarball, the DEB, the web ZIP, the source archive,
the SBOM and `MANIFEST.sha256`. The list does not cover itself and
does not cover the manifest, so it holds seven entries for nine
assets.

The source archive, `RELEASE_SHA256.txt` and `release-manifest.json`
were each downloaded and re-hashed locally: **3 of 3 match** the API's
digests byte for byte.

## The manifest names its tag and its run

`release-manifest.json`, downloaded from the draft:

| Field | Value |
|---|---|
| `schema` | `firecraft.release-manifest/1` |
| `version` | `0.2.5` |
| `tag` | **`v0.2.5-dryrun.6`** |
| `source_sha` | `c396b18afba0a81f9589b7f74bd412aaca53d62d` |
| `run_url` | `https://github.com/aliasfoxkde/FireCraft/actions/runs/44188107435` |
| `run_id` | `44188107435` |
| `built_at` | `2026-11-17T22:31:44Z` |
| artifacts | 8 entries, each with its name, size, digest and verification status |

Two things had to be fixed for this to be possible, and both were
fixed before this rehearsal ran:

- **The tag.** A rehearsal is built from `main` and its tag is created
  *after* the build, so neither the git tag nor the CI ref name can
  name it — the manifest came out with `"tag": null` while the release
  it described had a tag. The caller now *states* the tag
  (`RELEASE_TAG=v0.2.5-dryrun.6` in the workflow's environment) and
  `xtask` validates the statement fail-closed: the stated tag must
  name the workspace version, either exactly (`v0.2.5`) or with a
  disposable suffix (`v0.2.5-dryrun.6`).
- **The run URL.** GitHub Actions does **not** export `GITHUB_RUN_URL`,
  so the manifest reconstructed it from the three variables it does
  export — `GITHUB_SERVER_URL`, `GITHUB_REPOSITORY` and
  `GITHUB_RUN_ID`.

## What the rehearsal proved

- **The workflow runs end to end through `workflow_dispatch`.** This
  was T27-57's outstanding item: until now the rehearsal path had
  never been exercised through the manual trigger, which is the only
  trigger a rehearsal is allowed to use.
- **The native set builds and installs.** `xtask dist --native`
  produced three archives, verified the DEB with `dpkg-deb`, and
  parse-checked nine shell recipes; then the workflow's own Debian 12
  container ran the repository's eight-stage package verification and
  **all eight stages passed** — install, desktop entry, icon, CLI,
  CLI document round trip, GUI launch, **upgrade**, uninstall and
  reinstall. The upgrade stage is exercised here, which the separate
  distro matrix does not exercise.
- **The web bundle builds and smokes.** `xtask dist --web` produced 7
  files (wasm 15,780,814 B) and ran the bridge smoke in headless
  Chromium: the log reads `smoke green` and `dist --web ok`. The smoke
  is not skipped — the workflow installs headless Chrome and fails
  closed without it.
- **The source archive is clean.** `xtask dist --source` reported
  `firecraft-0.2.5-source.tar.gz (0cbee184…), clean-extract + cargo
  metadata --locked verified`.
- **The SBOM is schema-valid and complete against the lockfile.**
  `562 components, 562 locked package(s), schema-valid`.
- **The manifest verifies.** `dist --manifest` wrote eight artifact
  entries and `dist --verify-manifest` re-checked them against the
  files: both steps succeeded.
- **The aggregate gate and the mode classifier both held.** The
  `create draft release` job ran `scripts/validate_release_aggregate.sh`
  (fail-closed on a missing, empty, extra, misnamed or digest-mismatched
  artifact, and on a skipped required platform) and
  `scripts/classify_release_mode.sh`, which classified this run as a
  rehearsal and produced a `TEST`-titled prerelease with the
  do-not-publish banner.

## Independent verification against the published bytes

These checks were run against the **downloaded assets**, not against
the build that produced them.

| Check | Result |
|---|---|
| `RELEASE_SHA256.txt` from the draft vs the API's digests | **7 of 7 match** |
| The three downloaded assets re-hashed locally | **3 of 3 match** the API's digests |
| The source archive extracted, then `cargo metadata --locked --offline` | exit **0** |
| The source archive extracted on a second host (Fedora), then `cargo metadata --locked` and `cargo build --locked` | `METADATA OK`, then `Finished dev profile … in 53.78s` on cargo 1.99.1 |
| The source archive's contents | the only `.claude/` entry is `.allow_config_edits` — **no `scratch/` directory and no `run_engine_test.sh`** |

## Reproducibility, measured against the previous rehearsal

`v0.2.5-dryrun.5` (draft 449261624) and `v0.2.5-dryrun.6` (draft
449279892) are two independent runs of the workflow on two commits
that differ only in the release scripts and one removed scratch file.
Their assets compare as follows:

| Asset | dryrun.5 | dryrun.6 | Same bytes? |
|---|---|---|---|
| desktop tar.gz | `cb55a525…` | `cb55a525…` | **yes** |
| CLI tar.gz | `953633e4…` | `953633e4…` | **yes** |
| DEB | `79283696…` | `79283696…` | **yes** |
| `MANIFEST.sha256` | `6a8692ed…` | `6a8692ed…` | **yes** |
| source archive | `8184bacb…` (3,176,830 B) | `0cbee184…` (3,176,024 B) | no — the commits differ, and dryrun.6 dropped the scratch script (806 B smaller) |
| SBOM | `219684d4…` | `e50516b2…` | no — **only** `metadata.timestamp` differs (`22:11:21Z` vs `22:31:43Z`); the 562 components, the dependency graph and the serial number are identical |
| web ZIP | `5e214be7…` | `898be851…` | no — **the payload is identical**: all seven files inside hash the same as dryrun.5's (`host.html` `379720cc…`, the js `3248368c…`, the wasm `089eada8…`, `index.html` `81b6b805…`, `appinfo.json` `7d039c36…`, `MANIFEST.sha256` `4f47125f…`, `favicon.png` `6e5975dd…`); only the ZIP container's own timestamps differ |
| `release-manifest.json` | `71f78945…` | `54b8a231…` | no — it carries the tag, the run and the build timestamp, which are what it exists to record |

So the **native artifacts are bit-for-bit reproducible** across
rehearsals, and the two artifacts that are not (the SBOM's timestamp
and the ZIP container's timestamps) differ only in material that is
not content.

## What the rehearsal found

**The published source archive no longer ships a scratch script.**
Rehearsal `v0.2.5-dryrun.5` published an archive that contained
`.claude/scratch/run_engine_test.sh` — a session helper that polls the
host's CPU and I/O pressure and then runs one engine test in a
worktree named `/nas/Temp/repos/FireCraft/.claude/worktrees/wt-hist-session`.
It had been force-added before `.claude/scratch/` was ignored, so the
ignore rule did not stop it and `git archive` — which ships exactly
what is tracked — published it as part of the release source.

The file is untracked now (commit `c396b18a`), and this rehearsal's
archive is the proof: extracted, its only `.claude/` entry is
`.allow_config_edits`. The archive's size dropped by exactly 806 B.

**The rehearsal tag is lightweight.** `GET /git/ref/tags/v0.2.5-dryrun.6`
reports an object of type `commit`, and `GET /git/tags/v0.2.5-dryrun.6`
answers 404 — there is no annotated tag object and therefore **no tag
message**. Rehearsal `v0.2.5-dryrun.3` could state "the tag says so
in its own message"; that property does not hold for a tag the
workflow creates. The "this is not a release" intent lives in the
draft state, the `TEST` title and the do-not-publish banner — which
is where the rehearsal's safety actually comes from — but the tag
itself is now indistinguishable from a production tag except by its
name.

**The test target did not compile.** Not a defect of this rehearsal,
but found while it ran: the two manifest tests added for rehearsals 5
and 6 call `stated_tag_names_version` and `built_run_url` from their
own module without importing them, so `cargo clippy --all-targets`
failed with thirteen `E0425` errors on `c396b18a`. The release build
never compiles the tests, so every rehearsal passed while the test
target was broken. Fixed in `d1029347`.

## What this receipt does not claim

- **It does not certify anything.** Nothing in this repository is
  signed and no provenance or attestation exists, so no artifact can
  reach rung 9 of the evidence ladder. This receipt is evidence, not
  a certification; the certification report is
  [CERTIFICATION.md](CERTIFICATION.md), and it says the same thing.
- **It does not publish anything.** The release is a draft with
  `published_at: null`, and the production `v0.2.5` release is
  untouched by this rehearsal.
- **It does not make the live deployment a release.** The live site
  still serves a bundle that matches no release; the comparison is in
  [DEPLOYMENT-RECONCILIATION.json](DEPLOYMENT-RECONCILIATION.json) and
  its result is `differences`.
- **It does not cover the extended scope.** No Windows, macOS,
  AppImage, Flatpak, Android, ARM64 or RPM artifact exists.
- **It does not claim the desktop session was tested.** The matrix
  that consumes these bytes ran under Xvfb and weston's headless
  backend; see [LINUX-COMPAT.md](LINUX-COMPAT.md).

## Related documents

- [CERTIFICATION.md](CERTIFICATION.md) — what the approved release
  scope can honestly be certified for.
- [LINUX-COMPAT.md](LINUX-COMPAT.md) — the six-cell matrix that
  installs and launches these bytes.
- [EXECUTION-BOARD.md](EXECUTION-BOARD.md) — the per-unit ledger.
- [ARTIFACT-MATRIX.md](ARTIFACT-MATRIX.md) — one row per artifact.
- [RECEIPT-0.2.5-DRYRUN-3.md](RECEIPT-0.2.5-DRYRUN-3.md) — the first
  rehearsal, which found the source-archive and naming defects.
