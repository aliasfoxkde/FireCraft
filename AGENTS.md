# AGENTS.md — FireCraft operating manual

How any agent (or human) works in this repo. The plan lives in
`docs/PLANNING.md` (master) and `docs/planning/firecraft/` (milestone
tickets `M0`-`M12`). Read those before your first change; this file is
the day-to-day rules.

## What FireCraft is

A clean-room reimplementation of Adobe Fireworks CS6 in pure Rust: an
editable-document graphics editor with a WASM-first release target
(embeds in the author's WebOS) plus a wide native release matrix.
**Clean-room means:** reference material is documentation and behavioral
observation only. Never decompile, never copy Adobe code or assets into
the repo, never commit anything from `source/` (the local CS6 install —
git-ignored forever).

## Commands

```bash
cargo run -p xtask -- ci        # the whole gate; must be green before any commit
cargo run -p xtask -- ci --full # + libfuzzer compile check (cargo-fuzz, nightly)
cargo run -p xtask -- fuzz      # libfuzzer compile check alone
cargo run -p xtask -- layers    # workspace dependency tiers only
cargo run -p xtask -- wasm      # wasm32 check + runtime smoke (node, when runner on PATH)
cargo run -p xtask -- corpus    # pinned digests + round-trip floor
cargo run -p xtask -- corpus --report # per-fixture detail on stderr
cargo run -p xtask -- corpus-gen      # regenerate fixtures + MANIFEST.tsv digests
cargo run -p xtask -- parity    # regenerate parity floors
cargo run -p xtask -- scorecard # regenerate milestone scorecard
cargo test -p fwpng             # one crate
cargo test -p fwpng nodetree    # one module filter
cargo fmt --all                 # before committing
```

`target/` is a symlink to `/nas/Dev/Targets/firecraft-target` — recreate
it (not a plain `cargo build`) if it is missing.

## Layering (enforced by `xtask layers`)

```
L0  fwpng geom color raster     # foundations — zero cross-deps
L1  doc                         # document model
L2  vector filters text ops     # feature engines
L3  compose gpu engine          # rendering + orchestration
L4  io                          # import/export codecs
L5  app app-cli automation      # shells
L6  testkit                     # test-only, may depend on anything
L7  xtask                       # verification tooling
```

Every new workspace crate must be registered in `xtask/src/layers.rs`
in the same change that creates it — the gate fails on unregistered
crates. `fwpng` additionally must stay **standalone** (no workspace
path deps; workspace-inherited lints are fine, cargo inlines them on
publish).

## Golden rules (clippy-enforced, workspace lints)

- `unsafe_code = "forbid"`.
- No `unwrap()` / `expect()` / `panic!` / `unreachable!` /
  `unimplemented!` in non-test code — hostile input (files, strings,
  dimensions) returns typed errors. Test modules opt out with
  `#![allow(clippy::unwrap_used, clippy::expect_used)]`.
- No `dbg!` macros or stdout printing in library crates (xtask prints
  diagnostics to **stderr** so output stays pipeable).
- Never break the wasm build: core crates use no-C, no-thread
  dependencies. `xtask ci` checks `wasm32-unknown-unknown`.
- Never crash over features: a malformed file is an error value, never
  an abort.
- No placeholder markers, no versioned files (`foo_v2.rs`), no
  duplicated implementations (see `.claude/rules/quality.md`).
- Compilation-fail completeness: when the doc model maps a fixed set of
  on-disk fields (M3+), an unmapped field must be a compile error, not
  a silent skip.

## Numbers are hostile

Canvas dimensions, tile counts, string lengths, node counts all come
from files an attacker can craft. Every constructor that allocates
proportionally to an input takes caps and returns `Result`
(`raster::TileStore::new`, `fwpng::chunk::MAX_CHUNK_LEN`,
`fwpng::nodetree::MAX_DEPTH`). Follow that pattern for new surfaces.

## Fuzz anchors

`fuzz/` is its own workspace (never a member of the root): stable
deterministic driver + nightly libfuzzer targets sharing the anchors in
`fuzz/src/lib.rs`. Every hostile-input surface gets an anchor there; the
contract is "untrusted bytes must never panic".

- Quick gate (`xtask ci`): `cargo run --bin anchors` — fixed seeds,
  seeded xorshift mutations, 2048 iterations/anchor, deterministic.
- Full gate (`xtask ci --full` / `xtask fuzz`): `cargo fuzz build
  --features fuzzer` under nightly. Targets are `required-features =
  ["fuzzer"]` so the stable driver never drags libfuzzer-sys.
- A crash from either driver reproduces in the other (same anchor fns).
- Convert any crash into a unit test next to the code under test before
  fixing.

## Parity floors are rising-only

`docs/planning/generated/parity.md` records minimum bars (corpus files
parsed, round-trip identity, wasm-clean crates). A floor moves up, never
down. Lowering one means the milestone that set it regressed — fix that
instead.

## Workflow

1. Take one ticket at a time from `docs/planning/firecraft/`.
2. Track progress in the session task list; mirror milestone status in
   the scorecard (regenerated by `xtask`).
3. `cargo run -p xtask -- ci` green before every commit.
4. Conventional commits (`feat(fwpng): ...`), Co-Authored-By trailer,
   push to both remotes — `gitforge` first, then `origin`.
5. Record what happened + handoff state in `log/devlog.md` (untracked)
   at the end of a work session.
6. Corpus files (real Fireworks PNGs) are pinned by sha256 in
   `corpus/MANIFEST.tsv`; the files themselves never enter git.
   Adding a corpus row and its fixture is one change.

## Reference material map

- `docs/PLANNING.md` — master plan, format spec, stack decisions.
- `docs/research/fireworks-png-nodes.md` — vendored MIT node dictionary
  (davidje13/png-analyser); attribution in `docs/ATTRIBUTION.md`.
- `source/` — local Adobe CS6 install, reference-only, never committed.
- `/var/tmp` Adobe-derived text extracts — scratch, never committed.

## Session log

`log/devlog.md` is the untracked handoff journal: one entry per work
session — what landed, gate status, what's next. Read it first when
picking up work.
