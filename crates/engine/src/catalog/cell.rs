//! Cell commands: one state's element-list flags.

use doc::Document;

use crate::access;
use crate::command::TypedCommand;
use crate::delta::Delta;
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the cell commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(SetCellVisible)?;
    registry.register(SetCellLocked)
}

/// `cell.setVisible` — toggle one state's cell visibility.
struct SetCellVisible;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetCellVisiblePayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
    /// Cell index (state number).
    cell: usize,
    /// New visibility.
    visible: bool,
}

impl TypedCommand for SetCellVisible {
    type Payload = SetCellVisiblePayload;

    fn name(&self) -> &'static str {
        "cell.setVisible"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "cell.setVisible";
        let target = access::cell_mut(document, NAME, payload.page, payload.layer, payload.cell)?;
        let before = target.clone();
        target.visible = payload.visible;
        Ok(Delta::Cell {
            page: payload.page,
            layer: payload.layer,
            cell: payload.cell,
            before: Box::new(before),
        })
    }
}

/// `cell.setLocked` — lock or unlock one state's cell.
struct SetCellLocked;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetCellLockedPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
    /// Cell index (state number).
    cell: usize,
    /// New locked flag.
    locked: bool,
}

impl TypedCommand for SetCellLocked {
    type Payload = SetCellLockedPayload;

    fn name(&self) -> &'static str {
        "cell.setLocked"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "cell.setLocked";
        let target = access::cell_mut(document, NAME, payload.page, payload.layer, payload.cell)?;
        let before = target.clone();
        target.locked = payload.locked;
        Ok(Delta::Cell {
            page: payload.page,
            layer: payload.layer,
            cell: payload.cell,
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
    fn set_visible_round_trips() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 1, "layer": 0, "cell": 1, "visible": false });
        let delta = SetCellVisible.apply_json(&mut document, payload).unwrap();
        assert!(!document.pages[1].layers.layers[0].cells[1].visible);
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.pages[1].layers.layers[0].cells[1].visible);
    }

    #[test]
    fn set_locked_round_trips() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 0, "layer": 1, "cell": 0, "locked": true });
        let delta = SetCellLocked.apply_json(&mut document, payload).unwrap();
        assert!(document.pages[0].layers.layers[1].cells[0].locked);
        delta.apply_inverse(&mut document).unwrap();
        assert!(!document.pages[0].layers.layers[1].cells[0].locked);
    }

    #[test]
    fn out_of_range_cell_is_missing_target() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 0, "layer": 0, "cell": 9, "visible": false });
        let error = SetCellVisible
            .apply_json(&mut document, payload)
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
    }
}
