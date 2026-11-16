# T27-42 — Audit the DMG script and verify both macOS architectures build

> **RP:** RP-03 · **Priority:** P1 · **Status:** IN PROGRESS (audited 2026-11-12; the build half needs a macOS host)
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-04

**Objective:** audit the macOS packaging script and establish that both Apple Silicon
and Intel binaries build, and decide whether a universal2 artifact is achievable.

**Inputs:** `packaging/build-macos-dmg.sh`; `docs/TOOLCHAINS.md`; a macOS host with
Xcode CLT; `lipo`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.3 (macOS requirements, including
the audit and the both-architecture verification).

**Acceptance criteria:** the script is audited and its environment assumptions
recorded; `aarch64` and `x86_64` targets both build; universal2 feasibility is decided
and written; `lipo -info` confirms the slices actually present in the artifact.

**Verification:** `lipo -info <binary>`

**Evidence:** the audit below; [decision
0101](../../decisions/0101-off-fleet-packaging-recipe-audit.md) records the
consequences. The `lipo -info` receipt does not exist yet — no macOS host.

**Blockers:** no macOS host. The build and slice-verification halves of this row are
still open; T27-43 (bundle and DMG), T27-44 (native tests) and T27-45 (signing) depend
on them.

**Stopping condition:** both slices are built and verified, and the universal2 decision
is recorded. **Half met 2026-11-12** — the decision is recorded and the script is
audited; the slices have never been built.

---

## The audit

### Environment assumptions and their risks

| # | Assumption | How it is met | Risk if unmet |
|---|---|---|---|
| M1 | A macOS host with the Xcode command-line tools | `rustup target add`, `cargo build`, `lipo`, `codesign` and `hdiutil` all come from the Apple toolchain | Nothing builds; the recipe fails on the first `rustup` call |
| M2 | Both Rust targets are installable | `rustup target add aarch64-apple-darwin x86_64-apple-darwin` | The build for the missing architecture fails |
| M3 | The crates are fetchable | `--locked` pins the lockfile; the fetch itself still happens on a cold cache | The build fails on a fetch |
| M4 | `lipo` can combine the two slices | `lipo -create` for the desktop binary and the CLI | The universal bundle does not exist; the recipe fails |
| M5 | `codesign` is available | `codesign --force --deep -s -` (ad-hoc) | The bundle is unsigned and Gatekeeper refuses it on a quarantined download — see E4 |
| M6 | `hdiutil` can create a UDZO image | `hdiutil create -srcfolder ... -format UDZO` | No DMG is produced |
| M7 | The workspace version is readable | `awk` over `[workspace.package]` | **Was silent** — see E2 |

### Product identity

| Field | Value | Source |
|---|---|---|
| `CFBundleName` / `CFBundleDisplayName` | `FireCraft` | the project name |
| `CFBundleIdentifier` | `dev.firecraft.firecraft` | mirrors the WebOS packaging id (`docs/WEBOS.md`) |
| `CFBundleVersion` / `CFBundleShortVersionString` | the workspace version | `[workspace.package] version` in `Cargo.toml` |
| `CFBundleExecutable` | `firecraft` | the desktop binary's name |
| `CFBundlePackageType` | `APPL` | an application bundle |
| `NSHighResolutionCapable` | true | a Retina-capable bundle |
| `CFBundleIconFile` | **unset** | the repository has no `.icns` — see E3 |
| Output file | `firecraft-<version>-macos-universal.dmg` | the version stamp plus the platform label |

### The universal2 decision

**One universal2 artifact, not two per-architecture images.** The recipe already builds
both slices (`aarch64-apple-darwin` and `x86_64-apple-darwin`) and combines them with
`lipo -create` for both the desktop binary and the CLI, so the `.app` it stages is
already a universal2 bundle. Nothing in the dependency graph needs a per-architecture
dylib — the Rust binaries are statically linked against the Rust runtime and link the
system frameworks through `dlopen`-style weak imports, which is why a single
`lipo`-combined binary is enough.

The consequences of that decision:

- The DMG is named `macos-universal` and carries one universal `.app`; there is no
  separate `-aarch64` and `-x86_64` DMG to publish, and the release asset list has one
  macOS entry instead of two.
- `lipo -info` on the staged binary is the receipt that both slices are really there —
  it is exactly the verification this row names, and it has never been run.
- The one thing `lipo` cannot cover is the bundle **icon**: an `.icns` is a per-
  architecture container in practice, and no `.icns` exists yet.

## Defects the audit found, and the fixes

**E1 — the bundle shipped without its `NOTICE`.** Same defect as D2 on Windows: Apache-2.0
§4(d) requires the attribution notices to travel with the distribution, and the Adobe
trademark disclaimer lives in `NOTICE`. **Fixed:** the bundle stages `NOTICE` beside
`LICENSE`.

**E2 — an unreadable version failed silently.** The recipe read the version with
`readonly VERSION="$(version())"`. The `readonly` builtin returns success whatever the
command substitution did, so `set -e` never fired: a manifest without a readable
version produced a file named `firecraft--macos-universal.dmg` and a bundle whose
`Info.plist` carried an empty version — a published artifact with no version in it.
**Fixed:** the recipe now checks the value and exits with a message naming the missing
key.

**E3 — the bundle has no icon.** `CFBundleIconFile` is unset because the repository has
no `.icns`, and the brand refresh (issue #131) owns the artwork; an icon derived from
today's mark would be stale the moment it lands. The gap is recorded here and in T27-43
rather than papered over with the wrong icon. The same gap exists on the Windows
installer (`SetupIconFile`).

**E4 — the signature is ad-hoc, and ad-hoc is not notarization.** `codesign -force
--deep -s -` produces a signature that satisfies the loader but not Gatekeeper: a user
who downloads the DMG in a browser gets a quarantine attribute, and the bundle is
refused until it is signed with a Developer ID and notarized. This row records the
truth, and T27-45 owns the fix.

**E5 — the DMG had no drag-to-install target.** The staged volume carried only the
`.app`. **Fixed:** the recipe now also stages the standard `/Applications` symlink, so
the user drags the bundle onto the folder.

## What this audit does not close

- **Neither slice has ever been built.** The audit is a reading of the recipe; the
  receipt is a `lipo -info` on a real bundle, which needs a macOS host.
- **The bundle's behaviour on Apple Silicon is untested** — that is T27-44's job, and it
  cannot be inferred from the recipe.
- **The icon and the notarization are other rows' work**, recorded here so the campaign
  does not mistake a documented gap for a forgotten one.
