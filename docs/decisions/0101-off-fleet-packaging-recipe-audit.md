# 0101 — The off-fleet packaging recipes are audited, and the defects the audit found are fixed in the recipes

Date: 2026-11-12 · Tickets: [T27-35](../tasks/T27-35-windows-build-script-audit.md),
[T27-42](../tasks/T27-42-macos-architectures.md) · Status: accepted

## Context

The release campaign has two packaging recipes that this fleet cannot run:
`packaging/build-windows.ps1` (portable ZIP plus an optional Inno Setup installer) and
`packaging/build-macos-dmg.sh` (a universal `.app` in a DMG). There is no Windows host
and no macOS host, so neither recipe has ever been executed, and the artifact matrix
records both platforms as "recipe only".

T27-35 and T27-42 asked for exactly what is possible without those hosts: a written
audit of the two recipes — their environment assumptions, their product identity, their
shortcuts, their version metadata — with a per-assumption risk list. The audit is the
deliverable; a build is not.

Reading the recipes closely showed that they were not merely unbuilt. All four of the
defects the audit found would have stopped the artifacts they exist to produce, or
shipped one that was legally incomplete.

## Decision

- **The audit is the deliverable, and it lives with its ticket.** T27-35 and T27-42
  carry the assumptions, the risks, the identity tables and the defects. Nothing in
  either row claims a build.
- **The Windows installer is compiled through the recipe, never on its own.** The `.iss`
  carries two stand-ins — `__APPVERSION__` and `__LICENSEFILE__` — that the PowerShell
  recipe substitutes. Compiling the committed `.iss` directly now fails closed on the
  unresolved stand-in rather than producing an installer with a placeholder version.
- **The license path is substituted as an absolute path.** The staged `.iss` lives in
  the stage directory, so a relative `LicenseFile` resolves against the stage and not
  against the repository root. The committed `..\..\LICENSE` pointed at a file that
  exists in the repository and nowhere else, so the only path the recipe actually
  uses would have failed to compile the installer.
- **Both substitutions go through a `MatchEvaluator`.** The `-replace` replacement
  syntax reads `$` as a capture-group reference; a Windows profile path can contain
  one.
- **`NOTICE` travels with every distribution.** Apache-2.0 §4(d) requires the
  attribution notices to travel with the distribution when the work has a `NOTICE` file,
  and this repository's `NOTICE` carries the Adobe trademark disclaimer. The Windows
  tree and the macOS bundle now stage it beside `LICENSE`.
- **The macOS recipe fails closed on an unreadable version.** `readonly VAR="$(...)"`
  returns success whatever the substitution did, so `set -e` never fired and an empty
  version produced a `firecraft--macos-universal.dmg` with an empty `Info.plist`
  version.
- **The macOS artifact is one universal2 DMG.** `lipo` already combines both slices;
  the decision is recorded so the release asset list has one macOS entry rather than
  two, and so `lipo -info` on the staged bundle is named as the receipt.
- **The missing icons are recorded, not faked.** Neither platform ships an icon today
  (`SetupIconFile` and `CFBundleIconFile` are unset) because the repository has no
  `.ico` and no `.icns`. The brand refresh (issue #131) owns the artwork; an icon
  derived from today's mark would be stale the moment it lands.

## Evidence

Four defects, all found by reading rather than by running:

1. **The installer could not have compiled.** `LicenseFile=..\..\LICENSE` next to a
   staged script in `dist\windows-stage\` resolves to `dist\LICENSE`, which nothing
   creates.
2. **The distribution carried `LICENSE` but not `NOTICE`.**
3. **The macOS version read failed silently** because `readonly` masks the exit status
   of a command substitution.
4. **The Windows recipe moved a file the compiler never produced.** The installer's
   name comes from the `.iss`'s `OutputBaseFilename`, which carries the platform as well as
   the version; the recipe moved a name without it, so it failed after a successful
   compile.

The fleet can prove none of these by execution — there is no `pwsh` and no `ISCC` here,
and no macOS host — which is why the audit is the deliverable and why the fixes are
recorded in the recipes rather than in a procedure around them.

## Consequences

- The Windows and macOS recipes are closer to buildable than they were, and none of the
  four defects can reach an artifact.
- The `.iss` is no longer compilable by itself. That is deliberate: the recipe is the
  supported path, because the recipe is what substitutes the version and stages the
  license. T27-37's verification command now says so.
- The MSVC runtime assumption (audit assumption A5) stays open, and T27-36 now names
  the two honest answers — install the redistributable on the smoke VM, or build with
  `+crt-static` — instead of leaving the operator to discover it as a missing DLL.
- The icon gap is now explicit on both platforms and owned by the brand work, so it is
  not rediscovered as a mystery in T27-37 or T27-43.
- Nothing here claims a Windows or macOS artifact. The build rows (T27-36, T27-37,
  T27-38, T27-43, T27-44) are open, and only they can close the platforms.

See also decision 0091 (the DEB declares the libraries the desktop binary loads) — the
same shape of finding on the one platform the fleet can test.
