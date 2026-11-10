# T27-04 — Audit the dist implementation, packaging recipes, workflows and version stamps

> **RP:** RP-00 · **Priority:** P0 · **Status:** IN PROGRESS
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** none

**Objective:** map every dist subcommand, packaging recipe, CI workflow and version
stamp to exactly what it does and — more importantly — what it does not cover, so no
later row assumes coverage that does not exist.

**Inputs:** `xtask/src/dist.rs`; `xtask/src/main.rs`; `packaging/` (deb, flatpak,
appimage, windows, macos); `.github/workflows/ci.yml`; `.github/workflows/release.yml`;
`.gitforge.yml`; the eight version stamps.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §3.1–3.6 (per-platform requirements)
and §5.1 (job graph); `docs/release/ARTIFACT-MATRIX.md`; `docs/TOOLCHAINS.md`.

**Acceptance criteria:** every dist subcommand, recipe, workflow job and version stamp
is listed with its actual coverage and its uncovered cases; each recipe's
parse-checkability on this fleet is recorded.

**Verification:** `cargo run -p xtask -- dist --help; ls packaging; grep -rn 'uses:' .github/workflows/`

**Evidence:** `docs/release/ARTIFACT-MATRIX.md`; `xtask/src/dist.rs` (1225 lines);
`packaging/flatpak/dev.firecraft.FireCraft.yml` sources `type: dir, path: ..`;
`.gitforge.yml:168` sets `FIRECRAFT_DIST_SKIP_SMOKE=1` for the release job.

**Blockers:** none.

**Stopping condition:** the mapping exists and every downstream row's "Evidence" and
"Blockers" columns can be traced back to it.
