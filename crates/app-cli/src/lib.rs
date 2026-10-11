//! Headless FireCraft: the CLI's engine-facing operations, as a library
//! so the integration tests drive exactly what the binary runs.
//!
//! Surfaces rule: nothing here mutates a document except through the
//! [`engine`] command registry — open/save wrap the full `.fw.png`
//! read/write path, and journal replay goes through [`engine::Session`].

pub mod info;
pub mod io;
pub mod ops;

/// Library error: one message, printed to stderr by the binary.
#[derive(Debug)]
pub struct CliError(pub String);

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CliError {}
