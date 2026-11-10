# 0075 — Release scope: the MVP candidate and the extended campaign

Date: 2026-11-09 · Ticket: [T27](../tasks/T27-cross-platform-release.md) ·
Status: **recommended — awaiting maintainer approval**

## Decision

This record establishes **where release scope lives** and records the two
scopes the cross-platform release campaign distinguishes. It is the
**single authority for release scope** in this repository.

### The authority

**`docs/decisions/0075-release-scope-mvp.md` is the release-scope
authority.** Any document that states, summarizes, or implies what this
repository claims to release — the artifact matrix, the release
instructions, the toolchain inventory, the T27 ticket, the task board —
must point here for the scope and must not restate it as a second source
of truth.

Two other documents carry authority over *different* things and do not
compete with this record:

- `docs/RELEASE-ENGINEERING-PLAN.md` is the **work order**: the RP-00
  through RP-08 packages, the target matrix, and the per-target
  acceptance criteria.
- `docs/release/PHASE-0-AUDIT.md` (PR #16, branch
  `docs/phase-0-baseline-audit`) is the **evidence baseline**: what
  actually exists as of 2026-11-09, with receipts, blockers B-1…B-21,
  and documented-vs-implementation findings F-1…F-17.

Where this record, the plan, and the audit disagree about *what exists*,
the audit wins; the audit is dated and evidence-backed. Where they
disagree about *what should be released*, this record wins, because it
is the scope authority — but only once the maintainer has approved it.

### Approval state

**The maintainer has not approved an MVP scope.** The campaign mandate
is explicit on this point: *"Record the maintainer's approved scope. Do
not treat a recommendation as approval."*

Therefore:

- The scope below is a **recommendation**.
- **The approved scope of this repository is currently unset.** No
  release cut is authorized against a scope that has not been approved.
- Until an approval is recorded here, the **full cross-platform target
  matrix in `docs/RELEASE-ENGINEERING-PLAN.md` §3 remains the standing
  scope**, and the MVP candidate below is a proposal to narrow it.
- The approval is recorded by changing the *Status* line of this record
  to `accepted` and naming the approver and date. Nothing else in the
  repository implies approval; a plan, a ticket, a matrix, or a plan's
  own "recommended" wording is not approval.

### MVP candidate scope (recommended, not approved)

| # | Item | Campaign mandate wording | Candidate status today (audit 2026-11-09) |
|---|---|---|---|
| 1 | Web / WASM bundle | Web/WASM | `PUBLISHED` — rung 11 for the published ZIP; the deployed bundle is unreconciled (B-6) and the published bytes were never smoke-tested (F-16) |
| 2 | Linux x86_64 desktop archive | Linux x86_64 desktop archive | `PUBLISHED` — rung 11 |
| 3 | Linux x86_64 DEB | Linux x86_64 DEB | `PUBLISHED` — rung 11, but the declared glibc floor is under the binaries' real requirement (B-7, open) |
| 4 | Linux x86_64 CLI archive | Linux x86_64 CLI | `PUBLISHED` — rung 11 |
| 5 | Source archive | Source archive | `BUILD VERIFIED` on the PR #12 tree; the verb is absent from `main` (F-9) and it has never been published (B-13) |
| 6 | Checksums | Checksums | `PUBLISHED` — per-set `MANIFEST.sha256` only; the release-wide `RELEASE_SHA256.txt` has never been published (B-17) |
| 7 | SBOM | SBOM | `BUILD VERIFIED` on the PR #12 tree; absent from `main` (F-9), never published (B-13) |
| 8 | Verified production web deployment | Verified production web deployment | `BLOCKED` — the live bundle corresponds to no identifiable release (B-6) |
| 9 | Release certification | Release certification | `NOT STARTED` — nothing is signed and no provenance exists, so no artifact can reach rung 9 (B-14) |

The candidate also carries the campaign's own gate conditions: the PR
remediation merged, the canonical gate green, the packaged artifacts
verified, and the deployed web hashes matching the tagged build before
publishing.

### Extended campaign scope (recommended, not approved)

These are **not** part of the MVP candidate and are not to be advertised
as supported production packages until their own target-specific
acceptance evidence exists.

| # | Item | Campaign mandate wording | Candidate status today | Blocking condition |
|---|---|---|---|---|
| 10 | Linux AppImage | Linux AppImage | `NOT STARTED` — rung 1 | `linuxdeploy` absent (B-8); no pinned copy vendored |
| 11 | Linux Flatpak | Linux Flatpak | `NOT STARTED` — rung 1 | `flatpak` / `flatpak-builder` absent (B-11); no recipe on `main` |
| 12 | Windows | Windows | `NOT STARTED` — rung 1 | `pwsh` absent and no MSVC linker on this fleet (B-12); a Windows host is required |
| 13 | macOS | macOS | `NOT STARTED` — rung 1 | needs macOS + Xcode CLT; the recipe is off-fleet by design |
| 14 | Android | Android | `DEFERRED` — feasibility report only | the feasibility gate is unrecorded; no NDK/SDK image, no ARM64 device |
| 15 | ARM64 assessments | ARM64 assessments | `DEFERRED` | no ARM64 host and no ARM64 test environment |
| 16 | RPM | RPM only if formally reinstated | `DEFERRED` | [decision 0070](0070-rpm-deferred.md) — **not reopened by this record** |

### RPM

RPM stays **deferred** under
[decision 0070](0070-rpm-deferred.md) (accepted 2026-11-06). This
record does not reinstate it, does not schedule it, and does not treat
its absence from the MVP candidate as an omission to be filled. RPM
returns to the matrix only if it is **formally reinstated** — a new
decision record that supersedes 0070, with its own approval.

## Why

The campaign (§P1.3) requires that the full cross-platform matrix and
the proposed MVP v1 "must not silently conflict". Two documents each
described the target set in their own words — the plan's §1.1 "MVP v1
candidate" and the campaign's own lists — with no record of whether
anyone had approved the narrower cut. A reader could reasonably have
taken "MVP v1 candidate" as the shipping scope, and the plan's own text
says so in the passive voice: "This is a recommended scope cut for
maintainer approval, not an automatic change to the long-term
cross-platform target matrix."

That gap is the defect this record closes. It also gives the release
documents a single place to point, so the scope cannot drift into a
second copy.

## Consequences

- `docs/release/ARTIFACT-MATRIX.md`, `docs/RELEASING.md`,
  `docs/TOOLCHAINS.md`, `docs/tasks/T27-cross-platform-release.md` and
  `docs/TASKS.md` cite this record as the release-scope authority and
  describe evidence states, not scope.
- `docs/RELEASE-ENGINEERING-PLAN.md` §1.1 keeps its "recommended"
  wording; the plan's own §1.1 already states that it is a
  recommendation, and this record confirms it.
- An MVP cut, if approved, does not delete the extended campaign. It
  narrows what a *release* must contain; the extended items stay in
  the matrix as follow-on targets with their own evidence gates.
- The T27 stopping condition — "close T27 only when RP-08's
  certification checklist is complete for every required target, or when
  the maintainer explicitly approves a documented reduced support
  matrix" — can be satisfied by an approval recorded here, and by
  nothing else. A green workflow with skipped targets does not satisfy
  it.
- The Phase 0 audit's §5 finding that the MVP scope "is a
  recommendation, not an approved scope" is carried forward unchanged;
  this record gives that statement an address.
