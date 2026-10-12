//! Layer commands: add, delete, naming and display.

use doc::{Cell, Document, Layer};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::Delta;
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the layer commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(AddLayer)?;
    registry.register(DeleteLayer)?;
    registry.register(RenameLayer)?;
    registry.register(SetLayerDisplay)
}

/// `layer.add` — insert an empty layer (one unlocked, visible cell per
/// frame). Absent `name` gets the Fireworks-style `Layer {n+1}` from
/// the final position; absent `index` appends.
struct AddLayer;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddLayerPayload {
    /// Page index.
    page: usize,
    /// Name; absent derives from the final position.
    name: Option<String>,
    /// Insert position, bottom-first; absent appends.
    index: Option<usize>,
}

impl TypedCommand for AddLayer {
    type Payload = AddLayerPayload;

    fn name(&self) -> &'static str {
        "layer.add"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.add";
        let page = access::page_mut(document, NAME, payload.page)?;
        let len = page.layers.layers.len();
        let at = payload.index.unwrap_or(len);
        if at > len {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("insert index {at} beyond {len} layers"),
            });
        }
        // One unlocked, visible cell per frame: a new layer must be
        // editable in every state, mirroring `page.add`.
        let frame_count = page.info.frame_count.max(1);
        let cells = (0..frame_count)
            .map(|_| Cell {
                locked: false,
                visible: true,
                elements: Vec::new(),
            })
            .collect();
        let mut layer = Layer {
            cells,
            ..Layer::default()
        };
        layer.name = payload
            .name
            .clone()
            .unwrap_or_else(|| format!("Layer {}", at + 1));
        let before = page.layers.clone();
        page.layers.layers.insert(at, layer);
        Ok(Delta::Layers {
            page: payload.page,
            layers: before,
        })
    }
}

/// `layer.delete` — remove a layer and its elements' containment
/// (objects stay in the registry until the last reference is gone;
/// the registry generation bump invalidates their ids). The last
/// layer of a page cannot be deleted: a page with no layers has
/// nothing to edit.
struct DeleteLayer;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DeleteLayerPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
}

impl TypedCommand for DeleteLayer {
    type Payload = DeleteLayerPayload;

    fn name(&self) -> &'static str {
        "layer.delete"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.delete";
        let page = access::page_mut(document, NAME, payload.page)?;
        if page.layers.layers.len() <= 1 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "cannot delete a page's last layer".to_owned(),
            });
        }
        if payload.layer >= page.layers.layers.len() {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("layer {} absent on page {}", payload.layer, payload.page),
            });
        }
        let before = page.layers.clone();
        page.layers.layers.remove(payload.layer);
        Ok(Delta::Layers {
            page: payload.page,
            layers: before,
        })
    }
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
