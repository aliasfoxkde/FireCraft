//! Shared-field mutation for the object kinds that carry them.
//!
//! `Path`, `Group`, and `Image` all carry the display quartet
//! (`VISb`/`LCKb`/`OPAi`/`BLDi`); `Path`, `Group`, `Image`, and `Slice`
//! all carry the `OBNs` name. One helper per field family keeps the
//! per-kind matches in one place instead of four commands each.

use doc::{Document, ObjectData, ObjectId};

/// The display fields every `Path`, `Group`, and `Image` carries.
pub(crate) struct DisplayFields<'a> {
    /// `VISb`.
    pub(crate) visible: &'a mut bool,
    /// `LCKb`.
    pub(crate) locked: &'a mut bool,
    /// `OPAi` — per mille.
    pub(crate) opacity: &'a mut i64,
    /// `BLDi` — blend mode index.
    pub(crate) blend: &'a mut i64,
}

/// Runs `edit` against the display fields of the object `id` names.
/// `None` when the id is stale or names a kind without the quartet
/// (`Text`, `Slice`, `SymbolInstance`).
pub(crate) fn with_display<R>(
    document: &mut Document,
    id: ObjectId,
    edit: impl FnOnce(&mut DisplayFields<'_>) -> R,
) -> Option<R> {
    match document.registry.get_mut(id)? {
        ObjectData::Path(path) => Some(edit(&mut DisplayFields {
            visible: &mut path.visible,
            locked: &mut path.locked,
            opacity: &mut path.opacity,
            blend: &mut path.blend,
        })),
        ObjectData::Group(group) => Some(edit(&mut DisplayFields {
            visible: &mut group.visible,
            locked: &mut group.locked,
            opacity: &mut group.opacity,
            blend: &mut group.blend,
        })),
        ObjectData::Image(image) => Some(edit(&mut DisplayFields {
            visible: &mut image.visible,
            locked: &mut image.locked,
            opacity: &mut image.opacity,
            blend: &mut image.blend,
        })),
        _ => None,
    }
}

/// Runs `edit` against the `OBNs` name of the object `id` names.
/// `None` when the id is stale or names a kind without a name field
/// (`Text`, `SymbolInstance`).
pub(crate) fn with_name<R>(
    document: &mut Document,
    id: ObjectId,
    edit: impl FnOnce(&mut Option<String>) -> R,
) -> Option<R> {
    match document.registry.get_mut(id)? {
        ObjectData::Path(path) => Some(edit(&mut path.name)),
        ObjectData::Group(group) => Some(edit(&mut group.name)),
        ObjectData::Image(image) => Some(edit(&mut image.name)),
        ObjectData::Slice(slice) => Some(edit(&mut slice.name)),
        _ => None,
    }
}
