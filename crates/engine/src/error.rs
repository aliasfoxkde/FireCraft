//! Engine failures. One enum, shared by command application, undo/redo
//! and (later) journal replay — every variant names the command or
//! structure involved so scripted batches report precisely.

/// Everything that can go wrong between a surface and a mutated
/// document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineError {
    /// No handler is registered under this catalog id.
    UnknownCommand {
        /// The id that was not found.
        name: String,
    },
    /// A handler already owns this catalog id.
    DuplicateCommand {
        /// The contested id.
        name: String,
    },
    /// The payload did not deserialize into the command's typed
    /// payload.
    BadPayload {
        /// The command whose payload failed to parse.
        name: String,
        /// The deserializer's own message.
        detail: String,
    },
    /// The payload named a target that does not exist (stale object
    /// id, out-of-range page/layer/cell index).
    MissingTarget {
        /// The command that named the missing target.
        name: String,
        /// What was missing.
        detail: String,
    },
    /// The command refuses the current document state (e.g. an out-of
    /// -range opacity).
    InvalidState {
        /// The command that refused.
        name: String,
        /// Why.
        detail: String,
    },
    /// A captured delta could not be applied: the document has drifted
    /// from the lineage the delta was captured in (a history entry
    /// undone against the wrong document, a journal replayed onto a
    /// diverged base).
    DeltaMismatch {
        /// What did not line up.
        detail: String,
    },
    /// Undo or redo was requested with nothing on that stack.
    NothingToUndo,
    /// Redo was requested with nothing on the redo stack.
    NothingToRedo,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::UnknownCommand { name } => write!(f, "unknown command {name:?}"),
            EngineError::DuplicateCommand { name } => {
                write!(f, "command {name:?} is already registered")
            }
            EngineError::BadPayload { name, detail } => {
                write!(f, "bad payload for {name:?}: {detail}")
            }
            EngineError::MissingTarget { name, detail } => {
                write!(f, "{name:?}: missing target ({detail})")
            }
            EngineError::InvalidState { name, detail } => {
                write!(f, "{name:?}: invalid state ({detail})")
            }
            EngineError::DeltaMismatch { detail } => write!(f, "delta mismatch: {detail}"),
            EngineError::NothingToUndo => write!(f, "nothing to undo"),
            EngineError::NothingToRedo => write!(f, "nothing to redo"),
        }
    }
}

impl std::error::Error for EngineError {}
