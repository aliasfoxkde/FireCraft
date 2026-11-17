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

## Re-validation (2026-11-16)

[T27-31](../tasks/T27-31-rpm-decision.md) re-checks this deferral against
current demand. The demand has not moved:

- The repository's issues carry **no request for an RPM package**. The only
  issues that mention RPM are the release ledger's own rows (T27-31 itself
  and the neighbouring T27-3x tickets).
- Fedora-family users are served by the tar.gz, and T27-33's "second distro
  family" check is a DEB/tar.gz exercise, not a packaging-format exercise.
- The reasons in "Why" are unchanged: the DEB covers the dpkg-family share,
  and an RPM shipped without a clean Fedora install/launch receipt would be
  a false support claim.

**The deferral is retained.** The matrix's RPM row continues to record
`DEFERRED` against this decision, and RP-08 certification must still not
list RPM as a skipped-required target — it is deferred, not required and
skipped. The trigger for revisiting is unchanged: a release-consumption
survey showing real Fedora demand.
