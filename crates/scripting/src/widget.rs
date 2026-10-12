//! The `Widget` global: the rich-symbol re-derivation surface.
//!
//! The CS6 model, read off the shipped `Configuration/HTML Code/
//! Widget.jsf` and the Common Library companion scripts (`First Run/
//! Common Library/*/Rich Symbol-*.jsf`): a symbol definition carries a
//! script; when an instance is placed or its parameters change, the
//! host runs that script with `Widget.opCode` 1 (write defaults) or 2
//! (apply current values). The script reads the parameter array at
//! `Widget.elem.customData["currentValues"]` (`{name, type, value}`
//! triples) and mutates definition members it looks up by name —
//! `Widget.GetObjectByName("label").textChars = …`. `Widget.propString`
//! is the same array in the Component panel's `&;&`/`&|&` encoding.
//!
//! Mutations queue through the host's intent queue (decision 0011)
//! and drain through the registry, so a re-derivation journals,
//! undoes, and replays exactly like any other command chain. Only
//! the property shapes the corpus scripts use are projected: path
//! `pathAttributes.fillColor`/`brushColor`, text `textChars`/`font`.

use boa_engine::{Context, JsObject, JsValue, Source, js_string};
use doc::{Document, ObjectData, ObjectId, SymbolDefinition};

/// Member kinds the projection carries, with their read-side values.
enum Member {
    /// `pathAttributes.fillColor` / `.brushColor` (raw ABGR; `None`
    /// reads as transparent black, matching `object.setFill`).
    Path {
        fill_raw: Option<i64>,
        stroke_raw: Option<i64>,
    },
    /// `textChars` (the runs concatenated) and `font` (the object
    /// style's family).
    Text { chars: String, font: Option<String> },
}

/// The JS half: color helpers, the element builders (accessor
/// properties queue the mutation), the customData Proxy, and the
/// `Widget` assembly. Runs once per projection, after the Rust half
/// set `__fwWidget*` inputs.
const WIDGET_GLUE: &str = r##"
// CSS colour <-> raw ABGR (the path paint layout): #RRGGBB / #RGB in,
// `(255 << 24) | (b << 16) | (g << 8) | r` out; raw back to #RRGGBB.
globalThis.__fwColor = function (css) {
    var text = String(css).trim();
    var hex = text.charAt(0) === "#" ? text.slice(1) : text;
    var r, g, b;
    if (hex.length === 3) {
        r = parseInt(hex.charAt(0) + hex.charAt(0), 16);
        g = parseInt(hex.charAt(1) + hex.charAt(1), 16);
        b = parseInt(hex.charAt(2) + hex.charAt(2), 16);
    } else if (hex.length === 6) {
        r = parseInt(hex.slice(0, 2), 16);
        g = parseInt(hex.slice(2, 4), 16);
        b = parseInt(hex.slice(4, 6), 16);
    } else {
        throw new Error("Widget: unsupported colour " + text);
    }
    if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) {
        throw new Error("Widget: unsupported colour " + text);
    }
    // No `| 0`: the raw layout is UNSIGNED 32-bit (i64 on the wire),
    // and int32 coercion would flip the alpha bit negative.
    return 4278190080 + (b << 16) + (g << 8) + r;
};
globalThis.__fwCss = function (raw) {
    var v = Number(raw) & 0xFFFFFF;
    var r = v & 0xFF, g = (v >> 8) & 0xFF, b = (v >> 16) & 0xFF;
    var hex = function (n) { var s = n.toString(16); return s.length < 2 ? "0" + s : s; };
    return "#" + hex(r) + hex(g) + hex(b);
};

// The customData store: reads JSON-decode (entries are strings on the
// document; the corpus's array-of-triples marshals through JSON),
// writes queue `symbol.setCustomData` — strings verbatim, everything
// else JSON-encoded.
globalThis.__fwCustomData = function (instanceId, entries) {
    var target = {};
    for (var key in entries) {
        var value;
        try { value = JSON.parse(entries[key]); } catch (error) { value = entries[key]; }
        target[key] = value;
    }
    return new Proxy(target, {
        set: function (t, key, value) {
            if (typeof key === "string") {
                __fwQueue("symbol.setCustomData", JSON.stringify({
                    id: instanceId,
                    key: key,
                    value: typeof value === "string" ? value : JSON.stringify(value),
                }));
            }
            t[key] = value;
            return true;
        },
    });
};

// One projected element: plain snapshot data plus the settable
// properties the corpus scripts assign. Snapshot reads stay stale
// until the next attach — the same read model as `dom`.
globalThis.__fwWidgetElement = function (raw) {
    if (raw.__kind === "path") {
        raw.pathAttributes = {
            get fillColor() { return __fwCss(raw.__fillRaw); },
            set fillColor(v) {
                __fwQueue("object.setFill", JSON.stringify({
                    id: raw.__fwId, color_raw: __fwColor(v),
                }));
            },
            get brushColor() { return __fwCss(raw.__strokeRaw); },
            set brushColor(v) {
                __fwQueue("object.setStroke", JSON.stringify({
                    id: raw.__fwId, color_raw: __fwColor(v),
                }));
            },
        };
    } else if (raw.__kind === "text") {
        raw.__chars = String(raw.__chars);
        Object.defineProperty(raw, "textChars", {
            get: function () { return raw.__chars; },
            set: function (v) {
                raw.__chars = String(v);
                __fwQueue("text.setText", JSON.stringify({
                    id: raw.__fwId, text: raw.__chars,
                }));
            },
            configurable: true,
        });
        raw.__font = raw.__font === null ? "" : String(raw.__font);
        Object.defineProperty(raw, "font", {
            get: function () { return raw.__font; },
            set: function (v) {
                raw.__font = String(v);
                __fwQueue("text.setFont", JSON.stringify({
                    id: raw.__fwId, font: raw.__font,
                }));
            },
            configurable: true,
        });
    }
    return raw;
};

// The `Widget` global itself.
globalThis.Widget = (function () {
    var elem = __fwWidgetElement(__fwWidgetElemRaw);
    elem.customData = __fwCustomData(__fwWidgetInstanceId, __fwWidgetData);
    var lookup = {};
    for (var name in __fwWidgetMemberRaws) {
        lookup[name] = __fwWidgetElement(__fwWidgetMemberRaws[name]);
    }
    var widget = {
        opCode: __fwWidgetOpCode,
        elem: elem,
        GetObjectByName: function (wanted) {
            var found = lookup[String(wanted)];
            if (!found) {
                throw new Error(
                    "Widget.GetObjectByName: no object named " + wanted +
                    " in the symbol definition"
                );
            }
            return found;
        },
    };
    // The Component panel's encoding: `name&|&value&|&type` triples
    // joined by `&;&` (ReadToWidgetProp.jsf's grammar, verbatim).
    Object.defineProperty(widget, "propString", {
        get: function () {
            var values = elem.customData["currentValues"];
            if (!(values instanceof Array)) { return ""; }
            var parts = [];
            for (var i = 0; i < values.length; i++) {
                parts.push(
                    values[i].name + "&|&" + values[i].value + "&|&" + values[i].type
                );
            }
            return parts.join("&;&");
        },
        set: function (encoded) {
            var values = [];
            if (encoded !== undefined && encoded !== null && encoded !== "") {
                var objects = String(encoded).split("&;&");
                for (var i = 0; i < objects.length; i++) {
                    var fields = objects[i].split("&|&");
                    if (fields.length >= 3) {
                        values.push({ name: fields[0], value: fields[1], type: fields[2] });
                    }
                }
            }
            elem.customData["currentValues"] = values;
        },
        configurable: true,
    });
    return widget;
})();
"##;

/// Installs the `Widget` global bound to `instance` with `op_code`
/// (1 = defaults, 2 = apply), and returns the definition's script
/// source for the caller to run.
///
/// # Errors
/// A string naming the refusal: a non-instance target, a dangling
/// `MSNi`, a scriptless definition, or an out-of-contract op code.
pub(crate) fn project(
    context: &mut Context,
    document: &Document,
    instance: ObjectId,
    op_code: u8,
) -> Result<String, String> {
    if !matches!(op_code, 1 | 2) {
        return Err(format!(
            "Widget.opCode {op_code} is out of contract (1 or 2)"
        ));
    }
    let Some(ObjectData::SymbolInstance(data)) = document.registry.get(instance) else {
        return Err(format!(
            "object {} is not a symbol instance",
            instance.index()
        ));
    };
    let definition: &SymbolDefinition = document
        .symbols
        .iter()
        .find(|definition| definition.symbol_number == data.symbol_number)
        .ok_or_else(|| format!("no symbol numbered {}", data.symbol_number))?;
    let source = definition.op_code.clone().ok_or_else(|| {
        format!(
            "symbol {} carries no rich-symbol script",
            data.symbol_number
        )
    })?;

    let instance_id = JsObject::with_object_proto(context.intrinsics());
    instance_id
        .set(
            js_string!("index"),
            JsValue::from(instance.index() as f64),
            false,
            context,
        )
        .map_err(|error| error.to_string())?;
    instance_id
        .set(
            js_string!("generation"),
            JsValue::from(f64::from(instance.generation())),
            false,
            context,
        )
        .map_err(|error| error.to_string())?;

    // `Widget.elem`: the instance's read snapshot plus the live
    // customData store.
    let elem = JsObject::with_object_proto(context.intrinsics());
    set_name(context, &elem, data.name.as_deref())?;
    set_id(context, &elem, instance)?;
    elem.set(js_string!("__kind"), js_string!("instance"), false, context)
        .map_err(|error| error.to_string())?;

    let custom = JsObject::with_object_proto(context.intrinsics());
    for (key, value) in &data.custom_data {
        custom
            .set(
                js_string!(key.as_str()),
                js_string!(value.as_str()),
                false,
                context,
            )
            .map_err(|error| error.to_string())?;
    }

    // `Widget.GetObjectByName`: the definition's named members.
    let mut members = Vec::new();
    for layer in &definition.layers.layers {
        for cell in &layer.cells {
            collect_members(document, &cell.elements, 0, &mut members);
        }
    }
    let member_roots = JsObject::with_object_proto(context.intrinsics());
    for (name, id, member) in &members {
        let raw = JsObject::with_object_proto(context.intrinsics());
        set_name(context, &raw, Some(name))?;
        set_id(context, &raw, *id)?;
        match member {
            Member::Path {
                fill_raw,
                stroke_raw,
            } => {
                raw.set(js_string!("__kind"), js_string!("path"), false, context)
                    .map_err(|error| error.to_string())?;
                raw.set(
                    js_string!("__fillRaw"),
                    JsValue::from(fill_raw.unwrap_or(0) as f64),
                    false,
                    context,
                )
                .map_err(|error| error.to_string())?;
                raw.set(
                    js_string!("__strokeRaw"),
                    JsValue::from(stroke_raw.unwrap_or(0) as f64),
                    false,
                    context,
                )
                .map_err(|error| error.to_string())?;
            }
            Member::Text { chars, font } => {
                raw.set(js_string!("__kind"), js_string!("text"), false, context)
                    .map_err(|error| error.to_string())?;
                raw.set(
                    js_string!("__chars"),
                    js_string!(chars.as_str()),
                    false,
                    context,
                )
                .map_err(|error| error.to_string())?;
                let font = match font {
                    Some(font) => JsValue::from(js_string!(font.as_str())),
                    None => JsValue::null(),
                };
                raw.set(js_string!("__font"), font, false, context)
                    .map_err(|error| error.to_string())?;
            }
        }
        member_roots
            .set(js_string!(name.as_str()), raw, false, context)
            .map_err(|error| error.to_string())?;
    }

    for (key, value) in [
        ("__fwWidgetOpCode", JsValue::from(f64::from(op_code))),
        ("__fwWidgetInstanceId", instance_id.into()),
        ("__fwWidgetElemRaw", elem.into()),
        ("__fwWidgetData", custom.into()),
        ("__fwWidgetMemberRaws", member_roots.into()),
    ] {
        context
            .register_global_property(
                js_string!(key),
                value,
                boa_engine::property::Attribute::all(),
            )
            .map_err(|error| error.to_string())?;
    }
    context
        .eval(Source::from_bytes(WIDGET_GLUE))
        .map_err(|error| format!("Widget glue failed: {error}"))?;
    Ok(source)
}

/// Collects the definition's named members depth-first. Unnamed
/// objects are invisible to `GetObjectByName` (that is its contract);
/// groups recurse under the same depth ceiling compose uses.
fn collect_members(
    document: &Document,
    elements: &[ObjectId],
    depth: u32,
    out: &mut Vec<(String, ObjectId, Member)>,
) {
    if depth > 32 {
        return;
    }
    for id in elements {
        let Some(data) = document.registry.get(*id) else {
            continue;
        };
        match data {
            ObjectData::Path(path) => {
                if let Some(name) = &path.name {
                    let (fill_raw, stroke_raw) =
                        path.paint.as_ref().map_or((None, None), |paint| {
                            (Some(paint.fill_raw), Some(paint.stroke_raw))
                        });
                    out.push((
                        name.clone(),
                        *id,
                        Member::Path {
                            fill_raw,
                            stroke_raw,
                        },
                    ));
                }
            }
            ObjectData::Text(text) => {
                if let Some(name) = &text.name {
                    let chars: String = text.runs.iter().map(|run| run.text.as_str()).collect();
                    let font = text.style.as_ref().and_then(|style| style.font.clone());
                    out.push((name.clone(), *id, Member::Text { chars, font }));
                }
            }
            ObjectData::Group(group) => {
                collect_members(document, &group.children, depth + 1, out);
            }
            _ => {}
        }
    }
}

/// Sets `name` (string or null) on a projected element.
fn set_name(context: &mut Context, object: &JsObject, name: Option<&str>) -> Result<(), String> {
    let value = match name {
        Some(name) => JsValue::from(js_string!(name)),
        None => JsValue::null(),
    };
    object
        .set(js_string!("name"), value, false, context)
        .map(|_replaced| ())
        .map_err(|error| error.to_string())
}

/// Sets `__fwId` — the payload form of the object id, exactly what
/// command payloads carry (`{index, generation}`).
fn set_id(context: &mut Context, object: &JsObject, id: ObjectId) -> Result<(), String> {
    let fw_id = JsObject::with_object_proto(context.intrinsics());
    fw_id
        .set(
            js_string!("index"),
            JsValue::from(id.index() as f64),
            false,
            context,
        )
        .map_err(|error| error.to_string())?;
    fw_id
        .set(
            js_string!("generation"),
            JsValue::from(f64::from(id.generation())),
            false,
            context,
        )
        .map_err(|error| error.to_string())?;
    object
        .set(js_string!("__fwId"), fw_id, false, context)
        .map(|_replaced| ())
        .map_err(|error| error.to_string())
}
