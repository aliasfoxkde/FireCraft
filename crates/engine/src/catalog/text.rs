//! Text commands (`text.*`): insert a text object into a cell and edit
//! its characters and attributes.
//!
//! Like `autoshape.*`, insertion validates before the first mutation.
//! Attribute edits capture the whole object before mutating, so every
//! payload (accepted or refused) leaves the journal invertible. The
//! layout itself lives in the `text` crate — these commands only move
//! the model; nothing here shapes glyphs.

use doc::{Contour, Document, ObjectData, PathPoint, TextObject, TextOnPath, TextRun, TextStyle};
use geom::{Point, Rect};

use crate::access;
use crate::catalog::object::{attach_cell, detach, ensure_detachable};
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the text commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(CreateText)?;
    registry.register(SetText)?;
    registry.register(SetAttrs)?;
    registry.register(SetFont)?;
    registry.register(AttachPath)?;
    registry.register(DetachPath)?;
    registry.register(ConvertToPaths)
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

/// One anchor control (`XPCf`/`YPCf`, `XSCf`/`YSCf`) over the wire,
/// in absolute document coordinates — the model stores handles
/// absolutely, and the attach payload mirrors that exactly.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct WireHandle {
    x: f64,
    y: f64,
}

/// One path point of the attach payload: anchor plus optional handles.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct WirePoint {
    x: f64,
    y: f64,
    /// Arrival control (`XPCf`/`YPCf`), absolute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    primary: Option<WireHandle>,
    /// Departure control (`XSCf`/`YSCf`), absolute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    secondary: Option<WireHandle>,
    /// `CRVb` — the point is a curve point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    curve: Option<bool>,
}

/// One contour of the attach payload (`ISCb` + `PBTv`*).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct WireContour {
    /// `ISCb` — default false, matching the reader's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    closed: Option<bool>,
    points: Vec<WirePoint>,
}

/// `text.attachPath` — lay the object's text along guide contours
/// (`TAPv`): sets `TFTi` to 2 (follow path) and replaces any previous
/// guide. The guide renders as nothing itself; styling children it
/// would carry in a real file stay a writer concern, not a command one.
struct AttachPath;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AttachPathPayload {
    /// Target text object.
    id: doc::ObjectId,
    /// Guide contours (`PTHv`/`PBLv`).
    path: Vec<WireContour>,
    /// `TAMi` — 0 rotate with the path, 1 upright, 2 skew vertical,
    /// 3 skew horizontal. Absent leaves the model's default (rotate).
    alignment: Option<i64>,
    /// `CHOf` — distance along the path where the text starts. May be
    /// negative (leading glyphs fall off the rail and drop).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    offset: Option<f64>,
}

impl AttachPathPayload {
    fn validate(&self, name: &'static str) -> Result<Vec<Contour>, EngineError> {
        if self.path.is_empty() {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: "path must carry at least one contour".to_owned(),
            });
        }
        if let Some(alignment) = self.alignment {
            if !(0..=3).contains(&alignment) {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: format!("alignment {alignment} outside 0..3"),
                });
            }
        }
        if let Some(offset) = self.offset {
            if !offset.is_finite() {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: "offset must be finite".to_owned(),
                });
            }
        }
        self.path
            .iter()
            .enumerate()
            .try_for_each(|(index, contour)| contour.validate(name, index))?;
        Ok(self
            .path
            .iter()
            .map(|contour| contour.to_contour())
            .collect())
    }
}

impl WireContour {
    /// A rail contour needs two anchors or more; fewer flatten to a
    /// zero-length rail and would silently drop every glyph.
    fn validate(&self, name: &'static str, index: usize) -> Result<(), EngineError> {
        if self.points.len() < 2 {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!(
                    "contour {index} carries {} points, needs 2 or more",
                    self.points.len()
                ),
            });
        }
        for point in &self.points {
            let mut finite = [point.x, point.y].iter().all(|value| value.is_finite());
            if let Some(handle) = &point.primary {
                finite &= [handle.x, handle.y].iter().all(|value| value.is_finite());
            }
            if let Some(handle) = &point.secondary {
                finite &= [handle.x, handle.y].iter().all(|value| value.is_finite());
            }
            if !finite {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: "point coordinates must be finite".to_owned(),
                });
            }
        }
        Ok(())
    }

    fn to_contour(&self) -> Contour {
        Contour {
            closed: self.closed.unwrap_or(false),
            point_count: self.points.len() as i64,
            points: self
                .points
                .iter()
                .map(|point| PathPoint {
                    anchor: Point::new(point.x, point.y),
                    primary: point
                        .primary
                        .as_ref()
                        .map(|handle| Point::new(handle.x, handle.y)),
                    secondary: point
                        .secondary
                        .as_ref()
                        .map(|handle| Point::new(handle.x, handle.y)),
                    is_curve: point.curve.unwrap_or(false),
                })
                .collect(),
        }
    }
}

impl TypedCommand for AttachPath {
    type Payload = AttachPathPayload;

    fn name(&self) -> &'static str {
        "text.attachPath"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "text.attachPath";
        let contours = payload.validate(NAME)?;
        mutate_text(document, NAME, payload.id, |object| {
            object.on_path = Some(TextOnPath {
                alignment: payload.alignment,
                offset: payload.offset.unwrap_or(0.0),
                path: contours,
            });
            object.flow = Some(2);
        })
    }
}

/// `text.detachPath` — return the object to plain flow: clears the
/// guide and `TFTi` (absent means normal). Does not refuse text that
/// was never on a path; the clear is the requested end state.
struct DetachPath;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DetachPathPayload {
    /// Target text object.
    id: doc::ObjectId,
}

impl TypedCommand for DetachPath {
    type Payload = DetachPathPayload;

    fn name(&self) -> &'static str {
        "text.detachPath"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "text.detachPath";
        mutate_text(document, NAME, payload.id, |object| {
            object.on_path = None;
            object.flow = None;
        })
    }
}

/// `text.convertToPaths` — CS6 `dom.convertToPaths()`: "Converts the
/// selected text items into editable paths." Each selected text object
/// becomes a path in place (same registry id, same z-position): the
/// glyph outlines the renderer would draw, frozen as true contours,
/// with the text fill riding the path paint (the renderer's synthetic
/// path is the contract) and the display fields kept. A text item with
/// no ink (empty runs) leaves nothing behind and is removed, the way
/// an empty boolean result is.
struct ConvertToPaths;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ConvertToPathsPayload {}

impl TypedCommand for ConvertToPaths {
    type Payload = ConvertToPathsPayload;

    fn name(&self) -> &'static str {
        "text.convertToPaths"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "text.convertToPaths";
        if document.object_selection.is_empty() {
            return Ok(Delta::Noop);
        }
        // Compute every conversion before any mutation: one non-text
        // member refuses the whole command.
        let mut engine = text::TextEngine::new();
        let mut converted = Vec::new();
        for id in &document.object_selection {
            let Some(ObjectData::Text(object)) = document.registry.get(*id) else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "selected object {} is not a text object; only text converts",
                        id.index()
                    ),
                });
            };
            let geometry = engine.glyph_contours(object);
            // Fireworks' default text fill is opaque black; `FCLi`
            // overrides (the renderer's rule, mirrored here so the
            // converted path paints identically).
            const DEFAULT_FILL: i64 = 0xFF00_0000; // ABGR: alpha 255, black
            let fill = object
                .style
                .as_ref()
                .and_then(|style| style.fill_raw)
                .unwrap_or(DEFAULT_FILL);
            let path = doc::PathObject {
                contours: geometry.contours,
                even_odd: false,
                primitive: None,
                paint: Some(doc::PaintStyle {
                    stroke_raw: 0, // ABGR alpha 0 — text never strokes
                    fill_raw: fill,
                    fill: None,
                }),
                fill_start: None,
                fill_end: None,
                fill_third: None,
                brush_placement: None,
                fill_on_top: None,
                locked: object.locked,
                visible: object.visible,
                opacity: object.opacity,
                blend: object.blend,
                name: object.name.clone(),
                effects: object.effects.clone(),
            };
            converted.push((*id, path));
        }
        // Pre-flight the removals: an ink-less member must refuse
        // before the first conversion lands.
        for (id, path) in &converted {
            if path.contours.is_empty() {
                ensure_detachable(document, NAME, *id)?;
            }
        }
        let mut deltas = Vec::new();
        for (id, path) in converted {
            let before = access::object(document, NAME, id)?.clone();
            if path.contours.is_empty() {
                deltas.push(detach(document, NAME, id)?);
                if document.registry.remove(id).is_none() {
                    return Err(EngineError::MissingTarget {
                        name: NAME.to_owned(),
                        detail: format!("object {} vanished mid-conversion", id.index()),
                    });
                }
                deltas.push(Delta::Object {
                    id,
                    inverse: ObjectInverse::Removed {
                        object: Box::new(before),
                    },
                });
            } else {
                if let Some(slot) = document.registry.get_mut(id) {
                    *slot = ObjectData::Path(path);
                }
                deltas.push(Delta::Object {
                    id,
                    inverse: ObjectInverse::Mutated {
                        object: Box::new(before),
                    },
                });
            }
        }
        Ok(Delta::Group(deltas))
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
                frame_durations: Vec::new(),
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
            export: None,
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

    /// A three-point rail: one curved segment then a straight one.
    fn guide_json() -> serde_json::Value {
        json!([{
            "closed": false,
            "points": [
                {"x": 10.0, "y": 40.0, "secondary": {"x": 18.0, "y": 40.0}, "curve": true},
                {"x": 30.0, "y": 40.0, "primary": {"x": 22.0, "y": 40.0}, "curve": true},
                {"x": 30.0, "y": 60.0},
            ],
        }])
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
    fn attach_path_sets_the_guide_and_inverts() {
        let mut document = document();
        let (_, id) = create(&mut document, "curved");
        let delta = registry()
            .execute(
                &mut document,
                "text.attachPath",
                json!({
                    "id": id,
                    "path": guide_json(),
                    "alignment": 2,
                    "offset": 7.5,
                }),
            )
            .unwrap();
        let object = text_object(&document, id);
        let guide = object.on_path.as_ref().unwrap();
        assert_eq!(guide.alignment, Some(2));
        assert_eq!(guide.offset, 7.5);
        assert_eq!(guide.path.len(), 1);
        let contour = &guide.path[0];
        assert_eq!(contour.point_count, 3);
        assert!(!contour.closed);
        assert_eq!(contour.points[0].anchor, Point::new(10.0, 40.0));
        assert_eq!(contour.points[0].secondary, Some(Point::new(18.0, 40.0)));
        assert!(contour.points[0].is_curve);
        assert_eq!(contour.points[1].primary, Some(Point::new(22.0, 40.0)));
        assert!(contour.points[1].is_curve);
        assert_eq!(contour.points[2].primary, None);
        assert_eq!(contour.points[2].secondary, None);
        assert!(!contour.points[2].is_curve);
        assert_eq!(object.flow, Some(2));
        delta.apply_inverse(&mut document).unwrap();
        let object = text_object(&document, id);
        assert!(object.on_path.is_none());
        assert_eq!(object.flow, None);
    }

    #[test]
    fn attach_path_defaults_then_replaces_the_guide() {
        let mut document = document();
        let (_, id) = create(&mut document, "rail");
        let registry = registry();
        registry
            .execute(
                &mut document,
                "text.attachPath",
                json!({ "id": id, "path": guide_json() }),
            )
            .unwrap();
        let first = text_object(&document, id).on_path.clone().unwrap();
        assert_eq!(first.alignment, None);
        assert_eq!(first.offset, 0.0);
        // A second attach replaces the guide wholesale.
        registry
            .execute(
                &mut document,
                "text.attachPath",
                json!({
                    "id": id,
                    "path": [
                        {"closed": true, "points": [
                            {"x": 0.0, "y": 0.0},
                            {"x": 10.0, "y": 0.0},
                            {"x": 10.0, "y": 10.0},
                            {"x": 0.0, "y": 10.0},
                        ]},
                        {"points": [{"x": 50.0, "y": 50.0}, {"x": 90.0, "y": 50.0}]},
                    ],
                    "offset": -6.0,
                }),
            )
            .unwrap();
        let guide = text_object(&document, id).on_path.as_ref().unwrap();
        assert_eq!(guide.path.len(), 2);
        assert!(guide.path[0].closed);
        assert_eq!(guide.path[1].point_count, 2);
        assert_eq!(guide.offset, -6.0);
    }

    #[test]
    fn attach_path_refuses_degenerate_geometry() {
        let mut document = document();
        let (_, id) = create(&mut document, "checked");
        // JSON cannot carry non-finite floats (they encode as null), so
        // the wire-level loop accepts a decode error or the typed
        // refusal; the typed surface reaches the checks directly.
        let mut refused = |payload: serde_json::Value, label: &str| {
            let error = registry()
                .execute(&mut document, "text.attachPath", payload)
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    EngineError::BadPayload { .. } | EngineError::InvalidState { .. }
                ),
                "{label}"
            );
        };
        refused(json!({ "id": id, "path": [] }), "no contours");
        refused(
            json!({ "id": id, "path": [{"points": [{"x": 0.0, "y": 0.0}]}] }),
            "single point contour",
        );
        refused(
            json!({ "id": id, "path": guide_json(), "alignment": 4 }),
            "alignment out of range",
        );
        refused(
            json!({ "id": id, "path": [{"points": [
                {"x": 0.0, "y": 0.0, "secondary": {"x": 4.0}},
                {"x": 8.0, "y": 0.0},
            ]}]}),
            "handle missing a coordinate",
        );
        let typed = AttachPathPayload {
            id,
            path: vec![WireContour {
                closed: Some(false),
                points: vec![
                    WirePoint {
                        x: f64::NAN,
                        y: 0.0,
                        primary: None,
                        secondary: None,
                        curve: None,
                    },
                    WirePoint {
                        x: 8.0,
                        y: 0.0,
                        primary: None,
                        secondary: None,
                        curve: None,
                    },
                ],
            }],
            alignment: None,
            offset: Some(f64::INFINITY),
        };
        let error = AttachPath.apply(&mut document, &typed).unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        // Nothing mutated through any refusal.
        assert!(text_object(&document, id).on_path.is_none());
    }

    #[test]
    fn detach_path_clears_the_guide_and_inverts() {
        let mut document = document();
        let (_, id) = create(&mut document, "unravel");
        let registry = registry();
        registry
            .execute(
                &mut document,
                "text.attachPath",
                json!({ "id": id, "path": guide_json() }),
            )
            .unwrap();
        let delta = registry
            .execute(&mut document, "text.detachPath", json!({ "id": id }))
            .unwrap();
        let object = text_object(&document, id);
        assert!(object.on_path.is_none());
        assert_eq!(object.flow, None);
        // Inverse restores the attached guide.
        delta.apply_inverse(&mut document).unwrap();
        let object = text_object(&document, id);
        assert!(object.on_path.is_some());
        assert_eq!(object.flow, Some(2));
        // A fresh object detaches as a no-op end state.
        let (_, plain) = create(&mut document, "plain");
        registry
            .execute(&mut document, "text.detachPath", json!({ "id": plain }))
            .unwrap();
        assert!(text_object(&document, plain).on_path.is_none());
    }

    #[test]
    fn text_mutations_refuse_non_text_objects() {
        let mut document = document();
        let path_id = document.registry.insert(path_fixture());
        for (name, payload) in [
            ("text.setText", json!({ "id": path_id, "text": "x" })),
            ("text.setAttrs", json!({ "id": path_id, "size": 12.0 })),
            ("text.setFont", json!({ "id": path_id, "font": "Arial" })),
            (
                "text.attachPath",
                json!({ "id": path_id, "path": guide_json() }),
            ),
            ("text.detachPath", json!({ "id": path_id })),
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

    fn document_bytes(document: &Document) -> Vec<u8> {
        serde_json::to_vec(document).unwrap()
    }

    #[test]
    fn convert_to_paths_freezes_glyph_outlines() {
        let mut document = document();
        let (_, id) = create(&mut document, "FireCraft");
        document.object_selection = vec![id];
        let before = document_bytes(&document);
        let delta = registry()
            .execute(&mut document, "text.convertToPaths", json!({}))
            .unwrap();

        // Same id, same z-slot: the text became a path in place.
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![id]
        );
        let path = match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => path,
            other => panic!("wrong object kind: {other:?}"),
        };
        assert!(path.primitive.is_none(), "outlines are derived geometry");
        assert!(!path.even_odd, "font outlines fill nonzero");
        assert!(path.contours.len() > 1, "glyphs plus counters");
        assert!(path.contours.iter().all(|contour| contour.closed));
        assert!(
            path.contours
                .iter()
                .any(|contour| { contour.points.iter().any(|point| point.is_curve) }),
            "glyph outlines carry bezier handles"
        );
        // The text fill rides the path paint; text never strokes.
        let paint = path.paint.as_ref().unwrap();
        assert_eq!(paint.fill_raw, 0xFF00_0000, "default text fill");
        assert_eq!(paint.stroke_raw, 0);
        // Display fields survive the conversion.
        assert_eq!(path.name.as_deref(), Some("caption"));
        assert_eq!(path.opacity, 1000);
        assert!(path.visible && !path.locked);
        assert_eq!(document.object_selection, vec![id]);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before, "undo restores the text");
    }

    #[test]
    fn convert_to_paths_paints_like_the_text_did() {
        // The renderer's synthetic path is the conversion contract:
        // before and after must rasterize identically.
        let mut document = document();
        let (_, id) = create(&mut document, "Ag");
        document.pages[0].info.canvas = doc::Canvas {
            width: 64.0,
            height: 48.0,
            ..doc::Canvas::default()
        };
        document.object_selection = vec![id];
        let text_pixels = compose::render_subset(&document, 0, 0, &[id]).unwrap();
        registry()
            .execute(&mut document, "text.convertToPaths", json!({}))
            .unwrap();
        let path_pixels = compose::render_subset(&document, 0, 0, &[id]).unwrap();
        assert_eq!(text_pixels.pixels, path_pixels.pixels);
    }

    #[test]
    fn convert_to_paths_empty_selection_is_a_noop() {
        let mut document = document();
        assert!(matches!(
            registry()
                .execute(&mut document, "text.convertToPaths", json!({}))
                .unwrap(),
            Delta::Noop
        ));
    }

    #[test]
    fn convert_to_paths_refuses_non_text_untouched() {
        let mut document = document();
        let path_id = document.registry.insert(path_fixture());
        document.pages[0].layers.layers[0].cells[0].elements = vec![path_id];
        document.object_selection = vec![path_id];
        let before = document_bytes(&document);
        let error = registry()
            .execute(&mut document, "text.convertToPaths", json!({}))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn convert_to_paths_removes_inkless_text_and_undo_revives() {
        let mut document = document();
        let (_, id) = create(&mut document, "");
        document.object_selection = vec![id];
        let before = document_bytes(&document);
        let delta = registry()
            .execute(&mut document, "text.convertToPaths", json!({}))
            .unwrap();
        assert!(document.registry.get(id).is_none());
        assert!(
            document.pages[0].layers.layers[0].cells[0]
                .elements
                .is_empty()
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }
}
