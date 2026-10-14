//! Web-layer commands (`slice.*` / `hotspot.*`): the export-facing
//! surface CS6 drives from its Web Layer.
//!
//! A slice or hotspot is an ordinary [`ObjectData::Slice`] living on
//! the page's Web Layer — a layer flagged `URLb` + `MSLb` that compose
//! never renders (slices are export-time markers). The layer is
//! provisioned lazily on first creation, top of the stack, with one
//! visible cell per state; documents that never slice stay
//! byte-identical to pre-M11 output.

use crate::access;
use crate::catalog::object::attach_cell;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use doc::{
    Contour, Document, ExportOptions, ObjectData, PathPoint, SliceBackground, SliceBehavior,
    SliceObject,
};

/// The Web Layer's fixed name (CS6's own special-layer label).
const WEB_LAYER: &str = "Web Layer";

/// The page's Web Layer index, provisioning one when absent: top of
/// the stack, `URLb` + `MSLb`, one visible cell per state. Returns
/// the index and — when a layer was minted — the whole layer list as
/// it was before, so the caller's delta can undo the provisioning.
fn ensure_web_layer(
    document: &mut Document,
    name: &'static str,
    page: usize,
) -> Result<(usize, Option<doc::LayerList>), EngineError> {
    let target = access::page_mut(document, name, page)?;
    if let Some(index) = target
        .layers
        .layers
        .iter()
        .rposition(|layer| layer.has_url && layer.is_special)
    {
        return Ok((index, None));
    }
    let before = target.layers.clone();
    let states = target.info.frame_count.max(1);
    let layer = doc::Layer {
        name: WEB_LAYER.to_owned(),
        has_url: true,
        is_special: true,
        display: true,
        cells: (0..states)
            .map(|_| doc::Cell {
                locked: false,
                visible: true,
                elements: Vec::new(),
            })
            .collect(),
        ..doc::Layer::default()
    };
    target.layers.layers.push(layer);
    Ok((target.layers.layers.len() - 1, Some(before)))
}

/// The polygon contour from payload points.
fn polygon_contour(points: &[PathPoint]) -> Vec<Contour> {
    vec![Contour {
        closed: true,
        point_count: points.len() as i64,
        points: points.to_vec(),
    }]
}

/// Shared creation path for `slice.create` / `hotspot.create`.
struct CreateSpec {
    shape: i64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    points: Vec<PathPoint>,
    page: usize,
    name: Option<String>,
    link: Option<String>,
    alt: Option<String>,
}

fn create_slice(
    document: &mut Document,
    name: &'static str,
    spec: &CreateSpec,
    slice_type: Option<i64>,
) -> Result<Delta, EngineError> {
    if !spec.width.is_finite()
        || !spec.height.is_finite()
        || !spec.x.is_finite()
        || !spec.y.is_finite()
        || spec
            .points
            .iter()
            .any(|p| !p.anchor.x.is_finite() || !p.anchor.y.is_finite())
    {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "geometry must be finite".to_owned(),
        });
    }
    // Validate the destination (and provision the Web Layer) before
    // anything mutates.
    let (layer, layers_before) = ensure_web_layer(document, name, spec.page)?;
    access::cell_mut(document, name, spec.page, layer, 0)?;

    let object = ObjectData::Slice(SliceObject {
        internal_name: None,
        frame: geom::Rect::new(spec.x, spec.y, spec.x + spec.width, spec.y + spec.height),
        shape: spec.shape,
        fill_raw: 0,
        name: spec.name.clone(),
        slice_type,
        link: spec.link.clone(),
        alt: spec.alt.clone(),
        label: None,
        td_text: None,
        symbol_number: None,
        contours: polygon_contour(&spec.points),
        background: None,
        behaviors: Vec::new(),
        custom_data: std::collections::BTreeMap::new(),
        export: None,
    });

    let id = document.registry.insert(object);
    // Append to state 0 of the Web Layer — the cell lookup passed
    // above, so this cannot orphan the registration.
    let placed = attach_cell(document, name, spec.page, layer, 0, id, None)?;
    // A freshly minted Web Layer rides the delta first (undo pops the
    // group LIFO: object, cell, then layer list) so undo strips the
    // provisioning along with the slice.
    let mut deltas = Vec::new();
    if let Some(layers) = layers_before {
        deltas.push(Delta::Layers {
            page: spec.page,
            layers,
        });
    }
    deltas.push(placed);
    deltas.push(Delta::Object {
        id,
        inverse: ObjectInverse::Created,
    });
    Ok(Delta::Group(deltas))
}

/// One world-space anchor of a polygon slice.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WebPoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

/// What outline the created web object holds.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum WebShape {
    /// Bounding-box rectangle.
    Rect,
    /// Bounding-box ellipse.
    Ellipse,
    /// Point list polygon (the frame bounds still carry the bbox).
    Polygon,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct CreateWebPayload {
    shape: WebShape,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    /// Polygon anchors (required for `polygon`, rejected otherwise).
    points: Option<Vec<WebPoint>>,
    page: usize,
    name: Option<String>,
    link: Option<String>,
    alt: Option<String>,
}

impl CreateWebPayload {
    fn spec(&self) -> Result<CreateSpec, EngineError> {
        let shape = match self.shape {
            WebShape::Rect => 0,
            WebShape::Ellipse => 1,
            WebShape::Polygon => 2,
        };
        let points = match (&self.shape, &self.points) {
            (WebShape::Polygon, Some(points)) if points.len() >= 3 => points
                .iter()
                .map(|p| PathPoint {
                    anchor: geom::Point::new(p.x, p.y),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                })
                .collect::<Vec<_>>(),
            // Non-polygon shapes carry no contour; the bounds ride the
            // frame.
            (WebShape::Polygon, _) => {
                return Err(EngineError::InvalidState {
                    name: "payload".to_owned(),
                    detail: "a polygon needs at least 3 points".to_owned(),
                });
            }
            _ => Vec::new(),
        };
        Ok(CreateSpec {
            shape,
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            points,
            page: self.page,
            name: self.name.clone(),
            link: self.link.clone(),
            alt: self.alt.clone(),
        })
    }
}

/// `slice.create` — a foreground slice on the page's Web Layer.
struct CreateSlice;

impl TypedCommand for CreateSlice {
    type Payload = CreateWebPayload;

    fn name(&self) -> &'static str {
        "slice.create"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        create_slice(document, self.name(), &payload.spec()?, Some(0))
    }
}

/// `hotspot.create` — a link-only area on the page's Web Layer. The
/// wire format has no slice-vs-hotspot bit (both are `URLv`); the
/// distinction is `TSLi` presence, and the export workflows treat
/// `TSLi`-less objects as map-only areas (decision 0017).
struct CreateHotspot;

impl TypedCommand for CreateHotspot {
    type Payload = CreateWebPayload;

    fn name(&self) -> &'static str {
        "hotspot.create"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        create_slice(document, self.name(), &payload.spec()?, None)
    }
}

/// The target must be a slice; anything else is a typed refusal.
fn slice_mut<'a>(
    document: &'a mut Document,
    name: &'static str,
    id: doc::ObjectId,
) -> Result<&'a mut SliceObject, EngineError> {
    match document.registry.get_mut(id) {
        Some(ObjectData::Slice(slice)) => Ok(slice),
        _ => Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("object {} is not a slice or hotspot", id.index()),
        }),
    }
}

/// `slice.setLink` — anchor URL and alt text.
struct SetLink;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetLinkPayload {
    id: doc::ObjectId,
    /// The anchor URL; `null` clears it.
    link: Option<String>,
    /// Alt text; `null` clears it.
    alt: Option<String>,
}

impl TypedCommand for SetLink {
    type Payload = SetLinkPayload;

    fn name(&self) -> &'static str {
        "slice.setLink"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        let before = access::object(document, self.name(), payload.id)?.clone();
        let slice = slice_mut(document, self.name(), payload.id)?;
        slice.link = payload.link.clone();
        slice.alt = payload.alt.clone();
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `slice.setType` — foreground (`0`) / background (`2`) / hotspot
/// (`null`: the `TSLi`-less map-only shape).
struct SetSliceType;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetSliceTypePayload {
    id: doc::ObjectId,
    slice_type: Option<i64>,
}

impl TypedCommand for SetSliceType {
    type Payload = SetSliceTypePayload;

    fn name(&self) -> &'static str {
        "slice.setType"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "slice.setType";
        if payload.slice_type.is_some_and(|t| t != 0 && t != 2) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("unknown slice type {}", payload.slice_type.unwrap_or(-1)),
            });
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let slice = slice_mut(document, NAME, payload.id)?;
        slice.slice_type = payload.slice_type;
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// One optional `slice.setBackground` field with three states: absent
/// leaves the node untouched, `null` clears it, a value sets it. Plain
/// `Option` cannot carry this — absent and `null` both deserialize to
/// the outer `None` — so the tri-state deserializes by hand.
#[derive(Clone, Debug, Default, PartialEq)]
enum Field<T> {
    #[default]
    Leave,
    Clear,
    Set(T),
}

impl<'de, T> serde::Deserialize<'de> for Field<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match Option::<T>::deserialize(deserializer)? {
            None => Field::Clear,
            Some(value) => Field::Set(value),
        })
    }
}

impl<T> serde::Serialize for Field<T>
where
    T: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Input-only tri-state: both quiet states serialize as `null`.
        match self {
            Field::Set(value) => value.serialize(serializer),
            Field::Leave | Field::Clear => serializer.serialize_none(),
        }
    }
}

impl<T> Field<T> {
    /// Writes the field: `Clear` drops the node, `Set` replaces it,
    /// `Leave` does nothing.
    fn assign(self, slot: &mut Option<T>) {
        match self {
            Field::Leave => {}
            Field::Clear => *slot = None,
            Field::Set(value) => *slot = Some(value),
        }
    }
}

/// `slice.setBackground` — the cell-background export family. Setting
/// any field mints the family; an all-default family is dropped so the
/// wire matches a never-touched slice.
struct SetBackground;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetBackgroundPayload {
    id: doc::ObjectId,
    /// `CBTb`.
    #[serde(default)]
    is_background: Field<bool>,
    /// `CBRi` 0..=3.
    #[serde(default)]
    repeat: Field<i64>,
    /// `CBAi` 0..=1.
    #[serde(default)]
    attachment: Field<i64>,
    /// `CBHi` 0..=3.
    #[serde(default)]
    h_position: Field<i64>,
    /// `CHVi`.
    #[serde(default)]
    h_value: Field<i64>,
    /// `CBVi` 0..=3.
    #[serde(default)]
    v_position: Field<i64>,
    /// `CVVi`.
    #[serde(default)]
    v_value: Field<i64>,
}

impl TypedCommand for SetBackground {
    type Payload = SetBackgroundPayload;

    fn name(&self) -> &'static str {
        "slice.setBackground"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "slice.setBackground";
        for (label, field) in [
            ("repeat", &payload.repeat),
            ("attachment", &payload.attachment),
            ("h_position", &payload.h_position),
            ("v_position", &payload.v_position),
        ] {
            if let Field::Set(v) = field {
                if !(0..=3).contains(v) {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: format!("{label} must be 0..=3"),
                    });
                }
            }
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let slice = slice_mut(document, NAME, payload.id)?;
        let background = slice
            .background
            .get_or_insert_with(SliceBackground::default);
        payload
            .is_background
            .clone()
            .assign(&mut background.is_background);
        payload.repeat.clone().assign(&mut background.repeat);
        payload
            .attachment
            .clone()
            .assign(&mut background.attachment);
        payload
            .h_position
            .clone()
            .assign(&mut background.h_position);
        payload.h_value.clone().assign(&mut background.h_value);
        payload
            .v_position
            .clone()
            .assign(&mut background.v_position);
        payload.v_value.clone().assign(&mut background.v_value);
        // An all-`None` family carries nothing on the wire — drop it
        // so the written file matches a never-touched slice.
        if background == &SliceBackground::default() {
            slice.background = None;
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `slice.setExport` — the per-slice `EXPv` override (format, matte,
/// quality, palette). `clear` removes the override entirely.
struct SetSliceExport;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetSliceExportPayload {
    id: doc::ObjectId,
    /// `FMTi` — 0 gif, 1 jpeg, 2 png, 4 animated gif.
    format: Option<i64>,
    /// Matte color, raw ABGR.
    matte_raw: Option<i64>,
    /// JPEG quality 0..=100.
    jpeg_quality: Option<i64>,
    /// Palette dither 0..=100.
    dither: Option<i64>,
    /// Drop the override.
    clear: Option<bool>,
}

impl TypedCommand for SetSliceExport {
    type Payload = SetSliceExportPayload;

    fn name(&self) -> &'static str {
        "slice.setExport"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "slice.setExport";
        if payload
            .jpeg_quality
            .is_some_and(|q| !(0..=100).contains(&q))
        {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "quality must be 0..=100".to_owned(),
            });
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let slice = slice_mut(document, NAME, payload.id)?;
        if payload.clear.unwrap_or(false) {
            slice.export = None;
        } else {
            let export = slice.export.get_or_insert_with(ExportOptions::default);
            if payload.format.is_some() {
                export.format = payload.format;
            }
            if payload.matte_raw.is_some() {
                export.matte_raw = payload.matte_raw;
            }
            if payload.jpeg_quality.is_some() {
                export.jpeg_quality = payload.jpeg_quality;
            }
            if payload.dither.is_some() {
                export.dither = payload.dither;
            }
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `slice.addBehavior` — appends a behavior entry (`BETi` kind). The
/// payload of the behavior stays on the unknown-bag path; kinds the
/// dictionary documents (9 = popup menu) and kinds it does not are
/// both recorded raw.
struct AddBehavior;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddBehaviorPayload {
    id: doc::ObjectId,
    kind: i64,
}

impl TypedCommand for AddBehavior {
    type Payload = AddBehaviorPayload;

    fn name(&self) -> &'static str {
        "slice.addBehavior"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        let before = access::object(document, self.name(), payload.id)?.clone();
        let slice = slice_mut(document, self.name(), payload.id)?;
        slice.behaviors.push(SliceBehavior { kind: payload.kind });
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `slice.removeBehavior` — drops the behavior at `index`.
struct RemoveBehavior;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RemoveBehaviorPayload {
    id: doc::ObjectId,
    index: usize,
}

impl TypedCommand for RemoveBehavior {
    type Payload = RemoveBehaviorPayload;

    fn name(&self) -> &'static str {
        "slice.removeBehavior"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "slice.removeBehavior";
        let before = access::object(document, NAME, payload.id)?.clone();
        let slice = slice_mut(document, NAME, payload.id)?;
        if payload.index >= slice.behaviors.len() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "behavior index {} beyond {} entries",
                    payload.index,
                    slice.behaviors.len()
                ),
            });
        }
        slice.behaviors.remove(payload.index);
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `slice.setData` — one `DTAv` key/value entry; `null` value removes
/// the key. Export-facing behavior parameters (swap target/frame, nav
/// group) ride here — see decision 0017.
struct SetSliceData;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetSliceDataPayload {
    id: doc::ObjectId,
    key: String,
    value: Option<String>,
}

impl TypedCommand for SetSliceData {
    type Payload = SetSliceDataPayload;

    fn name(&self) -> &'static str {
        "slice.setData"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        let before = access::object(document, self.name(), payload.id)?.clone();
        let slice = slice_mut(document, self.name(), payload.id)?;
        match &payload.value {
            Some(value) => {
                slice.custom_data.insert(payload.key.clone(), value.clone());
            }
            None => {
                slice.custom_data.remove(&payload.key);
            }
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `slice.appendPoint` / `hotspot.appendPoint` — CS6
/// `dom.appendPointToSlice(pt, tolerance)` /
/// `dom.appendPointToHotspot(pt, tolerance)` (the slice form is
/// listed under deprecated arguments, but the verb remains): "Appends
/// a point to the selected unclosed polygon [slice | hotspot]. If an
/// unclosed polygon [slice | hotspot] is not selected, a new polygon
/// [slice | hotspot] is created with the single point that passed
/// in." The polyline closes once the new point lands within
/// `tolerance` of its starting point.
///
/// Selection-driven, like the original: the scan walks
/// [`Document::object_selection`] for the topmost polygon of the
/// command's class whose outline is still open. Class is `TSLi`
/// presence (decision 0017): a slice command matches slices only, a
/// hotspot command matches hotspots only, and rect/ellipse shapes
/// never count. The create branch needs a page — the engine has no
/// active-page state, so it rides the payload (`page`, the
/// `frame.add` precedent).
struct AppendPointToSlice;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AppendPointPayload {
    /// The point to append, world coordinates.
    x: f64,
    y: f64,
    /// Closing tolerance (>= 0) between the new point and the
    /// polyline's starting point.
    tolerance: f64,
    /// Page the create branch starts a new object on.
    page: usize,
}

impl TypedCommand for AppendPointToSlice {
    type Payload = AppendPointPayload;

    fn name(&self) -> &'static str {
        "slice.appendPoint"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        append_point(document, self.name(), payload, true)
    }
}

/// `hotspot.appendPoint` — the hotspot twin of `slice.appendPoint`.
struct AppendPointToHotspot;

impl TypedCommand for AppendPointToHotspot {
    type Payload = AppendPointPayload;

    fn name(&self) -> &'static str {
        "hotspot.appendPoint"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        append_point(document, self.name(), payload, false)
    }
}

/// The shared point-append state machine; `as_slice` selects the
/// matched and created class (`TSLi` presence).
fn append_point(
    document: &mut Document,
    name: &'static str,
    payload: &AppendPointPayload,
    as_slice: bool,
) -> Result<Delta, EngineError> {
    const SHAPE_POLYGON: i64 = 2;
    if !payload.x.is_finite() || !payload.y.is_finite() {
        return Err(EngineError::BadPayload {
            name: name.to_owned(),
            detail: "point is not finite".to_owned(),
        });
    }
    if !payload.tolerance.is_finite() || payload.tolerance < 0.0 {
        return Err(EngineError::BadPayload {
            name: name.to_owned(),
            detail: format!("tolerance {} is negative or not finite", payload.tolerance),
        });
    }
    // The selected target: topmost matching polygon of the class with
    // an open outline.
    let target = document.object_selection.iter().rev().copied().find(|id| {
        match document.registry.get(*id) {
            Some(ObjectData::Slice(slice)) => {
                slice.shape == SHAPE_POLYGON
                    && slice.slice_type.is_some() == as_slice
                    && slice.contours.last().is_some_and(|contour| !contour.closed)
            }
            _ => false,
        }
    });
    if let Some(id) = target {
        let Some(ObjectData::Slice(slice)) = document.registry.get_mut(id) else {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("object {} vanished mid-command", id.index()),
            });
        };
        let before = slice.clone();
        let Some(contour) = slice.contours.last_mut() else {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("polygon {} carries no outline", id.index()),
            });
        };
        let start = contour.points[0].anchor;
        contour.points.push(PathPoint {
            anchor: geom::Point::new(payload.x, payload.y),
            primary: None,
            secondary: None,
            is_curve: false,
        });
        contour.point_count += 1;
        if ((payload.x - start.x).powi(2) + (payload.y - start.y).powi(2)).sqrt()
            <= payload.tolerance
        {
            contour.closed = true;
        }
        slice.frame.x0 = slice.frame.x0.min(payload.x);
        slice.frame.y0 = slice.frame.y0.min(payload.y);
        slice.frame.x1 = slice.frame.x1.max(payload.x);
        slice.frame.y1 = slice.frame.y1.max(payload.y);
        return Ok(Delta::Object {
            id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(ObjectData::Slice(before)),
            },
        });
    }
    // No open polygon of the class selected: start a fresh one with
    // the single point (an open one-point polyline — the in-progress
    // marker the next append extends).
    let (layer, layers_before) = ensure_web_layer(document, name, payload.page)?;
    access::cell_mut(document, name, payload.page, layer, 0)?;
    let object = ObjectData::Slice(SliceObject {
        internal_name: None,
        frame: geom::Rect::new(payload.x, payload.y, payload.x, payload.y),
        shape: SHAPE_POLYGON,
        fill_raw: 0,
        name: None,
        slice_type: if as_slice { Some(0) } else { None },
        link: None,
        alt: None,
        label: None,
        td_text: None,
        symbol_number: None,
        contours: vec![Contour {
            closed: false,
            point_count: 1,
            points: vec![PathPoint {
                anchor: geom::Point::new(payload.x, payload.y),
                primary: None,
                secondary: None,
                is_curve: false,
            }],
        }],
        background: None,
        behaviors: Vec::new(),
        custom_data: std::collections::BTreeMap::new(),
        export: None,
    });
    let id = document.registry.insert(object);
    let placed = attach_cell(document, name, payload.page, layer, 0, id, None)?;
    let mut deltas = Vec::new();
    if let Some(layers) = layers_before {
        deltas.push(Delta::Layers {
            page: payload.page,
            layers,
        });
    }
    deltas.push(placed);
    deltas.push(Delta::Object {
        id,
        inverse: ObjectInverse::Created,
    });
    Ok(Delta::Group(deltas))
}

/// Registers the web-layer commands.
pub fn install(registry: &mut crate::registry::CommandRegistry) -> Result<(), EngineError> {
    registry.register(CreateSlice)?;
    registry.register(CreateHotspot)?;
    registry.register(AppendPointToSlice)?;
    registry.register(AppendPointToHotspot)?;
    registry.register(SetLink)?;
    registry.register(SetSliceType)?;
    registry.register(SetBackground)?;
    registry.register(SetSliceExport)?;
    registry.register(AddBehavior)?;
    registry.register(RemoveBehavior)?;
    registry.register(SetSliceData)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::super::testdoc;
    use super::*;
    use crate::registry::CommandRegistry;
    use serde_json::json;

    fn registry() -> CommandRegistry {
        let mut registry = CommandRegistry::new();
        install(&mut registry).unwrap();
        registry
    }

    fn web_layer_index(document: &Document, page: usize) -> Option<usize> {
        document.pages[page]
            .layers
            .layers
            .iter()
            .position(|layer| layer.has_url && layer.is_special)
    }

    fn the_slice(document: &Document) -> (doc::ObjectId, &SliceObject) {
        for (id, object) in document.registry.iter() {
            if let ObjectData::Slice(slice) = object {
                return (id, slice);
            }
        }
        panic!("no slice in the document");
    }

    #[test]
    fn create_provisions_the_web_layer_and_undo_strips_it() {
        let (mut document, _ids) = testdoc::sample_document();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        let delta = registry
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "rect", "x": 8.0, "y": 4.0,
                    "width": 32.0, "height": 16.0,
                    "page": 0,
                    "link": "https://firecraft.example/",
                    "alt": "hero",
                }),
            )
            .unwrap();

        let index = web_layer_index(&document, 0).expect("web layer provisioned");
        assert_eq!(
            index,
            document.pages[0].layers.layers.len() - 1,
            "the web layer sits on top of the stack"
        );
        let layer = &document.pages[0].layers.layers[index];
        assert_eq!(layer.name, "Web Layer");
        assert!(layer.has_url && layer.is_special && layer.display);
        assert_eq!(layer.cells.len(), 2, "one cell per state");
        assert!(layer.cells.iter().all(|cell| cell.visible && !cell.locked));

        let (id, slice) = the_slice(&document);
        assert_eq!(
            slice.slice_type,
            Some(0),
            "slice.create is a foreground slice"
        );
        assert_eq!(slice.shape, 0);
        assert_eq!(slice.link.as_deref(), Some("https://firecraft.example/"));
        assert_eq!(slice.alt.as_deref(), Some("hero"));
        assert_eq!(slice.frame, geom::Rect::new(8.0, 4.0, 40.0, 20.0));
        assert_eq!(layer.cells[0].elements, vec![id]);
        assert!(layer.cells[1].elements.is_empty());

        delta.apply_inverse(&mut document).unwrap();
        assert!(
            web_layer_index(&document, 0).is_none(),
            "undo strips the provisioning with the slice"
        );
        assert!(document.registry.get(id).is_none());
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            original,
            "undo restores the exact pre-create bytes"
        );
    }

    #[test]
    fn a_second_create_reuses_the_provisioned_web_layer() {
        let (mut document, _ids) = testdoc::sample_document();
        let registry = registry();
        for x in [8.0, 64.0] {
            registry
                .execute(
                    &mut document,
                    "slice.create",
                    json!({
                        "shape": "rect", "x": x, "y": 0.0,
                        "width": 16.0, "height": 16.0, "page": 0,
                    }),
                )
                .unwrap();
        }
        assert_eq!(
            document.pages[0]
                .layers
                .layers
                .iter()
                .filter(|layer| layer.is_special)
                .count(),
            1,
            "provisioning happens once"
        );
        let layer = &document.pages[0]
            .layers
            .layers
            .last()
            .expect("layer list nonempty");
        assert_eq!(layer.cells[0].elements.len(), 2);
    }

    #[test]
    fn hotspots_are_map_only_and_page_local() {
        let (mut document, _ids) = testdoc::sample_document();
        let registry = registry();
        registry
            .execute(
                &mut document,
                "hotspot.create",
                json!({
                    "shape": "rect", "x": 0.0, "y": 0.0,
                    "width": 10.0, "height": 10.0, "page": 1,
                }),
            )
            .unwrap();
        assert!(
            web_layer_index(&document, 0).is_none(),
            "page 0 keeps no web layer"
        );
        let (_, slice) = the_slice(&document);
        assert_eq!(slice.slice_type, None, "hotspots carry no TSLi");
    }

    #[test]
    fn polygons_need_three_points_and_keep_their_contour() {
        let (mut document, _ids) = testdoc::sample_document();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        let err = registry
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "polygon", "x": 0.0, "y": 0.0,
                    "width": 10.0, "height": 10.0,
                    "points": [{"x": 0.0, "y": 0.0}, {"x": 5.0, "y": 0.0}],
                    "page": 0,
                }),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidState { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            original,
            "the refusal mutates nothing"
        );

        registry
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "polygon", "x": 0.0, "y": 0.0,
                    "width": 10.0, "height": 10.0,
                    "points": [
                        {"x": 0.0, "y": 0.0},
                        {"x": 5.0, "y": 0.0},
                        {"x": 5.0, "y": 5.0},
                    ],
                    "page": 0,
                }),
            )
            .unwrap();
        let (_, slice) = the_slice(&document);
        assert_eq!(slice.shape, 2);
        assert_eq!(slice.contours.len(), 1);
        assert_eq!(slice.contours[0].point_count, 3);
        assert_eq!(slice.contours[0].points.len(), 3);
        assert_eq!(
            slice.contours[0].points[2].anchor,
            geom::Point::new(5.0, 5.0)
        );
    }

    #[test]
    fn set_commands_round_trip_through_undo() {
        let (mut document, _ids) = testdoc::sample_document();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        let mut deltas = Vec::new();
        deltas.push(
            registry
                .execute(
                    &mut document,
                    "slice.create",
                    json!({
                        "shape": "rect", "x": 0.0, "y": 0.0,
                        "width": 10.0, "height": 10.0, "page": 0,
                    }),
                )
                .unwrap(),
        );
        let (id, _) = the_slice(&document);

        for (command, payload) in [
            (
                "slice.setLink",
                json!({"id": id, "link": "https://firecraft.example/", "alt": "hero"}),
            ),
            ("slice.setType", json!({"id": id, "slice_type": 2})),
            (
                "slice.setBackground",
                json!({"id": id, "is_background": true, "repeat": 2,
                       "h_position": 3, "h_value": 12}),
            ),
            (
                "slice.setExport",
                json!({"id": id, "format": 1, "jpeg_quality": 82}),
            ),
            ("slice.addBehavior", json!({"id": id, "kind": 9})),
            (
                "slice.setData",
                json!({"id": id, "key": "swapTarget", "value": "hero_f2.png"}),
            ),
        ] {
            deltas.push(registry.execute(&mut document, command, payload).unwrap());
        }

        let (_, slice) = the_slice(&document);
        assert_eq!(slice.slice_type, Some(2));
        assert_eq!(slice.link.as_deref(), Some("https://firecraft.example/"));
        let background = slice.background.as_ref().expect("family minted");
        assert_eq!(background.is_background, Some(true));
        assert_eq!(background.repeat, Some(2));
        assert_eq!(background.h_position, Some(3));
        assert_eq!(background.h_value, Some(12));
        assert_eq!(background.v_position, None);
        let export = slice.export.as_ref().expect("override minted");
        assert_eq!(export.format, Some(1));
        assert_eq!(export.jpeg_quality, Some(82));
        assert_eq!(slice.behaviors, vec![SliceBehavior { kind: 9 }]);
        assert_eq!(
            slice.custom_data.get("swapTarget").map(String::as_str),
            Some("hero_f2.png")
        );

        // Clearing the last background field drops the whole family.
        registry
            .execute(
                &mut document,
                "slice.setBackground",
                json!({"id": id, "is_background": null, "repeat": null,
                       "attachment": null, "h_position": null, "h_value": null,
                       "v_position": null, "v_value": null}),
            )
            .unwrap();
        let (_, slice) = the_slice(&document);
        eprintln!("bg after clear: {:#?}", slice.background);
        assert!(
            slice.background.is_none(),
            "an all-default family is dropped"
        );

        // Clearing custom data removes the key.
        registry
            .execute(
                &mut document,
                "slice.setData",
                json!({"id": id, "key": "swapTarget", "value": null}),
            )
            .unwrap();
        let (_, slice) = the_slice(&document);
        assert!(slice.custom_data.is_empty());

        // Undo everything: the document returns to its pre-create bytes.
        for delta in deltas.iter().rev() {
            delta.apply_inverse(&mut document).unwrap();
        }
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            original,
            "undoing every command restores the exact bytes"
        );
    }

    #[test]
    fn set_type_and_export_reject_out_of_domain_values() {
        let (mut document, _ids) = testdoc::sample_document();
        let registry = registry();
        registry
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "rect", "x": 0.0, "y": 0.0,
                    "width": 10.0, "height": 10.0, "page": 0,
                }),
            )
            .unwrap();
        let baseline = serde_json::to_vec(&document).unwrap();
        let (id, _) = the_slice(&document);

        for (command, payload) in [
            ("slice.setType", json!({"id": id, "slice_type": 1})),
            ("slice.setBackground", json!({"id": id, "repeat": 4})),
            ("slice.setExport", json!({"id": id, "jpeg_quality": 101})),
            ("slice.removeBehavior", json!({"id": id, "index": 0})),
        ] {
            let err = registry
                .execute(&mut document, command, payload)
                .unwrap_err();
            assert!(matches!(err, EngineError::InvalidState { .. }), "{command}");
        }
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            baseline,
            "refusals mutate nothing"
        );
    }

    #[test]
    fn the_web_layer_refuses_deletion() {
        let (mut document, _ids) = testdoc::sample_document();
        let mut registry = registry();
        super::super::layer::install(&mut registry).unwrap();
        registry
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "rect", "x": 0.0, "y": 0.0,
                    "width": 10.0, "height": 10.0, "page": 0,
                }),
            )
            .unwrap();
        let index = web_layer_index(&document, 0).expect("provisioned");
        let err = registry
            .execute(
                &mut document,
                "layer.delete",
                json!({"page": 0, "layer": index}),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidState { .. }));
        assert!(web_layer_index(&document, 0).is_some(), "still there");
    }

    #[test]
    fn non_slice_targets_are_refused() {
        let (mut document, ids) = testdoc::sample_document();
        let registry = registry();
        let err = registry
            .execute(
                &mut document,
                "slice.setLink",
                json!({"id": ids[0], "link": "https://firecraft.example/", "alt": null}),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidState { .. }));
    }

    /// Every slice/hotspot id, in registry order.
    fn slice_ids(document: &Document) -> Vec<doc::ObjectId> {
        document
            .registry
            .iter()
            .filter(|(_, object)| matches!(object, ObjectData::Slice(_)))
            .map(|(id, _)| id)
            .collect()
    }

    #[test]
    fn append_point_starts_a_polygon_and_closes_it() {
        let (mut document, _) = testdoc::sample_document();
        let registry = registry();
        // Nothing selected: the verb starts a fresh polygon slice —
        // an open one-point polyline.
        let delta = registry
            .execute(
                &mut document,
                "slice.appendPoint",
                json!({"x": 0.0, "y": 0.0, "tolerance": 0.0, "page": 0}),
            )
            .unwrap();
        let id = slice_ids(&document)[0];
        let ObjectData::Slice(slice) = document.registry.get(id).unwrap() else {
            panic!("expected a slice");
        };
        assert_eq!(slice.shape, 2);
        assert_eq!(slice.slice_type, Some(0), "a slice, not a hotspot");
        assert!(!slice.contours[0].closed);
        assert_eq!(slice.contours[0].point_count, 1);

        // Selected: subsequent appends extend the polyline...
        document.object_selection = vec![id];
        for point in [[10.0, 0.0], [10.0, 10.0], [0.0, 10.0]] {
            registry
                .execute(
                    &mut document,
                    "slice.appendPoint",
                    json!({
                        "x": point[0], "y": point[1],
                        "tolerance": 0.0, "page": 0,
                    }),
                )
                .unwrap();
        }
        let ObjectData::Slice(slice) = document.registry.get(id).unwrap() else {
            panic!("expected a slice");
        };
        assert_eq!(slice.contours[0].point_count, 4, "start + three appends");
        assert!(!slice.contours[0].closed);
        assert_eq!(slice.frame, geom::Rect::new(0.0, 0.0, 10.0, 10.0));

        // ...and landing on the start point closes it (tolerance 0 is
        // an exact-match close).
        let delta2 = registry
            .execute(
                &mut document,
                "slice.appendPoint",
                json!({"x": 0.0, "y": 0.0, "tolerance": 0.0, "page": 0}),
            )
            .unwrap();
        let ObjectData::Slice(slice) = document.registry.get(id).unwrap() else {
            panic!("expected a slice");
        };
        assert!(slice.contours[0].closed, "within tolerance of the start");
        // ...but the point still lands (5 anchors).
        assert_eq!(slice.contours[0].point_count, 5);

        // Undo peels back to the fresh polygon, then to no slice.
        delta2.apply_inverse(&mut document).unwrap();
        let ObjectData::Slice(slice) = document.registry.get(id).unwrap() else {
            panic!("expected a slice");
        };
        assert!(!slice.contours[0].closed);
        assert_eq!(slice.contours[0].point_count, 4);
        delta.apply_inverse(&mut document).unwrap();
        assert!(slice_ids(&document).is_empty(), "undo removes the slice");
    }

    #[test]
    fn append_point_honors_class_and_shape() {
        let (mut document, _) = testdoc::sample_document();
        let registry = registry();
        // A rectangle slice cannot take points; a selected rect slice
        // is invisible to the verb, which starts a new polygon.
        registry
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "rect", "x": 0.0, "y": 0.0,
                    "width": 40.0, "height": 30.0,
                    "points": null, "page": 0,
                    "name": null, "link": null, "alt": null,
                }),
            )
            .unwrap();
        let rect_id = slice_ids(&document)[0];
        document.object_selection = vec![rect_id];
        registry
            .execute(
                &mut document,
                "slice.appendPoint",
                json!({"x": 5.0, "y": 5.0, "tolerance": 0.0, "page": 0}),
            )
            .unwrap();
        let ids = slice_ids(&document);
        assert_eq!(ids.len(), 2, "a fresh polygon, not the rect");
        let ObjectData::Slice(rect) = document.registry.get(rect_id).unwrap() else {
            panic!("expected a slice");
        };
        assert!(
            rect.contours.iter().all(|contour| contour.point_count == 0),
            "the rect is untouched (it carries one empty contour)"
        );
        let ObjectData::Slice(fresh) = document.registry.get(ids[1]).unwrap() else {
            panic!("expected a slice");
        };
        assert_eq!(fresh.slice_type, Some(0));

        // Class matters too: a selected open hotspot is invisible to
        // the slice verb. Select the fresh slice's twin by making it a
        // hotspot first.
        registry
            .execute(
                &mut document,
                "slice.setType",
                json!({"id": ids[1], "slice_type": null}),
            )
            .unwrap();
        document.object_selection = vec![ids[1]];
        registry
            .execute(
                &mut document,
                "slice.appendPoint",
                json!({"x": 6.0, "y": 6.0, "tolerance": 0.0, "page": 0}),
            )
            .unwrap();
        assert_eq!(slice_ids(&document).len(), 3, "class mismatch -> create");
        // The hotspot verb matches the open hotspot.
        document.object_selection = vec![ids[1]];
        registry
            .execute(
                &mut document,
                "hotspot.appendPoint",
                json!({"x": 7.0, "y": 7.0, "tolerance": 0.0, "page": 0}),
            )
            .unwrap();
        let count = slice_ids(&document).len();
        assert_eq!(count, 3, "the hotspot extended in place");
        let ObjectData::Slice(hotspot) = document.registry.get(ids[1]).unwrap() else {
            panic!("expected a slice");
        };
        assert_eq!(hotspot.contours[0].point_count, 2, "one append landed");
        assert_eq!(hotspot.slice_type, None, "still a hotspot");
    }

    #[test]
    fn append_point_refusals_leave_the_document_untouched() {
        let (mut document, _) = testdoc::sample_document();
        let registry = registry();
        let before = serde_json::to_vec(&document).unwrap();
        for payload in [
            json!({"x": 0.0, "y": 0.0, "tolerance": -1.0, "page": 0}),
            json!({"x": 0.0, "y": null, "tolerance": 1.0, "page": 0}),
        ] {
            let err = registry
                .execute(&mut document, "slice.appendPoint", payload)
                .unwrap_err();
            assert!(matches!(err, EngineError::BadPayload { .. }));
            assert_eq!(
                serde_json::to_vec(&document).unwrap(),
                before,
                "a refused append must leave the document untouched"
            );
        }
    }
}
