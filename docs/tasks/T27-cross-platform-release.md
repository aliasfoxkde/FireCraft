# T27 — Cross-platform release engineering and certification

> **Status:** Planned / execution not yet started  
> **Priority:** P0 release infrastructure  
> **Plan:** [RELEASE-ENGINEERING-PLAN.md](../RELEASE-ENGINEERING-PLAN.md)

## Objective

Deliver a complete, verifiable FireCraft release for all explicitly supported targets: Windows, macOS, Linux (AppImage, DEB, Flatpak, and RPM if adopted), web, CLI, and source archives. Android is a gated feasibility spike followed by a native port only if the current Rust/egui application can be supported without pretending a web wrapper is a native app.

## Inputs and authority

- [Cross-platform release engineering plan](../RELEASE-ENGINEERING-PLAN.md) — authoritative work packages, target matrix, acceptance criteria, and agent protocol.
- [Releasing FireCraft](../RELEASING.md) — current release commands and tag procedure.
- [Toolchains](../TOOLCHAINS.md) — current packaging recipes and verification boundaries.
- [HANDOFF.md](../HANDOFF.md) — project operating rules and deploy doors.
- [TASKS.md](../TASKS.md) — live task status.

The plan's baseline is `main` at `5fac7a2267ebc6b61432d5302ab6b02a1ff15c0f`, version `0.2.5`. Re-inspect current main before implementation; do not assume this baseline remains current.

## Scope

- RP-00 inventory and artifact contract.
- RP-01 Linux AppImage, DEB, Flatpak, RPM decision/build, tarball and distro tests.
- RP-02 Windows portable ZIP, installer, signing and native VM tests.
- RP-03 macOS DMG, universal/split architecture, notarization and native tests.
- RP-04 Android feasibility spike; native APK/AAB only after feasibility approval.
- RP-05 web, CLI, source archives and production deployment reconciliation.
- RP-06 checksums, manifest, SBOM, provenance, signatures and workflow hardening.
- RP-07 tag-driven build/aggregate/publish pipeline.
- RP-08 final independent release certification.

## Non-goals

- Do not claim every optional ecosystem format is supported merely because a manifest or recipe exists.
- Do not check compiled binaries, signing keys, keystores, provisioning profiles, or CI secrets into Git.
- Do not publish a native Android claim for a web wrapper.
- Do not bypass CI, security, license, package-install, or runtime gates to meet a date.

## Acceptance criteria

- One approved artifact matrix labels each target required, optional, experimental, or unsupported.
- Every required artifact is built from the exact release tag SHA with locked dependencies and pinned toolchains.
- Every required native package is installed/launched and functionally smoke-tested on its target OS.
- Android is either verified as a real native app on an ARM64 device or explicitly deferred with a feasibility report.
- Source archives extract cleanly and build with documented prerequisites.
- Every published asset has a verified SHA-256 and release-manifest entry.
- SBOM and provenance are published; signatures are verified where signing credentials are available.
- Release aggregation fails closed on missing, mislabeled, mismatched, or corrupt assets.
- Public web deployment hashes match the exact release bundle.
- Release notes and verification report state exact platform/architecture support and residual limitations.

## Verification

Use the exact commands and target-native procedures defined in [RELEASE-ENGINEERING-PLAN.md](../RELEASE-ENGINEERING-PLAN.md). Every PR must include build receipts, install/launch evidence, artifact names/sizes/digests, CI run links, and unverified gaps.

## Stopping condition

Close T27 only when RP-08's certification checklist is complete for every required target, or when the maintainer explicitly approves a documented reduced support matrix. A green workflow with skipped targets is not completion.
