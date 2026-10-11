//! `firecraft-cli` — the headless FireCraft surface.
//!
//! ```text
//! firecraft-cli info <file.fw.png>
//! firecraft-cli convert <in.fw.png> <out.fw.png>
//! firecraft-cli apply <doc.fw.png> <journal.fwj> [--out <out.fw.png>]
//! firecraft-cli export <in.fw.png> <out.(png|jpg|bmp|tiff|wbmp|svg)>
//! firecraft-cli import <in.raster> <out.fw.png> [name]
//! ```
//!
//! Every operation runs through the same engine registry the GUI uses;
//! there is no side door. Results on stdout, diagnostics on stderr.

// The deny lives in the workspace lints to keep library code silent;
// in the binary, stdout is the product (info summaries).
#![allow(clippy::print_stdout)]

use std::process::ExitCode;

use app_cli::{CliError, info, io, ops};
use engine::platform::FileSystem as _;

const USAGE: &str = "usage: firecraft-cli <info <file>|convert <in> <out>|apply <doc> <journal> [--out <out>]|export <in.fw.png> <out.(png|jpg|bmp|tiff|wbmp|svg)> [--quality <1-100>] [--progressive] [--matte <white|black>]|import <in.raster> <out.fw.png> [name]";

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
                        flags.quality = value
                            .parse()
                            .map_err(|_| CliError(format!("--quality: {value} is not 1-100")))?;
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
            let is_svg = output
                .rsplit('.')
                .next()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));
            let written = if is_svg {
                ops::export_svg(&loaded.document)?
            } else {
                ops::export_raster(&loaded.document, output, &flags)?
            };
            fs.write_file(output, &written)
                .map_err(|error| CliError(error.to_string()))?;
            eprintln!(
                "[firecraft-cli] {input} -> {output} ({} bytes)",
                written.len()
            );
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
        Some("apply") => {
            let (doc_path, journal_path, out) = parse_apply(args)?;
            let bytes = fs
                .read_file(doc_path)
                .map_err(|error| CliError(error.to_string()))?;
            let journal = fs
                .read_file(journal_path)
                .map_err(|error| CliError(error.to_string()))?;
            let journal_text = String::from_utf8(journal)
                .map_err(|error| CliError(format!("journal: {error}")))?;
            let loaded = io::open(&bytes)?;
            let applied = ops::apply_journal(loaded, &journal_text)?;
            match out {
                Some(out_path) => {
                    let written = io::save(
                        applied.session.document(),
                        &applied.unknown,
                        &applied.chunks,
                    )?;
                    fs.write_file(out_path, &written)
                        .map_err(|error| CliError(error.to_string()))?;
                    eprintln!(
                        "[firecraft-cli] {doc_path} + {} entries -> {out_path}",
                        applied.session.journal_len()
                    );
                }
                None => println!("{}", info::summarize(applied.session.document())),
            }
            Ok(())
        }
        _ => {
            eprintln!("{USAGE}");
            Err(CliError("unknown or missing command".to_owned()))
        }
    }
}

fn missing(usage: &'static str) -> impl Fn() -> CliError {
    move || CliError(format!("missing arguments (expected: {usage})"))
}

fn parse_apply(args: &[String]) -> Result<(&str, &str, Option<&str>), CliError> {
    let doc = args
        .get(1)
        .ok_or_else(missing("apply <doc> <journal> [--out <out>]"))?;
    let journal = args
        .get(2)
        .ok_or_else(missing("apply <doc> <journal> [--out <out>]"))?;
    let mut out = None;
    let mut rest = args.iter().skip(3);
    while let Some(flag) = rest.next() {
        if flag == "--out" {
            out = Some(
                rest.next()
                    .ok_or_else(|| CliError("--out needs a path".to_owned()))?
                    .as_str(),
            );
        } else {
            return Err(CliError(format!("unknown flag: {flag}")));
        }
    }
    Ok((doc, journal, out))
}
