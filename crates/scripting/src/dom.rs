//! Read-side projection of [`doc::Document`] into the JS host
//! objects the shipped scripts read: `fw.documents`,
//! `fw.getDocumentDOM()`, `fw.selection`, and the page/frame/layer/
//! element walk (`dom.width/height/left/top/resolution`,
//! `dom.pagesCount`, `dom.currentPageNum`, `dom.frames[]`,
//! `dom.layers[].elements[]` with `left/top/width/height/name`).
//!
//! The projection is a snapshot: the host re-attaches after any
//! mutation (once command dispatch lands), so reads always see fresh
//! state without re-entrant accessors. Element selection is UI state
//! the model does not carry (its `selection` field is a pixel-space
//! mask, not the corpus's element list), so `fw.selection` projects
//! as an empty read-only array until element selection exists.

use boa_engine::{
    Context, JsObject, JsResult, JsValue, Source, js_string, object::builtins::JsArray,
    property::Attribute,
};
use doc::{Document, ObjectData};
use geom::Point;

/// Projects `document` and installs the read surface: `fw.documents`
/// (one-element array), `fw.getDocumentDOM()` (native, returns the
/// projected DOM), `fw.selection` (empty). Called by
/// [`ScriptEngine::attach_document`].
pub fn attach(context: &mut Context, document: &Document) -> JsResult<()> {
    let dom = project_document(context, document)?;
    context.register_global_property(js_string!("__fwDom"), dom, Attribute::all())?;
    // The accessor half: `fw.documents` wraps the one DOM,
    // `getDocumentDOM()` returns it, `fw.selection` stays empty until
    // element selection exists.
    context.eval(Source::from_bytes(
        "fw.documents = [__fwDom];\nfw.getDocumentDOM = function () { return __fwDom; };\nfw.selection = [];\n",
    ))?;
    Ok(())
}

/// The projected DOM object (snapshot values; see module docs).
fn project_document(context: &mut Context, document: &Document) -> JsResult<JsObject> {
    let dom = JsObject::with_object_proto(context.intrinsics());
    let page = document.pages.first();
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
    set_number(context, &dom, "currentPageNum", 0.0)?;
    set_number(context, &dom, "currentFrameNum", 0.0)?;

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
    dom.set(js_string!("layers"), JsValue::from(layers), false, context)?;
    Ok(dom)
}

/// One projected layer: `name`, `display`, `elements[]` (the cell of
/// the current frame).
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

    // The cell of the current frame (0 for this snapshot); negative
    // frame counts collapse to the first cell, absurd ones to the last.
    let cell_index = if frame_count <= 0 {
        0
    } else {
        (frame_count as usize).min(layer.cells.len().saturating_sub(1))
    };
    let cell = layer.cells.get(cell_index);
    let elements = JsArray::new(context);
    for id in cell.map(|c| c.elements.as_slice()).unwrap_or(&[]) {
        let Some(data) = document.registry.get(*id) else {
            continue; // dangling id — refuse to project, never guess
        };
        elements.push(
            JsValue::from(project_element(context, document, data)?),
            context,
        )?;
    }
    object.set(
        js_string!("elements"),
        JsValue::from(elements),
        false,
        context,
    )?;
    Ok(object)
}

/// One projected element: `name` plus bounds (`left/top/width/height`).
fn project_element(
    context: &mut Context,
    document: &Document,
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
