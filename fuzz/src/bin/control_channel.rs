//! control_channel — deterministic stable driver over the two machine
//! surfaces (M9 task 8): the loopback JSON control channel and the MCP
//! dispatcher, exactly the code the `serve`/`mcp` verbs loop over.
//!
//! Each sweep drives mutated-and-truncated request templates, pure
//! structural junk JSON, and a fraction of unmuted valid traffic
//! through [`firecraft_fuzz::anchor_control_channel`] and
//! [`firecraft_fuzz::anchor_control_channel_junk`]. The contract —
//! violated by panic, or by an untyped reply, or by a shrinking
//! journal — is caught, reported with its seed and case, and fails the
//! run with exit 1. The sweep must clear
//! [`firecraft_fuzz::CONTROL_CHANNEL_FLOOR`] requests per dispatcher
//! (rising-only floor).
//!
//! Usage: `control_channel [seeds] [base-seed-u64]`

use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let seeds: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(256);
    let base_seed: u64 = args
        .next()
        .and_then(|a| a.parse().ok())
        .unwrap_or(0xC4A_11E);

    panic::set_hook(Box::new(|_| {}));

    let mut cases = 0usize;
    for index in 0..seeds {
        let seed = base_seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        for case in 0..firecraft_fuzz::CHANNEL_CASES_PER_SEED {
            let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
                firecraft_fuzz::anchor_control_channel(seed, case);
                firecraft_fuzz::anchor_control_channel_junk(seed, case);
            }));
            if let Err(panic_value) = outcome {
                let detail = panic_value
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_owned())
                    .or_else(|| panic_value.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "opaque panic".to_owned());
                eprintln!("[control_channel] FAILED seed {seed} case {case}: {detail}");
                eprintln!(
                    "[control_channel] repro: cargo run --release --bin control_channel -- \
                     {seeds} {seed}"
                );
                return ExitCode::FAILURE;
            }
            cases += 1;
        }
        if (index + 1) % 64 == 0 {
            eprintln!("[control_channel] {}/{} seeds clean", index + 1, seeds);
        }
    }
    if cases < firecraft_fuzz::CONTROL_CHANNEL_FLOOR {
        eprintln!(
            "[control_channel] FAILED: {cases} sweeps below the rising-only floor {}",
            firecraft_fuzz::CONTROL_CHANNEL_FLOOR
        );
        return ExitCode::FAILURE;
    }
    eprintln!(
        "[control_channel] {} sweeps clean, both dispatchers (floor {})",
        cases,
        firecraft_fuzz::CONTROL_CHANNEL_FLOOR
    );
    ExitCode::SUCCESS
}
