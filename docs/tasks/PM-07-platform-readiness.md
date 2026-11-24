# PM-07 — Per-target platform readiness baseline

> **Parent workstream:** PLATFORM-01 (POST-MVP-ROADMAP §5.18).
> **Priority:** P1 — support claims must not outrun receipts.
> **Status:** `NOT STARTED` — gated on Gate 0; partially blocked external hosts.
> **Owner:** unassigned. **Created:** 2026-11-23.
> **Dependencies:** Gate 0 integration; PR #181 (wasm CDN deploy) merge.

## 1. User problem

The repo mixes recipes with artifacts. A reader cannot tell which target has been
built, installed, launched, smoke-tested, published, or certified for which scope.
Nothing is `CERTIFIED`, and Windows/macOS/AppImage/Flatpak artifacts have never been
produced.

## 2. Current implementation and evidence

- Linux x86_64 tar.gz, `.deb`, CLI archive and web bundle are built by
  `xtask dist`; the web bundle is deployed via a Cloudflare Pages HTML/JS shell plus
  a WASM proxy Worker (PRODUCTION-TODO 2026-11-21).
- Windows/macOS/AppImage/Flatpak recipes exist and are parse-checked only;
  `linuxdeploy`, `flatpak`, MSVC/Inno, and macOS tooling are absent from the fleet
  (TASKS/T23, PRODUCTION-ROADMAP Phase D).
- `docs/release/ARTIFACT-MATRIX.md` records per-artifact state; TASKS states
  **nothing is CERTIFIED** and release scope (decision 0075) is unapproved.
- **UNVERIFIED at this SHA:** the deployed bundle's provenance and the current
  Linux artifact build.

## 3. Scope

In scope: a single readiness table per target (Windows x86_64/ARM64, macOS
arm64/Intel, Linux x86_64/ARM64, browser/WASM, WebOS embedding, Android research)
with columns source-compiles, package-builds, installs, launches, opens/edits/saves,
OS-integration tests, published, certified-for-scope; a receipt for each claimed
cell; a reproducibility recipe per row; and the honest "not started/blocked" states.

**Non-goals:** producing off-fleet artifacts without a host, signing without
identities, or changing release scope.

## 4. Files and surfaces likely affected

`docs/release/ARTIFACT-MATRIX.md`, `docs/RELEASING.md`, `docs/TOOLCHAINS.md`,
`docs/TASKS.md` (after owner approval), `.github/workflows/release.yml`,
`xtask/src/dist.rs`, `packaging/*`.

## 5. Decisions required

- *Reiteration of decision 0075:* which targets are in the approved release scope.
  Until approved, this ticket can only produce a readiness inventory, not change
  claims.

## 6. Implementation steps

1. Re-derive every row from the built artifacts/workflows, not from prose.
2. Add a `dist --readiness --json` output that lists artifacts, recipes, host
   requirements, and last verified SHA.
3. Produce or re-produce the Linux + web artifacts from one tagged commit.
4. Add install/launch/smoke receipts (host, version, date, command, result).
5. Mark blocked rows with the missing tool/host named.
6. Link the matrix from FEATURES/README and the docs index.

## 7. Acceptance criteria

- Every target row has exactly one status word from the release vocabulary.
- No row claims `INSTALLED`/`LAUNCHED`/`SMOKE TESTED`/`PUBLISHED`/`CERTIFIED`
  without a receipt.
- The readiness JSON and the matrix agree.
- A recipe-only target is labelled recipe-only everywhere.

## 8. Test plan

- xtask unit test: readiness JSON schema and status vocabulary.
- A docs check that the matrix's rows match the JSON rows.
- Existing `xtask dist --verify-manifest` and checksum verification.

## 9. Manual verification

Clean Debian 12 container: install `.deb`, launch, open a corpus file, export,
undo, close (record the receipt). Browser: load the served bundle, verify
`version.json`, exercise open/save.

## 10. Performance, security, portability

Readiness JSON must not expose secrets or local paths. Web row must state the CDN
dependency and the 25 MB page limit it works around.

## 11. Risks and rollback

Risk: pressure to overclaim. Mitigation: receipts are mandatory by construction.
Rollback is a docs/data revert.

## 12. Stopping condition and closure evidence

Stop if the release scope is unapproved and a status would change a claim; publish
the inventory only. Close with the matrix, the JSON, and at least one new
install/launch/smoke receipt for a target that lacked one.
