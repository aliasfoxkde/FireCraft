//! FireCraft editor engine (`engine`, L3): everything that mutates a
//! document goes through one command registry.
//!
//! The model in [`doc`] is plain data; this crate turns edits into
//! *commands*: named, serde-serializable payloads applied by handlers
//! that capture an inverse [`Delta`] before mutating. Surfaces — the
//! egui shell, the CLI, journal replay, later the scripting host — all
//! call [`CommandRegistry::execute`] with the same catalog ids, so the
//! same operation sequence produces the same document from anywhere.
//!
//! A failed command leaves the document untouched: handlers capture
//! before they mutate and validate before they capture.

pub mod access;
pub mod bitmap;
pub mod brush;
pub mod catalog;
pub mod command;
pub mod delta;
pub mod error;
pub mod fuzzdrive;
pub mod history;
pub mod journal;
pub mod mask;
pub mod platform;
pub mod registry;
pub mod session;

pub use command::{Command, TypedCommand};
pub use delta::{Delta, ObjectInverse};
pub use error::EngineError;
pub use history::{Call, History, Step};
pub use journal::{Journal, JournalEntry};
pub use registry::CommandRegistry;
pub use session::{Session, TransactionGuard};
