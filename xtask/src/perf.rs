//! Performance budgets (M12 task 1): measured ceilings, enforced.
//!
//! A budget that only reports is a wish. `xtask perf` measures the
//! three user-visible paths on the pinned corpus fixtures — cold open
//! (file bytes → document model → first composed pixels), pan-zoom
//! frame (one re-render of the page: compose is view-independent,
//! document coordinates are canvas pixels and the shell samples the
//! rendered texture), and canonical save (model → node tree →
//! `.fw.png` bytes, the app-cli save algorithm) — plus the open
//! path's peak memory, measured as `VmHWM` in a fresh child process so
//! the high-water mark belongs to the path, not to everything the
//! gate process touched before it.
//!
//! Every observation is median-of-N after a warm pass (page cache,
//! allocator pools, the lazy font database), and every ceiling in
//! [`BUDGETS`] is enforced in both directions: a measurement without
//! a budget, or a budget without a measurement, fails the gate, so a
//! new scenario cannot silently run unpinned and a stale ceiling
//! cannot outlive its scenario.
//!
//! Ceilings are pinned from observation with headroom (`xtask perf
//! --bless` prints the ready-to-paste block; the devlog records the
//! host class alongside), and they only move down — raising one
//! reopens the milestone that set it, the mirror of the parity
//! floors. Budgets are pinned for the dev profile (the default
//! `cargo run -p xtask` build the gate enforces in); release runs are
//! diagnostics only (`xtask perf --measure`), never compared against
//! dev-pinned ceilings.
//!
//! Beyond the committed fixtures, the gate generates a scale anchor —
//! a 2,000-object vector document (no bitmap payload) built through
//! the same registry commands a host drives (M12 task 2) — and
//! measures the same paths on it, the open path's peak-RSS ceiling
//! included. It lives in the temp directory (`TMPDIR`), never the
//! repo: it is a measurement artifact, not a format anchor.

use crate::util::workspace_root;
use std::hint::black_box;
use std::io::Write as _;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

/// Fixture stems under `corpus/fixtures/`: the 16×16 golden anchor and
/// the 64×48 scripted-autoshape page — small by design, so the budgets
/// measure the paths, not the pixel count.
const FIXTURES: &[&str] = &["composed", "scripted"];

/// Timed runs per scenario; the median is the observation.
const SAMPLES: usize = 7;

/// The scale anchor's object count (M12 task 2): 2,000 vector objects,
/// no bitmap payload — the model dominates, not the pixel buffer.
const STRESS_OBJECTS: usize = 2000;

/// Timed runs per scenario on the scale anchor: each pass costs a
/// full open + render of 2,000 objects, so three is what the gate can
/// afford in `ci`; the median of a deterministic computation is
/// stable at three.
const STRESS_SAMPLES: usize = 3;

/// One enforced ceiling. The name encodes the path, fixture, and unit
/// (`<path>_<stem>_<unit>`); `observed` is the median that pinned the
/// ceiling — the headroom receipt, re-printed by every run so drift
/// since blessing is visible.
struct Budget {
    name: &'static str,
    ceiling: f64,
    observed: f64,
}

/// The pinned ceilings (dev profile). Sealed from observation on an
/// AMD Ryzen 7 5700U (16 cores), taken under a loaded window (host
/// load ~16/16 — the regime CI's co-tenant waves share), so the
/// ceilings clear the slow steady state, not the idle one; see
/// `log/devlog.md`. Ceilings only move down.
const BUDGETS: &[Budget] = &[
    Budget {
        name: "open_composed_us",
        ceiling: 10125.0,
        observed: 3374.8,
    },
    Budget {
        name: "frame_composed_us",
        ceiling: 7153.0,
        observed: 2384.2,
    },
    Budget {
        name: "save_composed_us",
        ceiling: 3090.0,
        observed: 1029.8,
    },
    Budget {
        name: "open_composed_hwm_kib",
        ceiling: 22758.0,
        observed: 15172.0,
    },
    Budget {
        name: "open_scripted_us",
        ceiling: 57992.0,
        observed: 19330.5,
    },
    Budget {
        name: "frame_scripted_us",
        ceiling: 50951.0,
        observed: 16983.6,
    },
    Budget {
        name: "save_scripted_us",
        ceiling: 8844.0,
        observed: 2948.0,
    },
    Budget {
        name: "open_scripted_hwm_kib",
        ceiling: 22140.0,
        observed: 14760.0,
    },
    Budget {
        name: "open_stress_us",
        ceiling: 8527327.0,
        observed: 2842442.2,
    },
    Budget {
        name: "frame_stress_us",
        ceiling: 8217552.0,
        observed: 2739183.9,
    },
    Budget {
        name: "save_stress_us",
        ceiling: 791618.0,
        observed: 263872.5,
    },
    Budget {
        name: "open_stress_hwm_kib",
        ceiling: 34014.0,
        observed: 22676.0,
    },
];

/// One measured scenario.
struct Observation {
    name: String,
    value: f64,
    unit: &'static str,
}

/// Gate entry point: observe, then enforce every ceiling.
pub fn run() -> Result<(), String> {
    if !cfg!(debug_assertions) {
        return Err(
            "perf budgets are pinned for the dev profile — run `cargo run -p xtask -- perf` \
             (release numbers are diagnostics only: `perf --measure`)"
                .to_owned(),
        );
    }
    let observations = observe()?;
    let report = enforce(&observations)?;
    eprintln!("[xtask] perf: {report}");
    Ok(())
}

/// `xtask perf --measure`: print every observation against its pinned
/// budget without enforcing (the release-diagnostics door).
pub fn measure() -> Result<(), String> {
    let observations = observe()?;
    for budget in BUDGETS {
        let Some(o) = observations.iter().find(|o| o.name == budget.name) else {
            continue;
        };
        eprintln!(
            "[xtask] perf {}: observed {:.0} {} (ceiling {:.0}, pinned from {:.0})",
            budget.name, o.value, o.unit, budget.ceiling, budget.observed
        );
    }
    for o in observations {
        if !BUDGETS.iter().any(|b| b.name == o.name) {
            eprintln!(
                "[xtask] perf {}: observed {:.0} {} (unpinned)",
                o.name, o.value, o.unit
            );
        }
    }
    Ok(())
}

/// `xtask perf --bless`: observe and print the ready-to-paste
/// [`BUDGETS`] block — time ceilings at 3× the observed median,
/// memory at 1.5×. Blessing records; the committed constants are the
/// enforcement.
pub fn bless() -> Result<(), String> {
    let observations = observe()?;
    eprintln!("[xtask] perf bless: paste into perf.rs BUDGETS:");
    for o in &observations {
        let headroom = if o.unit == "µs" { 3.0 } else { 1.5 };
        eprintln!(
            "    Budget {{ name: \"{}\", ceiling: {:.1}, observed: {:.1} }},",
            o.name,
            (o.value * headroom).ceil(),
            o.value
        );
    }
    Ok(())
}

/// The child half of the memory probe (`xtask perf-child <file>`):
/// open + first render, nothing else, then one line — this process's
/// `VmHWM` in KiB — on stderr. A fresh process owns no history, so its
/// high-water mark is the open path's peak plus the runtime baseline.
pub fn child(path: &str) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    let loaded = open_document(&bytes)?;
    let render =
        compose::render(&loaded.document, 0, 0).map_err(|e| format!("perf-child render: {e}"))?;
    black_box(&render.pixels);
    eprintln!("{}", vm_hwm_kib()?);
    Ok(())
}

/// Runs every scenario on every fixture, then on the scale anchor.
fn observe() -> Result<Vec<Observation>, String> {
    let mut out = Vec::new();
    for stem in FIXTURES {
        let path = workspace_root()
            .join("corpus/fixtures")
            .join(format!("{stem}.fw.png"));
        measure_case(&mut out, stem, &path, SAMPLES)?;
    }
    // The generated scale anchor: built once per gate run through the
    // registry, opened through the identical read path as a fixture.
    let stress = stress_fixture()?;
    let measured = measure_case(&mut out, "stress", &stress, STRESS_SAMPLES);
    // Best-effort cleanup; the temp tier ages it out regardless.
    let _ = std::fs::remove_file(&stress);
    measured?;
    Ok(out)
}

/// Measures every path on one document file and appends the
/// observations.
fn measure_case(
    out: &mut Vec<Observation>,
    stem: &str,
    path: &Path,
    samples: usize,
) -> Result<(), String> {
    let loaded = {
        let bytes =
            std::fs::read(path).map_err(|e| format!("perf: read {}: {e}", path.display()))?;
        open_document(&bytes).map_err(|e| format!("perf {stem}: {e}"))?
    };
    // Warm pass, untimed: page cache, allocator pools, the lazy
    // font database — the steady state a real user sits in.
    let warm = compose::render(&loaded.document, 0, 0)
        .map_err(|e| format!("perf {stem}: warm render: {e}"))?;
    black_box(&warm.pixels);

    // Cold open: file bytes → model → first pixels, fresh every
    // pass — no document, cache, or tree survives between runs.
    let path_string = path.to_string_lossy().into_owned();
    out.push(Observation {
        name: format!("open_{stem}_us"),
        value: timed(samples, || {
            let bytes = std::fs::read(&path_string).map_err(|e| e.to_string())?;
            let opened = open_document(&bytes)?;
            let render = compose::render(&opened.document, 0, 0).map_err(|e| e.to_string())?;
            Ok(black_box(&render.pixels).len())
        })?,
        unit: "µs",
    });

    // Pan-zoom frame: compose is view-independent, so one
    // re-render of the page is the frame cost; the pan/zoom
    // transform itself is the shell sampling the rendered texture.
    let document = &loaded.document;
    out.push(Observation {
        name: format!("frame_{stem}_us"),
        value: timed(samples, || {
            let render = compose::render(document, 0, 0).map_err(|e| e.to_string())?;
            Ok(black_box(&render.pixels).len())
        })?,
        unit: "µs",
    });

    // Canonical save: the app-cli save path byte for byte.
    let unknown = &loaded.unknown;
    let chunks = &loaded.chunks;
    out.push(Observation {
        name: format!("save_{stem}_us"),
        value: timed(samples, || save_len(document, unknown, chunks))?,
        unit: "µs",
    });

    // Peak memory of the open path, isolated in a fresh child.
    out.push(Observation {
        name: format!("open_{stem}_hwm_kib"),
        value: peak_memory_kib(&path_string)?,
        unit: "KiB",
    });
    Ok(())
}

/// Builds the scale anchor (M12 task 2): a 256×256 page carrying
/// [`STRESS_OBJECTS`] small vector rects laid out on a grid, every one
/// created through `object.createPath` — the same construction path
/// any host drives — then written through the canonical writer into
/// the temp directory (`TMPDIR`, never the repo).
fn stress_fixture() -> Result<std::path::PathBuf, String> {
    let mut registry = engine::CommandRegistry::new();
    engine::catalog::install(&mut registry).map_err(|e| format!("stress doc: catalog: {e}"))?;
    let mut session = engine::Session::with_registry(doc::Document::default(), registry);
    let mut apply = |name: &str, payload: serde_json::Value| -> Result<(), String> {
        session
            .apply(name, payload)
            .map_err(|e| format!("stress doc: {name}: {e}"))
    };
    apply("page.add", serde_json::json!({ "index": null }))?;
    apply(
        "page.setCanvasSize",
        serde_json::json!({ "page": 0, "width": 256.0, "height": 256.0 }),
    )?;
    for i in 0..STRESS_OBJECTS {
        let (col, row) = (i % 50, i / 50);
        apply(
            "object.createPath",
            serde_json::json!({
                "kind": "rect",
                "x": 3.0 + (col as f64) * 5.0,
                "y": 3.0 + (row as f64) * 5.0,
                "width": 4.0, "height": 4.0,
                "page": 0, "layer": 0, "cell": 0, "name": null,
            }),
        )?;
    }
    let (tree, stats) = doc::fwmap::write(session.document(), &doc::fwmap::UnknownBag::default())
        .map_err(|e| format!("stress doc: doc write: {e}"))?;
    if stats.unconsumed_unknown != 0 {
        return Err(format!(
            "stress doc: doc write left {} unknown node(s)",
            stats.unconsumed_unknown
        ));
    }
    let payload = fwpng::nodetree::serialize(&tree);
    let standard = crate::corpus::encode_rgba8(256, 256, |_, _| [255, 255, 255, 255])?;
    let bytes = crate::corpus::insert_before_iend(
        &standard,
        &[
            (b"mkTS", crate::corpus::zlib(&payload)?),
            (b"mkBS", crate::corpus::zlib(&payload)?),
        ],
    )?;
    let path = std::env::temp_dir().join("firecraft-perf-stress.fw.png");
    std::fs::write(&path, &bytes).map_err(|e| format!("write stress doc: {e}"))?;
    Ok(path)
}

/// Median wall time of `samples` runs of `work`, in microseconds.
/// `work` returns the size of what it produced so the optimizer
/// cannot drop the measured work.
fn timed(samples: usize, mut work: impl FnMut() -> Result<usize, String>) -> Result<f64, String> {
    let mut times = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        let produced = work()?;
        times.push(start.elapsed().as_secs_f64() * 1e6);
        black_box(produced);
    }
    times.sort_unstable_by(f64::total_cmp);
    times
        .get(samples / 2)
        .copied()
        .ok_or_else(|| "no samples".to_owned())
}

/// One open, the app-cli shape: everything a later save needs.
struct Loaded {
    document: doc::Document,
    unknown: doc::fwmap::UnknownBag,
    chunks: Vec<fwpng::chunk::RawChunk>,
}

/// The full read path: PNG chunks → `mkTS` node tree → typed document
/// + unknown bag.
fn open_document(bytes: &[u8]) -> Result<Loaded, String> {
    let chunks = fwpng::chunk::parse_chunks(bytes).map_err(|e| format!("parse: {e}"))?;
    let nodes = fwpng::document::parse_mkts(&chunks)
        .map_err(|e| format!("mkTS: {e}"))?
        .ok_or_else(|| "no mkTS node tree in file".to_owned())?;
    let (document, unknown) = doc::fwmap::read(&nodes).map_err(|e| format!("model: {e}"))?;
    Ok(Loaded {
        document,
        unknown,
        chunks,
    })
}

/// Canonical save (the app-cli algorithm): document + bag → canonical
/// tree → zlib → `mkTS` re-embedded into the original chunk stream.
fn save_len(
    document: &doc::Document,
    unknown: &doc::fwmap::UnknownBag,
    chunks: &[fwpng::chunk::RawChunk],
) -> Result<usize, String> {
    let (tree, _stats) =
        doc::fwmap::write(document, unknown).map_err(|e| format!("doc write: {e}"))?;
    let inflated = fwpng::nodetree::serialize(&tree);
    let mut compressed = Vec::new();
    {
        let mut encoder =
            flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::default());
        encoder
            .write_all(&inflated)
            .map_err(|e| format!("deflate: {e}"))?;
        encoder.finish().map_err(|e| format!("deflate: {e}"))?;
    }
    let mut out = Vec::from(fwpng::chunk::PNG_SIGNATURE);
    let mut tree_written = false;
    for chunk in chunks {
        if &chunk.ctype == b"mkTS" {
            if tree_written {
                continue; // one canonical tree, not the split originals
            }
            fwpng::chunk::write_chunk(&mut out, b"mkTS", &compressed);
            tree_written = true;
        } else {
            fwpng::chunk::write_chunk(&mut out, &chunk.ctype, &chunk.data);
        }
    }
    Ok(out.len())
}

/// `VmHWM` (KiB) of a fresh process that runs the open path on `path` —
/// this binary re-exec'd with the probe verb.
fn peak_memory_kib(path: &str) -> Result<f64, String> {
    let output = spawn_verb(&["perf-child", path])?;
    // The child reports the KiB on its first stderr line; the gate's
    // own `ok` receipt follows it.
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .find_map(|line| line.trim().parse::<f64>().ok())
        .ok_or_else(|| "perf-child printed no VmHWM reading".to_owned())
}

/// Spawns this binary with `args`, trying each candidate resolution in
/// turn.
///
/// A candidate counts only when the child both spawned *and* exited
/// clean: a candidate that spawns into a failure must not shadow a
/// later candidate that would have worked.
fn spawn_verb(args: &[&str]) -> Result<std::process::Output, String> {
    let mut failures = Vec::new();
    for exe in probe_exe_candidates() {
        match Command::new(&exe).args(args).output() {
            Ok(output) if output.status.success() => return Ok(output),
            Ok(output) => failures.push(format!(
                "{}: exited {}: {}",
                exe.display(),
                output
                    .status
                    .code()
                    .map_or_else(|| "signal".to_owned(), |c| c.to_string()),
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            Err(e) => failures.push(format!("{}: {e}", exe.display())),
        }
    }
    Err(format!(
        "spawn {}: {}",
        args.first().map_or("self", |v| *v),
        if failures.is_empty() {
            "no executable path to re-exec".to_owned()
        } else {
            failures.join("; ")
        }
    ))
}

/// The paths this process may be re-exec'd from, best first.
///
/// `std::env::current_exe()` resolves through `/proc/self/exe` and so
/// reports the *inode's current path*, not the path this process was
/// launched from. The gate runs nested `cargo` steps (coverage, test)
/// that re-uplift `target/debug/xtask` while this process is still
/// running: the running inode is unlinked, `current_exe()` gains a
/// `" (deleted)"` suffix, and the spawn fails with `ENOENT` — the
/// memory probe cannot run at all.
///
/// `/proc/self/exe` names the inode directly, so it survives the
/// replacement. It is also the honest answer: the probe must measure
/// the code this gate is running, not a build that landed underneath
/// it mid-run.
fn probe_exe_candidates() -> Vec<std::path::PathBuf> {
    let proc_self = std::path::Path::new("/proc/self/exe");
    let proc_self = if proc_self.exists() {
        Some(proc_self.to_path_buf())
    } else {
        None
    };
    let current = std::env::current_exe().ok();
    probe_exe_order(proc_self.as_deref(), current.as_deref())
}

/// The decision core: which re-exec path to try, and in what order.
///
/// The environment is read at the edge ([`probe_exe_candidates`]) and
/// passed in here, so the stale-path case is testable without
/// replacing a live binary.
fn probe_exe_order(
    proc_self: Option<&std::path::Path>,
    current_exe: Option<&std::path::Path>,
) -> Vec<std::path::PathBuf> {
    let mut order: Vec<std::path::PathBuf> = Vec::new();
    for candidate in [proc_self, current_exe].into_iter().flatten() {
        if !order.iter().any(|seen| seen == candidate) {
            order.push(candidate.to_path_buf());
        }
    }
    order
}

/// The hidden self-exec probe: reproduce the CI shape where the gate's
/// own binary is replaced underneath the running gate, then run the
/// memory probe through the same re-exec resolution.
///
/// The gate re-execs itself to measure peak memory in a fresh process.
/// The CI gate runs nested `cargo` steps (coverage, test) that re-uplift
/// `target/debug/xtask` while the gate is still running, so the path the
/// gate was launched from stops resolving and the probe died with
/// `spawn perf-child: No such file or directory (os error 2)`. The
/// resolution prefers `/proc/self/exe`, which names the running inode
/// and survives the replacement; `current_exe()` does not.
///
/// This verb removes the path it was launched from — the running inode
/// stays alive because the caller linked it — and then runs the ordinary
/// memory probe. It fails when the stale path still resolves, because
/// then the replacement was not reproduced and the probe would prove
/// nothing.
///
/// The removed path is this process's own executable path, read at the
/// edge; the verb takes no path to delete, so it cannot be pointed at
/// anything else. Two guards keep it from removing anything of value:
/// the opt-in env var, and the requirement that another link to the
/// same inode exists — the probe never removes the last name of a
/// binary.
pub fn selfexec(doc_path: &str) -> Result<(), String> {
    if std::env::var("FIRECRAFT_PERF_SELFEXEC").as_deref() != Ok("1") {
        return Err(
            "perf-selfexec deletes the binary it runs from — set FIRECRAFT_PERF_SELFEXEC=1 to \
             opt in"
                .to_owned(),
        );
    }
    let exe = std::env::current_exe().map_err(|e| format!("perf-selfexec: current_exe: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(&exe)
            .map_err(|e| format!("perf-selfexec: stat {}: {e}", exe.display()))?;
        if meta.nlink() < 2 {
            return Err(format!(
                "perf-selfexec: {} is the only name of the running binary — refusing to remove it",
                exe.display()
            ));
        }
    }
    std::fs::remove_file(&exe)
        .map_err(|e| format!("perf-selfexec: unlink {}: {e}", exe.display()))?;
    let stale = std::env::current_exe().map_err(|e| format!("perf-selfexec: current_exe: {e}"))?;
    if std::fs::metadata(&stale).is_ok() {
        return Err(format!(
            "perf-selfexec: {} still resolves — the replaced-binary shape was not reproduced",
            stale.display()
        ));
    }
    // The CI failure, reproduced exactly: spawning the stale path is
    // `ENOENT`, which is what the gate reported.
    match Command::new(&stale)
        .arg("perf-child")
        .arg(doc_path)
        .output()
    {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("perf-selfexec: stale spawn failed oddly: {e}")),
        Ok(_) => {
            return Err(
                "perf-selfexec: the stale path still spawned — the replaced-binary shape was not \
                 reproduced"
                    .to_owned(),
            );
        }
    }
    // The repair: the resolution still finds a live executable.
    let kib = peak_memory_kib(doc_path)?;
    eprintln!("[xtask] perf-selfexec: peak {kib:.0} KiB through the replaced binary");
    Ok(())
}

/// This process's resident-set high-water mark, KiB (Linux).
fn vm_hwm_kib() -> Result<f64, String> {
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|e| format!("read /proc/self/status: {e}"))?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            return rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse::<f64>()
                .map_err(|e| format!("VmHWM value: {e}"));
        }
    }
    Err("VmHWM not found in /proc/self/status".to_owned())
}

/// Enforces the budget table in both directions and returns the
/// one-line summary for the gate log.
fn enforce(observations: &[Observation]) -> Result<String, String> {
    let mut errors = Vec::new();
    for budget in BUDGETS {
        match observations.iter().find(|o| o.name == budget.name) {
            Some(o) if o.value <= budget.ceiling => {}
            Some(o) => errors.push(format!(
                "{}: observed {:.0} {} exceeds ceiling {:.0} ({:.2}× over) — a regression, \
                 or the environment shifted; ceilings only move down (raising one reopens \
                 the milestone that set it)",
                budget.name,
                o.value,
                o.unit,
                budget.ceiling,
                o.value / budget.ceiling
            )),
            None => errors.push(format!(
                "{}: budget without a measurement — the scenario is gone, retire the ceiling",
                budget.name
            )),
        }
    }
    for o in observations {
        if !BUDGETS.iter().any(|b| b.name == o.name) {
            errors.push(format!(
                "{}: measured but unpinned — bless it (`perf --bless`) before the gate ships it",
                o.name
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let (host, cores) = host_class();
    let worst = BUDGETS
        .iter()
        .filter_map(|b| {
            observations
                .iter()
                .find(|o| o.name == b.name)
                .map(|o| (b.name, b.ceiling / o.value))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1));
    let headroom = worst
        .map(|(name, x)| format!(", worst headroom {name} {:.1}×", x))
        .unwrap_or_default();
    Ok(format!(
        "{} budget(s) enforced on {host} ({cores} cores, profile {}{})",
        BUDGETS.len(),
        profile(),
        headroom
    ))
}

/// CPU model + logical core count from /proc, or placeholders when
/// unavailable — recorded so a budget failure is diagnosable as
/// regression vs host shift.
fn host_class() -> (String, usize) {
    let Ok(info) = std::fs::read_to_string("/proc/cpuinfo") else {
        return ("unknown host".to_owned(), 0);
    };
    let model = info
        .lines()
        .find_map(|line| line.strip_prefix("model name"))
        .map(|rest| rest.trim_start().trim_start_matches(':').trim())
        .unwrap_or("unknown cpu")
        .to_owned();
    let cores = info
        .lines()
        .filter(|line| line.starts_with("processor"))
        .count();
    (model, cores)
}

/// The profile the gate was built with.
fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "dev"
    } else {
        "release"
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::probe_exe_order;
    use std::path::{Path, PathBuf};

    fn paths(order: &[PathBuf]) -> Vec<String> {
        order.iter().map(|p| p.display().to_string()).collect()
    }

    /// The CI failure: the running binary's path was replaced, so
    /// `current_exe()` names a path that no longer resolves. The
    /// `/proc/self/exe` candidate must therefore be tried first — it
    /// names the inode, which survives the replacement.
    #[test]
    fn the_proc_self_path_is_tried_before_the_current_exe_path() {
        let order = probe_exe_order(
            Some(Path::new("/proc/self/exe")),
            Some(Path::new("/build/target/debug/xtask (deleted)")),
        );
        assert_eq!(
            paths(&order),
            vec![
                "/proc/self/exe".to_owned(),
                "/build/target/debug/xtask (deleted)".to_owned()
            ]
        );
    }

    /// The same path from both sources is one candidate, not two.
    #[test]
    fn a_duplicate_candidate_is_tried_once() {
        let order = probe_exe_order(
            Some(Path::new("/proc/self/exe")),
            Some(Path::new("/proc/self/exe")),
        );
        assert_eq!(paths(&order), vec!["/proc/self/exe".to_owned()]);
    }

    /// Without `/proc` (a non-Linux host) the current path is the only
    /// candidate.
    #[test]
    fn the_current_exe_path_is_the_fallback() {
        let order = probe_exe_order(None, Some(Path::new("/build/target/debug/xtask")));
        assert_eq!(paths(&order), vec!["/build/target/debug/xtask".to_owned()]);
    }

    /// Neither source resolved: there is nothing to re-exec, and the
    /// caller must say so rather than spawn an empty path.
    #[test]
    fn no_candidate_means_no_re_exec() {
        assert!(probe_exe_order(None, None).is_empty());
    }
}
