//! Live-effect list commands: the filter stack on paths, text,
//! groups, and images.
//!
//! The list itself is the model's `effects: Vec<Effect>` (the `EFLv`
//! spine); these commands edit it with whole-object capture, so undo
//! restores the exact list including any raw (`GPLv`/`GDTv`) file
//! residue the reader preserved. Parameters here are string-valued
//! (`DCVs`) — raw node values enter the model only through the reader.

use doc::{Document, Effect, EffectProperty, EffectValue, ObjectData, ObjectId};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the filter commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(AddFilter)?;
    registry.register(RemoveFilter)?;
    registry.register(SetFilterParam)
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
struct FilterParamPayload {
    /// `DCKs` key, e.g. "gaussian_blur_radius".
    key: String,
    /// `DCVs` value; effects carry decimals as decimal strings.
    value: String,
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
        if payload.name.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "an effect needs an internal name".to_owned(),
            });
        }
        if payload.params.iter().any(|param| param.key.is_empty()) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "parameter keys must be non-empty".to_owned(),
            });
        }
        let effect = Effect {
            category: payload.category.clone(),
            internal_name: payload.name.clone(),
            properties: payload
                .params
                .iter()
                .map(|param| EffectProperty {
                    key: param.key.clone(),
                    value: EffectValue::Text(param.value.clone()),
                })
                .collect(),
        };
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
    /// `DCVs` value as a decimal string where numeric.
    value: String,
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
        mutate_effects(document, NAME, payload.id, |effects| {
            let properties = &mut effects[payload.index].properties;
            match properties.iter_mut().find(|prop| prop.key == key) {
                Some(prop) => prop.value = EffectValue::Text(value),
                None => properties.push(EffectProperty {
                    key,
                    value: EffectValue::Text(value),
                }),
            }
        })
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
    fn undo_preserves_raw_file_residue() {
        // A raw node value (curve grab points) can only enter through
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
}
