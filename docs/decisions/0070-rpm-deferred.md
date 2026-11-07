# 0070 — RPM deferred: unsupported in the T27 release matrix

Date: 2026-11-06 · Ticket: [T27](../tasks/T27-cross-platform-release.md) · Status: accepted

## Decision

RPM is **not supported** in the 0.2.x release matrix. Fedora-family users get
the tar.gz (`firecraft-{v}-linux-x86_64.tar.gz`) or the AppImage. Revisit if a
release-consumption survey shows real Fedora demand.

## Why

- The DEB already covers the dpkg-family desktop share; a second native
  package format doubles the metadata/maintenance surface (spec file,
  version-convention divergence, changelog discipline, distro gating rules)
  for a target nobody has asked for.
- The release pipeline's fail-closed aggregate (RP-07) must enumerate every
  required artifact; each added format multiplies the matrix that CI must
  build, verify, and certify on a fleet that cannot run distro install tests.
- The no-untested-claims rule (`docs/TOOLCHAINS.md`) means an RPM shipped
  without a clean Fedora install/launch receipt would be a false support
  claim — worse than an honest tar.gz.

## Consequences

- `docs/release/ARTIFACT-MATRIX.md` records RPM as "pending decision →
  unsupported (0070)".
- RP-01's Linux scope is: tar.gz, DEB, AppImage, Flatpak — x86_64 first.
- The T27 stopping condition ("maintainer approves a documented reduced
  support matrix") is satisfied for RPM by this record; RP-08 certification
  must not list RPM as a skipped-required target.
