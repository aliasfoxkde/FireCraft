//! Performance budgets (M12 task 1): measured ceilings, enforced.
//!
//! A budget that only reports is a wish. `xtask perf` measures the
//! three user-visible paths on the pinned corpus fixtures — cold open
//! (file bytes → document model → first composed pixels), pan-zoom
//! frame (one re-render of the page: compose is view-independent,
//! document coordinates are canvas pixels and the shell samples the
//! rendered texture), and canonical save (model → node tree →
//! `.fw.png` bytes, the app-cli save algorithm) — plus the open
//! path's peak memory, measured as VmHWM in a fresh child process so
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

use crate::util::workspace_root;
use std::hint::black_box;
use std::io::Write as _;
use std::process::Command;
use std::time::Instant;

/// Fixture stems under `corpus/fixtures/`: the 16×16 golden anchor and
/// the 64×48 scripted-autoshape page — small by design, so the budgets
/// measure the paths, not the pixel count.
const FIXTURES: &[&str] = &["composed", "scripted"];

/// Timed runs per scenario; the median is the observation.
const SAMPLES: usize = 7;

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
/// AMD Ryzen 7 5700U (16 cores) — see `log/devlog.md`; ceilings only
/// move down.
const BUDGETS: &[Budget] = &[
    Budget {
        name: "open_composed_us",
        ceiling: 5073.0,
        observed: 1690.7,
    },
    Budget {
        name: "frame_composed_us",
        ceiling: 3772.0,
        observed: 1257.3,
    },
    Budget {
        name: "save_composed_us",
        ceiling: 1563.0,
        observed: 520.8,
    },
    Budget {
        name: "open_composed_hwm_kib",
        ceiling: 22404.0,
        observed: 14936.0,
    },
    Budget {
        name: "open_scripted_us",
        ceiling: 30929.0,
        observed: 10309.4,
    },
    Budget {
        name: "frame_scripted_us",
        ceiling: 28566.0,
        observed: 9521.7,
    },
    Budget {
        name: "save_scripted_us",
        ceiling: 4077.0,
        observed: 1358.8,
    },
    Budget {
        name: "open_scripted_hwm_kib",
        ceiling: 21972.0,
        observed: 14648.0,
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
/// VmHWM in KiB — on stderr. A fresh process owns no history, so its
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

/// Runs every scenario on every fixture.
fn observe() -> Result<Vec<Observation>, String> {
    let mut out = Vec::new();
    for stem in FIXTURES {
        let path = workspace_root()
            .join("corpus/fixtures")
            .join(format!("{stem}.fw.png"));
        let loaded = {
            let bytes =
                std::fs::read(&path).map_err(|e| format!("perf: read {}: {e}", path.display()))?;
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
            value: timed(SAMPLES, || {
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
            value: timed(SAMPLES, || {
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
            value: timed(SAMPLES, || save_len(document, unknown, chunks))?,
            unit: "µs",
        });

        // Peak memory of the open path, isolated in a fresh child.
        out.push(Observation {
            name: format!("open_{stem}_hwm_kib"),
            value: peak_memory_kib(&path_string)?,
            unit: "KiB",
        });
    }
    Ok(out)
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

/// VmHWM (KiB) of a fresh process that runs the open path on `path` —
/// this binary re-exec'd with the probe verb.
fn peak_memory_kib(path: &str) -> Result<f64, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let output = Command::new(exe)
        .args(["perf-child", path])
        .output()
        .map_err(|e| format!("spawn perf-child: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "perf-child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    // The child reports the KiB on its first stderr line; the gate's
    // own `ok` receipt follows it.
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .find_map(|line| line.trim().parse::<f64>().ok())
        .ok_or_else(|| "perf-child printed no VmHWM reading".to_owned())
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
