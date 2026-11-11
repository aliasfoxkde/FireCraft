# T5 — 19.6 inert-menu burn-down

**Status: COMPLETE at `5fac7a2` (2026-11-04).** Batches 1–17 are landed. The remaining eight rows are assigned to future feature work or documented evidence gates; the campaign's stopping condition does not require pretending unsupported actions work.

**Objective:** reduce the CS6 menu's inert count to its evidenced floor:
every menu row either dispatches through the engine's 245-command
registry, is a shell-only action, or carries a recorded evidence gate.

**Inputs:** `crates/app/src/menus.rs` (`Action` table + `inert_worklist`
test prints the live worklist); `crates/app/src/shell/mod.rs`
(`run_action` dispatch); `crates/engine/src/catalog` (verb surface);
CS6 `.jsf` scripts in the extracted corpus (behavioral specs);
`docs/planning/generated/dom-parity.md` (verb↔menu mapping).

**Authority:** clean-room — behavior from the dictionary, corpus
evidence, and observed CS6 behavior only. A row with no engine verb and
no derivable behavior stays inert (evidence-gated), never invented.

**Stopping condition:** `inert_worklist` prints only rows with recorded
gates or engine-verb-backed wiring; `menus::tests::inert_count_never_rises`
ceiling equals that count; [`TASKS.md`](../TASKS.md) records the
campaign as complete and points to the live worklist for the remaining
rows.

**Verification:** per batch — `cargo test -p app --lib` (census + new
behavior tests), `cargo clippy -p app --all-targets -- -D warnings`,
then the full `cargo run -p xtask -- ci` before commit.

**Batch history:** 1 selection/mask (144) · 2 canvas dialogs (129) ·
3 Window toggles (120) · 4 page/frame (114) · 5 symbol/animation (111) ·
6 View (99) · 7 Text (85) · 8 Attach to Path (83; vertical-align rows
evidence-gated — no text vertical-justification node in the dictionary) ·
9–17 completed the transform, Window, bitmap, animation, File, Edit,
Modify, and CS6 web-panel groups. The current eight rows are shown by the
live worklist; see [`NEXT.md`](../NEXT.md) and [`TASKS.md`](../TASKS.md).
