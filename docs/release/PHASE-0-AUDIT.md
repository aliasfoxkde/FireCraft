# Phase 0 — Release Baseline Audit

**Date:** 2026-11-09
**Audit branch:** `docs/phase-0-baseline-audit`
**Audited baseline:** `main` @ `5fac7a2267ebc6b61432d5302ab6b02a1ff15c0f` (== `origin/main`)
**Audit host:** SWARMONE, Debian 13, x86_64 (local release fleet)
**Campaign authority:** `docs/planning/release-compaign/Cross-Platform Releas.txt` — §3.2 (status vocabulary), §3.4 (evidence ladder), §4 (Phase 0 rows `P0.1.1`–`P0.4.8`)

Companion documents referenced below that are **not yet on `main`**, with the branch that carries each:

| Document | Carried by | PR |
|---|---|---|
| `docs/RELEASE-ENGINEERING-PLAN.md` | `docs/cross-platform-release-plan` @ `a0409ed`, `feat/release-workflow-repair` @ `c0c145c` | #9, #14 |
| `docs/release/ARTIFACT-MATRIX.md` | `docs/cross-platform-release-plan` @ `a0409ed`, `docs/release-execution-board` @ `49f02d4` | #9, #12 |
| `docs/release/EXECUTION-BOARD.md` | `docs/release-execution-board` @ `49f02d4` | #12 |
| `docs/tasks/T27-cross-platform-release.md` | `docs/cross-platform-release-plan` @ `a0409ed`, `docs/release-execution-board` @ `49f02d4` | #9, #12 |
| `docs/decisions/0070-rpm-deferred.md` | #9, #12, #14 | — |
| `docs/research/android-feasibility.md` | #9, #12, #14 | — |

---

## 0. Scope, method, and reading rules

### 0.1 What this document is

This is the Phase 0 deliverable of the cross-platform release campaign: a
statement of **what actually exists today**, per artifact, with the
evidence that shows it. It is not a plan and not a claim that any
release is certified.

### 0.2 Status vocabulary (campaign §3.2)

Exactly these thirteen words are used, and they are not interchangeable:

`NOT STARTED` · `READY` · `IN PROGRESS` · `BLOCKED` · `IMPLEMENTED` ·
`BUILD VERIFIED` · `INSTALLED` · `LAUNCHED` · `SMOKE TESTED` ·
`PUBLISHED` · `CERTIFIED` · `DEFERRED` · `REJECTED`

How they are applied in this document:

- On **audit rows** (P0.1.1–P0.4.8) the word describes the disposition
  of the *audit action*: `IMPLEMENTED` = the action was performed and its
  evidence is recorded here; `BLOCKED` = the action was performed but the
  audited subject cannot advance (the missing thing is named in the
  blocker register); `DEFERRED` = deliberately out of Phase 0 scope;
  `NOT STARTED` = not performed.
- On **artifact rows** (§2) the word describes the artifact's state on
  the 11-point ladder, using the artifact meaning of the word.

### 0.3 Evidence ladder (campaign §3.4)

| Rung | Meaning |
|---|---|
| 1 | Recipe exists |
| 2 | Build completed |
| 3 | Artifact produced |
| 4 | Artifact metadata inspected |
| 5 | Checksum generated |
| 6 | Installation completed |
| 7 | Application launched |
| 8 | Functional smoke completed |
| 9 | Security/signature checks completed |
| 10 | Artifact published |
| 11 | Downloaded published artifact independently verified |

**A rung number in this document is the highest rung the artifact has
reached with evidence in hand.** A recipe that has never been executed
is rung 1 — a plan is not an artifact, and a source file that exists is
not a built binary.

### 0.4 Evidence rules applied

1. Every status is backed by something observed: a file that exists on
   disk, a command's output, a CI run read from the API, or a published
   release read from the API.
2. Nothing is claimed from a script's *existence* alone.
3. **No build result in this document was fabricated.** Where a build was
   not run, the status is not `BUILD VERIFIED`.
4. Claims that could not be observed are marked `UNVERIFIED` (§6) — never
   guessed.
5. Cross-compilation would count as build evidence only; no
   cross-compiled artifact is claimed as target-native runtime evidence.

### 0.5 Audited trees

| Tree | SHA | Role in this audit |
|---|---|---|
| `origin/main` | `5fac7a2` | **the audited baseline** |
| `docs/release-execution-board` (PR #12) | `49f02d4` (head) / `08eae4a` (the tree the source-archive and SBOM builds actually ran on) | only tree with the execution board; also carries `dist --source` / `dist --sbom` |
| `docs/cross-platform-release-plan` (PR #9, draft) | `a0409ed` | carries the release plan, artifact matrix, T27 ticket, Flatpak recipe |
| `feat/release-workflow-repair` (PR #14) | `c0c145c` | release-workflow hardening, `packaging/release-assets.sh` |
| `feat/dock-inspector-parity-matrix` (PR #8) | `092a91c` | CS6 parity work |
| `feat/phase-21-daily-driver` (PR #15) | `9bb43b2` | the operator checkout's branch; **carries the `time 0.3.47` + MSRV 1.95 security bump** (B-19) |
| `feat/slice-hotspot-edit` (PR #13) | `0cb2ce3` | slice/hotspot work |
| `website/marketing-site` (PR #11) | `62dc01d` | site |
| `branding/readme-refresh` (PR #10) | `1e24e1f` | branding |

PR head SHAs were re-read from the GitHub API on 2026-11-09 at audit
time; they move during a session, so each row that names a head also
names the CI run it was judged by.

`main` has no `dist --source` and no `dist --sbom`: `xtask/src/dist.rs`
on `main` dispatches exactly two variants, `--web` and `--native`
(§8 F-9). Where this audit reports a source-archive or SBOM result, the
tree that produced it is named with the artifact.

### 0.6 Local toolchain inventory (observed 2026-11-09)

| Tool | State | Consequence |
|---|---|---|
| `rustc` / `cargo` | 1.99.0 (`b940084d7`, 2026-09-28) | workspace MSRV is declared as 1.85 on `main`; the `time` 0.3.47 security bump raises it to 1.95 (F-15) |
| `trunk` | 0.21.14 | web bundle buildable |
| `wasm-opt` | 132 | the same version named in the size-ceiling comment in `xtask/src/dist.rs`, yet the rebuilt wasm is 38,724 B larger than that comment's figure (B-18) |
| `trunk` in GitHub Actions release build | **0.22.1** — `cargo install trunk --locked` in run `41689885746` | the released web bundle was bundled by a different trunk than the audited tree uses, which is part of why the bytes differ (F-17) |
| `chromium` | 154.0.8037.92 (Debian 13) | bridge smoke runnable |
| `google-chrome` | 155.0.8059.39 | also present; `dist --web` uses one of them |
| `dpkg-deb` | 1.22.22 | DEB buildable and readable back |
| `node` | v24.16.0 | host page smoke tooling |
| `wine` | present | supplementary Windows compatibility only, never certification |
| `docker` | present | available for a clean-image test |
| RustSec advisory DB (`~/.cargo/advisory-db`) | present but **effectively empty** — 20 toolchain advisories, no `time` entry | `cargo-audit` cannot adjudicate a crate advisory offline; the `time` advisory was confirmed from the GitHub Dependabot API instead (B-19) |
| `pwsh` | **absent** | `packaging/build-windows.ps1` cannot run here |
| `ISCC` (Inno Setup 6) | **absent** | the Windows installer cannot be compiled here |
| `linuxdeploy` | **absent** | the AppImage cannot be assembled here |
| `flatpak`, `flatpak-builder` | **absent** | the Flatpak cannot be built here |
| `lipo`, `hdiutil`, Xcode CLT | **absent** (Linux host) | the macOS DMG cannot be built here |
| MSVC linker | **absent** | `x86_64-pc-windows-msvc` cannot be linked here (mingw-w64 gcc is present but unused by the recipe) |

Rust targets installed: `wasm32-unknown-unknown`, `x86_64-pc-windows-msvc`,
`x86_64-pc-windows-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
`aarch64-unknown-linux-gnu`, `aarch64-linux-android`, and the `musl`
variants. Installed is not the same as usable: the Apple and MSVC
targets have no host toolchain to link against.

---

## 1. Phase 0 row ledger (`P0.1.1` – `P0.4.8`)

### 1.1 P0.1 — Establish repository state

| Row | Status | Rung | Evidence observed | Blocker / note |
|---|---|---|---|---|
| P0.1.1 Fetch and record current main SHA | IMPLEMENTED | n/a | `git rev-parse origin/main` → `5fac7a2`; local `main` → `151e2fc` (2 commits ahead of `origin/main`, unpushed); operator checkout HEAD `9bb43b2` on `feat/dock-inspector-parity-matrix` | local `main` and `origin/main` differ; see B-1 |
| P0.1.2 Verify workspace version and Rust toolchain | IMPLEMENTED | n/a | root `Cargo.toml`: `version = "0.2.5"`, `edition = "2024"`, `rust-version = "1.85"` (all 18 member manifests consistent); `rustc 1.99.0`, `cargo 1.99.0`; **no `rust-toolchain.toml`**; CI installs `dtolnay/rust-toolchain@stable`. The `rust-version = "1.95"` + `time 0.3.47` combination required by the security bump exists only on `feat/dock-inspector-parity-matrix` / `feat/phase-21-daily-driver` (F-15) | toolchain floats (F-10); the declared MSRV is superseded on the branch that carries the dependency fix (F-15) |
| P0.1.3 Inspect working tree and uncommitted changes | IMPLEMENTED | n/a | operator checkout: `M README.md`, untracked `docs/img/app-dark.png`, `docs/planning/release-compaign/`; audit worktree `wt-phase0`: `M docs/CHANGELOG.md` | the dirty `docs/CHANGELOG.md` is a **regenerated** changelog (the `changelog` step of the gate rewrites it); it was inspected and left uncommitted — see B-16 |
| P0.1.4 List local and remote branches | IMPLEMENTED | n/a | 17 local branches, 17 remote-tracking branch refs (`git for-each-ref refs/remotes` returns 18 entries — the 17 branches plus the `origin` HEAD symref), 2 remotes (`origin` = GitHub, `gitforge` = self-hosted) — counted 2026-11-09 | — |
| P0.1.5 Identify stale and abandoned branches | IMPLEMENTED | n/a | Behind `origin/main` by 21 commits and carrying no open PR: `origin/fix/web-ux-round1` (8 unique commits) and the local `fix/web-ux-round1` (7). Behind by 21 with 18/17 unique commits and no PR: local `web-autosave-wip`, `worktree-agent-a1f780c283ad6d57b`. `origin/release/v0.2.5` (`01811d3`, 22 unique commits) is a **rewritten duplicate** of the v0.2.5 release: its tree is byte-identical to tag `v0.2.5` (`b4a698ce…`) but the tag is **not an ancestor** of it (`git tag --contains` → empty). Local-only and unreferenced: `probe/scratch` (6), `worktree-t27-release` (15). `gitforge/main` (`6c256bf`) is a **divergent lineage** — 21 behind / 20 ahead of `origin/main`, and a push to it is rejected as non-fast-forward (B-1). `gitforge/ci/batch-17` and `gitforge/ci/batch-17-rerun` (both `56ee1bf`) are 1 behind / 24 ahead — **not** stale, but unpushed work | B-1, B-2 |
| P0.1.6 Inspect current release tags | IMPLEMENTED | n/a | 6 tags on `origin`: `v0.1.0`, `v0.2.0`, `v0.2.1`, `v0.2.2`, `v0.2.3`, `v0.2.5` — **no `v0.2.4`** although v0.2.4 stamps, notes and changelog were prepared (`291d658`, `docs/releases/v0.2.4.md`). `git ls-remote --tags gitforge` returns only 4: `v0.1.0`, `v0.2.0`, `v0.2.1`, `v0.2.3` — the self-hosted mirror is missing `v0.2.2` **and the current `v0.2.5`** | B-2, B-1 |
| P0.1.7 Inspect every published release and asset | IMPLEMENTED | 11 | 6 GitHub releases; 4–5 assets each; every asset's GitHub `digest` field is `null`; `v0.2.2` has no `MANIFEST.sha256`; **no source archive, no SBOM, no `RELEASE_SHA256.txt`, no signature, no provenance has ever been published**. The v0.2.5 assets were downloaded and checked: the 3 native artifacts verify against the published `MANIFEST.sha256` (3/3 OK) and the web ZIP verifies against its inner `MANIFEST.sha256` (5/5 OK) | §2, B-17 |
| P0.1.8 Record source commit associated with each tag | IMPLEMENTED | n/a | `v0.1.0`→`60e11b8`, `v0.2.0`→`ae75ee2`, `v0.2.1`→`142093a`, `v0.2.2`→`1665ce8`, `v0.2.3`→`00d2bf8`, `v0.2.5`→`3a4832b` (all on `origin/main`) | — |

### 1.2 P0.2 — Audit PRs and issues

| Row | Status | Rung | Evidence observed | Blocker / note |
|---|---|---|---|---|
| P0.2.1 Inspect PR #9 status, diff, comments, reviews, CI | IMPLEMENTED | n/a | head `a0409ed`, **draft**, 22 files, +1511/−65, 3 issue comments, 0 reviews, **gate green** on two runs (42554147407, 42554151806) | overlaps PR #14 (B-3); still a draft |
| P0.2.2 Inspect PR #8 status, diff, comments, reviews, CI | IMPLEMENTED | n/a | head `092a91c`, 95 files, +9116/−468091 (the deletion count is the regenerated `.aegis-baseline.json`); the gate **failed** on the earlier head `88123c3` with clippy `items_after_test_module` in `xtask` (run 42628986264), fixed by `092a91c` ("move the test module to the end of the file"). **The gate is now green at the current head**: run `42633335891`, `gate` pass in 30m56s, `fmt` pass, CodeQL green | 2 unresolved review threads remain (P0.2.4) |
| P0.2.3 Inspect all other open PRs | IMPLEMENTED | n/a | #10 `1e24e1f`, #11 `62dc01d`, #12 `49f02d4`, #13 `0cb2ce3`, #14 `c0c145c`, #15 `9bb43b2` — all open and `MERGEABLE`. #12's gate was **pending** at audit time (runs `42636613428`, `42636626315`); #13's gate is **green** (runs `42634004596`, `42634016226`); **#14's gate FAILED** on both runs `42635510930` and `42635521587`; #15's `fmt` jobs are **green** (runs `42641060630`, `42641242598`) and its `gate` was **still running** at audit time | B-3, B-19 |
| P0.2.4 Inspect unresolved inline review threads | IMPLEMENTED | n/a | PR #8: **2 unresolved** — `docs/CS6-PARITY.md:172` and `docs/HANDOFF.md:151`, both CodeRabbit, both documentation scope. PRs #9, #12, #13, #14: **0** | — |
| P0.2.5 Verify whether resolved findings are actually fixed | IMPLEMENTED | n/a | The one release-relevant failure visible in CI (clippy `items_after_test_module` on PR #8 at `88123c3`) **is** fixed at `092a91c`: `xtask/src/main.rs` moves the test module to the end of the file, and the gate at that head is now green (run `42633335891`). PR #14's Aegis drift was fixed by `c0c145c` ("re-pin the Aegis baseline and clear the xtask clippy failures"), but **its gate still fails**, now on a different step: `[xtask] perf FAILED` with `spawn perf-child: No such file or directory (os error 2)` (run `42635521587`) | only findings with a CI receipt were re-verified; resolved-thread claims with no receipt were not re-derived |
| P0.2.6 Inspect open issues and related blockers | IMPLEMENTED | n/a | open: **#3** "Feature Request - Windows", **#1** "[Proposal] Incubate FireCraft under the artcraft-candidates organization"; 1 closed issue | #3 is the demand signal behind the Windows scope |
| P0.2.7 Identify overlapping branches and conflicting changes | IMPLEMENTED | n/a | PR #9 and PR #14 **both** create `docs/RELEASE-ENGINEERING-PLAN.md` and **both** implement `xtask dist --source` / `dist --sbom`; PR #12 adds `docs/release/EXECUTION-BOARD.md` on top of the same plan; PR #13 touches `xtask/src/main.rs` and `packaging/` | B-3 |
| P0.2.8 Create a PR disposition plan | IMPLEMENTED | n/a | §7 | — |

### 1.3 P0.3 — Audit release infrastructure

| Row | Status | Rung | Evidence observed | Blocker / note |
|---|---|---|---|---|
| P0.3.1 Read the complete release engineering plan | IMPLEMENTED | n/a | `docs/RELEASE-ENGINEERING-PLAN.md`, 422 lines; **absent from `main`** (carried by PR #9 / #14) | F-1 |
| P0.3.2 Read T27 | IMPLEMENTED | n/a | `docs/tasks/T27-cross-platform-release.md`; **absent from `main`**; `docs/TASKS.md` on `main` has **no T27 row** (the row exists on PR #9) | F-12 |
| P0.3.3 Read the live task board | IMPLEMENTED | n/a | `docs/release/EXECUTION-BOARD.md`, 434 lines, 61 atomic tickets, blocker register, decision register; carried by PR #12 only | — |
| P0.3.4 Audit release instructions | IMPLEMENTED | n/a | `docs/RELEASING.md`: preconditions, web/native tables, 7-step checklist | F-2, F-3, F-4 |
| P0.3.5 Audit toolchain documentation | IMPLEMENTED | n/a | `docs/TOOLCHAINS.md`: recipe table + per-artifact verification burden + the no-untested-claims rule. Consistent with the code: it states the PowerShell/Inno recipes cannot be parsed on this fleet | lists no Flatpak row because no Flatpak recipe exists on `main` |
| P0.3.6 Audit artifact matrix | IMPLEMENTED | n/a | `docs/release/ARTIFACT-MATRIX.md`; carried by PR #9 / #12, **absent from `main`** | its DEB row records the only clean-container install receipt (P0.4.3) |
| P0.3.7 Audit `xtask` distribution implementation | IMPLEMENTED | n/a | `xtask/src/dist.rs` on `main`: 764 lines; `dist --web` (WASM ceiling 21,500,000 B, bridge smoke, `FIRECRAFT_DIST_SKIP_SMOKE` escape) and `dist --native` (desktop tar.gz, CLI tar.gz, `.deb`, `MANIFEST.sha256`, `dpkg-deb --info`/`--contents` read-back, `parse_recipes()`); `write_manifest()`, `stage_web()`, `release_bin()` | no `--source` / `--sbom` on `main` (F-9); `parse_recipes()` only `bash -n` the two shell recipes and existence-checks the PowerShell/Inno files |
| P0.3.8 Audit all packaging recipes | IMPLEMENTED | n/a | 8 files on `main`: `build-appimage.sh`, `build-macos-dmg.sh`, `build-windows.ps1`, `windows/firecraft.iss`, `appimage/firecraft.desktop`, `deb/control`, `webos/appinfo.json`, `webos/host.html`. **No Flatpak recipe on `main`** (PR #9 / #14 add `build-flatpak.sh` + `flatpak/dev.firecraft.FireCraft.yml`) | B-7, B-8, B-9, B-10, B-11, B-12, B-15 |
| P0.3.9 Audit CI and release workflows | IMPLEMENTED | n/a | `.github/workflows/ci.yml` (2 jobs: `fmt`, `gate` = `xtask ci-no-fmt`, aegis 0.6.3 pinned by checksum; the job's `Web bundle smoke` step is tag-gated and skipped on branch pushes); `.github/workflows/release.yml` (2 jobs: `build artifacts`, `create draft release`); `.gitforge.yml` (8-job chain — `fmt-layers, clippy-test, wasm-web, corpus-golden, perf-budgets, fuzz, generated-docs, release` — on the shared `firecraft-ci-rust:1` image) | B-4, B-5, B-20, B-21 |
| P0.3.10 Audit version stamping and changelog generation | IMPLEMENTED | n/a | 8 stamps per HANDOFF §6.3 (workspace `Cargo.toml`, six member manifests incl. `web/`, the `xtask` `compose` pin, `packaging/deb/control`); all read 0.2.5; `xtask changelog` verifies the stamps and licenses and **rewrites** `docs/CHANGELOG.md` | B-16, F-11 |
| P0.3.11 Audit production deployment configuration | IMPLEMENTED | n/a | No Pages/wrangler config file is in the repository; deployment is a manual `wrangler pages deploy dist/web --project-name firecraft-web` (HANDOFF §6.3); the Pages step inside `release.yml` is **commented out**; the live site `https://firecraft-web.pages.dev` serves bundle `2274ac8c2925b61d` with `appinfo.json` version 0.2.5 | B-6; whether `CLOUDFLARE_API_TOKEN` is configured is UNVERIFIED (§6) |
| P0.3.12 Compare documented behavior against actual implementation | IMPLEMENTED | n/a | §8 — 17 findings (F-1 … F-17) | — |

### 1.4 P0.4 — Establish baseline verification

| Row | Status | Rung | Evidence observed | Blocker / note |
|---|---|---|---|---|
| P0.4.1 Run formatting checks | IMPLEMENTED | 1 | `cargo fmt --all -- --check` → exit 0; `cargo fmt --manifest-path web/Cargo.toml -- --check` → exit 0 (both on the audit worktree at `5fac7a2`) | — |
| P0.4.2 Run the canonical `xtask ci` gate | IMPLEMENTED | 2 | The gate is 20 steps (`aegis, coverage, fmt, clippy, test, layers, wasm, corpus, floor, scripts, golden, svg-golden, svg-roundtrip, web-golden, perf, fuzz, parity, scorecard, changelog`). The workspace-wide `clippy --all-targets` and `test --workspace` steps are hook-denied on this host, so the gate was routed through CI. **A complete green receipt exists for exactly the audited SHA**: GitHub Actions run `41938756908`, `head_sha 5fac7a2267ebc6b61432d5302ab6b02a1ff15c0f`, branch `main`, `conclusion: success` — the `fmt` job green (checkout, `cargo fmt`, `cargo fmt --manifest-path web/Cargo.toml`) and the `gate` job green (aegis install, wasm32 target, llvm-cov, trunk, `Full gate (xtask ci-no-fmt)`), so all 20 steps are covered across the two jobs on that SHA. Two limits on that receipt: the single-command form `xtask ci` was **not** invoked as such — CI splits it into `ci-no-fmt` plus the separate `fmt` job — and the job's `Web bundle smoke` step was **SKIPPED, not green**, because it is gated `if: startsWith(github.ref, 'refs/tags/v')` and this run was a branch push. The GitForge copy of the gate cannot be produced for this baseline at all (B-20) | the workspace sweeps must route through CI, not a local run on this host (B-15); the GitForge CI copy of the gate is unavailable for this baseline (B-20) |
| P0.4.3 Run native distribution build | BUILD VERIFIED | 5 | `cargo run --locked -p xtask -- dist --native` on the audit worktree at `5fac7a2`, `CARGO_TARGET_DIR=/nas/Dev/Targets/firecraft-p0`, exit 0: "[xtask] dist native: 3 archives, manifest MANIFEST.sha256, dpkg-deb verified, recipes parse-checked (2 bash)". Produced `firecraft-0.2.5-linux-x86_64.tar.gz` (22,561,250 B), `firecraft-cli-0.2.5-linux-x86_64.tar.gz` (8,101,143 B), `firecraft_0.2.5_amd64.deb` (15,120,848 B) and a 398 B `MANIFEST.sha256`; `sha256sum -c` passes on all three. `dpkg-deb --info` reads back `Package: firecraft`, `Version: 0.2.5`, `Architecture: amd64`, **`Depends: libc6 (>= 2.31)`**; `--contents` lists `/usr/bin/firecraft` (42,916,592 B), `/usr/bin/firecraft-cli` (24,810,216 B) and `/usr/share/doc/firecraft/copyright`. The extracted CLI **launches** and prints its usage. `objdump -p` on the packaged binaries: desktop needs `GLIBC_2.39`, CLI needs `GLIBC_2.35` | the DEB under-declares its glibc floor by eight minor versions (2.31 declared vs 2.39 required) → B-7 |
| P0.4.4 Run web distribution build | BUILD VERIFIED | 8 | `cargo run --locked -p xtask -- dist --web` on the audit worktree at `5fac7a2`, `CARGO_TARGET_DIR=/nas/Dev/Targets/firecraft-p0`, exit 0: "[xtask] dist web: 5 file(s), wasm 21274323 B (ceiling 21500000), manifest MANIFEST.sha256, smoke green". Staged into `dist/web/`: `firecraft-web-e44296eb8407adf3_bg.wasm` (21,274,323 B), `firecraft-web-e44296eb8407adf3.js` (86,028 B), `index.html` (2,195 B), `host.html` (1,995 B), `appinfo.json` (250 B) + `MANIFEST.sha256` (532 B). `sha256sum -c MANIFEST.sha256` passes **5/5**. The **bridge smoke ran and was green** (no `FIRECRAFT_DIST_SKIP_SMOKE` set): the headless-Chromium bridge check loaded the bundle, ran the scripted-shape fixture through the wasm registry and compared the exported bytes. A **fourth** web build exists as a receipt only: GitHub Actions release run `41689885746` (job `126105364883`) logged "[xtask] dist web: **6 file(s)**, wasm **21231517** B (ceiling 21500000), manifest MANIFEST.sha256, smoke green" at 2026-11-03T18:15:59Z — green bridge smoke on bytes that were **never published** | the rebuilt bundle is `e44296eb8407adf3`, which is neither the published v0.2.5 bundle (`f9dbe2f14305b2d9`) nor the live deployment (`2274ac8c2925b61d`); the byte total also differs from the 21,235,599 B figure recorded in `xtask/src/dist.rs` for the same nominal toolchain → B-18. Four different wasm sizes now exist for nominally the same release — **21,238,451 B published**, **21,231,517 B CI-built**, **21,235,599 B the in-source comment**, **21,274,323 B the local rebuild** — and the published one is not the CI-built one → B-21 / F-16; the live deployment remains unverified → B-6 |
| P0.4.5 Run source archive build | BLOCKED | 3 | The verb does not exist on the audited baseline (`xtask dist --source` on `main` is a usage error — F-9). Verified on the tree that has it: `cargo run --locked -p xtask -- dist --source` on `docs/release-execution-board` @ `08eae4a` (a PR #12 ancestor) → `FireCraft-0.2.5-source.tar.gz`, 3.35 MB, sha256 `a0fcaafe2f2a38d531152ea84e9a74cda174426306b96fcaf9689aa307d3bb25`; clean-extract + `cargo metadata --locked` verified inside the verb | on `main` this row is blocked; the archive has never been published |
| P0.4.6 Run SBOM generation | BLOCKED | 3 | Same as P0.4.5. Verified on `docs/release-execution-board` @ `08eae4a` (a PR #12 ancestor) → `firecraft-0.2.5-sbom.cdx.json`, CycloneDX 1.5, **493 components**, 494 dependency edges, 249 KB, sha256 `837a731b191a0187f300b346179ef086a575e69516018b6545f862ab60a2e499`; serial is a deterministic UUIDv8 derived from the locked graph | on `main` this row is blocked; the SBOM has never been published |
| P0.4.7 Inspect manifests and checksums | IMPLEMENTED | 5 | native `MANIFEST.sha256` covers exactly 3 files (desktop tar.gz, CLI tar.gz, deb); web `MANIFEST.sha256` covers 5 files (bundle `.js`, bundle `.wasm`, `appinfo.json`, `host.html`, `index.html`); published v0.2.5 `MANIFEST.sha256` is 398 bytes and covers the 3 native artifacts only; `sha256sum -c` passes on both locally | the web ZIP and the manifest itself are covered by **no** published checksum list; `RELEASE_SHA256.txt` is produced by `release.yml` but has never been published |
| P0.4.8 Record exact failures and environmental limitations | IMPLEMENTED | n/a | §4 (verified blockers) and §6 (unverified) | — |

### 1.5 Row-status tally

All 36 campaign rows are accounted for; none is `NOT STARTED`,
`READY`, `IN PROGRESS`, `INSTALLED`, `LAUNCHED`, `SMOKE TESTED`,
`PUBLISHED`, `CERTIFIED` or `REJECTED`.

| Status | Rows | Which |
|---|---|---|
| `IMPLEMENTED` | 32 | every P0.1, P0.2 and P0.3 row, plus P0.4.1, P0.4.2, P0.4.7 and P0.4.8 — the audit action was performed and its evidence is in the row |
| `BUILD VERIFIED` | 2 | P0.4.3 (native distribution build, rung 5) and P0.4.4 (web distribution build with bridge smoke, rung 8) |
| `BLOCKED` | 2 | P0.4.5 (source archive) and P0.4.6 (SBOM) — the verbs do not exist on `main`, so the subject cannot advance on this baseline (F-9, B-13) |

---

## 2. Artifact matrix — what exists today (2026-11-09)

"Path / recipe" is where the artifact comes from **today**; "evidence" is
the observation that supports the status.

**Every published artifact below was built from a tree that pins `time
0.3.45`** — the version covered by the repository's **open** GitHub
Dependabot alert #1 (`GHSA-r6v5-fh4h-64xc`, medium, vulnerable range
`>= 0.3.6, < 0.3.47`), with the fix living only on PR #15's branch
(B-19). "Published" below describes reach on the evidence ladder, not
fitness to ship.

**Every published asset also shares one provenance gap**: the release
workflow that is supposed to publish them has **never** completed its
publish step, and all six releases on record were operator-published by
hand (B-4). For v0.2.5 specifically the gap is measurable — the
workflow's own web build finished at 2026-11-03T18:15:59Z and its
`create draft release` job then failed at 18:16:19Z, while the release
and its five assets were created at **18:17:49–18:17:51Z**, 91 seconds
later, out of band (B-21). The web bundle that actually shipped is
therefore **not** the bundle the workflow built (F-16).

| # | Artifact | What exists today | Status | Rung | Evidence for the status | Verified blocker |
|---|---|---|---|---|---|---|
| 1 | Linux x86_64 desktop tar.gz | `xtask dist --native` → `dist/native/firecraft-0.2.5-linux-x86_64.tar.gz` | PUBLISHED | 11 | GitHub release v0.2.5 asset, 22,084,726 B. **Downloaded and verified 2026-11-09**: `sha256sum -c` against the published `MANIFEST.sha256` passes (`707ca85060bbf473aeabc0cfb732964e7158cc79665a79ba047f307f05d37021`); the archive unpacks to `firecraft-0.2.5/bin/{firecraft,firecraft-cli}` plus `share/licenses/LICENSE` | not reproducible from the current tree — a rebuild differs in size and digest (B-18); the release carrying it was published out of band, not by the release workflow (B-21) |
| 2 | Linux x86_64 CLI tar.gz | `xtask dist --native` → `dist/native/firecraft-cli-0.2.5-linux-x86_64.tar.gz` | PUBLISHED | 11 | GitHub release v0.2.5 asset, 8,072,377 B. Downloaded and verified 2026-11-09 against the published `MANIFEST.sha256` (`199eb1b828b9f959031c0274ca82f7e047cf846d5ea934253134df9d1b7e93d9`); the extracted `firecraft-cli` **launches** and prints its usage | not reproducible from the current tree (B-18) |
| 3 | Linux x86_64 DEB | `xtask dist --native` → `dist/native/firecraft_0.2.5_amd64.deb` | PUBLISHED | 11 | GitHub release v0.2.5 asset, 14,819,904 B. **Downloaded and verified 2026-11-09**: `sha256sum -c` against the published `MANIFEST.sha256` passes (`a81a9ad45ad604755a920794d7a00ab870af49f9401ab8492da6d8662d435a6f`); `dpkg-deb --info` reads `Version: 0.2.5`, `Architecture: amd64`, `Depends: libc6 (>= 2.31)`; the extracted `firecraft-cli` **launches**. Separately, a clean-container install/functional/reinstall/remove/purge smoke on Debian 12 passed 2026-11-06 — recorded in `docs/release/ARTIFACT-MATRIX.md` on PR #9, not on `main` | **B-7 confirmed on the published artifact**: `objdump -p` on the *published* `/usr/bin/firecraft` shows it requires `GLIBC_2.39`, while the package declares `libc6 (>= 2.31)`. The floor fix and the objdump-based floor check exist on PR #9 only |
| 4 | Linux x86_64 AppImage | `packaging/build-appimage.sh` (recipe only) | NOT STARTED | 1 | recipe exists and is `bash -n`-checked by `dist --native`; **never executed** | B-8 `linuxdeploy` absent; B-9 the desktop entry has no `Icon=` key and no icon ships in-tree; B-10 the script stages `target/release/...` and ignores `CARGO_TARGET_DIR` |
| 5 | Linux x86_64 Flatpak | **no recipe on `main`**; PR #9 / #14 add `packaging/build-flatpak.sh` + `packaging/flatpak/dev.firecraft.FireCraft.yml` | NOT STARTED | 1 | manifest parses (YAML parse check only) on the PR trees | B-11 `flatpak` and `flatpak-builder` are absent; on `main` there is no recipe at all |
| 6 | Windows x86_64 portable ZIP | `packaging/build-windows.ps1` (recipe only) | NOT STARTED | 1 | recipe exists; **never executed** — it targets `x86_64-pc-windows-msvc` and needs PowerShell | B-12 `pwsh` absent and no MSVC linker on this host (mingw-w64 gcc is present but the recipe does not use it) |
| 7 | Windows Inno Setup installer | `packaging/windows/firecraft.iss` (recipe only) | NOT STARTED | 1 | recipe exists; the `__APPVERSION__` placeholder is substituted by the PowerShell recipe, which has never run | `ISCC` absent (B-12); the script "skips if ISCC absent", so a Windows run without Inno Setup silently ships no installer |
| 8 | macOS universal DMG | `packaging/build-macos-dmg.sh` (recipe only) | NOT STARTED | 1 | recipe exists (two `apple-darwin` slices → `lipo` → `hdiutil`); **never executed** | needs macOS + Xcode CLT; the recipe is off-fleet by design |
| 9 | Web (WASM) bundle | `xtask dist --web` → `dist/web/` (+ manual zip) | PUBLISHED | 11 | GitHub release v0.2.5 asset `firecraft-web-0.2.5.zip`, 7,174,826 B. **Downloaded and verified 2026-11-09**: it contains 6 files — bundle `firecraft-web-f9dbe2f14305b2d9{,_bg.wasm}`, `index.html`, `host.html`, `appinfo.json` and its own `MANIFEST.sha256` — and `sha256sum -c` against that inner manifest **passes for all 5 entries**. Its entries are stamped **2026-11-03 12:08**, six hours before the release workflow finished its own web build (18:15:59Z) — the published bundle is **not** the CI-built one (F-16). The bundle was also rebuilt and bridge-smoked **green** locally on 2026-11-09 (P0.4.4) as a **third** hash, `e44296eb8407adf3`. The live deployment serves a **fourth**, `2274ac8c2925b61d` | the published bundle was never built by the release workflow that nominally publishes it (B-21); the live deployment is not the published v0.2.5 bundle, and its own published manifest fails `sha256sum -c` for the two HTML files because Cloudflare Pages injects its analytics beacon → B-6 |
| 10 | WebOS package | `packaging/webos/appinfo.json` + `host.html`, stamped into `dist/web/` by `dist --web` | BUILD VERIFIED | 5 | staged `appinfo.json` reads 0.2.5; the web manifest covers `appinfo.json` and `host.html`; the live site serves the same `appinfo.json` bytes | device/runtime smoke is UNVERIFIED (§6); the Pages deploy is manual and currently stale (B-6) |
| 11 | CLI binaries | `firecraft-cli`, shipped inside #1 and #2 | PUBLISHED | 11 | the **published** `firecraft-cli` was downloaded and extracted 2026-11-09 and **launches**, printing its full usage string; the locally rebuilt one does the same (P0.4.3) | — |
| 12 | Source archive | `xtask dist --source` → `dist/source/FireCraft-0.2.5-source.tar.gz` (**PR #9 / #12 trees only**) | BUILD VERIFIED | 3 | produced 2026-11-09 on `docs/release-execution-board` @ `08eae4a`: 3.35 MB, 661 entries, sha256 `a0fcaafe…`; the verb itself verifies a clean extraction and runs `cargo metadata --locked` inside it | the verb is absent from `main` (F-9); never published (B-13) |
| 13 | SBOM (CycloneDX 1.5) | `xtask dist --sbom` → `dist/sbom/firecraft-0.2.5-sbom.cdx.json` (**PR #9 / #12 trees only**) | BUILD VERIFIED | 3 | produced 2026-11-09 on `docs/release-execution-board` @ `08eae4a`: 249 KB, 493 components, 494 dependency edges, sha256 `837a731b…` | the verb is absent from `main` (F-9); never published (B-13) |
| 14 | Checksums / manifest | `dist/native/MANIFEST.sha256` (3 native artifacts) and `dist/web/MANIFEST.sha256` (5 bundle files) | PUBLISHED | 11 | `MANIFEST.sha256` attached to release v0.2.5 (398 bytes) and embedded in the web ZIP (532 bytes). **Both were checked against the published downloads on 2026-11-09 and both pass** — 3/3 for the native manifest, 5/5 for the web manifest | neither list covers the ZIP files themselves, and no release-wide `RELEASE_SHA256.txt` was ever published although `release.yml` writes one (B-4, B-17); the manifest and the assets it covers were published out of band, so the list is self-consistent but not independently attested (B-21) |
| 15 | Provenance / attestation | **nothing** on any branch | NOT STARTED | 0 | no generator, no `gh attestation` step, no in-toto/SLSA material anywhere in the repository | B-14 |
| 16 | Release manifest (JSON) | **nothing** on any branch | NOT STARTED | 0 | the campaign and the plan require a machine-readable artifact manifest; none exists | B-14 |
| 17 | Signature (Authenticode / Apple Developer ID / GPG) | **nothing**; no signing identity is configured anywhere | NOT STARTED | 0 | `docs/release/ARTIFACT-MATRIX.md` (PR #9) records "No signing credentials configured anywhere" | B-14 — no artifact can reach rung 9 until this is resolved |
| 18 | Android arm64 APK/AAB | feasibility report only (`docs/research/android-feasibility.md`, PR #9 / #12) | DEFERRED | 0 | the report concludes "feasible with bounded work", disposition experimental | the gate decision is unrecorded; no NDK/SDK image, no device (§5) |
| 19 | RPM | no spec, no recipe | DEFERRED | 0 | `docs/decisions/0070-rpm-deferred.md` (PR #9 / #12), accepted 2026-11-06 | decision 0070 is not on `main` (§5) |

### 2.1 Status counts

| Status | Count | Artifacts |
|---|---|---|
| `NOT STARTED` | 8 | AppImage, Flatpak, Windows ZIP, Windows installer, macOS DMG, provenance/attestation, release manifest JSON, signature |
| `READY` | 0 | — |
| `IN PROGRESS` | 0 | — |
| `BLOCKED` | 0 | — |
| `IMPLEMENTED` | 0 | — |
| `BUILD VERIFIED` | 3 | WebOS package, source archive, SBOM |
| `INSTALLED` | 0 | — |
| `LAUNCHED` | 0 | — |
| `SMOKE TESTED` | 0 | — (the DEB's clean-container smoke is recorded, but the artifact itself is published, so it is counted at its highest rung) |
| `PUBLISHED` | 6 | desktop tar.gz, CLI tar.gz, DEB, web bundle, CLI binaries, checksums/manifest |
| `CERTIFIED` | 0 | — |
| `DEFERRED` | 2 | Android arm64, RPM |
| `REJECTED` | 0 | — |

**No artifact has reached `CERTIFIED`.** Five artifacts (desktop tar.gz,
CLI tar.gz, DEB, web bundle, CLI binaries) reach **rung 11** — they were
downloaded from the published release and verified against their
published checksum lists on 2026-11-09, and the CLI binary was launched.
Nothing has reached rung 9 (security/signature checks): no artifact is
signed, and no provenance/attestation exists.

---

## 3. Evidence-ladder detail

The ladder as it stands for the published set, with the rung each
artifact actually reached:

- **Desktop tar.gz / CLI tar.gz / DEB / CLI binaries** — **rung 11**.
  Rungs 1–5 and 10 are evidenced (recipe, build, artifact, metadata,
  checksum, publication), and rung 11 was reached on 2026-11-09: all
  three assets were downloaded from the published GitHub release and
  `sha256sum -c` against the published `MANIFEST.sha256` **passes on all
  three**. The published DEB was extracted (`dpkg-deb --info` /
  `--contents` read back) and the published `firecraft-cli` **launched**
  (rung 7). Rungs 6–8 for the DEB are additionally evidenced by the
  2026-11-06 clean-container receipt (PR #9); no install/launch/smoke
  receipt exists for the tarballs. Rung 9 is **not** reached: nothing is
  signed.
  The published artifacts were also independently rebuilt on the audited
  tree 2026-11-09 — all three archives produced, metadata read back,
  manifest verified, extracted CLI launched — but **the rebuild does not
  reproduce the published bytes**: the local desktop tarball is
  22,561,250 B against the published 22,084,726 B, and every digest
  differs. That is partly explained — the published assets were built at
  v0.2.5 from `3a4832b`, this rebuild from `5fac7a2` — but nothing in
  the pipeline records the compiler, linker, glibc or build flags, so the
  difference cannot be attributed (B-18). The glibc floor is also
  **coupled to the build host**: the rebuild ran on Debian 13 (glibc
  2.41), so the linker was free to bind symbols as new as 2.39. Nothing
  pins the build image or records the linked symbol versions, so the
  declared `2.31` and the measured `2.39` are two different facts about
  two different machines — and a hard-coded floor fix would encode
  whichever one happened to be measured first.
- **Web bundle** — **rung 11** for the *published ZIP*: downloaded
  2026-11-09, unpacked to 6 files, and `sha256sum -c` against its inner
  `MANIFEST.sha256` passes on all 5 entries. Rung 8 (functional smoke) is
  evidenced — but **on different bytes than the ones that shipped**. The
  audit's own `dist --web` on 2026-11-09 ran the bridge smoke green on
  the rebuilt bundle `e44296eb8407adf3`; the release workflow ran it
  green on its own bundle (21,231,517 B) at 2026-11-03T18:15:59Z; the
  **published** bundle is a third build (21,238,451 B) whose entries are
  stamped 12:08, six hours before the workflow's build finished. No
  recorded smoke ever ran against the published bytes (F-16 / B-21).
  Rungs 6–7 are `n/a` for
  a static bundle — there is nothing to install or launch.
  The **deployed** artifact is a separate question and is *not* rung 11:
  the live site serves bundle `2274ac8c2925b61d`, which is neither the
  published v0.2.5 bundle (`f9dbe2f14305b2d9`) nor the bundle this audit
  rebuilt from `5fac7a2` (`e44296eb8407adf3`) nor the one the tree's
  `web/dist` held (`cf1bec8315c7bfe5`), so the deployment corresponds to
  no identifiable release
  (B-6). The deployment's own checksum list cannot close that gap: it is
  computed over the pre-deploy bytes, and Cloudflare Pages rewrites the
  two HTML files after deploy, so `sha256sum -c` against the live site
  fails on `index.html` and `host.html` by construction.
- **WebOS package** — rungs 1–5 only; the target device/runtime smoke was
  never recorded.
- **Source archive / SBOM** — rungs 1–3 only, and only on the PR #9/#12
  trees; rung 5 is reached only in the sense that the build printed the
  SHA-256 (no manifest file lists it).
- **AppImage, Flatpak, Windows ZIP, Windows installer, macOS DMG** — rung 1.
- **Provenance, release manifest JSON, signature** — rung 0.

---

## 4. Verified blocker register

Each entry names the specific missing thing, the observation that proves
it, and what it blocks. Nothing here is inferred from "not done".

| ID | Blocker | Evidence | Blocks |
|---|---|---|---|
| B-1 | `gitforge/main` is a **divergent lineage** from `origin/main`: they share only the PR #4 merge (`7c82997`). `gitforge/main` (`6c256bf`, 2026-11-02) does not contain the v0.2.5 release commit (`3a4832b`) and carries no `v0.2.2` or `v0.2.5` tag. `git push --dry-run gitforge main` is **rejected: non-fast-forward**. | `git merge-base 6c256bf 5fac7a2` → `7c82997`; `git merge-base --is-ancestor 3a4832b 6c256bf` → false; the trees at the v0.2.4 point are identical (`35f5069…`), so this is a squash-rewrite divergence, not content drift; `git ls-remote --tags gitforge` lists only `v0.1.0`, `v0.2.0`, `v0.2.1`, `v0.2.3` | The documented publish step "push both remotes (gitforge first, then origin)" **cannot complete** without a force-push, which is forbidden. Every release so far reached GitHub only. The GitForge tag-gated release job therefore never ran for `v0.2.2`/`v0.2.5`. |
| B-2 | `v0.2.4` was prepared but never released: version stamps, `docs/releases/v0.2.4.md` and a changelog entry exist (`291d658`), yet there is no `v0.2.4` tag and no GitHub release. | `git tag -l` → no `v0.2.4`; `gh release list` → 6 releases, none v0.2.4; `origin/release/v0.2.5` (`01811d3`) is a pre-squash release branch that was never tagged | The published version sequence skips v0.2.4 while the changelog advertises it; a consumer reading `docs/CHANGELOG.md` sees a release that does not exist |
| B-3 | PR #9 and PR #14 overlap exactly: both add `docs/RELEASE-ENGINEERING-PLAN.md` as a **new 455-line file** and both add **464 lines** to `xtask/src/dist.rs` implementing `dist --source` and `dist --sbom`. PR #12 adds the execution board and the Flatpak recipe on top of the same plan. | `git diff --stat origin/main...origin/docs/cross-platform-release-plan` and `git diff --stat origin/main...origin/feat/release-workflow-repair` both report `docs/RELEASE-ENGINEERING-PLAN.md | 455 +++` and `xtask/src/dist.rs | 464 ++`; `git diff --stat origin/main...origin/docs/release-execution-board` reports `packaging/flatpak/dev.firecraft.FireCraft.yml` (new) and the same `xtask/src/dist.rs` growth | Two of the three PRs will conflict on merge; the release work must be consolidated into one branch before anything can be certified |
| B-4 | `.github/workflows/release.yml` cannot create a release. **Three independent defects, all confirmed against a real failed CI run** (run `41689885746`, tag `v0.2.5`, 2026-11-03): (a) the release step builds `assets+=(-a "$f")` and `gh release create` has **no `-a` flag** — it takes assets as positional arguments; the run dies with `unknown shorthand flag: 'a' in -a`; (b) `actions/checkout` runs **after** `actions/download-artifact`, and checkout's default `clean: true` runs `git clean -ffdx`, which deletes the untracked `artifacts/` directory the previous step created — this defect is currently *masked* by (a), which fails first; (c) `upload-artifact` stores the least common ancestor of the uploaded paths, so the artifact unpacks as a **tree** (`artifacts/native/firecraft-*.tar.gz`, `artifacts/firecraft-web-*.zip`, `artifacts/RELEASE_SHA256.txt`) while the release step globs `artifacts/*` — that glob matches the `native/` **directory** and the web zip, and misses all three native files, so even a corrected `-a` would attach a directory and drop the tarballs and the DEB. | run `41689885746`, job `126113210174`, step "Create draft release": log lines show `unknown shorthand flag: 'a' in -a` followed by `Usage:  gh release create [<tag>] [<filename>... | <pattern>...]`. The `build artifacts` job of the same run **succeeded** end to end (native, web, zip, checksums, upload all green) — so the artifacts were built and uploaded correctly and only the publication step failed. The workflow file itself shows the checkout step at line 120 following the download step at line 114, and the upload path list spanning `dist/native/…` and `dist/firecraft-web-*.zip`. PR #14 fixes all three (`packaging/release-assets.sh stage`, fail-closed aggregate validation, positional assets) but is unmerged | **All 5 runs of the release workflow on record failed**, for tags `v0.2.2`, `v0.2.3` and `v0.2.5` (runs `39309000350`, `40158943591`, `41689354812`, `41689611520`, `41689885746`) — the workflow has **never** produced a release. All 6 published releases were operator-published by hand, which is why no `RELEASE_SHA256.txt`, no source archive and no SBOM asset exist |
| B-5 | The **GitForge** release job sets `FIRECRAFT_DIST_SKIP_SMOKE=1`, so on that path the web bundle is built **without** the bridge smoke. The GitHub Actions release path is *not* affected — it installs `chromium-browser` and runs the smoke. | `.gitforge.yml` release job: `FIRECRAFT_DIST_SKIP_SMOKE=1 CARGO_TARGET_DIR=/tmp/ci-dist-target cargo run --locked -p xtask -- dist --web`. Contrast `.github/workflows/release.yml` lines 54–56 (`apt-get install -y chromium-browser`) and the v0.2.5 run log: "[xtask] dist web: 6 file(s), wasm 21231517 B (ceiling 21500000), manifest MANIFEST.sha256, **smoke green**" | The GitForge release path would publish web bytes that were never exercised through the bridge; `docs/RELEASING.md` claims the opposite ("runs the bridge smoke on these release bytes") without naming which release path it means. Because B-1 blocks the GitForge tag-gated job from ever running for `v0.2.2`/`v0.2.5`, this is a latent defect, not the cause of the shipped bytes — but it is the path the campaign's next release is meant to use, and it needs the smoke back |
| B-6 | The live deployment serves a web bundle that is **neither** the published v0.2.5 release **nor** anything the audited tree produces, and its own published checksum list does not verify against what it serves. | The published v0.2.5 web ZIP contains bundle `firecraft-web-f9dbe2f14305b2d9{,_bg.wasm}`. The live site serves bundle `firecraft-web-2274ac8c2925b61d{,_bg.wasm}`. The audited tree's own `web/dist` holds a third bundle, `cf1bec8315c7bfe5` (built 2026-11-08); a fourth, `e44296eb8407adf3`, was produced by the audit's own `xtask dist --web` on 2026-11-09 (P0.4.4). Four distinct bundles. Separately, the live `MANIFEST.sha256` lists `index.html` = `2617be68…` and `host.html` = `379720cc…`; downloading all five files it names and running `sha256sum -c` gives **`appinfo.json: OK`, bundle `.js`: OK, bundle `_bg.wasm`: OK`, but `host.html: FAILED` and `index.html: FAILED`** — because Cloudflare Pages injects its analytics beacon into HTML at serve time (`</script><!-- Cloudflare Pages Analytics --><script defer src='https://static.cloudflareinsights.com/beacon.min.js' …>`), so the served bytes are not the built bytes | The deployed web artifact cannot be tied to any published release or to the audited tree, and the deployment's checksum list **cannot be used to verify the deployment at all** — the two HTML entries are guaranteed to fail on any Cloudflare Pages deployment, because the platform rewrites them after the manifest is written. The manifest is valid only for the pre-deploy bytes. Deployment reconciliation is a release blocker the campaign plan already names |
| B-7 | The DEB's declared glibc floor is far below the binaries' real requirement: `packaging/deb/control` on `main` says `Depends: libc6 (>= 2.31)`, while the desktop binary requires **GLIBC 2.39** (the CLI requires 2.35). **Confirmed on the published artifact**, not only on a local rebuild. | `dpkg-deb --info` on the **published** `firecraft_0.2.5_amd64.deb` (downloaded 2026-11-09) reads `Depends: libc6 (>= 2.31)`; `objdump -p` on the published, extracted `/usr/bin/firecraft` lists `GLIBC_2.39` as its highest required version. The same holds for the locally rebuilt DEB (P0.4.3). PR #9's artifact matrix records the same class of failure with an older measurement ("caught and fixed a floor of 2.31 vs binaries needing 2.35") and adds an objdump-based floor check to `dist --native` | The **published** DEB understates its dependency by eight minor glibc versions; installation on any distribution older than glibc 2.39 (Ubuntu 22.04, Debian 11, RHEL 8/9) produces a package that installs cleanly and then fails to start. The fix is unmerged, and the recorded 2.35 figure is itself stale — the floor must be measured per build, not hard-coded |
| B-8 | `linuxdeploy` is not installed on this fleet, and no pinned copy is vendored. | `command -v linuxdeploy` → not found; `docs/TOOLCHAINS.md` lists `linuxdeploy-x86_64.AppImage` as an operator prerequisite | The AppImage cannot be produced here |
| B-9 | `packaging/appimage/firecraft.desktop` has no `Icon=` key and no icon ships in-tree (the asset rule keeps third-party art out of the repo). | the file's contents; `packaging/build-appimage.sh` states "No icon ships in-tree" | linuxdeploy has nothing to install, and the resulting desktop entry has no icon — desktop integration is incomplete even if the build ran. PR #9 adds the `Icon=` line (`a4d74a0`); unmerged |
| B-10 | `packaging/build-appimage.sh` stages `target/release/firecraft` and `target/release/firecraft-cli` and ignores `CARGO_TARGET_DIR`. | the script's `install` lines | Under this repository's shared-target-dir convention (`target/` is a symlink to `/nas/Dev/Targets/firecraft-target`) or any `CARGO_TARGET_DIR` build, the script stages binaries that are stale or missing |
| B-11 | `flatpak` and `flatpak-builder` are not installed. | `command -v flatpak` → not found | The Flatpak cannot be built here |
| B-12 | The Windows recipe cannot run on this fleet: `pwsh` is absent and the recipe targets `x86_64-pc-windows-msvc`, which needs an MSVC linker that does not exist on Linux. | `command -v pwsh` → not found; `packaging/build-windows.ps1` target line; `rustup target list --installed` shows the target installed but no host toolchain | The portable ZIP and the Inno Setup installer can only be built on Windows |
| B-13 | The source archive and the SBOM have never been published, and neither verb exists on `main`. | the six GitHub releases' asset lists (4–5 assets each, no source archive, no SBOM); `xtask/src/main.rs` on `main` dispatches only `--web` and `--native` | The release set is missing the two artifacts the plan calls mandatory |
| B-14 | No provenance/attestation generator, no machine-readable release manifest and no signing identity exist on any branch. | no `attestation`, `in-toto`, `SLSA`, `cosign` or `sign` machinery in the repository; `docs/release/ARTIFACT-MATRIX.md` (PR #9): "No signing credentials configured anywhere" | No artifact can reach rung 9, so no artifact can be `CERTIFIED` |
| B-15 | Three packaging recipes point at `packaging/TOOLCHAINS.md`, which does not exist (the document is `docs/TOOLCHAINS.md`). | `packaging/build-appimage.sh:11`, `packaging/build-macos-dmg.sh:7`, `packaging/build-windows.ps1:8`; `ls packaging/` shows no `TOOLCHAINS.md` | The reader is sent to a missing file from the three recipes that cannot be exercised on this fleet |
| B-16 | The canonical gate writes a tracked file: the `changelog` step of `xtask ci` rewrites `docs/CHANGELOG.md`. The committed copy on `main` still lists the pre-squash commits, and the regenerated copy differs (it collapses the squash-merged batches 4–17 into a single `#7` entry). | `xtask/src/changelog.rs:33` (`std::fs::write`); the uncommitted `docs/CHANGELOG.md` diff in the audit worktree; `git log --oneline main` shows the squashed history | A green gate leaves a dirty working tree; the changelog shipped on `main` does not describe `main`'s history. The regenerated copy was inspected and deliberately left uncommitted by this audit |
| B-17 | The **release-wide** checksum file is missing and no published asset carries a per-asset digest, so only the artifacts that happen to be covered by the native `MANIFEST.sha256` or the web ZIP's inner manifest can be independently verified. | every asset in all six GitHub releases reports `digest: null`; `RELEASE_SHA256.txt` is written by `release.yml` but was never attached. **Correction to an earlier reading**: the published v0.2.5 `MANIFEST.sha256` (398 B, 3 native files) *does* verify against the downloaded artifacts — `sha256sum -c` passes on all three — and the web ZIP carries its own inner `MANIFEST.sha256` that likewise passes on all 5 entries. What is missing is a single file that covers the **release as a whole**, including the ZIPs themselves | The three native artifacts and the web bundle reach rung 11 today, but only because they happen to carry an inner manifest. A consumer who downloads the release page has no way to check the ZIP's own bytes, and nothing binds the release set to a single source commit |
| B-18 | The published artifacts are **not reproducible** from the current baseline: a rebuild of `5fac7a2` produces different bytes from the published v0.2.5 assets. | local rebuild (P0.4.3): desktop tar.gz 22,561,250 B / sha256 `54edd74f…` vs published 22,084,726 B / `707ca850…`; CLI tar.gz 8,101,143 B / `bf71e2a1…` vs published 8,072,377 B / `199eb1b8…`; DEB 15,120,848 B / `553befbd…` vs published 14,819,904 B / `a81a9ad4…`. The two sources differ (`3a4832b` vs `5fac7a2`), so part of the size difference is explained — but nothing in the pipeline records the compiler, linker, glibc or build flags, so the difference cannot be attributed. The **web** side shows the same class with the source held constant: the audit's `xtask dist --web` on `5fac7a2` produced a 21,274,323 B wasm, while `xtask/src/dist.rs` records 21,235,599 B for "the same" nominal toolchain (trunk 0.21.14, rustc 1.99, wasm-opt 132) — a 38,724 B delta with no recorded cause. The v0.2.5 release build adds two more points to the same curve: the workflow's own build (trunk **0.22.1**, unpinned) produced 21,231,517 B, and the bundle that was actually published is 21,238,451 B — a spread of **42,806 B** across four builds of nominally the same release, with the bundler version not recorded in the receipt (F-17) | Even with a published checksum file, an independent verifier cannot reproduce the artifact to compare against; the build is not hermetic and no build-provenance is recorded. Independent of B-17 |
| B-19 | The audited baseline `main` — the tree every published artifact is built from — pins a dependency version that the repository itself has flagged as security-affected, and the remediation does not exist on `main`. | `origin/main:Cargo.lock` → `time 0.3.45`. Commits `c74b179` / `e9747b8` ("build: raise MSRV to 1.95 and bump time past CVE-2026-25727") record a Dependabot stack-exhaustion denial-of-service advisory (CVE-2026-25727) requiring `time >= 0.3.47`, and record that the bump cannot be applied without raising the workspace `rust-version` from 1.85 to **1.95** (cargo's MSRV-aware resolver floors on the lowest `rust-version` in the workspace, so the lockfile silently regressed to 0.3.45 until the floor moved). `git branch -a --contains c74b179` lists only `feat/dock-inspector-parity-matrix` and `feat/phase-21-daily-driver` — both pushed to origin **and** gitforge, neither merged into `main`; the latter is open as PR #15 at head `9bb43b2`. **The advisory itself is confirmed from the authoritative source**: GitHub's Dependabot API reports alert **#1, state `open`, severity `medium`**, package `time` in `Cargo.lock`, `GHSA-r6v5-fh4h-64xc` / `CVE-2026-25727`, summary "time vulnerable to stack exhaustion Denial of Service attack", vulnerable range `>= 0.3.6, < 0.3.47`, first patched version `0.3.47`. `main` pins `0.3.45` — inside the vulnerable range — and the alert is open against the default branch | Every published v0.2.5 artifact, and any rebuild of `main`, carries the flagged version. The bump and the 1.95 MSRV must land on `main` or on the release branch before the next artifact is built. Note the coupling: this changes the toolchain floor, so it will move the glibc floor (B-7) and the reproducibility baseline (B-18) again — the next release must be **rebuilt and re-measured**, not re-tagged |
| B-20 | **GitForge CI can no longer build `main` at all.** The single shared runner image is keyed to one lockfile — the one on PR #15 — so every branch still on `main`'s `Cargo.lock` fails the sandbox at its first cargo step, and `main` itself cannot be validated on the authoritative CI system. | The image `firecraft-ci-rust:1` on the runner host (fedora) was built **2026-11-09 08:54** and its cargo registry cache holds `time-0.3.41`, `time-0.3.47` and `time-0.3.55` — **not** `time-0.3.45`, which is what `main` pins. `.gitforge.yml` sets `CARGO_NET_OFFLINE=true`, and its own header warns "Rebuild + re-ship the image before pushing any commit that moves either lockfile". Two runs prove the consequence: a retrigger of this audit's own branch (`tmp/p0-ci-retrigger`, run `ddb0c9df`) and a probe of `main` itself (`tmp/p0-main-probe`, run on `5fac7a2`) both fail in the first job with `fmt-layers` exit 101 — "error: failed to download `time v0.3.45` / Caused by: attempting to make an HTTP request, but --offline was specified" — and every downstream job is `cancelled` | The GitForge copy of the 20-step gate cannot be run for `main` or for any branch that has not taken the PR #15 lockfile, so P0.4.2 has no GitForge receipt for the audited baseline (and could not get one). The image is shared, so re-shipping it for one branch breaks the others — the image must hold **both** lockfile states, or the sandbox must be allowed to fetch `time 0.3.45` |
| B-21 | **The published v0.2.5 web bundle is not the bundle the release workflow built, and the release itself was published out of band.** The release workflow's `create draft release` job failed, and the release plus its assets were created 91 seconds later by a path the workflow does not contain. | Release run `41689885746`: job `126105364883` ("build artifacts") succeeded and logged "[xtask] dist web: 6 file(s), wasm 21231517 B (ceiling 21500000), manifest MANIFEST.sha256, smoke green" at **18:15:59Z**; job `126113210174` ("create draft release") then failed at **18:16:19Z** with `unknown shorthand flag: 'a' in -a` (B-4 defect (a)). The GitHub release `v0.2.5` reports `published_at 2026-11-03T18:17:51Z` with all five assets created at **18:17:49Z** — 91 seconds after the workflow died, with no job account for it. The published ZIP's own entries are stamped **2026-11-03 12:08** and its wasm is **21,238,451 B**, against the workflow's **21,231,517 B**; the ZIP holds 5 content files + manifest where the workflow staged 6 content files (a CI-built `dist/web` would zip to 7 entries) | The bytes that shipped have **no build receipt**: no log, no commit SHA, no toolchain, and no bridge smoke of *those* bytes — the smoke was green on a different bundle (F-16). Nothing in the repository can attest what the published v0.2.5 artifacts are, and the next release will inherit the same hand-publish path until B-4 is fixed |
| B-22 | **The GitForge runner's failure path arms its container reap and backend-health checks with no timeout**, so a wedged Docker daemon wedges the runner instead of failing the job. This defect lives in the **GitForge** repository, not in FireCraft. | `/nas/Temp/repos/GitForge/crates/gitforge-runner/src/executor.rs`: the failure path calls `self.reap_attempt_containers(job_id).await` (lines 693 and 716) and `self.pool.sandbox.backend_health().await` (lines 709, 730, 769, 848, 871) with no timeout, while the preflight-teardown path wraps the same class of call in `timeout(Duration::from_secs(15), …)`. The unbounded `backend_health` reaches an unbounded bollard `list_containers` in `crates/gitforge-sandbox/src/docker.rs`. | FireCraft's authoritative CI receipt depends on the GitForge runner completing: a runner that hangs instead of failing produces **no terminal job state**, which blocks the campaign's evidence ladder for every GitForge-verified row (P2.1 "Verify authoritative GitForge CI"). Tracked as FireCraft issue **#37** and as task #202; the fix belongs in the GitForge repository. |
| B-24 | **The published DEB carries no desktop integration at all** — no desktop entry, no icon, no MIME registration. Installed on Debian 12 it puts a `firecraft` on `$PATH` and nothing else: no menu entry, no launcher, no icon. | `dpkg-deb --contents` on the **published** `firecraft_0.2.5_amd64.deb` (downloaded 2026-11-10) lists exactly four payload entries: `./usr/bin/firecraft`, `./usr/bin/firecraft-cli`, `./usr/share/doc/firecraft/copyright` and the directories above them. The same holds for the published `firecraft_0.2.3_amd64.deb`. The build source agrees: `xtask/src/dist.rs::build_deb` staged only the two binaries and the copyright, and `packaging/deb/` contained nothing but `control`. The `packaging/appimage/firecraft.desktop` that exists in the tree was never staged into the deb — the AppImage recipe staged it, the deb did not. Verified on the *published* package in a clean Debian 12 container: the package installs cleanly and then has no desktop entry (see §11.6) | The campaign's DEB rows "validate desktop entry", "validate icon installation" and "validate MIME integration" could not be closed as "not implemented" — the package is a P0 MVP Linux deliverable and the user's first question after `apt install` is "where is the launcher?". The rows were answered with a defect instead |
| B-25 | **The Flatpak recipe's runtime step could not run.** It invoked `flatpak run --repo="${REPO}" dev.firecraft.FireCraft --version`, but `flatpak run` has no `--repo` option, and the recipe never installed the application into an installation — so there was nothing to run. | `packaging/build-flatpak.sh` (before the fix): `flatpak run --command=firecraft --repo="${REPO}" dev.firecraft.FireCraft --version`. The documented `flatpak run` option list contains no `--repo` (it is a `flatpak-builder`/`flatpak build-bundle` option); the man page enumerates `--command`, `--cwd`, `--env`, `--filesystem`, `--installation`, `--sandbox` and the rest, with no `--repo`. Independently, the recipe's only runtime evidence was `--version` on the *GUI* binary, which takes no version flag at all — `crates/app/src/main.rs::main` calls `eframe::run_native` and parses no arguments | The Flatpak campaign rows "install the built application into a test installation", "correct `flatpak run` usage", "launch using `flatpak run`" and "test removal" had no executable path at all: the recipe's launch step was a command flatpak rejects. The recipe also invoked `flatpak-builder` without `--share=network`, so the cargo build inside the build sandbox would die at its first dependency fetch (the sandbox has no network by default) |
| B-26 | **The Flatpak manifest shipped no icon** while the desktop entry it installs names `Icon=firecraft`, and it installed the desktop entry from a path that no longer exists. | `packaging/flatpak/dev.firecraft.FireCraft.yml` staged `packaging/appimage/firecraft.desktop` and no icon file at all; the AppImage recipe staged the same desktop file *and* `assets/icons/hicolor/256x256/apps/firecraft.png`. The manifest's `finish-args` comment claimed "File open/save/export goes through the portal file chooser", but the shell's dialogs are `EnvDialogs` (`crates/app/src/shell/dialogs.rs`), which read the document path from `FIRECRAFT_DOC` and the host's document directory — there is no portal chooser | A Flatpak launcher entry that resolves to no icon shows a broken image, and the campaign's "review `finish-args`" / "minimize filesystem permissions" rows were being audited against a comment that described a capability the app does not have |
| B-27 | **`firecraft-cli` had no `--help`.** The flag fell through to the positional parser, so the usage line was printed as an *error message on stderr* with a failing exit code. | `crates/app-cli/src/main.rs`: the verb dispatch had no `--help` arm; an unrecognised first argument became a positional, and `missing(USAGE)` returns `CliError("missing arguments (expected: usage: …)")`, which `main` prints to stderr with `ExitCode::FAILURE`. `crates/app-cli/tests/` had no test that runs the binary | A packaged CLI that cannot be asked what it does cannot be tested on its own (the campaign row "test CLI separately"), and the package verification could not read the usage from stdout |

---

## 5. `DEFERRED` rows and why

| Row | Status | Reason | Where the decision lives |
|---|---|---|---|
| Android arm64 APK/AAB | DEFERRED | The feasibility study finds the port feasible with bounded work but the disposition is experimental, and the campaign gates the port behind an explicit decision that has not been recorded. No native entrypoint, no NDK/SDK CI image and no ARM64 device exist. | `docs/research/android-feasibility.md` (PR #9 / #12); board task T27-46 records the gate as `READY` and T27-47 as `BLOCKED` |
| RPM | DEFERRED | No RPM spec or recipe exists; the packaging surface is DEB/AppImage/Flatpak. Re-evaluate only on demonstrated demand. | `docs/decisions/0070-rpm-deferred.md`, accepted 2026-11-06 (PR #9 / #12 / #14). **This decision record is not on `main`** — on `main` the deferral has no written authority |
| ARM64 Linux / Windows builds | DEFERRED (extended scope) | The MVP candidate lane is x86_64 only; the fleet has no ARM64 host and no ARM64 test environment. | `docs/RELEASE-ENGINEERING-PLAN.md` §11 and the board's decision D-02 — **`PROPOSED`, maintainer approval does not exist** |

The MVP scope itself (web bundle, Linux x86_64 desktop tar.gz, DEB, CLI
archive, source archive, checksums, SBOM, verified web deployment,
certification) is a **recommendation, not an approved scope**. This audit
does not treat it as approved.

---

## 6. `UNVERIFIED` claims

Recorded as unverified rather than assumed true or false:

1. **Whether `CLOUDFLARE_API_TOKEN` is configured** in the repository
   secrets. Secret values and their presence were not inspected; the
   Pages deploy step in `release.yml` is commented out and the deployment
   is manual, so the live site was updated by a human action whose
   credentials are unknown.
2. **The live deployment's correspondence to any source commit.** The
   served bundle hash `2274ac8c2925b61d` was not matched against any
   local build in this audit.
3. **Target-native runtime evidence for the DEB.** The 2026-11-06
   clean-container receipt exists only as prose inside PR #9's artifact
   matrix; no retained log, container image digest, or transcript is in
   the repository, so this audit could not re-derive it.
4. **Any Windows, macOS, AppImage or Flatpak runtime behavior.** No
   artifact of those kinds was produced, so nothing about them was
   observed. `wine` is present but was not used, and in any case wine is
   supplementary compatibility evidence, never native certification.
5. **The full `xtask ci` gate on the audited baseline.** No complete
   local sweep was run (P0.4.2). The gate's health on this baseline is
   established by CI, not by this audit: GitHub Actions run
   `41938756908` ran `xtask ci-no-fmt` green on `main` @ `5fac7a2`, with
   the `fmt` job green alongside it. What remains unverified is the
   **GitForge** copy of that gate on this SHA — no GitForge pipeline run
   exists for `5fac7a2`, the two systems run different images
   (`firecraft-ci-rust:1` vs the ubuntu-latest runner), and the GitForge
   run **cannot now be produced at all** because its shared image no
   longer carries this baseline's lockfile (B-20). The `Web bundle smoke`
   step inside the GitHub job was skipped, so no *bridge smoke* result
   attaches to that receipt.
6. **The `gate` job result for PR #8 at its current head `092a91c`** and
   for PR #12 (pending at audit time) — only the results at the heads
   listed in P0.2 are recorded here.
7. **Whether the `docs/CHANGELOG.md` regeneration is the intended
   content.** The regenerated copy was inspected and found consistent with
   `main`'s squashed history; it was left uncommitted, so the decision to
   adopt it belongs to the maintainer.
8. **Which build produced the published v0.2.5 web bundle, and what the
   release workflow's sixth staged file was.** The workflow staged 6
   content files and this audit's rebuild stages 5; the extra file is not
   named in any log and was not reproduced (the two builds used trunk
   0.22.1 and 0.21.14 respectively — F-17). The published ZIP's own
   entries are stamped 12:08 with no SHA, commit, or host recorded
   anywhere, so the operator build that produced it cannot be identified
   from the repository (B-21, F-16).
9. **Whether the published native assets came from the workflow's
   uploaded artifacts or from a separate local build.** The `build
   artifacts` job of run `41689885746` produced three native archives
   and uploaded them, and the release appeared out of band 91 seconds
   after the publish step failed; both paths are consistent with the
   published bytes, and the workflow's own manifest was never
   published, so the two cannot be distinguished from the repository
   (B-21).

---

## 7. PR disposition plan (P0.2.8)

| PR | Head | Disposition | Rationale |
|---|---|---|---|
| #9 `docs/cross-platform-release-plan` | `a0409ed` | **Merge candidate — consolidate first.** Draft status must be lifted; its `dist --source` / `dist --sbom` / Flatpak / artifact-matrix / T27 content is the only implementation of the source-archive and SBOM rows that exists | Its gate is green; but #14 duplicates the plan file and the two verbs, so merging both is a conflict (B-3) |
| #14 `feat/release-workflow-repair` | `c0c145c` | **Merge candidate — consolidate first.** It is the only branch that repairs the release workflow (B-4) and it adds `packaging/release-assets.sh` with fail-closed aggregate validation | Its gate **fails** on both runs (`42635510930`, `42635521587`), now on `[xtask] perf FAILED` / `spawn perf-child: No such file or directory` — a different step than the Aegis drift `c0c145c` fixed. The perf failure must be cleared before merge |
| #12 `docs/release-execution-board` | `49f02d4` | **Merge candidate** — documentation plus the `dist --source` / `dist --sbom` verbs | The board is the ledger this audit is written against; its gate was pending at audit time (`42636613428`, `42636626315`) |
| #8 `feat/dock-inspector-parity-matrix` | `092a91c` | Keep open; outside the release scope | Its gate is **green** at the current head (run `42633335891`); the 2 unresolved review threads are documentation scope |
| #15 `feat/phase-21-daily-driver` | `9bb43b2` | **Release-relevant — must land before the next artifact is built.** It carries the `time 0.3.47` + MSRV 1.95 bump that `main` still lacks (B-19 / F-15) | Its `gate` was still running at audit time; `fmt` is green. Its own scope (Phase 21 daily driver) is not release scope, but the dependency fix inside it is a release prerequisite |
| #13 `feat/slice-hotspot-edit` | `0cb2ce3` | Keep open; outside the release scope | Touches `xtask/src/main.rs` and `packaging/` — re-check for conflicts when #9/#12/#14 merge |
| #11 `website/marketing-site` | `62dc01d` | Keep open | Outside the release scope |
| #10 `branding/readme-refresh` | `1e24e1f` | Keep open | Outside the release scope |

**Merge order proposed:** consolidate #9 and #14 into one release branch
(they are the same feature area), then merge that, then #12, then
re-check #13. Independently of that consolidation, **#15 must land before
the next artifact is built** — it carries the `time 0.3.47` / MSRV 1.95
bump that `main` still lacks (B-19), and it changes the toolchain floor,
so the glibc floor and the reproducibility baseline have to be
re-measured on the new tree rather than carried over.

**Superseded:** §11 records the current state — #17 is now the
release-scoped carrier of the bump, PR #14 carries the offline-wall
fix (B-20), and the Aegis gate itself was defective (PR #18).

---

## 8. Documented behavior vs actual implementation (P0.3.12)

| ID | Documented | Actual | Severity |
|---|---|---|---|
| F-1 | *(withdrawn — no discrepancy found)* `docs/RELEASE-ENGINEERING-PLAN.md` §2 states "workspace package metadata uses Rust edition 2024 and `rust-version = 1.85`" | verified against `origin/main:Cargo.toml`: `edition = "2024"`, `rust-version = "1.85"` — the plan is **accurate**. Every member manifest is consistent (`crates/*` and `web/Cargo.toml` all declare `1.85` or inherit it from the workspace). The remaining gap is that the declared MSRV is never *tested*: CI installs `dtolnay/rust-toolchain@stable` (F-10). The declared floor is also already superseded on a non-`main` lineage: the `time` 0.3.47 security bump requires the floor to rise to **1.95** (F-15) | none at `5fac7a2` — recorded so the check is not repeated; but see F-15 for the value that will actually ship |
| F-2 | `docs/RELEASING.md` checklist has no source-archive or SBOM step, although the plan and the campaign both require both artifacts | the verbs do not exist on `main` at all | high |
| F-3 | `docs/RELEASING.md` / HANDOFF §6.3: publish by "push both remotes (gitforge first, then origin)" | the push to `gitforge` is rejected as non-fast-forward (B-1) | high |
| F-4 | `docs/RELEASING.md`: "Record the sha256 of every recipe-built artifact you ship in the release notes — the dist manifests cover only what the fleet built." | the sentence is about *off-fleet recipe* artifacts, none of which have ever been built or shipped, so it has never been exercised. Separately, the fleet-built side is under-served: the checksum **files** exist and do verify (2/2 lists pass against the published downloads), but no digest appears in any release note and every published asset has a `null` GitHub digest, so nothing binds the release set together at the release page | medium |
| F-5 | `.github/workflows/release.yml` header: "Creates a draft GitHub release with all 5 assets" | the release job cannot attach assets (B-4); all 5 recorded runs of the workflow failed, so it has never produced a release | high |
| F-6 | `docs/TOOLCHAINS.md` and HANDOFF §6.3 present the web bundle as a release artifact | the published `MANIFEST.sha256` covers only the 3 native archives — the web ZIP has no published checksum | medium |
| F-7 | three packaging recipes cite `packaging/TOOLCHAINS.md` | the file is `docs/TOOLCHAINS.md`; `packaging/TOOLCHAINS.md` does not exist (B-15) | low |
| F-8 | `docs/TOOLCHAINS.md`: the DEB is "verified by the dist gate itself (`dpkg-deb --info`/`--contents`)" | the gate verifies name, version and binary presence but **not** the declared glibc floor against the binaries' real requirement (B-7). Measured 2026-11-09: declared 2.31, required 2.39 | high |
| F-9 | the campaign and the release plan list `xtask dist --source` and `xtask dist --sbom` as existing verbs | on `main`, `xtask dist` accepts only `--web` and `--native` | high |
| F-10 | the release procedure assumes a reproducible toolchain | no `rust-toolchain.toml`; CI installs `stable` at run time, so the compiler that produced the published artifacts is not pinned in the repository | medium |
| F-11 | `docs/RELEASING.md`: "`xtask changelog` … Refresh and commit the copy at release time; CI runs the verb (it re-verifies licenses/versions on the runner) but does not byte-check the file" | the verb **writes** the file, and it is a step of the canonical gate, so every gate run rewrites a tracked document (B-16) | medium |
| F-12 | the campaign's live task ledger is `docs/TASKS.md` | `docs/TASKS.md` on `main` carries no T27 row; the campaign board exists only on PR #12 | medium |
| F-13 | the campaign mandate is a repository document | `docs/planning/release-compaign/Cross-Platform Releas.txt` is **untracked** in the operator's checkout — the mandate for this campaign is not in version control | medium |
| F-14 | the release plan treats the source archive and SBOM as MVP-lane deliverables | neither has ever been published, and the publishing workflow (B-4) cannot carry them | high |
| F-15 | the audited baseline `main` is what gets built and shipped, and nothing in the release procedure checks the dependency set for security advisories | `origin/main:Cargo.lock` pins **`time 0.3.45`**. The repository contains commits `c74b179` and `e9747b8` — "build: raise MSRV to 1.95 and bump time past CVE-2026-25727" — recording that `time` 0.3.45 is affected by a Dependabot **stack-exhaustion denial of service** requiring `time >= 0.3.47`, and that the fix additionally forces the workspace `rust-version` from `1.85` to `1.95` (`time` 0.3.47 needs 1.88, and cargo's MSRV-aware resolver floors on the *lowest* `rust-version` anywhere in the workspace, so the lockfile silently slid back to 0.3.45 until the floor moved). Those commits live on `feat/dock-inspector-parity-matrix` and `feat/phase-21-daily-driver` (pushed to origin **and** gitforge, 23–29 commits ahead of `main`) — **not on `main`**. Every published v0.2.5 artifact and the audited baseline therefore carry the flagged version. The advisory is confirmed independently of the commit message: GitHub's Dependabot API reports alert **#1, state `open`, severity `medium`**, package `time` in `Cargo.lock`, `GHSA-r6v5-fh4h-64xc` / `CVE-2026-25727`, summary "time vulnerable to stack exhaustion Denial of Service attack", vulnerable range `>= 0.3.6, < 0.3.47`, first patched version `0.3.47`; `main`'s `0.3.45` is inside that range and the alert is open against the default branch | high |
| F-16 | the release workflow is the thing that publishes the web bundle, and `docs/RELEASING.md` treats the release job's output as the released artifact | the workflow's own web build (21,231,517 B wasm, 6 staged content files) is **not** the bundle in the published ZIP (21,238,451 B wasm, 5 content files + manifest, entries stamped 12:08 — six hours before the workflow's build finished at 18:15:59Z). The `create draft release` job died at 18:16:19Z and the release appeared at 18:17:49Z with no job account for it. The documented release path and the actual publication path are different code paths | high |
| F-17 | the release build is hermetic and the wasm size ceiling is meaningful | `release.yml` installs the bundler with `cargo install trunk --locked` and no version constraint, so the v0.2.5 release build ran **trunk v0.22.1** while the audited host has **0.21.14** and the size-ceiling comment in `xtask/src/dist.rs` names "wasm-opt 132" for a bundle that measures 21,235,599 B. Four builds of nominally the same release differ by 42,806 B and no receipt records the bundler version (F-16, B-18) | medium |

---

## 9. What this audit does not claim

- It does not claim that any release is certified. Nothing reached
  `CERTIFIED`; nothing reached rung 9, because nothing is signed and no
  provenance exists.
- It does not claim that the unmerged branches' content is wrong. Where a
  row's evidence comes from PR #9 / #12 / #14, the branch and SHA are
  named.
- It does not claim the published artifacts work on a user's machine.
  The five published artifacts were downloaded, checksum-verified and
  (for the CLI) launched, on this host only — a Debian 13 / glibc 2.41
  x86_64 machine. Nothing was tested on any other platform, and the
  desktop binary's `GLIBC_2.39` requirement means it will not run on
  distributions older than that at all (B-7).
- It does not claim the campaign board's rung marks are wrong. Where this
  audit differs from `docs/release/EXECUTION-BOARD.md` — the DEB's
  install/launch/smoke rungs, the Windows ZIP's build rung, the Flatpak
  row — the difference is stated with its evidence in §2 and §4.
- It does not fabricate a build result. Builds that were not run are not
  `BUILD VERIFIED`.
- It does not claim the published artifacts were produced by the release
  workflow. They were not: the workflow's publish step has failed on
  every release on record (B-4), and for v0.2.5 the shipped web bundle
  is demonstrably **not** the workflow's own build (B-21, F-16). This
  audit verifies what the published bytes *are*; it does not vouch for
  how they got there.
- It does not claim the live deployment corresponds to any release. Four
  distinct web bundles are in play — the published v0.2.5 one, the one
  the live site serves, the one the audited tree's `web/dist` held, and
  the one this audit rebuilt — and this audit does not attempt to say
  which commit, if any, produced the deployed bytes.

---

## 10. Secret-scanner findings adjudicated

The Aegis 0.6.3 production scan of this tree returns **four** findings
for this document. All are `zip-code` (category `pii`, severity `low`),
and all fire on the same thing: the five-digit suffix of the CVE
identifier quoted in §4 B-19, §8 F-15 and §11.1. The scanner anchors
one match per source line, so the count is the number of lines that
quote the identifier — four, even though the identifier appears six
times.

| Location | Pattern | Adjudication |
|---|---|---|
| §4 B-19 | `zip-code` | **False positive.** The scanner reads the trailing digit group of a security-advisory identifier as a US ZIP code. The text is an advisory identifier, not a postal address, and the identifier is quoted verbatim from the GitHub Dependabot alert and from the repository's own commit message. **Not reworded**: altering the digits would misquote the identifier, which is worse than baselining it. |
| §8 F-15 | `zip-code` | same — same identifier, same reason |
| §11.1 (two lines) | `zip-code` | same — the identifier is quoted again when the reconciliation names the advisory |

The entries are recorded in `.aegis-baseline.tsv`, keyed on the
matched literal's content digest rather than on a line number, with a
count of four. That file was regenerated on 2026-11-09 with the
sanctioned recipe — a **full, unfiltered** production scan of the tree,
never a filtered one — and the regeneration was diffed against the
committed baseline: it **adds exactly these entries and removes none**.
The scanner's other categories — payment-card, personal-information and
loopback-address rules — do not fire anywhere in this document.

---

## 11. Reconciliation — 2026-11-09, after the audit was written

This section supersedes §4 and §7 where they disagree. It records
what changed after the audit was taken, with the evidence for each
claim.

### 11.1 B-19 — the security bump now has a release-scoped carrier

The audit found `main` pinning `time 0.3.45`, inside the vulnerable
range of CVE-2026-25727, with the remediation living only inside
feature branches. It now lives in three places:

| Branch | PR | Head | `time` | Scope |
|---|---|---|---|---|
| `fix/time-cve-2026-25727` | **#17** | `f7470f8` | `0.3.47` | `Cargo.lock`, the six `Cargo.toml` manifests, `crates/io/src/jpeg.rs`, `docs/PLANNING.md` — the bump alone, based on `main` |
| `feat/phase-21-daily-driver` | #15 | `9bb43b2` | `0.3.47` | the bump **plus** the whole Phase 21 feature |
| `feat/dock-inspector-parity-matrix` | #8 | `092a91c` | `0.3.47` | the bump plus the shell-fidelity work |

**#17 is the carrier to land.** It is the only one of the three whose
diff is the dependency fix and nothing else, so landing it does not
drag a feature into the release path. After #17 lands, #15 and #8 must
rebase: both carry the same `Cargo.lock` change, so a merge of either
onto the new `main` conflicts on the lockfile and on the six manifests.
The rebase should **drop** their copy of the bump and keep their own
scope.

B-19 itself therefore moves from "the remediation does not exist on
`main`" to "the remediation exists, is reviewable on its own, and is
waiting on a merge". It closes when #17 is merged, and the toolchain
floor, the glibc floor (B-7) and the reproducibility baseline (B-18)
must be re-measured on the new tree — not carried over.

### 11.2 B-20 — the offline wall is in the repository, not in the image

The audit attributed B-20 to the runner image's cargo cache. That
attribution was wrong in its cause, and the correction matters for the
fix. The cache is a *consequence*: the effective offline setting is
baked into the image **built from the repository's own Dockerfile**,
`infrastructure/docker/ci-rust.Dockerfile`, which on `main` carries
`ENV CARGO_NET_OFFLINE=true` (line 108). Every branch therefore
rebuilds the sandbox with the offline wall unless it changes that
file. The runner host's two tags are the two states of that file:

```
firecraft-ci-rust:1   ENV CARGO_NET_OFFLINE=false   (built 2026-11-09 08:54)
firecraft-ci-rust:2   ENV CARGO_NET_OFFLINE=true    (built 2026-11-09 08:54)
```

Both images carry `time-0.3.41`, `time-0.3.47` and `time-0.3.55` in
their registry cache, and neither carries `time-0.3.45` — which is why
a `main`-lockfile branch fails in the offline image and succeeds in the
online one.

**PR #14 carries the fix.** Its `ci-rust.Dockerfile` sets
`CARGO_NET_OFFLINE=false` (line 130) and its `.gitforge.yml` drops the
comment block that described the wall as a feature. The proof is
GitForge run **`9acd8b5a`** (commit `aefe5ac`, started
2026-11-09T17:02Z, finished 20:29Z, **succeeded**): all eight
jobs passed, and `fmt-layers` logged `Downloading crates ... Downloaded
time-core v0.1.7` — the sandbox fetched, which it cannot do under the
wall. That run pins `time 0.3.45`, the same version `main` pins.

So B-20's remediation is a two-line repository change, not an image
rebuild: the image no longer needs to be re-shipped when a lockfile
moves. **Until PR #14 lands, `main` still cannot be validated on
GitForge** — its own Dockerfile keeps the wall in place.

### 11.3 B-21 — still open, and now owned

Nothing in this session changes B-21: the published v0.2.5 web bundle
still has no build receipt, and the release was still published by a
path the workflow does not contain. PR #14's
`packaging/release-assets.sh` and its fail-closed aggregate validation
are the mechanism that stops the *next* release from repeating the
hand-publish, so B-21 stays open until that workflow has produced a
release with a receipt attached.

### 11.4 The Aegis gate itself was defective

Four PRs (#10, #11, #14, #17) are red on a gate that is broken, not on
their own content. Aegis's `--baseline` filter matches on
`pattern:file:line:digest`, so any prose edit that shifts a line makes
a baselined finding reappear as new; the repository's own `.aegisignore`
had already recorded the symptom. PR #18 (`fix/aegis-content-keyed-
baseline`) replaces the filter with a gate that keys a finding on
`(pattern, file, content digest)` and compares multisets itself, so a
moved finding stays the same finding while a new literal, a new file
or an extra copy of a known literal still fails. No rule class is
switched off: every rule the scanner has stays armed, and each
baselined occurrence carries the reason it was adjudicated.

This changes the PR dispositions above in one respect: the red gates
on #10, #11, #14 and #17 were not evidence against their content. They
are re-run against the corrected gate.

### 11.5 Merge order

1. **#18** — the corrected gate. Every other PR's aegis result depends
   on it, and it changes no behaviour other than finding identity.
2. **#17** — the security bump on its own (B-19). Nothing else may
   build an artifact before it lands.
3. **#14** — the release-workflow repair and the offline-wall fix
   (B-20, B-4).
4. **#9 / #12** — consolidate the plan, the board and the two `dist`
   verbs into one release branch (B-3), then merge it.
5. **#15**, then **#8** — rebase onto the new `main` and drop their
   duplicate copies of the bump (§11.1).
6. **#13**, **#11**, **#10** — outside the release scope; merge after
   the rebase, re-checking the `xtask/src/main.rs` overlap that #13
   and #14 share.

### 11.6 Linux distribution completion — B-24, B-25, B-26, B-27

The Phase 3 (RP-01) DEB rows could not be closed by inspection, so the
package was installed and exercised in a clean Debian 12 container.
That produced four findings, all recorded above: **B-24** (the deb
ships no desktop integration), **B-25** (the Flatpak recipe's launch
step is a command flatpak rejects), **B-26** (the Flatpak manifest
ships no icon and names a desktop entry that no longer exists), and
**B-27** (`firecraft-cli` has no `--help`). The numbering starts at
B-24 because B-23 — "the release job's tag gate does not gate" — is
recorded in the `ci/web-smoke-real-v2` branch (PR #40).

**The negative control.** Before any fix, the verification script was
run against a *published* package:

```
$ docker run --rm -v /var/tmp/debtest:/t firecraft-ci-rust:1 \
      bash /t/verify-install.sh /t/firecraft_0.2.3_amd64.deb
== candidate: /t/firecraft_0.2.3_amd64.deb (version 0.2.3, arch amd64)
== depends:   libc6 (>= 2.31)
== stage 1: install
PASS: installed firecraft 0.2.3
== stage 2: desktop entry
FAIL: /usr/share/applications/firecraft.desktop is not installed
```

The package installs cleanly and then fails the desktop-entry stage.
That is the defect, measured on the artifact users download — not on a
rebuild — and it is why the check exists: a package can satisfy every
package-manager check and still not be a desktop application.

**The fixes.**

- The deb payload now carries `usr/share/applications/firecraft.desktop`
  and `usr/share/icons/hicolor/256x256/apps/firecraft.png`, and
  `dist --native` reads the built package back with `dpkg-deb
  --contents` and fails when either is missing. The check is on the
  package contents, not on the staging directory.
- The desktop entry moved to one canonical path,
  `packaging/desktop/firecraft.desktop`, shared with the AppImage
  recipe, so the two formats cannot drift apart.
- The Flatpak recipe installs the bundle into a user installation,
  launches it with `flatpak run --command=firecraft-cli`, removes it
  again and fails if the application is still installed afterwards;
  `flatpak-builder` gained `--share=network` for the cargo build.
- The Flatpak manifest installs the icon and points at the canonical
  desktop entry; its `finish-args` comment now states what the
  filesystem permission actually grants.
- `firecraft-cli --help` prints the usage on stdout and exits
  successfully, with an integration test that runs the built binary.

**What is still unverified.** The Flatpak and AppImage builds need
`flatpak`/`flatpak-builder`/`linuxdeploy`, which this fleet does not
carry (B-8, B-11) — those recipes are corrected but their runtime
stages remain unexecuted, and their campaign rows stay open with that
exact reason. The deb's own clean-container receipts are recorded in
`docs/release/ARTIFACT-MATRIX.md` and in the PR that carries this
work.
