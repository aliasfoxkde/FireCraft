//! panic_hunt — deterministic stable driver over the command catalog.
//!
//! Runs [`firecraft_fuzz::anchor_engine_commands`] across a fixed seed
//! range; any contract violation (a panic anywhere in the engine, a
//! failed call that mutates, undo/redo divergence, replay divergence)
//! is caught, reported with its seed, and fails the run with exit 1.
//! Part of `xtask ci` (quick gate) via `xtask fuzz`.
//!
//! Usage: `panic_hunt [seeds] [steps-per-seed] [base-seed-u64]`

use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let seeds: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(256);
    let base_seed: u64 = args
        .next()
        .and_then(|a| a.parse().ok())
        .unwrap_or(0x4A2D_C0DE);

    // The driver reports the seed itself; the default hook's backtrace
    // chatter per catch would bury it.
    panic::set_hook(Box::new(|_| {}));

    for index in 0..seeds {
        let seed = base_seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
            firecraft_fuzz::anchor_engine_commands(seed);
        }));
        if let Err(panic_value) = outcome {
            let detail = panic_value
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| panic_value.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "opaque panic".to_owned());
            eprintln!("[panic_hunt] FAILED seed {seed}: {detail}");
            eprintln!(
                "[panic_hunt] repro: cargo run --release --bin panic_hunt -- \
                 1 {seed} (args: [seeds] [base-seed]; seed = base ^ i*0x9E37...)"
            );
            return ExitCode::FAILURE;
        }
        if (index + 1) % 64 == 0 {
            eprintln!("[panic_hunt] {}/{} seeds clean", index + 1, seeds);
        }
    }
    eprintln!("[panic_hunt] {seeds} seeds clean");
    ExitCode::SUCCESS
}
