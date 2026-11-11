//! Reachability gate: the share of the engine's command registry the
//! shell's designed surface can execute.
//!
//! The command palette (Ctrl+P) can run *every* registered command, so
//! counting it would make this metric vacuous — the gate could never
//! fail. What the gate measures is the surface CS6 actually shows a
//! user: the menu bar, the panels, the dialogs, and the canvas tools
//! — every place the shell hands a registry command id to the engine.
//! That is the surface the Phase 19 batches have been widening, and
//! the number the docs quote ("N commands unreachable") was always
//! about.
//!
//! The measurement is derived, never hand-written: the gate reads the
//! shell crate's own source and collects the command ids it binds (the
//! string literals it hands to the engine). A binding that is added
//! shows up without a second edit; a binding that is deleted shows up
//! as a hole. Test code is excluded — a `#[cfg(test)]` module does
//! not make a command reachable for a user.
//!
//! The scan is a literal scan, so a command id mentioned in a comment
//! would count as bound. No id is bound that way today (all 146 sit
//! in code), and a mention that inflates the count also *raises* the
//! share, which the ratchet then pins — the failure mode is a floor
//! that is a little too generous, never a floor that hides a
//! regression.
//!
//! The floor lives in [`.reachability-floor.tsv`], the same shape as
//! the Aegis baseline: a committed file the gate reads, regenerated
//! deliberately with `--bless`. The floor is a one-way ratchet — the
//! unreachable share may fall and never rise — so a removed binding
//! fails the build and only a deliberate re-pin can forgive it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::util::workspace_root;

/// The committed floor file.
pub const FLOOR_FILE: &str = ".reachability-floor.tsv";

/// The shell crate whose source carries the bindings.
const SHELL_SOURCE: &str = "crates/app/src";

/// The header written above the floor records.
const FLOOR_HEADER: &str = "\
# FireCraft reachability floor — the share of the engine's command
# registry the shell's designed surface can execute.
#
# The gate fails when the measured reachable share drops below
# `floor_reachable_percent`. The floor is a one-way ratchet: the
# unreachable share may fall and never rise, so a removed binding
# fails the build and only a deliberate re-pin forgives it.
#
# `reachable_percent` records the measurement the floor was last set
# from; `registry_commands` and `shell_bound_commands` record the two
# counts behind it.
#
# Regenerate with: cargo run -p xtask -- reachability --bless
# The bless verb refuses to lower the floor.
#
# metric<TAB>value
";

/// The gate: measure the shell's reachability and hold it to the floor.
pub fn run() -> Result<(), String> {
    run_inner(false)
}

/// `xtask reachability --bless`: re-measure and rewrite the floor.
///
/// The bless refuses to lower the floor — the unreachable share may
/// fall and never rise.
pub fn bless() -> Result<(), String> {
    run_inner(true)
}

/// One reachability measurement: how much of the registry the shell binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Measurement {
    /// The commands the engine's catalog owns.
    registry: usize,
    /// The commands the shell's source binds.
    bound: usize,
    /// The reachable share, in hundredths of a percent.
    reachable_hundredths: u32,
}

impl Measurement {
    /// The reachable share in percent, for a human reader.
    fn reachable_percent(&self) -> f64 {
        f64::from(self.reachable_hundredths) / 100.0
    }
}

/// The recorded floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Floor {
    /// The registry size the floor was measured against.
    registry_commands: usize,
    /// The bound count the floor was measured from.
    shell_bound_commands: usize,
    /// The reachable share the floor was measured at.
    reachable_hundredths: u32,
    /// The bar: the reachable share must not drop below this.
    floor_hundredths: u32,
}

impl Floor {
    /// The floor's share in percent, for a human reader.
    fn floor_percent(&self) -> f64 {
        f64::from(self.floor_hundredths) / 100.0
    }
}

fn run_inner(bless: bool) -> Result<(), String> {
    let root = workspace_root();
    let registry = engine::catalog::ids();
    let bound = bound_commands(&root.join(SHELL_SOURCE), &registry)?;
    let measurement = measure(&registry, &bound);
    let unreachable: Vec<String> = registry
        .iter()
        .filter(|id| !bound.contains(**id))
        .map(|id| (*id).to_owned())
        .collect();
    let floor = read_floor(&root)?;

    eprintln!(
        "[xtask] reachability: {}/{} registry commands bound by the shell ({:.2}%), \
         floor {:.2}% (recorded at {}/{} commands, {:.2}%; {} unreachable)",
        measurement.bound,
        measurement.registry,
        measurement.reachable_percent(),
        floor.floor_percent(),
        floor.shell_bound_commands,
        floor.registry_commands,
        f64::from(floor.reachable_hundredths) / 100.0,
        unreachable.len()
    );

    if bless {
        return write_floor(&root, &measurement, &floor);
    }
    evaluate(&measurement, &floor, &unreachable)
}

/// Measures the reachability of one registry against one bound set.
///
/// The share is computed in hundredths of a percent with integer
/// arithmetic, so the same inputs always produce the same number and
/// the floor comparison never depends on a float rounding.
fn measure(registry: &[&str], bound: &BTreeSet<String>) -> Measurement {
    let registry = registry.len();
    let bound = bound.len();
    let reachable_hundredths = if registry == 0 {
        0
    } else {
        // bound / registry as a share of 100% (10 000 hundredths).
        ((bound as u64 * 10_000 + registry as u64 / 2) / registry as u64) as u32
    };
    Measurement {
        registry,
        bound,
        reachable_hundredths,
    }
}

/// The verdict for one measurement against one floor.
///
/// This is the decision core: it takes the measurement and the floor as
/// data and answers "does the shell still reach enough of the
/// registry?". It reads no process state, so a test can drive it with
/// a deliberately broken set.
fn evaluate(
    measurement: &Measurement,
    floor: &Floor,
    unreachable: &[String],
) -> Result<(), String> {
    if measurement.reachable_hundredths >= floor.floor_hundredths {
        return Ok(());
    }
    let mut message = format!(
        "reachability below the recorded floor: {}/{} registry commands are bound by the shell \
         ({:.2}%), floor {:.2}% — wire the command back, or re-pin the floor deliberately \
         (`cargo run -p xtask -- reachability --bless`)",
        measurement.bound,
        measurement.registry,
        measurement.reachable_percent(),
        floor.floor_percent()
    );
    if !unreachable.is_empty() {
        message.push_str("\nunreachable commands:");
        for id in unreachable.iter().take(20) {
            message.push_str(&format!("\n  {id}"));
        }
        if unreachable.len() > 20 {
            message.push_str(&format!("\n  ... and {} more", unreachable.len() - 20));
        }
    }
    Err(message)
}

/// The registry commands the shell's source binds.
///
/// A binding is a command id the shell hands to the engine: the string
/// literal it passes to `run_command` / `apply`. The scan reads the
/// shell crate's own source rather than a hand-maintained table, so
/// the number cannot drift away from the code.
fn bound_commands(dir: &Path, registry: &[&str]) -> Result<BTreeSet<String>, String> {
    let known: BTreeSet<&str> = registry.iter().copied().collect();
    let mut bound = BTreeSet::new();
    for file in rust_files(dir)? {
        let text =
            std::fs::read_to_string(&file).map_err(|e| format!("read {}: {e}", file.display()))?;
        for literal in string_literals(non_test_code(&text)) {
            if known.contains(literal.as_str()) {
                bound.insert(literal);
            }
        }
    }
    if bound.is_empty() {
        return Err(format!(
            "reachability: no command bindings found under {} — the scan is broken, not the shell",
            dir.display()
        ));
    }
    Ok(bound)
}

/// Every Rust file under `dir`, sorted.
fn rust_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = std::fs::read_dir(&current)
            .map_err(|e| format!("read {}: {e}", current.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|e| format!("read {}: {e}", current.display()))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The file's non-test code: everything before the first
/// `#[cfg(test)]` marker.
///
/// A test module drives the engine, but it does not put a command in
/// front of a user, so a binding that only exists in test code is not
/// a reachable command.
fn non_test_code(text: &str) -> &str {
    match text.find("#[cfg(test)]") {
        Some(at) => &text[..at],
        None => text,
    }
}

/// The string literals in `text`, with the escapes resolved.
fn string_literals(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_literal = false;
    let mut escaped = false;
    for ch in text.chars() {
        if in_literal {
            if escaped {
                current.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                out.push(std::mem::take(&mut current));
                in_literal = false;
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_literal = true;
            current.clear();
        }
    }
    out
}

/// Reads the committed floor.
fn read_floor(root: &Path) -> Result<Floor, String> {
    let path = root.join(FLOOR_FILE);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "read {}: {e} — run `cargo run -p xtask -- reachability --bless`",
            path.display()
        )
    })?;
    parse_floor(FLOOR_FILE, &text)
}

/// Parses a floor file's records.
fn parse_floor(label: &str, text: &str) -> Result<Floor, String> {
    let mut registry_commands = None;
    let mut shell_bound_commands = None;
    let mut reachable_hundredths = None;
    let mut floor_hundredths = None;
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('\t') else {
            return Err(format!(
                "{label}:{}: not a `metric<TAB>value` record",
                number + 1
            ));
        };
        match key {
            "registry_commands" => registry_commands = Some(parse_count(label, key, value, number)?),
            "shell_bound_commands" => {
                shell_bound_commands = Some(parse_count(label, key, value, number)?);
            }
            "reachable_percent" => {
                reachable_hundredths = Some(parse_percent(label, key, value, number)?);
            }
            "floor_reachable_percent" => {
                floor_hundredths = Some(parse_percent(label, key, value, number)?);
            }
            other => {
                return Err(format!("{label}:{}: unknown metric `{other}`", number + 1));
            }
        }
    }
    Ok(Floor {
        registry_commands: registry_commands
            .ok_or_else(|| format!("{label}: missing `registry_commands`"))?,
        shell_bound_commands: shell_bound_commands
            .ok_or_else(|| format!("{label}: missing `shell_bound_commands`"))?,
        reachable_hundredths: reachable_hundredths
            .ok_or_else(|| format!("{label}: missing `reachable_percent`"))?,
        floor_hundredths: floor_hundredths
            .ok_or_else(|| format!("{label}: missing `floor_reachable_percent`"))?,
    })
}

/// The floor the next bless records: the measured share, never below
/// the recorded one.
///
/// The ratchet: the floor may rise (reachability improved) and never
/// fall (a regression is fixed, not forgiven).
fn next_floor(measurement: &Measurement, floor: &Floor) -> Result<u32, String> {
    if measurement.reachable_hundredths < floor.floor_hundredths {
        return Err(format!(
            "reachability regressed: {}/{} registry commands are bound by the shell ({:.2}%), \
             floor {:.2}% — the floor is a one-way ratchet; wire the command back instead of \
             lowering it",
            measurement.bound,
            measurement.registry,
            measurement.reachable_percent(),
            floor.floor_percent()
        ));
    }
    Ok(measurement.reachable_hundredths.max(floor.floor_hundredths))
}

/// Rewrites the committed floor from a fresh measurement.
fn write_floor(root: &Path, measurement: &Measurement, floor: &Floor) -> Result<(), String> {
    let floor_hundredths = next_floor(measurement, floor)?;
    let content = render_floor(measurement, floor_hundredths);
    let path = root.join(FLOOR_FILE);
    std::fs::write(&path, content).map_err(|e| format!("write {}: {e}", path.display()))?;
    eprintln!(
        "[xtask] reachability --bless: wrote {FLOOR_FILE} (floor {:.2}%)",
        f64::from(floor_hundredths) / 100.0
    );
    Ok(())
}

/// Renders the floor file from a measurement.
fn render_floor(measurement: &Measurement, floor_hundredths: u32) -> String {
    format!(
        "{FLOOR_HEADER}registry_commands\t{}\nshell_bound_commands\t{}\nreachable_percent\t{:.2}\nfloor_reachable_percent\t{:.2}\n",
        measurement.registry,
        measurement.bound,
        measurement.reachable_percent(),
        f64::from(floor_hundredths) / 100.0
    )
}

/// Parses a count metric.
fn parse_count(label: &str, key: &str, value: &str, number: usize) -> Result<usize, String> {
    value
        .trim()
        .parse::<usize>()
        .map_err(|e| format!("{label}:{}: `{key}` is not a count: {e}", number + 1))
}

/// Parses a percent metric into hundredths of a percent.
fn parse_percent(label: &str, key: &str, value: &str, number: usize) -> Result<u32, String> {
    let text = value.trim();
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (text, ""),
    };
    let whole = whole
        .parse::<u32>()
        .map_err(|e| format!("{label}:{}: `{key}` is not a percent: {e}", number + 1))?;
    if !fraction.is_empty() && !fraction.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!(
            "{label}:{}: `{key}` is not a percent: {text}",
            number + 1
        ));
    }
    let mut fraction = fraction.to_owned();
    while fraction.len() < 2 {
        fraction.push('0');
    }
    if fraction.len() > 2 {
        return Err(format!(
            "{label}:{}: `{key}` carries more precision than a hundredth of a percent",
            number + 1
        ));
    }
    let hundredths = fraction
        .parse::<u32>()
        .map_err(|e| format!("{label}:{}: `{key}` is not a percent: {e}", number + 1))?;
    Ok(whole * 100 + hundredths)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A three-command registry with two of them bound.
    fn sample() -> (Vec<&'static str>, BTreeSet<String>) {
        let registry = vec!["object.align", "object.delete", "text.setAttrs"];
        let bound = ["object.align", "object.delete"]
            .iter()
            .map(|id| (*id).to_owned())
            .collect();
        (registry, bound)
    }

    /// A floor that the sample measurement meets exactly.
    fn sample_floor() -> Floor {
        Floor {
            registry_commands: 3,
            shell_bound_commands: 2,
            reachable_hundredths: 6667,
            floor_hundredths: 6667,
        }
    }

    #[test]
    fn the_measurement_reports_the_counts_and_the_share() {
        let (registry, bound) = sample();
        let measurement = measure(&registry, &bound);
        assert_eq!(measurement.registry, 3);
        assert_eq!(measurement.bound, 2);
        assert_eq!(measurement.reachable_hundredths, 6667);
    }

    #[test]
    fn an_unchanged_shell_clears_the_floor() {
        let (registry, bound) = sample();
        let measurement = measure(&registry, &bound);
        assert!(evaluate(&measurement, &sample_floor(), &[]).is_ok());
    }

    /// The discriminating counterexample: a command that stops being
    /// reachable drops the share below the floor and fails the gate.
    #[test]
    fn a_command_removed_from_the_reachable_set_fails_the_gate() {
        let (registry, mut bound) = sample();
        bound.remove("object.delete");
        let measurement = measure(&registry, &bound);
        let unreachable = vec!["object.delete".to_owned()];
        let error = evaluate(&measurement, &sample_floor(), &unreachable)
            .expect_err("a removed binding must fail the gate");
        assert!(error.contains("below the recorded floor"), "{error}");
        assert!(error.contains("object.delete"), "{error}");
    }

    /// A registry that grows without a binding drops the share too —
    /// the floor guards the share, not the raw counts.
    #[test]
    fn a_new_unbound_command_fails_the_gate() {
        let (mut registry, bound) = sample();
        registry.push("text.setFont");
        let measurement = measure(&registry, &bound);
        let error = evaluate(&measurement, &sample_floor(), &["text.setFont".to_owned()])
            .expect_err("an unbound new command must fail the gate");
        assert!(error.contains("below the recorded floor"), "{error}");
    }

    /// The ratchet: a better measurement may raise the floor, a worse
    /// one may not lower it.
    #[test]
    fn the_floor_is_a_one_way_ratchet() {
        let floor = sample_floor();
        let improved = Measurement {
            registry: 3,
            bound: 3,
            reachable_hundredths: 10_000,
        };
        assert_eq!(next_floor(&improved, &floor), Ok(10_000));

        let worse = Measurement {
            registry: 3,
            bound: 1,
            reachable_hundredths: 3333,
        };
        let error = next_floor(&worse, &floor)
            .expect_err("a regressed measurement must not lower the floor");
        assert!(error.contains("one-way ratchet"), "{error}");
    }

    /// The scan finds the bindings a shell really has, and ignores the
    /// ones that only live in test code.
    #[test]
    fn the_scan_reads_bindings_and_skips_test_code() {
        let registry = vec!["object.align", "object.delete", "text.setAttrs"];
        let source = r#"
            fn dispatch() {
                let _ = run_command("object.align", payload());
                let _ = run_command("text.setAttrs", payload());
            }
            #[cfg(test)]
            mod tests {
                fn test_only() {
                    let _ = run_command("object.delete", payload());
                }
            }
        "#;
        let bound = scan_source(source, &registry);
        assert!(bound.contains("object.align"));
        assert!(bound.contains("text.setAttrs"));
        assert!(!bound.contains("object.delete"));
    }

    /// The scan matches whole literals: a longer id that starts with a
    /// catalog id is not a binding of it.
    #[test]
    fn the_scan_matches_whole_literals_only() {
        let registry = vec!["object.align"];
        let bound = scan_source("let _ = run_command(\"object.alignToPath\", p);", &registry);
        assert!(bound.is_empty());
    }

    /// The floor file round-trips: what the gate writes, it reads.
    #[test]
    fn the_floor_file_round_trips() {
        let measurement = Measurement {
            registry: 245,
            bound: 146,
            reachable_hundredths: 5959,
        };
        let rendered = render_floor(&measurement, measurement.reachable_hundredths);
        let parsed = parse_floor(FLOOR_FILE, &rendered).expect("the rendered floor must parse");
        assert_eq!(parsed.registry_commands, 245);
        assert_eq!(parsed.shell_bound_commands, 146);
        assert_eq!(parsed.reachable_hundredths, 5959);
        assert_eq!(parsed.floor_hundredths, 5959);
    }

    /// The scan helper the gate uses, over one source text.
    fn scan_source(source: &str, registry: &[&str]) -> BTreeSet<String> {
        let known: BTreeSet<&str> = registry.iter().copied().collect();
        string_literals(non_test_code(source))
            .into_iter()
            .filter(|literal| known.contains(literal.as_str()))
            .collect()
    }
}
