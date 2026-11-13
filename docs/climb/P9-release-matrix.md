# P9 — Release matrix: every in-scope artifact, built and proven

**Goal:** every artifact in the v1 scope (decision 0075, after C1.06) is:
- built by CI from a tag;
- installed and launched on a clean target;
- smoke-tested, with a receipt;
- checksummed;
- covered by the SBOM and an attested provenance.

**Exit check:**
- every in-scope row of `docs/release/ARTIFACT-MATRIX.md` is `SMOKE TESTED`
  or better;
- every out-of-scope row is `DEFERRED` with a decision;
- S10 = the in-scope count.

**Lane:** release. **Gated on C1.06.** Until the scope verdict lands, only
C9.01–C9.03 may start.

**Key files:**
- `.github/workflows/release.yml`: today a single `build` job on
  `ubuntu-24.04` plus a `release` job.
- `packaging/`: `build-appimage.sh`, `build-flatpak.sh`,
  `build-windows.ps1`, `windows/firecraft.iss`, `build-macos-dmg.sh`,
  `deb/`, `desktop/`, `webos/`.
- `xtask/src/dist.rs`: `--native`, `--web`, `--source`, `--sbom`,
  `--manifest` and `--verify-manifest`.
- `docs/release/ARTIFACT-MATRIX.md`.
- `docs/RELEASE-CHECKLIST.md`, if it exists.

**Key idea:** the "off-fleet" Windows and macOS blocker no longer applies.
GitHub-hosted `windows-latest` and `macos-14` runners build and smoke-test
those artifacts, so no maintainer hardware is needed (AUDIT §8).

**Receipt rule:** a status moves forward only when its receipt is committed
under `docs/release/receipts/<artifact>-<version>.md`. Each receipt holds:
- the runner or OS image and version;
- the exact commands;
- the SHA-256 of the artifact tested;
- the output excerpts;
- a screenshot of the launched app, for GUI artifacts.

The status vocabulary is the one in `docs/TASKS.md`. Do not invent statuses.

---

### C9.01 — Bring ARTIFACT-MATRIX up to date `[ ]`
- **Needs:** — · **Tags:** `[DOCS]` · **Size:** XS
- **Steps:**
  1. Rows 12 (source), 13 (SBOM) and 16 (release manifest) say "not on
     `main`", but `release.yml` on `main` now builds and uploads all three
     (`dist --source`, `--sbom`, `--manifest`).
  2. Change their "What exists" cells. Cite `release.yml` line numbers
     (`grep -n "dist --" .github/workflows/release.yml`).
  3. Recount the status summary table.
- **Done when:** every row matches `main`.
- **Verify:** compare each row with `release.yml`.
- **Moves:** —

### C9.02 — Disposable-tag release rehearsal (#99, #33) `[ ]`
- **Needs:** C0.12 · **Tags:** `[HOST:linux]` `[MAINTAINER]` (pushes a tag) ·
  **Size:** S
- **Steps:**
  1. Ask the maintainer to push a tag `v0.0.0-rehearsal.N` to a fork, or use
     `workflow_dispatch` if `release.yml` allows it.
  2. Confirm that the run produces every current asset, that
     `dist --verify-manifest` passes, and that the draft release is
     created, never published.
  3. Record the run URL in a receipt, then delete the draft release and the
     tag. The maintainer does the deletion.
- **Done when:** the receipt is committed.
- **Verify:** the receipt.
- **Moves:** proves the pipeline before any scope is added.

### C9.03 — Release permissions: draft, then publish (#110) `[ ]`
- **Needs:** C9.02 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. In `release.yml`, the `release` job creates a **draft** with the least
     `permissions:` it needs (`contents: write` only on that job).
  2. Publishing is a separate manual `workflow_dispatch` that a maintainer
     runs. Alternatively the maintainer publishes from the GitHub UI;
     document whichever is chosen in `docs/RELEASE-CHECKLIST.md`.
- **Done when:** a rehearsal leaves a draft, and no job can publish on its
  own.
- **Verify:** the C9.02 rerun receipt.
- **Moves:** close #110.

### C9.04 — Attested build provenance `[ ]`
- **Needs:** C9.03, C1.06 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Add `actions/attest-build-provenance`, pinned by SHA like the other
     actions in the file, for every uploaded asset. Grant `id-token: write`
     and `attestations: write` on that job only.
  2. Document how users verify a download:
     `gh attestation verify <file> --repo <owner>/FireCraft`, in the
     release notes template.
- **Done when:** a rehearsal produces attestations, and `gh attestation
  verify` passes on an asset (receipt).
- **Verify:** the receipt.
- **Moves:** ARTIFACT-MATRIX row 15 → `BUILD VERIFIED`. Part of #32.

### C9.05 — Checksums naming and the SHA-256 list (#71) `[ ]`
- **Needs:** C9.02 · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. Merge `MANIFEST.sha256` (native and web) and `RELEASE_SHA256.txt` into
     one `firecraft-<ver>-SHA256SUMS` file covering every asset, in
     `sha256sum -c` format.
  2. `release-manifest.json` stays the machine-readable twin of that file.
- **Done when:** `sha256sum -c firecraft-<ver>-SHA256SUMS` passes on the
  downloaded rehearsal assets.
- **Verify:** the receipt.
- **Moves:** close #71.

### C9.06 — SBOM completeness (#32) `[ ]`
- **Needs:** C8.06 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Check that `dist --sbom` covers both dependency graphs: the workspace
     and `web/`, which is outside the workspace.
  2. Check that the licences match `cargo deny`'s view (C8.06). Add the
     web graph if it is missing.
- **Done when:** every crate in both lockfiles appears in the SBOM; there is
  a unit test on the counts.
- **Verify:** `cargo test -p xtask sbom`.
- **Moves:** close #32 once C9.04 has landed.

### C9.07 — DEB verified on Debian 12 and Ubuntu 24.04 (#38/#39/#45) `[ ]`
- **Needs:** C9.02 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Add a release-workflow job that runs on clean containers:
     - `debian:12` and `ubuntu:24.04`;
     - `apt install ./firecraft_*.deb`;
     - `firecraft-cli --version`;
     - a CLI round trip on a corpus fixture;
     - under `xvfb-run`, launch the GUI for 10 s and assert it is still
       alive, with a screenshot via `import`;
     - `apt remove`.
  2. Write a receipt for each distribution.
- **Done when:** both receipts are committed.
- **Verify:** the receipts.
- **Moves:** DEB → `SMOKE TESTED`; S10 +1.

### C9.08 — Linux tar.gz and CLI smoke (#69) `[ ]`
- **Needs:** C9.07 · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:** use the same containers as C9.07. Extract the archive, run
  `--version` and the round trip, and launch the GUI under xvfb.
- **Done when:** the receipt is committed.
- **Verify:** the receipt.
- **Moves:** rows 1, 2 and 11 → `SMOKE TESTED`; S10 +2…3.

### C9.09 — AppImage in CI (#49–#51) `[ ]`
- **Needs:** C9.02, C1.06 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. Run `packaging/build-appimage.sh` in the release `build` job. Fetch
     `linuxdeploy` by a pinned URL and SHA-256, and install `libfuse2` or
     use `--appimage-extract-and-run`.
  2. Smoke test on `ubuntu:22.04` and `ubuntu:24.04`: run it, check
     `--version`, launch under xvfb.
- **Done when:** the AppImage is a release asset and both receipts are
  committed.
- **Verify:** the receipts.
- **Moves:** row 4 → `SMOKE TESTED`; S10 +1.

### C9.10 — Linux compatibility table (#57) `[ ]`
- **Needs:** C9.07–C9.09 · **Tags:** `[DOCS]` · **Size:** XS
- **Steps:**
  1. Write `docs/release/LINUX-COMPAT.md`: for each distribution and
     artifact, the result plus a receipt link.
  2. Record the glibc floor, from `objdump -T` on the binary (the max
     `GLIBC_` version).
- **Done when:** the table is merged.
- **Verify:** the links resolve.
- **Moves:** close #57.

### C9.11 — Windows MSVC zip on `windows-latest` (#96, #3) `[ ]`
- **Needs:** C9.02, C1.06 · **Tags:** `[HOST:gha-win]` · **Size:** M
- **Steps:**
  1. Add a release job on `windows-latest`. Install the toolchain from
     `rust-toolchain.toml`, then run `packaging/build-windows.ps1` for the
     zip part.
  2. Upload the zip.
  3. Fix whatever the recipe audit (#134) missed. Record each fix in the PR.
- **Done when:** the zip is a release asset in a rehearsal.
- **Verify:** the run URL.
- **Moves:** row 6 → `BUILD VERIFIED`.

### C9.12 — Windows zip smoke `[ ]`
- **Needs:** C9.11 · **Tags:** `[HOST:gha-win]` · **Size:** S
- **Steps:**
  1. In a separate job on a fresh `windows-latest`, download the artifact.
  2. Extract it and run `firecraft-cli.exe --version` and the round trip.
  3. Launch `firecraft.exe`, wait 10 s, and check the process is alive.
  4. Take a screenshot with PowerShell `System.Drawing`.
- **Done when:** the receipt is committed with the screenshot.
- **Verify:** the receipt.
- **Moves:** row 6 → `SMOKE TESTED`; S10 +1.

### C9.13 — Inno Setup installer, plus install, upgrade and uninstall smoke (#58–#60) `[ ]`
- **Needs:** C9.12 · **Tags:** `[HOST:gha-win]` · **Size:** M
- **Steps:**
  1. Install Inno Setup with `choco install innosetup` (pinned version), and
     compile `packaging/windows/firecraft.iss`.
  2. Smoke test:
     - `/VERYSILENT` install;
     - launch;
     - install the next version over it (build the rehearsal twice with
       different stamps);
     - uninstall;
     - assert that no files remain in Program Files.
- **Done when:** the receipts cover install, upgrade and uninstall.
- **Verify:** the receipts.
- **Moves:** row 7 → `SMOKE TESTED`; S10 +1.

### C9.14 — macOS universal app and DMG on `macos-14` (#62–#64) `[ ]`
- **Needs:** C9.02, C1.06 · **Tags:** `[HOST:gha-mac]` · **Size:** M
- **Steps:**
  1. Add a release job on `macos-14`. Add both targets,
     `aarch64-apple-darwin` and `x86_64-apple-darwin`, then run
     `packaging/build-macos-dmg.sh` (`lipo` + `hdiutil`).
  2. Upload the `.dmg`.
- **Done when:** the DMG is a rehearsal asset, and `lipo -info` shows both
  architectures.
- **Verify:** the run log.
- **Moves:** row 8 → `BUILD VERIFIED`.

### C9.15 — macOS smoke `[ ]`
- **Needs:** C9.14 · **Tags:** `[HOST:gha-mac]` · **Size:** S
- **Steps:**
  1. On a fresh runner, `hdiutil attach` the DMG and copy the app to
     `/Applications`.
  2. Run `xattr -dr com.apple.quarantine`, because the app is unsigned (see
     C9.17).
  3. `open` the app, wait 10 s, check the process with `pgrep`, and take a
     `screencapture`.
  4. Run the CLI round trip.
  5. Also test on `macos-13` (Intel) for the x86_64 slice.
- **Done when:** both receipts are committed.
- **Verify:** the receipts.
- **Moves:** row 8 → `SMOKE TESTED`; S10 +1.

### C9.16 — Web bundle smoke from the release asset `[ ]`
- **Needs:** C3.17 · **Tags:** `[HOST:browser]` · **Size:** XS
- **Steps:**
  1. Unzip `firecraft-web-<ver>.zip` and serve it statically.
  2. Run the C3.17 cross-browser smoke against it.
  3. Check that `version.json` (C3.01) matches the tag.
- **Done when:** the receipt is committed.
- **Verify:** the receipt.
- **Moves:** row 9 → `SMOKE TESTED`; rows 10 (WebOS) and 12 (source) follow
  through C9.18. S10 +1.

### C9.17 — Code signing `[ ]`
- **Needs:** C9.13, C9.15 · **Tags:** `[MAINTAINER]` (identities and
  secrets) · **Size:** M
- **Steps (agent):**
  1. Prepare signing steps behind secret-presence conditions, the same
     pattern as C0.11:
     - **Windows:** `signtool` with an Authenticode certificate or Azure
       Trusted Signing;
     - **macOS:** `codesign --options runtime`, then `notarytool submit
       --wait`, then `stapler`.
  2. With no secrets set, the steps skip with a `::warning::`.
  3. Write `docs/release/UNSIGNED.md` with the SmartScreen and Gatekeeper
     instructions for users.
- **Steps (maintainer):** provide the identities, or decide that v1 ships
  unsigned (record the choice in decision 0075).
- **Done when:** signed artifacts verify (`signtool verify /pa`, `spctl -a
  -vv`), or unsigned is the recorded decision.
- **Verify:** the receipt.
- **Moves:** row 17.

### C9.18 — Source archive builds clean (#70) `[ ]`
- **Needs:** C9.02 · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. In a clean container, extract `firecraft-<ver>-source.tar.gz` and run
     `cargo build --release --locked -p app-cli` **offline after
     `cargo fetch`**.
  2. Check that the archive contains no `source/` (Adobe) path:
     `tar tzf … | grep -c '^[^/]*/source/'` must be 0.
- **Done when:** the receipt is committed.
- **Verify:** the receipt.
- **Moves:** row 12 → `SMOKE TESTED`; close #70.

### C9.19 — Flatpak (#52/#53) — scope-dependent `[ ]`
- **Needs:** C1.06 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. If C1.06 defers Flatpak, write the deferral decision and set row 5 to
     `DEFERRED`. Done.
  2. Otherwise run `flatpak-builder` in CI with the manifest in
     `packaging/flatpak/`. That needs `cargo-sources.json` generated with
     `flatpak-cargo-generator`. Smoke test with `flatpak run` under xvfb.
- **Done when:** the row is `SMOKE TESTED` or `DEFERRED` with a decision.
- **Verify:** the receipt or the decision.
- **Moves:** row 5.

### C9.20 — ARM assessments (#55, #61) `[ ]`
- **Needs:** C1.06 · **Tags:** `[DOCS]` · **Size:** S
- **Steps:**
  1. GitHub offers `ubuntu-24.04-arm` and `windows-11-arm` hosted runners.
     **Research** current availability and pricing for this repository's
     plan before claiming it; label the source and date.
  2. Write a decision: in scope for v1, or deferred, with the build and
     smoke cost.
  3. If in scope, clone C9.07/C9.12 for arm64.
- **Done when:** a decision is recorded for each of the two rows.
- **Verify:** the decisions.
- **Moves:** rows 20 and 21.

### C9.21 — Android and RPM stay deferred (#65/#66, #54) `[ ]`
- **Needs:** C1.06 · **Tags:** `[DOCS]` · **Size:** XS
- **Steps:**
  1. Confirm decision 0070 (RPM) and the Android feasibility report are
     linked from the matrix.
  2. Add the label `post-v1` to those issues. The maintainer applies it, or
     it goes through the C0.10 checklist.
- **Done when:** the rows and issues are marked deferred.
- **Verify:** the matrix.
- **Moves:** S11 −3 or more, once they are relabelled as post-v1.

### C9.22 — Release certification checklist (RP-07) `[ ]`
- **Needs:** C9.04–C9.21 · **Tags:** `[DOCS]` · **Size:** S
- **Steps:**
  1. Update `docs/RELEASE-CHECKLIST.md`, creating it if absent, so it is the
     single ordered runbook:
     - stamp (C10.02);
     - tag;
     - CI;
     - receipts;
     - draft review;
     - publish;
     - web deploy (C3.03/C3.04);
     - announce.
  2. Each step names its command and its receipt.
- **Done when:** a fresh agent can follow the checklist without other docs.
  Test this with a dry read by a second agent, and record its questions.
- **Verify:** review.
- **Moves:** close #33.
