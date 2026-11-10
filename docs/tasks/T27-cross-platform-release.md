# T27 — Cross-platform release engineering and certification

> **Status:** `IN PROGRESS` — implementation is ahead of verification. No
> release target is `CERTIFIED`; nothing is signed, so no artifact can
> reach evidence-ladder rung 9.  
> **Priority:** P0 release infrastructure  
> **Plan:** [RELEASE-ENGINEERING-PLAN.md](../RELEASE-ENGINEERING-PLAN.md)  
> **Scope authority:** [decision 0075](../decisions/0075-release-scope-mvp.md)  
> **Evidence baseline:** [PHASE-0-AUDIT.md](../release/PHASE-0-AUDIT.md) (PR #16 merged; dated 2026-11-09 historical baseline)

## Objective

Deliver a complete, verifiable FireCraft release for the explicitly
supported targets, with the release-scope cut decided by the maintainer
rather than inferred from a plan.

The **scope** of the release — which platforms and artifacts constitute
an acceptable cut — is governed by
[decision 0075](../decisions/0075-release-scope-mvp.md). This ticket
governs the **work**; it does not decide the scope.

The supported-target set this work order covers is: Windows, macOS,
Linux (AppImage, DEB, Flatpak, and RPM if adopted), web, CLI, and
source archives. Android is a gated feasibility spike followed by a
native port only if the current Rust/egui application can be supported
without pretending a web wrapper is a native app.

## Inputs and authority

- [decision 0075 — release scope](../decisions/0075-release-scope-mvp.md)
  — **the release-scope authority.** It records the MVP candidate and
  the extended campaign scope, and its approval state. As of 2026-11-09
  the approved scope is **unset**: the maintainer has not approved an
  MVP cut, so the plan's full target matrix remains the standing scope.
- [RELEASE-ENGINEERING-PLAN.md](../RELEASE-ENGINEERING-PLAN.md) — the
  **work order**: RP-00 through RP-08, the target matrix, and per-target
  acceptance criteria. Its §1.1 "MVP v1 candidate" is a *recommendation*
  and is not scope authority on its own.
- [PHASE-0-AUDIT.md](../release/PHASE-0-AUDIT.md) — the **evidence
  baseline**: what exists as of 2026-11-09, with receipts, the blocker
  register B-1…B-21, the documented-vs-implementation findings F-1…F-17,
  and the unverified-claims list. Where a document and the audit
  disagree about what exists, the audit wins.
- [ARTIFACT-MATRIX.md](../release/ARTIFACT-MATRIX.md) — per-artifact
  evidence state, one row per artifact.
- [Releasing FireCraft](../RELEASING.md) — the operational release
  procedure.
- [Toolchains](../TOOLCHAINS.md) — recipes and verification boundaries.
- [HANDOFF.md](../HANDOFF.md) — project operating rules and deploy doors.
- [PRODUCTION-READINESS-PLAN.md](../PRODUCTION-READINESS-PLAN.md) — application cleanup, WASM optimization, UI QA, agent roles, and integration sequence.
- [TASKS.md](../TASKS.md) — live task status.
- `docs/release/EXECUTION-BOARD.md` — the per-task execution board.
  **It is not present on this branch**; it arrives with PR #12
  (`docs/release-execution-board`). This ticket does not duplicate it —
  the RP table below is the work-package view, and the board is the
  atomic-task view when it lands.

## Status vocabulary

Every row below uses the campaign's status words, and they are not
interchangeable labels (campaign §3.2):

`NOT STARTED` · `READY` · `IN PROGRESS` · `BLOCKED` · `IMPLEMENTED` ·
`BUILD VERIFIED` · `INSTALLED` · `LAUNCHED` · `SMOKE TESTED` ·
`PUBLISHED` · `CERTIFIED` · `DEFERRED` · `REJECTED`

**The rule this ticket enforces:** a row may only claim `INSTALLED`,
`LAUNCHED`, `SMOKE TESTED`, `PUBLISHED` or `CERTIFIED` when it carries a
receipt naming what was observed, on what host, and on what date. A row
with no receipt is `IMPLEMENTED` (the code or recipe exists and the
change is in the tree) or `BLOCKED` (the work cannot advance here, and
the blocker is named). **Nothing in this ticket claims `PUBLISHED` or
`CERTIFIED` without a receipt.**

## Implementation versus verification

The campaign's central principle: *"FireCraft's release readiness must
be demonstrated by actual artifacts and reproducible target-platform
evidence, not inferred from implementation progress."* The table below
separates the two for every work package.

| RP | Scope | Implementation | Verification | Owner | Next action |
|---|---|---|---|---|---|
| RP-00 | Inventory and artifact contract | `IMPLEMENTED` — [ARTIFACT-MATRIX.md](../release/ARTIFACT-MATRIX.md) rewritten 2026-11-09 against the Phase 0 audit, one row per artifact with a status word and a receipt | `IMPLEMENTED` — the matrix itself is the receipt; the audit's 19 artifacts are each re-derived from observed evidence | docs/release lane | keep the matrix in step with the audit on every artifact change |
| RP-01 | Linux package completion | `IMPLEMENTED` — `xtask dist --native` builds desktop tar.gz, CLI tar.gz and DEB, manifest-checks all three, reads the DEB back with `dpkg-deb`, and fail-closes on the declared libc6 floor against the binaries' real requirement (`objdump`) | `BUILD VERIFIED` (native set, rung 5) — audit P0.4.3; the DEB reached rung 11 as a published asset. The AppImage and Flatpak recipes are `NOT STARTED` (rung 1): `linuxdeploy` absent (B-8), `flatpak` absent (B-11) | release-workflow lane | AppImage/Flatpak need a host with their SDKs, or the recipes stay recipe-only and the scope decision follows |
| RP-02 | Windows packaging and verification | `IMPLEMENTED` — `packaging/build-windows.ps1` + `packaging/windows/firecraft.iss` exist; the PowerShell and Inno recipes are **existence-checked only** (the gate cannot parse them on this fleet) | `NOT STARTED` (rung 1) — `pwsh` absent and no MSVC linker here (B-12). The GNU cross-build and wine smoke recorded on the PR #9 branch are preview evidence, not native Windows certification (campaign §8) | release-workflow lane | needs a Windows host with MSVC and Inno Setup; until then Windows is recipe-only |
| RP-03 | macOS packaging and notarization | `IMPLEMENTED` — `packaging/build-macos-dmg.sh` exists and is `bash -n` parse-checked by the gate | `NOT STARTED` (rung 1) — needs macOS + Xcode CLT; the recipe is off-fleet by design | release-workflow lane | needs a macOS host with `lipo`/`hdiutil` and a Developer ID identity |
| RP-04 | Android feasibility spike and native port | `IMPLEMENTED` — [android-feasibility.md](../research/android-feasibility.md) concludes "feasible with bounded work" | `DEFERRED` — the gate decision is unrecorded; no NDK/SDK CI image, no native entrypoint, no ARM64 device | research lane | record an explicit feasibility-gate decision before any Android work is scheduled |
| RP-05 | Source, CLI, and web release assets | `IMPLEMENTED` — `xtask dist --source` and `xtask dist --sbom` are present on main and wired into `release.yml` | `BLOCKED` — generators exist, but no successful post-repair tag run has published and independently verified their outputs. The v0.2.5 web ZIP is historical and production deployment reconciliation remains open (B-6/B-21) | release-workflow lane | run a disposable tag end to end; clean-build the downloaded source archive, schema-validate the SBOM, and reconcile production Pages hashes |
| RP-06 | Supply-chain metadata and signing | `IMPLEMENTED` — checksum generation, fail-closed aggregation, and `packaging/release-assets.sh` are on main | `BLOCKED` — canonical machine-readable release manifest, schema-valid published SBOM, provenance/attestation, and signing verification are still absent | release-workflow lane | validate SBOM; fix release staging edge cases; add manifest and provenance/signing; independently verify downloads |
| RP-07 | Release orchestration and publication | `IMPLEMENTED` — `release.yml` builds native + web + source + SBOM, validates the aggregate fail-closed, and creates the draft release with positional assets (`packaging/release-assets.sh create`) | `NOT STARTED` — **the workflow has never produced a release.** All five recorded runs failed (B-4); all six published releases were operator-published by hand, and the shipped v0.2.5 web bundle is not the bundle the workflow built (B-21, F-16) | release-workflow lane | run the repaired workflow end to end on a disposable tag and attach the receipt to this ticket |
| RP-08 | Final release certification | `NOT STARTED` | `NOT STARTED` — no artifact is `CERTIFIED`; no certification report exists | maintainer | blocked on RP-05…RP-07 evidence and on the scope approval in decision 0075 |

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

The MVP candidate lane inside that scope is defined — and *only* defined
— by [decision 0075](../decisions/0075-release-scope-mvp.md).

## Non-goals

- Do not claim every optional ecosystem format is supported merely because a manifest or recipe exists.
- Do not check compiled binaries, signing keys, keystores, provisioning profiles, or CI secrets into Git.
- Do not publish a native Android claim for a web wrapper.
- Do not bypass CI, security, license, package-install, or runtime gates to meet a date.
- Do not treat a recommendation as an approved scope.

## Acceptance criteria

- One approved artifact matrix labels each target required, optional,
  experimental, or unsupported — and the matrix says which of those
  labels is *approved* and which is *recommended*.
- Every required artifact is built from the exact release tag SHA with
  locked dependencies and pinned toolchains.
- Every required native package is installed/launched and functionally
  smoke-tested on its target OS.
- Android is either verified as a real native app on an ARM64 device or
  explicitly deferred with a feasibility report.
- Source archives extract cleanly and build with documented prerequisites.
- Every published asset has a verified SHA-256 and release-manifest entry.
- SBOM and provenance are published; signatures are verified where
  signing credentials are available.
- Release aggregation fails closed on missing, mislabeled, mismatched,
  or corrupt assets.
- Public web deployment hashes match the exact release bundle.
- Release notes and verification report state exact platform/architecture
  support and residual limitations.

## Verification

Use the exact commands and target-native procedures defined in
[RELEASE-ENGINEERING-PLAN.md](../RELEASE-ENGINEERING-PLAN.md). Every PR
must include build receipts, install/launch evidence, artifact
names/sizes/digests, CI run links, and unverified gaps.

A CI run id and a date are part of a receipt. A bare statement that a
gate was green, without the run that proves it, is not a receipt.

## Known blockers (as of 2026-11-09)

The full register with evidence is in the audit's §4. The blockers that
gate this ticket:

| ID | Blocker | Effect on this ticket |
|---|---|---|
| B-3 | PR #9 and PR #14 duplicated the release plan and `dist --source` / `dist --sbom` implementation | **Resolved on main:** the plan and both commands are present; RP-05 remains blocked on end-to-end tag evidence, not command consolidation |
| B-4 | the release workflow could not create a release (three defects; all five recorded runs failed) | repaired on PR #14, but unproven end to end — RP-07 stays `NOT STARTED` |
| B-6 | the live deployment serves a bundle that corresponds to no identifiable release, and its own checksum list cannot verify it | RP-05's "verified production web deployment" is `BLOCKED` |
| B-7 | the DEB's declared glibc floor is below the binaries' real requirement, confirmed on the **published** artifact | the DEB row stays an open defect, not a settled fact; the floor numbers are owned by the stacked glibc-floor PR |
| B-13 | the source archive and the SBOM have never been published | RP-05 cannot be `PUBLISHED` |
| B-14 | no provenance, no release manifest, no signing identity | nothing can be `CERTIFIED` |
| B-19 | the `time` 0.3.47 / MSRV 1.95 update was pending in the 2026-11-09 audit baseline | **Resolved on main**; new artifacts must still be rebuilt from the current tag SHA rather than re-tagged from v0.2.5 |
| B-21 | the published v0.2.5 web bundle has no build receipt and the release was published out of band | the published set is not attested by anything in this repository |
| B-22 | the GitForge runner's failure path arms its container reap and backend-health calls with no timeout, so a wedged Docker daemon wedges the runner instead of failing the job — a defect in the **GitForge** repository (`crates/gitforge-runner/src/executor.rs`), tracked as FireCraft issue #37 | every GitForge-verified row needs a terminal job state from the runner; a hang yields no terminal state, so no GitForge receipt can be attached until the calls are bounded. The fix is owned by the GitForge repository; FireCraft tracks it as issue #37 and task #202 |

## Stopping condition

Close T27 only when RP-08's certification checklist is complete for
every required target, **or** when the maintainer explicitly approves a
documented reduced support matrix in
[decision 0075](../decisions/0075-release-scope-mvp.md). A green
workflow with skipped targets is not completion.
