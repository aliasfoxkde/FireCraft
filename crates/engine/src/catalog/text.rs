//! Text commands (`text.*`): insert a text object into a cell and edit
//! its characters and attributes.
//!
//! Like `autoshape.*`, insertion validates before the first mutation.
//! Attribute edits capture the whole object before mutating, so every
//! payload (accepted or refused) leaves the journal invertible. The
//! layout itself lives in the `text` crate — these commands only move
//! the model; nothing here shapes glyphs.

use doc::{Document, ObjectData, TextObject, TextRun, TextStyle};
use geom::Rect;

use crate::access;
use crate::catalog::object::attach_cell;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the text commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(CreateText)?;
    registry.register(SetText)?;
    registry.register(SetAttrs)?;
    registry.register(SetFont)
}

/// Edits the text object `id` names, capturing it for the inverse.
/// `MissingTarget` when the id is stale or names a non-text object.
fn mutate_text(
    document: &mut Document,
    name: &'static str,
    id: doc::ObjectId,
    edit: impl FnOnce(&mut TextObject),
) -> Result<Delta, EngineError> {
    let before = access::object(document, name, id)?.clone();
    let ObjectData::Text(object) = document
        .registry
        .get_mut(id)
        .ok_or_else(|| missing(name, id))?
    else {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("object {} is not a text object", id.index()),
        });
    };
    edit(object);
    Ok(Delta::Object {
        id,
        inverse: ObjectInverse::Mutated {
            object: Box::new(before),
        },
    })
}

fn missing(name: &'static str, id: doc::ObjectId) -> EngineError {
    EngineError::MissingTarget {
        name: name.to_owned(),
        detail: format!(
            "object {} (generation {}) is not live",
            id.index(),
            id.generation()
        ),
    }
}

/// The style slot of a text object, creating the default style on first
/// write — an attribute command implies the object now carries style.
fn style_of(object: &mut TextObject) -> &mut TextStyle {
    object.style.get_or_insert_with(TextStyle::default)
}

/// `text.createText` — insert a text object carrying `text` at `frame`.
struct CreateText;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct CreateTextPayload {
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Frame origin and size, world coordinates.
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    /// The initial characters (one run, one style).
    text: String,
    /// Optional object name (`OBNs`).
    name: Option<String>,
}

impl TypedCommand for CreateText {
    type Payload = CreateTextPayload;

    fn name(&self) -> &'static str {
        "text.createText"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "text.createText";
        let frame = Rect::new(
            payload.x,
            payload.y,
            payload.x + payload.width,
            payload.y + payload.height,
        );
        if ![frame.x0, frame.y0, frame.x1, frame.y1]
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "frame must be finite".to_owned(),
            });
        }
        access::cell_mut(document, NAME, payload.page, payload.layer, payload.cell)?;

        let mut object = TextObject::new(frame);
        if !payload.text.is_empty() {
            object.runs = vec![TextRun {
                text: payload.text.clone(),
            }];
        }
        object.name = payload.name.clone();
        let id = document.registry.insert(ObjectData::Text(object));
        let placed = attach_cell(
            document,
            NAME,
            payload.page,
            payload.layer,
            payload.cell,
            id,
            None,
        )?;
        Ok(Delta::Group(vec![
            placed,
            Delta::Object {
                id,
                inverse: ObjectInverse::Created,
            },
        ]))
    }
}

/// `text.setText` — replace the object's characters with one run.
struct SetText;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetTextPayload {
    /// Target text object.
    id: doc::ObjectId,
    /// The new characters.
    text: String,
}

impl TypedCommand for SetText {
    type Payload = SetTextPayload;

    fn name(&self) -> &'static str {
        "text.setText"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "text.setText";
        mutate_text(document, NAME, payload.id, |object| {
            object.runs = if payload.text.is_empty() {
                Vec::new()
            } else {
                vec![TextRun {
                    text: payload.text.clone(),
                }]
            };
        })
    }
}

/// `text.setAttrs` — write any subset of the text style attributes.
struct SetAttrs;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetAttrsPayload {
    /// Target text object.
    id: doc::ObjectId,
    /// `PTSf` — point size.
    size: Option<f64>,
    /// `BOLb`.
    bold: Option<bool>,
    /// `ITLb`.
    italic: Option<bool>,
    /// `UNDb`.
    underline: Option<bool>,
    /// `HSCf` — percent, 100 = identity.
    horizontal_scale: Option<f64>,
    /// `RKNf` — per-mille of em.
    kerning: Option<f64>,
    /// `BLSf` — pixels, positive raises.
    baseline_shift: Option<f64>,
    /// `LEDf` magnitude.
    leading: Option<f64>,
    /// `LDMi` — 0 em, 1 pixels.
    leading_unit: Option<i64>,
    /// `JSTi` — 0 left, 1 centre, 2 right, 3 justified (lays out left).
    justification: Option<i64>,
    /// `FCLi` fill color, packed ABGR like the path `PATv` paints.
    fill_raw: Option<i64>,
    /// `TAAi` — anti-alias preset index (round-trips; layout ignores it
    /// until raster-time AA selection lands).
    anti_alias: Option<i64>,
    /// `ATEb` — auto-enlarge the frame to fit the text.
    auto_enlarge: Option<bool>,
    /// `ATKb` — auto-kern pairs.
    auto_kern: Option<bool>,
}

impl TypedCommand for SetAttrs {
    type Payload = SetAttrsPayload;

    fn name(&self) -> &'static str {
        "text.setAttrs"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "text.setAttrs";
        let positive = |value: f64, field: &str| -> Result<(), EngineError> {
            if value.is_finite() && value > 0.0 {
                Ok(())
            } else {
                Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("{field} must be finite and positive"),
                })
            }
        };
        let finite = |value: f64, field: &str| -> Result<(), EngineError> {
            if value.is_finite() {
                Ok(())
            } else {
                Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("{field} must be finite"),
                })
            }
        };
        if let Some(size) = payload.size {
            positive(size, "size")?;
        }
        if let Some(scale) = payload.horizontal_scale {
            positive(scale, "horizontal_scale")?;
        }
        if let Some(leading) = payload.leading {
            positive(leading, "leading")?;
        }
        if let Some(kerning) = payload.kerning {
            finite(kerning, "kerning")?;
        }
        if let Some(shift) = payload.baseline_shift {
            finite(shift, "baseline_shift")?;
        }
        match payload.leading_unit {
            Some(0) | Some(1) | None => {}
            other => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("leadingUnit {other:?} outside 0..1"),
                });
            }
        }
        if let Some(justification) = payload.justification {
            if !(0..=3).contains(&justification) {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("justification {justification} outside 0..3"),
                });
            }
        }
        if payload.size.is_none()
            && payload.bold.is_none()
            && payload.italic.is_none()
            && payload.underline.is_none()
            && payload.horizontal_scale.is_none()
            && payload.kerning.is_none()
            && payload.baseline_shift.is_none()
            && payload.leading.is_none()
            && payload.leading_unit.is_none()
            && payload.justification.is_none()
            && payload.fill_raw.is_none()
            && payload.anti_alias.is_none()
            && payload.auto_enlarge.is_none()
            && payload.auto_kern.is_none()
        {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "payload carries no attribute".to_owned(),
            });
        }
        mutate_text(document, NAME, payload.id, |object| {
            let style = style_of(object);
            if let Some(size) = payload.size {
                style.size = Some(size);
            }
            if let Some(bold) = payload.bold {
                style.bold = Some(bold);
            }
            if let Some(italic) = payload.italic {
                style.italic = Some(italic);
            }
            if let Some(underline) = payload.underline {
                style.underline = Some(underline);
            }
            if let Some(scale) = payload.horizontal_scale {
                style.horizontal_scale = Some(scale);
            }
            if let Some(kerning) = payload.kerning {
                style.kerning = Some(kerning);
            }
            if let Some(shift) = payload.baseline_shift {
                style.baseline_shift = Some(shift);
            }
            if let Some(leading) = payload.leading {
                style.leading = Some(leading);
            }
            if let Some(unit) = payload.leading_unit {
                style.leading_unit = Some(unit);
            }
            if let Some(justification) = payload.justification {
                style.justification = Some(justification);
            }
            if let Some(fill) = payload.fill_raw {
                style.fill_raw = Some(fill);
            }
            if let Some(alias) = payload.anti_alias {
                object.anti_alias = Some(alias);
            }
            if let Some(enlarge) = payload.auto_enlarge {
                object.auto_enlarge = Some(enlarge);
            }
            if let Some(kern) = payload.auto_kern {
                object.auto_kern = Some(kern);
            }
        })
    }
}

/// `text.setFont` — set the family name (`FONs`). The name round-trips
/// verbatim; unknown families render in the bundled sans.
struct SetFont;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetFontPayload {
    /// Target text object.
    id: doc::ObjectId,
    /// The family name.
    font: String,
}

impl TypedCommand for SetFont {
    type Payload = SetFontPayload;

    fn name(&self) -> &'static str {
        "text.setFont"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "text.setFont";
        mutate_text(document, NAME, payload.id, |object| {
            style_of(object).font = Some(payload.font.clone());
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::registry::CommandRegistry;
    use doc::{Cell, Contour, Layer, LayerList, Page, PageInfo, PathObject, PathPoint};
    use serde_json::json;

    /// A minimal live path object for "not a text object" refusals.
    fn path_fixture() -> ObjectData {
        ObjectData::Path(PathObject {
            contours: vec![Contour {
                closed: true,
                point_count: 2,
                points: vec![
                    PathPoint {
                        anchor: geom::Point::new(0.0, 0.0),
                        primary: None,
                        secondary: None,
                        is_curve: false,
                    },
                    PathPoint {
                        anchor: geom::Point::new(8.0, 8.0),
                        primary: None,
                        secondary: None,
                        is_curve: false,
                    },
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
            effects: Vec::new(),
        })
    }

    /// One page, one layer, one visible cell, empty document registry.
    fn document() -> Document {
        let mut document = Document::default();
        let layer = Layer {
            name: "Layer 0".to_owned(),
            display: true,
            cells: vec![Cell {
                locked: false,
                visible: true,
                elements: Vec::new(),
            }],
            ..Layer::default()
        };
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: 1,
                frame_count_duplicate: 1,
                request_id: 0,
                canvas: Default::default(),
                onion_before: 0,
                onion_after: 0,
            },
            name: Some("Page 0".to_owned()),
            is_master: false,
            is_linked: false,
            page_id: None,
            last_export_dir: None,
            last_export_file: None,
            layers: LayerList {
                layers: vec![layer],
            },
            grid: None,
            guides: Default::default(),
        });
        document
    }

    fn registry() -> CommandRegistry {
        // The full built-in catalog: the display-quartet test exercises
        // `object.*` commands against text objects too.
        let mut registry = CommandRegistry::new();
        crate::catalog::install(&mut registry).unwrap();
        registry
    }

    fn create(document: &mut Document, text: &str) -> (Delta, doc::ObjectId) {
        let payload = json!({
            "page": 0, "layer": 0, "cell": 0,
            "x": 10.0, "y": 20.0, "width": 200.0, "height": 40.0,
            "text": text,
            "name": "caption",
        });
        let delta = registry()
            .execute(document, "text.createText", payload)
            .unwrap();
        let id = document.pages[0].layers.layers[0].cells[0].elements[0];
        (delta, id)
    }

    fn text_object(document: &Document, id: doc::ObjectId) -> &TextObject {
        match document.registry.get(id).unwrap() {
            ObjectData::Text(text) => text,
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn create_text_inserts_and_captures() {
        let mut document = document();
        let (delta, id) = create(&mut document, "FireCraft");
        let object = text_object(&document, id);
        assert_eq!(object.runs.len(), 1);
        assert_eq!(object.runs[0].text, "FireCraft");
        assert_eq!(object.frame, Rect::new(10.0, 20.0, 210.0, 60.0));
        assert_eq!(object.name.as_deref(), Some("caption"));
        assert_eq!(object.opacity, 1000);
        assert!(object.visible);
        assert!(!object.locked);
        // Inverse removes the object and its cell entry.
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.registry.get(id).is_none());
        assert!(
            document.pages[0].layers.layers[0].cells[0]
                .elements
                .is_empty()
        );
    }

    #[test]
    fn create_text_refuses_non_finite_frames() {
        let mut document = document();
        // JSON cannot carry NaN, so a null slot fails payload decoding;
        // the typed surface (apply) hits the finiteness validation.
        for field in ["x", "y", "width", "height"] {
            let mut payload = json!({
                "page": 0, "layer": 0, "cell": 0,
                "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0,
                "text": "hi",
                "name": null,
            });
            payload[field] = serde_json::Value::Null;
            let error = registry()
                .execute(&mut document, "text.createText", payload)
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    EngineError::BadPayload { .. } | EngineError::InvalidState { .. }
                ),
                "{field}"
            );
        }
        let typed = CreateTextPayload {
            page: 0,
            layer: 0,
            cell: 0,
            x: f64::NAN,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            text: "hi".to_owned(),
            name: None,
        };
        let error = CreateText.apply(&mut document, &typed).unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        // Nothing was inserted by any refused call.
        assert!(
            document.pages[0].layers.layers[0].cells[0]
                .elements
                .is_empty()
        );
    }

    #[test]
    fn set_text_replaces_runs_and_inverts() {
        let mut document = document();
        let (_, id) = create(&mut document, "before");
        let delta = registry()
            .execute(
                &mut document,
                "text.setText",
                json!({ "id": id, "text": "after" }),
            )
            .unwrap();
        assert_eq!(text_object(&document, id).runs[0].text, "after");
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(text_object(&document, id).runs[0].text, "before");
        // Empty text clears the runs entirely.
        let delta = registry()
            .execute(
                &mut document,
                "text.setText",
                json!({ "id": id, "text": "" }),
            )
            .unwrap();
        assert!(text_object(&document, id).runs.is_empty());
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(text_object(&document, id).runs.len(), 1);
    }

    #[test]
    fn set_attrs_writes_the_style_slot_and_inverts() {
        let mut document = document();
        let (_, id) = create(&mut document, "styled");
        let delta = registry()
            .execute(
                &mut document,
                "text.setAttrs",
                json!({
                    "id": id,
                    "size": 24.0,
                    "bold": true,
                    "justification": 2,
                    "fill_raw": -16777216,
                    "anti_alias": 2,
                    "auto_enlarge": true,
                }),
            )
            .unwrap();
        let object = text_object(&document, id);
        let style = object.style.as_ref().unwrap();
        assert_eq!(style.size, Some(24.0));
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.justification, Some(2));
        assert_eq!(style.fill_raw, Some(-16777216));
        assert_eq!(object.anti_alias, Some(2));
        assert_eq!(object.auto_enlarge, Some(true));
        delta.apply_inverse(&mut document).unwrap();
        // The whole object reverts, style slot included.
        let object = text_object(&document, id);
        assert!(object.style.is_none());
        assert_eq!(object.anti_alias, None);
    }

    #[test]
    fn set_attrs_validates_ranges() {
        let mut document = document();
        let (_, id) = create(&mut document, "checked");
        let mut refused = |payload: serde_json::Value, label: &str| {
            let error = registry()
                .execute(&mut document, "text.setAttrs", payload)
                .unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }), "{label}");
        };
        refused(json!({ "id": id, "size": 0.0 }), "size zero");
        refused(json!({ "id": id, "size": -4.0 }), "size negative");
        refused(json!({ "id": id, "size": f64::NAN }), "size NaN");
        refused(json!({ "id": id, "leading": 0.0 }), "leading zero");
        refused(
            json!({ "id": id, "horizontal_scale": -1.0 }),
            "scale negative",
        );
        refused(json!({ "id": id, "kerning": f64::INFINITY }), "kerning inf");
        refused(json!({ "id": id, "baseline_shift": f64::NAN }), "shift NaN");
        refused(json!({ "id": id, "leading_unit": 2 }), "unit 2");
        refused(json!({ "id": id, "justification": 4 }), "justification 4");
        refused(json!({ "id": id }), "no attribute");
        // And nothing mutated through any refusal.
        assert!(text_object(&document, id).style.is_none());
    }

    #[test]
    fn set_font_creates_the_style_slot() {
        let mut document = document();
        let (_, id) = create(&mut document, "fonts");
        let delta = registry()
            .execute(
                &mut document,
                "text.setFont",
                json!({ "id": id, "font": "Liberation Sans" }),
            )
            .unwrap();
        assert_eq!(
            text_object(&document, id).style.as_ref().unwrap().font,
            Some("Liberation Sans".to_owned())
        );
        delta.apply_inverse(&mut document).unwrap();
        assert!(text_object(&document, id).style.is_none());
    }

    #[test]
    fn text_mutations_refuse_non_text_objects() {
        let mut document = document();
        let path_id = document.registry.insert(path_fixture());
        for (name, payload) in [
            ("text.setText", json!({ "id": path_id, "text": "x" })),
            ("text.setAttrs", json!({ "id": path_id, "size": 12.0 })),
            ("text.setFont", json!({ "id": path_id, "font": "Arial" })),
        ] {
            let error = registry()
                .execute(&mut document, name, payload)
                .unwrap_err();
            assert!(
                matches!(error, EngineError::InvalidState { .. }),
                "{name} on a path"
            );
        }
    }

    #[test]
    fn display_commands_cover_text() {
        let mut document = document();
        let (_, id) = create(&mut document, "visible");
        let delta = registry()
            .execute(
                &mut document,
                "object.setOpacity",
                json!({ "id": id, "opacity": 250 }),
            )
            .unwrap();
        assert_eq!(text_object(&document, id).opacity, 250);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(text_object(&document, id).opacity, 1000);
    }
}
