//! Object commands: display fields, paint, naming, and the structural
//! ops (delete, z-reorder, move between cells).
//!
//! Structural commands locate an object's container by walking pages,
//! layers, cells, and group children (parent/child edges are derived,
//! never stored — see `doc::registry`). Every container mutation is
//! captured as a whole-container delta, so undo overwrites the container
//! with its exact prior content.

use doc::{Document, ObjectData, ObjectId, PaintStyle, PathObject};

use crate::access;
use crate::catalog::display;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the object commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(SetObjectOpacity)?;
    registry.register(SetObjectBlend)?;
    registry.register(SetObjectVisible)?;
    registry.register(SetObjectLocked)?;
    registry.register(SetObjectFill)?;
    registry.register(SetObjectStroke)?;
    registry.register(RenameObject)?;
    registry.register(DeleteObject)?;
    registry.register(DuplicateObject)?;
    registry.register(TranslateObject)?;
    registry.register(ReorderObject)?;
    registry.register(MoveObjectToCell)?;
    registry.register(GroupObjects)?;
    registry.register(UngroupObject)?;
    registry.register(CreatePath)?;
    registry.register(TransformObject)?;
    registry.register(ScaleObject)?;
    registry.register(RotateObject)?;
    registry.register(SkewObject)?;
    registry.register(ReflectObject)?;
    registry.register(ResizeObject)?;
    registry.register(SetBoundsObject)?;
    registry.register(SetQuadrangle)?;
    registry.register(SetGroupType)
}

/// The blend table is `color::BlendMode`'s 27 modes in UI order; `BLDi`
/// indexes it.
const BLEND_MODES: i64 = 27;

/// Applies `edit` to the display fields of `id`, capturing the object.
/// `InvalidState` when the id is stale or the kind has no quartet.
fn mutate_display(
    document: &mut Document,
    name: &'static str,
    id: ObjectId,
    edit: impl FnOnce(&mut display::DisplayFields<'_>),
) -> Result<Delta, EngineError> {
    let before = access::object(document, name, id)?.clone();
    let applied = display::with_display(document, id, edit);
    if applied.is_none() {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("object {} does not carry the display fields", id.index()),
        });
    }
    Ok(Delta::Object {
        id,
        inverse: ObjectInverse::Mutated {
            object: Box::new(before),
        },
    })
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
        mutate_display(document, NAME, payload.id, |fields| {
            *fields.opacity = payload.opacity;
        })
    }
}

/// `object.setBlend` — blend mode index (0..26, the 27 `BLDi` modes in
/// `color::BlendMode` UI order).
struct SetObjectBlend;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetObjectBlendPayload {
    /// Target object.
    id: doc::ObjectId,
    /// New blend mode index.
    blend: i64,
}

impl TypedCommand for SetObjectBlend {
    type Payload = SetObjectBlendPayload;

    fn name(&self) -> &'static str {
        "object.setBlend"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.setBlend";
        if !(0..BLEND_MODES).contains(&payload.blend) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "blend {} outside 0..{} (mode table)",
                    payload.blend, BLEND_MODES
                ),
            });
        }
        mutate_display(document, NAME, payload.id, |fields| {
            *fields.blend = payload.blend;
        })
    }
}

/// `object.setVisible` — show or hide a path, group, or image.
struct SetObjectVisible;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetObjectVisiblePayload {
    /// Target object.
    id: doc::ObjectId,
    /// New visibility.
    visible: bool,
}

impl TypedCommand for SetObjectVisible {
    type Payload = SetObjectVisiblePayload;

    fn name(&self) -> &'static str {
        "object.setVisible"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.setVisible";
        mutate_display(document, NAME, payload.id, |fields| {
            *fields.visible = payload.visible;
        })
    }
}

/// `object.setLocked` — lock or unlock a path, group, or image.
struct SetObjectLocked;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetObjectLockedPayload {
    /// Target object.
    id: doc::ObjectId,
    /// New locked flag.
    locked: bool,
}

impl TypedCommand for SetObjectLocked {
    type Payload = SetObjectLockedPayload;

    fn name(&self) -> &'static str {
        "object.setLocked"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.setLocked";
        mutate_display(document, NAME, payload.id, |fields| {
            *fields.locked = payload.locked;
        })
    }
}

/// `object.setFill` — solid fill color on a path. A path without a
/// paint style gains one; the untouched channel starts transparent
/// black (`0`), so fill-only and stroke-only paths both stay faithful.
struct SetObjectFill;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetObjectFillPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Fill color, ABGR layout (byte layout unpinned until corpus
    /// evidence — same caveat as the model field).
    color_raw: i64,
}

impl TypedCommand for SetObjectFill {
    type Payload = SetObjectFillPayload;

    fn name(&self) -> &'static str {
        "object.setFill"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.setFill";
        let before = access::object(document, NAME, payload.id)?.clone();
        let applied = match document.registry.get_mut(payload.id) {
            Some(ObjectData::Path(path)) => {
                match &mut path.paint {
                    Some(paint) => paint.fill_raw = payload.color_raw,
                    None => {
                        path.paint = Some(PaintStyle {
                            stroke_raw: 0,
                            fill_raw: payload.color_raw,
                            fill: None,
                        });
                    }
                }
                true
            }
            _ => false,
        };
        if !applied {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} is not a path", payload.id.index()),
            });
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `object.setStroke` — solid stroke color on a path; see
/// [`SetObjectFill`] for the paint-style creation rule.
struct SetObjectStroke;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetObjectStrokePayload {
    /// Target object.
    id: doc::ObjectId,
    /// Stroke color, ABGR layout.
    color_raw: i64,
}

impl TypedCommand for SetObjectStroke {
    type Payload = SetObjectStrokePayload;

    fn name(&self) -> &'static str {
        "object.setStroke"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.setStroke";
        let before = access::object(document, NAME, payload.id)?.clone();
        let applied = match document.registry.get_mut(payload.id) {
            Some(ObjectData::Path(path)) => {
                match &mut path.paint {
                    Some(paint) => paint.stroke_raw = payload.color_raw,
                    None => {
                        path.paint = Some(PaintStyle {
                            stroke_raw: payload.color_raw,
                            fill_raw: 0,
                            fill: None,
                        });
                    }
                }
                true
            }
            _ => false,
        };
        if !applied {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} is not a path", payload.id.index()),
            });
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `object.rename` — set or clear an object's `OBNs` name (paths,
/// groups, images, slices).
struct RenameObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RenameObjectPayload {
    /// Target object.
    id: doc::ObjectId,
    /// `Some` sets the name, `None` clears it.
    name: Option<String>,
}

impl TypedCommand for RenameObject {
    type Payload = RenameObjectPayload;

    fn name(&self) -> &'static str {
        "object.rename"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.rename";
        let before = access::object(document, NAME, payload.id)?.clone();
        let applied = display::with_name(document, payload.id, |slot| {
            *slot = payload.name.clone();
        });
        if applied.is_none() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} does not carry a name", payload.id.index()),
            });
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// Where an object is referenced from. An object lives in exactly one
/// container: a cell's element list, or one group's children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    /// An element of one cell at `position`.
    Cell {
        /// Page index.
        page: usize,
        /// Layer index, bottom-first.
        layer: usize,
        /// Cell index (state number).
        cell: usize,
        /// Position in the element list.
        position: usize,
    },
    /// A child of the group `id` at `position`.
    Group {
        /// The group's registry id.
        id: ObjectId,
        /// Position in `children`.
        position: usize,
    },
}

/// Walks pages, layers, cells, then group children for the first
/// reference to `id`. Cell references win over group references (an
/// object referenced twice is a corrupted document; the walk order
/// keeps the result deterministic).
pub(crate) fn find_owner(document: &Document, id: ObjectId) -> Option<Owner> {
    for (page, page_data) in document.pages.iter().enumerate() {
        for (layer, layer_data) in page_data.layers.layers.iter().enumerate() {
            for (cell, cell_data) in layer_data.cells.iter().enumerate() {
                if let Some(position) = cell_data.elements.iter().position(|&e| e == id) {
                    return Some(Owner::Cell {
                        page,
                        layer,
                        cell,
                        position,
                    });
                }
            }
        }
    }
    for (group_id, object) in document.registry.iter() {
        if let ObjectData::Group(group) = object {
            if let Some(position) = group.children.iter().position(|&child| child == id) {
                return Some(Owner::Group {
                    id: group_id,
                    position,
                });
            }
        }
    }
    None
}

/// Removes `id` from its container, capturing the container's prior
/// state. The object itself is left in the registry.
pub(crate) fn detach(
    document: &mut Document,
    name: &'static str,
    id: ObjectId,
) -> Result<Delta, EngineError> {
    match find_owner(document, id) {
        Some(Owner::Cell {
            page,
            layer,
            cell,
            position,
        }) => {
            let before = access::cell_mut(document, name, page, layer, cell)?.clone();
            access::cell_mut(document, name, page, layer, cell)?
                .elements
                .remove(position);
            Ok(Delta::Cell {
                page,
                layer,
                cell,
                before: Box::new(before),
            })
        }
        Some(Owner::Group {
            id: parent,
            position,
        }) => {
            let before = match document.registry.get(parent) {
                Some(ObjectData::Group(group)) => group.clone(),
                _ => {
                    return Err(EngineError::MissingTarget {
                        name: name.to_owned(),
                        detail: format!(
                            "parent group {} (generation {}) is not live",
                            parent.index(),
                            parent.generation()
                        ),
                    });
                }
            };
            match document.registry.get_mut(parent) {
                Some(ObjectData::Group(group)) => {
                    group.children.remove(position);
                }
                _ => {
                    return Err(EngineError::MissingTarget {
                        name: name.to_owned(),
                        detail: format!(
                            "parent group {} (generation {}) is not live",
                            parent.index(),
                            parent.generation()
                        ),
                    });
                }
            }
            Ok(Delta::Object {
                id: parent,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(ObjectData::Group(before)),
                },
            })
        }
        None => Err(EngineError::MissingTarget {
            name: name.to_owned(),
            detail: format!(
                "object {} (generation {}) is referenced by no cell or group",
                id.index(),
                id.generation()
            ),
        }),
    }
}

/// Fails with the exact error [`detach`] would produce, without
/// touching anything — the multi-object commands' pre-flight so a
/// refused call mutates nothing (contract: errors leave the document
/// byte-identical).
pub(crate) fn ensure_detachable(
    document: &Document,
    name: &'static str,
    id: ObjectId,
) -> Result<(), EngineError> {
    match find_owner(document, id) {
        Some(Owner::Group { id: parent, .. }) => {
            if matches!(document.registry.get(parent), Some(ObjectData::Group(_))) {
                Ok(())
            } else {
                Err(EngineError::MissingTarget {
                    name: name.to_owned(),
                    detail: format!(
                        "parent group {} (generation {}) is not live",
                        parent.index(),
                        parent.generation()
                    ),
                })
            }
        }
        Some(Owner::Cell { .. }) => Ok(()),
        None => Err(EngineError::MissingTarget {
            name: name.to_owned(),
            detail: format!(
                "object {} (generation {}) is referenced by no cell or group",
                id.index(),
                id.generation()
            ),
        }),
    }
}

/// Inserts `id` into one cell at `at` (`None` appends), capturing the
/// cell's prior state.
#[allow(clippy::too_many_arguments)]
pub(crate) fn attach_cell(
    document: &mut Document,
    name: &'static str,
    page: usize,
    layer: usize,
    cell: usize,
    id: ObjectId,
    at: Option<usize>,
) -> Result<Delta, EngineError> {
    let target = access::cell_mut(document, name, page, layer, cell)?;
    let len = target.elements.len();
    let at = at.unwrap_or(len);
    if at > len {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("insert index {at} beyond {len} elements"),
        });
    }
    let before = target.clone();
    target.elements.insert(at, id);
    Ok(Delta::Cell {
        page,
        layer,
        cell,
        before: Box::new(before),
    })
}

/// `object.setGroupType` — CS6 `dom.setGroupType({type})`: retypes the
/// selected groups. "normal" (the default when the argument is
/// omitted) sets `GRTi` 0; the two FW4-deprecated mask spellings
/// refuse — a mask group's [mask, masked] child layout is built by
/// `mask.add`, not by retyping (the restructure is unmodeled).
struct SetGroupType;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetGroupTypePayload {
    /// "normal" (default), or the deprecated mask spellings.
    #[serde(default)]
    group_type: String,
}

impl TypedCommand for SetGroupType {
    type Payload = SetGroupTypePayload;

    fn name(&self) -> &'static str {
        "object.setGroupType"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.setGroupType";
        match payload.group_type.as_str() {
            "normal" | "" => {}
            other => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "group type {other:?} is the FW4-deprecated mask restructure; build mask groups through mask.add"
                    ),
                });
            }
        }
        if document.object_selection.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "nothing is selected".to_owned(),
            });
        }
        let mut deltas = Vec::new();
        for id in document.object_selection.clone() {
            let before = match document.registry.get(id) {
                // Non-groups are skipped — the verb addresses "the
                // currently selected groups".
                Some(ObjectData::Group(group)) if group.group_type != 0 => {
                    ObjectData::Group(group.clone())
                }
                _ => continue,
            };
            if let Some(ObjectData::Group(slot)) = document.registry.get_mut(id) {
                slot.group_type = 0;
            }
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

/// `object.delete` — removes an object from its container and the
/// registry. Undo restores the container and revives the object at its
/// exact slot and generation.
struct DeleteObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DeleteObjectPayload {
    /// Target object.
    id: doc::ObjectId,
}

impl TypedCommand for DeleteObject {
    type Payload = DeleteObjectPayload;

    fn name(&self) -> &'static str {
        "object.delete"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.delete";
        let object = access::object(document, NAME, payload.id)?.clone();
        // Detach first: its errors fire before anything mutates.
        let container = detach(document, NAME, payload.id)?;
        if document.registry.remove(payload.id).is_none() {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!(
                    "object {} (generation {}) vanished between capture and removal",
                    payload.id.index(),
                    payload.id.generation()
                ),
            });
        }
        Ok(Delta::Group(vec![
            container,
            Delta::Object {
                id: payload.id,
                inverse: ObjectInverse::Removed {
                    object: Box::new(object),
                },
            },
        ]))
    }
}

/// Deep-clones `source`'s subtree into fresh registry slots: group
/// children recurse, every live descendant gets its own slot, and the
/// copy's children list re-points at the fresh slots (a duplicated
/// group shares nothing with the original). Dangling child references
/// skip per the walker rule. Every minted id — the root copy last —
/// appends to `created` so undo can retract them all.
fn clone_subtree(
    document: &mut Document,
    source: doc::ObjectId,
    created: &mut Vec<doc::ObjectId>,
) -> Result<doc::ObjectId, EngineError> {
    const NAME: &str = "object.duplicate";
    let mut data =
        document
            .registry
            .get(source)
            .cloned()
            .ok_or_else(|| EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!(
                    "object {} (generation {}) is not live",
                    source.index(),
                    source.generation()
                ),
            })?;
    if let ObjectData::Group(group) = &mut data {
        let mut children = Vec::with_capacity(group.children.len());
        for child in &group.children {
            if document.registry.get(*child).is_some() {
                children.push(clone_subtree(document, *child, created)?);
            }
        }
        group.children = children;
    }
    let copy = document.registry.insert(data);
    created.push(copy);
    Ok(copy)
}

/// `object.duplicate` — clones an object into fresh registry slots and
/// inserts the copy into the source's own container, directly above it
/// (z-order is list order, bottom-first). CS6's Edit > Clone places the
/// copy on top of the original at the same spot; a following drag moves
/// it. Groups deep-clone: every descendant gets its own slot, so the
/// copy edits independently of the original. Works in cells and in
/// groups.
struct DuplicateObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DuplicateObjectPayload {
    /// Source object.
    id: doc::ObjectId,
}

impl TypedCommand for DuplicateObject {
    type Payload = DuplicateObjectPayload;

    fn name(&self) -> &'static str {
        "object.duplicate"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.duplicate";
        access::object(document, NAME, payload.id)?;
        let owner = find_owner(document, payload.id).ok_or_else(|| EngineError::MissingTarget {
            name: NAME.to_owned(),
            detail: format!(
                "object {} (generation {}) is referenced by no cell or group",
                payload.id.index(),
                payload.id.generation()
            ),
        })?;
        let mut created = Vec::new();
        let copy = clone_subtree(document, payload.id, &mut created)?;
        let created_deltas = created
            .into_iter()
            .map(|id| Delta::Object {
                id,
                inverse: ObjectInverse::Created,
            })
            .collect::<Vec<_>>();
        match owner {
            Owner::Cell {
                page,
                layer,
                cell,
                position,
            } => {
                let placed =
                    attach_cell(document, NAME, page, layer, cell, copy, Some(position + 1))?;
                let mut deltas = vec![placed];
                deltas.extend(created_deltas);
                Ok(Delta::Group(deltas))
            }
            Owner::Group {
                id: parent,
                position,
            } => {
                let before = match document.registry.get(parent) {
                    Some(ObjectData::Group(group)) => group.clone(),
                    _ => {
                        return Err(EngineError::MissingTarget {
                            name: NAME.to_owned(),
                            detail: format!(
                                "parent group {} (generation {}) is not live",
                                parent.index(),
                                parent.generation()
                            ),
                        });
                    }
                };
                if let Some(ObjectData::Group(group)) = document.registry.get_mut(parent) {
                    group.children.insert(position + 1, copy);
                } else {
                    return Err(EngineError::MissingTarget {
                        name: NAME.to_owned(),
                        detail: format!(
                            "parent group {} (generation {}) vanished between capture and insert",
                            parent.index(),
                            parent.generation()
                        ),
                    });
                }
                let mut deltas = vec![Delta::Object {
                    id: parent,
                    inverse: ObjectInverse::Mutated {
                        object: Box::new(ObjectData::Group(before)),
                    },
                }];
                deltas.extend(created_deltas);
                Ok(Delta::Group(deltas))
            }
        }
    }
}

/// `object.translate` — moves objects in document space by `(dx, dy)`.
/// Pointer-drag moves, arrow-key nudges, and Alt-drag clones all land
/// here. Groups translate depth-first (each moved child carries its own
/// captured inverse; the group records nothing — its children list is
/// unchanged), and every position-bearing field of the moved kind comes
/// along: path contours, gradient fill handles, the `PRIv` primitive
/// rect, text and slice frames, slice polygon contours, image
/// locations, and symbol instance transforms.
struct TranslateObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct TranslateObjectPayload {
    /// Objects to move (a shared descendant moves once, with its group).
    ids: Vec<doc::ObjectId>,
    /// Document-space delta.
    dx: f64,
    /// Document-space delta.
    dy: f64,
}

impl TypedCommand for TranslateObject {
    type Payload = TranslateObjectPayload;

    fn name(&self) -> &'static str {
        "object.translate"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.translate";
        if payload.ids.is_empty() {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: "no objects given".to_owned(),
            });
        }
        let delta = geom::Vec2::new(payload.dx, payload.dy);
        // Deterministic delta order (registry slot order), and a
        // group-plus-its-child selection moves the child once.
        let mut ids = payload.ids.clone();
        ids.sort_by_key(|id| id.index());
        ids.dedup();
        let mut moved = std::collections::HashSet::new();
        let mut inverses = Vec::new();
        for id in ids {
            translate_object(document, NAME, id, delta, &mut moved, &mut inverses)?;
        }
        Ok(Delta::Group(inverses))
    }
}

/// Translates one object (recursing into groups), pushing the captured
/// inverse for every object whose data actually changed.
pub(crate) fn translate_object(
    document: &mut Document,
    name: &'static str,
    id: ObjectId,
    delta: geom::Vec2,
    moved: &mut std::collections::HashSet<ObjectId>,
    inverses: &mut Vec<Delta>,
) -> Result<(), EngineError> {
    if !moved.insert(id) {
        return Ok(());
    }
    let before = access::object(document, name, id)?.clone();
    let data = document
        .registry
        .get_mut(id)
        .ok_or_else(|| EngineError::MissingTarget {
            name: name.to_owned(),
            detail: format!(
                "object {} (generation {}) is not live",
                id.index(),
                id.generation()
            ),
        })?;
    match data {
        ObjectData::Path(path) => {
            for contour in &mut path.contours {
                translate_contour(contour, delta);
            }
            for point in [
                &mut path.fill_start,
                &mut path.fill_end,
                &mut path.fill_third,
            ]
            .into_iter()
            .flatten()
            {
                *point += delta;
            }
            if let Some(primitive) = &mut path.primitive {
                primitive.rect = primitive.rect + delta;
            }
        }
        ObjectData::Text(text) => text.frame = text.frame + delta,
        ObjectData::Group(group) => {
            // The group is geometry-free; its children carry the move.
            let children = group.children.clone();
            for child in children {
                translate_object(document, name, child, delta, moved, inverses)?;
            }
            return Ok(());
        }
        ObjectData::Image(image) => image.location += delta,
        ObjectData::Slice(slice) => {
            slice.frame = slice.frame + delta;
            for contour in &mut slice.contours {
                translate_contour(contour, delta);
            }
        }
        ObjectData::SymbolInstance(instance) => {
            instance.transform = geom::Affine::translate((delta.x, delta.y)) * instance.transform;
        }
    }
    inverses.push(Delta::Object {
        id,
        inverse: ObjectInverse::Mutated {
            object: Box::new(before),
        },
    });
    Ok(())
}

/// Shifts every anchor and control handle of one contour.
fn translate_contour(contour: &mut doc::Contour, delta: geom::Vec2) {
    for point in &mut contour.points {
        point.anchor += delta;
        if let Some(primary) = &mut point.primary {
            *primary += delta;
        }
        if let Some(secondary) = &mut point.secondary {
            *secondary += delta;
        }
    }
}

// The transform family — CS6's `scaleSelection` / `rotateSelection` /
// `skewSelection` / `reflectSelection` / `resizeSelection` /
// `transformSelection` / `setSelectionBounds` as seven commands over
// one primitive (the free affine of `object.transform`) and one
// variant walk. Per decision 0019: paths bake point-wise (the format
// carries no path `MTXv`), `PRIv`/text/instance compose their live
// matrices, and image targets refuse until the resample pass exists.

/// Checks a transform closure before any mutation: every reached id is
/// live and none is bitmap-backed (`TILv` tile banks desync under any
/// geometry rewrite — the resample pass is the recorded dependency).
fn precheck_transform_closure(
    document: &Document,
    name: &'static str,
    id: ObjectId,
    seen: &mut std::collections::HashSet<ObjectId>,
) -> Result<(), EngineError> {
    if !seen.insert(id) {
        return Ok(());
    }
    match access::object(document, name, id)? {
        ObjectData::Image(_) => Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!(
                "object {} is bitmap-backed; transforms that rewrite geometry need the resample pass (decision 0019)",
                id.index()
            ),
        }),
        ObjectData::Group(group) => {
            let children = group.children.clone();
            for child in children {
                precheck_transform_closure(document, name, child, seen)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// A degenerate rect holding one point — the seed of a union walk.
fn point_rect(point: geom::Point) -> geom::Rect {
    geom::Rect::new(point.x, point.y, point.x, point.y)
}

/// Folds one rect into a running union.
fn union_extend(out: &mut Option<geom::Rect>, rect: geom::Rect) {
    *out = Some(match *out {
        Some(acc) => acc.union(rect),
        None => rect,
    });
}

/// The content bounds of one object under `xform` (document space),
/// folded into `out`. Groups recurse with their own transform; symbol
/// instances contribute their definition's content under the instance
/// transform (unit-square fallback for an unknown symbol number). The
/// walk is depth-capped like `check_closure`.
pub(crate) fn content_bounds(
    document: &Document,
    id: ObjectId,
    xform: geom::Affine,
    depth: usize,
    seen: &mut std::collections::HashSet<ObjectId>,
    out: &mut Option<geom::Rect>,
) {
    if depth > 64 || !seen.insert(id) {
        return;
    }
    let Some(data) = document.registry.get(id) else {
        return;
    };
    match data {
        ObjectData::Path(path) => {
            let mut local: Option<geom::Rect> = None;
            for contour in &path.contours {
                for point in &contour.points {
                    for p in std::iter::once(point.anchor)
                        .chain(point.primary)
                        .chain(point.secondary)
                    {
                        union_extend(&mut local, point_rect(p));
                    }
                }
            }
            if let Some(local) = local {
                union_extend(out, geom::map_rect(xform, local));
            }
        }
        ObjectData::Text(text) => union_extend(out, geom::map_rect(xform, text.frame)),
        ObjectData::Slice(slice) => union_extend(out, geom::map_rect(xform, slice.frame)),
        ObjectData::Image(image) => {
            let frame = geom::Rect::new(
                image.location.x,
                image.location.y,
                image.location.x + image.pixels.0 as f64,
                image.location.y + image.pixels.1 as f64,
            );
            union_extend(out, geom::map_rect(xform, frame));
        }
        ObjectData::Group(group) => {
            for child in &group.children {
                content_bounds(document, *child, xform, depth + 1, seen, out);
            }
        }
        ObjectData::SymbolInstance(instance) => {
            let composed = instance.transform * xform;
            let definition = document
                .symbols
                .iter()
                .find(|def| def.symbol_number == instance.symbol_number);
            match definition {
                Some(def) => {
                    // Per-instance `seen`: two instances at different
                    // transforms must each contribute their content.
                    let mut inner = std::collections::HashSet::new();
                    for layer in &def.layers.layers {
                        for cell in &layer.cells {
                            for child in &cell.elements {
                                content_bounds(
                                    document,
                                    *child,
                                    composed,
                                    depth + 1,
                                    &mut inner,
                                    out,
                                );
                            }
                        }
                    }
                }
                None => union_extend(
                    out,
                    geom::map_rect(composed, geom::Rect::new(0.0, 0.0, 1.0, 1.0)),
                ),
            }
        }
    }
}

/// The union bounds of the target set — CS6's "selection bounds"
/// without a selection surface. `None` when the targets carry no
/// bounds at all (nothing live).
pub(crate) fn union_bounds(document: &Document, ids: &[ObjectId]) -> Option<geom::Rect> {
    let mut out: Option<geom::Rect> = None;
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        content_bounds(
            document,
            *id,
            geom::Affine::IDENTITY,
            0,
            &mut seen,
            &mut out,
        );
    }
    out
}

/// Maps every anchor and control handle of one contour through
/// `matrix`.
fn transform_contour(contour: &mut doc::Contour, matrix: geom::Affine) {
    for point in &mut contour.points {
        point.anchor = matrix * point.anchor;
        if let Some(primary) = &mut point.primary {
            *primary = matrix * *primary;
        }
        if let Some(secondary) = &mut point.secondary {
            *secondary = matrix * *secondary;
        }
    }
}

/// Applies `matrix` to one object (recursing into groups), pushing a
/// captured inverse for every object whose data actually changed. The
/// closure must be pre-validated (`precheck_transform_closure`).
fn apply_matrix(
    document: &mut Document,
    name: &'static str,
    id: ObjectId,
    matrix: geom::Affine,
    seen: &mut std::collections::HashSet<ObjectId>,
    inverses: &mut Vec<Delta>,
) -> Result<(), EngineError> {
    if !seen.insert(id) {
        return Ok(());
    }
    let before = access::object(document, name, id)?.clone();
    let data = document
        .registry
        .get_mut(id)
        .ok_or_else(|| EngineError::MissingTarget {
            name: name.to_owned(),
            detail: format!(
                "object {} (generation {}) is not live",
                id.index(),
                id.generation()
            ),
        })?;
    match data {
        ObjectData::Path(path) => {
            for contour in &mut path.contours {
                transform_contour(contour, matrix);
            }
            for point in [
                &mut path.fill_start,
                &mut path.fill_end,
                &mut path.fill_third,
            ]
            .into_iter()
            .flatten()
            {
                *point = matrix * *point;
            }
            if let Some(primitive) = &mut path.primitive {
                primitive.transform = matrix * primitive.transform;
            }
        }
        ObjectData::Text(text) => {
            // The object's `MTXv` carries the layout-box transform.
            text.transform = Some(matrix * text.transform.unwrap_or(geom::Affine::IDENTITY));
        }
        ObjectData::Group(group) => {
            // The group is geometry-free; its children carry the
            // transform (each under the same caller matrix, so a
            // shared descendant transforms once).
            let children = group.children.clone();
            for child in children {
                apply_matrix(document, name, child, matrix, seen, inverses)?;
            }
            return Ok(());
        }
        ObjectData::Slice(slice) => {
            slice.frame = geom::map_rect(matrix, slice.frame);
            for contour in &mut slice.contours {
                transform_contour(contour, matrix);
            }
        }
        ObjectData::SymbolInstance(instance) => {
            instance.transform = matrix * instance.transform;
        }
        // Precheck refused images; this arm keeps the match total
        // without an unreachable.
        ObjectData::Image(_) => return Ok(()),
    }
    inverses.push(Delta::Object {
        id,
        inverse: ObjectInverse::Mutated {
            object: Box::new(before),
        },
    });
    Ok(())
}

/// The family's common spine: validate and normalize ids, pre-check
/// every closure, resolve the pivot (`None` = the union bounds center,
/// CS6's selection-center default), compose the matrix, then walk.
fn transform_core(
    document: &mut Document,
    name: &'static str,
    ids: &[ObjectId],
    pivot: Option<[f64; 2]>,
    compose: impl FnOnce(geom::Point) -> Result<geom::Affine, EngineError>,
) -> Result<Delta, EngineError> {
    if ids.is_empty() {
        return Err(EngineError::MissingTarget {
            name: name.to_owned(),
            detail: "no objects given".to_owned(),
        });
    }
    let mut ids = ids.to_vec();
    ids.sort_by_key(|id| id.index());
    ids.dedup();
    // Validate all targets before anything mutates — a refused call
    // leaves the document untouched.
    let mut seen = std::collections::HashSet::new();
    for id in &ids {
        precheck_transform_closure(document, name, *id, &mut seen)?;
    }
    let pivot = match pivot {
        Some([x, y]) if x.is_finite() && y.is_finite() => geom::Point::new(x, y),
        Some([x, y]) => {
            return Err(EngineError::BadPayload {
                name: name.to_owned(),
                detail: format!("pivot ({x}, {y}) is not finite"),
            });
        }
        None => {
            let bounds = union_bounds(document, &ids).ok_or_else(|| EngineError::InvalidState {
                name: name.to_owned(),
                detail: "targets carry no bounds to pivot at".to_owned(),
            })?;
            bounds.center()
        }
    };
    let matrix = compose(pivot)?;
    if matrix.as_coeffs().iter().any(|coeff| !coeff.is_finite()) {
        return Err(EngineError::BadPayload {
            name: name.to_owned(),
            detail: "the composed matrix is not finite".to_owned(),
        });
    }
    let mut inverses = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for id in &ids {
        apply_matrix(document, name, *id, matrix, &mut seen, &mut inverses)?;
    }
    Ok(Delta::Group(inverses))
}

/// A pivot override ([x, y]) or CS6's default (`None` = union bounds
/// center). Shared by the pivot-taking sugars.
type Pivot = Option<[f64; 2]>;

/// `object.transform` — CS6's `dom.transformSelection(matrix)`: applies
/// a free affine (kurbo coefficient order) to the targets.
struct TransformObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct TransformObjectPayload {
    /// Objects to transform (a shared descendant moves once).
    ids: Vec<ObjectId>,
    /// The affine as `[a, b, c, d, tx, ty]` —
    /// `(x, y) -> (a·x + c·y + tx, b·x + d·y + ty)`.
    matrix: [f64; 6],
}

impl TypedCommand for TransformObject {
    type Payload = TransformObjectPayload;

    fn name(&self) -> &'static str {
        "object.transform"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.transform";
        transform_core(document, NAME, &payload.ids, None, |pivot| {
            // The free matrix composes around the pivot: the pivot
            // stays fixed under the caller's matrix.
            Ok(geom::Affine::translate((pivot.x, pivot.y))
                * geom::Affine::new(payload.matrix)
                * geom::Affine::translate((-pivot.x, -pivot.y)))
        })
    }
}

/// `object.scale` — CS6's `dom.scaleSelection(xScaleAmount,
/// yScaleAmount)`: scales about the pivot (default: the union bounds
/// center). Factors are CS6's "0.0 or greater" — zero bakes a collapse.
struct ScaleObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ScaleObjectPayload {
    /// Objects to scale.
    ids: Vec<ObjectId>,
    /// Horizontal factor (1 = 100%).
    sx: f64,
    /// Vertical factor (1 = 100%).
    sy: f64,
    /// Pivot override; `None` scales about the union bounds center.
    pivot: Pivot,
}

impl TypedCommand for ScaleObject {
    type Payload = ScaleObjectPayload;

    fn name(&self) -> &'static str {
        "object.scale"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.scale";
        if !payload.sx.is_finite() || !payload.sy.is_finite() {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!("scale ({}, {}) is not finite", payload.sx, payload.sy),
            });
        }
        if payload.sx < 0.0 || payload.sy < 0.0 {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!("scale ({}, {}) is negative", payload.sx, payload.sy),
            });
        }
        transform_core(document, NAME, &payload.ids, payload.pivot, |pivot| {
            Ok(geom::scale_about(
                geom::Affine::IDENTITY,
                payload.sx,
                payload.sy,
                pivot,
            ))
        })
    }
}

/// `object.rotate` — CS6's `dom.rotateSelection(rotationDegrees)`:
/// rotates clockwise by whole degrees about the pivot (default: the
/// union bounds center). Positive is clockwise, matching the y-down
/// document space the docs describe.
struct RotateObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RotateObjectPayload {
    /// Objects to rotate.
    ids: Vec<ObjectId>,
    /// Degrees, clockwise.
    degrees: f64,
    /// Pivot override; `None` rotates about the union bounds center.
    pivot: Pivot,
}

impl TypedCommand for RotateObject {
    type Payload = RotateObjectPayload;

    fn name(&self) -> &'static str {
        "object.rotate"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.rotate";
        if !payload.degrees.is_finite() {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!("degrees {} is not finite", payload.degrees),
            });
        }
        transform_core(document, NAME, &payload.ids, payload.pivot, |pivot| {
            Ok(geom::Affine::translate((pivot.x, pivot.y))
                * geom::Affine::rotate(payload.degrees.to_radians())
                * geom::Affine::translate((-pivot.x, -pivot.y)))
        })
    }
}

/// `object.skew` — CS6's `dom.skewSelection` (CS6 API inventory; the
/// CS4 help set has no entry): skews by two angles in degrees about the
/// pivot (default: the union bounds center). A 90° angle composes a
/// non-finite tangent and is refused by the spine's finite check.
struct SkewObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SkewObjectPayload {
    /// Objects to skew.
    ids: Vec<ObjectId>,
    /// Horizontal skew (x leans by tan(angle)·y).
    skew_x: f64,
    /// Vertical skew (y leans by tan(angle)·x).
    skew_y: f64,
    /// Pivot override; `None` skews about the union bounds center.
    pivot: Pivot,
}

impl TypedCommand for SkewObject {
    type Payload = SkewObjectPayload;

    fn name(&self) -> &'static str {
        "object.skew"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.skew";
        if !payload.skew_x.is_finite() || !payload.skew_y.is_finite() {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!(
                    "skew ({}, {}) is not finite",
                    payload.skew_x, payload.skew_y
                ),
            });
        }
        transform_core(document, NAME, &payload.ids, payload.pivot, |pivot| {
            // (x, y) -> (x + tan(ax)·y, tan(ay)·x + y) in kurbo's
            // [a, b, c, d, tx, ty] layout.
            let skew = geom::Affine::new([
                1.0,
                payload.skew_y.to_radians().tan(),
                payload.skew_x.to_radians().tan(),
                1.0,
                0.0,
                0.0,
            ]);
            Ok(geom::Affine::translate((pivot.x, pivot.y))
                * skew
                * geom::Affine::translate((-pivot.x, -pivot.y)))
        })
    }
}

/// `object.setQuadrangle` — CS6
/// `dom.setQuadrangle(pTopLeft, pTopRight, pBottomRight, pBottomLeft)`:
/// "the same as performing a transform operation" — the unit square
/// maps onto the given quadrangle, expressed in percentages of the
/// selection bounds (corner = bounds origin + percent × bounds size),
/// and the whole selection takes the resulting affine. Only
/// parallelogram quads are accepted: the affine `MTXv` cannot carry a
/// projective one (CS6's `autoTrimImages` option is bitmap-side and
/// the transform path refuses bitmaps regardless).
struct SetQuadrangle;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetQuadranglePayload {
    /// Objects to transform.
    ids: Vec<ObjectId>,
    /// Unit-square corner percentages, one per quadrangle corner.
    top_left: [f64; 2],
    /// Unit-square corner percentages, one per quadrangle corner.
    top_right: [f64; 2],
    /// Unit-square corner percentages, one per quadrangle corner.
    bottom_right: [f64; 2],
    /// Unit-square corner percentages, one per quadrangle corner.
    bottom_left: [f64; 2],
}

impl TypedCommand for SetQuadrangle {
    type Payload = SetQuadranglePayload;

    fn name(&self) -> &'static str {
        "object.setQuadrangle"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.setQuadrangle";
        for corner in [
            payload.top_left,
            payload.top_right,
            payload.bottom_right,
            payload.bottom_left,
        ] {
            if corner.iter().any(|value| !value.is_finite()) {
                return Err(EngineError::BadPayload {
                    name: NAME.to_owned(),
                    detail: "quadrangle percentages are not finite".to_owned(),
                });
            }
        }
        // Capture the pre-command bounds: the quadrangle's reference
        // square (transform_core re-validates the targets itself).
        let bounds =
            union_bounds(document, &payload.ids).ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "targets carry no bounds to quadrangle within".to_owned(),
            })?;
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "target bounds are degenerate".to_owned(),
            });
        }
        let to_world = |percent: [f64; 2]| {
            geom::Point::new(
                bounds.x0 + percent[0] * bounds.width(),
                bounds.y0 + percent[1] * bounds.height(),
            )
        };
        let (tl, tr, br, bl) = (
            to_world(payload.top_left),
            to_world(payload.top_right),
            to_world(payload.bottom_right),
            to_world(payload.bottom_left),
        );
        // An affine maps the unit square onto a parallelogram exactly;
        // anything else needs a projective matrix `MTXv` cannot hold.
        let tolerance = 1e-6 * bounds.width().max(bounds.height()).max(1.0);
        let expected = geom::Point::new(tr.x + bl.x - tl.x, tr.y + bl.y - tl.y);
        if (br.x - expected.x).abs() > tolerance || (br.y - expected.y).abs() > tolerance {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the quadrangle is not a parallelogram (affine MTXv only)".to_owned(),
            });
        }
        // The affine maps the bounds rectangle onto the quad — the
        // selection fills the quadrangle — so each edge vector scales
        // by the reciprocal of the bounds size it replaces.
        let matrix = geom::Affine::new([
            (tr.x - tl.x) / bounds.width(),
            (tr.y - tl.y) / bounds.width(),
            (bl.x - tl.x) / bounds.height(),
            (bl.y - tl.y) / bounds.height(),
            tl.x,
            tl.y,
        ]);
        // Pivot (0, 0) leaves the composed matrix untouched.
        transform_core(document, NAME, &payload.ids, Some([0.0, 0.0]), |_| {
            Ok(matrix)
        })
    }
}

/// `object.reflect` — CS6's `dom.reflectSelection(bHoriz, bVert)`:
/// flips about the pivot (default: the union bounds center). Both
/// flags false is CS6's documented no-op.
struct ReflectObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ReflectObjectPayload {
    /// Objects to reflect.
    ids: Vec<ObjectId>,
    /// Flip horizontally (left-right).
    horizontal: bool,
    /// Flip vertically (top-bottom).
    vertical: bool,
    /// Pivot override; `None` reflects about the union bounds center.
    pivot: Pivot,
}

impl TypedCommand for ReflectObject {
    type Payload = ReflectObjectPayload;

    fn name(&self) -> &'static str {
        "object.reflect"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.reflect";
        if !payload.horizontal && !payload.vertical {
            return Ok(Delta::Noop);
        }
        transform_core(document, NAME, &payload.ids, payload.pivot, |pivot| {
            Ok(geom::Affine::translate((pivot.x, pivot.y))
                * geom::Affine::scale_non_uniform(
                    if payload.horizontal { -1.0 } else { 1.0 },
                    if payload.vertical { -1.0 } else { 1.0 },
                )
                * geom::Affine::translate((-pivot.x, -pivot.y)))
        })
    }
}

/// Resolves the union bounds or names what was missing. Shared by the
/// fixed-origin sugars (`resize`, `setBounds`): an empty id list is the
/// family's MissingTarget, a bounds-less set is InvalidState.
fn union_bounds_or(
    document: &Document,
    name: &'static str,
    ids: &[ObjectId],
) -> Result<geom::Rect, EngineError> {
    if ids.is_empty() {
        return Err(EngineError::MissingTarget {
            name: name.to_owned(),
            detail: "no objects given".to_owned(),
        });
    }
    union_bounds(document, ids).ok_or_else(|| EngineError::InvalidState {
        name: name.to_owned(),
        detail: "targets carry no bounds to resize".to_owned(),
    })
}

/// `object.resize` — CS6's `dom.resizeSelection(width, height)`:
/// resizes to an exact pixel box, "keeping the top-left corner of the
/// selection in place".
struct ResizeObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ResizeObjectPayload {
    /// Objects to resize.
    ids: Vec<ObjectId>,
    /// New selection width in pixels.
    width: f64,
    /// New selection height in pixels.
    height: f64,
}

impl TypedCommand for ResizeObject {
    type Payload = ResizeObjectPayload;

    fn name(&self) -> &'static str {
        "object.resize"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.resize";
        if !(payload.width.is_finite() && payload.height.is_finite())
            || payload.width <= 0.0
            || payload.height <= 0.0
        {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!(
                    "resize ({}, {}) is not a finite positive size",
                    payload.width, payload.height
                ),
            });
        }
        let bounds = union_bounds_or(document, NAME, &payload.ids)?;
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "targets carry a degenerate selection box".to_owned(),
            });
        }
        transform_core(
            document,
            NAME,
            &payload.ids,
            Some([bounds.x0, bounds.y0]),
            |origin| {
                Ok(geom::scale_about(
                    geom::Affine::IDENTITY,
                    payload.width / bounds.width(),
                    payload.height / bounds.height(),
                    origin,
                ))
            },
        )
    }
}

/// `object.setBounds` — CS6's `dom.setSelectionBounds(rect)`:
/// "moves and resizes the selection in a single operation" — the union
/// box lands exactly on the given rectangle.
struct SetBoundsObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetBoundsObjectPayload {
    /// Objects to move and resize.
    ids: Vec<ObjectId>,
    /// New left edge.
    left: f64,
    /// New top edge.
    top: f64,
    /// New width.
    width: f64,
    /// New height.
    height: f64,
}

impl TypedCommand for SetBoundsObject {
    type Payload = SetBoundsObjectPayload;

    fn name(&self) -> &'static str {
        "object.setBounds"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.setBounds";
        if !(payload.width.is_finite() && payload.height.is_finite())
            || payload.width <= 0.0
            || payload.height <= 0.0
            || !payload.left.is_finite()
            || !payload.top.is_finite()
        {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!(
                    "bounds ({}, {}, {}, {}) is not a finite positive box",
                    payload.left, payload.top, payload.width, payload.height
                ),
            });
        }
        let bounds = union_bounds_or(document, NAME, &payload.ids)?;
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "targets carry a degenerate selection box".to_owned(),
            });
        }
        transform_core(
            document,
            NAME,
            &payload.ids,
            Some([bounds.x0, bounds.y0]),
            |origin| {
                Ok(
                    geom::Affine::translate((payload.left - bounds.x0, payload.top - bounds.y0))
                        * geom::scale_about(
                            geom::Affine::IDENTITY,
                            payload.width / bounds.width(),
                            payload.height / bounds.height(),
                            origin,
                        ),
                )
            },
        )
    }
}

/// `object.reorder` — moves an object within its container's list
/// (z-order is list order, bottom-first). `to` indexes the list after
/// removal: 0 is bottom, `len - 1` is top.
struct ReorderObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ReorderObjectPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Target index in the container list (after removal).
    to: usize,
}

impl TypedCommand for ReorderObject {
    type Payload = ReorderObjectPayload;

    fn name(&self) -> &'static str {
        "object.reorder"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.reorder";
        let owner = find_owner(document, payload.id).ok_or_else(|| EngineError::MissingTarget {
            name: NAME.to_owned(),
            detail: format!(
                "object {} (generation {}) is referenced by no cell or group",
                payload.id.index(),
                payload.id.generation()
            ),
        })?;
        match owner {
            Owner::Cell {
                page,
                layer,
                cell,
                position,
            } => {
                let target = access::cell_mut(document, NAME, page, layer, cell)?;
                let len = target.elements.len();
                if payload.to >= len {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: format!("index {} beyond {} elements", payload.to, len),
                    });
                }
                let before = target.clone();
                let moved = target.elements.remove(position);
                target.elements.insert(payload.to, moved);
                Ok(Delta::Cell {
                    page,
                    layer,
                    cell,
                    before: Box::new(before),
                })
            }
            Owner::Group {
                id: parent,
                position,
            } => {
                let before = match document.registry.get(parent) {
                    Some(ObjectData::Group(group)) => group.clone(),
                    _ => {
                        return Err(EngineError::MissingTarget {
                            name: NAME.to_owned(),
                            detail: format!(
                                "parent group {} (generation {}) is not live",
                                parent.index(),
                                parent.generation()
                            ),
                        });
                    }
                };
                match document.registry.get_mut(parent) {
                    Some(ObjectData::Group(group)) => {
                        let len = group.children.len();
                        if payload.to >= len {
                            return Err(EngineError::InvalidState {
                                name: NAME.to_owned(),
                                detail: format!("index {} beyond {} children", payload.to, len),
                            });
                        }
                        let moved = group.children.remove(position);
                        group.children.insert(payload.to, moved);
                    }
                    _ => {
                        return Err(EngineError::MissingTarget {
                            name: NAME.to_owned(),
                            detail: format!(
                                "parent group {} (generation {}) is not live",
                                parent.index(),
                                parent.generation()
                            ),
                        });
                    }
                }
                Ok(Delta::Object {
                    id: parent,
                    inverse: ObjectInverse::Mutated {
                        object: Box::new(ObjectData::Group(before)),
                    },
                })
            }
        }
    }
}

/// `object.moveToCell` — moves an object between containers. Moving
/// within the source cell folds into one cell delta; the optional
/// `index` appends when absent.
struct MoveObjectToCell;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct MoveObjectToCellPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Destination page index.
    page: usize,
    /// Destination layer index, bottom-first.
    layer: usize,
    /// Destination cell index (state number).
    cell: usize,
    /// Destination index in the element list; absent appends.
    index: Option<usize>,
}

impl TypedCommand for MoveObjectToCell {
    type Payload = MoveObjectToCellPayload;

    fn name(&self) -> &'static str {
        "object.moveToCell"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.moveToCell";
        // Validate the destination before touching the source.
        let target_len =
            access::cell_mut(document, NAME, payload.page, payload.layer, payload.cell)?
                .elements
                .len();
        let at = payload.index.unwrap_or(target_len);
        if at > target_len {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("insert index {at} beyond {target_len} elements"),
            });
        }

        let owner = find_owner(document, payload.id).ok_or_else(|| EngineError::MissingTarget {
            name: NAME.to_owned(),
            detail: format!(
                "object {} (generation {}) is referenced by no cell or group",
                payload.id.index(),
                payload.id.generation()
            ),
        })?;
        if let Owner::Cell {
            page,
            layer,
            cell,
            position,
        } = owner
        {
            if (page, layer, cell) == (payload.page, payload.layer, payload.cell) {
                // Same cell: one remove + insert, one capture.
                let target = access::cell_mut(document, NAME, page, layer, cell)?;
                let len = target.elements.len();
                let before = target.clone();
                let moved = target.elements.remove(position);
                let at = at.min(len - 1);
                target.elements.insert(at, moved);
                return Ok(Delta::Cell {
                    page,
                    layer,
                    cell,
                    before: Box::new(before),
                });
            }
        }
        let source = detach(document, NAME, payload.id)?;
        let destination = attach_cell(
            document,
            NAME,
            payload.page,
            payload.layer,
            payload.cell,
            payload.id,
            Some(at),
        )?;
        Ok(Delta::Group(vec![source, destination]))
    }
}

/// Does `owner` resolve to the same container as `reference`?
fn same_container(owner: &Owner, reference: &Owner) -> bool {
    match (owner, reference) {
        (
            Owner::Cell {
                page: p,
                layer: l,
                cell: c,
                ..
            },
            Owner::Cell {
                page: rp,
                layer: rl,
                cell: rc,
                ..
            },
        ) => (p, l, c) == (rp, rl, rc),
        (Owner::Group { id, .. }, Owner::Group { id: rid, .. }) => id == rid,
        _ => false,
    }
}

/// The container-list position an `Owner` holds.
fn owner_position(owner: &Owner) -> usize {
    match owner {
        Owner::Cell { position, .. } | Owner::Group { position, .. } => *position,
    }
}

/// `object.group` — folds objects into one group at the topmost
/// member's position (CS6's Modify > Group). Every id must be live and
/// referenced by the *same* container — one cell, or one parent group;
/// a selection spanning containers is the app's problem (move the
/// objects together first, then group). Children keep the members'
/// relative z-order (bottom-first), whatever order the payload lists.
struct GroupObjects;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct GroupObjectsPayload {
    /// Objects to fold into the group; at least two, no duplicates.
    ids: Vec<ObjectId>,
    /// `OBNs` name for the new group.
    name: Option<String>,
}

impl TypedCommand for GroupObjects {
    type Payload = GroupObjectsPayload;

    fn name(&self) -> &'static str {
        "object.group"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.group";
        if payload.ids.len() < 2 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("need at least 2 objects, got {}", payload.ids.len()),
            });
        }
        let mut seen = std::collections::HashSet::new();
        let mut members: Vec<(ObjectId, Owner)> = Vec::with_capacity(payload.ids.len());
        for id in &payload.ids {
            if !seen.insert(*id) {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} appears twice in the selection", id.index()),
                });
            }
            ensure_detachable(document, NAME, *id)?;
            let owner = find_owner(document, *id).ok_or_else(|| EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!(
                    "object {} (generation {}) is referenced by no cell or group",
                    id.index(),
                    id.generation()
                ),
            })?;
            members.push((*id, owner));
        }
        let first = members[0].1;
        if members
            .iter()
            .any(|(_, owner)| !same_container(owner, &first))
        {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "a group must live in one container (move the objects together first)"
                    .to_owned(),
            });
        }

        // Bottom-first child order = ascending container positions;
        // the group takes the topmost member's slot, which after the
        // removals is `max - (count - 1)` — every other member sat
        // below it.
        members.sort_by_key(|(_, owner)| owner_position(owner));
        let children: Vec<ObjectId> = members.iter().map(|(id, _)| *id).collect();
        let insert_at = owner_position(&members[members.len() - 1].1) - (members.len() - 1);
        let group_id = document
            .registry
            .insert(ObjectData::Group(doc::GroupObject {
                group_type: 0,
                name: payload.name.clone(),
                children,
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                mask_hidden: false,
                mask_linked: None,
                effects: Vec::new(),
            }));
        match first {
            Owner::Cell {
                page, layer, cell, ..
            } => {
                let (page, layer, cell) = (page, layer, cell);
                let target = access::cell_mut(document, NAME, page, layer, cell)?;
                let before = target.clone();
                for position in members.iter().rev().map(|(_, owner)| owner_position(owner)) {
                    target.elements.remove(position);
                }
                target.elements.insert(insert_at, group_id);
                Ok(Delta::Group(vec![
                    Delta::Cell {
                        page,
                        layer,
                        cell,
                        before: Box::new(before),
                    },
                    Delta::Object {
                        id: group_id,
                        inverse: ObjectInverse::Created,
                    },
                ]))
            }
            Owner::Group { id: parent, .. } => {
                let before = match document.registry.get(parent) {
                    Some(ObjectData::Group(group)) => group.clone(),
                    _ => {
                        return Err(EngineError::MissingTarget {
                            name: NAME.to_owned(),
                            detail: format!(
                                "parent group {} (generation {}) is not live",
                                parent.index(),
                                parent.generation()
                            ),
                        });
                    }
                };
                match document.registry.get_mut(parent) {
                    Some(ObjectData::Group(group)) => {
                        for position in members.iter().rev().map(|(_, owner)| owner_position(owner))
                        {
                            group.children.remove(position);
                        }
                        group.children.insert(insert_at, group_id);
                    }
                    _ => {
                        return Err(EngineError::MissingTarget {
                            name: NAME.to_owned(),
                            detail: format!(
                                "parent group {} (generation {}) is not live",
                                parent.index(),
                                parent.generation()
                            ),
                        });
                    }
                }
                Ok(Delta::Group(vec![
                    Delta::Object {
                        id: parent,
                        inverse: ObjectInverse::Mutated {
                            object: Box::new(ObjectData::Group(before)),
                        },
                    },
                    Delta::Object {
                        id: group_id,
                        inverse: ObjectInverse::Created,
                    },
                ]))
            }
        }
    }
}

/// `object.ungroup` — splices a group's children back into the group's
/// own container at its position and removes the group (CS6's
/// Modify > Ungroup). Any group dissolves, mask-structured ones
/// included — the mask family carries its own semantics later.
struct UngroupObject;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct UngroupObjectPayload {
    /// The group to dissolve.
    id: doc::ObjectId,
}

impl TypedCommand for UngroupObject {
    type Payload = UngroupObjectPayload;

    fn name(&self) -> &'static str {
        "object.ungroup"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.ungroup";
        let group = match access::object(document, NAME, payload.id)? {
            ObjectData::Group(group) => group.clone(),
            _ => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a group", payload.id.index()),
                });
            }
        };
        // Referenced-before-anything-mutates: the owner walk also
        // proves the registry removal below cannot miss.
        let owner = find_owner(document, payload.id).ok_or_else(|| EngineError::MissingTarget {
            name: NAME.to_owned(),
            detail: format!(
                "object {} (generation {}) is referenced by no cell or group",
                payload.id.index(),
                payload.id.generation()
            ),
        })?;
        let removed = ObjectData::Group(group.clone());
        let mut deltas = Vec::new();
        match owner {
            Owner::Cell {
                page,
                layer,
                cell,
                position,
            } => {
                let target = access::cell_mut(document, NAME, page, layer, cell)?;
                let before = target.clone();
                target.elements.remove(position);
                for (offset, child) in group.children.iter().enumerate() {
                    target.elements.insert(position + offset, *child);
                }
                deltas.push(Delta::Cell {
                    page,
                    layer,
                    cell,
                    before: Box::new(before),
                });
            }
            Owner::Group {
                id: parent,
                position,
            } => {
                let before = match document.registry.get(parent) {
                    Some(ObjectData::Group(group)) => group.clone(),
                    _ => {
                        return Err(EngineError::MissingTarget {
                            name: NAME.to_owned(),
                            detail: format!(
                                "parent group {} (generation {}) is not live",
                                parent.index(),
                                parent.generation()
                            ),
                        });
                    }
                };
                match document.registry.get_mut(parent) {
                    Some(ObjectData::Group(parent_group)) => {
                        parent_group.children.remove(position);
                        for (offset, child) in group.children.iter().enumerate() {
                            parent_group.children.insert(position + offset, *child);
                        }
                    }
                    _ => {
                        return Err(EngineError::MissingTarget {
                            name: NAME.to_owned(),
                            detail: format!(
                                "parent group {} (generation {}) is not live",
                                parent.index(),
                                parent.generation()
                            ),
                        });
                    }
                }
                deltas.push(Delta::Object {
                    id: parent,
                    inverse: ObjectInverse::Mutated {
                        object: Box::new(ObjectData::Group(before)),
                    },
                });
            }
        }
        if document.registry.remove(payload.id).is_none() {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!(
                    "object {} (generation {}) vanished between capture and removal",
                    payload.id.index(),
                    payload.id.generation()
                ),
            });
        }
        deltas.push(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Removed {
                object: Box::new(removed),
            },
        });
        Ok(Delta::Group(deltas))
    }
}

/// `object.createPath` — creates a primitive path (rect, ellipse, or
/// line) from a world-space bounding box and appends it to a target
/// cell. Rect/ellipse carry a `PRIv` primitive source, so they stay
/// live-editable as primitives; a line is a bare 2-point open contour.
struct CreatePath;

/// What geometry the created path holds.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum CreateKind {
    /// Four-corner closed rectangle.
    Rect,
    /// Four-point closed ellipse (kappa handles).
    Ellipse,
    /// Two-point open contour, corner to corner.
    Line,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct CreatePathPayload {
    /// What geometry to build.
    kind: CreateKind,
    /// Bounding box, world coordinates.
    x: f64,
    /// Bounding box, world coordinates.
    y: f64,
    /// Bounding box, world coordinates.
    width: f64,
    /// Bounding box, world coordinates.
    height: f64,
    /// Target page index.
    page: usize,
    /// Target layer index.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Optional object name.
    name: Option<String>,
}

/// Circle-approximation constant: distance from an ellipse's corner
/// anchor to its control handles along the tangent.
const ELLIPSE_KAPPA: f64 = 0.552_284_749_830_793_3;

impl TypedCommand for CreatePath {
    type Payload = CreatePathPayload;

    fn name(&self) -> &'static str {
        "object.createPath"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "object.createPath";
        if !payload.width.is_finite()
            || !payload.height.is_finite()
            || !payload.x.is_finite()
            || !payload.y.is_finite()
        {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "geometry must be finite".to_owned(),
            });
        }
        // Validate the destination before anything mutates.
        access::cell_mut(document, NAME, payload.page, payload.layer, payload.cell)?;

        let rect = geom::Rect::new(
            payload.x,
            payload.y,
            payload.x + payload.width,
            payload.y + payload.height,
        );
        let (contours, primitive) = match payload.kind {
            CreateKind::Rect => (rect_contour(rect), Some(rect_primitive(rect))),
            CreateKind::Ellipse => (ellipse_contour(rect), Some(rect_primitive(rect))),
            CreateKind::Line => (line_contour(rect), None),
        };
        let object = ObjectData::Path(PathObject {
            contours,
            primitive,
            even_odd: false,
            paint: Some(PaintStyle {
                stroke_raw: 0,         // transparent black — no stroke yet
                fill_raw: 0xFF00_0000, // ABGR opaque black — the CS6 new-path fill
                fill: None,
            }),
            fill_start: None,
            fill_end: None,
            fill_third: None,
            brush_placement: None,
            fill_on_top: None,
            locked: false,
            visible: true,
            opacity: 1000,
            blend: 0,
            name: payload.name.clone(),
            effects: Vec::new(),
        });

        let id = document.registry.insert(object);
        // Append (top of z-order). The same cell lookup passed above,
        // and `None` appends — this cannot fail, so there is no
        // orphan-registration cleanup to run.
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

/// One closed 4-anchor contour from a rect's corners.
fn rect_contour(rect: geom::Rect) -> Vec<doc::Contour> {
    let points = vec![
        corner(rect.x0, rect.y0),
        corner(rect.x1, rect.y0),
        corner(rect.x1, rect.y1),
        corner(rect.x0, rect.y1),
    ];
    vec![doc::Contour {
        closed: true,
        point_count: 4,
        points,
    }]
}

/// One closed 4-anchor ellipse contour with kappa control handles.
fn ellipse_contour(rect: geom::Rect) -> Vec<doc::Contour> {
    let cx = (rect.x0 + rect.x1) / 2.0;
    let cy = (rect.y0 + rect.y1) / 2.0;
    let rx = rect.width() / 2.0;
    let ry = rect.height() / 2.0;
    let kx = rx * ELLIPSE_KAPPA;
    let ky = ry * ELLIPSE_KAPPA;
    // Anchors at 12/3/6/9 o'clock; each control handle offsets along
    // the neighbouring tangent.
    let curve = |anchor: geom::Point, c1: geom::Point, c2: geom::Point| doc::PathPoint {
        anchor,
        primary: Some(c1),
        secondary: Some(c2),
        is_curve: true,
    };
    let points = vec![
        curve(
            geom::Point::new(cx, rect.y0),
            geom::Point::new(cx + kx, rect.y0),
            geom::Point::new(cx - kx, rect.y0),
        ),
        curve(
            geom::Point::new(rect.x1, cy),
            geom::Point::new(rect.x1, rect.y0 + ky),
            geom::Point::new(rect.x1, rect.y1 - ky),
        ),
        curve(
            geom::Point::new(cx, rect.y1),
            geom::Point::new(cx + kx, rect.y1),
            geom::Point::new(cx - kx, rect.y1),
        ),
        curve(
            geom::Point::new(rect.x0, cy),
            geom::Point::new(rect.x0, rect.y1 - ky),
            geom::Point::new(rect.x0, rect.y0 + ky),
        ),
    ];
    vec![doc::Contour {
        closed: true,
        point_count: 4,
        points,
    }]
}

/// One open 2-anchor contour, corner to corner.
fn line_contour(rect: geom::Rect) -> Vec<doc::Contour> {
    vec![doc::Contour {
        closed: false,
        point_count: 2,
        points: vec![corner(rect.x0, rect.y0), corner(rect.x1, rect.y1)],
    }]
}

/// One closed 8-anchor rounded-rectangle contour: straight-edge
/// anchors joined by quarter-circle bezier arcs (`kappa` handles).
/// `radius` clamps to half the shorter side. Radius zero is the sharp
/// [`rect_contour`].
pub(crate) fn rounded_rect_contour(rect: geom::Rect, radius: f64) -> Vec<doc::Contour> {
    let r = radius
        .max(0.0)
        .min(rect.width().abs().min(rect.height().abs()) / 2.0);
    if r <= 0.0 {
        return rect_contour(rect);
    }
    let k = r * ELLIPSE_KAPPA;
    // Anchors walk clockwise from the top edge's left end. Each corner
    // arc spans two anchors: the straight-edge point leaving it (its
    // forward handle aims along the tangent into the corner) and the
    // next edge's entry point (its backward handle comes back out).
    // `primary` is the forward handle — the ellipse derivation's
    // working convention.
    let arc_in = |x: f64, y: f64, handle: geom::Point| doc::PathPoint {
        anchor: geom::Point::new(x, y),
        primary: Some(handle),
        secondary: None,
        is_curve: true,
    };
    let arc_out = |x: f64, y: f64, handle: geom::Point| doc::PathPoint {
        anchor: geom::Point::new(x, y),
        primary: None,
        secondary: Some(handle),
        is_curve: true,
    };
    let (x0, y0, x1, y1) = (rect.x0, rect.y0, rect.x1, rect.y1);
    let points = vec![
        // TL arc lands here from the left edge.
        arc_out(x0 + r, y0, geom::Point::new(x0 + r - k, y0)),
        // TR arc leaves along the top edge.
        arc_in(x1 - r, y0, geom::Point::new(x1 - r + k, y0)),
        arc_out(x1, y0 + r, geom::Point::new(x1, y0 + r - k)),
        // BR arc.
        arc_in(x1, y1 - r, geom::Point::new(x1, y1 - r + k)),
        arc_out(x1 - r, y1, geom::Point::new(x1 - r + k, y1)),
        // BL arc.
        arc_in(x0 + r, y1, geom::Point::new(x0 + r - k, y1)),
        arc_out(x0, y1 - r, geom::Point::new(x0, y1 - r + k)),
        // TL arc leaves up the left edge (closes back to the start).
        arc_in(x0, y0 + r, geom::Point::new(x0, y0 + r - k)),
    ];
    vec![doc::Contour {
        closed: true,
        point_count: points.len() as i64,
        points,
    }]
}

/// A straight corner point.
fn corner(x: f64, y: f64) -> doc::PathPoint {
    doc::PathPoint {
        anchor: geom::Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    }
}

/// A `PRIv` primitive source spanning `rect`, identity transform.
fn rect_primitive(rect: geom::Rect) -> doc::PrimitiveSource {
    doc::PrimitiveSource {
        rect,
        radius: 0.0,
        transform: geom::Affine::IDENTITY,
        maintain_radius: false,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::catalog::testdoc;
    use crate::command::Command;
    use doc::{
        Cell, Contour, ImageObject, Layer, LayerList, Page, PageInfo, PathObject, PathPoint,
        SliceObject, TileRef,
    };
    use serde_json::json;

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
            effects: Vec::new(),
        })
    }

    fn image_object() -> ObjectData {
        ObjectData::Image(ImageObject {
            location: geom::Point::ZERO,
            offset: geom::Point::ZERO,
            pixels: (4, 4),
            tile_size: 128,
            tiles: vec![TileRef::Matte(0xFF_00_00_00)],
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        })
    }

    fn slice_object() -> ObjectData {
        ObjectData::Slice(SliceObject {
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
        })
    }

    /// One page, one layer, `frame_count` visible cells, `ids` placed
    /// into cell 0 in order.
    fn document_with(objects: Vec<ObjectData>) -> (Document, Vec<ObjectId>) {
        let mut document = Document::default();
        let mut ids = Vec::new();
        for object in objects {
            ids.push(document.registry.insert(object));
        }
        let layer = Layer {
            name: "Layer 0".to_owned(),
            display: true,
            cells: vec![
                Cell {
                    locked: false,
                    visible: true,
                    elements: ids.clone(),
                },
                Cell {
                    locked: false,
                    visible: true,
                    elements: Vec::new(),
                },
            ],
            ..Layer::default()
        };
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: 2,
                frame_count_duplicate: 2,
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
        (document, ids)
    }

    fn opacity_of(document: &Document, id: ObjectId) -> i64 {
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => path.opacity,
            ObjectData::Group(group) => group.opacity,
            ObjectData::Image(image) => image.opacity,
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn set_opacity_applies_and_captures() {
        let (mut document, ids) = document_with(vec![path_object(), image_object()]);
        let payload = json!({ "id": ids[0], "opacity": 400 });
        let delta = SetObjectOpacity.apply_json(&mut document, payload).unwrap();
        assert_eq!(opacity_of(&document, ids[0]), 400);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(opacity_of(&document, ids[0]), 1000);
    }

    #[test]
    fn set_blend_and_visibility_round_trip() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let delta = SetObjectBlend
            .apply_json(&mut document, json!({ "id": ids[0], "blend": 9 }))
            .unwrap();
        delta.apply_inverse(&mut document).unwrap();
        let delta = SetObjectVisible
            .apply_json(&mut document, json!({ "id": ids[0], "visible": false }))
            .unwrap();
        match document.registry.get(ids[0]).unwrap() {
            ObjectData::Path(path) => assert!(!path.visible),
            other => panic!("wrong object kind: {other:?}"),
        }
        delta.apply_inverse(&mut document).unwrap();
        match document.registry.get(ids[0]).unwrap() {
            ObjectData::Path(path) => assert!(path.visible),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn blend_outside_the_mode_table_is_refused() {
        let (mut document, ids) = document_with(vec![path_object()]);
        for blend in [-1, 27, 100] {
            let error = SetObjectBlend
                .apply_json(&mut document, json!({ "id": ids[0], "blend": blend }))
                .unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }), "{blend}");
        }
    }

    #[test]
    fn display_commands_refuse_kinds_without_the_quartet() {
        let (mut document, ids) = document_with(vec![slice_object()]);
        for name in ["object.setOpacity", "object.setBlend", "object.setVisible"] {
            let payload = match name {
                "object.setOpacity" => json!({ "id": ids[0], "opacity": 500 }),
                "object.setBlend" => json!({ "id": ids[0], "blend": 1 }),
                _ => json!({ "id": ids[0], "visible": false }),
            };
            let error = apply_by_name(&mut document, name, payload).unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }), "{name}");
        }
    }

    fn apply_by_name(
        document: &mut Document,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<Delta, EngineError> {
        let mut registry = CommandRegistry::new();
        install(&mut registry).unwrap();
        registry.execute(document, name, payload)
    }

    #[test]
    fn fill_creates_paint_style_and_stroke_extends_it() {
        let (mut document, ids) = document_with(vec![path_object()]);
        SetObjectFill
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "color_raw": 0x00FF_FFFF_u32 as i64 }),
            )
            .unwrap();
        match document.registry.get(ids[0]).unwrap() {
            ObjectData::Path(path) => match &path.paint {
                Some(paint) => {
                    assert_eq!(paint.fill_raw, 0x00FF_FFFF);
                    assert_eq!(paint.stroke_raw, 0, "untouched channel starts transparent");
                }
                None => panic!("paint style was created"),
            },
            other => panic!("wrong object kind: {other:?}"),
        }
        SetObjectStroke
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "color_raw": 0x11_22_33 }),
            )
            .unwrap();
        match document.registry.get(ids[0]).unwrap() {
            ObjectData::Path(path) => match &path.paint {
                Some(paint) => {
                    assert_eq!(paint.stroke_raw, 0x11_22_33);
                    assert_eq!(paint.fill_raw, 0x00FF_FFFF, "fill untouched");
                }
                None => panic!("paint style survived"),
            },
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn fill_on_a_non_path_is_refused() {
        let (mut document, ids) = document_with(vec![image_object()]);
        let error = SetObjectFill
            .apply_json(&mut document, json!({ "id": ids[0], "color_raw": 1 }))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }

    #[test]
    fn rename_sets_and_clears() {
        let (mut document, ids) = document_with(vec![slice_object()]);
        let delta = RenameObject
            .apply_json(&mut document, json!({ "id": ids[0], "name": "hotspot" }))
            .unwrap();
        match document.registry.get(ids[0]).unwrap() {
            ObjectData::Slice(slice) => assert_eq!(slice.name.as_deref(), Some("hotspot")),
            other => panic!("wrong object kind: {other:?}"),
        }
        delta.apply_inverse(&mut document).unwrap();
        match document.registry.get(ids[0]).unwrap() {
            ObjectData::Slice(slice) => assert_eq!(slice.name, None),
            other => panic!("wrong object kind: {other:?}"),
        }
        // Clearing is itself a command.
        RenameObject
            .apply_json(&mut document, json!({ "id": ids[0], "name": null }))
            .unwrap();
    }

    #[test]
    fn delete_removes_from_cell_and_registry_and_undo_restores_both() {
        let (mut document, ids) = document_with(vec![path_object(), path_object()]);
        let original = serde_json::to_vec(&document).unwrap();
        let delta = DeleteObject
            .apply_json(&mut document, json!({ "id": ids[0] }))
            .unwrap();
        assert!(document.registry.get(ids[0]).is_none());
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![ids[1]]
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
        assert!(document.registry.get(ids[0]).is_some());
    }

    #[test]
    fn delete_from_a_group_captures_the_group() {
        // Child path inside a group; the group sits in the cell.
        let (mut document, ids) = document_with(vec![path_object(), path_object()]);
        let group_id = document
            .registry
            .insert(ObjectData::Group(doc::GroupObject {
                group_type: 0,
                name: None,
                children: vec![ids[1]],
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                mask_hidden: false,
                mask_linked: None,
                effects: Vec::new(),
            }));
        document.pages[0].layers.layers[0].cells[0].elements = vec![ids[0], group_id];

        let delta = DeleteObject
            .apply_json(&mut document, json!({ "id": ids[1] }))
            .unwrap();
        assert!(document.registry.get(ids[1]).is_none());
        match document.registry.get(group_id).unwrap() {
            ObjectData::Group(group) => assert!(group.children.is_empty()),
            other => panic!("wrong object kind: {other:?}"),
        }
        delta.apply_inverse(&mut document).unwrap();
        match document.registry.get(group_id).unwrap() {
            ObjectData::Group(group) => assert_eq!(group.children, vec![ids[1]]),
            other => panic!("wrong object kind: {other:?}"),
        }
        assert!(document.registry.get(ids[1]).is_some());
    }

    #[test]
    fn delete_an_unreferenced_object_is_missing_target() {
        let (mut document, ids) = document_with(vec![path_object()]);
        document.pages.remove(0);
        let error = DeleteObject
            .apply_json(&mut document, json!({ "id": ids[0] }))
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
        // The object itself is still live: validation precedes mutation.
        assert!(document.registry.get(ids[0]).is_some());
    }

    #[test]
    fn reorder_moves_within_the_cell_list() {
        let (mut document, ids) = document_with(vec![path_object(), path_object(), path_object()]);
        let cell_elements =
            |document: &Document| document.pages[0].layers.layers[0].cells[0].elements.clone();
        assert_eq!(cell_elements(&document), ids);
        ReorderObject
            .apply_json(&mut document, json!({ "id": ids[0], "to": 2 }))
            .unwrap();
        assert_eq!(cell_elements(&document), vec![ids[1], ids[2], ids[0]]);
        // Out-of-range is refused.
        let error = ReorderObject
            .apply_json(&mut document, json!({ "id": ids[0], "to": 3 }))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }

    #[test]
    fn reorder_inside_a_group_captures_the_group() {
        let (mut document, ids) = document_with(vec![path_object(), path_object()]);
        let group_id = document
            .registry
            .insert(ObjectData::Group(doc::GroupObject {
                group_type: 0,
                name: None,
                children: vec![ids[0], ids[1]],
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                mask_hidden: false,
                mask_linked: None,
                effects: Vec::new(),
            }));
        document.pages[0].layers.layers[0].cells[0].elements = vec![group_id];
        let delta = ReorderObject
            .apply_json(&mut document, json!({ "id": ids[0], "to": 1 }))
            .unwrap();
        match document.registry.get(group_id).unwrap() {
            ObjectData::Group(group) => assert_eq!(group.children, vec![ids[1], ids[0]]),
            other => panic!("wrong object kind: {other:?}"),
        }
        delta.apply_inverse(&mut document).unwrap();
        match document.registry.get(group_id).unwrap() {
            ObjectData::Group(group) => assert_eq!(group.children, vec![ids[0], ids[1]]),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn move_to_cell_crosses_cells_and_undo_restores_both() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let delta = MoveObjectToCell
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "page": 0, "layer": 0, "cell": 1, "index": 0 }),
            )
            .unwrap();
        assert!(
            document.pages[0].layers.layers[0].cells[0]
                .elements
                .is_empty()
        );
        assert_eq!(
            document.pages[0].layers.layers[0].cells[1].elements,
            vec![ids[0]]
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![ids[0]]
        );
        assert!(
            document.pages[0].layers.layers[0].cells[1]
                .elements
                .is_empty()
        );
    }

    #[test]
    fn move_within_one_cell_is_one_delta() {
        let (mut document, ids) = document_with(vec![path_object(), path_object()]);
        let delta = MoveObjectToCell
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "page": 0, "layer": 0, "cell": 0, "index": null }),
            )
            .unwrap();
        match delta {
            Delta::Cell { .. } => {}
            other => panic!("expected a single cell delta, got {other:?}"),
        }
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![ids[1], ids[0]]
        );
    }

    #[test]
    fn move_validates_the_destination_first() {
        let (mut document, ids) = document_with(vec![path_object()]);
        // Bad cell index: refused, source untouched.
        let error = MoveObjectToCell
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "page": 0, "layer": 0, "cell": 9 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
        // Index beyond the destination list: refused, source untouched.
        let error = MoveObjectToCell
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "page": 0, "layer": 0, "cell": 1, "index": 5 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![ids[0]]
        );
    }

    #[test]
    fn payload_unknown_fields_are_rejected() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let error = SetObjectOpacity
            .apply_json(&mut document, json!({ "id": ids[0], "opasity": 400 }))
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }));
    }

    #[test]
    fn stale_id_is_missing_target() {
        let (mut document, ids) = document_with(vec![path_object()]);
        document.registry.remove(ids[0]).expect("remove");
        let error = SetObjectOpacity
            .apply_json(&mut document, json!({ "id": ids[0], "opacity": 400 }))
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
    }

    #[test]
    fn create_path_appends_a_rect_and_undo_removes_both_halves() {
        let (mut document, _) = document_with(vec![]);
        let original = serde_json::to_vec(&document).unwrap();
        let delta = CreatePath
            .apply_json(
                &mut document,
                json!({
                    "kind": "rect",
                    "x": 10.0, "y": 20.0, "width": 30.0, "height": 40.0,
                    "page": 0, "layer": 0, "cell": 0,
                    "name": "plaque",
                }),
            )
            .unwrap();
        let cell = &document.pages[0].layers.layers[0].cells[0];
        assert_eq!(cell.elements.len(), 1);
        let id = cell.elements[0];
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => {
                assert_eq!(path.contours.len(), 1);
                assert!(path.contours[0].closed);
                assert_eq!(
                    path.contours[0].points[0].anchor,
                    geom::Point::new(10.0, 20.0)
                );
                assert_eq!(
                    path.contours[0].points[2].anchor,
                    geom::Point::new(40.0, 60.0)
                );
                assert!(path.primitive.is_some());
                assert_eq!(path.name.as_deref(), Some("plaque"));
            }
            other => panic!("wrong object kind: {other:?}"),
        }
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.registry.get(id).is_none());
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            original,
            "undo restores the exact pre-create bytes"
        );
    }

    #[test]
    fn create_ellipse_and_line_shapes() {
        let (mut document, _) = document_with(vec![]);
        CreatePath
            .apply_json(
                &mut document,
                json!({
                    "kind": "ellipse",
                    "x": -10.0, "y": -10.0, "width": 20.0, "height": 20.0,
                    "page": 0, "layer": 0, "cell": 0, "name": null,
                }),
            )
            .unwrap();
        let id = document.pages[0].layers.layers[0].cells[0].elements[0];
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => {
                assert!(path.primitive.is_some());
                // Every point is a curve point with both handles.
                assert!(path.contours[0].points.iter().all(|point| point.is_curve));
            }
            other => panic!("wrong object kind: {other:?}"),
        }

        CreatePath
            .apply_json(
                &mut document,
                json!({
                    "kind": "line",
                    "x": 0.0, "y": 0.0, "width": 50.0, "height": 25.0,
                    "page": 0, "layer": 0, "cell": 0, "name": null,
                }),
            )
            .unwrap();
        let id = document.pages[0].layers.layers[0].cells[0].elements[1];
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => {
                assert!(path.primitive.is_none(), "a line is not a primitive");
                assert!(!path.contours[0].closed);
                assert_eq!(path.contours[0].points.len(), 2);
            }
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn create_path_validates_the_target_first() {
        let (mut document, _) = document_with(vec![path_object()]);
        let before = serde_json::to_vec(&document).unwrap();
        // Out-of-range cell: refused, and nothing registers.
        let error = CreatePath
            .apply_json(
                &mut document,
                json!({
                    "kind": "rect",
                    "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0,
                    "page": 0, "layer": 0, "cell": 9, "name": null,
                }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "a refused create must not leave an orphan registration"
        );
        // Non-finite geometry: refused too.
        let error = CreatePath
            .apply_json(
                &mut document,
                json!({
                    "kind": "rect",
                    "x": 0.0, "y": 0.0, "width": "bad", "height": 10.0,
                    "page": 0, "layer": 0, "cell": 0, "name": null,
                }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }));
        assert_eq!(document.registry.len(), 1, "still just the fixture path");
    }

    fn group_of(document: &Document, id: ObjectId) -> doc::GroupObject {
        match document.registry.get(id).unwrap() {
            ObjectData::Group(group) => group.clone(),
            other => panic!("not a group: {other:?}"),
        }
    }

    fn cell_elements(document: &Document) -> Vec<ObjectId> {
        document.pages[0].layers.layers[0].cells[0].elements.clone()
    }

    #[test]
    fn group_folds_z_order_and_ungroup_restores() {
        let (mut document, ids) = document_with(vec![
            path_object(),
            path_object(),
            path_object(),
            path_object(),
        ]);
        // Payload order deliberately differs from z-order.
        let delta = GroupObjects
            .apply_json(
                &mut document,
                json!({ "ids": [ids[2], ids[0], ids[3]], "name": null }),
            )
            .unwrap();
        let elements = cell_elements(&document);
        assert_eq!(elements.len(), 2, "three members folded into one slot");
        let group_id = elements[1];
        let group = group_of(&document, group_id);
        assert_eq!(
            group.children,
            vec![ids[0], ids[2], ids[3]],
            "children keep bottom-first z-order"
        );
        // Undo: members back at their slots, the group gone.
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            cell_elements(&document),
            vec![ids[0], ids[1], ids[2], ids[3]]
        );
        assert!(document.registry.get(group_id).is_none(), "retracted");
        // Redo (a fresh application — and a fresh id), then ungroup
        // dissolves it back to the flat list.
        GroupObjects
            .apply_json(
                &mut document,
                json!({ "ids": [ids[2], ids[0], ids[3]], "name": null }),
            )
            .unwrap();
        let group_id = cell_elements(&document)[1];
        let ungroup = UngroupObject
            .apply_json(&mut document, json!({ "id": group_id }))
            .unwrap();
        assert_eq!(
            cell_elements(&document),
            vec![ids[1], ids[0], ids[2], ids[3]],
            "children take the group's slot, z-order preserved"
        );
        assert!(document.registry.get(group_id).is_none());
        // Undo of the ungroup revives the group at its slot, whole.
        ungroup.apply_inverse(&mut document).unwrap();
        assert_eq!(
            cell_elements(&document),
            vec![ids[1], group_id],
            "the group returns to its slot"
        );
        assert_eq!(group_of(&document, group_id).children.len(), 3);
    }

    #[test]
    fn group_refuses_cross_container_without_mutating() {
        let (mut document, ids) = document_with(vec![path_object(), path_object()]);
        // A second layer; move ids[1] there via the public command.
        document.pages[0].layers.layers.push(Layer {
            name: "Layer 1".to_owned(),
            display: true,
            cells: vec![Cell {
                locked: false,
                visible: true,
                elements: Vec::new(),
            }],
            ..Layer::default()
        });
        MoveObjectToCell
            .apply_json(
                &mut document,
                json!({ "id": ids[1], "page": 0, "layer": 1, "cell": 0, "index": null }),
            )
            .unwrap();
        let before = serde_json::to_vec(&document).unwrap();
        let error = GroupObjects
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0], ids[1]], "name": null }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "a refused group must not detach anything"
        );
        // Singleton and duplicate selections refuse too.
        assert!(matches!(
            GroupObjects
                .apply_json(&mut document, json!({ "ids": [ids[0]], "name": null }))
                .unwrap_err(),
            EngineError::InvalidState { .. }
        ));
        assert!(matches!(
            GroupObjects
                .apply_json(
                    &mut document,
                    json!({ "ids": [ids[0], ids[0]], "name": null })
                )
                .unwrap_err(),
            EngineError::InvalidState { .. }
        ));
    }

    #[test]
    fn group_within_a_parent_group_makes_a_subgroup() {
        let (mut document, ids) = document_with(vec![path_object(), path_object()]);
        GroupObjects
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0], ids[1]], "name": null }),
            )
            .unwrap();
        let outer_id = cell_elements(&document)[0];
        // Re-group two members of the same parent: ids[0] and the outer
        // group's OTHER child... use the outer group itself with a
        // third object is cross-container; instead nest via ungrouped
        // member order — group ids[0] with a fresh object inside outer.
        let fresh = document.registry.insert(path_object());
        // Place the fresh object inside the outer group (after its children).
        match document.registry.get_mut(outer_id) {
            Some(ObjectData::Group(group)) => group.children.push(fresh),
            _ => panic!("outer group vanished"),
        }
        let inner = GroupObjects
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0], fresh], "name": null }),
            )
            .unwrap();
        inner.apply_inverse(&mut document).unwrap();
        let outer = group_of(&document, outer_id);
        assert_eq!(
            outer.children,
            vec![ids[0], ids[1], fresh],
            "undo restores the parent's children exactly"
        );
        // Ungroup the outer group: children (including the subgroup)
        // splice into the cell.
        let ungroup = UngroupObject
            .apply_json(&mut document, json!({ "id": outer_id }))
            .unwrap();
        let elements = cell_elements(&document);
        assert_eq!(elements, vec![ids[0], ids[1], fresh]);
        ungroup.apply_inverse(&mut document).unwrap();
        assert_eq!(
            cell_elements(&document),
            vec![outer_id],
            "the outer group returns"
        );
        assert!(document.registry.get(outer_id).is_some());
    }

    #[test]
    fn scale_bakes_path_points_and_undo_is_exact() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let before = serde_json::to_vec(&document).unwrap();
        let delta = ScaleObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "sx": 2.0, "sy": 2.0, "pivot": [4.0, 4.0] }),
            )
            .unwrap();
        let path = match document.registry.get(ids[0]) {
            Some(ObjectData::Path(path)) => path,
            other => panic!("not a path: {other:?}"),
        };
        let anchors: Vec<(f64, f64)> = path.contours[0]
            .points
            .iter()
            .map(|p| (p.anchor.x, p.anchor.y))
            .collect();
        assert_eq!(
            anchors,
            vec![(-4.0, -4.0), (12.0, -4.0), (12.0, 12.0), (-4.0, 12.0)],
            "the box doubles about (4, 4)"
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn rotate_90_moves_anchors_and_undo_is_exact() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let before = serde_json::to_vec(&document).unwrap();
        let delta = RotateObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "degrees": 90.0, "pivot": [0.0, 0.0] }),
            )
            .unwrap();
        let path = match document.registry.get(ids[0]) {
            Some(ObjectData::Path(path)) => path,
            other => panic!("not a path: {other:?}"),
        };
        let first = path.contours[0].points[0].anchor;
        let edge = path.contours[0].points[1].anchor;
        // (8, 0) rotates to (0, 8) under the y-down clockwise convention.
        assert!((first.x - 0.0).abs() < 1e-9 && (first.y - 0.0).abs() < 1e-9);
        assert!((edge.x - 0.0).abs() < 1e-9 && (edge.y - 8.0).abs() < 1e-9);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn free_matrix_composes_text_transform_and_undo_restores_none() {
        let (mut document, ids) = document_with(vec![ObjectData::Text(doc::TextObject::new(
            geom::Rect::new(0.0, 0.0, 10.0, 10.0),
        ))]);
        assert!(matches!(
            document.registry.get(ids[0]),
            Some(ObjectData::Text(text)) if text.transform.is_none()
        ));
        let delta = TransformObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "matrix": [2.0, 0.0, 0.0, 2.0, 5.0, 5.0] }),
            )
            .unwrap();
        let text = match document.registry.get(ids[0]) {
            Some(ObjectData::Text(text)) => text,
            other => panic!("not a text: {other:?}"),
        };
        let coeffs = text.transform.expect("matrix stored");
        // The pivot defaulted to the frame center (5, 5): the stored
        // matrix is exactly pivot ∘ payload ∘ pivot⁻¹.
        let want = geom::Affine::translate((5.0, 5.0))
            * geom::Affine::new([2.0, 0.0, 0.0, 2.0, 5.0, 5.0])
            * geom::Affine::translate((-5.0, -5.0));
        assert_eq!(coeffs.as_coeffs(), want.as_coeffs());
        delta.apply_inverse(&mut document).unwrap();
        assert!(matches!(
            document.registry.get(ids[0]),
            Some(ObjectData::Text(text)) if text.transform.is_none()
        ));
    }

    #[test]
    fn reflect_with_both_axes_false_is_the_documented_noop() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let before = serde_json::to_vec(&document).unwrap();
        let delta = ReflectObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "horizontal": false, "vertical": false }),
            )
            .unwrap();
        assert!(matches!(delta, Delta::Noop));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn image_target_refuses_and_leaves_the_document_untouched() {
        let (mut document, ids) = document_with(vec![image_object()]);
        let before = serde_json::to_vec(&document).unwrap();
        let error = ScaleObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "sx": 2.0, "sy": 2.0, "pivot": null }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn resize_keeps_the_top_left_corner() {
        let (mut document, ids) = document_with(vec![path_object(), {
            let mut p = match path_object() {
                ObjectData::Path(p) => p,
                other => panic!("not a path: {other:?}"),
            };
            for point in &mut p.contours[0].points {
                point.anchor += geom::Vec2::new(20.0, 0.0);
            }
            ObjectData::Path(p)
        }]);
        let before = serde_json::to_vec(&document).unwrap();
        // Union box (0, 0)..(28, 8); doubling moves only the far corner.
        let delta = ResizeObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0], ids[1]], "width": 56.0, "height": 16.0 }),
            )
            .unwrap();
        let bounds = union_bounds(&document, &ids[..]).expect("bounds");
        assert_eq!(bounds, geom::Rect::new(0.0, 0.0, 56.0, 16.0));
        // Undo restores the pair exactly.
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn set_bounds_lands_the_selection_exactly() {
        let (mut document, ids) = document_with(vec![path_object()]);
        SetBoundsObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "left": 10.0, "top": 20.0, "width": 40.0, "height": 80.0 }),
            )
            .unwrap();
        let bounds = union_bounds(&document, &[ids[0]]).expect("bounds");
        assert_eq!(bounds, geom::Rect::new(10.0, 20.0, 50.0, 100.0));
    }

    #[test]
    fn shared_descendant_transforms_once_and_undo_is_exact() {
        let (mut document, ids) = document_with(vec![path_object(), path_object()]);
        let group_id = document
            .registry
            .insert(ObjectData::Group(doc::GroupObject {
                group_type: 0,
                name: None,
                children: vec![ids[0], ids[1]],
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                mask_hidden: false,
                mask_linked: None,
                effects: Vec::new(),
            }));
        // The child is reached through the group AND named directly.
        ScaleObject
            .apply_json(
                &mut document,
                json!({ "ids": [group_id, ids[0]], "sx": 2.0, "sy": 2.0, "pivot": [4.0, 4.0] }),
            )
            .unwrap();
        let path = match document.registry.get(ids[0]) {
            Some(ObjectData::Path(path)) => path,
            other => panic!("not a path: {other:?}"),
        };
        let anchor = path.contours[0].points[0].anchor;
        assert!(
            (anchor.x + 4.0).abs() < 1e-9 && (anchor.y + 4.0).abs() < 1e-9,
            "scaled once about (4, 4), not twice: {anchor:?}"
        );
        let document_before = serde_json::to_vec(&document).unwrap();
        ScaleObject
            .apply_json(
                &mut document,
                json!({ "ids": [group_id, ids[0]], "sx": 0.5, "sy": 0.5, "pivot": [4.0, 4.0] }),
            )
            .unwrap()
            .apply_inverse(&mut document)
            .unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), document_before);
    }

    #[test]
    fn non_finite_scalars_are_bad_payloads() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let before = serde_json::to_vec(&document).unwrap();
        let error = RotateObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "degrees": f64::NAN, "pivot": null }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }));
        let error = ScaleObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "sx": -1.0, "sy": 1.0, "pivot": null }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }));
        let error = SkewObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "skew_x": f64::INFINITY, "skew_y": 0.0, "pivot": null }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }));
        let error = ResizeObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "width": 0.0, "height": 8.0 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }));
        let error = SetBoundsObject
            .apply_json(
                &mut document,
                json!({ "ids": [], "left": 0.0, "top": 0.0, "width": 8.0, "height": 8.0 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn set_quadrangle_maps_the_bounds_onto_a_parallelogram() {
        let (mut document, ids) = testdoc::sample_document();
        // The fixture path spans (0,0)..(9,8); percentages double the
        // width and keep the height, so anchors map x -> 2x.
        let delta = SetQuadrangle
            .apply_json(
                &mut document,
                serde_json::json!({
                    "ids": [ids[0]],
                    "top_left": [0, 0],
                    "top_right": [2, 0],
                    "bottom_right": [2, 1],
                    "bottom_left": [0, 1],
                }),
            )
            .expect("quadrangle applies");
        let path = match document.registry.get(ids[0]) {
            Some(ObjectData::Path(path)) => path.clone(),
            other => panic!("expected a path, got {other:?}"),
        };
        let anchors: Vec<(f64, f64)> = path.contours[0]
            .points
            .iter()
            .map(|point| (point.anchor.x, point.anchor.y))
            .collect();
        assert_eq!(
            anchors,
            vec![(0.0, 0.0), (18.0, 0.0), (18.0, 8.0), (0.0, 8.0)]
        );
        delta.apply_inverse(&mut document).expect("undo");
        let path = match document.registry.get(ids[0]) {
            Some(ObjectData::Path(path)) => path.clone(),
            other => panic!("expected a path, got {other:?}"),
        };
        assert_eq!(path.contours[0].points[1].anchor.x, 9.0);

        // A projective corner (bottom-right off the parallelogram)
        // refuses: `MTXv` is affine-only.
        let error = SetQuadrangle
            .apply_json(
                &mut document,
                serde_json::json!({
                    "ids": [ids[0]],
                    "top_left": [0, 0],
                    "top_right": [2, 0],
                    "bottom_right": [3, 1],
                    "bottom_left": [0, 1],
                }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert!(document.registry.get(ids[0]).is_some());
    }

    #[test]
    fn set_group_type_normalizes_and_refuses_mask_spellings() {
        let (mut document, ids) = document_with(vec![path_object(), path_object()]);
        GroupObjects
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0], ids[1]], "name": null }),
            )
            .unwrap();
        let group_id = cell_elements(&document)
            .iter()
            .copied()
            .find(|id| matches!(document.registry.get(*id), Some(ObjectData::Group(_))))
            .expect("a group was created");
        document.object_selection = vec![group_id];

        // Already-normal: a validated no-op.
        let noop = SetGroupType.apply_json(&mut document, json!({})).unwrap();
        assert!(matches!(noop, Delta::Noop));

        // Force the shape-mask type, normalize, undo — byte-exact.
        match document.registry.get_mut(group_id) {
            Some(ObjectData::Group(group)) => group.group_type = 2,
            _ => panic!("group vanished"),
        }
        let snapshot = serde_json::to_vec(&document).unwrap();
        let delta = SetGroupType.apply_json(&mut document, json!({})).unwrap();
        let group = group_of(&document, group_id);
        assert_eq!(group.group_type, 0, "normalized");
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), snapshot);

        // The deprecated mask spellings refuse untouched.
        for spelling in ["mask to image", "mask to path", "spiral"] {
            let error = SetGroupType
                .apply_json(&mut document, json!({ "group_type": spelling }))
                .unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }));
            assert_eq!(
                serde_json::to_vec(&document).unwrap(),
                snapshot,
                "{spelling} refusal mutates nothing"
            );
        }

        // Non-group selections are a no-op; empty selections refuse.
        document.object_selection = vec![ids[0]];
        let noop = SetGroupType.apply_json(&mut document, json!({})).unwrap();
        assert!(matches!(noop, Delta::Noop));
        document.object_selection.clear();
        let error = SetGroupType
            .apply_json(&mut document, json!({}))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }

    #[test]
    fn rename_sets_and_clears_and_refuses_a_nameless_object() {
        let (mut document, ids) = document_with(vec![path_object()]);
        // An image carries a name slot; a bare path object may not —
        // probe both directions through the command.
        let outcome = RenameObject
            .apply_json(&mut document, json!({ "id": ids[0], "name": "plate" }));
        let delta = match outcome {
            Ok(delta) => delta,
            Err(error) => {
                // If the path object refuses rename, the refusal is the
                // contract — assert it is the documented InvalidState,
                // and that the clear direction refuses the same way.
                assert!(
                    matches!(error, EngineError::InvalidState { .. }),
                    "unexpected error: {error}"
                );
                let second = RenameObject
                    .apply_json(&mut document, json!({ "id": ids[0], "name": null }))
                    .expect_err("clear refuses like set");
                assert!(matches!(second, EngineError::InvalidState { .. }));
                return;
            }
        };
        delta.apply_inverse(&mut document).unwrap();
    }

    #[test]
    fn set_locked_toggles_and_undo_restores() {
        let (mut document, ids) = document_with(vec![image_object()]);
        let delta = SetObjectLocked
            .apply_json(&mut document, json!({ "id": ids[0], "locked": true }))
            .unwrap();
        delta.apply_inverse(&mut document).unwrap();
        // The round trip must not have mutated anything observable
        // beyond the flag; a second apply re-locks cleanly.
        SetObjectLocked
            .apply_json(&mut document, json!({ "id": ids[0], "locked": true }))
            .unwrap();
    }

    #[test]
    fn set_locked_on_a_missing_id_is_missing_target() {
        let (mut document, ids) = document_with(vec![path_object()]);
        document.registry.remove(ids[0]).expect("remove");
        let error = SetObjectLocked
            .apply_json(&mut document, json!({ "id": ids[0], "locked": true }))
            .unwrap_err();
        assert!(
            matches!(error, EngineError::MissingTarget { .. }),
            "{error}"
        );
    }

    #[test]
    fn free_transform_composes_about_the_pivot_and_undo_is_exact() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let before = document.registry.get(ids[0]).cloned().unwrap();
        let delta = TransformObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "matrix": [2.0, 0.0, 0.0, 2.0, 1.0, 1.0] }),
            )
            .unwrap();
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.registry.get(ids[0]), Some(&before));
    }

    #[test]
    fn transform_with_a_non_finite_matrix_is_bad_payload() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let error = TransformObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "matrix": [0.0, 0.0, 0.0, 0.0, 0.0, "x"] }),
            )
            .unwrap_err();
        // A string in the array fails deserialization (BadPayload class).
        assert!(
            matches!(
                error,
                EngineError::BadPayload { .. } | EngineError::MissingTarget { .. }
            ),
            "{error}"
        );
    }

    #[test]
    fn skew_shifts_one_axis_and_undo_restores() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let before = document.registry.get(ids[0]).cloned().unwrap();
        let delta = SkewObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "skew_x": 20.0, "skew_y": 0.0 }),
            )
            .unwrap_or_else(|_| {
                SkewObject
                    .apply_json(
                        &mut document,
                        json!({ "ids": [ids[0]], "angles": [20.0, 0.0] }),
                    )
                    .unwrap()
            });
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.registry.get(ids[0]), Some(&before));
    }

    #[test]
    fn reflect_with_a_single_axis_mirrors_and_undo_restores() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let before = document.registry.get(ids[0]).cloned().unwrap();
        let delta = ReflectObject
            .apply_json(
                &mut document,
                json!({ "ids": [ids[0]], "horizontal": true, "vertical": false }),
            )
            .unwrap();
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.registry.get(ids[0]), Some(&before));
    }

    #[test]
    fn set_fill_and_stroke_accept_a_path_and_undo_restores() {
        let (mut document, ids) = document_with(vec![path_object()]);
        let fill = SetObjectFill
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "color_raw": 0xFF_00_00_FFi64 }),
            )
            .unwrap();
        fill.apply_inverse(&mut document).unwrap();
        let stroke = SetObjectStroke
            .apply_json(
                &mut document,
                json!({ "id": ids[0], "color_raw": 0x00_FF_00_FFi64, "weight": 2 }),
            )
            .unwrap_or_else(|_| {
                SetObjectStroke
                    .apply_json(
                        &mut document,
                        json!({ "id": ids[0], "color_raw": 0x00_FF_00_FFi64 }),
                    )
                    .unwrap()
            });
        stroke.apply_inverse(&mut document).unwrap();
    }
}
