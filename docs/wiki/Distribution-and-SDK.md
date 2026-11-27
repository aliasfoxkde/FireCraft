# Distribution and SDK

FireCraft has two complementary purposes:

1. A complete, independent graphics editor people can install and use without another product.
2. A reusable creative/automation component that other applications and operating environments can package, embed or orchestrate.

Prebuilt artifacts serve both purposes. Embedding in ArtCraft WebOS is one use case, not the reason FireCraft has a release matrix.

## Standalone product distribution

The latest published release is v0.2.5. Its verified published set includes Linux x86_64 desktop and CLI archives, a Debian package and the WebAssembly/web bundle. Other packaging recipes are not stable binaries until built, installed, launched and functionally tested on the target platform.

The long-term target matrix includes Windows (x64/ARM64 subject to native verification), macOS (universal or architecture-specific DMG), Linux DEB/RPM/AppImage/Flatpak/Arch/tar.gz, BSD, browser/PWA, CLI/MCP, source and SDK distributions. Android and other mobile targets remain native-feasibility workstreams. Every asset needs a build receipt and target-native verification; a recipe or cross-compile is not a support claim.

## Developer and embedding distribution

The planned FireCraft SDK should expose a stable facade over the existing command registry and document engine:

- Rust API for native applications and plugins.
- Versioned language-neutral schemas for app identity, commands, parameters, results, capabilities, errors and lifecycle.
- WASM module plus browser host bridge/client for web products.
- Native CLI and MCP adapters that invoke the same command service.
- Examples, API documentation, compatibility manifest, licenses, checksums, SBOM and provenance for released SDK components.

The SDK must remain usable by third-party hosts. WebOS is a primary integration consumer and conformance target, not a runtime dependency. FireCraft's document model, Fireworks compatibility rules, editor UX and product policy remain owned by FireCraft.

## Shared ArtCraft foundation

Evaluate the existing ecosystem rather than recreating its useful mechanisms:

- [craft-libs](https://github.com/storytold/craft-libs) — Rust libraries and the candidate extraction process.
- [craft-common-actions](https://github.com/storytold/craft-common-actions) — reusable CI, packaging lint and cross-platform release automation.
- [artcraft-candidates](https://github.com/artcraft-candidates) — community proving ground for testing compatibility without transferring ownership of the canonical FireCraft repository.

Adoption is selective: confirm target compatibility, license, feature footprint and real benefit; keep app-specific checks; contribute reusable fixes upstream. See [RELEASE-ENGINEERING-PLAN.md §3.7, §5.3 and RP-09](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/RELEASE-ENGINEERING-PLAN.md) and [WEBOS-PLATFORM-INTEGRATION.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/WEBOS-PLATFORM-INTEGRATION.md).

## Release evidence and current limits

- [Release plan and artifact matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/RELEASE-ENGINEERING-PLAN.md)
- [Current artifact receipts](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/release/ARTIFACT-MATRIX.md)
- [Release checklist](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/RELEASING.md)
- [Toolchain and packaging recipes](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TOOLCHAINS.md)

Support status is per artifact: Stable, Preview/Community or Planned/Unsupported. Never infer Windows/macOS/Flatpak/Android support from a script alone.


---

## 1.0 distribution target and evidence policy

The 1.0 distribution roadmap targets a ready-to-use browser/WASM bundle, Windows installer and portable ZIP, macOS universal DMG, Linux AppImage/DEB/Flatpak/tar.gz and a formally decided RPM path, plus CLI and source/integrity artifacts. Linux ARM64 and Windows ARM64 are additional architecture targets to assess. These are **targets**, not claims that every format is presently available.

Use the [GitHub Releases page](https://github.com/aliasfoxkde/FireCraft/releases) as the current source for downloadable artifacts. A package is listed as supported only after it has been built from a tagged source SHA, package-inspected, installed/launched and smoke-tested on the target OS/architecture. A packaging recipe or cross-compile alone is insufficient.

For the complete 1.0 matrix, install/upgrade/uninstall checks, checksums/SBOM/provenance, release process and decisions, see [the End-to-End 1.0 Execution Plan](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/planning/FIRECRAFT-1.0-END-TO-END-EXECUTION-AND-RELEASE-CLOSEOUT.md) and [the Release Engineering Plan](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/RELEASE-ENGINEERING-PLAN.md).

The static WASM download and hosted browser app are separate from the native desktop installers. Hosting/embed instructions remain in [Browser and WebOS](Browser-and-WebOS). The marketing site and WASM application are also separate Cloudflare Pages deployments.
