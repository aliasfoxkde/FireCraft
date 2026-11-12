//! `FireCraft` verification gates (`cargo run -p xtask -- <gate>`).
//!
//! Gates: `ci [--full]` (everything; `--full` adds the libfuzzer
//! compile check), `aegis [--bless|--report]` (secret/PII scan against the
//! content-keyed baseline; `--bless` rewrites it, `--report` lists the
//! uncovered findings read-only), `layers` (workspace
//! dependency tiers), `wasm`
//! (wasm32 check for pure crates), `corpus` (pinned corpus digests),
//! `golden` (render pins + semantic anchors + CS6 report), `fuzz`
//! (libfuzzer compile check alone), `fuzz-quick` (the stable driver
//! pair from `ci`; the CI pipeline's fuzz job), `panic-hunt` (catalog
//! fuzz
//! campaign), `parity` (rising-only parity floors), `scorecard`
//! (milestone status), `changelog` (release chore: generated
//! CHANGELOG + license/version stamps; `--check` is the release
//! freshness gate and `--cut <version>` freezes the unreleased block
//! into a released section), `stamps` (the git-free
//! license/version stamp checks, for a CI image that ships no git),
//! `stamp <version>` (move every release version stamp together —
//! the workspace, the member manifests, the path dependencies' version
//! requirements, the DEB control file and `Cargo.lock` — its package
//! entries and its dependency references),
//! `reachability` (the shell's
//! share of the command registry, against a rising floor),
//! `panel-snapshot` (the CS6 panel inventory, against a committed
//! golden), `dist --manifest` / `dist --verify-manifest` (the
//! machine-readable release manifest and its verification).
//! Diagnostics go to stderr; stdout stays clean so output can
//! be piped.

mod aegis;
mod changelog;
mod corpus;
mod coverage;
mod dist;
mod floor;
mod fuzz;
mod golden;
mod hero;
mod layers;
mod library;
mod panelsnapshot;
mod perf;
mod reachability;
mod scripts;
mod stamp;
mod status;
mod styles;
#[allow(clippy::module_inception)]
mod svggolden;
mod util;
mod wasm;
mod webgolden;

use std::process::ExitCode;

const USAGE: &str = "usage: cargo run -p xtask -- <ci [--full]|ci-no-fmt|aegis [--bless|--report]|coverage [--bless|--measure]|layers|wasm|corpus [--report]|corpus-gen|library-gen|hero-gen|styles-gen|scripts|floor|golden [--bless]|svg-golden [--bless]|svg-roundtrip [--measure]|web-golden [--bless|--measure]|perf [--bless|--measure]|dist --web|--native|--source|--sbom|--manifest|--verify-manifest|fuzz|fuzz-quick|panic-hunt|reachability [--bless]|panel-snapshot [--bless]|parity|scorecard|changelog [--check|--cut <version>]|stamps|stamp <version>>";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("ci") => run_ci(std::env::args().nth(2).as_deref(), false),
        Some("ci-no-fmt") => run_ci(std::env::args().nth(2).as_deref(), true),
        Some("aegis") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("aegis --bless", aegis::bless()),
            Some("--report") => finish("aegis --report", aegis::report()),
            _ => finish("aegis", aegis::run()),
        },
        Some("coverage") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("coverage --bless", coverage::bless()),
            Some("--measure") => finish("coverage --measure", coverage::measure()),
            _ => finish("coverage", coverage::run()),
        },
        Some("layers") => finish("layers", layers::run()),
        Some("wasm") => finish("wasm", wasm::run()),
        Some("corpus") => match std::env::args().nth(2).as_deref() {
            Some("--report") => finish("corpus --report", corpus::report()),
            _ => finish("corpus", corpus::run()),
        },
        Some("corpus-gen") => finish("corpus-gen", corpus::gen_fixtures()),
        Some("floor") => finish("floor", floor::run()),
        Some("reachability") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("reachability --bless", reachability::bless()),
            _ => finish("reachability", reachability::run()),
        },
        Some("panel-snapshot") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("panel-snapshot --bless", panelsnapshot::bless()),
            _ => finish("panel-snapshot", panelsnapshot::run()),
        },
        Some("library-gen") => finish("library-gen", library::run()),
        Some("hero-gen") => finish("hero-gen", hero::run()),
        Some("styles-gen") => finish("styles-gen", styles::run()),
        Some("scripts") => finish("scripts", scripts::run()),
        Some("golden") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("golden --bless", golden::bless()),
            _ => finish("golden", golden::run()),
        },
        Some("svg-golden") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("svg-golden --bless", svggolden::bless()),
            Some("--measure") => finish("svg-golden --measure", svggolden::measure()),
            _ => finish("svg-golden", svggolden::run()),
        },
        Some("svg-roundtrip") => match std::env::args().nth(2).as_deref() {
            Some("--measure") => finish("svg-roundtrip --measure", svggolden::roundtrip_measure()),
            _ => finish("svg-roundtrip", svggolden::roundtrip()),
        },
        Some("web-golden") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("web-golden --bless", webgolden::bless()),
            Some("--measure") => finish("web-golden --measure", webgolden::measure()),
            _ => finish("web-golden", webgolden::run()),
        },
        Some("perf") => match std::env::args().nth(2).as_deref() {
            Some("--bless") => finish("perf --bless", perf::bless()),
            Some("--measure") => finish("perf --measure", perf::measure()),
            _ => finish("perf", perf::run()),
        },
        // Hidden probe verb: `perf` re-execs this binary in a fresh
        // process so the VmHWM it reports is the open path's peak, not
        // the gate's lifetime high-water mark.
        Some("perf-child") => match std::env::args().nth(2) {
            Some(path) => finish("perf-child", perf::child(&path)),
            None => {
                eprintln!("{USAGE}");
                ExitCode::FAILURE
            }
        },
        // Hidden probe verb: `perf-selfexec` reproduces the CI shape
        // where the gate's own binary is replaced underneath the
        // running gate, then runs the memory probe through the same
        // re-exec resolution. See `perf::selfexec`.
        Some("perf-selfexec") => match std::env::args().nth(2) {
            Some(path) => finish("perf-selfexec", perf::selfexec(&path)),
            None => {
                eprintln!("{USAGE}");
                ExitCode::FAILURE
            }
        },
        Some("dist") => match std::env::args().nth(2).as_deref() {
            Some("--web") => finish("dist --web", dist::web()),
            Some("--native") => finish("dist --native", dist::native()),
            Some("--source") => finish("dist --source", dist::source()),
            Some("--sbom") => finish("dist --sbom", dist::sbom()),
            Some("--manifest") => finish("dist --manifest", dist::manifest()),
            Some("--verify-manifest") => finish("dist --verify-manifest", dist::verify_manifest()),
            _ => {
                eprintln!("{USAGE}");
                ExitCode::FAILURE
            }
        },
        Some("fuzz") => finish("fuzz", fuzz::full()),
        Some("fuzz-quick") => finish("fuzz-quick", fuzz::quick()),
        Some("panic-hunt") => finish("panic-hunt", fuzz::panic_hunt()),
        Some("parity") => finish("parity", status::parity()),
        Some("scorecard") => finish("scorecard", status::scorecard()),
        // `changelog` regenerates the committed changelog from the log
        // and verifies the release stamps. `--check` is the release
        // freshness gate: it fails when the committed file is not what
        // the generator produces. `--cut <version>` freezes the
        // unreleased block into a released version section, and
        // refuses a version the workspace does not carry.
        Some("changelog") => match std::env::args().nth(2).as_deref() {
            Some("--check") => finish("changelog --check", changelog::check()),
            Some("--cut") => match std::env::args().nth(3) {
                Some(version) => finish("changelog --cut", changelog::cut(&version)),
                None => {
                    eprintln!("{USAGE}");
                    ExitCode::FAILURE
                }
            },
            _ => finish("changelog", changelog::run()),
        },
        Some("stamps") => finish("stamps", changelog::stamps()),
        // `stamp <version>` moves every release version stamp together:
        // the workspace version, the member manifests that carry their
        // own copy, the path dependencies that pin a local crate to a
        // version requirement, the DEB control file and `Cargo.lock`
        // — its package entries and the dependency references that
        // carry a local package's version. The stamp used to be a
        // two-file manual edit, and the lock was never in it — which
        // is how the published source archive ended up unbuildable
        // with `--locked`.
        Some("stamp") => match std::env::args().nth(2) {
            Some(version) => finish("stamp", stamp::set(&version)),
            None => {
                eprintln!("{USAGE}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// One gate: a name plus the check it runs.
type Gate = (&'static str, fn() -> Result<(), String>);

fn run_ci(full: Option<&str>, skip_fmt: bool) -> ExitCode {
    // Build the step list. fmt is conditionally included so that `ci-no-fmt`
    // (used by the CI fmt-gate job) stays independent from the gate job.
    let mut steps: Vec<Gate> = vec![
        ("aegis", || {
            // Fast (~10 s): catches PII / credential regressions before fmt.
            // The committed baseline (`.aegis-baseline.tsv`) is keyed on
            // content, not on line — see `xtask/src/aegis.rs`.
            aegis::run()
        }),
        ("coverage", coverage::run),
        ("fmt", || {
            util::cargo(&["fmt", "--all", "--", "--check"]).and_then(|()| {
                // `web` is workspace-excluded (trunk) — `--all` never
                // sees it, so it is checked here explicitly.
                util::cargo(&["fmt", "--manifest-path", "web/Cargo.toml", "--", "--check"])
            })
        }),
        ("clippy", || {
            util::cargo(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])
            .and_then(|()| {
                // `web` is workspace-excluded (trunk) — clippy needs the
                // explicit manifest, same as the fmt check above.
                util::run_in(
                    Some(std::path::Path::new("web")),
                    "cargo",
                    &["clippy", "--all-targets", "--", "-D", "warnings"],
                )
            })
        }),
        ("test", || util::cargo(&["test", "--workspace"])),
        ("layers", layers::run),
        ("wasm", wasm::run),
        ("corpus", corpus::run),
        ("floor", floor::run),
        ("scripts", scripts::run),
        ("golden", golden::run),
        ("svg-golden", svggolden::run),
        ("svg-roundtrip", svggolden::roundtrip),
        ("web-golden", webgolden::run),
        ("perf", perf::run),
        ("fuzz", fuzz::quick),
        ("reachability", reachability::run),
        ("panel-snapshot", panelsnapshot::run),
        ("parity", status::parity),
        ("scorecard", status::scorecard),
        ("changelog", changelog::run),
    ];
    if skip_fmt {
        steps.remove(2); // remove "fmt" at index 2
    }
    if full == Some("--full") || full == Some("full") {
        steps.insert(steps.len() - 2, ("fuzz-full", fuzz::full));
    }
    for (name, step) in steps {
        eprintln!("[xtask] {name} ...");
        if let Err(err) = step() {
            eprintln!("[xtask] {name} FAILED");
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
        eprintln!("[xtask] {name} ok");
    }
    eprintln!("[xtask] ci: all gates green");
    ExitCode::SUCCESS
}

fn finish(name: &str, result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => {
            eprintln!("[xtask] {name} ok");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("[xtask] {name} FAILED");
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
