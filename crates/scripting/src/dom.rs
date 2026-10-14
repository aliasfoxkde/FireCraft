//! Read-side projection of [`doc::Document`] into the JS host
//! objects the shipped scripts read: `fw.documents`,
//! `fw.getDocumentDOM()`, `fw.selection`, and the page/frame/layer/
//! element walk — in the corpus's shape (`dom.layers[i].frames[f].
//! elements[]`, settable `currentLayerNum`, `isMasterPageLayer`).
//!
//! The projection is a snapshot: the host re-attaches after mutations
//! ([`crate::run_in_session`] does), so reads always see fresh state
//! without re-entrant accessors. The object selection (`fw.selection`,
//! decision 0024) is carried by the model
//! ([`Document::object_selection`]) as command truth; the projection
//! reads the host-side session list, seeding it from the model when
//! the session has none — so a selection set through commands before
//! the run is visible to the script. `fw.selection = [...]`
//! assignments remain plain JS view state (the `selection.*` commands
//! are the durable, undoable path).

use boa_engine::{
    Context, JsObject, JsResult, JsValue, Source, js_string, object::builtins::JsArray,
    property::Attribute,
};
use doc::{Document, ObjectData, ObjectId};
use geom::Point;

use crate::host;

/// What the projection needs from the host state beyond the document:
/// the persistent selection (object ids as `"index:generation"`;
/// seeded from the model's when empty) and the current page.
pub struct AttachView {
    /// Object ids (`"index:generation"`) selected across the session;
    /// dangling entries are dropped. Empty means "seed from
    /// `document.object_selection`" (decision 0024).
    pub selection: Vec<String>,
    /// Page index to project as current (out of range falls back to
    /// the first page).
    pub current_page: usize,
    /// Canvas view flags — pure session state (decision 0035): the
    /// wire has no node for them and CS6 keeps them in app
    /// preferences, not in the document.
    pub show_grid: bool,
    pub show_guides: bool,
    pub snap_to_grid: bool,
    pub snap_to_guides: bool,
}

/// Projects `document` and installs the read + mutate surface:
/// `fw.documents` (one-element array), `fw.getDocumentDOM()`,
/// `fw.selection`, `dom` (the projection, carrying the mutating
/// methods from [`host::DOM_MUTATORS`]). Called by
/// [`crate::ScriptEngine::attach_document`].
pub fn attach(context: &mut Context, document: &Document, view: &AttachView) -> JsResult<()> {
    let dom = project_document(context, document, view)?;
    context.register_global_property(js_string!("__fwDom"), dom, Attribute::all())?;
    // `dom` names the projection (replacing the prelude's refusing
    // stub), and `fw.selection` projects from the host-side selection.
    context.eval(Source::from_bytes(
        "fw.documents = [__fwDom];\nfw.getDocumentDOM = function () { return __fwDom; };\nglobalThis.dom = __fwDom;\n",
    ))?;
    context.eval(Source::from_bytes(host::DOM_MUTATORS))?;
    project_selection(context, document, view)?;
    Ok(())
}

/// Projects `fw.selection` from the host-side selection list (each
/// entry `"index:generation"`; dangling ids are dropped silently —
/// deleted objects stop being selected). An empty session list seeds
/// from the model's `object_selection` (decision 0024): a selection
/// set through `selection.*` commands before the run is visible.
fn project_selection(
    context: &mut Context,
    document: &Document,
    view: &AttachView,
) -> JsResult<()> {
    let selection = JsArray::new(context);
    let ids: Vec<ObjectId> = if view.selection.is_empty() {
        document.object_selection.clone()
    } else {
        view.selection
            .iter()
            .filter_map(|key| parse_id(key))
            .collect()
    };
    for id in ids {
        let Some(data) = document.registry.get(id) else {
            continue;
        };
        selection.push(
            JsValue::from(project_element(context, document, id, data)?),
            context,
        )?;
    }
    context.register_global_property(js_string!("__fwSelArray"), selection, Attribute::all())?;
    context.eval(Source::from_bytes("fw.selection = __fwSelArray;\n"))?;
    Ok(())
}

/// `"index:generation"` → [`ObjectId`].
fn parse_id(key: &str) -> Option<ObjectId> {
    let (index, generation) = key.split_once(':')?;
    Some(ObjectId::from_parts(
        index.parse().ok()?,
        generation.parse().ok()?,
    ))
}

/// The projected DOM object (snapshot values; see module docs).
fn project_document(
    context: &mut Context,
    document: &Document,
    view: &AttachView,
) -> JsResult<JsObject> {
    let dom = JsObject::with_object_proto(context.intrinsics());
    let page_index = if view.current_page < document.pages.len() {
        view.current_page
    } else {
        0
    };
    let page = document.pages.get(page_index);
    let canvas = &document.info.canvas;
    let frame_count = page
        .map(|p| p.info.frame_count)
        .unwrap_or(document.info.frame_count);

    set_number(context, &dom, "width", canvas.width)?;
    set_number(context, &dom, "height", canvas.height)?;
    set_number(context, &dom, "left", canvas.x)?;
    set_number(context, &dom, "top", canvas.y)?;
    set_number(context, &dom, "resolution", canvas.resolution)?;
    set_number(
        context,
        &dom,
        "resolutionUnits",
        canvas.resolution_unit as f64,
    )?;
    set_number(context, &dom, "pagesCount", document.pages.len() as f64)?;
    set_number(context, &dom, "currentPageNum", page_index as f64)?;
    set_number(context, &dom, "currentFrameNum", 0.0)?;
    // View flags (session state — decision 0035): the `setShowGrid`
    // family writes back here so reads in the same run agree.
    set_bool(context, &dom, "showGrid", view.show_grid)?;
    set_bool(context, &dom, "showGuides", view.show_guides)?;
    set_bool(context, &dom, "snapToGrid", view.snap_to_grid)?;
    set_bool(context, &dom, "snapToGuides", view.snap_to_guides)?;

    // The master page (first flagged `is_master`), or -1 when none —
    // `isMasterPageLayer` reads it.
    let master = document
        .pages
        .iter()
        .position(|page| page.is_master)
        .map_or(-1.0, |index| index as f64);
    set_number(context, &dom, "masterPageNum", master)?;

    let frames = JsArray::new(context);
    for _ in 0..frame_count.max(0) {
        let frame = JsObject::with_object_proto(context.intrinsics());
        frames.push(JsValue::from(frame), context)?;
    }
    dom.set(js_string!("frames"), JsValue::from(frames), false, context)?;

    let layers = JsArray::new(context);
    if let Some(page) = page {
        for layer in &page.layers.layers {
            layers.push(
                JsValue::from(project_layer(context, document, layer, frame_count)?),
                context,
            )?;
        }
    }
    // The active layer defaults to the top of the stack (where
    // Fireworks puts new elements); scripts assign `currentLayerNum`.
    let top = layers.length(context)?;
    dom.set(
        js_string!("currentLayerNum"),
        JsValue::from(top.saturating_sub(1) as f64),
        false,
        context,
    )?;
    dom.set(js_string!("layers"), JsValue::from(layers), false, context)?;
    Ok(dom)
}

/// One projected layer: `name`, `display`, `frames[]` (per-state
/// `elements`/`locked`/`visible`, the corpus's access shape) and the
/// state-0 `elements` convenience.
fn project_layer(
    context: &mut Context,
    document: &Document,
    layer: &doc::Layer,
    frame_count: i64,
) -> JsResult<JsObject> {
    let object = JsObject::with_object_proto(context.intrinsics());
    object.set(
        js_string!("name"),
        JsValue::from(js_string!(layer.name.as_str())),
        false,
        context,
    )?;
    object.set(
        js_string!("display"),
        JsValue::from(layer.display),
        false,
        context,
    )?;

    // Per-state frames; negative frame counts collapse to the first
    // cell, absurd ones to the last.
    let cell_index = |state: i64| -> usize {
        if state <= 0 {
            0
        } else {
            (state as usize).min(layer.cells.len().saturating_sub(1))
        }
    };
    let frames_array = JsArray::new(context);
    let mut first_elements: Option<JsArray> = None;
    for state in 0..frame_count.max(0) {
        let cell = layer.cells.get(cell_index(state));
        let frame = JsObject::with_object_proto(context.intrinsics());
        frame.set(
            js_string!("locked"),
            JsValue::from(cell.map(|c| c.locked).unwrap_or(false)),
            false,
            context,
        )?;
        frame.set(
            js_string!("visible"),
            JsValue::from(cell.map(|c| c.visible).unwrap_or(true)),
            false,
            context,
        )?;
        let elements = JsArray::new(context);
        for id in cell.map(|c| c.elements.as_slice()).unwrap_or(&[]) {
            let Some(data) = document.registry.get(*id) else {
                continue; // dangling id — refuse to project, never guess
            };
            elements.push(
                JsValue::from(project_element(context, document, *id, data)?),
                context,
            )?;
        }
        if first_elements.is_none() {
            first_elements = Some(elements.clone());
        }
        frame.set(
            js_string!("elements"),
            JsValue::from(elements),
            false,
            context,
        )?;
        frames_array.push(JsValue::from(frame), context)?;
    }
    object.set(
        js_string!("frames"),
        JsValue::from(frames_array),
        false,
        context,
    )?;
    // Convenience: state 0's elements directly on the layer.
    if let Some(elements) = first_elements {
        object.set(
            js_string!("elements"),
            JsValue::from(elements),
            false,
            context,
        )?;
    }
    Ok(object)
}

/// One projected element: `name`, bounds (`left/top/width/height`),
/// and `__fwId` (`"index:generation"` — the handle mutating methods
/// put into command payloads).
fn project_element(
    context: &mut Context,
    document: &Document,
    id: ObjectId,
    data: &doc::ObjectData,
) -> JsResult<JsObject> {
    let object = JsObject::with_object_proto(context.intrinsics());
    let (name, bounds) = match data {
        ObjectData::Path(path) => (path.name.clone(), path_bounds(path)),
        ObjectData::Image(image) => (
            image.name.clone(),
            (
                image.location.x,
                image.location.y,
                image.pixels.0 as f64,
                image.pixels.1 as f64,
            ),
        ),
        ObjectData::Text(text) => (
            None,
            (
                text.frame.x0,
                text.frame.y0,
                (text.frame.x1 - text.frame.x0).max(0.0),
                (text.frame.y1 - text.frame.y0).max(0.0),
            ),
        ),
        ObjectData::Slice(slice) => (
            slice.name.clone(),
            (
                slice.frame.x0,
                slice.frame.y0,
                (slice.frame.x1 - slice.frame.x0).max(0.0),
                (slice.frame.y1 - slice.frame.y0).max(0.0),
            ),
        ),
        ObjectData::Group(group) => {
            let mut bounds = (0.0, 0.0, 0.0, 0.0);
            let mut first = true;
            for child in &group.children {
                let Some(child_data) = document.registry.get(*child) else {
                    continue;
                };
                let child_bounds = group_child_bounds(document, child_data, 0);
                if first {
                    bounds = child_bounds;
                    first = false;
                } else {
                    bounds = union(bounds, child_bounds);
                }
            }
            (group.name.clone(), bounds)
        }
        ObjectData::SymbolInstance(instance) => {
            let coeffs = instance.transform.as_coeffs();
            (instance.name.clone(), (coeffs[4], coeffs[5], 0.0, 0.0))
        }
    };
    let name = match name {
        Some(name) => JsValue::from(js_string!(name.as_str())),
        None => JsValue::null(),
    };
    object.set(js_string!("name"), name, false, context)?;
    set_number(context, &object, "left", bounds.0)?;
    set_number(context, &object, "top", bounds.1)?;
    set_number(context, &object, "width", bounds.2)?;
    set_number(context, &object, "height", bounds.3)?;
    // `__fwId` is the payload form of the object id — an object with
    // `index`/`generation`, exactly what command payloads carry, so
    // the mutating methods can embed it verbatim.
    let fw_id = JsObject::with_object_proto(context.intrinsics());
    fw_id.set(
        js_string!("index"),
        JsValue::from(id.index() as f64),
        false,
        context,
    )?;
    fw_id.set(
        js_string!("generation"),
        JsValue::from(f64::from(id.generation())),
        false,
        context,
    )?;
    object.set(js_string!("__fwId"), JsValue::from(fw_id), false, context)?;
    Ok(object)
}

/// Recursion cap for group-child bounds (corrupt deep nesting must not
/// blow the native stack).
const GROUP_DEPTH_CAP: u32 = 32;

/// Bounds of a group child (recursing through nested groups up to the
/// depth cap).
fn group_child_bounds(document: &Document, data: &ObjectData, depth: u32) -> (f64, f64, f64, f64) {
    match data {
        ObjectData::Path(path) => path_bounds(path),
        ObjectData::Image(image) => (
            image.location.x,
            image.location.y,
            image.pixels.0 as f64,
            image.pixels.1 as f64,
        ),
        ObjectData::Text(text) => (
            text.frame.x0,
            text.frame.y0,
            (text.frame.x1 - text.frame.x0).max(0.0),
            (text.frame.y1 - text.frame.y0).max(0.0),
        ),
        ObjectData::Slice(slice) => (
            slice.frame.x0,
            slice.frame.y0,
            (slice.frame.x1 - slice.frame.x0).max(0.0),
            (slice.frame.y1 - slice.frame.y0).max(0.0),
        ),
        ObjectData::Group(group) if depth < GROUP_DEPTH_CAP => {
            let mut bounds = (0.0, 0.0, 0.0, 0.0);
            let mut first = true;
            for child in &group.children {
                let Some(child_data) = document.registry.get(*child) else {
                    continue;
                };
                let child_bounds = group_child_bounds(document, child_data, depth + 1);
                if first {
                    bounds = child_bounds;
                    first = false;
                } else {
                    bounds = union(bounds, child_bounds);
                }
            }
            bounds
        }
        _ => (0.0, 0.0, 0.0, 0.0),
    }
}

/// Anchor-point bounds of a path (`(left, top, width, height)`; the
/// editable geometry, not the primitive source). Empty contours give
/// zeros.
pub fn path_bounds(path: &doc::PathObject) -> (f64, f64, f64, f64) {
    let mut min = Point::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut any = false;
    for contour in &path.contours {
        for point in &contour.points {
            min.x = min.x.min(point.anchor.x);
            min.y = min.y.min(point.anchor.y);
            max.x = max.x.max(point.anchor.x);
            max.y = max.y.max(point.anchor.y);
            any = true;
        }
    }
    if !any {
        return (0.0, 0.0, 0.0, 0.0);
    }
    (
        min.x,
        min.y,
        (max.x - min.x).max(0.0),
        (max.y - min.y).max(0.0),
    )
}

/// Component-wise union of two `(left, top, width, height)` bounds.
fn union(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
    let x0 = a.0.min(b.0);
    let y0 = a.1.min(b.1);
    let x1 = (a.0 + a.2).max(b.0 + b.2);
    let y1 = (a.1 + a.3).max(b.1 + b.3);
    (x0, y0, x1 - x0, y1 - y0)
}

/// Sets a plain number property on `object`.
fn set_number(
    context: &mut Context,
    object: &JsObject,
    property: &str,
    value: f64,
) -> JsResult<()> {
    object
        .set(js_string!(property), JsValue::from(value), false, context)
        .map(|_replaced| ())
}

fn set_bool(context: &mut Context, object: &JsObject, property: &str, value: bool) -> JsResult<()> {
    object
        .set(js_string!(property), JsValue::from(value), false, context)
        .map(|_replaced| ())
}
