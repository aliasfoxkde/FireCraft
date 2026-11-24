# T27-24 — Verify the remaining PR #8 findings, fix the valid ones, document the rejected ones

> **RP:** Phase-2 (PR #8 remediation) · **Priority:** P1 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-03

**Objective:** close out every remaining Major/Critical review finding on PR #8 by
verifying it against current code, fixing it if valid, and rejecting it with a written
reason if not.

**Inputs:** the PR #8 inline comments; the census in `docs/release/EXECUTION-BOARD.md`
§6; `crates/app/src/editor.rs`; `crates/app/src/shell/panels_web.rs`;
`crates/engine/src/catalog/retouch.rs`; `xtask/src/main.rs`; `docs/CS6-PARITY.md`,
`docs/NEXT.md`, `docs/PROGRESS.md`, `docs/HANDOFF.md`, `docs/wiki/`, `README.md`.

**Authority:** the PR #8 reviews; `docs/RELEASE-ENGINEERING-PLAN.md` §9 (a PR must
state unverified gaps explicitly).

**Acceptance criteria:** every remaining Major/Critical finding is independently
verified against current code; valid ones are fixed with a regression test; rejected
ones carry a written reason naming the code that is actually correct.

**Verification:** `cargo test -p app -p engine`

**Evidence:** Candidates: `editor.rs:619` drag_quadrangle, `editor.rs:2376` redraw trace, `editor.rs:2595` live_selection, `panels_web.rs:574/595` payload, `retouch.rs:656/1149` saturating multiplication

**Blockers:** —

**Stopping condition:** every enumerated finding has a disposition — fixed with a
regression test, or rejected with a reason — and the dispositions are recorded.

## Dispositions

PR #8 carried 38 inline comments: 34 distinct findings (9 code, 25 documentation)
and 4 CodeRabbit thread replies. Each finding was re-read and adjudicated against
`main` at `f1a45bec` — the code findings against the source, the documentation
findings against the current text of the file they name. A finding is **rejected**
when the code or the text it names is already correct, and the disposition names the
code that settles it; it is **fixed** when the claim still holds.

### Code findings (9)

| Finding | Severity | Disposition | What settles it |
|---|---|---|---|
| `editor.rs:3271-3276` — a plotted draft carries no tool, so a Pen's points can be folded into a Hotspot commit | Major | fixed on the T27-23 branch | `PointDraft` records its plotting tool; a tool switch commits the pending path as the tool that plotted it; three regression tests in `crates/app/tests/plotted_draft.rs` |
| `editor.rs:619` — `drag_quadrangle` moves the grabbed corner and its opposite, so a Distort drag produces a non-parallelogram and the engine rejects it | Major | rejected | `drag_quadrangle` moves the grabbed corner and the **next adjacent** corner (`let adjacent = (grabbed + 1) % 4;`), which adds the same delta to both sides of `TL + BR = TR + BL` and keeps the parallelogram the engine requires; the doc comment at `editor.rs:583` says so |
| `editor.rs:2376` — Scale, Skew and Distort read `document().object_selection`, which a Pointer click never updates | Major | rejected | all three branches read `self.live_selection()` — `editor.rs:2494` (scale), `:2525` (skew), `:2563` (distort), `:2587` |
| `editor.rs:2595` — Redraw Path replaces the whole contour with a two-point segment | Major | rejected | the drag records pointer samples and the commit sends them: `self.redraw_selected_path(&drag.trace)` at `editor.rs:2614`, the signature takes `trace: &[geom::Point]` at `:2733`; `redraw_path_uses_the_sampled_pointer_trace` is the regression |
| `panels_web.rs:574` — a symbol instance is placed from the insert box corner, so artwork lands outside the canvas | Major | rejected | `symbol_insert_translation` (`panels_web.rs:275`) translates the symbol's `definition_content_bounds` onto the canvas centre; `panels_web.rs:1522` asserts the centring |
| `panels_web.rs:595` — the Library click payload sends `width`/`height`, which `CreateInstancePayload` does not accept | Major | rejected | `place_symbol_instance` (`panels_web.rs:1316`) sends only `symbol`, `x`, `y`, `page`, `layer` and `cell`, so the payload's `deny_unknown_fields` is satisfied |
| `retouch.rs:656` — `radius * 2` is unchecked, so a large radius wraps and bypasses the brush-size cap | Major | rejected | `brush_diameter` (`retouch.rs:118`) uses `checked_mul(2)` and returns `EngineError::BadPayload`; all four retouch commands call it |
| `retouch.rs:1149` — the overflow test asserts "any error", so it passes without the fix | Minor | rejected | `retouch_commands_reject_overflowing_brush_diameters` (`retouch.rs:1141`) asserts `Err(EngineError::BadPayload { .. })` for Sharpen, Dodge, Burn and Replace Color |
| `xtask/src/main.rs:222` — `install_pattern_state` writes to a fixed, shared `/tmp` path | Minor | rejected | `main` has no `install_pattern_state`, no `temp_dir()` and no `firecraft-aegis-home`; the Aegis leg scans the repository root and is judged against the content-keyed `.aegis-baseline.tsv` (decision 0074). The scratch-home implementation exists only on unmerged branches |
| `canvas_ui.rs:1048` — the Vector Path tool begins a node drag on click release, and a pure click never fires `drag_stopped` | Major | rejected | fixed on `main` by `79fe2759` (#168): the drag opens in the press routing (`canvas_ui.rs:1060` for Subselect, `:1111` for VectorPath, both → `begin_node_reshape`) and closes only on `drag_stopped` (`canvas_ui.rs:1193` → `:1205`), so a pure click opens no transaction |

### Documentation findings (25)

| Finding | Severity | Disposition | What settles it |
|---|---|---|---|
| `docs/CS6-PARITY.md:160` — the `1280×800` row has five cells, so `22.5 / T22` lands in the wrong column | Minor | rejected | the row at `docs/CS6-PARITY.md:194` carries all six cells, with the ticket in the `Ticket` column |
| `docs/CS6-PARITY.md:95` — the menu-resource evidence says the bar resource was never decoded | Minor | rejected | `docs/CS6-PARITY.md:101` states that the `RT_MENU` 128 tree **was** walked and transcribed into `crates/app/src/menus.rs`, and says which rows were not re-read |
| `docs/CS6-PARITY.md:172` — the row still says the screenshot capture is pending | Minor | rejected | the row records the 2026-11-16 re-capture for the current dock and names the frames |
| `docs/CS6-PARITY.md:183` — the reachability verdict claims every panel | Minor | rejected | the verdict is narrowed to **the captured build** and cites the measured frames and the T22 receipts |
| `docs/NEXT.md:464` — the floating-window text contradicts the dock text in the same bullet | Minor | rejected | Phase 22.1 at `docs/NEXT.md:603` records that left↔right movement is implemented and tested |
| `docs/NEXT.md:466` — the list says five panels are unimplemented, which `TASKS.md` records as landed on 2026-11-06 | Minor | rejected | `docs/NEXT.md:622` marks the phase complete and points at the ledger |
| `docs/PROGRESS.md:19` — the earlier optimized build is presented as the current artifact | Minor | rejected | `docs/PROGRESS.md:91-92` records both builds — the earlier 21,274,323-byte wasm and the T22 21,271,259-byte rebuild — and says which check covered which |
| `docs/HANDOFF.md:3` — the orientation carries merge-conflict markers | Minor | rejected | the file is conflict-free; a marker grep over `docs/` and `README.md` returns nothing |
| `docs/HANDOFF.md:147` — the remaining-work list repeats work the same section calls complete | Minor | rejected | the list is gone; `docs/HANDOFF.md:205` names the remaining Phase 22 items only |
| `docs/HANDOFF.md:166` — the list omits left-right docking | Minor | rejected | `docs/HANDOFF.md:205` records the 22.1 docking movement landing in PR #128 (merged 2026-11-14) |
| `docs/TASKS.md:44` — Task 5 is marked `IN PROGRESS` and asks for a burn-down that is complete | Minor | rejected | `docs/TASKS.md:77` records `DONE — 2026-11-11` with the pinned 2-of-298 census |
| `docs/decisions/0083` — the Aegis `zip-code` rule matches this decision and the baseline has no entry for it | Major | rejected | `.aegis-baseline.tsv` carries an `eval-usage` entry for `docs/decisions/0083-aegis-rebless-parity-matrix.md`; the baseline is content-keyed (decision 0074) and was re-blessed for decisions 0083 and 0084 |
| `docs/research/fw-ui-evidence.md:167` — the five non-Window items count `ToolPalette`, which the Window-menu roster already includes | Minor | rejected | `docs/research/fw-ui-evidence.md:184` says `ToolPalette` is the workspace name for Tools, already in the Window-menu roster, and gates the panel count instead |
| `docs/research/fw-ui-evidence.md:170` — the separate Slices panel is claimed without evidence | Minor | rejected | `docs/research/fw-ui-evidence.md:181` gates the Slices panel's existence and parity verdict as evidence-gated |
| `docs/research/fw-ui-evidence.md:194` — the `57651` claim is stated as a proven menu row | Minor | rejected | the evidence note at `docs/research/fw-ui-evidence.md:212` keeps the command definition and the inferred menu placement apart, and says the row is not a claim that a decoded menu dump was read |
| `docs/wiki/Architecture.md:13` — the L4 row lists only `io` | Minor | rejected | the row lists `io`, `scripting` and `webexport`, matching `xtask/src/layers.rs` |
| `docs/wiki/Editor-and-Layout.md:11` — the text describes floating panel windows | Minor | rejected | the page describes a persistent, resizable right-side dock with a scrolling stack of collapsible panels |
| `docs/wiki/Editor-and-Layout.md:15` — the historical tall-capture limitation is presented as a current gap | Minor | rejected | the page keeps the inspector caveat and points at the T22 receipts instead |
| `docs/wiki/Project-Status.md:14` — the page tells readers to repeat the screenshot work | Minor | rejected | the page links the T22 screenshot audit and records the 2026-11-11 review |
| `docs/wiki/Parity-Roadmap.md:30` — the roadmap promises bitmap fill/stroke the document model cannot carry | Minor | rejected | the remaining-work item states the constraint and points at decision 0019 instead of promising the feature |
| `docs/wiki/Parity-Roadmap.md:65` — the summary implies only eight rows lack behavior, and omits the clickable gap-reporting rows | Minor | **fixed** | the page now says the 2-inert-of-298 census is not a coverage number and names the 35 `ShellAction::ReportGap` rows — a count verified against the ten menu statics in `crates/app/src/menus.rs` |
| `README.md:5` — the trademark notice omits the sponsorship and endorsement disclaimer | Minor | rejected | `README.md:22` carries the full notice: "not affiliated with, sponsored by, or endorsed by Adobe" |
| `README.md:26` — the CLI commands say `-p firecraft-cli`, but the Cargo package is `app-cli` | Major | rejected | `README.md:102` and `:112` use `-p app-cli`; the package is `app-cli` and the binary is `firecraft-cli` (`crates/app-cli/Cargo.toml`) |
| `README.md:55` — task-status readers are sent to `docs/NEXT.md` | Minor | rejected | `README.md:55` names `docs/TASKS.md` as the authoritative live ledger and keeps `docs/NEXT.md` for the Phase 22 detail |
| `README.md:149` — `docs/GAPS.md` is labelled current | Minor | **fixed** | the documentation table (`README.md:156`) already called it a historical audit; the **Start here** bullet at `README.md:60` still said "current gaps" and now says "historical gaps audit" |
| `README.md:149` — the gaps audit is not labelled historical in the Start-here list | Minor | **fixed** | same edit as the row above — the two halves of the same finding |

### The Aegis re-bless

The Aegis gate compares the scan against the committed baseline
`.aegis-baseline.tsv`, which is content-keyed on `(rule, file, digest)`
(decision 0074). PR #170's documentation-only merge added four Windows
build literals that no CI job had blessed, so the baseline was missing
keys for them. The re-bless adds **24 keys** in total — the four
Windows build identifiers wherever they now appear, the CS6 menu id
`57651`, and the `ssrf` / `ssrf-localhost` keys this change's own new
content introduced (decision 0132, the two installer scripts, the class
note and this ticket). Every one of them is a verified false positive,
and the class note that records the reason is in `xtask/src/aegis.rs`:

| Class | Keys added | Why it is a false positive |
|---|---|---|
| `zip-code` | 18 — the Windows build identifiers `19041`, `22621`, `26100` and `37710` (four keys each in `docs/tasks/T27-41-windows-arm64-assessment.md`, in `docs/decisions/0132-the-secret-scan-runs-on-documentation-changes.md` and in the gate's own class note, `xtask/src/aegis.rs`) and the CS6 menu id `57651` (one key in `docs/release/EXECUTION-BOARD.md`, four in this ticket) | build identifiers and a menu id, not postal data; the class note names each source |
| `ssrf` | 4 — the node `fetch(url)` fallback in `scripts/install-aegis.sh`, the HTTP client dependency the class note names in `xtask/src/aegis.rs`, and the prose in decision 0132 and in this ticket that names the downloaders | the only URL the repository fetches is the pinned scanner release, whose SHA-256 the installer checks before running it; nothing serves a request on anyone's behalf |
| `ssrf-localhost` | 2 — the loopback fixture server in `scripts/install-aegis-selftest.sh`, and the prose in this ticket that names it | a test fixture that serves a local tarball to the installer under test |

The counts are the baseline's own, not a summary of them: `git show
7bcf28fd -- .aegis-baseline.tsv` adds 24 rows, and `cut -f1` over those
rows counts 18 `zip-code`, 4 `ssrf` and 2 `ssrf-localhost`.

The `ssrf` class had no note of its own and fell back to the generic
"individually adjudicated" line; it has one now, and the `ssrf-localhost`
note names the new fixture server. No rule class was switched off, and
no finding was admitted without a reason.

### What this row does not claim

The dispositions above are verdicts about the findings, not a fresh certification of
the features they mention. `docs/CS6-PARITY.md` still carries the evidence gates it
records, and the gap-reporting rows it counts are still gap-reporting rows: this
change corrects what the documentation says about them, it does not close them.
