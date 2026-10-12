//! `firecraft-cli` — the headless FireCraft surface.
//!
//! ```text
//! firecraft-cli info <file.fw.png>
//! firecraft-cli convert <in.fw.png> <out.fw.png>
//! firecraft-cli apply <doc.fw.png> <journal.fwj> [--out <out.fw.png>] [--steps <out.fwsteps>]
//! firecraft-cli export <in.fw.png> <out.(png|jpg|bmp|tiff|wbmp|svg)>
//! firecraft-cli import <in.raster> <out.fw.png> [name]
//! firecraft-cli run-script <script.jsf> <doc.fw.png> [--out <out.fw.png>]
//! firecraft-cli serve <doc.fw.png> [--out <out.fw.png>]
//! firecraft-cli mcp <doc.fw.png> [--out <out.fw.png>]
//! ```
//!
//! Every operation runs through the same engine registry the GUI uses;
//! there is no side door. Results on stdout, diagnostics on stderr.
//! `serve` speaks newline-delimited JSON control requests on stdin
//! (native-only, PLANNING §7.4); `mcp` speaks the MCP JSON-RPC 2.0
//! stdio framing over the same registry ids.

// The deny lives in the workspace lints to keep library code silent;
// in the binary, stdout is the product (info summaries).
#![allow(clippy::print_stdout)]

use std::io::BufRead as _;
use std::process::ExitCode;
use std::rc::Rc;

use app_cli::{CliError, channel, info, io, mcp, ops};
use engine::Session;
use engine::platform::FileSystem as _;

const USAGE: &str = "usage: firecraft-cli <info <file>|convert <in> <out>|apply <doc> <journal> [--out <out>] [--steps <out.fwsteps>]|export <in.fw.png> <out.(png|jpg|bmp|tiff|wbmp|svg|psd|gif)> [--quality <1-100>] [--progressive] [--matte <white|black>]|import <in.raster> <out.fw.png> [name]|batch <dir> --format <png|jpg|bmp|tiff|wbmp|svg|psd|gif> [--options <json|file>] [--out <dir>]|export-web <in.fw.png> <out-dir> [--workflow <images|html|css|sprites|properties|pages|states|layers|gif>] [--page <n>] [--state <n>] [--name <base>] [--images-dir <prefix>] [--pct-width <n>] [--no-shim] [--skip-undefined]|run-script <script.jsf> <doc> [--out <out>]|serve <doc> [--out <out>]|mcp <doc> [--out <out>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("firecraft-cli: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), CliError> {
    let fs = io::StdFs;
    match args.first().map(String::as_str) {
        Some("info") => {
            let path = args.get(1).ok_or_else(missing("info <file>"))?;
            let bytes = fs
                .read_file(path)
                .map_err(|error| CliError(error.to_string()))?;
            let loaded = io::open(&bytes)?;
            println!("{}", info::summarize(&loaded.document));
            Ok(())
        }
        Some("convert") => {
            let (input, output) = match (args.get(1), args.get(2)) {
                (Some(input), Some(output)) => (input, output),
                _ => return Err(missing("convert <in> <out>")()),
            };
            let bytes = fs
                .read_file(input)
                .map_err(|error| CliError(error.to_string()))?;
            let converted = ops::convert(&bytes)?;
            fs.write_file(output, &converted)
                .map_err(|error| CliError(error.to_string()))?;
            eprintln!(
                "[firecraft-cli] {input} -> {output} ({} bytes)",
                converted.len()
            );
            Ok(())
        }
        Some("export") => {
            let mut positional = Vec::new();
            let mut flags = ops::ExportFlags::default();
            let mut rest = args.iter().skip(1);
            while let Some(flag) = rest.next() {
                match flag.as_str() {
                    "--quality" => {
                        let value = rest
                            .next()
                            .ok_or_else(|| CliError("--quality needs a value".to_owned()))?;
                        flags.quality =
                            Some(value.parse().map_err(|_| {
                                CliError(format!("--quality: {value} is not 1-100"))
                            })?);
                    }
                    "--progressive" => flags.progressive = true,
                    "--matte" => {
                        let value = rest
                            .next()
                            .ok_or_else(|| CliError("--matte needs white|black".to_owned()))?;
                        flags.matte = match value.as_str() {
                            "white" => ::io::Matte::White,
                            "black" => ::io::Matte::Black,
                            other => return Err(CliError(format!("--matte: unknown {other}"))),
                        };
                    }
                    other => positional.push(other.to_owned()),
                }
            }
            let (input, output) = match (positional.first(), positional.get(1)) {
                (Some(input), Some(output)) => (input.as_str(), output.as_str()),
                _ => return Err(missing(USAGE)()),
            };
            let bytes = fs
                .read_file(input)
                .map_err(|error| CliError(error.to_string()))?;
            let loaded = io::open(&bytes)?;
            let ext = output
                .rsplit('.')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let written = match ext.as_str() {
                "svg" => ops::export_svg(&loaded.document)?,
                "psd" => ops::export_psd(&loaded.document)?,
                "gif" => ops::export_gif(&loaded.document)?,
                _ => ops::export_raster(&loaded.document, output, &flags)?,
            };
            fs.write_file(output, &written)
                .map_err(|error| CliError(error.to_string()))?;
            eprintln!(
                "[firecraft-cli] {input} -> {output} ({} bytes)",
                written.len()
            );
            Ok(())
        }
        Some("export-web") => {
            let mut positional = Vec::new();
            let mut workflow = webexport::Workflow::HtmlAndImages;
            let mut options = webexport::WebExportOptions::default();
            let mut rest = args.iter().skip(1);
            while let Some(flag) = rest.next() {
                let mut value = |name: &str| -> Result<String, CliError> {
                    rest.next()
                        .cloned()
                        .ok_or_else(|| CliError(format!("{name} needs a value")))
                };
                match flag.as_str() {
                    "--workflow" => {
                        let value = value("--workflow")?;
                        workflow = ops::parse_workflow(&value)?;
                    }
                    "--page" => {
                        let value = value("--page")?;
                        options.page = value
                            .parse()
                            .map_err(|_| CliError(format!("--page: {value} is not a number")))?;
                    }
                    "--state" => {
                        let value = value("--state")?;
                        options.state = value
                            .parse()
                            .map_err(|_| CliError(format!("--state: {value} is not a number")))?;
                    }
                    "--name" => {
                        let name = value("--name")?;
                        options.doc_name = name.clone();
                        options.title = name;
                    }
                    "--images-dir" => options.images_dir = value("--images-dir")?,
                    "--pct-width" => {
                        let value = value("--pct-width")?;
                        options.table_pct_width = Some(value.parse().map_err(|_| {
                            CliError(format!("--pct-width: {value} is not a number"))
                        })?);
                    }
                    "--no-shim" => options.shim = false,
                    "--skip-undefined" => options.include_undefined = false,
                    other => positional.push(other.to_owned()),
                }
            }
            let (input, out_dir) = match (positional.first(), positional.get(1)) {
                (Some(input), Some(out_dir)) => (input.as_str(), out_dir.as_str()),
                _ => return Err(missing("export-web <in.fw.png> <out-dir> [flags]")()),
            };
            let bytes = fs
                .read_file(input)
                .map_err(|error| CliError(error.to_string()))?;
            let loaded = io::open(&bytes)?;
            let export = ops::export_web(&loaded.document, &options, workflow)?;
            let write = |name: &str, data: &[u8]| -> Result<String, CliError> {
                let path = std::path::Path::new(out_dir)
                    .join(name)
                    .to_string_lossy()
                    .into_owned();
                fs.write_file(&path, data)
                    .map_err(|error| CliError(error.to_string()))?;
                Ok(path)
            };
            let mut count = 0;
            for file in &export.files {
                write(&file.name, &file.bytes)?;
                count += 1;
            }
            if let Some(html) = &export.html {
                let stem = format!("{}.html", options.doc_name);
                write(&stem, html.as_bytes())?;
                count += 1;
            }
            eprintln!("[firecraft-cli] {input} -> {out_dir} ({count} files)");
            Ok(())
        }
        Some("import") => {
            let (input, output) = match (args.get(1), args.get(2)) {
                (Some(input), Some(output)) => (input.as_str(), output.as_str()),
                _ => return Err(missing("import <in.raster> <out.fw.png> [name]")()),
            };
            // Default the object name to the input file's stem.
            let name = args.get(3).cloned().or_else(|| {
                std::path::Path::new(input)
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            });
            let bytes = fs
                .read_file(input)
                .map_err(|error| CliError(error.to_string()))?;
            let written = ops::import_to_fwpng(&bytes, name)?;
            fs.write_file(output, &written)
                .map_err(|error| CliError(error.to_string()))?;
            eprintln!(
                "[firecraft-cli] {input} -> {output} ({} bytes)",
                written.len()
            );
            Ok(())
        }
        Some("batch") => {
            let mut positional = Vec::new();
            let mut format: Option<String> = None;
            let mut options: Option<String> = None;
            let mut out_dir: Option<String> = None;
            let mut rest = args.iter().skip(1);
            while let Some(flag) = rest.next() {
                match flag.as_str() {
                    "--format" => {
                        format = Some(
                            rest.next()
                                .ok_or_else(|| CliError("--format needs a value".to_owned()))?
                                .to_ascii_lowercase(),
                        );
                    }
                    "--options" => {
                        options = Some(
                            rest.next()
                                .ok_or_else(|| CliError("--options needs a value".to_owned()))?
                                .to_owned(),
                        );
                    }
                    "--out" => {
                        out_dir = Some(
                            rest.next()
                                .ok_or_else(|| CliError("--out needs a path".to_owned()))?
                                .to_owned(),
                        );
                    }
                    other => positional.push(other.to_owned()),
                }
            }
            let dir = positional.first().ok_or_else(missing(
                "batch <dir> --format <f> [--options <json>] [--out <dir>]",
            ))?;
            let format = format
                .as_deref()
                .ok_or_else(|| CliError("batch: --format is required".to_owned()))?;
            let overlay = match options.as_deref() {
                Some(value) => Some(ops::parse_export_options(value)?),
                None => None,
            };
            let written = ops::batch(&fs, dir, format, overlay.as_ref(), out_dir.as_deref())?;
            eprintln!(
                "[firecraft-cli] batch: {} file(s) exported as {format}",
                written.len()
            );
            Ok(())
        }
        Some("apply") => {
            let args = parse_apply(args)?;
            let bytes = fs
                .read_file(args.doc)
                .map_err(|error| CliError(error.to_string()))?;
            let journal = fs
                .read_file(args.journal)
                .map_err(|error| CliError(error.to_string()))?;
            let journal_text = String::from_utf8(journal)
                .map_err(|error| CliError(format!("journal: {error}")))?;
            let loaded = io::open(&bytes)?;
            let applied = ops::apply_journal(loaded, &journal_text)?;
            // `--steps`: the replayed session's journal as a `.fwsteps`
            // command file (Save Steps As Command, headless form).
            if let Some(steps_path) = args.steps {
                let text = applied.session.journal_lines();
                fs.write_file(steps_path, text.as_bytes())
                    .map_err(|error| CliError(error.to_string()))?;
                eprintln!(
                    "[firecraft-cli] {} -> {steps_path} ({} rows)",
                    args.journal,
                    applied.session.journal_len()
                );
            }
            match args.out {
                Some(out_path) => {
                    let written = io::save(
                        applied.session.document(),
                        &applied.unknown,
                        &applied.chunks,
                    )?;
                    fs.write_file(out_path, &written)
                        .map_err(|error| CliError(error.to_string()))?;
                    eprintln!(
                        "[firecraft-cli] {} + {} entries -> {out_path}",
                        args.doc,
                        applied.session.journal_len()
                    );
                }
                None => println!("{}", info::summarize(applied.session.document())),
            }
            Ok(())
        }
        #[cfg(feature = "scripting")]
        Some("run-script") => {
            let parsed =
                parse_session_args(args, "run-script <script.jsf> <doc> [--out <out>]", 2)?;
            let script_path = parsed.positional[0];
            let doc_path = parsed.positional[1];
            let script_bytes = fs
                .read_file(script_path)
                .map_err(|error| CliError(error.to_string()))?;
            let source = String::from_utf8(script_bytes)
                .map_err(|error| CliError(format!("{script_path}: {error}")))?;
            let bytes = fs
                .read_file(doc_path)
                .map_err(|error| CliError(error.to_string()))?;
            let loaded = io::open(&bytes)?;
            let applied = ops::run_script(loaded, Rc::new(io::StdFs), script_path, &source)?;
            finish_session(applied, parsed.out)
        }
        Some("serve") => {
            let parsed = parse_session_args(args, "serve <doc> [--out <out>]", 1)?;
            let doc_path = parsed.positional[0];
            let bytes = fs
                .read_file(doc_path)
                .map_err(|error| CliError(error.to_string()))?;
            let loaded = io::open(&bytes)?;
            let io::Loaded {
                document,
                unknown,
                chunks,
            } = loaded;
            let mut session = Session::new(document);
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let line = line.map_err(|error| CliError(format!("stdin: {error}")))?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let reply = match serde_json::from_str(trimmed) {
                    Ok(value) => channel::handle(&mut session, &value),
                    Err(error) => {
                        serde_json::json!({ "ok": false, "error": format!("bad JSON: {error}") })
                    }
                };
                println!("{reply}");
            }
            save_session(&session, &unknown, &chunks, parsed.out)?;
            Ok(())
        }
        Some("mcp") => {
            let parsed = parse_session_args(args, "mcp <doc> [--out <out>]", 1)?;
            let doc_path = parsed.positional[0];
            let bytes = fs
                .read_file(doc_path)
                .map_err(|error| CliError(error.to_string()))?;
            let loaded = io::open(&bytes)?;
            let io::Loaded {
                document,
                unknown,
                chunks,
            } = loaded;
            let mut session = Session::new(document);
            let fs: Rc<dyn engine::platform::FileSystem> = Rc::new(io::StdFs);
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let line = line.map_err(|error| CliError(format!("stdin: {error}")))?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let reply = match serde_json::from_str(trimmed) {
                    Ok(value) => mcp::handle(&mut session, fs.clone(), &value),
                    Err(error) => Some(mcp::parse_error(&error.to_string())),
                };
                if let Some(reply) = reply {
                    println!("{reply}");
                }
            }
            save_session(&session, &unknown, &chunks, parsed.out)?;
            Ok(())
        }
        _ => {
            eprintln!("{USAGE}");
            Err(CliError("unknown or missing command".to_owned()))
        }
    }
}

/// The saved result of a session-shaped verb: save to `out` when given,
/// otherwise summarize on stdout. (Only `run-script` returns an
/// [`ops::Applied`] today, so this rides its feature gate.)
#[cfg_attr(not(feature = "scripting"), allow(dead_code))]
fn finish_session(applied: ops::Applied, out: Option<&str>) -> Result<(), CliError> {
    match out {
        Some(out_path) => {
            let fs = io::StdFs;
            let written = io::save(
                applied.session.document(),
                &applied.unknown,
                &applied.chunks,
            )?;
            fs.write_file(out_path, &written)
                .map_err(|error| CliError(error.to_string()))?;
            eprintln!(
                "[firecraft-cli] -> {out_path} ({} bytes, {} journal rows)",
                written.len(),
                applied.session.journal_len()
            );
        }
        None => println!("{}", info::summarize(applied.session.document())),
    }
    Ok(())
}

/// Saves a live session's document (the `serve`/`mcp` EOF path) when
/// `out` is given; otherwise nothing is written. `unknown`/`chunks`
/// ride along from the opened file, as in every other save path.
fn save_session(
    session: &Session,
    unknown: &doc::fwmap::UnknownBag,
    chunks: &[fwpng::chunk::RawChunk],
    out: Option<&str>,
) -> Result<(), CliError> {
    let Some(out_path) = out else {
        return Ok(());
    };
    let fs = io::StdFs;
    let written = io::save(session.document(), unknown, chunks)?;
    fs.write_file(out_path, &written)
        .map_err(|error| CliError(error.to_string()))?;
    eprintln!(
        "[firecraft-cli] -> {out_path} ({} bytes, {} journal rows)",
        written.len(),
        session.journal_len()
    );
    Ok(())
}

/// Positional + `--out` parsing for the session-shaped verbs
/// (`run-script`, `serve`, `mcp`): exactly `positional_count` positionals
/// in front, then optional `--out <path>`.
struct SessionArgs<'a> {
    positional: Vec<&'a str>,
    out: Option<&'a str>,
}

fn parse_session_args<'a>(
    args: &'a [String],
    usage: &'static str,
    positional_count: usize,
) -> Result<SessionArgs<'a>, CliError> {
    let mut positional = Vec::new();
    let mut out = None;
    let mut rest = args.iter().skip(1);
    while let Some(item) = rest.next() {
        match item.as_str() {
            "--out" => {
                out = Some(
                    rest.next()
                        .ok_or_else(|| CliError("--out needs a path".to_owned()))?
                        .as_str(),
                );
            }
            other if positional.len() < positional_count => positional.push(other),
            other => {
                let _ = other;
                return Err(missing(usage)());
            }
        }
    }
    if positional.len() != positional_count {
        return Err(missing(usage)());
    }
    Ok(SessionArgs { positional, out })
}

fn missing(usage: &'static str) -> impl Fn() -> CliError {
    move || CliError(format!("missing arguments (expected: {usage})"))
}

/// Parsed `apply` arguments: the base document, the journal/steps
/// file, and the two optional outputs.
struct ApplyArgs<'a> {
    doc: &'a str,
    journal: &'a str,
    out: Option<&'a str>,
    steps: Option<&'a str>,
}

fn parse_apply(args: &[String]) -> Result<ApplyArgs<'_>, CliError> {
    let usage = "apply <doc> <journal> [--out <out>] [--steps <out.fwsteps>]";
    let doc = args.get(1).ok_or_else(missing(usage))?;
    let journal = args.get(2).ok_or_else(missing(usage))?;
    let mut out = None;
    let mut steps = None;
    let mut rest = args.iter().skip(3);
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--out" => {
                out = Some(
                    rest.next()
                        .ok_or_else(|| CliError("--out needs a path".to_owned()))?
                        .as_str(),
                );
            }
            "--steps" => {
                steps = Some(
                    rest.next()
                        .ok_or_else(|| CliError("--steps needs a path".to_owned()))?
                        .as_str(),
                );
            }
            other => return Err(CliError(format!("unknown flag: {other}"))),
        }
    }
    Ok(ApplyArgs {
        doc,
        journal,
        out,
        steps,
    })
}
