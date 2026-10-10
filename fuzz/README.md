# FireCraft fuzz anchors

Every hostile-input surface gets an anchor in `src/lib.rs` — the contract
is "untrusted bytes must never panic". Two drivers share the anchors:

- **Stable driver** (`cargo run --bin anchors`): deterministic, bounded.
  Fixed seeds + seeded xorshift mutations, no external fuzzer needed.
  This is what `xtask ci` runs (fast gate, a few seconds).
- **libfuzzer targets** (`fuzz_targets/`, `--features fuzzer`): real
  coverage-guided fuzzing on nightly via cargo-fuzz. Full gate.

## Commands

```bash
# quick (stable, deterministic — wired into xtask ci)
cargo run --bin anchors                       # 2048 iterations/anchor
cargo run --bin anchors -- 100000 0x1234      # more iterations, other seed

# full (nightly + cargo-fuzz)
cargo install cargo-fuzz
cd fuzz
cargo fuzz build                                # compile check
cargo fuzz run fwpng_nodetree -- -max_total_time=300
cargo fuzz run fwpng_chunks fwpng_prvw fwpng_mkbt geom_transform
```

Crashes land in `fuzz/artifacts/` (git-ignored); minimize, then convert
into a unit test next to the code under test before fixing.
