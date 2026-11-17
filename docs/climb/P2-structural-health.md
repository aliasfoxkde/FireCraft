# P2 — Structural health (unblocks parallel work)

**Goal.** Several agents can wire UI at the same time without every PR
conflicting in `shell/mod.rs` / `editor.rs` / `menus.rs`. Lint and
toolchain gaps are closed.

**Exit check:**
- S7 ≤ 12.
- `shell/mod.rs` < 3,000 lines.
- `io` no longer allows unsafe code crate-wide.
- `rust-toolchain.toml` exists.

**Lane:** trunk.

**Rule for every split rung.** It is a **pure move**:
- No logic changes, no renames of public items.
- Test count unchanged (S6 equal).
- `git diff --stat` shows code moving between files, with net line change
  ≈ 0 apart from `mod`/`use` lines.
- Review with `git diff -M --color-moved`.

---

### C2.01 — Pin the toolchain `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. Add a root `rust-toolchain.toml` with:
     - `channel = "1.99.1"` — the version CI pins (`ci-rust.Dockerfile`,
       `.github/workflows/*.yml`);
     - `components = ["rustfmt", "clippy", "llvm-tools-preview"]`;
     - `targets = ["wasm32-unknown-unknown"]`.
  2. Note it in `docs/TOOLCHAINS.md`.
- **Done when:** `rustup show` in the repo selects 1.99.1, and the gate is
  green.
- **Verify:** `rustup show active-toolchain`; `cargo run -p xtask -- ci`.
- **Moves:** reproducible measurements, which S13 depends on.

### C2.02 — Move `shell/mod.rs` tests out of line `[ ]`
- **Needs:** C0.03 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Cut the `#[cfg(test)] mod tests { … }` body (about 40 % of the file)
     from `crates/app/src/shell/mod.rs` into a new
     `crates/app/src/shell/tests.rs`.
  2. Leave `#[cfg(test)] mod tests;` in its place. Keep the module-level
     `#![allow(clippy::unwrap_used, clippy::expect_used)]` at the top of the
     new file.
  3. Fix `use super::*` paths if needed.
- **Done when:** `shell/mod.rs` has shrunk by about 2,600 lines, and test
  names and counts are unchanged.
- **Verify:** run `cargo test -p app 2>&1 | grep "test result"` before and
  after; the numbers must be identical.
- **Moves:** S7 −1 (most likely).

### C2.03 — Move `editor.rs` tests out of line `[ ]`
- **Needs:** C2.02 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. `git mv crates/app/src/editor.rs crates/app/src/editor/mod.rs`.
  2. Move its test module to `editor/tests.rs`, the same way as C2.02.
- **Done when:** test counts are identical and `editor/mod.rs` < 4,100 lines.
- **Verify:** same as C2.02.
- **Moves:** S7 −0/−1.

### C2.04 — Split `run_sel` by command family `[ ]`
- **Needs:** C2.02 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. In `crates/app/src/shell/mod.rs`, `run_sel` is a match of about 563
     lines. Create `shell/dispatch_sel.rs`.
  2. For each family of arms (selection, text, symbol, path, mask, web…),
     move the arm bodies into `fn sel_<family>(&mut self, …) -> …` in the
     new file.
  3. `run_sel` keeps the match, but each arm becomes a one-line delegate.
  4. Do one family per commit, inside one PR.
- **Done when:** `run_sel` is under 120 lines, all tests pass, and there is no
  behaviour change.
- **Verify:** `cargo test -p app`; gate.
- **Moves:** S7 (with C2.05) −1.

### C2.05 — Split `run_shell` the same way `[ ]`
- **Needs:** C2.04 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. Do the same as C2.04, into `shell/dispatch_shell.rs`.
  2. Group the arms by menu: File, Edit, View, Window, Help, Commands.
- **Done when:** `run_shell` is under 120 lines and `shell/mod.rs` is under
  3,000 lines.
- **Verify:** `wc -l crates/app/src/shell/mod.rs`; gate.
- **Moves:** S7.

### C2.06 — Split `menus.rs` by top-level menu `[ ]`
- **Needs:** C2.02 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. `git mv crates/app/src/menus.rs crates/app/src/menus/mod.rs`.
  2. Move each top-level menu's `Entry` tree into its own file:
     `menus/{file,edit,view,select,modify,text,commands,filters,window,help}.rs`.
     Each is a `pub(super) const` or fn returning the tree.
  3. Move the `tests` module to `menus/tests.rs`.
  4. `menus/mod.rs` keeps the types, assembly, and the keymap tables.
- **Done when:** the pinned census tests pass unchanged:
  - `menu_item_total_is_pinned`
  - `inert_count_never_rises`
  - `cs6_keymap_entries_are_bound`
- **Verify:** `cargo test -p app menus`.
- **Moves:** S7 −1.

### C2.07 — Scope `io`'s unsafe code to the FFI module `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. In `crates/io/Cargo.toml` `[lints.rust]`, change
     `unsafe_code = "allow"` to `"deny"`. (`forbid` cannot be overridden
     locally; `deny` can.)
  2. Restore the four clippy denies the crate's table dropped (`todo`,
     `doc_markdown`, `must_use_candidate`, `needless_pass_by_value`). Copy
     their levels from the root `[workspace.lints]`.
  3. Add `#![allow(unsafe_code)]` with a `// FFI to libwebp; feature
     lossy-webp only` comment at the top of `crates/io/src/webp_lossy.rs`.
  4. Fix any new clippy findings properly. Do not add `allow`s.
- **Done when:** clippy is clean with `--all-features`, and `git grep -n
  "unsafe" crates/io/src` shows only `webp_lossy.rs`.
- **Verify:** `cargo clippy -p io --all-targets --all-features -- -D warnings`.
- **Moves:** lint coverage.

### C2.08 — Remove phantom crates from the layer table `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. `xtask/src/layers.rs` lists `ops`, `gpu`, `automation` and `testkit`.
     Confirm that none of them exists: `ls crates`, and the `members` list in
     the root `Cargo.toml`.
  2. Delete those entries, or turn them into a commented "reserved names"
     list if a test depends on them.
  3. Update the layering diagram in `AGENTS.md` to match the real crates.
- **Done when:** the layer table equals the real workspace.
- **Verify:** `cargo run -p xtask -- layers`.
- **Moves:** —

### C2.09 — Record the split in a decision `[ ]`
- **Needs:** C2.02–C2.06 · **Tags:** `[DOCS]` · **Size:** XS
- **Steps:**
  1. Write `docs/decisions/NNNN-shell-module-layout.md`, with these points:
     - where each kind of code now lives;
     - "new dispatch arms go in `dispatch_*.rs`";
     - "new menu rows go in `menus/<menu>.rs`";
     - "new panel UI goes in `shell/panels*.rs`".
  2. Link it from `AGENTS.md` §Layering.
- **Done when:** the decision is merged.
- **Verify:** link resolves.
- **Moves:** —
