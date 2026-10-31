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
use crate::catalog::document::map_contours;
use crate::catalog::object::{attach_cell, content_bounds};
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

/// `slice.addSwapImageBehaviorFromPoint` — CS6's
/// `dom.addSwapImageBehaviorFromPoint(where)`: "If a single hotspot or
/// slice is selected, this function adds to it a swap image behavior
/// from the hotspot or slice located at `where`." The selected slice
/// gains behavior kind 2 (swap image — the webexport hook constant)
/// and `DTAv` `swapTarget` = the resolved slice's object name (falling
/// back to its internal name; a target-less swap emits no hook).
/// CS6's `false` return ("no suitable hotspot at the location")
/// carries as a refusal. Polygon slices hit-test their outline; the
/// rest hit-test their frame. Nothing mutates before the source
/// resolves (000557d).
struct AddSwapImageBehaviorFromPoint;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddSwapImageBehaviorFromPointPayload {
    /// Document point naming the swap-image source slice.
    #[serde(rename = "where")]
    at: geom::Point,
}

/// Even-odd point-in-polygon over a slice's `PBPv` outline.
fn polygon_contains(slice: &SliceObject, at: geom::Point) -> bool {
    let mut inside = false;
    for contour in &slice.contours {
        let mut previous = contour.points.last().map(|point| point.anchor);
        for point in &contour.points {
            let Some(a) = previous else { break };
            let b = point.anchor;
            previous = Some(b);
            if (a.y > at.y) == (b.y > at.y) {
                continue;
            }
            if at.x < (b.x - a.x) * (at.y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
        }
    }
    inside
}

/// The slice containing `at`, scanning every cell of every layer of
/// every page topmost-last (z-order is list order, bottom-first).
fn slice_at(document: &Document, at: geom::Point) -> Option<doc::ObjectId> {
    let mut found = None;
    for page in &document.pages {
        for layer in &page.layers.layers {
            for cell in &layer.cells {
                for id in &cell.elements {
                    let Some(ObjectData::Slice(slice)) = document.registry.get(*id) else {
                        continue;
                    };
                    let hit = match slice.shape {
                        // 2 polygon: the outline; the frame is the
                        // bounding box, not the shape.
                        2 => polygon_contains(slice, at),
                        _ => slice.frame.contains(at),
                    };
                    if hit {
                        found = Some(*id);
                    }
                }
            }
        }
    }
    found
}

impl TypedCommand for AddSwapImageBehaviorFromPoint {
    type Payload = AddSwapImageBehaviorFromPointPayload;

    fn name(&self) -> &'static str {
        "slice.addSwapImageBehaviorFromPoint"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "slice.addSwapImageBehaviorFromPoint";
        if !payload.at.x.is_finite() || !payload.at.y.is_finite() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "where must be finite".to_owned(),
            });
        }
        let mut selected = document.object_selection.clone();
        selected.retain(|id| matches!(document.registry.get(*id), Some(ObjectData::Slice(_))));
        if selected.len() != 1 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "exactly one selected slice or hotspot is required ({} selected)",
                    selected.len()
                ),
            });
        }
        // Resolve the source before any capture or mutation.
        let Some(source) = slice_at(document, payload.at) else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "no slice or hotspot at the given point".to_owned(),
            });
        };
        let target = match document.registry.get(source) {
            Some(ObjectData::Slice(slice)) => slice
                .name
                .clone()
                .or_else(|| slice.internal_name.clone())
                .unwrap_or_default(),
            _ => String::new(),
        };
        let id = selected[0];
        let before = access::object(document, NAME, id)?.clone();
        let slice = slice_mut(document, NAME, id)?;
        slice.behaviors.push(SliceBehavior { kind: 2 });
        if !target.is_empty() {
            slice.custom_data.insert("swapTarget".to_owned(), target);
        }
        Ok(Delta::Object {
            id,
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

/// `hotspot.setRectangle` — CS6
/// `dom.setHotspotRectangle(boundingRectangle, bMakeCopy)`: the single
/// selected hotspot or slice moves (or copies, `bMakeCopy`) to the
/// rect; polygon contours map through `T(new.origin)·S(sx, sy)` so a
/// polyline stays glued to its frame (the 0041 mapper).
struct SetHotspotRectangle;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct HotspotRectPayload {
    /// The new bounds.
    rect: HotspotRect,
    /// Copy-then-resize instead of move-and-resize; the selection
    /// lands on the copy.
    #[serde(default)]
    make_copy: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct HotspotRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl TypedCommand for SetHotspotRectangle {
    type Payload = HotspotRectPayload;

    fn name(&self) -> &'static str {
        "hotspot.setRectangle"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "hotspot.setRectangle";
        let rect = &payload.rect;
        if !rect.left.is_finite()
            || !rect.top.is_finite()
            || !rect.right.is_finite()
            || !rect.bottom.is_finite()
            || rect.right <= rect.left
            || rect.bottom <= rect.top
        {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "rect must be finite with positive extents".to_owned(),
            });
        }
        if document.object_selection.len() != 1 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "the selection must be a single hotspot or slice (got {})",
                    document.object_selection.len()
                ),
            });
        }
        let id = document.object_selection[0];
        let before = access::object(document, NAME, id)?.clone();
        let ObjectData::Slice(slice) = &before else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} is not a slice or hotspot", id.index()),
            });
        };
        if slice.frame.width() <= 0.0 || slice.frame.height() <= 0.0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the current frame is degenerate; recreate the hotspot instead".to_owned(),
            });
        }
        // p ↦ new.origin + s·(p − old.origin), as T(t)·S(s).
        let (new_w, new_h) = (rect.right - rect.left, rect.bottom - rect.top);
        let (sx, sy) = (new_w / slice.frame.width(), new_h / slice.frame.height());
        let map = geom::Affine::translate(geom::Vec2::new(
            rect.left - slice.frame.x0 * sx,
            rect.top - slice.frame.y0 * sy,
        )) * geom::Affine::new([sx, 0.0, 0.0, sy, 0.0, 0.0]);
        if !payload.make_copy {
            let slot = match document.registry.get_mut(id) {
                Some(ObjectData::Slice(slot)) => slot,
                _ => {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: format!("object {} vanished mid-command", id.index()),
                    });
                }
            };
            slot.frame = geom::Rect::new(rect.left, rect.top, rect.right, rect.bottom);
            map_contours(&mut slot.contours, map);
            return Ok(Delta::Object {
                id,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(before),
                },
            });
        }
        // Copy mode: the clone lands next to the original in the same
        // cell; the selection moves to it (CS6 "copied and resized").
        let Some(super::object::Owner::Cell {
            page,
            layer,
            cell,
            position,
        }) = super::object::find_owner(document, id)
        else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the hotspot is not attached to a cell".to_owned(),
            });
        };
        let mut copy = slice.clone();
        copy.frame = geom::Rect::new(rect.left, rect.top, rect.right, rect.bottom);
        map_contours(&mut copy.contours, map);
        let copy_id = document.registry.insert(ObjectData::Slice(copy));
        let placed = attach_cell(
            document,
            NAME,
            page,
            layer,
            cell,
            copy_id,
            Some(position + 1),
        )?;
        let selection_before = std::mem::take(&mut document.object_selection);
        document.object_selection.push(copy_id);
        Ok(Delta::Group(vec![
            Delta::ObjectSelection {
                before: selection_before,
            },
            placed,
            Delta::Object {
                id: copy_id,
                inverse: ObjectInverse::Created,
            },
        ]))
    }
}

/// `hotspot.setShape` — CS6
/// `dom.setHotspotShape(whatToSet, shape)`: sets the `URSi` shape on
/// the selected slices, hotspots, or both. `polyline` needs no
/// contours on the wire (the frame carries bounds either way).
struct SetHotspotShape;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetHotspotShapePayload {
    /// "hotspots", "slices", or "hotspots and slices".
    what: String,
    /// "rectangle", "oval", or "polyline".
    shape: String,
}

impl TypedCommand for SetHotspotShape {
    type Payload = SetHotspotShapePayload;

    fn name(&self) -> &'static str {
        "hotspot.setShape"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "hotspot.setShape";
        let shape = match payload.shape.as_str() {
            "rectangle" => 0i64,
            "oval" => 1,
            "polyline" => 2,
            other => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("unknown shape {other:?} (want rectangle/oval/polyline)"),
                });
            }
        };
        // Which class the verb touches: `None` = both.
        let class = match payload.what.as_str() {
            "hotspots" => Some(false),
            "slices" => Some(true),
            "hotspots and slices" => None,
            other => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "unknown whatToSet {other:?} (want hotspots/slices/hotspots and slices)"
                    ),
                });
            }
        };
        if document.object_selection.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "nothing is selected".to_owned(),
            });
        }
        let mut deltas = Vec::new();
        for id in document.object_selection.clone() {
            let before = match document.registry.get(id) {
                Some(ObjectData::Slice(slice))
                    if class.is_none_or(|want_slice| slice.slice_type.is_some() == want_slice) =>
                {
                    if slice.shape == shape {
                        continue;
                    }
                    slice.clone()
                }
                // Non-web objects and off-class slices are skipped —
                // the verb addresses "the hotspots and slices in the
                // selection".
                _ => continue,
            };
            if let Some(ObjectData::Slice(slot)) = document.registry.get_mut(id) {
                slot.shape = shape;
            }
            deltas.push(Delta::Object {
                id,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(ObjectData::Slice(before)),
                },
            });
        }
        if deltas.is_empty() {
            return Ok(Delta::Noop);
        }
        Ok(Delta::Group(deltas))
    }
}

/// `hotspot.copyToHotspot` — CS6
/// `dom.copyToHotspot(hotspotType, whatIfMultipleSelected,
/// makeRectangular)`: creates hotspots (or slices) covering the
/// selection — one union shape for "single", one per item for
/// "multiple". Omitted `whatIfMultipleSelected` is CS6's "ask user",
/// which has no headless answer and refuses. With
/// `make_rectangular` (the default) every shape is a rectangle;
/// otherwise a polygon source donates its contours.
struct CopyToHotspot;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct CopyToHotspotPayload {
    page: usize,
    /// "hotspot" or "slice".
    hotspot_type: String,
    /// "single" or "multiple"; `None` is "ask user" and refuses
    /// headless.
    #[serde(default)]
    what_if_multiple: Option<String>,
    /// Rectangle unless the source is a polygon and this is false.
    #[serde(default = "default_make_rectangular")]
    make_rectangular: bool,
}

fn default_make_rectangular() -> bool {
    true
}

/// The anchor bounds of a contour list (handles never extend the
/// frame on the wire).
fn contour_bounds(contours: &[Contour]) -> Option<geom::Rect> {
    let mut bounds: Option<geom::Rect> = None;
    for contour in contours {
        for point in &contour.points {
            let (x, y) = (point.anchor.x, point.anchor.y);
            bounds = Some(match bounds {
                Some(rect) => geom::Rect::new(
                    rect.x0.min(x),
                    rect.y0.min(y),
                    rect.x1.max(x),
                    rect.y1.max(y),
                ),
                None => geom::Rect::new(x, y, x, y),
            });
        }
    }
    bounds
}

impl TypedCommand for CopyToHotspot {
    type Payload = CopyToHotspotPayload;

    fn name(&self) -> &'static str {
        "hotspot.copyToHotspot"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "hotspot.copyToHotspot";
        let slice_type = match payload.hotspot_type.as_str() {
            "hotspot" => None,
            "slice" => Some(0i64),
            other => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("unknown hotspotType {other:?} (want hotspot/slice)"),
                });
            }
        };
        let multiple = match payload.what_if_multiple.as_deref() {
            Some("single") => false,
            Some("multiple") => true,
            // CS6 shows a dialog; headless has no one to ask.
            _ => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: "what_if_multiple must be \"single\" or \"multiple\" (CS6's \"ask user\" has no headless answer)"
                        .to_owned(),
                });
            }
        };
        if document.object_selection.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "nothing is selected".to_owned(),
            });
        }
        // Resolve every source shape before anything mutates
        // (000557d): bounds for the frame, contours when a polygon
        // source donates them.
        let mut shapes = Vec::with_capacity(if multiple {
            document.object_selection.len()
        } else {
            1
        });
        if !multiple {
            let mut bounds = None;
            let mut seen = std::collections::HashSet::new();
            for id in &document.object_selection {
                content_bounds(
                    document,
                    *id,
                    geom::Affine::IDENTITY,
                    0,
                    &mut seen,
                    &mut bounds,
                );
            }
            let Some(bounds) = bounds else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: "the selection carries no bounds to cover".to_owned(),
                });
            };
            shapes.push((bounds, 0i64, Vec::new()));
        } else {
            for id in document.object_selection.clone() {
                // A polygon source donates its contours when
                // make_rectangular is false; everything else takes a
                // rectangular frame over its content bounds.
                let donated = if payload.make_rectangular {
                    None
                } else {
                    match document.registry.get(id) {
                        Some(ObjectData::Slice(slice))
                            if slice.shape == 2 && !slice.contours.is_empty() =>
                        {
                            Some(slice.contours.clone())
                        }
                        Some(ObjectData::Path(path)) if !path.contours.is_empty() => {
                            Some(path.contours.clone())
                        }
                        _ => None,
                    }
                };
                if let Some(contours) = donated {
                    let Some(bounds) = contour_bounds(&contours) else {
                        return Err(EngineError::InvalidState {
                            name: NAME.to_owned(),
                            detail: "polygon contours carry no anchors".to_owned(),
                        });
                    };
                    shapes.push((bounds, 2, contours));
                    continue;
                }
                let mut bounds = None;
                let mut seen = std::collections::HashSet::new();
                content_bounds(
                    document,
                    id,
                    geom::Affine::IDENTITY,
                    0,
                    &mut seen,
                    &mut bounds,
                );
                let Some(bounds) = bounds else {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: format!("object {} carries no bounds to cover", id.index()),
                    });
                };
                shapes.push((bounds, 0, Vec::new()));
            }
        }
        // Provision + validate the destination first.
        let (layer, layers_before) = ensure_web_layer(document, NAME, payload.page)?;
        access::cell_mut(document, NAME, payload.page, layer, 0)?;
        let mut deltas = Vec::new();
        if let Some(layers) = layers_before {
            deltas.push(Delta::Layers {
                page: payload.page,
                layers,
            });
        }
        let selection_before = std::mem::take(&mut document.object_selection);
        let mut created = Vec::with_capacity(shapes.len());
        for (bounds, shape, contours) in shapes {
            let object = ObjectData::Slice(SliceObject {
                internal_name: None,
                frame: bounds,
                shape,
                fill_raw: 0,
                name: None,
                slice_type,
                link: None,
                alt: None,
                label: None,
                td_text: None,
                symbol_number: None,
                contours,
                background: None,
                behaviors: Vec::new(),
                custom_data: std::collections::BTreeMap::new(),
                export: None,
            });
            let id = document.registry.insert(object);
            deltas.push(attach_cell(
                document,
                NAME,
                payload.page,
                layer,
                0,
                id,
                None,
            )?);
            deltas.push(Delta::Object {
                id,
                inverse: ObjectInverse::Created,
            });
            created.push(id);
        }
        document.object_selection = created;
        deltas.push(Delta::ObjectSelection {
            before: selection_before,
        });
        Ok(Delta::Group(deltas))
    }
}

/// `page.setWebObjectsVisibility` — CS3
/// `dom.setWebObjectsVisibility(value)`: the Web Layer eye bit on the
/// page (the layer provisions like any other first web touch).
struct SetWebObjectsVisibility;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetWebObjectsVisibilityPayload {
    page: usize,
    value: bool,
}

impl TypedCommand for SetWebObjectsVisibility {
    type Payload = SetWebObjectsVisibilityPayload;

    fn name(&self) -> &'static str {
        "page.setWebObjectsVisibility"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.setWebObjectsVisibility";
        let (layer, layers_before) = ensure_web_layer(document, NAME, payload.page)?;
        let before = Box::new(document.pages[payload.page].layers.layers[layer].clone());
        document.pages[payload.page].layers.layers[layer].display = payload.value;
        let mut deltas = Vec::new();
        if let Some(layers) = layers_before {
            deltas.push(Delta::Layers {
                page: payload.page,
                layers,
            });
        }
        deltas.push(Delta::Layer {
            page: payload.page,
            layer,
            before,
        });
        Ok(Delta::Group(deltas))
    }
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
    registry.register(AddSwapImageBehaviorFromPoint)?;
    registry.register(SetSliceData)?;
    registry.register(SetHotspotRectangle)?;
    registry.register(SetHotspotShape)?;
    registry.register(CopyToHotspot)?;
    registry.register(SetWebObjectsVisibility)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::super::testdoc;
    use super::*;
    use crate::command::Command;
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

    fn slice_at(document: &Document, id: doc::ObjectId) -> &SliceObject {
        match document.registry.get(id) {
            Some(ObjectData::Slice(slice)) => slice,
            _ => panic!("object {id:?} is not a slice"),
        }
    }

    fn make_polygon_hotspot(document: &mut Document) -> doc::ObjectId {
        registry()
            .execute(
                document,
                "hotspot.create",
                json!({
                    "shape": "polygon", "x": 0.0, "y": 0.0,
                    "width": 10.0, "height": 10.0,
                    "points": [
                        {"x": 0.0, "y": 0.0},
                        {"x": 10.0, "y": 0.0},
                        {"x": 10.0, "y": 10.0},
                    ],
                    "page": 0,
                }),
            )
            .unwrap();
        the_slice(document).0
    }

    #[test]
    fn set_rectangle_moves_and_maps_polygon_contours() {
        let (mut document, _ids) = testdoc::sample_document();
        let id = make_polygon_hotspot(&mut document);
        document.object_selection = vec![id];
        let before = serde_json::to_vec(&document).unwrap();

        let delta = registry()
            .execute(
                &mut document,
                "hotspot.setRectangle",
                json!({"rect": {"left": 10.0, "top": 10.0, "right": 30.0, "bottom": 30.0}}),
            )
            .unwrap();
        let slice = slice_at(&document, id);
        assert_eq!(
            (
                slice.frame.x0,
                slice.frame.y0,
                slice.frame.x1,
                slice.frame.y1
            ),
            (10.0, 10.0, 30.0, 30.0)
        );
        let anchors: Vec<(f64, f64)> = slice.contours[0]
            .points
            .iter()
            .map(|p| (p.anchor.x, p.anchor.y))
            .collect();
        assert_eq!(
            anchors,
            vec![(10.0, 10.0), (30.0, 10.0), (30.0, 30.0)],
            "contours ride the same map as the frame"
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), before, "undo");
    }

    #[test]
    fn set_rectangle_copy_mode_keeps_the_original() {
        let (mut document, _ids) = testdoc::sample_document();
        let original = make_polygon_hotspot(&mut document);
        document.object_selection = vec![original];

        registry()
            .execute(
                &mut document,
                "hotspot.setRectangle",
                json!({
                    "rect": {"left": 5.0, "top": 5.0, "right": 15.0, "bottom": 15.0},
                    "make_copy": true,
                }),
            )
            .unwrap();
        assert_eq!(
            slice_at(&document, original).frame,
            geom::Rect::new(0.0, 0.0, 10.0, 10.0),
            "the original stands"
        );
        let copy_id = document.object_selection[0];
        assert_ne!(copy_id, original, "the selection lands on the copy");
        assert_eq!(
            slice_at(&document, copy_id).frame,
            geom::Rect::new(5.0, 5.0, 15.0, 15.0)
        );
        assert_eq!(
            slice_at(&document, copy_id).contours[0].points.len(),
            slice_at(&document, original).contours[0].points.len(),
            "the copy carries the donor's contours"
        );

        // The second copy undoes cleanly too (fresh ids each time).
        let delta = registry()
            .execute(
                &mut document,
                "hotspot.setRectangle",
                json!({
                    "rect": {"left": 5.0, "top": 5.0, "right": 15.0, "bottom": 15.0},
                    "make_copy": true,
                }),
            )
            .unwrap();
        delta.apply_inverse(&mut document).unwrap();
        assert!(
            document
                .registry
                .get(document.object_selection[0])
                .is_some()
        );
    }

    #[test]
    fn set_rectangle_copy_undo_is_byte_exact() {
        let (mut document, _ids) = testdoc::sample_document();
        let original = make_polygon_hotspot(&mut document);
        document.object_selection = vec![original];
        let before = serde_json::to_vec(&document).unwrap();
        let delta = registry()
            .execute(
                &mut document,
                "hotspot.setRectangle",
                json!({
                    "rect": {"left": 5.0, "top": 5.0, "right": 15.0, "bottom": 15.0},
                    "make_copy": true,
                }),
            )
            .unwrap();
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "undo retracts the copy at its generation"
        );
    }

    #[test]
    fn set_shape_filters_by_class_and_refuses_junk() {
        let (mut document, _ids) = testdoc::sample_document();
        let registry = registry();
        registry
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "rect", "x": 0.0, "y": 0.0,
                    "width": 16.0, "height": 16.0, "page": 0,
                }),
            )
            .unwrap();
        let (slice_id, _) = the_slice(&document);
        registry
            .execute(
                &mut document,
                "hotspot.create",
                json!({
                    "shape": "rect", "x": 20.0, "y": 20.0,
                    "width": 8.0, "height": 8.0, "page": 0,
                }),
            )
            .unwrap();
        // the_slice returns the first; collect both web ids.
        let mut web_ids = Vec::new();
        for (id, object) in document.registry.iter() {
            if matches!(object, ObjectData::Slice(_)) {
                web_ids.push(id);
            }
        }
        web_ids.retain(|id| *id != slice_id);
        let hotspot_id = web_ids[0];
        document.object_selection = vec![slice_id, hotspot_id];

        // "slices" touches the slice only; "hotspots" the hotspot only.
        registry
            .execute(
                &mut document,
                "hotspot.setShape",
                json!({"what": "slices", "shape": "oval"}),
            )
            .unwrap();
        assert_eq!(slice_at(&document, slice_id).shape, 1);
        assert_eq!(slice_at(&document, hotspot_id).shape, 0);
        registry
            .execute(
                &mut document,
                "hotspot.setShape",
                json!({"what": "hotspots", "shape": "polyline"}),
            )
            .unwrap();
        assert_eq!(slice_at(&document, hotspot_id).shape, 2);
        assert_eq!(slice_at(&document, slice_id).shape, 1);
        let snapshot = serde_json::to_vec(&document).unwrap();

        for bad in [
            json!({"what": "areas", "shape": "oval"}),
            json!({"what": "slices", "shape": "round"}),
        ] {
            let err = registry
                .execute(&mut document, "hotspot.setShape", bad)
                .unwrap_err();
            assert!(matches!(err, EngineError::InvalidState { .. }));
            assert_eq!(
                serde_json::to_vec(&document).unwrap(),
                snapshot,
                "refusals mutate nothing"
            );
        }
    }

    #[test]
    fn copy_to_hotspot_covers_selects_and_undoes() {
        let (mut document, _ids) = testdoc::sample_document();
        let registry = registry();
        registry
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "rect", "x": 4.0, "y": 4.0,
                    "width": 16.0, "height": 16.0, "page": 0,
                }),
            )
            .unwrap();
        let (source_id, _) = the_slice(&document);
        document.object_selection = vec![source_id];
        let before = serde_json::to_vec(&document).unwrap();

        let delta = registry
            .execute(
                &mut document,
                "hotspot.copyToHotspot",
                json!({"page": 0, "hotspot_type": "hotspot", "what_if_multiple": "multiple"}),
            )
            .unwrap();
        let hotspot_id = document.object_selection[0];
        assert_ne!(hotspot_id, source_id, "the hotspot is selected");
        let hotspot = slice_at(&document, hotspot_id);
        assert_eq!(hotspot.slice_type, None, "map-only");
        assert_eq!(hotspot.shape, 0, "rectangular by default");
        assert_eq!(hotspot.frame, geom::Rect::new(4.0, 4.0, 20.0, 20.0));
        assert!(document.registry.get(source_id).is_some());

        // Undo the create: the hotspot is gone, the selection is back.
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.registry.get(hotspot_id).is_none());
        assert_eq!(document.object_selection, vec![source_id]);
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "undo restores the pre-create bytes"
        );

        // CS6's "ask user" default has no headless answer.
        let err = registry
            .execute(
                &mut document,
                "hotspot.copyToHotspot",
                json!({"page": 0, "hotspot_type": "hotspot"}),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidState { .. }));
        let err = registry
            .execute(
                &mut document,
                "hotspot.copyToHotspot",
                json!({"page": 0, "hotspot_type": "area", "what_if_multiple": "single"}),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidState { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "the refusals mutated nothing"
        );
    }

    #[test]
    fn copy_to_hotspot_donates_polygons_and_unions() {
        let (mut document, _ids) = testdoc::sample_document();
        let registry = registry();
        let id = make_polygon_hotspot(&mut document);
        document.object_selection = vec![id];

        // "single" covers the union as a rectangle.
        registry
            .execute(
                &mut document,
                "hotspot.copyToHotspot",
                json!({
                    "page": 0, "hotspot_type": "slice",
                    "what_if_multiple": "single", "make_rectangular": false,
                }),
            )
            .unwrap();
        let union_id = document.object_selection[0];
        let union = slice_at(&document, union_id);
        assert_eq!(union.shape, 0, "the union has no single source shape");
        assert_eq!(union.slice_type, Some(0), "hotspot_type slice");
        assert_eq!(union.frame, geom::Rect::new(0.0, 0.0, 10.0, 10.0));

        // "multiple" with make_rectangular false donates the contours.
        document.object_selection = vec![id];
        registry
            .execute(
                &mut document,
                "hotspot.copyToHotspot",
                json!({
                    "page": 0, "hotspot_type": "slice",
                    "what_if_multiple": "multiple", "make_rectangular": false,
                }),
            )
            .unwrap();
        let polygon = slice_at(&document, document.object_selection[0]);
        assert_eq!(polygon.shape, 2);
        let anchors: Vec<(f64, f64)> = polygon.contours[0]
            .points
            .iter()
            .map(|p| (p.anchor.x, p.anchor.y))
            .collect();
        assert_eq!(anchors, vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
    }

    #[test]
    fn web_objects_visibility_provisions_and_undoes() {
        let (mut document, _ids) = testdoc::sample_document();
        let before = serde_json::to_vec(&document).unwrap();
        let registry = registry();

        let delta = registry
            .execute(
                &mut document,
                "page.setWebObjectsVisibility",
                json!({"page": 1, "value": false}),
            )
            .unwrap();
        assert!(web_layer_index(&document, 1).is_some());
        let layer = &document.pages[1].layers.layers[web_layer_index(&document, 1).unwrap()];
        assert!(!layer.display, "the eye is off");
        assert!(web_layer_index(&document, 0).is_none(), "page 0 untouched");

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "undo retracts the provisioning"
        );

        let err = registry
            .execute(
                &mut document,
                "page.setWebObjectsVisibility",
                json!({"page": 9, "value": true}),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            EngineError::InvalidState { .. } | EngineError::MissingTarget { .. }
        ));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "the refused page mutated nothing"
        );
    }

    #[test]
    fn from_point_resolves_the_source_slice_and_undo_is_exact() {
        let (mut document, _ids) = testdoc::sample_document();
        let registry = registry();
        registry
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "rect", "x": 0.0, "y": 0.0,
                    "width": 10.0, "height": 10.0,
                    "name": "hero_f2", "page": 0,
                }),
            )
            .unwrap();
        registry
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "rect", "x": 20.0, "y": 20.0,
                    "width": 10.0, "height": 10.0,
                    "page": 0,
                }),
            )
            .unwrap();
        let (source, _) = the_slice(&document);
        let (target, _) = document
            .registry
            .iter()
            .find(|(id, object)| matches!(object, ObjectData::Slice(_)) && *id != source)
            .unwrap();
        document.object_selection = vec![target];
        let before = serde_json::to_vec(&document).unwrap();

        let delta = AddSwapImageBehaviorFromPoint
            .apply_json(&mut document, json!({ "where": { "x": 5.0, "y": 5.0 } }))
            .unwrap();
        let slice = slice_at(&document, target);
        assert_eq!(
            slice.behaviors.last().map(|behavior| behavior.kind),
            Some(2),
            "kind 2 is the swap-image behavior"
        );
        assert_eq!(
            slice.custom_data.get("swapTarget").map(String::as_str),
            Some("hero_f2"),
            "the swap target is the resolved slice's name"
        );
        assert!(slice_at(&document, source).behaviors.is_empty());

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "undo is byte-exact"
        );
    }

    #[test]
    fn from_point_walks_polygon_contours_and_gates() {
        let (mut document, _ids) = testdoc::sample_document();
        let polygon = make_polygon_hotspot(&mut document);
        assert_eq!(the_slice(&document).1.shape, 2, "fixture is a polygon");
        document.object_selection = vec![polygon];

        // Outside the triangle but inside its bounding box: no source.
        let error = AddSwapImageBehaviorFromPoint
            .apply_json(&mut document, json!({ "where": { "x": 1.0, "y": 9.0 } }))
            .unwrap_err();
        assert!(error.to_string().contains("no slice or hotspot"));

        // Inside the triangle: resolves (the name is empty, so no
        // swapTarget key is written).
        AddSwapImageBehaviorFromPoint
            .apply_json(&mut document, json!({ "where": { "x": 5.0, "y": 4.0 } }))
            .unwrap();
        assert_eq!(
            slice_at(&document, polygon)
                .behaviors
                .last()
                .map(|b| b.kind),
            Some(2)
        );
        assert!(
            !slice_at(&document, polygon)
                .custom_data
                .contains_key("swapTarget")
        );

        // Non-finite points cannot survive JSON (null fails decode
        // first); the finite gate is exercised directly.
        let error = AddSwapImageBehaviorFromPoint
            .apply(
                &mut document,
                &AddSwapImageBehaviorFromPointPayload {
                    at: geom::Point::new(f64::NAN, 0.0),
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("where must be finite"));

        // Two selected slices refuse before any resolution.
        let (mut document, _ids) = testdoc::sample_document();
        let registry = registry();
        for (x, y) in [(0.0, 0.0), (40.0, 40.0)] {
            registry
                .execute(
                    &mut document,
                    "slice.create",
                    json!({
                        "shape": "rect", "x": x, "y": y,
                        "width": 10.0, "height": 10.0, "page": 0,
                    }),
                )
                .unwrap();
        }
        let ids: Vec<doc::ObjectId> = document
            .registry
            .iter()
            .filter(|(_, object)| matches!(object, ObjectData::Slice(_)))
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids.len(), 2);
        document.object_selection = ids;
        let before = serde_json::to_vec(&document).unwrap();
        let error = AddSwapImageBehaviorFromPoint
            .apply_json(&mut document, json!({ "where": { "x": 5.0, "y": 5.0 } }))
            .unwrap_err();
        assert!(error.to_string().contains("exactly one selected slice"));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "refusals mutate nothing"
        );
    }

    #[test]
    fn set_export_quality_gate_and_clear() {
        let (mut document, _ids) = testdoc::sample_document();
        registry()
            .execute(
                &mut document,
                "slice.create",
                json!({
                    "shape": "rect", "x": 0.0, "y": 0.0,
                    "width": 10.0, "height": 10.0, "page": 0,
                }),
            )
            .unwrap();
        let (id, _) = the_slice(&document);

        let out_of_range = registry().execute(
            &mut document,
            "slice.setExport",
            json!({"id": id, "jpeg_quality": 101}),
        );
        assert!(matches!(
            out_of_range,
            Err(EngineError::InvalidState { .. })
        ));

        // Seed an export, clear it, and undo restores the node.
        registry()
            .execute(
                &mut document,
                "slice.setExport",
                json!({"id": id, "format": 1}),
            )
            .unwrap();
        let delta = registry()
            .execute(
                &mut document,
                "slice.setExport",
                json!({"id": id, "format": 1, "clear": true}),
            )
            .unwrap();
        assert!(slice_at(&document, id).export.is_none());
        delta.apply_inverse(&mut document).unwrap();
        assert!(slice_at(&document, id).export.is_some(), "undo restores");
    }

    #[test]
    fn remove_behavior_drops_the_indexed_entry_and_gates() {
        let (mut document, _ids) = testdoc::sample_document();
        let id = make_polygon_hotspot(&mut document);
        registry()
            .execute(
                &mut document,
                "slice.addBehavior",
                json!({"id": id, "kind": 7}),
            )
            .unwrap();
        registry()
            .execute(
                &mut document,
                "slice.addBehavior",
                json!({"id": id, "kind": 9}),
            )
            .unwrap();

        let delta = registry()
            .execute(
                &mut document,
                "slice.removeBehavior",
                json!({"id": id, "index": 0}),
            )
            .unwrap();
        assert_eq!(
            slice_at(&document, id).behaviors,
            vec![SliceBehavior { kind: 9 }],
            "the indexed entry is the one removed"
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            slice_at(&document, id).behaviors,
            vec![SliceBehavior { kind: 7 }, SliceBehavior { kind: 9 }]
        );

        let err = registry()
            .execute(
                &mut document,
                "slice.removeBehavior",
                json!({"id": id, "index": 2}),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidState { .. }));
    }

    #[test]
    fn set_rectangle_refusals_leave_the_document_alone() {
        let (mut document, _ids) = testdoc::sample_document();
        let id = make_polygon_hotspot(&mut document);
        document.object_selection = vec![id];
        let before = serde_json::to_vec(&document).unwrap();
        for payload in [
            json!({"rect": {"left": 10.0, "top": 10.0, "right": 10.0, "bottom": 20.0}}),
            json!({"rect": {"left": 10.0, "top": 20.0, "right": 10.0, "bottom": 10.0}}),
        ] {
            let err = registry()
                .execute(&mut document, "hotspot.setRectangle", payload)
                .unwrap_err();
            assert!(matches!(err, EngineError::InvalidState { .. }));
            assert_eq!(serde_json::to_vec(&document).unwrap(), before);
        }

        // An empty selection is refused before the id is resolved.
        document.object_selection.clear();
        let err = registry()
            .execute(
                &mut document,
                "hotspot.setRectangle",
                json!({"rect": {"left": 0.0, "top": 0.0, "right": 5.0, "bottom": 5.0}}),
            )
            .unwrap_err();
        assert!(err.to_string().contains("single hotspot or slice"));
        // And the wrong class of selection, too.
        let (_, ids) = testdoc::sample_document();
        let path_id = *ids.first().unwrap();
        document.object_selection = vec![path_id];
        let err = registry()
            .execute(
                &mut document,
                "hotspot.setRectangle",
                json!({"rect": {"left": 0.0, "top": 0.0, "right": 5.0, "bottom": 5.0}}),
            )
            .unwrap_err();
        assert!(err.to_string().contains("not a slice or hotspot"));
    }

    #[test]
    fn copy_to_hotspot_gates_before_mutating() {
        let (mut document, _ids) = testdoc::sample_document();
        let registry = registry();

        // No selection.
        let err = registry
            .execute(
                &mut document,
                "hotspot.copyToHotspot",
                json!({"page": 0, "hotspot_type": "hotspot", "what_if_multiple": "single"}),
            )
            .unwrap_err();
        assert!(err.to_string().contains("nothing is selected"));

        // A selection with no drawable bounds: a path with no
        // contours contributes nothing to content_bounds.
        let orphan = document.registry.insert(path_object_like());
        document.object_selection = vec![orphan];
        let before = serde_json::to_vec(&document).unwrap();
        let err = registry
            .execute(
                &mut document,
                "hotspot.copyToHotspot",
                json!({
                    "page": 0, "hotspot_type": "hotspot",
                    "what_if_multiple": "multiple", "make_rectangular": false,
                }),
            )
            .unwrap_err();
        assert!(err.to_string().contains("carries no bounds"));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "the gated copies mutated nothing"
        );
    }

    /// A path object with no contours — contributes no bounds.
    fn path_object_like() -> ObjectData {
        ObjectData::Path(doc::PathObject {
            contours: Vec::new(),
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

    #[test]
    fn set_shape_on_an_empty_selection_is_a_noop_refusal() {
        let (mut document, _ids) = testdoc::sample_document();
        let err = registry()
            .execute(
                &mut document,
                "hotspot.setShape",
                json!({"what": "hotspots", "shape": "oval"}),
            )
            .unwrap_err();
        assert!(err.to_string().contains("nothing is selected"));

        // Same shape twice: the second pass is a Noop delta, not an
        // error.
        let id = make_polygon_hotspot(&mut document);
        document.object_selection = vec![id];
        let delta = registry()
            .execute(
                &mut document,
                "hotspot.setShape",
                json!({"what": "hotspots", "shape": "polyline"}),
            )
            .unwrap();
        assert_eq!(slice_at(&document, id).shape, 2);
        let again = registry()
            .execute(
                &mut document,
                "hotspot.setShape",
                json!({"what": "hotspots", "shape": "polyline"}),
            )
            .unwrap();
        assert!(matches!(again, crate::delta::Delta::Noop));
        delta.apply_inverse(&mut document).unwrap();
    }
}
