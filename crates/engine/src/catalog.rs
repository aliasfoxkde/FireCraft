//! Built-in command catalog.
//!
//! Every command id is a stable surface for UI, CLI, journal, and (M9)
//! scripting. Payloads use `deny_unknown_fields` — a typo'd key is a
//! rejected payload, never a silently ignored one.

use doc::{Document, ObjectData};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the built-in catalog into `registry`.
pub fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(SetObjectOpacity)?;
    registry.register(SetCellVisible)?;
    registry.register(RenameLayer)
}

/// Every registered built-in, sorted (surfaces enumerate this).
#[must_use]
pub fn ids() -> Vec<&'static str> {
    let mut registry = CommandRegistry::new();
    let _ = install(&mut registry);
    registry.names()
}

/// `object.setOpacity` — per-mille opacity (0..1000) on the object
/// kinds that carry `OPAi` (paths, groups, images).
struct SetObjectOpacity;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetObjectOpacityPayload {
    /// Target object.
    id: doc::ObjectId,
    /// New opacity, per mille.
    opacity: i64,
}

impl TypedCommand for SetObjectOpacity {
    type Payload = SetObjectOpacityPayload;

    fn name(&self) -> &'static str {
        "object.setOpacity"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.setOpacity";
        if !(0..=1000).contains(&payload.opacity) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("opacity {} outside 0..1000 per mille", payload.opacity),
            });
        }
        if !matches!(
            access::object(document, NAME, payload.id)?,
            ObjectData::Path(_) | ObjectData::Group(_) | ObjectData::Image(_)
        ) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} does not carry opacity", payload.id.index()),
            });
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let id = payload.id;
        let applied = match document.registry.get_mut(id) {
            Some(ObjectData::Path(path)) => {
                path.opacity = payload.opacity;
                true
            }
            Some(ObjectData::Group(group)) => {
                group.opacity = payload.opacity;
                true
            }
            Some(ObjectData::Image(image)) => {
                image.opacity = payload.opacity;
                true
            }
            _ => false,
        };
        if !applied {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} does not carry opacity", id.index()),
            });
        }
        Ok(Delta::Object {
            id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::command::Command;
    use doc::{Contour, ObjectId, PathObject, PathPoint};

    fn point(x: f64, y: f64) -> PathPoint {
        PathPoint {
            anchor: geom::Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        }
    }

    fn path_object() -> ObjectData {
        ObjectData::Path(PathObject {
            contours: vec![Contour {
                closed: true,
                point_count: 4,
                points: vec![
                    point(0.0, 0.0),
                    point(8.0, 0.0),
                    point(8.0, 8.0),
                    point(0.0, 8.0),
                ],
            }],
            primitive: None,
            even_odd: false,
            paint: None,
            fill_start: None,
            fill_end: None,
            fill_third: None,
            brush_placement: None,
            fill_on_top: None,
            locked: false,
            visible: true,
            opacity: 1000,
            blend: 0,
            name: None,
        })
    }

    fn document_with_path() -> (Document, ObjectId) {
        let mut document = Document::default();
        let id = document.registry.insert(path_object());
        (document, id)
    }

    #[test]
    fn set_opacity_applies_and_captures() {
        let (mut document, id) = document_with_path();
        let payload = serde_json::json!({ "id": id, "opacity": 400 });
        let delta = SetObjectOpacity.apply_json(&mut document, payload).unwrap();
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => assert_eq!(path.opacity, 400),
            other => panic!("wrong object kind: {other:?}"),
        }
        delta.apply_inverse(&mut document).unwrap();
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => assert_eq!(path.opacity, 1000),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn payload_json_round_trips() {
        let (document, id) = document_with_path();
        let payload = serde_json::json!({ "id": id, "opacity": 250 });
        let bytes = serde_json::to_vec(&payload).unwrap();
        let parsed: SetObjectOpacityPayload = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed.opacity, 250);
        assert!(document.registry.get(parsed.id).is_some());
    }

    #[test]
    fn unknown_payload_field_is_rejected() {
        let (mut document, id) = document_with_path();
        let payload = serde_json::json!({ "id": id, "opasity": 400 });
        let error = SetObjectOpacity
            .apply_json(&mut document, payload)
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }));
    }

    #[test]
    fn out_of_range_opacity_is_invalid_state() {
        let (mut document, id) = document_with_path();
        let payload = serde_json::json!({ "id": id, "opacity": 1001 });
        let error = SetObjectOpacity
            .apply_json(&mut document, payload)
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => assert_eq!(path.opacity, 1000, "document untouched"),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn stale_id_is_missing_target() {
        let (mut document, id) = document_with_path();
        document.registry.remove(id).expect("remove");
        let payload = serde_json::json!({ "id": id, "opacity": 400 });
        let error = SetObjectOpacity
            .apply_json(&mut document, payload)
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
    }

    #[test]
    fn opacity_on_kind_without_field_is_invalid() {
        let mut document = Document::default();
        let id = document
            .registry
            .insert(ObjectData::Slice(doc::SliceObject {
                internal_name: None,
                frame: geom::Rect::new(0.0, 0.0, 4.0, 4.0),
                shape: 0,
                fill_raw: 0,
                name: None,
                slice_type: None,
            }));
        let payload = serde_json::json!({ "id": id, "opacity": 400 });
        let error = SetObjectOpacity
            .apply_json(&mut document, payload)
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }
}
