# P1 — One source of truth

**Goal:** an agent can learn "what is true" and "what is next" from two
files, `docs/TASKS.md` (status) and `docs/climb/` (ordered work), without
reconciling eight others.

**Exit check:**
- every fact in AUDIT §4 is fixed;
- frozen docs carry a banner;
- decision 0075 has a maintainer verdict.

**Lane:** trunk. All rungs are `[DOCS]` unless tagged otherwise. These rungs
can run in parallel with P2.

---

### C1.01 — Point every entry doc at the climb `[ ]`
- **Needs:** C0.08 · **Size:** XS
- **Steps:**
  1. Edit `AGENTS.md` (§Workflow, step 1). Replace "Take active work from
     `docs/TASKS.md`. Use `docs/NEXT.md`…" with: "Take the next rung from
     `docs/climb/README.md` (the climbing loop); `docs/TASKS.md` is the
     status board."
  2. Add a `docs/climb/` row at the top of the map in `docs/README.md`.
  3. Add a one-line pointer to `docs/climb/` in the `docs/TASKS.md` header
     quote.
- **Done when:** all three files point at the climb, and no file tells an
  agent to pick work from NEXT.md.
- **Verify:** `git grep -n "NEXT.md" AGENTS.md docs/README.md docs/TASKS.md`
  has no "take work from" wording.
- **Moves:** —

### C1.02 — Freeze the superseded plan documents `[ ]`
- **Needs:** C1.01 · **Size:** XS
- **Steps:**
  1. Add this banner as the first line under the title of each of these
     files: `docs/NEXT.md`, `docs/GAPS.md`,
     `docs/PRODUCTION-READINESS-PLAN.md`,
     `docs/RELEASE-ENGINEERING-PLAN.md`, `docs/release/EXECUTION-BOARD.md`,
     and `docs/HANDOFF.md` (§4 only, as a note under the §4 heading):
     `> **Frozen 2026-11-13.** Historical plan. Ordered work: docs/climb/. Live status: docs/TASKS.md. Do not update this file's status.`
  2. Change no other content in those files.
- **Done when:** all six banners are present.
- **Verify:** `git grep -l "Frozen 2026-11-13" docs | wc -l` = 6.
- **Moves:** —

### C1.03 — Fix the stale facts `[ ]`
- **Needs:** C0.02 · **Size:** S
- **Steps:**
  1. For each row in AUDIT §4 except the `CLAUDE.md` row and the decision
     numbers, change the stale text to the current truth. Take each value
     from a measurement, not from this file:
     - `git rev-parse --short origin/main`;
     - `cargo run -q -p xtask -- reachability`;
     - `gh pr view 118 --json state`.
  2. Delete the pasted CodeRabbit markers at `NEXT.md:557`,
     `HANDOFF.md:193`, `CS6-PARITY.md:176` and `PROGRESS.md:10`. Search
     `git grep -n "CodeRabbit\|coderabbit" docs` for others.
  3. In `docs/TASKS.md`, delete the queued T18 row that says
     "RECONCILED — REPUBLISH PENDING". The in-flight row with the digest
     measurement is the truth.
  4. In the same file, change the 22.1 row "the merge waits on that branch's
     receipts" to "landed in #128".
- **Done when:** every row in AUDIT §4 (except `CLAUDE.md` and the decision
  numbers) is fixed.
- **Verify:** `git grep -nE "5fac7a2|b61f198|243 commands|245 commands|v0\.2\.2" docs README.md`
  returns only frozen or historical contexts (SESSIONS, HISTORY, CHANGELOG).
- **Moves:** —

### C1.04 — De-duplicate decision numbers `[x]`
- **Needs:** C0.04 · **Size:** XS
- **Steps:**
  1. Two files were numbered 0101:
     - `0104-brand-mark-in-the-mock-up.md` (renamed from 0101; the older
       `0101-off-fleet-packaging-recipe-audit.md` keeps its number)
     - `0101-off-fleet-packaging-recipe-audit.md`
  2. Keep the older one, judged by
     `git log --diff-filter=A --format=%ad -- <file>`. Rename the other with
     `git mv` to the next free number after the open PRs' 0103–0112.
  3. Update every reference: `git grep -n "0101-"`.
  4. Do the same for the doubly-cited 0096 (`NEXT.md:448` vs `:470`): find
     which text is wrong and correct the citation.
- **Done when:** `ls docs/decisions | cut -c1-4 | sort | uniq -d` prints
  nothing.
- **Verify:** that command, plus a link check
  (`git grep -n "decisions/0101"` all resolve).
- **Moves:** —

### C1.05 — Correct CLAUDE.md's status paragraph `[ ]`
- **Needs:** C1.03 · **Tags:** `[MAINTAINER]` (CLAUDE.md is protected) ·
  **Size:** XS
- **Steps:**
  1. Prepare a replacement for the `**Status (…)**` paragraph at
     `CLAUDE.md:9`. It must state:
     - the latest tag;
     - `main`'s SHA;
     - the measured command count and reachability (from `xtask climb`);
     - that Phases 20/21 have landed;
     - "ordered work: `docs/climb/`".
  2. Post the replacement as a PR comment and wait for the maintainer to
     unlock or apply it.
- **Done when:** `CLAUDE.md` no longer says 245 / 5fac7a2 / "Phase 21
  queued".
- **Verify:** `grep -n "Status" CLAUDE.md`.
- **Moves:** —

### C1.06 — Get a verdict on the release scope (decision 0075) `[ ]`
- **Needs:** — · **Tags:** `[MAINTAINER]` · **Size:** S
- **Steps:**
  1. Read `docs/decisions/0075-release-scope-mvp.md`, `docs/release/ARTIFACT-MATRIX.md`
     and AUDIT §8.
  2. Write a one-page recommendation as a comment on issue #43. Recommended
     v1 scope:
     - **In:** web bundle, Linux x86_64 tar.gz + .deb + AppImage, Windows
       x64 zip + Inno installer, macOS universal .dmg, CLI archives, source
       tarball, SBOM, SHA-256 list, build provenance.
     - **Deferred** (decision records exist or get one): Flatpak, RPM
       (0070), Linux/Windows ARM64, Android.
     - **Signing:** in scope only if the maintainer provides identities;
       otherwise unsigned with a documented SmartScreen/Gatekeeper note.
  3. Point out that GitHub-hosted runners remove the "off-fleet" hardware
     blocker.
- **Done when:** the maintainer approves or amends, and decision 0075's
  status line is updated to *accepted* with the date.
- **Verify:** `grep -n "Status" docs/decisions/0075-release-scope-mvp.md`.
- **Moves:** unblocks P9.

### C1.07 — Slim TASKS.md to a status board `[ ]`
- **Needs:** C1.03 · **Size:** S
- **Steps:**
  1. Restructure `docs/TASKS.md` into four short sections:
     - **Now** (claimed rungs, by climb ID + PR);
     - **Blocked** (`[!]` rungs + maintainer queue);
     - **Recently landed** (last 15, SHA + rung);
     - **Ledger rules**.
  2. Move the long prose rows (Phase 22 detail, T27 detail) verbatim to
     `docs/history/TASKS-2026-11-13.md`, and link to it.
  3. Keep the release-status vocabulary paragraph, since P9 uses it.
- **Done when:** `docs/TASKS.md` is ≤ 120 lines and every in-flight item names
  a climb ID.
- **Verify:** `wc -l docs/TASKS.md`.
- **Moves:** —
