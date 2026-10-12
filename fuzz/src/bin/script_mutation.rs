//! script_mutation — deterministic stable driver over the scripting
//! host (M9 task 8).
//!
//! Every seed sweep runs each synthetic seed script unmutated (the
//! baseline: known outcomes from throw to limit halt) and then
//! [`SCRIPT_MUTANTS_PER_SEED`] byte-level mutants of the seed corpus
//! through [`firecraft_fuzz::anchor_script_mutation`]. The host
//! contract — limit halts leave the document untouched, every other
//! outcome leaves a journal that replays to the current document, and
//! nothing anywhere panics — is enforced per case: a violation is
//! caught, reported with its seed and case, and fails the run with
//! exit 1. The sweep must clear
//! [`firecraft_fuzz::SCRIPT_MUTATION_FLOOR`] cases (rising-only floor).
//!
//! Usage: `script_mutation [seeds] [base-seed-u64]`

use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let seeds: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(256);
    let base_seed: u64 = args
        .next()
        .and_then(|a| a.parse().ok())
        .unwrap_or(0x5C1_9C1);

    panic::set_hook(Box::new(|_| {}));

    let per_seed = firecraft_fuzz::SCRIPT_SEEDS.len() + firecraft_fuzz::SCRIPT_MUTANTS_PER_SEED;
    let mut cases = 0usize;
    for index in 0..seeds {
        let seed = base_seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        for case in 0..per_seed {
            let source: String = if case < firecraft_fuzz::SCRIPT_SEEDS.len() {
                firecraft_fuzz::SCRIPT_SEEDS[case].to_owned()
            } else {
                firecraft_fuzz::mutated_script(seed, case)
            };
            let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
                firecraft_fuzz::anchor_script_mutation(seed, case, &source);
            }));
            if let Err(panic_value) = outcome {
                let detail = panic_value
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_owned())
                    .or_else(|| panic_value.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "opaque panic".to_owned());
                eprintln!("[script_mutation] FAILED seed {seed} case {case}: {detail}");
                eprintln!("[script_mutation] source: {source:?}");
                eprintln!(
                    "[script_mutation] repro: cargo run --release --bin script_mutation -- \
                     {seeds} {seed}"
                );
                return ExitCode::FAILURE;
            }
            cases += 1;
        }
        if (index + 1) % 64 == 0 {
            eprintln!("[script_mutation] {}/{} seeds clean", index + 1, seeds);
        }
    }
    if cases < firecraft_fuzz::SCRIPT_MUTATION_FLOOR {
        eprintln!(
            "[script_mutation] FAILED: {cases} cases below the rising-only floor {}",
            firecraft_fuzz::SCRIPT_MUTATION_FLOOR
        );
        return ExitCode::FAILURE;
    }
    eprintln!(
        "[script_mutation] {cases} cases clean (floor {})",
        firecraft_fuzz::SCRIPT_MUTATION_FLOOR
    );
    ExitCode::SUCCESS
}
