# P0 — Base camp: a clean trunk

**Goal.** Every open PR is either landed or has a recorded reason; stale
branches and issues are pruned; CI spends its minutes on code; the
scoreboard exists as one command.
**Exit check:** S1 green · S2 ≥ 196 · S11 ≤ 55 · S12 ≤ 4 · `xtask climb`
prints every metric.
**Lane:** trunk. Nothing else starts until C0.01–C0.08 are `[x]`.

---

### C0.01 — Climbing gear: confirm the toolchain `[ ]`
- **Needs:** — · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. `rustc --version` must print `1.99.1`; CI pins it in
     `infrastructure/docker/ci-rust.Dockerfile` and the GitHub workflows.
     If it doesn't, run inside the CI image `firecraft-ci-rust:2`; the build
     recipe is in the Dockerfile header.
  2. Check that `target/` resolves (AGENTS.md: a symlink to
     `/nas/Dev/Targets/firecraft-target`).
  3. Run `cargo run -p xtask -- ci` on `main`. Record pass/fail and the wall
     time in the PR body.
- **Done when:** the gate's result on `main` with the pinned toolchain is
  recorded.
- **Verify:** the gate output is pasted (last 30 lines).
- **Moves:** S1 is confirmed locally, not just on GitHub's backup CI.

### C0.02 — Fix PR #143's red test and land it `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` `[MAINTAINER]` (merge) ·
  **Size:** S
- **Steps:**
  1. `git fetch origin && git checkout feat/selection-command-routing`.
  2. `cargo test -p app the_markup_verbs_restyle_the_runs`. It is expected to
     fail at `crates/app/src/shell/mod.rs:6935`, got `None`, expected
     `Some(false)`.
  3. `git show 47886aa`. This is the CodeRabbit autofix commit at the branch
     head. If its diff touches the markup/restyle path, `git revert 47886aa`.
     Otherwise fix the code so the test's documented expectation holds. Do
     **not** edit the expectation unless decision 0109 says it is wrong.
  4. `git merge origin/main` (no rebase or force-push); resolve conflicts;
     `cargo run -p xtask -- ci`.
  5. `cargo run -p xtask -- reachability`. Expect 196/253; then
     `cargo run -p xtask -- reachability --bless`.
  6. Push, and ask the maintainer to merge.
- **Done when:** #143 is merged and `main`'s floor file reads
  `shell_bound_commands 196`.
- **Verify:** `tail -4 .reachability-floor.tsv` on `main`.
- **Moves:** S2 168 → 196.

### C0.03 — Land PR #113 (mask stack) `[ ]`
- **Needs:** C0.02 · **Tags:** `[HOST:linux]` `[HOST:browser]`
  `[MAINTAINER]` · **Size:** S
- **Steps:**
  1. `git checkout feat/mask-stack-ui-v2 && git merge origin/main`.
  2. Resolve conflicts. Six files overlap with #143: `panels.rs`,
     `render.rs`, `mask.rs`, the fwmap reader and its tests, and `TASKS.md`.
     In each conflict, keep both sides' additions.
  3. Run `cargo run -p xtask -- ci` and the HANDOFF §6.2 browser proof on the
     Layers panel.
  4. Run `cargo run -p xtask -- reachability`. If any `mask.*` command from
     the AUDIT §7 list became bound, `--bless`.
  5. Ask the maintainer to merge. Leave issue #114 open (screenshot
     acceptance).
- **Done when:** #113 is merged; issue #76 is closed with the merge SHA.
- **Verify:** `git log origin/main --oneline | grep -i mask`.
- **Moves:** S5a +1 (mask stack row), S2 ≥ 196.

### C0.04 — Land PR #142 with corrected numbers `[ ]`
- **Needs:** C0.03 · **Tags:** `[DOCS]` `[MAINTAINER]` · **Size:** XS
- **Steps:**
  1. `git checkout docs/reachability-gap-and-coverage-measurement && git merge origin/main`.
  2. In `docs/GAPS.md` §16, replace the "85 unbound" figures with the
     measured post-#143/#113 counts from `xtask reachability`, and list the
     remaining command IDs as in AUDIT §7.
  3. Run the gate, then ask for the merge.
- **Done when:** #142 is merged with numbers matching `.reachability-floor.tsv`.
- **Verify:** the numbers in `grep -n unreachable docs/GAPS.md` equal
  `253 - shell_bound_commands`.
- **Moves:** — (truthfulness).

### C0.05 — Land PR #144 (approved artwork) `[ ]`
- **Needs:** C0.02 · **Tags:** `[HOST:browser]` `[MAINTAINER]` · **Size:** XS
- **Steps:** merge `origin/main` in. Resolve against #132 (fox motif
  retired) by keeping #132's removals plus #144's flame assets. Check that
  every asset has an `ATTRIBUTION.md` row. Load `dist/web` and the `site/`
  page in a browser and look at the icon and favicon.
- **Done when:** #144 is merged.
- **Verify:** `gh pr view 144 --json state`.
- **Moves:** —.

### C0.06 — Salvage `fix/web-ux-round1` `[ ]`
- **Needs:** C0.02 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. `git show b49afe2 --stat` and `git show b49afe2`. It contains:
     - `run_command_checked` (surface paint-command failures);
     - raster import keeping artwork;
     - a dist path fix.
  2. For each of the three changes, check whether `main` already has the
     behaviour (`git grep run_command_checked`; reproduce the import case in
     a test).
  3. Port only what is still missing, on a new branch
     `climb/C0.06-web-ux-salvage`, with a regression test for each ported
     fix.
  4. If nothing is missing, write that, with evidence, in the PR body and
     close the old PR #5 reference.
- **Done when:** each of the three changes is either on `main` with a test or
  recorded as obsolete.
- **Verify:** `cargo test -p app <new test names>`.
- **Moves:** S6 ↑ if ported.

### C0.07 — `xtask reachability --list` `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. In `xtask/src/reachability.rs`, `run_inner` already builds
     `unreachable: Vec<String>` but never prints it.
  2. Add a `--list` arm in `xtask/src/main.rs` beside `--bless` that prints
     one ID per line to **stdout**, sorted, so the output can be piped.
  3. Add a unit test.
- **Done when:** `cargo run -q -p xtask -- reachability --list | wc -l` equals
  `253 - shell_bound_commands`.
- **Verify:** that command.
- **Moves:** S6 +1. Unblocks P5.

### C0.08 — Build the altimeter: `xtask climb` `[ ]`
- **Needs:** C0.07 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. Create `xtask/src/climb.rs`, and register `Some("climb")` in
     `xtask/src/main.rs`.
  2. Compute S2–S8 and S12 **in Rust from the repo**, reusing the existing
     modules: `reachability::measure`, the menu census used by `menus` tests
     (or a count of `ReportGap(` in `crates/app/src/menus.rs`), the
     CS6-PARITY status grep, the `#[test]` count, and the file-length scan.
     For S8, read the size from `dist/web/*.wasm` if present; otherwise
     print `n/a`.
  3. Print a TSV row in the column order of `docs/climb/SCOREBOARD.tsv` to
     stdout.
  4. `climb --check` compares the measurement with the **last row** of
     `SCOREBOARD.tsv` and exits non-zero if any ↑ metric fell or any ↓
     metric rose. The direction of each metric is in the README table.
  5. `climb --append <rung-id>` appends the row.
  6. Add `climb --check` as a step in `xtask ci`, after reachability.
  7. Write unit tests for each metric parser and for the direction check.
- **Done when:** `cargo run -q -p xtask -- climb` prints a row matching the
  baseline (allowing for merged PRs), and `--check` fails on a synthetic drop
  in a test.
- **Verify:** `cargo test -p xtask climb`; `cargo run -p xtask -- ci`.
- **Moves:** every later rung becomes machine-checked.

### C0.09 — Prune stale branches and stray tags `[ ]`
- **Needs:** C0.02–C0.05 · **Tags:** `[MAINTAINER]` · **Size:** XS
- **Steps:**
  1. Re-run the audit check for each branch in AUDIT §2's DELETE rows:
     `git rev-list --left-right --count origin/main...origin/<b>` and
     `gh pr list --state all --head <b>`.
  2. Post a single issue titled "Climb C0.09: branch prune". It should hold
     the verified list plus the exact commands:
     - `git push origin --delete <b>` for each branch;
     - the same for the `gitforge` remote;
     - `git push origin :refs/tags/v0.2.5-dryrun.{1,2,3}`.
  3. Leave `release/v0.2.5` to the maintainer's choice. **Do not run the
     deletes yourself.**
- **Done when:** the maintainer has run them, or declined in writing.
- **Verify:** `git fetch --prune && git branch -r | wc -l`.
- **Moves:** S12 34 → ≤ 4.

### C0.10 — Issue hygiene `[ ]`
- **Needs:** C0.02–C0.05 · **Tags:** `[MAINTAINER]` · **Size:** S
- **Steps:**
  1. For each "Close: already done" issue in AUDIT §5, verify the cited
     evidence: run the named test, or `grep` the cited file.
  2. Draft a closing comment with the SHA and test name.
  3. Draft the duplicate pointers (#75 → #24, #80 → #28/#29, #3 →
     #96/#59/#60).
  4. Draft residual-narrowing edits for #23, #24, #30 and #97, and epic
     conversions for #31–#34.
  5. Draft the transfer note for #37 (it belongs to the gitforge-runner
     repo).
  6. Post everything as one checklist issue, "Climb C0.10: issue hygiene",
     for the maintainer to apply. **Do not close issues yourself** unless the
     maintainer has said in writing that agents may.
- **Done when:** the maintainer has applied or declined each line.
- **Verify:** `gh issue list --state open --limit 500 --json number | jq length`.
- **Moves:** S11 70 → ≤ 55.

### C0.11 — Marketing-site deploy: fix it or switch it off `[ ]`
- **Needs:** — · **Tags:** `[MAINTAINER]` (secrets) · **Size:** XS
- **Steps:**
  1. `.github/workflows/deploy-site.yml` fails because
     `CLOUDFLARE_API_TOKEN` is unset.
  2. Agent part: make the deploy step conditional on the secret's presence
     (`if: ${{ secrets.CLOUDFLARE_API_TOKEN != '' }}` via an env indirection),
     and emit a `::warning::` when it is absent, so `main` stops showing red
     for a missing secret.
  3. Maintainer part: add `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`
     to the repo secrets, and confirm the Pages project `firecraft-site`
     exists. The DNS for `firecraft-site.pages.dev` does not resolve today.
- **Done when:** the workflow is green, with a deploy or with a warning.
- **Verify:** `gh run list --workflow deploy-site.yml --limit 1`.
- **Moves:** S1 — no red workflows on `main`.

### C0.12 — Run the GitForge `release` job only on tags (#126) `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. In `.gitforge.yml`, restrict the `release` job (180-minute watchdog) to
     tag refs `v*` and manual triggers. Follow the conditional style the
     file already uses for other jobs.
  2. Leave the `wasm`, `fmt`, `clippy-test` and `corpus` jobs untouched.
  3. Record the before/after job list for a branch push in the PR.
- **Done when:** a branch push pipeline has no `release` job, and a tag
  pipeline still has it.
- **Verify:** the GitForge pipeline view for one branch push (run ID in the
  PR).
- **Moves:** CI throughput. Close #126 with the run ID.
