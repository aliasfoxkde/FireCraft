//! Layer commands: add, delete, naming and display.

use doc::{Cell, Document, Layer, ObjectId, SharedLayerDef, SharedLayerLink};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the layer commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(AddLayer)?;
    registry.register(DeleteLayer)?;
    registry.register(RenameLayer)?;
    registry.register(SetLayerDisplay)?;
    registry.register(ReorderLayer)?;
    registry.register(SetLayerLocked)?;
    registry.register(SetLayerSharing)?;
    registry.register(ShareLayerToPages)?;
    registry.register(DetachSharedLayer)
}

/// `layer.add` — insert an empty layer (one unlocked, visible cell per
/// frame). Absent `name` gets the Fireworks-style `Layer {n+1}` from
/// the final position; absent `index` appends.
struct AddLayer;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddLayerPayload {
    /// Page index.
    page: usize,
    /// Name; absent derives from the final position.
    name: Option<String>,
    /// Insert position, bottom-first; absent appends.
    index: Option<usize>,
}

impl TypedCommand for AddLayer {
    type Payload = AddLayerPayload;

    fn name(&self) -> &'static str {
        "layer.add"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.add";
        let page = access::page_mut(document, NAME, payload.page)?;
        let len = page.layers.layers.len();
        let at = payload.index.unwrap_or(len);
        if at > len {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("insert index {at} beyond {len} layers"),
            });
        }
        // One unlocked, visible cell per frame: a new layer must be
        // editable in every state, mirroring `page.add`.
        let frame_count = page.info.frame_count.max(1);
        let cells = (0..frame_count)
            .map(|_| Cell {
                locked: false,
                visible: true,
                elements: Vec::new(),
            })
            .collect();
        let mut layer = Layer {
            cells,
            ..Layer::default()
        };
        layer.name = payload
            .name
            .clone()
            .unwrap_or_else(|| format!("Layer {}", at + 1));
        let before = page.layers.clone();
        page.layers.layers.insert(at, layer);
        Ok(Delta::Layers {
            page: payload.page,
            layers: before,
        })
    }
}

/// `layer.delete` — remove a layer and its elements' containment
/// (objects stay in the registry until the last reference is gone;
/// the registry generation bump invalidates their ids). The last
/// layer of a page cannot be deleted: a page with no layers has
/// nothing to edit.
struct DeleteLayer;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DeleteLayerPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
}

impl TypedCommand for DeleteLayer {
    type Payload = DeleteLayerPayload;

    fn name(&self) -> &'static str {
        "layer.delete"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.delete";
        let page = access::page_mut(document, NAME, payload.page)?;
        if page.layers.layers.len() <= 1 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "cannot delete a page's last layer".to_owned(),
            });
        }
        if payload.layer >= page.layers.layers.len() {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("layer {} absent on page {}", payload.layer, payload.page),
            });
        }
        // `MSLb` special layers (the Web Layer) block deletion —
        // CS6's behavior for its special layer.
        if page.layers.layers[payload.layer].is_special {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "a special layer (Web Layer) cannot be deleted".to_owned(),
            });
        }
        let before = page.layers.clone();
        page.layers.layers.remove(payload.layer);
        Ok(Delta::Layers {
            page: payload.page,
            layers: before,
        })
    }
}

/// `layer.rename` — set a layer's `LNMs` name.
struct RenameLayer;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RenameLayerPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
    /// New name (may be empty; Fireworks layers are rarely nameless).
    name: String,
}

impl TypedCommand for RenameLayer {
    type Payload = RenameLayerPayload;

    fn name(&self) -> &'static str {
        "layer.rename"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.rename";
        let target = access::layer_mut(document, NAME, payload.page, payload.layer)?;
        let before = target.clone();
        target.name = payload.name.clone();
        Ok(Delta::Layer {
            page: payload.page,
            layer: payload.layer,
            before: Box::new(before),
        })
    }
}

/// `layer.setDisplay` — set a layer's `DISb` display flag (display vs
/// smart-layer-reference display).
struct SetLayerDisplay;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetLayerDisplayPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
    /// New display flag.
    display: bool,
}

impl TypedCommand for SetLayerDisplay {
    type Payload = SetLayerDisplayPayload;

    fn name(&self) -> &'static str {
        "layer.setDisplay"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.setDisplay";
        let target = access::layer_mut(document, NAME, payload.page, payload.layer)?;
        let before = target.clone();
        target.display = payload.display;
        Ok(Delta::Layer {
            page: payload.page,
            layer: payload.layer,
            before: Box::new(before),
        })
    }
}

/// `layer.reorder` — CS6's `dom.reorderLayer(layerToMove,
/// layerToPutItBefore, bMakeCopy, posInLayer, aboveBelowInto)`:
/// moves (or copies) the layer at `from` to sit before `before`.
/// CS6's `posInLayer` and `aboveBelowInto = 2` (into-as-sub-layer)
/// are nesting operations on a flat list — refused (decision 0021).
/// A move onto its own current slot is CS6's no-op.
struct ReorderLayer;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ReorderLayerPayload {
    /// Page index.
    page: usize,
    /// Layer to move or copy, bottom-first.
    from: usize,
    /// Insert before this layer index (may equal the layer count for
    /// "move to top").
    before: usize,
    /// Copy instead of move.
    make_copy: Option<bool>,
}

impl TypedCommand for ReorderLayer {
    type Payload = ReorderLayerPayload;

    fn name(&self) -> &'static str {
        "layer.reorder"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.reorder";
        let make_copy = payload.make_copy.unwrap_or(false);
        let len = access::page(document, NAME, payload.page)?
            .layers
            .layers
            .len();
        if payload.from >= len {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("layer {} out of range ({len} layers)", payload.from),
            });
        }
        if payload.before > len {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("insert-before {} beyond {} layers", payload.before, len),
            });
        }
        if !make_copy && (payload.before == payload.from || payload.before == payload.from + 1) {
            return Ok(Delta::Noop);
        }
        // For a copy, validate every cell's closure before minting
        // anything: a refusal must leave no fresh slots (the
        // frame.duplicate rule).
        let source =
            access::page(document, NAME, payload.page)?.layers.layers[payload.from].clone();
        let mut fresh_slots: Vec<ObjectId> = Vec::new();
        let copied = if make_copy {
            for cell in &source.cells {
                for id in &cell.elements {
                    super::cell::check_closure(document, NAME, *id, 0)?;
                }
            }
            let mut cells = Vec::with_capacity(source.cells.len());
            for cell in &source.cells {
                let mut remap = std::collections::HashMap::new();
                cells.push(doc::Cell {
                    locked: cell.locked,
                    visible: cell.visible,
                    elements: cell
                        .elements
                        .iter()
                        .map(|id| {
                            super::cell::clone_element(document, *id, &mut remap, &mut fresh_slots)
                        })
                        .collect(),
                });
            }
            let mut layer = source.clone();
            layer.cells = cells;
            Some(layer)
        } else {
            None
        };
        let page = access::page_mut(document, NAME, payload.page)?;
        let before_list = page.layers.clone();
        if let Some(copied) = copied {
            // The original stays at `from`; the fresh clone slots in
            // before `before`.
            page.layers.layers.insert(payload.before, copied);
        } else {
            let moved = page.layers.layers.remove(payload.from);
            let at = if payload.before > payload.from {
                payload.before - 1
            } else {
                payload.before
            };
            page.layers.layers.insert(at, moved);
        }
        // List capture first, then one Created per fresh slot in mint
        // order: Group undoes last-captured-first, which is exactly
        // the LIFO retraction the mint order requires.
        let mut deltas = vec![Delta::Layers {
            page: payload.page,
            layers: before_list,
        }];
        deltas.extend(fresh_slots.into_iter().map(|id| Delta::Object {
            id,
            inverse: ObjectInverse::Created,
        }));
        Ok(if deltas.len() == 1 {
            deltas.pop().unwrap_or(Delta::Noop)
        } else {
            Delta::Group(deltas)
        })
    }
}

/// `layer.setLocked` — CS6's `dom.setLayerLocked(layerIndex,
/// frameIndex, bLock, bAllLayers)`: the lock is per frame — exactly
/// the `LCKb` inside `CELv` (`Cell.locked`). `all_layers` locks every
/// layer's cell for the frame and ignores `layer`, as CS6's
/// `bAllLayers` ignores `layerIndex`.
struct SetLayerLocked;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetLayerLockedPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first (ignored when `all_layers`).
    layer: usize,
    /// Frame (state) index.
    frame: usize,
    /// New lock flag.
    locked: bool,
    /// Lock every layer's cell for `frame`.
    all_layers: Option<bool>,
}

impl TypedCommand for SetLayerLocked {
    type Payload = SetLayerLockedPayload;

    fn name(&self) -> &'static str {
        "layer.setLocked"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.setLocked";
        let all_layers = payload.all_layers.unwrap_or(false);
        let page = access::page(document, NAME, payload.page)?;
        let targets: Vec<usize> = if all_layers {
            (0..page.layers.layers.len()).collect()
        } else {
            if payload.layer >= page.layers.layers.len() {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "layer {} out of range ({} layers)",
                        payload.layer,
                        page.layers.layers.len()
                    ),
                });
            }
            vec![payload.layer]
        };
        for index in &targets {
            let cells = &page.layers.layers[*index].cells;
            if payload.frame >= cells.len() {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "layer {} carries {} cells, frame {} is out of range",
                        index,
                        cells.len(),
                        payload.frame
                    ),
                });
            }
        }
        if all_layers {
            let before = page.layers.clone();
            let page = access::page_mut(document, NAME, payload.page)?;
            for layer in &mut page.layers.layers {
                layer.cells[payload.frame].locked = payload.locked;
            }
            Ok(Delta::Layers {
                page: payload.page,
                layers: before,
            })
        } else {
            let target = access::layer_mut(document, NAME, payload.page, payload.layer)?;
            let before = target.cells[payload.frame].clone();
            target.cells[payload.frame].locked = payload.locked;
            Ok(Delta::Cell {
                page: payload.page,
                layer: payload.layer,
                cell: payload.frame,
                before: Box::new(before),
            })
        }
    }
}

/// Mints a collision-checked UUID-shaped `UIDs` for a new shared-layer
/// definition (decision 0022): a fixed prefix plus a counter, skipped
/// past every uid already present in the document.
fn mint_shared_uid(document: &Document) -> String {
    let taken: std::collections::BTreeSet<String> = document
        .shared_layer_defs
        .iter()
        .map(|def| def.uid.clone())
        .chain(
            document
                .pages
                .iter()
                .flat_map(|page| page.layers.layers.iter())
                .filter_map(|layer| layer.shared_link.as_ref())
                .map(|link| link.uid.clone()),
        )
        .collect();
    let mut n = 1u64;
    loop {
        let uid = format!("fc000000-0000-4000-8000-{n:012}");
        if !taken.contains(&uid) {
            return uid;
        }
        n += 1;
    }
}

/// The definition's cells expanded to a page's frame span: repeated
/// cyclically to `frame_count` (an unshare or detach gives the page
/// the shared content on every state), never fewer than one cell.
fn filled_cells(definition: &Layer, frame_count: usize) -> Vec<Cell> {
    if definition.cells.is_empty() {
        return vec![Cell::default(); frame_count.max(1)];
    }
    (0..frame_count.max(1))
        .map(|i| definition.cells[i % definition.cells.len()].clone())
        .collect()
}

/// `layer.setSharing` — CS6's `dom.setLayerSharing(layerIndex,
/// sharedStatus, bUnshareCopiesToAllFrames, bWarnUser)`. `shared=true`
/// converts the layer document-wide: one `LSEv` is minted from the
/// layer's cells, the slot becomes an `LSAv` link, and every other
/// page gains a link at the same index (clamped) — CS6 shares to all
/// pages. `shared=false` unshares everywhere: each page's link becomes
/// a real layer cloned from the definition (`unshare_copies_to_all_
/// frames` fills the page's frame span; without it content lands on
/// frame 0 only, CS6's "placed only on the current frame") and the
/// definition is removed. `bWarnUser` is UI-only and dropped. The
/// special layer (Web Layer) cannot be shared.
struct SetLayerSharing;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetLayerSharingPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
    /// CS6's `sharedStatus`: `true` is "shared", `false` is
    /// "not shared".
    shared: bool,
    /// CS6's `bUnshareCopiesToAllFrames`, honored on unshare.
    unshare_copies_to_all_frames: Option<bool>,
}

impl TypedCommand for SetLayerSharing {
    type Payload = SetLayerSharingPayload;

    fn name(&self) -> &'static str {
        "layer.setSharing"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.setSharing";
        let owner = access::page(document, NAME, payload.page)?;
        let slot =
            owner
                .layers
                .layers
                .get(payload.layer)
                .ok_or_else(|| EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "layer {} out of range ({} layers on page {})",
                        payload.layer,
                        owner.layers.layers.len(),
                        payload.page
                    ),
                })?;
        if slot.is_special {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the special layer (Web Layer) cannot be shared".to_owned(),
            });
        }
        if slot.shared_link.is_some() == payload.shared {
            // Already in the requested state (a link is "shared").
            return Ok(Delta::Noop);
        }
        if !payload.shared {
            // Unshare everywhere: every link to this uid becomes a
            // real layer; the definition goes away.
            let uid = slot.shared_link.as_ref().map(|link| link.uid.clone());
            let Some(uid) = uid else {
                return Ok(Delta::Noop);
            };
            let Some(definition) = document
                .shared_layer_defs
                .iter()
                .find(|def| def.uid == uid)
                .map(|def| def.layer.clone())
            else {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!("shared layer {uid} has no definition"),
                });
            };
            let defs_before = document.shared_layer_defs.clone();
            let mut deltas = vec![Delta::SharedLayers {
                before: defs_before,
            }];
            for page_index in 0..document.pages.len() {
                let all_frames = payload.unshare_copies_to_all_frames.unwrap_or(false);
                let page = access::page_mut(document, NAME, page_index)?;
                let before = page.layers.clone();
                let mut replaced = false;
                for slot in &mut page.layers.layers {
                    if slot
                        .shared_link
                        .as_ref()
                        .map(|link| link.uid == uid)
                        .unwrap_or(false)
                    {
                        let frame_count = page.info.frame_count.max(0) as usize;
                        let mut fresh = definition.clone();
                        fresh.cells = if all_frames {
                            filled_cells(&definition, frame_count)
                        } else {
                            let mut cells = vec![Cell::default(); frame_count.max(1)];
                            if let Some(first) = definition.cells.first() {
                                cells[0] = first.clone();
                            }
                            cells
                        };
                        fresh.shared_link = None;
                        *slot = fresh;
                        replaced = true;
                    }
                }
                if replaced {
                    deltas.push(Delta::Layers {
                        page: page_index,
                        layers: before,
                    });
                }
            }
            document.shared_layer_defs.retain(|def| def.uid != uid);
            return Ok(Delta::Group(deltas));
        }
        // Share: mint the definition from the layer, link every page.
        let source = owner.layers.layers[payload.layer].clone();
        let uid = mint_shared_uid(document);
        let defs_before = document.shared_layer_defs.clone();
        let mut deltas = vec![Delta::SharedLayers {
            before: defs_before,
        }];
        for page_index in 0..document.pages.len() {
            let at = payload.layer.min(
                access::page(document, NAME, page_index)?
                    .layers
                    .layers
                    .len(),
            );
            let page = access::page_mut(document, NAME, page_index)?;
            let before = page.layers.clone();
            if page_index == payload.page {
                // The owner slot becomes the link; the content moved
                // into the definition.
                page.layers.layers[at] = Layer {
                    shared_link: Some(SharedLayerLink {
                        uid: uid.clone(),
                        is_primary: false,
                    }),
                    ..Layer::default()
                };
            } else {
                let already = page.layers.layers.iter().any(|slot| {
                    slot.shared_link
                        .as_ref()
                        .map(|link| link.uid == uid)
                        .unwrap_or(false)
                });
                if already {
                    continue;
                }
                page.layers.layers.insert(
                    at,
                    Layer {
                        shared_link: Some(SharedLayerLink {
                            uid: uid.clone(),
                            is_primary: false,
                        }),
                        ..Layer::default()
                    },
                );
            }
            deltas.push(Delta::Layers {
                page: page_index,
                layers: before,
            });
        }
        document.shared_layer_defs.push(SharedLayerDef {
            uid,
            is_primary: true,
            layer: source,
        });
        Ok(Delta::Group(deltas))
    }
}

/// `layer.shareToPages` — CS6's `dom.shareLayerToPages(layerNum,
/// addToPages, deleteFromPages)`: adds or removes a shared layer's
/// link on specific pages. CS6 names pages by comma-separated name
/// strings (UI-era); the engine is index-based everywhere, so the
/// payload takes page indices (decision 0022). Adds are idempotent,
/// deletions of pages without the link change nothing — a command
/// that alters no list is a `Noop`.
struct ShareLayerToPages;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ShareLayerToPagesPayload {
    /// Page whose layer list carries the link (`layer` resolves there).
    page: usize,
    /// Layer index of the link on `page`.
    layer: usize,
    /// Pages gaining the link.
    add_to_pages: Option<Vec<usize>>,
    /// Pages losing the link.
    delete_from_pages: Option<Vec<usize>>,
}

impl TypedCommand for ShareLayerToPages {
    type Payload = ShareLayerToPagesPayload;

    fn name(&self) -> &'static str {
        "layer.shareToPages"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.shareToPages";
        let slot = access::page(document, NAME, payload.page)?
            .layers
            .layers
            .get(payload.layer)
            .ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "layer {} of page {} is not a shared-layer link",
                    payload.layer, payload.page
                ),
            })?;
        let uid = slot
            .shared_link
            .as_ref()
            .map(|link| link.uid.clone())
            .ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the target layer is not a shared-layer link".to_owned(),
            })?;
        if !document.shared_layer_defs.iter().any(|def| def.uid == uid) {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("shared layer {uid} has no definition"),
            });
        }
        let mut deltas: Vec<Delta> = Vec::new();
        for page_index in payload.add_to_pages.iter().flatten() {
            let count = access::page(document, NAME, *page_index)?
                .layers
                .layers
                .len();
            let at = payload.layer.min(count);
            let page = access::page_mut(document, NAME, *page_index)?;
            let before = page.layers.clone();
            let already = page.layers.layers.iter().any(|slot| {
                slot.shared_link
                    .as_ref()
                    .map(|link| link.uid == uid)
                    .unwrap_or(false)
            });
            if already {
                continue;
            }
            page.layers.layers.insert(
                at,
                Layer {
                    shared_link: Some(SharedLayerLink {
                        uid: uid.clone(),
                        is_primary: false,
                    }),
                    ..Layer::default()
                },
            );
            deltas.push(Delta::Layers {
                page: *page_index,
                layers: before,
            });
        }
        for page_index in payload.delete_from_pages.iter().flatten() {
            let Some(slot) = access::page(document, NAME, *page_index)?
                .layers
                .layers
                .iter()
                .position(|slot| {
                    slot.shared_link
                        .as_ref()
                        .map(|link| link.uid == uid)
                        .unwrap_or(false)
                })
            else {
                continue;
            };
            let page = access::page_mut(document, NAME, *page_index)?;
            let before = page.layers.clone();
            page.layers.layers.remove(slot);
            deltas.push(Delta::Layers {
                page: *page_index,
                layers: before,
            });
        }
        Ok(match deltas.len() {
            0 => Delta::Noop,
            1 => deltas.pop().unwrap_or(Delta::Noop),
            _ => Delta::Group(deltas),
        })
    }
}

/// `layer.detachShared` — CS6's `dom.detachSharedLayer(layerNum,
/// pageNum)`: one page's link becomes that page's private copy of the
/// definition (frame span filled); the definition and every other
/// page's link stay. CS6's sub-layer auto-detach is moot — `LYLv` is
/// flat (decision 0021).
struct DetachSharedLayer;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DetachSharedLayerPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
}

impl TypedCommand for DetachSharedLayer {
    type Payload = DetachSharedLayerPayload;

    fn name(&self) -> &'static str {
        "layer.detachShared"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "layer.detachShared";
        let slot = access::page(document, NAME, payload.page)?
            .layers
            .layers
            .get(payload.layer)
            .ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "layer {} of page {} is not a shared-layer link",
                    payload.layer, payload.page
                ),
            })?;
        let uid = slot
            .shared_link
            .as_ref()
            .map(|link| link.uid.clone())
            .ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the target layer is not a shared-layer link".to_owned(),
            })?;
        let Some(definition) = document
            .shared_layer_defs
            .iter()
            .find(|def| def.uid == uid)
            .map(|def| def.layer.clone())
        else {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("shared layer {uid} has no definition"),
            });
        };
        let frame_count = access::page(document, NAME, payload.page)?
            .info
            .frame_count
            .max(0) as usize;
        let target = access::layer_mut(document, NAME, payload.page, payload.layer)?;
        let mut fresh = definition.clone();
        fresh.cells = filled_cells(&definition, frame_count);
        fresh.shared_link = None;
        let before = target.clone();
        *target = fresh;
        Ok(Delta::Layer {
            page: payload.page,
            layer: payload.layer,
            before: Box::new(before),
        })
    }
}

#[cfg(test)]
mod sharing_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::super::testdoc;
    use super::*;
    use crate::command::Command;
    use serde_json::json;

    #[test]
    fn set_sharing_links_every_page_and_undoes() {
        let (mut document, ids) = testdoc::sample_document();
        let before = serde_json::to_vec(&document).unwrap();
        let delta = SetLayerSharing
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "shared": true }),
            )
            .unwrap();
        assert_eq!(document.shared_layer_defs.len(), 1);
        let uid = document.shared_layer_defs[0].uid.clone();
        let definition = &document.shared_layer_defs[0];
        assert!(definition.is_primary);
        assert_eq!(definition.layer.cells[0].elements, ids[..2.min(ids.len())]);
        // The owner slot became the link; the other page gained one at
        // the same index.
        let owner = &document.pages[0].layers.layers[0];
        assert_eq!(
            owner.shared_link.as_ref().map(|l| l.uid.as_str()),
            Some(uid.as_str())
        );
        assert!(owner.cells.is_empty(), "a link carries no content");
        let other = &document.pages[1].layers.layers[0];
        assert_eq!(
            other.shared_link.as_ref().map(|l| l.uid.as_str()),
            Some(uid.as_str())
        );
        assert_eq!(document.pages[1].layers.layers.len(), 3);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "undo removes the definition and every link"
        );
    }

    #[test]
    fn set_sharing_noops_and_refuses() {
        let (mut document, _) = testdoc::sample_document();
        // Sharing an already-normal layer off is a no-op.
        let noop = SetLayerSharing
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "shared": false }),
            )
            .unwrap();
        assert!(matches!(noop, Delta::Noop));
        SetLayerSharing
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "shared": true }),
            )
            .unwrap();
        // Sharing an already-shared layer is a no-op.
        let noop = SetLayerSharing
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "shared": true }),
            )
            .unwrap();
        assert!(matches!(noop, Delta::Noop));
        // The special layer cannot be shared.
        document.pages[0].layers.layers[1].is_special = true;
        let error = SetLayerSharing
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 1, "shared": true }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }

    #[test]
    fn unshare_restores_real_layers_and_undoes() {
        let (mut document, _) = testdoc::sample_document();
        SetLayerSharing
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "shared": true }),
            )
            .unwrap();
        let after_share = serde_json::to_vec(&document).unwrap();
        let expected = document.shared_layer_defs[0].layer.cells.clone();
        let delta = SetLayerSharing
            .apply_json(
                &mut document,
                json!({
                    "page": 0,
                    "layer": 0,
                    "shared": false,
                    "unshare_copies_to_all_frames": true,
                }),
            )
            .unwrap();
        assert!(document.shared_layer_defs.is_empty());
        for page in &document.pages {
            let slot = &page.layers.layers[0];
            assert!(slot.shared_link.is_none(), "every page holds a real layer");
            assert_eq!(
                slot.cells, expected,
                "cyclic fill of the definition's cells"
            );
        }
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), after_share);

        // Without the flag, content lands on frame 0 only (CS6's
        // "placed only on the current frame").
        SetLayerSharing
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "shared": true }),
            )
            .unwrap();
        SetLayerSharing
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "shared": false }),
            )
            .unwrap();
        for page in &document.pages {
            let slot = &page.layers.layers[0];
            assert!(!slot.cells[0].elements.is_empty());
            assert!(slot.cells[1].elements.is_empty());
        }
    }

    #[test]
    fn share_to_pages_adds_and_removes_links() {
        let (mut document, _) = testdoc::sample_document();
        SetLayerSharing
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "shared": true }),
            )
            .unwrap();
        // Drop page 1's link.
        ShareLayerToPages
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "delete_from_pages": [1] }),
            )
            .unwrap();
        assert_eq!(document.pages[1].layers.layers.len(), 2);
        assert!(
            document.pages[1]
                .layers
                .layers
                .iter()
                .all(|l| l.shared_link.is_none())
        );
        assert_eq!(document.shared_layer_defs.len(), 1, "definition stays");
        // Dropping it again changes nothing.
        let noop = ShareLayerToPages
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "delete_from_pages": [1] }),
            )
            .unwrap();
        assert!(matches!(noop, Delta::Noop));
        // Bring it back; a repeated add is also a no-op.
        ShareLayerToPages
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "add_to_pages": [1] }),
            )
            .unwrap();
        assert_eq!(document.pages[1].layers.layers.len(), 3);
        let noop = ShareLayerToPages
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "add_to_pages": [1] }),
            )
            .unwrap();
        assert!(matches!(noop, Delta::Noop));
        // A non-link target refuses.
        let error = ShareLayerToPages
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 1, "add_to_pages": [1] }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }

    #[test]
    fn detach_gives_the_page_its_own_copy() {
        let (mut document, _) = testdoc::sample_document();
        SetLayerSharing
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "shared": true }),
            )
            .unwrap();
        let delta = DetachSharedLayer
            .apply_json(&mut document, json!({ "page": 1, "layer": 0 }))
            .unwrap();
        // Page 1 now carries a private copy; the definition and page
        // 0's link stay.
        let slot = &document.pages[1].layers.layers[0];
        assert!(slot.shared_link.is_none());
        assert_eq!(slot.cells.len(), 2);
        assert_eq!(
            slot.cells[0].elements,
            document.shared_layer_defs[0].layer.cells[0].elements
        );
        assert!(document.pages[0].layers.layers[0].shared_link.is_some());
        assert_eq!(document.shared_layer_defs.len(), 1);
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.pages[1].layers.layers[0].shared_link.is_some());
        // A non-link refuses.
        let error = DetachSharedLayer
            .apply_json(&mut document, json!({ "page": 1, "layer": 1 }))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::super::testdoc;
    use super::*;
    use crate::command::Command;
    use serde_json::json;

    #[test]
    fn rename_round_trips() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 0, "layer": 1, "name": "Foreground" });
        let delta = RenameLayer.apply_json(&mut document, payload).unwrap();
        assert_eq!(document.pages[0].layers.layers[1].name, "Foreground");
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].layers.layers[1].name, "Layer 1");
    }

    #[test]
    fn set_display_round_trips() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 1, "layer": 0, "display": false });
        let delta = SetLayerDisplay.apply_json(&mut document, payload).unwrap();
        assert!(!document.pages[1].layers.layers[0].display);
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.pages[1].layers.layers[0].display);
    }

    #[test]
    fn out_of_range_layer_is_missing_target() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 0, "layer": 7, "display": true });
        let error = SetLayerDisplay
            .apply_json(&mut document, payload)
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
    }
}

#[cfg(test)]
mod reorder_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::super::testdoc;
    use super::*;
    use crate::command::Command;
    use serde_json::json;

    #[test]
    fn reorder_moves_and_undoes() {
        let (mut document, _) = testdoc::sample_document();
        let first = document.pages[0].layers.layers[0].name.clone();
        let delta = ReorderLayer
            .apply_json(&mut document, json!({ "page": 0, "from": 0, "before": 2 }))
            .unwrap();
        let layers = &document.pages[0].layers.layers;
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[1].name, first, "moved layer sits last");
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].layers.layers[0].name, first);

        // Own-slot and next-slot moves are CS6's no-op.
        let noop = ReorderLayer
            .apply_json(&mut document, json!({ "page": 0, "from": 1, "before": 2 }))
            .unwrap();
        assert!(matches!(noop, Delta::Noop));
    }

    #[test]
    fn reorder_copy_mints_objects_and_undo_retracts_them() {
        let (mut document, ids) = testdoc::sample_document();
        let before = serde_json::to_vec(&document).unwrap();
        let delta = ReorderLayer
            .apply_json(
                &mut document,
                json!({
                    "page": 0,
                    "from": 0,
                    "before": 2,
                    "make_copy": true,
                }),
            )
            .unwrap();
        let layers = &document.pages[0].layers.layers;
        assert_eq!(layers.len(), 3);
        let original = &layers[0].cells[0].elements;
        let copy = &layers[2].cells[0].elements;
        assert_eq!(original.len(), copy.len());
        assert!(
            original.iter().zip(copy.iter()).all(|(a, b)| a != b),
            "copies live in fresh slots"
        );
        assert!(copy.iter().all(|id| document.registry.get(*id).is_some()));
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].layers.layers.len(), 2);
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
        assert!(ids.iter().all(|id| document.registry.get(*id).is_some()));
    }

    #[test]
    fn set_locked_targets_one_cell_and_undoes() {
        let (mut document, _) = testdoc::sample_document();
        let delta = SetLayerLocked
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 1, "frame": 0, "locked": true }),
            )
            .unwrap();
        assert!(document.pages[0].layers.layers[1].cells[0].locked);
        assert!(
            !document.pages[0].layers.layers[1].cells[1].locked,
            "per-frame lock"
        );
        delta.apply_inverse(&mut document).unwrap();
        assert!(!document.pages[0].layers.layers[1].cells[0].locked);

        let error = SetLayerLocked
            .apply_json(
                &mut document,
                json!({ "page": 0, "layer": 0, "frame": 9, "locked": true }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }

    #[test]
    fn set_locked_all_layers_sweeps_the_frame() {
        let (mut document, _) = testdoc::sample_document();
        let delta = SetLayerLocked
            .apply_json(
                &mut document,
                json!({
                    "page": 0,
                    "layer": 0,
                    "frame": 1,
                    "locked": true,
                    "all_layers": true,
                }),
            )
            .unwrap();
        for layer in &document.pages[0].layers.layers {
            assert!(layer.cells[1].locked);
            assert!(!layer.cells[0].locked);
        }
        delta.apply_inverse(&mut document).unwrap();
        assert!(!document.pages[0].layers.layers[0].cells[1].locked);
        assert!(!document.pages[0].layers.layers[1].cells[1].locked);
    }
}
