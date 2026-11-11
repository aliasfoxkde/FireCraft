# Releasing FireCraft

> **Full cross-platform release campaign:** the commands below describe
> the release path this repository can actually perform today — a
> Linux-x86_64 native set, a web bundle, a source archive, an SBOM, and
> a draft GitHub release. For the Windows, macOS, AppImage, Flatpak,
> Android, signing, and certification work order see
> [`RELEASE-ENGINEERING-PLAN.md`](RELEASE-ENGINEERING-PLAN.md) and
> ticket [T27](tasks/T27-cross-platform-release.md). **Do not infer that
> a target is supported from a recipe alone.**
>
> **Scope authority:**
> [decision 0075](decisions/0075-release-scope-mvp.md). Which artifacts
> constitute an acceptable cut is decided there; its approval state is
> *recommended — awaiting maintainer approval*.
>
> **Evidence baseline:** [PHASE-0-AUDIT.md](release/PHASE-0-AUDIT.md)
> (PR #16, dated 2026-11-09 — the link resolves once that PR merges).
> Per-artifact evidence state:
> [ARTIFACT-MATRIX.md](release/ARTIFACT-MATRIX.md).

Everything below is one verb per artifact set, run from a clean tree.
Artifacts are local; the signing ceremony is the operator's. The repo
never carries an untested claim of a built artifact — platforms this
fleet cannot build ship as recipes under `packaging/`
([docs/TOOLCHAINS.md](TOOLCHAINS.md)).

## Preconditions

- `cargo run -p xtask -- ci` — the canonical gate. On a host where the
  workspace-wide `clippy --all-targets` and `test --workspace` steps are
  not permitted, this gate is routed through CI; the receipt is the CI
  **run id**, not a local statement that it was green.
- Version stamp: `[workspace.package] version` in the root `Cargo.toml`
  **and** the literal `Version:` in `packaging/deb/control` move
  together. `xtask stamps` and `xtask dist --native` fail on drift.
- Every crate carries the workspace version; `xtask stamps`
  enforces one stamp across all workspace-local packages.

## Web bundle (WASM-first — the primary target)

    cargo run -p xtask -- dist --web

Produces `dist/web/`:

| artifact | note |
|---|---|
| `index.html`, `host.html`, `appinfo.json`, one `.js`, one `.wasm` | trunk `--release` bundle |
| `MANIFEST.sha256` | over the bundle; `sha256sum -c` verifies |

The gate enforces the wasm size ceiling (parity table) and runs the
bridge smoke **on these release bytes**: the WEBOS.md host page loads
the bundle in headless chromium and completes one
open → apply → exportPng → exportDocument round trip through the M5
postMessage bridge. A dist run that cannot prove the bridge does not
claim the artifact.

The bridge smoke has no escape hatch. `FIRECRAFT_DIST_SKIP_SMOKE` was
removed with the CI image's missing browser: the image now bakes
chromium, and a dist run that cannot find a browser — or whose bridge
round trip does not report `green` — fails instead of skipping.

## Linux native set

    cargo run -p xtask -- dist --native

Produces `dist/native/`:

| artifact | note |
|---|---|
| `firecraft-<v>-linux-x86_64.tar.gz` | desktop shell + CLI, `firecraft-<v>/{bin,share/licenses}` layout |
| `firecraft-cli-<v>-linux-x86_64.tar.gz` | CLI alone |
| `firecraft_<v>_amd64.deb` | built from the pinned `packaging/deb/control` |
| `MANIFEST.sha256` | over the three archives; `sha256sum -c` verifies |

The gate itself runs `dpkg-deb --info` and `--contents` on the deb
(name/version fields, both binaries present) and fail-closes when the
control file's declared libc6 floor is below either binary's real GLIBC
requirement. Install test by hand: `apt install ./firecraft_<v>_amd64.deb`
or `dpkg -i`.

## Source archive and SBOM

    cargo run -p xtask -- dist --source
    cargo run -p xtask -- dist --sbom

Both verbs exist on this branch and are wired into the release workflow.
They do **not** exist on `main` — the two release branches carry
duplicate implementations that have to be consolidated before either
lands. `dist --source` produces `dist/source/FireCraft-<v>-source.tar.gz`
and verifies a clean extraction plus `cargo metadata --locked` inside it;
`dist --sbom` produces `dist/sbom/firecraft-<v>-sbom.cdx.json`
(CycloneDX 1.5) over the locked dependency graph.

## Off-fleet platforms (recipes — the operator builds these)

- AppImage: `packaging/build-appimage.sh` (needs
  linuxdeploy-x86_64.AppImage; `bash -n` parse-checked by
  `dist --native`)
- Flatpak: `packaging/build-flatpak.sh` + manifest
  `packaging/flatpak/dev.firecraft.FireCraft.yml` (needs the Flatpak SDK;
  script `bash -n` parse-checked and manifest YAML-parse-checked by
  `dist --native`)
- macOS universal dmg: `packaging/build-macos-dmg.sh` (needs macOS +
  Xcode CLT; lipo universal + ad-hoc codesign + hdiutil)
- Windows portable zip: `packaging/build-windows.ps1`; optional Inno
  Setup installer from `packaging/windows/firecraft.iss`

The PowerShell and Inno Setup recipes **cannot be parsed on this
fleet** — the gate only checks that the files exist. Their syntax is
verified on the target platform by whoever first runs them.

Record the sha256 of every recipe-built artifact you ship in the
release notes — the dist manifests cover only what the fleet built.

## Release notes

    cargo run -p xtask -- changelog

Regenerates `docs/CHANGELOG.md` from the conventional-commit log
(grouped by type, git log order, every commit accounted for) and
verifies the license files against the dual claim and the version
stamps. **The verb writes the file** — it is a step of the canonical
gate, so every gate run leaves a regenerated copy in the working tree.
Commit the regenerated copy deliberately rather than being surprised by
it in a diff. A changelog generated from the log cannot contain its own
tip commit, so the file is never byte-identical to a freshly generated
one.

The stamp half of that verb is also available on its own:

    cargo run -p xtask -- stamps

`xtask stamps` verifies the license claim and the workspace version
stamps and needs no git. That is the verb CI runs: the CI image ships
no git, so `xtask changelog` cannot run there (decision 0086).

## Tag-driven release (what the workflow actually does)

`.github/workflows/release.yml` fires on a `v*` tag push and runs two
jobs:

1. **`build artifacts`** — `dist --native`, `dist --web`,
   `dist --source`, `dist --sbom`; zips the web bundle; writes
   `dist/RELEASE_SHA256.txt` over the seven digested files plus
   `MANIFEST.sha256`; uploads eight paths as one artifact.
2. **`create draft release`** — downloads that artifact, flattens it
   with `packaging/release-assets.sh stage`, runs a **fail-closed
   aggregate validation** (all eight required artifacts present and
   non-empty, `sha256sum -c RELEASE_SHA256.txt` passing, and every
   artifact name carrying the tag version), then creates a **draft**
   GitHub release with the assets passed as positional arguments.

The workflow does **not** publish the release, does not bump versions,
does not run `xtask changelog`, and does not deploy to Cloudflare Pages
(the Pages step is commented out).

> **The workflow has never produced a release.** Every recorded run of
> the old workflow failed, and all six published releases were created
> by hand. The repaired workflow on this branch is unproven end to end:
> its first real run is itself the receipt this repository lacks.

## Cloudflare Pages deploy (manual)

The Pages deploy is **not** part of the release workflow — the step is
commented out in `release.yml`. The operator deploys the built bundle
directly:

    cd dist/web && zip -qr ../firecraft-web-<v>.zip .
    wrangler pages deploy dist/web --project-name firecraft-web

Authenticated via `CLOUDFLARE_API_TOKEN`; the project is a direct-upload
project at <https://firecraft-web.pages.dev>.

**A successful deploy command is not a deployment reconciliation.**
Cloudflare Pages injects its analytics beacon into the HTML it serves,
so the served `index.html` and `host.html` no longer match the
manifest's digests — `sha256sum -c` against the live site fails on
those two entries *by construction*, and the manifest is valid only for
the pre-deploy bytes. The live bundle must be reconciled against a
tagged release by hashing the deployed assets individually and
recording which commit produced them.

## Checklist

Every step below is one the repository can actually perform today.

1. `xtask ci` green — **record the CI run id and the date**. A bare
   "CI is green" without a run id is not a receipt. The CI job's
   `Web bundle smoke` step is tag-gated and is **skipped on branch
   pushes**; on a branch, the bridge smoke has to come from a
   `dist --web` run, not from the CI job.
2. Version stamped in `Cargo.toml` **and** `packaging/deb/control`.
3. `xtask dist --web` green — manifest verifies, size budget holds,
   bridge smoke green on the release bytes.
4. `xtask dist --native` green — manifest verifies, `dpkg-deb`
   read-back passes.
5. `xtask dist --source` and `xtask dist --sbom` green — clean-extract
   verification and CycloneDX output present.
6. `xtask changelog` refreshed and the regenerated
   `docs/CHANGELOG.md` committed.
7. Off-fleet recipes executed on their platforms (only for those
   targets being released); hashes recorded in the release notes. If a
   recipe was not executed on its target platform, it is not a release
   artifact.
8. Tag `v<version>` and push to **origin**. The GitForge remote is a
   **divergent lineage** — a push to `gitforge main` is rejected as
   non-fast-forward, so the "push both remotes" instruction cannot be
   completed without a force-push, which is forbidden. Push the tag to
   GitForge only when the lineage question is resolved; the GitForge
   remote carries tags only and has no release-asset surface.
9. The tag push fires `release.yml`, which creates a **draft** release
   with the eight validated assets. Inspect the draft: confirm the
   asset list, confirm the digests against `RELEASE_SHA256.txt`, and
   confirm the release notes are `docs/releases/<tag>.md`.
10. Deploy the web bundle to Cloudflare Pages manually, then reconcile
    the deployed assets against the tagged bundle and record the result.
11. **Publishing the draft is a separate, authorized action.** Publishing
    a production release requires explicit maintainer approval; the
    workflow never publishes.

## What this checklist does not cover

- **Signing.** No signing identity is configured anywhere; no artifact
  can pass a security/signature check, so nothing here is
  `CERTIFIED`.
- **Provenance and attestation.** No generator exists.
- **A release-wide checksum file that has actually been published.**
  `release.yml` writes `RELEASE_SHA256.txt`, but no published release
  carries one yet, and no published asset has a GitHub digest.
- **Target-native Windows, macOS, AppImage, or Flatpak evidence.** None
  of those artifacts has been produced.
- **Reproducibility.** A rebuild of the current tree does not reproduce
  the published bytes, and the bundler version is not recorded in any
  build receipt.
