# Release certification — FireCraft 0.2.5

> What this repository can honestly certify about the 0.2.5 release
> line, what it cannot, and what would have to change before a
> production release is published.
>
> Authority: [decision 0075](../decisions/0075-release-scope-mvp.md)
> (the approved release scope) and
> [ARTIFACT-MATRIX.md](ARTIFACT-MATRIX.md) (the per-artifact evidence
> ledger). Execution ledger: [EXECUTION-BOARD.md](EXECUTION-BOARD.md),
> rows T27-19 and T27-60.

## What is certified, and what is not

**Nothing in this repository is `CERTIFIED` in the strict sense, and
this report does not change that.** The evidence ladder in
[PHASE-0-AUDIT.md](PHASE-0-AUDIT.md) §0.3 puts *security/signature
checks completed* at rung 9, and this repository has no signing
identity, no attestation generator and no provenance material. Every
artifact here therefore stops at rung 8 (functional smoke completed) on
the rehearsal path, and at rung 11 (downloaded published artifact
independently verified) only for the assets that were published out of
band on 2026-11-09.

What this report *does* certify is narrower and real:

1. The release workflow **runs end to end** and produces the approved
   artifact set with a fail-closed aggregate gate.
2. The **artifacts themselves** are real, installable, launchable and
   functionally smoke-tested on the platforms named below, and their
   bytes are reproducible.
3. The **scope** is recorded and authorized — decision 0075 is
   accepted and names the approver and the date.
4. The **publication** step is *not* authorized and has not been
   taken; the only publication that exists is the out-of-band
   `v0.2.5` release, and it carries three open defects.

## The release identity

| Fact | Value | Where it was read |
|---|---|---|
| Workspace version | `0.2.5` | `cargo metadata` (the workspace's `xtask` package version) |
| Source SHA (rehearsal head) | `c396b18a` | the rehearsal run's `head_sha` |
| Rehearsal tag | `v0.2.5-dryrun.6` | the draft release's `tag_name` |
| Rehearsal workflow run | `44188107435` | `GET /actions/runs/44188107435` |
| Draft release | id **449279892**, `draft: true`, `prerelease: true`, title `TEST 0.2.5-dryrun.6` | `GET /releases/449279892` |
| Rehearsal tag shape | **lightweight** — `GET /git/ref/tags/v0.2.5-dryrun.6` reports an object of type `commit`, and `GET /git/tags/…` answers 404, so the tag carries no message of its own | same |
| Release mode | rehearsal | `scripts/classify_release_mode.sh` — `vX.Y.Z-dryrun.N` is a rehearsal, never a production release |
| Release state | **draft, unpublished** | `GET /releases/<id>` → `draft: true`, `published_at: null` |
| Toolchain | rustc / cargo **1.99.1** | the workflows' `toolchain:` pin (decision 0102) |
| Native build image | `rust:1.99-slim-bookworm` (Debian 12) | `.github/workflows/release.yml` |
| CI image | `rust:1.99.1-slim-bookworm` | `infrastructure/docker/ci-rust.Dockerfile` |
| GitHub runner | `ubuntu-24.04` | the workflow's `runs-on` |
| GitForge runner | the Fedora host | the CI job store's `runner_id` |

The rehearsal receipt with the run-by-run detail is
[RECEIPT-0.2.5-DRYRUN-6.md](RECEIPT-0.2.5-DRYRUN-6.md).

## The approved scope and its matrix

[decision 0075](../decisions/0075-release-scope-mvp.md) approved nine
items as the release scope. This is where each one stands:

| # | Approved item | Artifact | Rung | Evidence |
|---|---|---|---|---|
| 1 | Web / WASM bundle | `firecraft-web-0.2.5.zip` | 8 | built by the workflow; the bridge smoke ran green in headless Chromium against the built bundle |
| 2 | Linux x86_64 desktop archive | `firecraft-0.2.5-linux-x86_64.tar.gz` | 8 | installed and launched on six cells; the bytes are the published rehearsal bytes |
| 3 | Linux x86_64 DEB | `firecraft_0.2.5_amd64.deb` | 8 | installed with `apt` on Ubuntu 24.04 and Debian 12; the repository's own eight-stage verification passes |
| 4 | Linux x86_64 CLI archive | `firecraft-cli-0.2.5-linux-x86_64.tar.gz` | 8 | extracted and run on six cells; `firecraft-cli --version` prints the stamped version |
| 5 | Source archive | `firecraft-0.2.5-source.tar.gz` | 8 | clean extraction on a second host runs `cargo metadata --locked` and `cargo build --locked` to completion |
| 6 | Checksums | `RELEASE_SHA256.txt` | 8 | 7 of 7 entries match the API's own digests for the assets it covers |
| 7 | SBOM | `firecraft-0.2.5-sbom.cdx.json` | 8 | validated against the vendored CycloneDX 1.5 schema and compared component-by-component with `Cargo.lock` |
| 8 | Verified production web deployment | — | **blocked** | the live deployment serves a bundle that matches no release (see below) |
| 9 | Release certification | this document | — | exists; it certifies nothing beyond the evidence it lists |

**Item 8 is not satisfied.** The approved scope asks for a *verified*
production deployment, and the deployment cannot be verified: see
[Deployment reconciliation](#deployment-reconciliation). This is the
one approved-scope item that is blocked, and it blocks the release
cut on its own evidence — not on authorization.

## The artifacts, as published on the rehearsal draft

Every asset is `state: uploaded`; the digest is the GitHub API's own
`sha256:` field for the asset bytes, not a digest this repository
computed.

| Asset | Bytes | sha256 | Runner / toolchain |
|---|---|---|---|
| `firecraft-0.2.5-linux-x86_64.tar.gz` | 24,389,908 | `cb55a52507477c9af327c35be8bb0e6d7dfc16a767e34bfde1aab0d92f346a3d` | ubuntu-24.04 · `rust:1.99-slim-bookworm` (Debian 12) · rustc 1.99.1 |
| `firecraft-cli-0.2.5-linux-x86_64.tar.gz` | 8,158,124 | `953633e40c2021858e7001f9691a47ad4ed4491ec752b38ccd716f8e99339093` | same |
| `firecraft_0.2.5_amd64.deb` | 16,228,104 | `7928369626375dfca80516551870d0ab7c98b3f3021ff5c82fc36a86dc39fa42` | same |
| `firecraft-web-0.2.5.zip` | 5,853,417 | `898be851f2e1abac03e0e01a58a3d5f593e74dd44acd572f0d47c4cbfef9d3c4` | same (wasm 15,780,814 B) |
| `firecraft-0.2.5-source.tar.gz` | 3,176,024 | `0cbee1846cb24211fe3be899d286dd1917fd2464d2219f754ae9df7744d5f523` | same |
| `firecraft-0.2.5-sbom.cdx.json` | 369,745 | `e50516b21fcb5254824a262d73649d746b971a27b1d4a4ffcabe554f0b703174` | same (562 components, 562 locked packages) |
| `MANIFEST.sha256` | 398 | `6a8692ed6806fe9074e8f29762bbbbaabd78ace8bba3e1c5bd5702ead4a5a101` | same |
| `release-manifest.json` | 3,295 | `54b8a231e2d4ff7e155915867f8d085dcfb86f944815032bd67e06ce2277ee57` | same (schema `firecraft.release-manifest/1`) |
| `RELEASE_SHA256.txt` | 664 | `af054b99c8c3b864de127597f23bf0f133b4d65f6504de70ba8ede11a61e8381` | same |

All nine are `state: uploaded` on draft release **449279892**
(`tag_name` `v0.2.5-dryrun.6`, `draft: true`, `prerelease: true`,
`published_at: null`).

## Install, launch and smoke evidence

The evidence is the **six-cell distro × display matrix**, run on the
Fedora host on 2026-11-17 against the repository's own tracked scripts
(`scripts/release-matrix.sh` and `packaging/linux/leg-*.sh` at
`8f9a602c`).

| Distribution | Version | DEB | desktop tar.gz | CLI tar.gz | X11 | Wayland |
|---|---|---|---|---|---|---|
| Ubuntu | 24.04.5 LTS | installed + 8 stages | launched | ran | pass | pass |
| Debian | 12 (bookworm) | installed + 8 stages | launched | ran | pass | pass |
| Fedora | 45 | n/a — format | launched | ran | pass | pass |

The Debian-family cells install the package with `apt` — so apt itself
resolves the declared dependencies — and then run the repository's own
`packaging/deb/verify-install.sh`: metadata, desktop entry (accepted by
`desktop-file-validate`), icon, CLI, CLI document round trip (a 97 B
PNG with the signature `89504e470d0a1a0a`), GUI launch under Xvfb
(stayed up for the 25 s launch window, exit 124), uninstall and
reinstall. The Wayland cells run weston's headless backend and launch
the binary with `WAYLAND_DISPLAY=wayland-0`. The full statement of what
each cell did, and of what no cell did, is
[LINUX-COMPAT.md](LINUX-COMPAT.md).

The web bundle's smoke is the bridge round trip — open → apply →
`exportPng` → `exportDocument` — run by `xtask dist --web` in headless
Chromium. It is **not** skipped: the release workflow installs headless
Chrome and fails closed without it, and the environment variable that
would skip it appears nowhere in the CI configuration.

## Signature, SBOM and provenance status

| Property | Status | Consequence |
|---|---|---|
| Artifact signatures | **none** — no Authenticode, Developer ID or GPG signing step exists in any workflow, and no signing identity is configured | no artifact can reach rung 9; no artifact is `CERTIFIED` |
| Provenance / attestation | **none** — no in-toto, SLSA, `cosign` or `gh attestation` machinery exists in the repository (board T27-56, blocker B-14) | the release set is not bound to a source commit by anything a third party can verify |
| SBOM | **present** — CycloneDX 1.5 JSON, validated against the vendored schema and compared component-by-component with `Cargo.lock`; the comparison names both sides of any difference | the *dependency inventory* is attested; the *artifact* is not |
| Release manifest | **present** — `release-manifest.json`, schema `firecraft.release-manifest/1`, carrying the version, the tag, the source SHA, the run URL, the timestamp and one entry per artifact with its size, digest, format and target triple | the release describes itself; a third party cannot verify the description without the digests being independently attested |

The gap between "the release describes itself" and "the release is
attested" is exactly what T27-56 and blocker B-14 describe, and it is
the reason this report stops where it stops.

## Deployment reconciliation

The comparison is a tracked tool and a machine-readable report:
`scripts/reconcile_deployment.py` fetches the live deployment's
`MANIFEST.sha256` and every file it names, hashes the bytes it
actually received, and compares them file by file with the release
bundle's manifest. The report is
[DEPLOYMENT-RECONCILIATION.json](DEPLOYMENT-RECONCILIATION.json), and
its result is **`differences`**.

- The live deployment serves bundle `2274ac8c2925b61d`.
- The `v0.2.5` release bundle carries bundle `f9dbe2f14305b2d9`.
- Each asset name appears only on its own side of the inventory.
- `appinfo.json` matches byte for byte (`7d039c36…`); `index.html`
  and `host.html` differ only by the substituted asset names, their
  integrity digests and the Pages analytics beacon.

**The live deployment corresponds to no identifiable release.** The
deploy step in `release.yml` is commented out and no
`CLOUDFLARE_API_TOKEN` is provisioned (blockers B-11, B-12), so the
deployment cannot be advanced from the release pipeline and cannot be
reconciled from it either. Approved-scope item 8 stays blocked.

**Re-verified 2026-11-17.** The comparison was run again against the
live site: the deployment's `MANIFEST.sha256` digest is unchanged at
`61e3aeda5e8dcc340725a52c6a421422198f29b9ee8c1ed9a91832f372985261`,
and the comparison against the rehearsal's web bundle reports eight
differences — `appinfo.json` matches, and every other file differs.
The live bundle has not moved.

## CI links and the state of the gates

| Gate | What it proves | Where |
|---|---|---|
| `cargo fmt --all -- --check` | the tree is rustfmt-clean | GitHub `fmt`; GitForge `fmt-layers` |
| `cargo clippy --workspace --all-targets -- -D warnings` | no lint, in any target including the tests | GitHub `gate`; GitForge `clippy-test` |
| workspace tests | the suite passes | GitHub `gate`; GitForge `clippy-test` |
| `xtask layers` | the crate dependency layers hold | GitHub `gate`; GitForge `layers` |
| Aegis secret scan | no new secret finding against the content-keyed baseline | GitHub `gate`; GitForge `aegis` |
| the release ledger | the board and its tickets agree | GitHub `gate`; GitForge `release-ledger` |
| the aggregate release gate | every required artifact is present, non-empty, correctly named, digest-matching and platform-covering | `scripts/validate_release_aggregate.sh`, run by the release workflow |

The rehearsal also exercised the release workflow's own pin and mode
gates: `scripts/validate_workflow_pins.py` (every third-party action
pinned to a full commit SHA), `scripts/classify_release_mode.sh` (a
manual run may only rehearse; a rehearsal is a prerelease titled
`TEST <version>` with a do-not-publish banner), and
`scripts/validate_release_aggregate.sh` (fails closed on a missing,
empty, extra, misnamed or digest-mismatched artifact, and on a skipped
required platform).

## Limitations

- **Nothing is signed or attested.** See the table above; this is the
  load-bearing limitation of the whole report.
- **The rehearsal is a draft.** Its assets are published to a draft
  release, not to a published release, and its tag is disposable. The
  evidence in this report is about *the rehearsal's bytes*, which are
  the bytes the workflow produces — not about a published release.
- **The production `v0.2.5` assets are not these bytes.** They were
  published out of band on 2026-11-09 (blocker B-21), they are smaller
  than the workflow's rebuilds, and they carry the three open defects
  below. The matrix in this report tests the *rehearsal's* bytes.
- **The matrix ran in containers**, with software rendering, under Xvfb
  and weston's headless backend. No cell used a real GPU, a real
  compositor or a real desktop session; desktop integration was
  checked at the package level only.
- **The DEB's upgrade path was not exercised** — the eight-stage
  verification needs a previous package to upgrade from, and none was
  supplied.
- **No Windows, macOS, AppImage, Flatpak, Android, ARM64 or RPM
  artifact exists.** Those are outside the approved scope
  ([decision 0075](../decisions/0075-release-scope-mvp.md), extended
  campaign) and are not claimed here.
- **The web deployment is unreconciled and the live site's bytes match
  no release.**

## Unsupported targets

Windows (portable ZIP and Inno Setup installer), macOS (`.app` and
DMG), Linux AppImage, Linux Flatpak, Linux RPM, Linux aarch64,
Windows ARM64 and Android are **not supported by this release**. Each
is either outside the approved scope or inside the extended campaign
with its own acceptance evidence outstanding. Nothing in this
repository may advertise them as supported before that evidence
exists.

## Residual risk

| Risk | Severity | Why it is still open |
|---|---|---|
| A consumer cannot verify that a downloaded artifact is the one this repository built | **High** | no signature and no provenance exist (T27-56, B-14) |
| The live web deployment is not a release | **High** | the deploy step is disabled and no token is provisioned (B-11, B-12); the deployed bundle matches no release |
| The published `v0.2.5` DEB installs cleanly and then fails to start on older distributions | **High** | the published asset under-declares its glibc floor (B-7); the workflow's rebuild fixes it, but the published asset is out of band and unreplaced |
| The published `v0.2.5` DEB is not a desktop application | **Medium** | it carries no desktop entry and no icon (B-24); the workflow's rebuild carries both |
| The published `v0.2.5` DEB declares only `libc6` | **Medium** | it omits the X11/GL runtime stack the binary needs (issue #39); the workflow's rebuild declares the full set |
| The published `v0.2.5` assets are not reproducible from the tree | **Medium** | they were published out of band (B-21); the workflow's own rebuilds *are* byte-identical to each other across rehearsals |
| A release cut is published from a tree whose gates were not run on that exact SHA | **Low** | mitigated by requiring the GitForge receipt for the same candidate SHA before any merge or publication |

## Independent verification results

These are checks run **against the published bytes**, not against the
build that produced them.

| Check | Result |
|---|---|
| `RELEASE_SHA256.txt` downloaded from the draft and compared with the API's own digests | **7 of 7 match** (the list does not cover itself or the manifest, so it holds seven entries for nine assets) |
| The source archive, `RELEASE_SHA256.txt` and `release-manifest.json` downloaded and re-hashed locally | **3 of 3 match** the API's digests byte for byte |
| The source archive extracted, then `cargo metadata --locked --offline` | exit **0** |
| The source archive extracted on a second host (Fedora), then `cargo metadata --locked` and `cargo build --locked` | `METADATA OK`, then `Finished dev profile … in 53.78s` on cargo 1.99.1 |
| The source archive's contents | the only `.claude/` entry is `.allow_config_edits` — no scratch helper is published |
| The native artifacts of two independent rehearsals compared byte for byte | **identical** — the desktop tarball, the CLI tarball, the DEB and `MANIFEST.sha256` have the same digests in two separate runs |
| The web bundle's payload of two independent rehearsals compared file by file | **identical** — all seven files hash the same; only the ZIP container's timestamps differ |
| The SBOM of two independent rehearsals compared field by field | identical in every field except `metadata.timestamp`; the 562 components, the dependency graph and the serial number match |
| The six-cell matrix run against the published asset digests | all six cells pass; the artifacts under test are the published bytes, not a local rebuild |

## What publication would require

1. A maintainer decision on the live web deployment: either provision
   `CLOUDFLARE_API_TOKEN` and re-enable the deploy step so that
   approved-scope item 8 can be satisfied, or record that the release
   is published without a verified deployment.
2. A signing identity and a provenance generator, so that the release
   can reach rung 9 and its artifacts can be `CERTIFIED` (T27-56).
3. The maintainer's written publication authorization (T27-61,
   blocker B-04). Nothing in this repository may publish a production
   release without it; the rehearsals above are what that authorization
   would replace.
4. A decision on the out-of-band `v0.2.5` assets — re-cut them from
   the workflow, or record them as superseded.

## Related documents

- [ARTIFACT-MATRIX.md](ARTIFACT-MATRIX.md) — one row per artifact,
  with its evidence state and blockers.
- [EXECUTION-BOARD.md](EXECUTION-BOARD.md) — the per-unit ledger.
- [LINUX-COMPAT.md](LINUX-COMPAT.md) — what is *tested* on Linux, and
  what is only built or deferred.
- [RECEIPT-0.2.5-DRYRUN-6.md](RECEIPT-0.2.5-DRYRUN-6.md) — the
  rehearsal receipt behind this report.
- [PHASE-0-AUDIT.md](PHASE-0-AUDIT.md) — the evidence baseline, the
  evidence ladder and blockers B-1…B-21.
- [../decisions/0075-release-scope-mvp.md](../decisions/0075-release-scope-mvp.md)
  — the approved release scope.
