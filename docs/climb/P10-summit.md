# P10 — Summit: FireCraft 1.0

**Goal:** a tagged 1.0 that meets every summit target in the README
scoreboard. The release is built, proven and published, and the live web
app runs exactly that tag.

**Lane:** trunk. This phase starts only when every earlier phase's exit
check holds. Run `xtask climb` and compare against the README targets.

---

### C10.01 — Summit readiness review `[ ]`
- **Needs:** P0–P9 exit checks · **Tags:** `[DOCS]` · **Size:** S
- **Steps:**
  1. Run `cargo run -q -p xtask -- climb`.
  2. For each metric S1–S13, write the measured value next to its summit
     target in `docs/climb/SUMMIT-REVIEW.md`.
  3. List every `[!]` and `[-]` rung across `docs/climb/P*.md`
     (`git grep -n "\[!\]\|\[-\]" docs/climb`), with its decision link.
  4. If any target is missed, add the rungs needed to close it to the right
     phase file, and stop here.
- **Done when:** every target is met, or has a maintainer-approved
  exception in the review.
- **Verify:** the review doc.
- **Moves:** —

### C10.02 — Stamp the version `[ ]`
- **Needs:** C10.01 · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. Run `cargo run -p xtask -- stamp 1.0.0`. It sets every version stamp;
     HANDOFF §6.3 lists them.
  2. Run `cargo run -p xtask -- stamps` to check that they all agree.
  3. Run `cargo run -p xtask -- changelog --cut 1.0.0`.
  4. Run the gate.
- **Done when:** the stamps agree and the changelog has a 1.0.0 section.
- **Verify:** `cargo run -p xtask -- stamps`; `cargo run -p xtask -- ci`.
- **Moves:** —

### C10.03 — Release candidate on a disposable tag `[ ]`
- **Needs:** C10.02 · **Tags:** `[MAINTAINER]` (tag) · **Size:** S
- **Steps:**
  1. The maintainer pushes `v1.0.0-rc.1`.
  2. The full release pipeline runs: every P9 job, the smoke jobs,
     attestations and the draft release.
  3. Collect every receipt into `docs/release/receipts/`.
  4. On any failure, fix it on `main` and repeat with `rc.N+1`.
- **Done when:** one rc run is fully green with every receipt.
- **Verify:** the run URL plus the receipts.
- **Moves:** S10 = the in-scope count.

### C10.04 — Exploratory parity pass `[ ]`
- **Needs:** C10.03 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Take the rc web build and the rc desktop build, and run
     `docs/CS6-PARITY.md`'s *implemented* rows as a manual checklist (one
     action per row).
  2. File an issue for each failure.
  3. A failure that causes data loss or a crash blocks 1.0; anything else
     becomes a post-v1 issue.
- **Done when:** the checklist results are committed, and the blocking count
  is 0.
- **Verify:** the results file.
- **Moves:** —

### C10.05 — Publish 1.0.0 `[ ]`
- **Needs:** C10.04 · **Tags:** `[MAINTAINER]` · **Size:** XS
- **Steps:**
  1. The maintainer tags `v1.0.0`.
  2. The pipeline creates the draft.
  3. The maintainer reviews the assets against `firecraft-1.0.0-SHA256SUMS`
     and publishes, following C9.22's checklist.
- **Done when:** the GitHub release is published with every in-scope asset.
- **Verify:** `gh release view v1.0.0 --json assets`.
- **Moves:** the matrix rows → `PUBLISHED`.

### C10.06 — Deploy the web app from the tag `[ ]`
- **Needs:** C10.05, C3.03 · **Tags:** `[MAINTAINER]` · **Size:** XS
- **Steps:**
  1. Run the tag-production deploy (C3.03) for `v1.0.0`.
  2. Check `https://firecraft-web.pages.dev/version.json` against the tag's
     commit.
  3. Check that the WebOS `appinfo.json` version reads 1.0.0.
- **Done when:** `version.json` matches the tag.
- **Verify:** `curl -s https://firecraft-web.pages.dev/version.json`.
- **Moves:** S9 = yes for 1.0.

### C10.07 — Certification (RP-08, #34) `[ ]`
- **Needs:** C10.06 · **Tags:** `[MAINTAINER]` `[DOCS]` · **Size:** S
- **Steps:**
  1. Download every published asset from the public release on clean
     runners.
  2. Repeat each smoke from P9 against the *published* bytes, not the CI
     artifacts.
  3. Mark each row `CERTIFIED`, with a receipt.
  4. Append the final SCOREBOARD row:
     `xtask climb --append C10.07`.
  5. Add a `docs/HISTORY.md` entry.
  6. Write the post-v1 roadmap stub at `docs/climb/NEXT-CLIMB.md`, seeded
     from the `[-]` rungs and the post-v1 issues.
- **Done when:** every in-scope row is `CERTIFIED`, and #34 is closed.
- **Verify:** `docs/release/ARTIFACT-MATRIX.md`.
- **Moves:** the summit is reached.
