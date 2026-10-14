//! Live-effect list commands: the filter stack on paths, text,
//! groups, and images.
//!
//! The list itself is the model's `effects: Vec<Effect>` (the `EFLv`
//! spine); these commands edit it with whole-object capture, so undo
//! restores the exact list including any raw (`GPLv`/`GDTv`) file
//! residue the reader preserved. Parameters are string-valued
//! (`DCVs`); the one exception is curve knot lists, which CS6 files
//! carry as raw `GPLv` grab-point nodes and the CS6 scripts author as
//! JS arrays — the JSON `points` param is the equivalent, encoded to
//! the same node shape on apply (and mirrored by compose's curves
//! interpreter on the way back out).

use doc::{Document, Effect, EffectProperty, EffectValue, Node, NodeValue, ObjectData, ObjectId};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the filter commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(AddFilter)?;
    registry.register(RemoveFilter)?;
    registry.register(SetFilterParam)?;
    registry.register(SetEffectName)
}

/// The `GPLv` grab-point list for one `*_points` property: a `GPTv`
/// child per knot with `XLCf`/`YLCf` float coordinates — the shape
/// the reader preserves from files and compose's curves interpreter
/// reads back.
fn grab_list(points: &[[f64; 2]]) -> Node {
    Node {
        fourcc: *b"GPLv",
        value: NodeValue::Children(
            points
                .iter()
                .map(|[x, y]| Node {
                    fourcc: *b"GPTv",
                    value: NodeValue::Children(vec![
                        Node {
                            fourcc: *b"XLCf",
                            value: NodeValue::Float(*x),
                        },
                        Node {
                            fourcc: *b"YLCf",
                            value: NodeValue::Float(*y),
                        },
                    ]),
                })
                .collect(),
        ),
    }
}

/// Validates one parameter's encoding (text xor knot list, knots
/// non-empty and finite) and yields its `DCEv` value.
pub(crate) fn param_value(
    name: &'static str,
    key: &str,
    value: &str,
    points: &Option<Vec<[f64; 2]>>,
) -> Result<EffectValue, EngineError> {
    if let Some(knots) = points {
        if !value.is_empty() {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("parameter {key} takes a value or points, not both"),
            });
        }
        if knots.is_empty() {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("parameter {key} needs at least one knot"),
            });
        }
        if knots.iter().any(|[x, y]| !x.is_finite() || !y.is_finite()) {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("parameter {key} carries a non-finite knot"),
            });
        }
        return Ok(EffectValue::Raw(grab_list(knots)));
    }
    Ok(EffectValue::Text(value.to_owned()))
}

/// Runs `edit` against the effect list of the object `id` names.
/// `None` when the id is stale or names a kind without an `EFLv` spine
/// (`Slice`, `SymbolInstance`).
pub(crate) fn with_effects<R>(
    document: &mut Document,
    id: ObjectId,
    edit: impl FnOnce(&mut Vec<Effect>) -> R,
) -> Option<R> {
    match document.registry.get_mut(id)? {
        ObjectData::Path(path) => Some(edit(&mut path.effects)),
        ObjectData::Text(text) => Some(edit(&mut text.effects)),
        ObjectData::Group(group) => Some(edit(&mut group.effects)),
        ObjectData::Image(image) => Some(edit(&mut image.effects)),
        _ => None,
    }
}

/// Applies `edit` to the effect list of `id`, capturing the object.
/// `InvalidState` when the id is stale or the kind has no spine.
fn mutate_effects(
    document: &mut Document,
    name: &'static str,
    id: ObjectId,
    edit: impl FnOnce(&mut Vec<Effect>),
) -> Result<Delta, EngineError> {
    let before = access::object(document, name, id)?.clone();
    let applied = with_effects(document, id, edit);
    if applied.is_none() {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("object {} does not carry an effect list", id.index()),
        });
    }
    Ok(Delta::Object {
        id,
        inverse: ObjectInverse::Mutated {
            object: Box::new(before),
        },
    })
}

/// `filter.add` — append (or insert) one effect on an object's stack.
struct AddFilter;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddFilterPayload {
    /// Target object.
    id: ObjectId,
    /// Internal name (`INMs`), e.g. "dropShadow".
    name: String,
    /// UI category (`CATs`); defaults to empty.
    #[serde(default)]
    category: String,
    /// `DCVs` parameters in list order (a map would not preserve it).
    #[serde(default)]
    params: Vec<FilterParamPayload>,
    /// Insert slot; `None` appends. Slot `effects.len()` also appends.
    #[serde(default)]
    index: Option<usize>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FilterParamPayload {
    /// `DCKs` key, e.g. "gaussian_blur_radius".
    key: String,
    /// `DCVs` value; effects carry decimals as decimal strings. Must
    /// stay empty when `points` is present — a parameter takes one
    /// encoding or the other.
    value: String,
    /// Curve knot list (`*_points`), as `[x, y]` pairs. Encoded to the
    /// raw `GPLv`/`GPTv`/`XLCf`/`YLCf` node shape the reader preserves
    /// from files — the JSON equivalent of the CS6 scripts' JS arrays.
    #[serde(default)]
    points: Option<Vec<[f64; 2]>>,
}

/// Builds an `Effect` from a command payload's parts, validating the
/// internal name and every parameter encoding. Shared by the stack
/// commands (`filter.add`) and the bake family (`filter.selection` …)
/// so both speak the exact same effect wire shape.
pub(crate) fn build_effect(
    name: &'static str,
    category: String,
    internal_name: String,
    params: &[FilterParamPayload],
) -> Result<Effect, EngineError> {
    if internal_name.is_empty() {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "an effect needs an internal name".to_owned(),
        });
    }
    if params.iter().any(|param| param.key.is_empty()) {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "parameter keys must be non-empty".to_owned(),
        });
    }
    let mut properties = Vec::with_capacity(params.len());
    for param in params {
        properties.push(EffectProperty {
            key: param.key.clone(),
            value: param_value(name, &param.key, &param.value, &param.points)?,
        });
    }
    Ok(Effect {
        category,
        internal_name,
        properties,
    })
}

impl TypedCommand for AddFilter {
    type Payload = AddFilterPayload;

    fn name(&self) -> &'static str {
        "filter.add"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "filter.add";
        let effect = build_effect(
            NAME,
            payload.category.clone(),
            payload.name.clone(),
            &payload.params,
        )?;
        // Slot check before the mutate: `Vec::insert` cannot be handed
        // an out-of-range slot, and the generic mutate error would
        // misreport it as a missing spine.
        let len = match with_effects(document, payload.id, |effects| effects.len()) {
            Some(len) => len,
            None => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "object {} does not carry an effect list",
                        payload.id.index()
                    ),
                });
            }
        };
        let at = payload.index.unwrap_or(len);
        if at > len {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("insert slot {at} out of range ({len} effects)"),
            });
        }
        mutate_effects(document, NAME, payload.id, |effects| {
            effects.insert(at, effect.clone());
        })
    }
}

/// `filter.remove` — drop the effect at `index`.
struct RemoveFilter;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RemoveFilterPayload {
    /// Target object.
    id: ObjectId,
    /// Effect slot, top of the stack is the last element.
    index: usize,
}

impl TypedCommand for RemoveFilter {
    type Payload = RemoveFilterPayload;

    fn name(&self) -> &'static str {
        "filter.remove"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "filter.remove";
        let effects = access::object(document, NAME, payload.id).ok();
        let in_range = effects.is_some_and(|object| {
            object_effects_len(object).is_some_and(|len| payload.index < len)
        });
        if !in_range {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("effect slot {} out of range", payload.index),
            });
        }
        mutate_effects(document, NAME, payload.id, |effects| {
            effects.remove(payload.index);
        })
    }
}

/// `filter.setParam` — set one `DCVs` parameter on the effect at
/// `index`, replacing the first entry with that key or appending.
struct SetFilterParam;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetFilterParamPayload {
    /// Target object.
    id: ObjectId,
    /// Effect slot.
    index: usize,
    /// `DCKs` key, e.g. "brightness".
    key: String,
    /// `DCVs` value as a decimal string where numeric. Must stay empty
    /// when `points` is present.
    value: String,
    /// Curve knot list (`*_points`), encoded like `filter.add`'s.
    #[serde(default)]
    points: Option<Vec<[f64; 2]>>,
}

impl TypedCommand for SetFilterParam {
    type Payload = SetFilterParamPayload;

    fn name(&self) -> &'static str {
        "filter.setParam"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "filter.setParam";
        if payload.key.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "parameter keys must be non-empty".to_owned(),
            });
        }
        let effects = access::object(document, NAME, payload.id).ok();
        let in_range = effects.is_some_and(|object| {
            object_effects_len(object).is_some_and(|len| payload.index < len)
        });
        if !in_range {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("effect slot {} out of range", payload.index),
            });
        }
        let (key, value) = (payload.key.clone(), payload.value.clone());
        let encoded = param_value(NAME, &key, &value, &payload.points)?;
        mutate_effects(document, NAME, payload.id, |effects| {
            let properties = &mut effects[payload.index].properties;
            match properties.iter_mut().find(|prop| prop.key == key) {
                Some(prop) => prop.value = encoded.clone(),
                None => properties.push(EffectProperty {
                    key,
                    value: encoded.clone(),
                }),
            }
        })
    }
}

/// `filter.setEffectName` — CS6's
/// `dom.setEffectName(category, oldName, newName)` (Fireworks MX):
/// renames every effect on the selected objects whose `CATs` equals
/// `category` and whose `INMs` equals `old_name`. Both names are wire
/// fields, so the rename is a plain slot write; CS6's "current effect"
/// (the Effects-panel selection) carries to the selected objects, the
/// `setFill` walker's rule. Members with no match skip; no match
/// anywhere is the no-op.
struct SetEffectName;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetEffectNamePayload {
    /// `CATs` the effect lives under (exact match).
    category: String,
    /// The `INMs` to rename.
    old_name: String,
    /// The replacement `INMs`.
    new_name: String,
}

impl TypedCommand for SetEffectName {
    type Payload = SetEffectNamePayload;

    fn name(&self) -> &'static str {
        "filter.setEffectName"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "filter.setEffectName";
        if payload.new_name.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "new_name is empty".to_owned(),
            });
        }
        let ids = document.object_selection.clone();
        if ids.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "nothing is selected".to_owned(),
            });
        }
        let matches = |effect: &Effect| {
            effect.category == payload.category && effect.internal_name == payload.old_name
        };
        let mut deltas = Vec::new();
        for id in ids {
            // Stale ids skip (the animation-instance walker's rule);
            // capture only objects that will actually change.
            let Some(before) = document.registry.get(id).cloned() else {
                continue;
            };
            let Some(effects) = object_effects(&before) else {
                continue;
            };
            if !effects.iter().any(matches) {
                continue;
            }
            with_effects(document, id, |effects| {
                for effect in effects.iter_mut().filter(|effect| matches(effect)) {
                    effect.internal_name = payload.new_name.clone();
                }
            });
            deltas.push(Delta::Object {
                id,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(before),
                },
            });
        }
        if deltas.is_empty() {
            return Ok(Delta::Noop);
        }
        Ok(Delta::Group(deltas))
    }
}

/// Effect list of `object`, `None` for kinds without a spine.
fn object_effects(object: &ObjectData) -> Option<&Vec<Effect>> {
    match object {
        ObjectData::Path(path) => Some(&path.effects),
        ObjectData::Text(text) => Some(&text.effects),
        ObjectData::Group(group) => Some(&group.effects),
        ObjectData::Image(image) => Some(&image.effects),
        _ => None,
    }
}

/// Effect-list length of `object`, `None` for kinds without a spine.
fn object_effects_len(object: &ObjectData) -> Option<usize> {
    match object {
        ObjectData::Path(path) => Some(path.effects.len()),
        ObjectData::Text(text) => Some(text.effects.len()),
        ObjectData::Group(group) => Some(group.effects.len()),
        ObjectData::Image(image) => Some(image.effects.len()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::super::testdoc;
    use super::*;
    use crate::command::Command;
    use doc::SliceObject;
    use serde_json::json;

    #[test]
    fn add_setparam_and_undo_chain() {
        let (mut document, ids) = testdoc::sample_document();
        let id = ids[0];
        let delta = AddFilter
            .apply_json(
                &mut document,
                json!({
                    "id": id,
                    "name": "dropShadow",
                    "category": "Shadow and Glow",
                    "params": [
                        { "key": "gaussian_blur_radius", "value": "5" },
                    ],
                }),
            )
            .unwrap();
        let second = AddFilter
            .apply_json(
                &mut document,
                json!({ "id": id, "name": "gaussianBlur", "index": 0 }),
            )
            .unwrap();
        {
            let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
                panic!("wrong kind");
            };
            let names: Vec<&str> = path
                .effects
                .iter()
                .map(|effect| effect.internal_name.as_str())
                .collect();
            assert_eq!(names, ["gaussianBlur", "dropShadow"], "insert slot 0 first");
            assert_eq!(path.effects[1].category, "Shadow and Glow");
            assert_eq!(
                path.effects[1].properties[0].value,
                EffectValue::Text("5".to_owned())
            );
        }

        // setParam replaces an existing key, appends a new one.
        SetFilterParam
            .apply_json(
                &mut document,
                json!({ "id": id, "index": 1, "key": "gaussian_blur_radius", "value": "9" }),
            )
            .unwrap();
        SetFilterParam
            .apply_json(
                &mut document,
                json!({ "id": id, "index": 1, "key": "offset", "value": "3" }),
            )
            .unwrap();
        {
            let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
                panic!("wrong kind");
            };
            let keys: Vec<&str> = path.effects[1]
                .properties
                .iter()
                .map(|p| p.key.as_str())
                .collect();
            assert_eq!(keys, ["gaussian_blur_radius", "offset"]);
            assert_eq!(
                path.effects[1].properties[0].value,
                EffectValue::Text("9".to_owned())
            );
        }

        second.apply_inverse(&mut document).unwrap();
        delta.apply_inverse(&mut document).unwrap();
        let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
            panic!("wrong kind");
        };
        assert!(path.effects.is_empty(), "undo unwinds to the empty stack");
    }

    #[test]
    fn add_rejects_bad_payloads_without_mutation() {
        let (mut document, ids) = testdoc::sample_document();
        let id = ids[0];
        let before = serde_json::to_vec(&document).unwrap();
        for payload in [
            json!({ "id": id, "name": "" }),
            json!({ "id": id, "name": "x", "params": [{ "key": "", "value": "1" }] }),
            json!({ "id": id, "name": "x", "index": 1 }),
            json!({ "id": ids[0], "name": "x", "index": 999 }),
        ] {
            let error = AddFilter.apply_json(&mut document, payload).unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }));
        }
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "refused adds leave the document untouched"
        );
    }

    #[test]
    fn kinds_without_a_spine_are_invalid_state() {
        let (mut document, _) = testdoc::sample_document();
        let slice = document.registry.insert(ObjectData::Slice(SliceObject {
            internal_name: None,
            frame: geom::Rect::new(0.0, 0.0, 4.0, 4.0),
            shape: 0,
            fill_raw: 0,
            name: None,
            slice_type: None,
            link: None,
            alt: None,
            label: None,
            td_text: None,
            symbol_number: None,
            contours: Vec::new(),
            background: None,
            behaviors: Vec::new(),
            custom_data: std::collections::BTreeMap::new(),
            export: None,
        }));
        for payload in [
            json!({ "id": slice, "name": "x" }),
            json!({ "id": slice, "index": 0 }),
            json!({ "id": slice, "index": 0, "key": "k", "value": "v" }),
        ] {
            let error = match payload["name"].is_null() {
                false => AddFilter.apply_json(&mut document, payload).unwrap_err(),
                true => match payload["key"].is_null() {
                    false => SetFilterParam
                        .apply_json(&mut document, payload)
                        .unwrap_err(),
                    true => RemoveFilter.apply_json(&mut document, payload).unwrap_err(),
                },
            };
            assert!(matches!(error, EngineError::InvalidState { .. }));
        }
    }

    #[test]
    fn remove_restores_the_exact_list() {
        let (mut document, ids) = testdoc::sample_document();
        let id = ids[0];
        AddFilter
            .apply_json(&mut document, json!({ "id": id, "name": "first" }))
            .unwrap();
        AddFilter
            .apply_json(&mut document, json!({ "id": id, "name": "second" }))
            .unwrap();
        let full = match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => path.effects.clone(),
            _ => panic!("wrong kind"),
        };
        let delta = RemoveFilter
            .apply_json(&mut document, json!({ "id": id, "index": 0 }))
            .unwrap();
        let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
            panic!("wrong kind");
        };
        assert_eq!(path.effects.len(), 1);
        assert_eq!(path.effects[0].internal_name, "second");
        delta.apply_inverse(&mut document).unwrap();
        let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
            panic!("wrong kind");
        };
        assert_eq!(path.effects, full, "undo restores the exact list");
    }

    #[test]
    fn points_param_encodes_the_grab_point_node_shape() {
        let (mut document, ids) = testdoc::sample_document();
        let id = ids[0];
        AddFilter
            .apply_json(
                &mut document,
                json!({
                    "id": id,
                    "name": "curves",
                    "category": "Adjust Color",
                    "params": [
                        { "key": "rgb_points", "value": "", "points":
                          [[0.0, 0.0], [128.0, 64.0], [255.0, 255.0]] },
                        { "key": "brightness", "value": "0" },
                    ],
                }),
            )
            .unwrap();
        let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
            panic!("wrong kind");
        };
        let EffectValue::Raw(node) = &path.effects[0].properties[0].value else {
            panic!("points must encode as the raw node");
        };
        assert_eq!(node.fourcc, *b"GPLv");
        let NodeValue::Children(grab_points) = &node.value else {
            panic!("GPLv carries GPTv children");
        };
        assert_eq!(grab_points.len(), 3);
        let NodeValue::Children(coords) = &grab_points[1].value else {
            panic!("GPTv carries XLCf/YLCf children");
        };
        assert_eq!(grab_points[1].fourcc, *b"GPTv");
        assert_eq!((coords[0].fourcc, coords[1].fourcc), (*b"XLCf", *b"YLCf"));
        assert_eq!(coords[0].value, NodeValue::Float(128.0));
        assert_eq!(coords[1].value, NodeValue::Float(64.0));
        // The text sibling stayed text.
        assert_eq!(
            path.effects[0].properties[1].value,
            EffectValue::Text("0".to_owned())
        );
    }

    #[test]
    fn setparam_swaps_between_text_and_knot_encodings() {
        let (mut document, ids) = testdoc::sample_document();
        let id = ids[0];
        AddFilter
            .apply_json(
                &mut document,
                json!({ "id": id, "name": "curves", "params": [
                    { "key": "rgb_points", "value": "" },
                ]}),
            )
            .unwrap();
        // Text → knots.
        SetFilterParam
            .apply_json(
                &mut document,
                json!({ "id": id, "index": 0, "key": "rgb_points", "value": "",
                        "points": [[0.0, 0.0], [255.0, 255.0]] }),
            )
            .unwrap();
        let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
            panic!("wrong kind");
        };
        assert!(matches!(
            path.effects[0].properties[0].value,
            EffectValue::Raw(_)
        ));
        // Knots → text.
        SetFilterParam
            .apply_json(
                &mut document,
                json!({ "id": id, "index": 0, "key": "rgb_points", "value": "0 0 255 255" }),
            )
            .unwrap();
        let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
            panic!("wrong kind");
        };
        assert_eq!(
            path.effects[0].properties[0].value,
            EffectValue::Text("0 0 255 255".to_owned())
        );
    }

    #[test]
    fn bad_point_encodings_are_refused_without_mutation() {
        let (mut document, ids) = testdoc::sample_document();
        let id = ids[0];
        // The add-side encoding refusals (both at once, empty list).
        for params in [
            json!([{ "key": "rgb_points", "value": "x", "points": [[0.0, 0.0]] }]),
            json!([{ "key": "rgb_points", "value": "", "points": [] }]),
        ] {
            let error = AddFilter
                .apply_json(
                    &mut document,
                    json!({ "id": id, "name": "curves", "params": params }),
                )
                .unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }));
        }
        // A valid base effect so the setParam path has a slot to hit.
        AddFilter
            .apply_json(
                &mut document,
                json!({ "id": id, "name": "curves", "params": [
                    { "key": "rgb_points", "value": "" },
                ]}),
            )
            .unwrap();
        let before = serde_json::to_vec(&document).unwrap();
        for payload in [
            // Both encodings at once.
            json!({ "id": id, "index": 0, "key": "rgb_points", "value": "x",
                    "points": [[0.0, 0.0]] }),
            // An empty knot list.
            json!({ "id": id, "index": 0, "key": "rgb_points", "value": "",
                    "points": [] }),
        ] {
            let error = SetFilterParam
                .apply_json(&mut document, payload)
                .unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }));
        }
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "refused encodings leave the document untouched"
        );
        // Non-finite coordinates: JSON cannot carry NaN, so this
        // exercises the typed entry the registry wraps.
        let non_finite = param_value(
            "filter.add",
            "rgb_points",
            "",
            &Some(vec![[0.0, 0.0], [128.0, f64::NAN]]),
        )
        .unwrap_err();
        assert!(matches!(non_finite, EngineError::InvalidState { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "refused encodings leave the document untouched"
        );
    }

    #[test]
    fn undo_preserves_raw_file_residue() {
        // A raw node value (curve grab points) can enter through the
        // the reader; a later setParam + undo must bring it back whole.
        let (mut document, ids) = testdoc::sample_document();
        let id = ids[0];
        let raw = doc::Node {
            fourcc: *b"GPLv",
            value: doc::NodeValue::Children(vec![doc::Node {
                fourcc: *b"GPTv",
                value: doc::NodeValue::Children(vec![
                    doc::Node {
                        fourcc: *b"XLCf",
                        value: doc::NodeValue::Float(0.0),
                    },
                    doc::Node {
                        fourcc: *b"YLCf",
                        value: doc::NodeValue::Float(1.0),
                    },
                ]),
            }]),
        };
        with_effects(&mut document, id, |effects| {
            effects.push(Effect {
                category: "Adjust Color".to_owned(),
                internal_name: "curves".to_owned(),
                properties: vec![
                    EffectProperty {
                        key: "rgb_points".to_owned(),
                        value: EffectValue::Raw(raw.clone()),
                    },
                    EffectProperty {
                        key: "brightness".to_owned(),
                        value: EffectValue::Text("0".to_owned()),
                    },
                ],
            });
        })
        .expect("path carries a spine");
        let delta = SetFilterParam
            .apply_json(
                &mut document,
                json!({ "id": id, "index": 0, "key": "brightness", "value": "40" }),
            )
            .unwrap();
        delta.apply_inverse(&mut document).unwrap();
        let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
            panic!("wrong kind");
        };
        assert_eq!(
            path.effects[0].properties[0].value,
            EffectValue::Raw(raw),
            "raw grab points ride through the delta byte-exact"
        );
    }

    #[test]
    fn set_effect_name_renames_matching_effects_and_undo_is_exact() {
        let (mut document, ids) = testdoc::sample_document();
        AddFilter
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "name": "dropShadow", "category": "Shadow and Glow" }),
            )
            .unwrap();
        AddFilter
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "name": "gaussianBlur", "category": "Blur" }),
            )
            .unwrap();
        AddFilter
            .apply_json(
                &mut document,
                json!({ "id": ids[1], "name": "dropShadow", "category": "Shadow and Glow" }),
            )
            .unwrap();
        document.object_selection = vec![ids[0], ids[1]];
        let before = serde_json::to_vec(&document).unwrap();

        let delta = SetEffectName
            .apply_json(
                &mut document,
                json!({
                    "category": "Shadow and Glow",
                    "old_name": "dropShadow",
                    "new_name": "innerShadow",
                }),
            )
            .unwrap();
        for id in [ids[0], ids[1]] {
            let ObjectData::Path(path) = document.registry.get(id).unwrap() else {
                panic!("wrong kind");
            };
            let names: Vec<&str> = path
                .effects
                .iter()
                .map(|effect| effect.internal_name.as_str())
                .collect();
            if id == ids[0] {
                assert_eq!(
                    names,
                    ["innerShadow", "gaussianBlur"],
                    "only the match renames"
                );
            } else {
                assert_eq!(names, ["innerShadow"]);
            }
        }
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "undo is byte-exact"
        );
    }

    #[test]
    fn set_effect_name_gates() {
        let (mut document, ids) = testdoc::sample_document();
        AddFilter
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "name": "dropShadow", "category": "Shadow and Glow" }),
            )
            .unwrap();

        // Empty replacement name refuses.
        let error = SetEffectName
            .apply_json(
                &mut document,
                json!({ "category": "Shadow and Glow", "old_name": "dropShadow", "new_name": "" }),
            )
            .unwrap_err();
        assert!(error.to_string().contains("new_name is empty"));

        // Empty selection refuses.
        let error = SetEffectName
            .apply_json(
                &mut document,
                json!({ "category": "Shadow and Glow", "old_name": "dropShadow", "new_name": "x" }),
            )
            .unwrap_err();
        assert!(error.to_string().contains("nothing is selected"));

        // A category miss is the no-op, not a rename of the wrong row.
        document.object_selection = vec![ids[0]];
        let before = serde_json::to_vec(&document).unwrap();
        assert!(matches!(
            SetEffectName
                .apply_json(
                    &mut document,
                    json!({
                        "category": "Blur",
                        "old_name": "dropShadow",
                        "new_name": "x",
                    }),
                )
                .unwrap(),
            Delta::Noop
        ));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }
}
