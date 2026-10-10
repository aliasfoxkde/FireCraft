//! The name-keyed command registry — the single mutation door.
//!
//! Surfaces never mutate a `Document` directly; they call
//! [`CommandRegistry::execute`] with a catalog id and a payload. The
//! registry is additive: built-ins come from [`crate::catalog`],
//! runtime registrations (scripting host, later milestones) use the
//! same door and cannot shadow a catalog id.

use std::collections::HashMap;

use doc::Document;

use crate::command::Command;
use crate::delta::Delta;
use crate::error::EngineError;

/// Every registered command, keyed by catalog id.
#[derive(Default)]
pub struct CommandRegistry {
    handlers: HashMap<&'static str, Box<dyn Command>>,
}

impl CommandRegistry {
    /// An empty registry (use [`crate::catalog::builtins`] for the
    /// stock set).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler under its own catalog id. Re-registering an
    /// id is an error — shadowing a catalog command would break
    /// surface equivalence and journal replay.
    pub fn register(&mut self, command: impl Command + 'static) -> Result<(), EngineError> {
        if self.handlers.contains_key(command.name()) {
            return Err(EngineError::DuplicateCommand {
                name: command.name().to_owned(),
            });
        }
        self.handlers.insert(command.name(), Box::new(command));
        Ok(())
    }

    /// Applies one command by catalog id.
    pub fn execute(
        &self,
        document: &mut Document,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<Delta, EngineError> {
        let handler = self
            .handlers
            .get(name)
            .ok_or_else(|| EngineError::UnknownCommand {
                name: name.to_owned(),
            })?;
        handler.apply_json(document, payload)
    }

    /// Registered catalog ids, sorted (drives menus and CLI help).
    #[must_use]
    pub fn names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self.handlers.keys().copied().collect();
        names.sort_unstable();
        names
    }

    /// `true` when `name` is registered.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.handlers.contains_key(name)
    }

    /// Number of registered commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// `true` when nothing is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field("commands", &self.names())
            .finish()
    }
}
