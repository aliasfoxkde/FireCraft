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
    registry.register(CreatePath)
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

/// `object.duplicate` — clones an object into a fresh registry slot and
/// inserts the copy into the source's own container, directly above it
/// (z-order is list order, bottom-first). CS6's Edit > Clone places the
/// copy on top of the original at the same spot; a following drag moves
/// it. Works in cells and in groups.
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
        let data = access::object(document, NAME, payload.id)?.clone();
        let owner = find_owner(document, payload.id).ok_or_else(|| EngineError::MissingTarget {
            name: NAME.to_owned(),
            detail: format!(
                "object {} (generation {}) is referenced by no cell or group",
                payload.id.index(),
                payload.id.generation()
            ),
        })?;
        let copy = document.registry.insert(data);
        match owner {
            Owner::Cell {
                page,
                layer,
                cell,
                position,
            } => {
                let placed =
                    attach_cell(document, NAME, page, layer, cell, copy, Some(position + 1))?;
                Ok(Delta::Group(vec![
                    placed,
                    Delta::Object {
                        id: copy,
                        inverse: ObjectInverse::Created,
                    },
                ]))
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
                Ok(Delta::Group(vec![
                    Delta::Object {
                        id: parent,
                        inverse: ObjectInverse::Mutated {
                            object: Box::new(ObjectData::Group(before)),
                        },
                    },
                    Delta::Object {
                        id: copy,
                        inverse: ObjectInverse::Created,
                    },
                ]))
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
fn translate_object(
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
                stroke_raw: 0xFF00_0000, // transparent black — no stroke yet
                fill_raw: 0xFF00_0000,   // transparent black — no fill yet
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
    use crate::command::Command;
    use doc::{
        Cell, Contour, GroupObject, ImageObject, Layer, LayerList, Page, PageInfo, PathObject,
        PathPoint, SliceObject, TileRef,
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
        let group_id = document.registry.insert(ObjectData::Group(GroupObject {
            group_type: 0,
            name: None,
            children: vec![ids[1]],
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            mask_hidden: false,
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
        let group_id = document.registry.insert(ObjectData::Group(GroupObject {
            group_type: 0,
            name: None,
            children: vec![ids[0], ids[1]],
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            mask_hidden: false,
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
}
