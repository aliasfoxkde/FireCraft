//! `FireCraft` verification gates (`cargo run -p xtask -- <gate>`).
//!
//! Gates: `ci [--full]` (everything; `--full` adds the libfuzzer
//! compile check), `layers` (workspace dependency tiers), `wasm`
//! (wasm32 check for pure crates), `corpus` (pinned corpus digests),
//! `golden` (render pins + semantic anchors + CS6 report), `fuzz`
//! (libfuzzer compile check alone), `fuzz-quick` (the stable driver
//! pair from `ci`; the CI pipeline's fuzz job), `panic-hunt` (catalog
//! fuzz
//! campaign), `parity` (rising-only parity floors), `scorecard`
//! (milestone status), `changelog` (release chore: generated
//! CHANGELOG + license/version stamps). Diagnostics go to stderr;
//! stdout stays clean so output can be piped.

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
mod perf;
mod scripts;
mod status;
mod styles;
#[allow(clippy::module_inception)]
mod svggolden;
mod util;
mod wasm;
mod webgolden;

use std::process::ExitCode;

const USAGE: &str = "usage: cargo run -p xtask -- <ci [--full]|ci-no-fmt|aegis [--report]|coverage [--bless|--measure]|layers|wasm|corpus [--report]|corpus-gen|library-gen|hero-gen|styles-gen|scripts|floor|golden [--bless]|svg-golden [--bless]|svg-roundtrip [--measure]|web-golden [--bless|--measure]|perf [--bless|--measure]|dist --web|--native|fuzz|fuzz-quick|panic-hunt|parity|scorecard>";

/// Run the Aegis secret scan and fail on any finding the repo has not
/// explicitly accounted for.
///
/// Two suppression mechanisms, both committed to the repo and both
/// keyed so that an ordinary edit cannot invalidate them:
///
/// * `.aegis-pattern-state.json` — the **rule-class** suppression.
///   Aegis reads it from `$HOME/.config/aegis/pattern-state.json`, so
///   the gate installs it into a private home rather than the
///   developer's. It names whole rule classes that cannot be true
///   positives in this tree (see decision 0070 for the evidence behind
///   each name).
/// * `.aegis-allowlist.json` — the **per-finding** suppression, for
///   the handful of individual false positives no rule class covers
///   (a test fixture that deliberately carries an XXE payload, the
///   Apple plist doctype in a packaging script).
///
/// The allowlist key is `pattern:file:content-digest` and deliberately
/// omits the line number. Aegis's own `--baseline` keys on
/// `file:line:content-digest`, so any edit above a finding invalidates
/// its baseline entry and the gate fails on noise rather than on a
/// leak. A gate that fails on noise is a gate nobody runs.
fn aegis() -> Result<(), String> {
    let live = live_findings()?;
    if live.is_empty() {
        return Ok(());
    }
    let mut report = format!("{} new Aegis finding(s):\n", live.len());
    for finding in &live {
        report.push_str(&format!(
            "  {} {}:{} [{}] {}\n",
            finding.pattern, finding.file, finding.line, finding.severity, finding.description
        ));
    }
    report.push_str(
        "\nEach finding must be verified before it is suppressed. If it is a false \
         positive, add its key to .aegis-allowlist.json (decision 0070); if it is \
         real, fix the source.\n",
    );
    Err(report)
}

/// Print the findings the allowlist does not cover, with the keys the
/// allowlist would need. Read-only: suppressing a finding is a human
/// decision, so this gate never writes the allowlist itself.
fn aegis_report() -> Result<(), String> {
    for finding in live_findings()? {
        eprintln!("{}", finding.key());
        eprintln!(
            "  {} {}:{} [{}] {}",
            finding.pattern, finding.file, finding.line, finding.severity, finding.description
        );
    }
    Ok(())
}

/// One Aegis finding, reduced to the fields the gate reasons about.
struct AegisFinding {
    pattern: String,
    file: String,
    line: u64,
    severity: String,
    description: String,
    digest: String,
}

impl AegisFinding {
    /// The allowlist key: rule, file, and the digest of the matched
    /// content — deliberately *not* the line number.
    fn key(&self) -> String {
        format!("{}:{}:{}", self.pattern, self.file, self.digest)
    }
}

/// Every finding the committed allowlist does not cover.
fn live_findings() -> Result<Vec<AegisFinding>, String> {
    let root = util::workspace_root();
    let home = install_pattern_state(&root)?;
    let home = home.to_string_lossy().into_owned();
    let output = util::capture_with_env(
        Some(root.as_path()),
        &aegis_binary()?,
        &["--format", "json", "-c", "production", "scan", ".", "-q"],
        &[("HOME", home.as_str())],
    )?;
    let report: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("aegis produced unreadable JSON: {e}"))?;
    let allowed = load_allowlist(&root)?;
    let mut live = Vec::new();
    let Some(findings) = report.get("findings").and_then(serde_json::Value::as_array) else {
        return Err("the aegis report has no findings array".to_owned());
    };
    for finding in findings {
        let parsed = parse_finding(finding);
        if !allowed.contains(&parsed.key()) {
            live.push(parsed);
        }
    }
    Ok(live)
}

/// Read one finding out of the scanner's JSON.
fn parse_finding(finding: &serde_json::Value) -> AegisFinding {
    let text = |key: &str| {
        finding
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let location = finding.get("location");
    let file = location
        .and_then(|l| l.get("file"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let line = location
        .and_then(|l| l.get("line"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default();
    // The scanner's fingerprint is `pattern:file:line:content-digest`;
    // the digest is the part that identifies the matched content
    // regardless of where in the file it sits.
    let digest = text("fingerprint")
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .to_owned();
    AegisFinding {
        pattern: text("pattern"),
        file: normalize_path(file),
        line,
        severity: text("severity"),
        description: text("description"),
        digest,
    }
}

/// The scanner reports paths as `./path`; the allowlist writes them
/// without the prefix.
fn normalize_path(path: &str) -> String {
    path.strip_prefix("./").unwrap_or(path).to_owned()
}

/// The set of allowlisted content keys.
fn load_allowlist(root: &std::path::Path) -> Result<std::collections::HashSet<String>, String> {
    let path = root.join(".aegis-allowlist.json");
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let document: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))?;
    let entries = document
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("{} has no entries array", path.display()))?;
    let mut keys = std::collections::HashSet::new();
    for entry in entries {
        let field = |name: &str| {
            entry
                .get(name)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let file = normalize_path(&field("file"));
        keys.insert(format!("{}:{}:{}", field("pattern"), file, field("digest")));
    }
    Ok(keys)
}

/// Installs the committed pattern-state suppression into a private
/// home and returns that home's path.
///
/// The home is a scratch directory outside the tree: the scanner only
/// reads `$HOME/.config/aegis/pattern-state.json` from it, and the
/// developer's own Aegis configuration is never touched.
fn install_pattern_state(root: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let source = root.join(".aegis-pattern-state.json");
    let state =
        std::fs::read_to_string(&source).map_err(|e| format!("read {}: {e}", source.display()))?;
    let home = std::env::temp_dir().join("firecraft-aegis-home");
    let config = home.join(".config").join("aegis");
    std::fs::create_dir_all(&config).map_err(|e| format!("create {}: {e}", config.display()))?;
    std::fs::write(config.join("pattern-state.json"), state)
        .map_err(|e| format!("write the Aegis pattern state: {e}"))?;
    Ok(home)
}

/// The Aegis scanner's CLI. An unrelated tool also ships under the
/// bare name `aegis` (a security-file generator), so the versioned
/// scanner binary is preferred and the plain name is used only when
/// it really is the scanner — decided by its own `--help`, which lists
/// the `scan` subcommand the gate needs. `FIRECRAFT_AEGIS` overrides
/// both.
fn aegis_binary() -> Result<String, String> {
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(explicit) = std::env::var("FIRECRAFT_AEGIS") {
        candidates.push(explicit);
    }
    candidates.push("aegis-0.6.3".to_owned());
    candidates.push("aegis".to_owned());
    for candidate in &candidates {
        let Ok(output) = std::process::Command::new(candidate).arg("--help").output() else {
            continue;
        };
        let help = String::from_utf8_lossy(&output.stdout);
        if help.contains("scan") {
            return Ok(candidate.clone());
        }
    }
    Err(format!(
        "no Aegis scanner found (looked for {}); set FIRECRAFT_AEGIS to the scanner's path",
        candidates.join(", ")
    ))
}

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("ci") => run_ci(std::env::args().nth(2).as_deref(), false),
        Some("ci-no-fmt") => run_ci(std::env::args().nth(2).as_deref(), true),
        Some("aegis") => match std::env::args().nth(2).as_deref() {
            Some("--report") => finish("aegis --report", aegis_report()),
            _ => finish("aegis", aegis()),
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
        Some("dist") => match std::env::args().nth(2).as_deref() {
            Some("--web") => finish("dist --web", dist::web()),
            Some("--native") => finish("dist --native", dist::native()),
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
        Some("changelog") => finish("changelog", changelog::run()),
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
            // Suppressions are committed as `.aegis-pattern-state.json`
            // (rule classes) and `.aegis-allowlist.json` (individual
            // findings); see decision 0070.
            aegis()
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
