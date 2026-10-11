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
