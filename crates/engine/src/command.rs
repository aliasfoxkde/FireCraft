//! The command contract: typed payloads in, inverse deltas out.
//!
//! Handlers implement [`TypedCommand`] (typed payload, checked
//! mutation); the registry stores them behind the object-safe
//! [`Command`] wrapper that deserializes the payload JSON. A handler
//! must capture its delta before it mutates and validate before it
//! captures — a failing command leaves the document untouched.

use doc::Document;

use crate::delta::Delta;
use crate::error::EngineError;

/// Object-safe front the registry dispatches through.
pub trait Command: Send + Sync {
    /// The catalog id, e.g. `"object.setOpacity"` — stable across
    /// surfaces (UI, CLI, journal, scripting host).
    fn name(&self) -> &'static str;

    /// Applies the (already name-dispatched) payload to `document`.
    fn apply_json(
        &self,
        document: &mut Document,
        payload: serde_json::Value,
    ) -> Result<Delta, EngineError>;
}

/// The ergonomic form a handler actually implements.
pub trait TypedCommand: Send + Sync {
    /// The command's payload type. Deserialized with
    /// `deny_unknown_fields` semantics enforced by the payload types
    /// themselves, so a typo'd journal key cannot silently no-op.
    type Payload: serde::Serialize + for<'de> serde::Deserialize<'de>;

    /// The catalog id.
    fn name(&self) -> &'static str;

    /// Validates the payload against `document`, captures the inverse,
    /// mutates. Errors must leave `document` untouched.
    fn apply(&self, document: &mut Document, payload: &Self::Payload)
    -> Result<Delta, EngineError>;
}

impl<C: TypedCommand> Command for C {
    fn name(&self) -> &'static str {
        TypedCommand::name(self)
    }

    fn apply_json(
        &self,
        document: &mut Document,
        payload: serde_json::Value,
    ) -> Result<Delta, EngineError> {
        let typed: C::Payload =
            serde_json::from_value(payload).map_err(|error| EngineError::BadPayload {
                name: self.name().to_owned(),
                detail: error.to_string(),
            })?;
        TypedCommand::apply(self, document, &typed)
    }
}
