//! The CLI's document operations: convert and journal replay.

use std::io::Cursor;

use engine::Session;

use crate::CliError;
use crate::io::Loaded;

/// A replayed document plus everything needed to save it.
pub struct Applied {
    /// The replayed session (document, rebuilt undo history).
    pub session: Session,
    /// Unknown-node bag from the opened file (rides along on save).
    pub unknown: doc::fwmap::UnknownBag,
    /// Original chunk stream (mkTS is replaced on save).
    pub chunks: Vec<fwpng::chunk::RawChunk>,
}

/// Canonical re-emit: opens `input` through the full read path and
/// returns the re-emitted `.fw.png` bytes. A file that round-trips on
/// read comes back with a canonically-rewritten `mkTS` and everything
/// else preserved.
pub fn convert(input: &[u8]) -> Result<Vec<u8>, CliError> {
    let loaded = crate::io::open(input)?;
    crate::io::save(&loaded.document, &loaded.unknown, &loaded.chunks)
}

/// Replays a journal (JSON lines) onto an opened document through the
/// engine's restart path — the same registry, transaction folding, and
/// undo reconstruction a restarted UI session gets.
pub fn apply_journal(loaded: Loaded, journal_text: &str) -> Result<Applied, CliError> {
    let session = Session::reopen(loaded.document, journal_text)
        .map_err(|error| CliError(format!("replay: {error}")))?;
    Ok(Applied {
        session,
        unknown: loaded.unknown,
        chunks: loaded.chunks,
    })
}

/// Runs a `.jsf` script against an opened document (M9 task 7's
/// `run-script` verb). The script's queued intents drain through the
/// same registry only when the run completes — a failing or
/// limit-halted script leaves the document untouched and comes back as
/// an error carrying the report. `fs` backs `Files.*`/`runScript`
/// inside the script (desktop: the real file system).
///
/// # Errors
/// [`CliError`] wrapping attach failures and non-completed runs.
#[cfg(feature = "scripting")]
pub fn run_script(
    loaded: Loaded,
    fs: std::rc::Rc<dyn engine::platform::FileSystem>,
    name: &str,
    source: &str,
) -> Result<Applied, CliError> {
    let mut session = Session::new(loaded.document);
    let mut engine = scripting::ScriptEngine::new(scripting::Limits::default())
        .map_err(|error| CliError(format!("script engine: {error}")))?;
    let report = scripting::run_with_file_system(fs, || {
        scripting::run_in_session(&mut session, &mut engine, name, source)
    });
    if report.completed() {
        Ok(Applied {
            session,
            unknown: loaded.unknown,
            chunks: loaded.chunks,
        })
    } else {
        Err(CliError(format!(
            "{name}: {}",
            report
                .message
                .as_deref()
                .unwrap_or("script did not complete")
        )))
    }
}

/// Imports any supported raster stream (`io::raster::decode`) into a
/// fresh single-page document and re-emits it as a canonical `.fw.png`
/// stream. Import-as-open: nothing enters a journal. Animated GIFs
/// (more than one frame) route to the states importer instead — each
/// frame composites into its own state.
///
/// # Errors
/// [`CliError`] wrapping decode or save failures.
pub fn import_to_fwpng(input: &[u8], name: Option<String>) -> Result<Vec<u8>, CliError> {
    if is_animated_gif(input) {
        let imported =
            ::io::gif::decode_animated(input).map_err(|error| CliError(error.to_string()))?;
        for note in &imported.report.notes {
            eprintln!("gif note: {note}");
        }
        return crate::io::save(&imported.document, &doc::fwmap::UnknownBag::default(), &[]);
    }
    let document = ::io::raster::decode_to_document(input, name)
        .map_err(|error| CliError(error.to_string()))?;
    crate::io::save(&document, &doc::fwmap::UnknownBag::default(), &[])
}

/// True when `input` sniffs as GIF and the frame loop yields more than
/// one frame (the animated/static routing line).
fn is_animated_gif(input: &[u8]) -> bool {
    if !input.starts_with(b"GIF8") {
        return false;
    }
    let Ok(mut decoder) = gif::DecodeOptions::new().read_info(Cursor::new(input)) else {
        return false;
    };
    let mut frames = 0usize;
    while frames <= 1 {
        match decoder.read_next_frame() {
            Ok(Some(_)) => frames += 1,
            _ => break,
        }
    }
    frames > 1
}

/// Raster export flags parsed from the CLI (`::io` — the leading
/// path separator reaches the extern `io` codec crate; `crate::io`
/// is this crate's own file-access module).
#[derive(Clone, Copy, Debug, Default)]
pub struct ExportFlags {
    /// Explicit JPEG quality 1..=100. `None` defers to the document's
    /// `EXPv` `JPQi` (page first, then document), then the codec
    /// default.
    pub quality: Option<u8>,
    /// Progressive JPEG scans (explicit flag wins over `EXPv`).
    pub progressive: bool,
    /// Matte for alpha-less formats (explicit flag wins over `EXPv`).
    pub matte: ::io::Matte,
}

/// Renders page 0 / state 0 of `document` through `compose` and
/// encodes it in the format named by `output`'s extension.
///
/// # Errors
/// [`CliError`] wrapping compose, plane, or codec failures, and for
/// unknown extensions.
pub fn export_raster(
    document: &doc::Document,
    output: &str,
    flags: &ExportFlags,
) -> Result<Vec<u8>, CliError> {
    let render =
        compose::render(document, 0, 0).map_err(|error| CliError(format!("render: {error}")))?;
    let straight = compose::straight_alpha(&render.pixels);
    let plane = ::io::Plane::new(u32::from(render.width), u32::from(render.height), &straight)
        .map_err(|error| CliError(error.to_string()))?;

    let extension = output
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let codec_err = |error: ::io::ExportError| CliError(error.to_string());
    // `EXPv` resolution: page 0's export block, falling back to the
    // document's. Explicit flags win over stored values everywhere.
    let export = ::io::expv::for_page(document, 0);
    match extension.as_str() {
        "png" => {
            let options = ::io::expv::png_options(export);
            ::io::png::encode_with(&plane, &options).map_err(codec_err)
        }
        "jpg" | "jpeg" => {
            let stated = ::io::expv::StatedJpeg {
                quality: flags.quality,
                progressive: flags.progressive,
                matte: flags.matte,
            };
            let options = ::io::expv::jpeg_options(export, stated);
            ::io::jpeg::encode(&plane, &options).map_err(codec_err)
        }
        "bmp" => {
            let options = ::io::bmp::BmpOptions {
                matte: flags.matte,
                ..::io::bmp::BmpOptions::default()
            };
            ::io::bmp::encode(&plane, &options).map_err(codec_err)
        }
        "tif" | "tiff" => {
            ::io::tiff::encode(&plane, &::io::tiff::TiffOptions::default()).map_err(codec_err)
        }
        "wbmp" => ::io::wbmp::encode(&plane, flags.matte).map_err(codec_err),
        other => Err(CliError(format!(
            "unsupported export format .{other} (png|jpg|bmp|tiff|wbmp)"
        ))),
    }
}

/// Serializes page 0 / state 0 of `document` to SVG (vector export —
/// the model walks straight to `<path>`/`<text>`/`<image>` elements;
/// no rasterization step).
///
/// # Errors
/// [`CliError`] wrapping the serializer's refusals.
pub fn export_svg(document: &doc::Document) -> Result<Vec<u8>, CliError> {
    let out = ::io::svg::encode(document, &::io::svg::SvgOptions::default())
        .map_err(|error| CliError(error.to_string()))?;
    for note in &out.report.notes {
        eprintln!("svg note: {note}");
    }
    Ok(out.svg.into_bytes())
}

/// Encodes page 0 of `document` as a layered RGB/8 PSD (merged
/// composite + RLE layer channels — the maximize-compatibility shape).
///
/// # Errors
/// [`CliError`] wrapping the encoder's refusals.
pub fn export_psd(document: &doc::Document) -> Result<Vec<u8>, CliError> {
    ::io::psd::encode(document, 0).map_err(|error| CliError(error.to_string()))
}

/// Encodes page 0 of `document` as an animated GIF — one frame per
/// state (a single-state document writes a one-frame GIF). Loop, crop,
/// diff, and dither resolve from the page/document `EXPv` block
/// (defaults: infinite loop, full frames, no dither).
///
/// # Errors
/// [`CliError`] wrapping the encoder's refusals.
pub fn export_gif(document: &doc::Document) -> Result<Vec<u8>, CliError> {
    let options = ::io::expv::gif_options(::io::expv::for_page(document, 0));
    ::io::gif::encode_animated(document, 0, &options).map_err(|error| CliError(error.to_string()))
}

/// Parses a batch `--options` value: either inline JSON or the path of
/// a file holding JSON. Both decode to a partial [`doc::ExportOptions`]
/// — only the fields present are stamped onto each document, so a
/// batch can set (say) JPEG quality while every file keeps its own
/// matte.
///
/// # Errors
/// [`CliError`] on unreadable files or malformed JSON.
pub fn parse_export_options(value: &str) -> Result<doc::ExportOptions, CliError> {
    let text = if value.trim_start().starts_with('{') {
        value.to_owned()
    } else {
        let bytes = std::fs::read(value).map_err(|error| CliError(format!("{value}: {error}")))?;
        String::from_utf8(bytes)
            .map_err(|error| CliError(format!("{value}: not UTF-8: {error}")))?
    };
    serde_json::from_str(&text).map_err(|error| CliError(format!("options: {error}")))
}

/// Result of one [`batch`] entry.
#[derive(Clone, Debug)]
pub struct BatchEntry {
    /// Source file name inside the input directory.
    pub source: String,
    /// Output file name in the output directory.
    pub output: String,
    /// Encoded byte count.
    pub bytes: usize,
}

/// Batch export: every `*.fw.png` directly inside `dir` is opened and
/// written as `format` into `out_dir` (default: alongside the source).
/// `overlay` is stamped onto each document's page-0 `EXPv` before
/// encoding, so per-file stored settings still win where the overlay
/// leaves `None` — this is the selective-quality path: a document
/// carrying slice-level `JPQi` keeps it, a plain one takes the batch
/// default.
///
/// Diagnostics go to stderr per file; the first failure aborts the run
/// (files already written stay written).
///
/// # Errors
/// [`CliError`] on directory listing, unknown format, or the first
/// per-file failure.
pub fn batch(
    fs: &dyn engine::platform::FileSystem,
    dir: &str,
    format: &str,
    options: Option<&doc::ExportOptions>,
    out_dir: Option<&str>,
) -> Result<Vec<BatchEntry>, CliError> {
    let mut names = fs
        .read_dir(dir)
        .map_err(|error| CliError(format!("{dir}: {error}")))?;
    names.retain(|name| name.ends_with(".fw.png"));
    names.sort();
    if names.is_empty() {
        return Err(CliError(format!("{dir}: no .fw.png files")));
    }

    let out_dir = out_dir.unwrap_or(dir);
    let mut written = Vec::new();
    for name in &names {
        let stem = name.strip_suffix(".fw.png").unwrap_or(name);
        let source = format!("{dir}/{name}");
        let output = format!("{out_dir}/{stem}.{format}");
        let bytes = fs
            .read_file(&source)
            .map_err(|error| CliError(format!("{source}: {error}")))?;
        let result = batch_one(&bytes, format, options);
        let encoded = match result {
            Ok(encoded) => encoded,
            Err(error) => {
                eprintln!("[batch] FAILED {name}: {error}");
                return Err(CliError(format!("{name}: {error}")));
            }
        };
        fs.write_file(&output, &encoded)
            .map_err(|error| CliError(format!("{output}: {error}")))?;
        eprintln!("[batch] {name} -> {output} ({} bytes)", encoded.len());
        written.push(BatchEntry {
            source: name.clone(),
            output,
            bytes: encoded.len(),
        });
    }
    Ok(written)
}

/// Opens one document, stamps the overlay, and encodes it in
/// `format` (same format set as [`export_raster`] plus svg/psd/gif).
fn batch_one(
    bytes: &[u8],
    format: &str,
    overlay: Option<&doc::ExportOptions>,
) -> Result<Vec<u8>, CliError> {
    let mut loaded = crate::io::open(bytes)?;
    if let Some(overlay) = overlay {
        stamp(&mut loaded.document, overlay);
    }
    match format {
        "svg" => export_svg(&loaded.document),
        "psd" => export_psd(&loaded.document),
        "gif" => export_gif(&loaded.document),
        _ => {
            let fake_name = format!("out.{format}");
            export_raster(&loaded.document, &fake_name, &ExportFlags::default())
        }
    }
}

/// Fills `None` fields of page 0's `export` from `overlay`, attaching
/// a fresh `ExportOptions` when the document carries none.
fn stamp(document: &mut doc::Document, overlay: &doc::ExportOptions) {
    let Some(page) = document.pages.first_mut() else {
        return;
    };
    let export = page.export.get_or_insert_with(Default::default);
    if export.format.is_none() {
        export.format = overlay.format;
    }
    if export.matte_raw.is_none() {
        export.matte_raw = overlay.matte_raw;
    }
    if export.jpeg_quality.is_none() {
        export.jpeg_quality = overlay.jpeg_quality;
    }
    if export.jpeg_smoothing.is_none() {
        export.jpeg_smoothing = overlay.jpeg_smoothing;
    }
    if export.jpeg_colour.is_none() {
        export.jpeg_colour = overlay.jpeg_colour;
    }
    if export.jpeg_progressive.is_none() {
        export.jpeg_progressive = overlay.jpeg_progressive;
    }
    if export.interlaced.is_none() {
        export.interlaced = overlay.interlaced;
    }
    if export.gif_loss.is_none() {
        export.gif_loss = overlay.gif_loss;
    }
    if export.palette_type.is_none() {
        export.palette_type = overlay.palette_type;
    }
    if export.dither.is_none() {
        export.dither = overlay.dither;
    }
    if export.palette_pruning.is_none() {
        export.palette_pruning = overlay.palette_pruning;
    }
    if export.transparency_type.is_none() {
        export.transparency_type = overlay.transparency_type;
    }
    if export.frame_loop.is_none() {
        export.frame_loop = overlay.frame_loop;
    }
    if export.anim_crop.is_none() {
        export.anim_crop = overlay.anim_crop;
    }
    if export.anim_diff.is_none() {
        export.anim_diff = overlay.anim_diff;
    }
}
