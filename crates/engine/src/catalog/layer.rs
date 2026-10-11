//! Layer commands: naming and display.

use doc::Document;

use crate::access;
use crate::command::TypedCommand;
use crate::delta::Delta;
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the layer commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(RenameLayer)?;
    registry.register(SetLayerDisplay)
}

/// `layer.rename` — set a layer's `LNMs` name.
struct RenameLayer;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RenameLayerPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
    /// New name (may be empty; Fireworks layers are rarely nameless).
    name: String,
}

impl TypedCommand for RenameLayer {
    type Payload = RenameLayerPayload;

    fn name(&self) -> &'static str {
        "layer.rename"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.rename";
        let target = access::layer_mut(document, NAME, payload.page, payload.layer)?;
        let before = target.clone();
        target.name = payload.name.clone();
        Ok(Delta::Layer {
            page: payload.page,
            layer: payload.layer,
            before: Box::new(before),
        })
    }
}

/// `layer.setDisplay` — set a layer's `DISb` display flag (display vs
/// smart-layer-reference display).
struct SetLayerDisplay;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetLayerDisplayPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
    /// New display flag.
    display: bool,
}

impl TypedCommand for SetLayerDisplay {
    type Payload = SetLayerDisplayPayload;

    fn name(&self) -> &'static str {
        "layer.setDisplay"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.setDisplay";
        let target = access::layer_mut(document, NAME, payload.page, payload.layer)?;
        let before = target.clone();
        target.display = payload.display;
        Ok(Delta::Layer {
            page: payload.page,
            layer: payload.layer,
            before: Box::new(before),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::super::testdoc;
    use super::*;
    use crate::command::Command;
    use serde_json::json;

    #[test]
    fn rename_round_trips() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 0, "layer": 1, "name": "Foreground" });
        let delta = RenameLayer.apply_json(&mut document, payload).unwrap();
        assert_eq!(document.pages[0].layers.layers[1].name, "Foreground");
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].layers.layers[1].name, "Layer 1");
    }

    #[test]
    fn set_display_round_trips() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 1, "layer": 0, "display": false });
        let delta = SetLayerDisplay.apply_json(&mut document, payload).unwrap();
        assert!(!document.pages[1].layers.layers[0].display);
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.pages[1].layers.layers[0].display);
    }

    #[test]
    fn out_of_range_layer_is_missing_target() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 0, "layer": 7, "display": true });
        let error = SetLayerDisplay
            .apply_json(&mut document, payload)
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
    }
}
