//! The document clipboard (decision 0039): CS6's `dom.clipCopy`
//! family — copy, cut, paste, and the paste-as-mask trio.
//!
//! The clipboard is document-level editing state (`Document.clipboard`,
//! the `selection`/`jpeg_mask` class — never file content) holding
//! deep object clones. Captures are subtree-closed: a group is stored
//! before its children, paste re-mints bottom-up and rewrites child
//! id lists through the minted-id map. Tile data rides by reference;
//! the fresh-mint law keeps shared bank tiles pristine while the
//! clipboard holds them.

use std::collections::HashMap;

use doc::{Clipboard, Document, GroupObject, ImageObject, ObjectData, ObjectId};

use super::mask::{GRAYSCALE_MASK, SHAPE_MASK, attach_at, owning_mask_group};
use super::object::{attach_cell, detach, find_owner};
use crate::access;
use crate::bitmap::{assemble_rgba, retile_rgba};
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the clipboard commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(Copy)?;
    registry.register(Cut)?;
    registry.register(Paste)?;
    registry.register(PasteAsMask)?;
    registry.register(PasteInside)?;
    registry.register(PasteFromChannelToChannel)
}

/// The clipboard as it was before a command (the whole state rides the
/// capture, mirroring `Delta::SavedSelections`).
fn clipboard_delta(before: Option<Clipboard>) -> Delta {
    Delta::Clipboard {
        before: before.map(Box::new),
    }
}

/// Deep-captures `id` into `out`: the object first, then its group
/// children's subtrees. Index-aligned `ids` records the source ids —
/// the old→new keys for paste's rewrite.
fn capture_into(document: &Document, id: ObjectId, out: &mut Clipboard) {
    match document.registry.get(id) {
        None => {} // stale (an ancestor in the same selection was cut first)
        Some(ObjectData::Group(group)) => {
            out.elements.push(ObjectData::Group(group.clone()));
            out.ids.push(id);
            for child in group.children.clone() {
                capture_into(document, child, out);
            }
        }
        Some(other) => {
            out.elements.push(other.clone());
            out.ids.push(id);
        }
    }
}

/// Captures an id list as a clipboard.
fn capture_selection_with(selection: &[ObjectId], document: &Document) -> Clipboard {
    let mut clip = Clipboard {
        elements: Vec::new(),
        ids: Vec::new(),
        roots: Vec::new(),
    };
    for id in selection {
        clip.roots.push(clip.elements.len());
        capture_into(document, *id, &mut clip);
    }
    clip
}

/// Captures the whole object selection as a clipboard.
fn capture_selection(document: &Document) -> Clipboard {
    capture_selection_with(&document.object_selection, document)
}

/// The clipboard's single top-level element (the paste-mask and
/// channel commands' operand).
fn single_root<'a>(
    document: &'a Document,
    name: &'static str,
) -> Result<&'a ObjectData, EngineError> {
    let clip = document
        .clipboard
        .as_ref()
        .ok_or_else(|| EngineError::InvalidState {
            name: name.to_owned(),
            detail: "the clipboard is empty".to_owned(),
        })?;
    if clip.roots.len() != 1 {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!(
                "the clipboard carries {} elements; exactly one is required",
                clip.roots.len()
            ),
        });
    }
    let index = clip.roots[0];
    // A root is always its own element slot.
    Ok(&clip.elements[index])
}

/// `clipboard.copy` — CS6's `dom.clipCopy()`: deep-copies the object
/// selection into the document clipboard. Groups carry their whole
/// subtree; the selection itself is untouched.
struct Copy;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct CopyPayload {}

impl TypedCommand for Copy {
    type Payload = CopyPayload;

    fn name(&self) -> &'static str {
        "clipboard.copy"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "clipboard.copy";
        if document.object_selection.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "there is no selection to copy".to_owned(),
            });
        }
        // Capture before mutating (the standing law).
        let before = document.clipboard.clone();
        document.clipboard = Some(capture_selection(document));
        Ok(clipboard_delta(before))
    }
}

/// `clipboard.cut` — CS6's `dom.clipCut()`: copies the selection to
/// the clipboard, then deletes it through the `object.delete`
/// machinery. The selection clears (CS6's Edit > Cut posture).
struct Cut;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct CutPayload {}

impl TypedCommand for Cut {
    type Payload = CutPayload;

    fn name(&self) -> &'static str {
        "clipboard.cut"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "clipboard.cut";
        if document.object_selection.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "there is no selection to cut".to_owned(),
            });
        }
        // Capture before mutating (the standing law): the clipboard
        // and the selection both.
        let clip_before = document.clipboard.clone();
        let selection = std::mem::take(&mut document.object_selection);
        document.clipboard = Some(capture_selection_with(&selection, document));
        let mut deltas = vec![
            clipboard_delta(clip_before),
            Delta::ObjectSelection {
                before: selection.clone(),
            },
        ];
        for id in &selection {
            // An ancestor cut earlier in this same loop already
            // removed the subtree — a stale id rides along.
            let Some(object) = document.registry.get(*id).cloned() else {
                continue;
            };
            let container = detach(document, NAME, *id)?;
            if document.registry.remove(*id).is_none() {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "object {} (generation {}) vanished between capture and removal",
                        id.index(),
                        id.generation()
                    ),
                });
            }
            deltas.push(container);
            deltas.push(Delta::Object {
                id: *id,
                inverse: ObjectInverse::Removed {
                    object: Box::new(object),
                },
            });
        }
        Ok(Delta::Group(deltas))
    }
}

/// Re-mints a clipboard element list bottom-up (children were captured
/// after their parents, so the reverse walk has every child id mapped
/// by the time its parent's children list is rewritten).
fn mint_all(
    document: &mut Document,
    name: &'static str,
) -> Result<(Vec<ObjectId>, HashMap<ObjectId, ObjectId>), EngineError> {
    let clip = document
        .clipboard
        .as_ref()
        .ok_or_else(|| EngineError::InvalidState {
            name: name.to_owned(),
            detail: "the clipboard is empty".to_owned(),
        })?;
    let mut map: HashMap<ObjectId, ObjectId> = HashMap::new();
    let mut minted = Vec::with_capacity(clip.elements.len());
    for index in (0..clip.elements.len()).rev() {
        let mut data = clip.elements[index].clone();
        if let ObjectData::Group(group) = &mut data {
            for child in &mut group.children {
                if let Some(fresh) = map.get(child) {
                    *child = *fresh;
                }
            }
        }
        let fresh = document.registry.insert(data);
        map.insert(clip.ids[index], fresh);
        minted.push(fresh);
    }
    // Chronological mint order: the Created inverses push in this
    // order, so the LIFO undo retracts the newest mint first (the
    // registry's retract_insert rule).
    Ok((minted, map))
}

/// `clipboard.paste` — CS6's `dom.clipPaste()`: re-mints the clipboard
/// into the named cell, top-level items in capture order, world
/// positions carried (paste-in-place). Explicit addressing per
/// decision 0037's deviation 2; the resolution-mismatch and
/// mask-edit-mode options are shell concerns (decision 0039's
/// deviations 1/3).
struct Paste;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PastePayload {
    /// Placement.
    page: usize,
    layer: usize,
    cell: usize,
}

impl TypedCommand for Paste {
    type Payload = PastePayload;

    fn name(&self) -> &'static str {
        "clipboard.paste"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "clipboard.paste";
        let clip_empty = match document.clipboard.as_ref() {
            None => true,
            Some(clip) => clip.roots.is_empty(),
        };
        if clip_empty {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the clipboard is empty".to_owned(),
            });
        }
        // Pre-flight the target cell before minting anything (the
        // refused-calls-mutate-nothing law).
        access::cell(document, NAME, payload.page, payload.layer, payload.cell)?;
        let before = document.clipboard.clone();
        let clip = document
            .clipboard
            .as_ref()
            .ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the clipboard vanished between check and mint".to_owned(),
            })?;
        let roots: Vec<ObjectId> = clip.roots.iter().map(|&r| clip.ids[r]).collect();
        let (minted, map) = mint_all(document, NAME)?;
        let mut deltas = vec![clipboard_delta(before)];
        for old in roots {
            let Some(fresh) = map.get(&old) else {
                return Err(EngineError::DeltaMismatch {
                    detail: "a clipboard root was not minted".to_owned(),
                });
            };
            deltas.push(attach_cell(
                document,
                NAME,
                payload.page,
                payload.layer,
                payload.cell,
                *fresh,
                None,
            )?);
        }
        for id in minted {
            deltas.push(Delta::Object {
                id,
                inverse: ObjectInverse::Created,
            });
        }
        Ok(Delta::Group(deltas))
    }
}

/// The pasteAsMask core (decision 0039): the clipboard's single
/// element becomes the mask of the one selected element. Shared by
/// `clipboard.pasteAsMask` and the retired `clipboard.pasteInside`.
fn paste_as_mask_core(
    document: &mut Document,
    name: &'static str,
    mask_type: Option<&str>,
    mask_replace_options: Option<&str>,
) -> Result<Delta, EngineError> {
    // CS6 throws unless exactly one element is selected — kept.
    if document.object_selection.len() != 1 {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "exactly one element must be selected".to_owned(),
        });
    }
    let target = document.object_selection[0];
    let mask_data = single_root(document, name)?.clone();
    // "ask" is a dialog — refused. "image" against a path has no
    // rasterizer hook yet (deviation 4). The kind dictates the group
    // type; the CS6 rule "a single image is pasted as an image mask
    // even if you pass vector" falls out of the kind check.
    match mask_type {
        Some("ask") => {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: "mask_type \"ask\" is a shell dialog".to_owned(),
            });
        }
        None | Some("image") | Some("vector") => {}
        Some(other) => {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("mask_type {other:?} is not one of image/vector/ask"),
            });
        }
    }
    let group_type = match (&mask_data, mask_type) {
        (ObjectData::Image(_), _) => GRAYSCALE_MASK,
        (ObjectData::Path(_), Some("image")) => {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: "mask_type \"image\" needs a rasterizer hook (not wired)".to_owned(),
            });
        }
        (ObjectData::Path(_), _) => SHAPE_MASK,
        _ => {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: "the clipboard element is not mask-kind (image or path)".to_owned(),
            });
        }
    };
    let replace = match mask_replace_options {
        None | Some("replace") => true,
        Some("add") => {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: "mask_replace_options \"add\" (mask union) is recorded debt".to_owned(),
            });
        }
        Some("ask") => {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: "mask_replace_options \"ask\" is a shell dialog".to_owned(),
            });
        }
        Some(other) => {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("mask_replace_options {other:?} is not one of replace/add/ask"),
            });
        }
    };

    // Replace-into-existing swaps the mask child; otherwise a fresh
    // mask group takes the target's owner slot.
    let existing = owning_mask_group(document, name, target).ok();
    if let Some(old) = existing {
        if !replace {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: "the target already carries a mask".to_owned(),
            });
        }
        let Some(removed_mask_id) = old.group.children.first().copied() else {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: "the mask group carries no mask child".to_owned(),
            });
        };
        // Mint the new mask first, then rewrite the group in place.
        let fresh = document.registry.insert(mask_data);
        let Some(ObjectData::Group(group)) = document.registry.get_mut(old.id) else {
            return Err(EngineError::MissingTarget {
                name: name.to_owned(),
                detail: format!(
                    "mask group {} (generation {}) vanished between capture and rewrite",
                    old.id.index(),
                    old.id.generation()
                ),
            });
        };
        group.children[0] = fresh;
        let Some(removed_mask) = document.registry.remove(removed_mask_id) else {
            return Err(EngineError::MissingTarget {
                name: name.to_owned(),
                detail: "the replaced mask is not live".to_owned(),
            });
        };
        return Ok(Delta::Group(vec![
            Delta::Object {
                id: old.id,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(ObjectData::Group(old.group)),
                },
            },
            Delta::Object {
                id: fresh,
                inverse: ObjectInverse::Created,
            },
            Delta::Object {
                id: removed_mask_id,
                inverse: ObjectInverse::Removed {
                    object: Box::new(removed_mask),
                },
            },
        ]));
    }

    let owner = find_owner(document, target).ok_or_else(|| EngineError::MissingTarget {
        name: name.to_owned(),
        detail: format!(
            "object {} (generation {}) is referenced by no cell or group",
            target.index(),
            target.generation()
        ),
    })?;
    // Detach first: its errors fire before anything mutates.
    let container = detach(document, name, target)?;
    let mask_id = document.registry.insert(mask_data);
    let group_id = document.registry.insert(ObjectData::Group(GroupObject {
        group_type,
        name: None,
        children: vec![mask_id, target],
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        mask_linked: None,
        effects: Vec::new(),
    }));
    let mut deltas = vec![container, attach_at(document, name, owner, group_id)?];
    deltas.push(Delta::Object {
        id: mask_id,
        inverse: ObjectInverse::Created,
    });
    deltas.push(Delta::Object {
        id: group_id,
        inverse: ObjectInverse::Created,
    });
    Ok(Delta::Group(deltas))
}

/// `clipboard.pasteAsMask` — CS6's `dom.clipPasteAsMask()`: the
/// clipboard contents paste as the selected element's mask. Exactly
/// one selected element and a single-element clipboard are required
/// (CS6 throws the same way).
struct PasteAsMask;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PasteAsMaskPayload {
    /// "image" / "vector" / "ask" — the kind the clipboard element
    /// already dictates wins where they disagree (CS6: an image
    /// clipboard pastes as an image mask even when "vector").
    #[serde(default)]
    mask_type: Option<String>,
    /// "replace" (default) / "add" / "ask" — behavior when the target
    /// already carries a mask; "add" is recorded debt, "ask" is a
    /// shell dialog.
    #[serde(default)]
    mask_replace_options: Option<String>,
}

impl TypedCommand for PasteAsMask {
    type Payload = PasteAsMaskPayload;

    fn name(&self) -> &'static str {
        "clipboard.pasteAsMask"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        paste_as_mask_core(
            document,
            "clipboard.pasteAsMask",
            payload.mask_type.as_deref(),
            payload.mask_replace_options.as_deref(),
        )
    }
}

/// `clipboard.pasteInside` — CS6's `dom.clipPasteInside()`, retired in
/// Fireworks 4 in favor of `clipPasteAsMask`: the same operation with
/// every option defaulted.
struct PasteInside;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PasteInsidePayload {}

impl TypedCommand for PasteInside {
    type Payload = PasteInsidePayload;

    fn name(&self) -> &'static str {
        "clipboard.pasteInside"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        paste_as_mask_core(document, "clipboard.pasteInside", None, None)
    }
}

/// The RGBA channel a channel-paste names.
fn channel_of(name: &'static str, value: &str) -> Result<usize, EngineError> {
    match value {
        "red" => Ok(0),
        "green" => Ok(1),
        "blue" => Ok(2),
        "alpha" => Ok(3),
        other => Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("channel {other:?} is not one of red/green/blue/alpha"),
        }),
    }
}

/// An image's pixel extent as `usize`s (assembled planes index by it;
/// `assemble_rgba` validates addressability before any pixel flows).
fn dims_of(image: &ImageObject, name: &'static str) -> Result<(usize, usize), EngineError> {
    let (Ok(width), Ok(height)) = (
        usize::try_from(image.pixels.0.max(0)),
        usize::try_from(image.pixels.1.max(0)),
    ) else {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("image pixels {:?} are not addressable", image.pixels),
        });
    };
    Ok((width, height))
}

/// `clipboard.pasteFromChannelToChannel` — CS6's
/// `dom.clipPasteFromChannelToChannel()`: pastes a clipboard image's
/// channel into the selected image's channel — or, without a single
/// bitmap selection, into a fresh opaque grayscale image.
struct PasteFromChannelToChannel;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PasteFromChannelToChannelPayload {
    /// The clipboard channel to read.
    from_channel: String,
    /// The destination channel.
    to_channel: String,
    /// Placement of the minted grayscale image (required only when no
    /// single image is selected).
    #[serde(default)]
    page: Option<usize>,
    #[serde(default)]
    layer: Option<usize>,
    #[serde(default)]
    cell: Option<usize>,
}

impl TypedCommand for PasteFromChannelToChannel {
    type Payload = PasteFromChannelToChannelPayload;

    fn name(&self) -> &'static str {
        "clipboard.pasteFromChannelToChannel"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "clipboard.pasteFromChannelToChannel";
        let from = channel_of(NAME, &payload.from_channel)?;
        let to = channel_of(NAME, &payload.to_channel)?;
        let clip_image = match single_root(document, NAME)? {
            ObjectData::Image(image) => image.clone(),
            _ => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: "the clipboard element is not a bitmap image".to_owned(),
                });
            }
        };
        let plane = assemble_rgba(document, &clip_image, NAME)?;
        let (clip_w, clip_h) = dims_of(&clip_image, NAME)?;
        let channel_byte = |x: usize, y: usize| -> u8 { plane[(y * clip_w + x) * 4 + from] };

        // Exactly one selected image: overlay its `to` channel.
        let selected: Option<ObjectId> = match document.object_selection.as_slice() {
            [only] => Some(*only),
            _ => None,
        };
        if let Some(only) = selected {
            let before = access::object(document, NAME, only)?.clone();
            if let ObjectData::Image(target) = &before {
                let mut pixels = assemble_rgba(document, target, NAME)?;
                let (w, h) = dims_of(target, NAME)?;
                for y in 0..h.min(clip_h) {
                    for x in 0..w.min(clip_w) {
                        pixels[(y * w + x) * 4 + to] = channel_byte(x, y);
                    }
                }
                let (tiles, minted) = retile_rgba(document, &pixels, w as i64, h as i64);
                let Some(ObjectData::Image(slot)) = document.registry.get_mut(only) else {
                    return Err(EngineError::DeltaMismatch {
                        detail: "the target image vanished between capture and write".to_owned(),
                    });
                };
                slot.tiles = tiles;
                return Ok(Delta::Group(vec![
                    Delta::Object {
                        id: only,
                        inverse: ObjectInverse::Mutated {
                            object: Box::new(before),
                        },
                    },
                    Delta::Bank { added: minted },
                ]));
            }
        }

        // No single bitmap selected: a fresh opaque grayscale image.
        let (Some(page), Some(layer), Some(cell)) = (payload.page, payload.layer, payload.cell)
        else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "no single image is selected; page/layer/cell are required".to_owned(),
            });
        };
        // Pre-flight the placement cell before minting anything.
        access::cell(document, NAME, page, layer, cell)?;
        let mut pixels = vec![0u8; clip_w * clip_h * 4];
        for index in 0..(clip_w * clip_h) {
            let byte = plane[index * 4 + from];
            pixels[index * 4] = byte;
            pixels[index * 4 + 1] = byte;
            pixels[index * 4 + 2] = byte;
            pixels[index * 4 + 3] = 255;
        }
        let (tiles, minted) = retile_rgba(document, &pixels, clip_w as i64, clip_h as i64);
        let id = document.registry.insert(ObjectData::Image(ImageObject {
            location: clip_image.location,
            offset: clip_image.offset,
            pixels: clip_image.pixels,
            tile_size: clip_image.tile_size,
            tiles,
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }));
        let placed = attach_cell(document, NAME, page, layer, cell, id, None)?;
        Ok(Delta::Group(vec![
            placed,
            Delta::Object {
                id,
                inverse: ObjectInverse::Created,
            },
            Delta::Bank { added: minted },
        ]))
    }
}

#[cfg(test)]
mod clipboard_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use serde_json::json;

    use super::*;
    use crate::catalog::testdoc;
    use crate::command::Command;

    /// Mints a `w×h` image with deterministic pixel bytes and returns
    /// its id (tiles are already in the bank).
    fn image_at(document: &mut Document, w: i64, h: i64, seed: u8) -> ObjectId {
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for (index, byte) in pixels.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_add(seed);
        }
        let (tiles, _minted) = crate::bitmap::retile_rgba(document, &pixels, w, h);
        document.registry.insert(ObjectData::Image(ImageObject {
            location: geom::Point::ZERO,
            offset: geom::Point::ZERO,
            pixels: (w, h),
            tile_size: i64::from(raster::TILE_SIDE as u32),
            tiles,
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }))
    }

    #[test]
    fn copy_paste_round_trips_and_undoes_byte_exactly() {
        let (mut document, ids) = testdoc::sample_document();
        document.object_selection = vec![ids[0], ids[1]];
        let snapshot = serde_json::to_vec(&document).unwrap();

        let copy_delta = Copy
            .apply_json(&mut document, json!({}))
            .expect("copy applies");
        let clip = document.clipboard.as_ref().expect("clipboard set");
        assert_eq!(clip.roots, vec![0, 1]);
        assert_eq!(clip.elements.len(), 2, "paths carry no children");

        let delta = Paste
            .apply_json(&mut document, json!({"page": 0, "layer": 0, "cell": 1}))
            .expect("paste applies");
        let pasted = document.pages[0].layers.layers[0].cells[1].elements.clone();
        assert_eq!(pasted.len(), 2);
        for fresh in &pasted {
            assert!(!ids.contains(fresh), "paste mints fresh ids");
        }
        delta.apply_inverse(&mut document).expect("undo paste");
        assert!(
            document.pages[0].layers.layers[0].cells[1]
                .elements
                .is_empty()
        );
        assert!(document.clipboard.is_some(), "paste keeps the clipboard");

        // Undoing the copy empties the clipboard and returns the
        // byte-exact start.
        copy_delta.apply_inverse(&mut document).expect("undo copy");
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            snapshot,
            "both undos return the byte-exact start"
        );

        // Copying nothing is a refusal that mutates nothing.
        document.object_selection.clear();
        let before = serde_json::to_vec(&document).unwrap();
        let err = Copy
            .apply_json(&mut document, json!({}))
            .expect_err("empty refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn cut_removes_populates_clipboard_clears_selection_and_undoes() {
        let (mut document, ids) = testdoc::sample_document();
        document.object_selection = vec![ids[0], ids[1]];
        let snapshot = serde_json::to_vec(&document).unwrap();

        let delta = Cut
            .apply_json(&mut document, json!({}))
            .expect("cut applies");
        assert!(
            document.pages[0].layers.layers[0].cells[0]
                .elements
                .is_empty()
        );
        assert!(document.object_selection.is_empty());
        let clip = document.clipboard.as_ref().expect("clipboard set");
        assert_eq!(clip.roots, vec![0, 1]);
        delta.apply_inverse(&mut document).expect("undo cut");
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            snapshot,
            "undo restores objects, selection, and clipboard"
        );

        // The undo also restored the pre-cut (empty) clipboard: cut
        // again, then paste — the material revives under fresh ids.
        document.object_selection = vec![ids[0], ids[1]];
        Cut.apply_json(&mut document, json!({}))
            .expect("re-cut applies");
        Paste
            .apply_json(&mut document, json!({"page": 0, "layer": 1, "cell": 0}))
            .expect("paste applies");
        let pasted = document.pages[0].layers.layers[1].cells[0].elements.clone();
        assert_eq!(
            pasted.len(),
            4,
            "the fixture's two paths plus the two pasted"
        );
        assert!(!ids.contains(&pasted[2]) && !ids.contains(&pasted[3]));

        // Cutting nothing is a refusal.
        document.object_selection.clear();
        let err = Cut
            .apply_json(&mut document, json!({}))
            .expect_err("empty refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));
    }

    #[test]
    fn group_capture_is_subtree_closed_and_paste_rewrites_children() {
        let (mut document, ids) = testdoc::sample_document();
        let group_id = document.registry.insert(ObjectData::Group(GroupObject {
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
        document.object_selection = vec![group_id];

        Copy.apply_json(&mut document, json!({}))
            .expect("copy group");
        let clip = document.clipboard.as_ref().expect("clipboard set");
        assert_eq!(clip.roots, vec![0]);
        assert_eq!(clip.elements.len(), 3, "group plus two children");

        Paste
            .apply_json(&mut document, json!({"page": 0, "layer": 1, "cell": 1}))
            .expect("paste group");
        let pasted = document.pages[0].layers.layers[1].cells[1].elements.clone();
        let [fresh_group] = pasted.as_slice() else {
            panic!("one pasted root")
        };
        let Some(ObjectData::Group(group)) = document.registry.get(*fresh_group) else {
            panic!("pasted root is a group")
        };
        assert_eq!(group.children.len(), 2);
        for child in &group.children {
            assert!(
                document.registry.get(*child).is_some(),
                "each child was reminted live"
            );
            assert_ne!(ids[0], *child);
            assert_ne!(ids[1], *child);
        }
        // The reminted children differ from each other, too.
        assert_ne!(group.children[0], group.children[1]);
    }

    #[test]
    fn paste_as_mask_builds_the_mask_group_and_refuses_bad_states() {
        let (mut document, ids) = testdoc::sample_document();
        // Copy path ids[0]; mask ids[1] with it.
        document.object_selection = vec![ids[0]];
        Copy.apply_json(&mut document, json!({})).expect("copy");
        document.object_selection = vec![ids[1]];
        let snapshot = serde_json::to_vec(&document).unwrap();
        let delta = PasteAsMask
            .apply_json(&mut document, json!({}))
            .expect("pasteAsMask applies");
        let elements = document.pages[0].layers.layers[0].cells[0].elements.clone();
        assert_eq!(
            elements,
            vec![ids[0], elements[1]],
            "the group takes the target's slot"
        );
        let Some(ObjectData::Group(group)) = document.registry.get(elements[1]) else {
            panic!("the slot holds a mask group")
        };
        assert_eq!(group.group_type, 2, "a path mask is a shape mask");
        assert_eq!(group.children[1], ids[1], "the target is the content child");
        assert_ne!(group.children[0], ids[0], "the mask is a fresh mint");
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(serde_json::to_vec(&document).unwrap(), snapshot);
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![ids[0], ids[1]],
            "undo restores the flat pair"
        );

        // Two selected: the CS6 throw.
        document.object_selection = vec![ids[1], ids[2]];
        let err = PasteAsMask
            .apply_json(&mut document, json!({}))
            .expect_err("two targets refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));

        // No clipboard: refused (pasteInside shares the core).
        document.object_selection = vec![ids[1]];
        document.clipboard = None;
        let err = PasteInside
            .apply_json(&mut document, json!({}))
            .expect_err("empty clipboard refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));

        // Non-mask-kind clipboard: refused.
        let path = document.registry.get(ids[0]).cloned().expect("path");
        let text_id = document
            .registry
            .insert(ObjectData::Text(doc::TextObject::new(geom::Rect::new(
                0.0, 0.0, 8.0, 8.0,
            ))));
        let text = document.registry.get(text_id).cloned().expect("text");
        document.clipboard = Some(Clipboard {
            elements: vec![text],
            ids: vec![text_id],
            roots: vec![0],
        });
        let err = PasteAsMask
            .apply_json(&mut document, json!({}))
            .expect_err("text clipboard refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));

        // Bogus vocabulary: refused, clipboard still the path.
        document.clipboard = Some(Clipboard {
            elements: vec![path],
            ids: vec![ids[0]],
            roots: vec![0],
        });
        let before = serde_json::to_vec(&document).unwrap();
        for payload in [
            json!({"mask_type": "ask"}),
            json!({"mask_type": "sideways"}),
            json!({"mask_replace_options": "add"}),
            json!({"mask_replace_options": "ask"}),
            json!({"mask_replace_options": "maybe"}),
        ] {
            assert!(
                PasteAsMask
                    .apply_json(&mut document, payload.clone())
                    .is_err(),
                "bad options refused: {payload}"
            );
        }
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn paste_as_mask_replaces_an_existing_mask() {
        let (mut document, ids) = testdoc::sample_document();
        // A mask group over ids[1] with a uniform mask image.
        let (tiles, _minted) =
            crate::bitmap::retile_rgba(&mut document, &vec![255u8; 64 * 4], 8, 8);
        let mask_id = document.registry.insert(ObjectData::Image(ImageObject {
            location: geom::Point::ZERO,
            offset: geom::Point::ZERO,
            pixels: (8, 8),
            tile_size: i64::from(raster::TILE_SIDE as u32),
            tiles,
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }));
        let group_id = document.registry.insert(ObjectData::Group(GroupObject {
            group_type: 1,
            name: None,
            children: vec![mask_id, ids[1]],
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            mask_hidden: false,
            mask_linked: None,
            effects: Vec::new(),
        }));
        document.pages[0].layers.layers[0].cells[0].elements = vec![ids[0], group_id];
        document.object_selection = vec![ids[1]];
        // The clipboard's path becomes the group's new mask child.
        let path = document.registry.get(ids[0]).cloned().expect("path");
        document.clipboard = Some(Clipboard {
            elements: vec![path],
            ids: vec![ids[0]],
            roots: vec![0],
        });
        let snapshot = serde_json::to_vec(&document).unwrap();
        let delta = PasteAsMask
            .apply_json(&mut document, json!({"mask_replace_options": "replace"}))
            .expect("replace applies");
        let Some(ObjectData::Group(group)) = document.registry.get(group_id) else {
            panic!("group survives")
        };
        assert_eq!(group.children[1], ids[1], "content child kept");
        assert_ne!(group.children[0], mask_id, "mask child swapped");
        assert!(document.registry.get(mask_id).is_none(), "old mask removed");
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(serde_json::to_vec(&document).unwrap(), snapshot);
        assert!(document.registry.get(mask_id).is_some(), "old mask revived");
    }

    #[test]
    fn channel_paste_overlays_a_selected_image() {
        let (mut document, _ids) = testdoc::sample_document();
        // Clipboard image: byte pattern seed 7; target: seed 100.
        let source = image_at(&mut document, 4, 2, 7);
        let target = image_at(&mut document, 4, 2, 100);
        document.clipboard = Some(Clipboard {
            elements: vec![document.registry.get(source).cloned().expect("image")],
            ids: vec![source],
            roots: vec![0],
        });
        document.object_selection = vec![target];
        let snapshot = serde_json::to_vec(&document).unwrap();

        let delta = PasteFromChannelToChannel
            .apply_json(
                &mut document,
                json!({"from_channel": "green", "to_channel": "alpha"}),
            )
            .expect("channel paste applies");
        let assembled = crate::bitmap::assemble_rgba(
            &document,
            match document.registry.get(target) {
                Some(ObjectData::Image(image)) => image,
                _ => panic!("target is an image"),
            },
            "test",
        )
        .expect("assembled");
        let source_plane =
            crate::bitmap::assemble_rgba(&document, &image_of_test(&document, source), "test")
                .expect("source assembled");
        for index in 0..8 {
            assert_eq!(
                assembled[index * 4 + 3],
                source_plane[index * 4 + 1],
                "target alpha took the clipboard green"
            );
        }
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(serde_json::to_vec(&document).unwrap(), snapshot);

        // Unknown channels refuse.
        for payload in [
            json!({"from_channel": "cyan", "to_channel": "alpha"}),
            json!({"from_channel": "red", "to_channel": "bogus"}),
        ] {
            assert!(
                PasteFromChannelToChannel
                    .apply_json(&mut document, payload)
                    .is_err(),
                "bad channel refused"
            );
        }
    }

    #[test]
    fn channel_paste_mints_a_grayscale_image_without_a_selection() {
        let (mut document, _ids) = testdoc::sample_document();
        let source = image_at(&mut document, 4, 2, 7);
        document.clipboard = Some(Clipboard {
            elements: vec![document.registry.get(source).cloned().expect("image")],
            ids: vec![source],
            roots: vec![0],
        });
        let snapshot = serde_json::to_vec(&document).unwrap();

        let delta = PasteFromChannelToChannel
            .apply_json(
                &mut document,
                json!({
                    "from_channel": "red",
                    "to_channel": "blue",
                    "page": 0, "layer": 0, "cell": 1,
                }),
            )
            .expect("grayscale mint applies");
        let pasted = document.pages[0].layers.layers[0].cells[1].elements.clone();
        let [fresh] = pasted.as_slice() else {
            panic!("one minted image")
        };
        let plane =
            crate::bitmap::assemble_rgba(&document, &image_of_test(&document, *fresh), "test")
                .expect("assembled");
        for index in 0..8 {
            let gray = plane[index * 4];
            assert_eq!(plane[index * 4 + 1], gray, "channels equal (grayscale)");
            assert_eq!(plane[index * 4 + 2], gray);
            assert_eq!(plane[index * 4 + 3], 255, "opaque");
        }
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(serde_json::to_vec(&document).unwrap(), snapshot);

        // Without a selection OR placement: refused.
        let err = PasteFromChannelToChannel
            .apply_json(
                &mut document,
                json!({"from_channel": "red", "to_channel": "blue"}),
            )
            .expect_err("placement required");
        assert!(matches!(err, EngineError::InvalidState { .. }));
    }

    /// A clone of the named image object (test helper).
    fn image_of_test(document: &Document, id: ObjectId) -> ImageObject {
        match document.registry.get(id) {
            Some(ObjectData::Image(image)) => image.clone(),
            _ => panic!("object {id:?} is an image"),
        }
    }
}
