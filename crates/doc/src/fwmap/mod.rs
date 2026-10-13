//! Node-tree ⇄ model mapping (read in [`reader`], write in [`writer`]).
//!
//! Path scheme used by the [`UnknownBag`]: a bag entry is keyed by the
//! structural path of the *container* whose child list holds it, plus the
//! child index within that list. Container paths are FourCC chains
//! (`/MKBv/LYLv/LAYv/CLLv/CELv/ELMv`); repetition that is canonical in
//! real files is disambiguated with an index (`/…/ELMv[3]` is the third
//! child of that `ELMv`, `/PDCv[1]` is the second page). The chain form
//! assumes a FourCC container appears at most once per parent — true for
//! every canonical file; malformed duplicates collapse onto the same
//! path, which degrades to re-emission in the first position, never to
//! data loss.

mod reader;
mod unknown;
mod writer;

#[cfg(test)]
mod tests;

pub use reader::{read, read_style_library};
pub use unknown::{UnknownBag, UnknownEntry};
pub use writer::{WriteStats, write, write_style_library};

/// A Fireworks node FourCC (`PTHv`, `OPAi`, …).
pub type FourCC = [u8; 4];

/// Mapping failures. Type-level violations on *known* nodes are errors;
/// anything merely outside the mapped spine is preserved instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapError {
    /// A known node carried a payload of the wrong kind or an unusable
    /// shape (e.g. an int where the model needs a string).
    BadShape {
        /// Path of the container holding the offending node.
        path: String,
        /// The offending node's FourCC.
        node: FourCC,
        /// What the mapper needed.
        expected: &'static str,
        /// What the node actually carried.
        found: &'static str,
    },
    /// A structural list referenced an object id that is not in the
    /// registry (or is stale by generation).
    DanglingId {
        /// Slot index of the dangling id.
        index: u32,
        /// Generation of the dangling id.
        generation: u32,
    },
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapError::BadShape {
                path,
                node,
                expected,
                found,
            } => write!(
                f,
                "bad node shape at {path}: {} expected {expected}, found {found}",
                String::from_utf8_lossy(node),
            ),
            MapError::DanglingId { index, generation } => {
                write!(f, "dangling object id {index} (generation {generation})")
            }
        }
    }
}

impl std::error::Error for MapError {}
