//! The CLI's document operations: convert and journal replay.

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

/// Raster export flags parsed from the CLI (`::io` — the leading
/// path separator reaches the extern `io` codec crate; `crate::io`
/// is this crate's own file-access module).
#[derive(Clone, Copy, Debug)]
pub struct ExportFlags {
    /// JPEG quality 1..=100.
    pub quality: u8,
    /// Progressive JPEG scans.
    pub progressive: bool,
    /// Matte for alpha-less formats.
    pub matte: ::io::Matte,
}

impl Default for ExportFlags {
    fn default() -> Self {
        Self {
            quality: 90,
            progressive: false,
            matte: ::io::Matte::default(),
        }
    }
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
    match extension.as_str() {
        "png" => ::io::png::encode(&plane).map_err(codec_err),
        "jpg" | "jpeg" => {
            let options = ::io::jpeg::JpegOptions {
                quality: flags.quality,
                progressive: flags.progressive,
                matte: flags.matte,
                ..::io::jpeg::JpegOptions::default()
            };
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
