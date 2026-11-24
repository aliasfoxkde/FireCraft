# FireCraft — Post-MVP Execution Plan

> **Document type:** execution plan derived from
> [POST-MVP-BASELINE-AUDIT.md](POST-MVP-BASELINE-AUDIT.md), per
> [POST-MVP-AGENT-HANDOFF.md](POST-MVP-AGENT-HANDOFF.md) §5–§9.
> **Date:** 2026-11-23.
> **Branch/worktree:** `feat/post-mvp-execution` in
> `.claude/worktrees/wt-post-mvp-exec` (base:
> `origin/docs/post-mvp-product-roadmap` `645f6699`).
> **Authority rule:** [TASKS.md](TASKS.md) remains the live execution ledger. This
> document is a **proposal** until the maintainer approves the Wave 1 scope; no
> row below has been written into `TASKS.md`.

---

## 1. Why Gate 0 comes first

The baseline audit found an unattributed 1,296-line local diff, three dirty or
unstable PRs, 109 worktrees, and documentation that disagrees with source on
command counts and keymap sets. Starting feature work in that state would create a
second competing integration line — exactly what handoff §2 forbids.

Gate 0 is therefore the only work authorized now. It is integration and
reconciliation, not new features.

---

## 2. Gate 0 — merge-safe sequence

Each step is a separate, reversible change with its own evidence. Do not start a
step before the previous one is green.

### G0-1. Protect the local uncommitted work (no commits yet)

- Confirm the 9 modified + 10 untracked paths in the main checkout are the
  intended welcome-overlay/branding work and not generated debris.
- Add `node_modules/` and `test-results/` to `.gitignore` in that branch's tree.
- Decide `r2-cors.json` and `docs/validation/` placement (commit with a consumer
  doc, or keep untracked).
- **Acceptance:** `git status` shows only attributable source/docs changes;
  `node_modules` and `test-results` no longer appear as untracked.

### G0-2. Commit the local UI work as one focused PR

- Run `cargo fmt --all`, `cargo test -p app`, `cargo clippy -p app --all-targets
  -- -D warnings`.
- Commit only the shell/branding/highlevel files; no feature work mixed in.
- **Acceptance:** focused tests green; the diff review names one objective.

### G0-3. Land PR #181 (`feat/wasm-cdn-deploy`)

- Rebase on `origin/main` after G0-1/G0-2 so the 233-file change does not collide
  with the local UI diff.
- Resolve the `DIRTY` state; re-run the gate.
- **Acceptance:** merged to `main`; CI green; the CDN bridge smoke receipt is
  recorded.

### G0-4. Land PR #178 (behavior export) and PR #182 (docs)

- Rebase #178, run `cargo test -p webexport`, merge.
- For #182, reproduce `docs-only-check` locally:
  `grep -RInE '^(<<<<<<<|=======|>>>>>>>)' docs README.md CONTRIBUTING.md` then
  `cargo run --locked -p xtask -- aegis`. Reproduced on 2026-11-23: the
  compliance rule flags a three-letter view-navigation word that is also a
  financial-account abbreviation, in the roadmap's performance row; it is a
  false positive. This branch rewords it to "viewport navigation" and removes
  the same word from PM-02. If Aegis reports further
  findings, adjudicate them (wording fix preferred, baseline row only with
  maintainer review). Do not weaken the rule class. PR #182 has since advanced
  to `645f6699` (9 commits, 6 files); its two additional Aegis hits are also
  false positives (one view-navigation word, one analytics phrase) and are
  reworded in this change.
- **Acceptance:** all three PRs merged or explicitly re-scoped; `main` green.

### G0-5. Prune worktrees and branches

- Remove worktrees with no unmerged, attributable work; keep any that hold an
  active PR.
- Delete merged local/remote branches; target ≤4 remote branches.
- **Acceptance:** `git worktree list` and `git branch -r` reflect only active work;
  no unmerged commit is deleted (verify each with `git log --not --remotes`).

### G0-6. Regenerate the ledgers from the merged tree

- `cargo run -p xtask -- reachability --bless` (only if the measurement is
  understood) · `parity` · `scorecard` · `floor` · `corpus`.
- Refresh `TASKS.md`, `NEXT.md`, `PROGRESS.md`, `HANDOFF.md`, `FEATURES.md`, and
  `CS6-PARITY.md` where the refresh rule requires it. Fix the two known drifts:
  command count (253 → measured) and the keymap-set row.
- **Acceptance:** generated docs match the generator; the dated snapshots name the
  merged SHA.

**Gate 0 exit:** owner-approved MVP baseline, `cargo run -p xtask -- ci` green on
`main`, docs reconciled, no unattributed local work.

---

## 3. Wave 1 — production confidence and discoverability

Tickets already written, each with the handoff's 18 required fields:

| Ticket | Parent | Priority | Depends on | Exit evidence |
|---|---|---|---|---|
| [PM-01](tasks/PM-01-data-integrity.md) | REL-01 | P0 | Gate 0 | Atomic-save ADR + fault-injection tests + crash-recovery receipt |
| [PM-02](tasks/PM-02-performance-budget.md) | REL-02 | P0/P1 | Gate 0 | Published benchmark table with SHA/host/profile + one rising-only floor |
| [PM-03](tasks/PM-03-format-capability-registry.md) | IO-01, IO-03 | P0 | Gate 0 | Registry consumed by dialog/CLI + fidelity matrix + pre-export loss report |
| [PM-04](tasks/PM-04-help-center.md) | UX-01 | P1 | PM-05 (soft) | Offline searchable Help + topic coverage gate + browser receipt |
| [PM-05](tasks/PM-05-discovery-and-keybindings.md) | UX-02, UX-03 | P1 | Gate 0 | Multi-domain search golden sets + rebinding with conflict tests |
| [PM-06](tasks/PM-06-accessibility-baseline.md) | A11Y-01, A11Y-02 | P1/P2 | Gate 0 | Inspector report schema + object navigator + per-criterion table |
| [PM-07](tasks/PM-07-platform-readiness.md) | PLATFORM-01 | P1 | Gate 0, #181 | Readiness JSON + matrix + one new install/launch/smoke receipt |
| [PM-08](tasks/PM-08-marketing-site.md) | SITE-01 | P1 | Gate 0; site ownership | Site baseline + claim matrix + verified Pages topology; deploy receipt or blocked report |

`REL-03` needs no ticket: it is the standing invariant (fuzz anchors, caps, typed
errors) and every new surface must extend it in the same change.

### 3.1 Suggested execution order inside Wave 1

1. **PM-03** — its registry is consumed by Help, search, and platform docs, and it
   needs no shell redesign.
2. **PM-01** — data integrity is the highest user risk and touches the save path
   only.
3. **PM-02** — measurement basis for all later performance claims.
4. **PM-05** — metadata/search/rebinding; largest shared-shell change.
5. **PM-04** — consumes PM-05 metadata and PM-03 format rows.
6. **PM-06** — accessibility inspector/navigator.
7. **PM-07** — readiness receipts; can run in parallel where hosts exist.
8. **PM-08** — public site; independent of editor code and safe to run in parallel
   once Gate 0 confirms no competing site owner. Production deploy stays
   owner-gated.

Parallelism is safe only across disjoint crates: PM-01 (`app`/`engine` save),
PM-03 (`io` + dialogs), PM-07 (docs/xtask). PM-05 and PM-06 both touch the panel
shell and must not overlap.

---

## 4. Waves 2–6 — backlog mapping (no tickets yet)

Do not pre-author implementation tickets before dependencies are understood
(handoff §5). The mapping is:

| Wave | Workstreams | Gate before tickets |
|---|---|---|
| 2 — Professional creative foundation | COLOR-01/02/03, TYPE-01, RES-01/02/03, EDIT-01/02 | Color-management ADR; font-discovery spike |
| 3 — Asset production | EXPORT-01, ASSET-01, ASSET-02, BATCH-01, SVG-01 | PM-03 registry; export manifest schema |
| 4 — Design system and interactive design | DOC-01, COMP-01, DESIGN-01/02, ANIM-01, PROTO-01 | Page/artboard/state/component ADR; format migration plan |
| 5 — Developer and automation ecosystem | DEV-01/02, AUTO-01/02/03, PLUG-01/02, PLATFORM-02/03 | Command schema versioning ADR; MCP scope/permission ADR; plugin sandbox ADR |
| 6 — Community and advanced | COMMUNITY-01–07, COLLAB-01/02, AI-01/02 | GitHub auth/privacy ADR; identity/merge model; AI consent/boundary ADR |
| M — Public marketing site | SITE-01 | Site baseline + verified Pages topology; no production deploy without owner approval |

---

## 5. Task-board proposal (not applied)

Proposed rows for `TASKS.md` after owner approval. The live ledger is deliberately
untouched in this change.

| Priority | Task | Status | Ticket | Owner | Next action |
|---|---|---|---|---|---|
| P0 | Gate 0: integrate local UI work and dirty PRs | `IN PROGRESS` | this plan §2 | integration lane | G0-1 through G0-6 |
| P0 | Data integrity and loss prevention | `NOT STARTED` | PM-01 | reliability lane | after Gate 0 |
| P0 | Format capability registry and fidelity contract | `NOT STARTED` | PM-03 | format lane | after Gate 0 |
| P0 | Performance budget harness and baseline table | `NOT STARTED` | PM-02 | reliability lane | after Gate 0 |
| P1 | Searchable offline Help Center | `NOT STARTED` | PM-04 | UI lane | after PM-05 |
| P1 | Universal discovery and keybinding editor | `NOT STARTED` | PM-05 | UI lane | after Gate 0 |
| P1 | Accessibility inspector and object navigator | `NOT STARTED` | PM-06 | a11y lane | after Gate 0 |
| P1 | Per-target platform readiness receipts | `NOT STARTED` | PM-07 | release lane | after #181 |
| P1 | Marketing site audit and deploy repair | `IN PROGRESS` | PM-08 | site lane | Phase 0 audit; deploy gate fixed |
| P2 | Wave 2 backlog (color, type, resources) | `DEFERRED` | — | — | after Wave 1 exit |
| P3 | Collaboration and AI | `DEFERRED` | — | — | after architecture decisions |

---

## 6. Verification plan

Re-read `AGENTS.md` before running; the CI pin and commands may evolve.

| Change area | Required checks |
|---|---|
| Any | `cargo run -p xtask -- ci`; `cargo fmt --all`; `cargo clippy -p <crate> --all-targets -- -D warnings` |
| Save/load, formats | `cargo test -p app -p engine -p io`; `cargo run -p xtask -- corpus`; new fault-injection tests |
| Rendering/color | `cargo test -p compose -p color`; golden images; CPU/GPU parity receipt when GPU exists |
| Shell/UI | `cargo test -p app`; served `dist/web` real-browser walkthrough (HANDOFF §6.2) |
| Command registry | `cargo run -p xtask -- reachability`; `xtask shortcuts`; catalog completeness tests |
| Accessibility | Golden findings JSON; manual AT receipt (OS, reader, version, workflows) |
| Performance | `xtask perf --measure` with SHA/host/profile recorded; no claim without a measurement |
| Release/platform | `xtask dist --…`; `--verify-manifest`; install/launch/smoke receipt per host |
| Docs-only | conflict-marker grep + `cargo run --locked -p xtask -- aegis` (the #182 job) |
| Marketing site | simulate the workflow's validation greps; serve `site/` and run Playwright at desktop/tablet/mobile; check links, metadata, release fallback, console/network, axe, Lighthouse; record viewport/tool versions |

Never treat a green compile as proof of a workflow. Every completion claim in a
ticket names the receipt.

---

## 7. Risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| Gate 0 stalls on the 233-file PR #181 | High | Extract/commit local UI work first; rebase #181 in a dedicated worktree; do not start Wave 1 early |
| Aegis baseline churn blocks docs PRs | Medium | Reproduce locally, adjudicate findings with reasons; never disable a rule class |
| Benchmark floors are flaky | Medium | Report-only first; floors only after three agreeing runs |
| Format registry drifts from codecs | Medium | Dialog + CLI read the registry; exhaustiveness test |
| Search/rebinding metadata duplicates the registry | Medium | Registration-owned metadata + completeness test |
| MCP write access is used before scoping | Medium | Do not expand MCP tools until the AUTO-02 scope ADR lands |
| Document-model work (artboards/components) breaks `.fw.png` | High if rushed | ADR + unknown-node preservation + corpus before serialization changes |
| 109 worktrees hide conflicts | High | G0-5 prune; one active worktree per lane |
| Release claims outrun receipts | Medium | Status vocabulary enforced by PM-07's JSON/matrix test |
| Marketing site drifts from `firecraft-web` or claims outrun evidence | Medium | Separate projects/outputs; PM-08 claim matrix; production deploy owner-gated |

---

## 8. Decision records to write (one per consequential choice)

1. Atomic save and recovery generations (`00NN-atomic-save.md`).
2. Format capability and fidelity contract.
3. Benchmark corpus and budget policy.
4. Command metadata and keybinding profile schema.
5. Accessibility model and conformance reporting.
6. Page / artboard / state / component / variant semantics.
7. Color-management model and dependency.
8. Command/API schema versioning.
9. MCP write scopes, permissions, and transactions.
10. Plugin runtime and permissions.
11. GitHub authentication and issue-submission privacy.
12. Collaboration merge model.
13. AI provider/local boundary and consent.

---

## 9. Next-agent handoff

**After Gate 0**, the site track (PM-08) may start in parallel if no other agent owns
`site/`; it touches no editor code. Otherwise take **PM-01 (REL-01)** if the save path has been reconciled, or
**PM-03 (IO-01)** if the first agent prefers a change that does not touch the save
path. Both are P0 and disjoint.

- **First ticket:** [PM-03](tasks/PM-03-format-capability-registry.md) is the
  recommended first implementation because it is additive, testable without a GUI,
  and unblocks Help, export, and platform documentation.
- **Files owned:** `crates/io` (new registry module), `crates/app/src/shell/dialogs.rs`,
  `crates/app-cli/src/main.rs`, `docs/FEATURES.md` §Import/export.
- **Do not touch:** `TASKS.md` (until scope approval), `crates/app/src/shell/mod.rs`
  (Gate 0 workstream), `crates/doc` serialization, any `.fw.png` node mapping.
- **Required tests:** registry completeness, round-trip per editable format,
  known-loss assertions, `xtask corpus`, `xtask ci`.
- **Stopping condition:** stop if a format row would require guessing undocumented
  semantics; record `rejected`/`reference-only` with the reason instead.
- **Definition of done:** ticket §7 acceptance criteria, PM-03 evidence, a decision
  record, and a devlog entry naming SHA/host/date.

---

## 10. Explicit deferrals

- Real-time collaboration, hosted AI, and any service requiring accounts.
- PDF/print until the color-management ADR exists.
- Plugin runtime until the permission/sandbox ADR exists.
- New document-format semantics until the page/artboard/component ADR and a
  migration plan exist.
- Off-fleet release artifacts until hosts or an approved scope exist.
- Production marketing-site deployment, DNS, and domain changes until the owner
  approves and the Pages project topology is verified.

These are deferrals, not omissions: each has a named gate above.
