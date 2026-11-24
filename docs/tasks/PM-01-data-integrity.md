# PM-01 — Data integrity and loss prevention

> **Parent workstream:** REL-01 (POST-MVP-ROADMAP §5.1).
> **Priority:** P0 — a creative tool that can lose work is not usable.
> **Status:** `NOT STARTED` — gated on Gate 0 (see POST-MVP-EXECUTION-PLAN §2).
> **Owner:** unassigned. **Created:** 2026-11-23.
> **Dependencies:** none technical; starts after the integration baseline is clean.

## 1. User problem

A designer must be able to trust that a save either completed or left the previous
file intact, that a crash costs at most the autosave interval, and that a recovery
is chosen deliberately rather than silently applied. Today the shell has journal
autosave and a desktop recovery sidecar, but the failure paths (interrupted write,
disk full, read-only directory, externally modified file) have no fault-injection
coverage and no transactional-write proof.

## 2. Current implementation and evidence

- Autosave/recovery landed 2026-11-07 (Phase 21.1): `Shell::check_recovery`,
  `Shell::recovery_ui`, `recovery_path`, and the journal sidecar in
  `crates/app/src/shell/mod.rs`; web persistence is a localStorage document copy
  with boot restore (CS6-PARITY §E, row *Autosave / crash recovery*).
- The journal is `crates/engine/src/journal.rs`; the undo stack is
  `crates/engine/src/history.rs`.
- **No atomic-replacement implementation was found** in `crates/app`,
  `crates/io`, or `crates/fwpng` (`rg 'tempfile|persist\(|rename\(|atomic'`
  returns nothing). The save path writes through the platform service; its
  durability and rollback behaviour are **UNVERIFIED**.
- Recovery generations, timestamps, and a chooser with more than one candidate are
  absent.
- External-modification detection is absent.

## 3. Scope

In scope: transactional writes (temp file + fsync + atomic rename where the
filesystem supports it, documented fallback otherwise), bounded rotating recovery
generations with timestamps and source, a recovery chooser, external-modification
detection before overwrite, typed errors for disk-full/permission/invalid-path,
and fault-injection tests.

**Non-goals:** cloud backup, project packaging (RES-03), a second document format,
or changes to undo semantics.

## 4. Files and surfaces likely affected

`crates/app/src/shell/mod.rs` (save, autosave, recovery UI),
`crates/app/src/shell/file_operations.rs` if present on the integrated tree,
`crates/engine/src/platform.rs` (filesystem trait), `crates/app/src/platform_*.rs`,
`crates/app-cli/src/io.rs` (headless save must use the same path),
`crates/fwpng` writer only if the serialization boundary must change.
Surfaces: desktop, web/WASM (OPFS/localStorage fallback), CLI, scripting save verbs.

## 5. Decisions required

- Write one ADR: *atomic save and recovery generation semantics*, covering
  durability claims per platform, the fallback when `rename` is not atomic, and
  what "saved" means in the UI before durable completion.
- Decide whether the CLI shares the shell save path or a documented headless variant.

## 6. Implementation steps

1. Add a `DocumentSink` abstraction over write/rename/fsync in the platform trait.
2. Implement write-to-sibling + atomic rename on desktop; document and implement
   the fallback path (copy + rename + verify length/hash).
3. Rotate N recovery generations with timestamp + source revision metadata; prune
   deterministically by count and age.
4. Add external-modification check (mtime + size + digest) before overwrite.
5. Build the recovery chooser listing candidates with timestamps and a preview
   action; default to the most recent valid generation.
6. Add fault injection: injectable sink that fails at each stage.
7. Route CLI save through the same sink.

## 7. Acceptance criteria

- Interrupting a save at any stage leaves the previous file readable and unchanged.
- A simulated disk-full/permission error reports failure and does not truncate.
- Recovery offers the newest valid generation with its timestamp and source.
- Open → edit → save → close → reopen preserves semantic content; no success is
  reported before durable completion.
- Recovery storage stays within a documented bound and never overwrites a
  user-selected backup.
- A file changed externally triggers a warning before overwrite.

## 8. Test plan

- Unit: sink stage failures, generation pruning, digest comparison.
- Integration: tempdir round trip, read-only directory, injected write errors.
- Existing corpus: save/reopen byte-equivalence where the format guarantees it.
- `cargo test -p app -p engine -p app-cli`, then `cargo run -p xtask -- ci`.

## 9. Manual verification

Desktop crash test (kill -9 during save), recover, confirm undo history; browser
test with quota exceeded (devtools) and the download fallback.

## 10. Performance, security, portability

Recovery writes must not block the UI thread for large documents; hashing is
streamed. No path leaking in default error text. WASM: no threads, no `std::fs`;
the sink is feature/platform-gated with a localStorage/OPFS implementation.

## 11. Risks and rollback

Risk: platform-specific rename semantics differ. Mitigation: capability probe and
documented fallback; the change is internal and reversible per commit.

## 12. Stopping condition and closure evidence

Stop if the integrated tree's save path cannot be identified without resolving
Gate 0 first. Close only with the ADR, fault-injection tests in CI, and a recorded
manual crash-recovery receipt naming host, build SHA, and date.
