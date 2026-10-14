//! Cell commands: one state's element-list flags, plus the frame
//! (state) management family — add, delete, duplicate, reorder,
//! distribute.

use doc::{Cell, Document, ObjectData, ObjectId};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the cell commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(SetCellVisible)?;
    registry.register(SetCellLocked)?;
    registry.register(AddFrames)?;
    registry.register(DeleteFrames)?;
    registry.register(DuplicateFrame)?;
    registry.register(ReorderFrame)?;
    registry.register(DistributeLayerToFrames)
}

/// Where a frame lands, using CS6's verbatim `where` strings. The
/// current-relative forms take the caller's active frame (`current`) —
/// the engine has no active frame of its own.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum FrameLocation {
    Beginning,
    #[serde(rename = "before current")]
    BeforeCurrent,
    #[serde(rename = "after current")]
    AfterCurrent,
    End,
}

/// Resolves a location to an insertion index against `count` frames.
fn insert_at(
    command: &'static str,
    location: FrameLocation,
    current: Option<usize>,
    count: usize,
) -> Result<usize, EngineError> {
    let current = match location {
        FrameLocation::Beginning => return Ok(0),
        FrameLocation::End => return Ok(count),
        FrameLocation::BeforeCurrent | FrameLocation::AfterCurrent => {
            current.ok_or_else(|| EngineError::InvalidState {
                name: command.to_owned(),
                detail: "a current-relative location needs the caller's active frame".to_owned(),
            })?
        }
    };
    if current >= count {
        return Err(EngineError::InvalidState {
            name: command.to_owned(),
            detail: format!("active frame {current} out of range ({count} frames)"),
        });
    }
    Ok(match location {
        FrameLocation::BeforeCurrent => current,
        _ => current + 1,
    })
}

/// A CS6 `howMany` count: at least 1, at most 4096 (one less than the
/// canvas pixel bound — no real document mints more).
fn how_many(command: &'static str, how_many: i64) -> Result<usize, EngineError> {
    if !(1..=4096).contains(&how_many) {
        return Err(EngineError::InvalidState {
            name: command.to_owned(),
            detail: format!("howMany {how_many} outside 1..=4096"),
        });
    }
    Ok(how_many as usize)
}

/// The frame count of a page (the `FRCi` pair is kept equal by every
/// frame command).
fn frame_count(document: &Document, page: usize) -> usize {
    document.pages[page].info.frame_count.max(0) as usize
}

/// The `FNDi` Frame Duration default the GIF exporter documents.
pub(crate) const DEFAULT_FRAME_DURATION: i64 = 10;

/// Map key for the duplicate-frame remap table — `ObjectId` is `Hash`
/// but not `Ord` (see `doc::library` for the same idiom).
type IdKey = (usize, u32);

/// Walks an element closure checking every id is live, with a nesting
/// ceiling mirroring `symbol.rs`'s (a cycle would otherwise hang the
/// walk). Run before any mint so a refused duplicate leaves nothing
/// behind.
pub(crate) fn check_closure(
    document: &Document,
    command: &'static str,
    id: ObjectId,
    depth: usize,
) -> Result<(), EngineError> {
    if depth > 64 {
        return Err(EngineError::InvalidState {
            name: command.to_owned(),
            detail: format!("object nesting deeper than 64 levels at {}", id.index()),
        });
    }
    match document.registry.get(id) {
        Some(ObjectData::Group(group)) => {
            for child in &group.children {
                check_closure(document, command, *child, depth + 1)?;
            }
            Ok(())
        }
        Some(_) => Ok(()),
        None => Err(EngineError::MissingTarget {
            name: command.to_owned(),
            detail: format!(
                "frame content references object {} (generation {}) which is not live",
                id.index(),
                id.generation()
            ),
        }),
    }
}

/// Deep-clones one element closure into fresh registry slots, rewriting
/// group children through `remap`. Tiles are shared (copy-on-write by
/// design); symbol-definition references are shared (instances, not
/// copies). The closure must be live (`check_closure` ran first). Every
/// fresh slot is appended to `minted` in insertion order — retraction
/// is LIFO, so undo must walk that list backwards.
pub(crate) fn clone_element(
    document: &mut Document,
    id: ObjectId,
    remap: &mut std::collections::HashMap<IdKey, ObjectId>,
    minted: &mut Vec<ObjectId>,
) -> ObjectId {
    let key = (id.index(), id.generation());
    if let Some(copy) = remap.get(&key) {
        return *copy;
    }
    let data = match document.registry.get(id) {
        Some(ObjectData::Group(group)) => {
            let mut group = group.clone();
            group.children = group
                .children
                .iter()
                .map(|child| clone_element(document, *child, remap, minted))
                .collect();
            ObjectData::Group(group)
        }
        Some(other) => other.clone(),
        // Unreachable when check_closure ran; mint nothing and drop the
        // slot rather than mint a copy of nothing.
        None => return id,
    };
    let copy = document.registry.insert(data);
    remap.insert(key, copy);
    minted.push(copy);
    copy
}

/// The duration entry for a new frame copied from `source`: the
/// source's own when the page's duration vec is aligned, else the
/// format default.
fn copied_duration(durations: &[i64], source: usize) -> i64 {
    durations
        .get(source)
        .copied()
        .unwrap_or(DEFAULT_FRAME_DURATION)
}

/// Splices `cells` into every layer's cell list at `at`, keeping each
/// layer's per-frame visibility vec aligned when it is present.
pub(crate) fn splice_cells(
    page: &mut doc::Page,
    at: usize,
    cells: impl Fn() -> Cell,
    count: usize,
) {
    for layer in &mut page.layers.layers {
        for _ in 0..count {
            layer.cells.insert(at, cells());
        }
        if let Some(visibility) = &mut layer.visibility_per_frame {
            if visibility.len() + count == layer.cells.len() {
                for _ in 0..count {
                    visibility.insert(at, true);
                }
            }
        }
    }
}

/// Appends `count` empty frames to the page (cells on every layer plus
/// the `FRCi` counts and, when aligned, the duration vec), the same
/// shape `frame.add` uses for end-appended frames.
pub(crate) fn append_frames(document: &mut Document, page: usize, count: usize) {
    let Some(target) = document.pages.get_mut(page) else {
        return;
    };
    let old_count = target.info.frame_count.max(0) as usize;
    let aligned =
        target.info.frame_durations.len() == old_count && !target.info.frame_durations.is_empty();
    splice_cells(
        target,
        old_count,
        || Cell {
            locked: false,
            visible: true,
            elements: Vec::new(),
        },
        count,
    );
    if aligned {
        for _ in 0..count {
            target
                .info
                .frame_durations
                .insert(old_count, DEFAULT_FRAME_DURATION);
        }
    }
    target.info.frame_count += count as i64;
    target.info.frame_count_duplicate += count as i64;
}

/// A frame's content as an owned list — the element ids of one cell of
/// every layer, in layer order.
fn frame_slice(page: &doc::Page, index: usize) -> Vec<Vec<ObjectId>> {
    page.layers
        .layers
        .iter()
        .map(|layer| {
            layer
                .cells
                .get(index)
                .map(|cell| cell.elements.clone())
                .unwrap_or_default()
        })
        .collect()
}

/// `frame.add` — CS6's `dom.addFrames(howMany, where)`: splices empty
/// cells into every layer of the page and bumps both `FRCi` counts.
/// New frames are visible and unlocked; their duration entry (when the
/// page's duration vec is aligned) is the format default.
struct AddFrames;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddFramesPayload {
    /// Page index.
    page: usize,
    /// How many frames to add.
    how_many: i64,
    /// Where the new frames land.
    location: FrameLocation,
    /// The caller's active frame, required by the current-relative
    /// locations.
    current: Option<usize>,
}

impl TypedCommand for AddFrames {
    type Payload = AddFramesPayload;

    fn name(&self) -> &'static str {
        "frame.add"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "frame.add";
        let count = how_many(NAME, payload.how_many)?;
        access::page(document, NAME, payload.page)?;
        let old_count = frame_count(document, payload.page);
        let at = insert_at(NAME, payload.location, payload.current, old_count)?;
        let target = access::page_mut(document, NAME, payload.page)?;
        let layers_before = target.layers.clone();
        let info_before = target.info.clone();
        splice_cells(
            target,
            at,
            || Cell {
                locked: false,
                visible: true,
                elements: Vec::new(),
            },
            count,
        );
        if info_before.frame_durations.len() == old_count && !info_before.frame_durations.is_empty()
        {
            for _ in 0..count {
                target
                    .info
                    .frame_durations
                    .insert(at, DEFAULT_FRAME_DURATION);
            }
        }
        target.info.frame_count += count as i64;
        target.info.frame_count_duplicate += count as i64;
        Ok(Delta::Group(vec![
            Delta::Layers {
                page: payload.page,
                layers: layers_before,
            },
            Delta::PageInfo {
                page: payload.page,
                before: Box::new(info_before),
            },
        ]))
    }
}

/// `frame.delete` — CS6's `dom.deleteFrames(frameIndex, howMany)`: drops
/// the cell slice at `index` from every layer, the matching duration
/// entries, and per-frame visibility overrides. The last frame cannot
/// be deleted (same rule as pages); negative indices (CS6's "current
/// frame" form) are refused — the engine has no active frame.
struct DeleteFrames;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DeleteFramesPayload {
    /// Page index.
    page: usize,
    /// First frame to delete.
    index: usize,
    /// How many frames to delete.
    how_many: i64,
}

impl TypedCommand for DeleteFrames {
    type Payload = DeleteFramesPayload;

    fn name(&self) -> &'static str {
        "frame.delete"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "frame.delete";
        let count = how_many(NAME, payload.how_many)?;
        access::page(document, NAME, payload.page)?;
        let total = frame_count(document, payload.page);
        if payload.index >= total {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("frame {} out of range ({total} frames)", payload.index),
            });
        }
        if payload.index + count > total {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "frames {}..{} beyond {} frames",
                    payload.index,
                    payload.index + count,
                    total
                ),
            });
        }
        if total - count == 0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "cannot delete the last frame".to_owned(),
            });
        }
        let target = access::page_mut(document, NAME, payload.page)?;
        let layers_before = target.layers.clone();
        let info_before = target.info.clone();
        let end = payload.index + count;
        for layer in &mut target.layers.layers {
            layer.cells.drain(payload.index..end);
            if let Some(visibility) = &mut layer.visibility_per_frame {
                if visibility.len() == total {
                    visibility.drain(payload.index..end);
                }
            }
        }
        if info_before.frame_durations.len() == total {
            target.info.frame_durations.drain(payload.index..end);
        }
        target.info.frame_count -= count as i64;
        target.info.frame_count_duplicate -= count as i64;
        Ok(Delta::Group(vec![
            Delta::Layers {
                page: payload.page,
                layers: layers_before,
            },
            Delta::PageInfo {
                page: payload.page,
                before: Box::new(info_before),
            },
        ]))
    }
}

/// `frame.duplicate` — CS6's `dom.duplicateFrame(frameIndex, howMany,
/// where, bDupeSelectionOnly)`: deep-copies a frame's content into
/// fresh registry slots (groups rewrite their children through the
/// remap; tiles and symbol definitions are shared by design) and
/// splices the copies in per `where`. `bDupeSelectionOnly` needs the
/// app's object selection, which the engine does not carry — refused.
struct DuplicateFrame;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DuplicateFramePayload {
    /// Page index.
    page: usize,
    /// Frame to duplicate.
    index: usize,
    /// How many copies to make.
    how_many: i64,
    /// Where the copies land.
    location: FrameLocation,
    /// The caller's active frame, required by the current-relative
    /// locations.
    current: Option<usize>,
    /// CS6's `bDupeSelectionOnly`; the engine has no object selection,
    /// so only `false`/absent is accepted.
    selection_only: Option<bool>,
}

impl TypedCommand for DuplicateFrame {
    type Payload = DuplicateFramePayload;

    fn name(&self) -> &'static str {
        "frame.duplicate"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "frame.duplicate";
        let count = how_many(NAME, payload.how_many)?;
        if payload.selection_only == Some(true) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "bDupeSelectionOnly needs the app's object selection; duplicate the \
                         whole frame instead"
                    .to_owned(),
            });
        }
        access::page(document, NAME, payload.page)?;
        let total = frame_count(document, payload.page);
        if payload.index >= total {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("frame {} out of range ({total} frames)", payload.index),
            });
        }
        // Validate every layer's source cell and its closure before
        // minting anything: a refusal must leave no fresh slots.
        {
            let page = access::page(document, NAME, payload.page)?;
            for layer in &page.layers.layers {
                let Some(cell) = layer.cells.get(payload.index) else {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: format!(
                            "layer carries {} cells, frame {} is out of range",
                            layer.cells.len(),
                            payload.index
                        ),
                    });
                };
                for id in &cell.elements {
                    check_closure(document, NAME, *id, 0)?;
                }
            }
        }
        let at = insert_at(NAME, payload.location, payload.current, total)?;
        let source = frame_slice(access::page(document, NAME, payload.page)?, payload.index);
        let layers_before = access::page(document, NAME, payload.page)?.layers.clone();
        let info_before = access::page(document, NAME, payload.page)?.info.clone();
        let duration = copied_duration(&info_before.frame_durations, payload.index);
        // Mint every copy before splicing: clone_element needs the
        // registry, the splice needs the page — sequential borrows.
        let mut minted: Vec<Vec<Cell>> = Vec::with_capacity(source.len());
        let mut fresh_slots: Vec<ObjectId> = Vec::new();
        for ids in &source {
            let mut layer_cells = Vec::with_capacity(count);
            for _ in 0..count {
                let mut remap = std::collections::HashMap::new();
                let mut slots = Vec::new();
                layer_cells.push(Cell {
                    locked: false,
                    visible: true,
                    elements: ids
                        .iter()
                        .map(|id| clone_element(document, *id, &mut remap, &mut slots))
                        .collect(),
                });
                fresh_slots.extend(slots);
            }
            minted.push(layer_cells);
        }
        let target = access::page_mut(document, NAME, payload.page)?;
        for (layer_index, layer) in target.layers.layers.iter_mut().enumerate() {
            for (offset, cell) in minted[layer_index].iter().enumerate() {
                layer.cells.insert(at + offset, cell.clone());
            }
            if let Some(visibility) = &mut layer.visibility_per_frame {
                if visibility.len() == total {
                    let flag = visibility.get(payload.index).copied().unwrap_or(true);
                    for _ in 0..count {
                        visibility.insert(at, flag);
                    }
                }
            }
        }
        if info_before.frame_durations.len() == total {
            for _ in 0..count {
                target.info.frame_durations.insert(at, duration);
            }
        }
        let minted_count = count as i64;
        target.info.frame_count += minted_count;
        target.info.frame_count_duplicate += minted_count;
        // Every fresh slot rides its own Created inverse so undo
        // retracts the copies — not just the cell lists. Deltas are
        // laid down in mint order: Group undoes last-captured-first,
        // which is exactly the LIFO order retraction requires.
        let mut deltas = vec![
            Delta::Layers {
                page: payload.page,
                layers: layers_before,
            },
            Delta::PageInfo {
                page: payload.page,
                before: Box::new(info_before),
            },
        ];
        deltas.extend(fresh_slots.into_iter().map(|id| Delta::Object {
            id,
            inverse: ObjectInverse::Created,
        }));
        Ok(Delta::Group(deltas))
    }
}

/// `frame.reorder` — CS6's `dom.reorderFrame(frameToMove,
/// frameToPutItBefore, bMakeCopy)`: moves (or copies) the frame slice
/// at `from` to sit before `before`. A move onto its own current slot
/// is CS6's no-op.
struct ReorderFrame;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ReorderFramePayload {
    /// Page index.
    page: usize,
    /// Frame to move or copy.
    from: usize,
    /// Insert before this frame index (may equal the frame count for
    /// "move to end").
    before: usize,
    /// Copy instead of move.
    make_copy: bool,
}

impl TypedCommand for ReorderFrame {
    type Payload = ReorderFramePayload;

    fn name(&self) -> &'static str {
        "frame.reorder"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "frame.reorder";
        access::page(document, NAME, payload.page)?;
        let total = frame_count(document, payload.page);
        if payload.from >= total {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("frame {} out of range ({total} frames)", payload.from),
            });
        }
        if payload.before > total {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("insert-before {} beyond {total} frames", payload.before),
            });
        }
        if !payload.make_copy
            && (payload.before == payload.from || payload.before == payload.from + 1)
        {
            return Ok(Delta::Noop);
        }
        if payload.make_copy {
            for layer in &access::page(document, NAME, payload.page)?.layers.layers {
                let Some(cell) = layer.cells.get(payload.from) else {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: format!(
                            "layer carries {} cells, frame {} is out of range",
                            layer.cells.len(),
                            payload.from
                        ),
                    });
                };
                for id in &cell.elements {
                    check_closure(document, NAME, *id, 0)?;
                }
            }
        }
        let page = access::page(document, NAME, payload.page)?;
        let layers_before = page.layers.layers.clone();
        let info_before = page.info.clone();
        let durations_aligned = info_before.frame_durations.len() == total;
        // For a copy, mint the clone before splicing (clone_element
        // needs the registry; the splice needs the page).
        let mut fresh_slots: Vec<ObjectId> = Vec::new();
        let minted: Vec<Cell> = if payload.make_copy {
            frame_slice(page, payload.from)
                .iter()
                .map(|ids| {
                    let mut remap = std::collections::HashMap::new();
                    Cell {
                        locked: false,
                        visible: true,
                        elements: ids
                            .iter()
                            .map(|id| clone_element(document, *id, &mut remap, &mut fresh_slots))
                            .collect(),
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        let target = access::page_mut(document, NAME, payload.page)?;
        for (layer_index, layer) in target.layers.layers.iter_mut().enumerate() {
            if payload.make_copy {
                layer
                    .cells
                    .insert(payload.before, minted[layer_index].clone());
                if let Some(visibility) = &mut layer.visibility_per_frame {
                    if visibility.len() == total {
                        let flag = layers_before[layer_index]
                            .cells
                            .get(payload.from)
                            .map(|cell| cell.visible)
                            .unwrap_or(true);
                        visibility.insert(payload.before, flag);
                    }
                }
            } else {
                let cell = layer.cells.remove(payload.from);
                let at = if payload.before > payload.from {
                    payload.before - 1
                } else {
                    payload.before
                };
                layer.cells.insert(at, cell);
                if let Some(visibility) = &mut layer.visibility_per_frame {
                    if visibility.len() == total {
                        let flag = visibility.remove(payload.from);
                        visibility.insert(at, flag);
                    }
                }
            }
        }
        if payload.make_copy {
            if durations_aligned {
                let duration = copied_duration(&info_before.frame_durations, payload.from);
                target.info.frame_durations.insert(payload.before, duration);
            }
            target.info.frame_count += 1;
            target.info.frame_count_duplicate += 1;
        } else if durations_aligned {
            let duration = target.info.frame_durations.remove(payload.from);
            let at = if payload.before > payload.from {
                payload.before - 1
            } else {
                payload.before
            };
            target.info.frame_durations.insert(at, duration);
        }
        // Fresh slots from a copy ride their own Created inverses so
        // undo retracts the clones — not just the cell lists. Deltas
        // are laid down in mint order: Group undoes last-captured-
        // first, which is exactly the LIFO order retraction requires.
        let mut deltas = vec![
            Delta::Layers {
                page: payload.page,
                layers: doc::LayerList {
                    layers: layers_before,
                },
            },
            Delta::PageInfo {
                page: payload.page,
                before: Box::new(info_before),
            },
        ];
        deltas.extend(fresh_slots.into_iter().map(|id| Delta::Object {
            id,
            inverse: ObjectInverse::Created,
        }));
        Ok(Delta::Group(deltas))
    }
}

/// `frame.distributeLayer` — CS6's `dom.distributeLayerToFrames()`: the
/// items of one layer's cell move one-per-frame into consecutive frames
/// (the animation workflow). Item 0 stays on the source frame; item i
/// moves to frame i, appending to that cell. Frames are added when the
/// layer has more items than the page has frames. A single item is
/// CS6's documented no-op.
struct DistributeLayerToFrames;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DistributeLayerToFramesPayload {
    /// Page index.
    page: usize,
    /// Layer whose items distribute.
    layer: usize,
    /// The source frame (CS6's current frame).
    cell: usize,
}

impl TypedCommand for DistributeLayerToFrames {
    type Payload = DistributeLayerToFramesPayload;

    fn name(&self) -> &'static str {
        "frame.distributeLayer"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "frame.distributeLayer";
        access::layer_mut(document, NAME, payload.page, payload.layer)?;
        let total = frame_count(document, payload.page);
        if payload.cell >= total {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("frame {} out of range ({total} frames)", payload.cell),
            });
        }
        let target = access::page_mut(document, NAME, payload.page)?;
        let layers_before = target.layers.clone();
        let info_before = target.info.clone();
        let items: Vec<ObjectId> = target.layers.layers[payload.layer]
            .cells
            .get(payload.cell)
            .map(|cell| cell.elements.clone())
            .unwrap_or_default();
        if items.len() < 2 {
            return Ok(Delta::Noop);
        }
        // Grow the frame set when the items outnumber the frames.
        let needed = items.len();
        if needed > total {
            let extra = needed - total;
            splice_cells(
                target,
                total,
                || Cell {
                    locked: false,
                    visible: true,
                    elements: Vec::new(),
                },
                extra,
            );
            if info_before.frame_durations.len() == total {
                for _ in 0..extra {
                    target.info.frame_durations.push(DEFAULT_FRAME_DURATION);
                }
            }
            target.info.frame_count = needed as i64;
            target.info.frame_count_duplicate = needed as i64;
        }
        // Move item i to frame i, descending so source removals stay
        // index-stable; item 0 keeps its slot.
        for index in (1..items.len()).rev() {
            let id = items[index];
            let source = &mut target.layers.layers[payload.layer].cells[payload.cell];
            source.elements.retain(|element| *element != id);
            target.layers.layers[payload.layer].cells[index]
                .elements
                .push(id);
        }
        Ok(Delta::Group(vec![
            Delta::Layers {
                page: payload.page,
                layers: layers_before,
            },
            Delta::PageInfo {
                page: payload.page,
                before: Box::new(info_before),
            },
        ]))
    }
}

/// `cell.setVisible` — toggle one state's cell visibility.
struct SetCellVisible;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetCellVisiblePayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
    /// Cell index (state number).
    cell: usize,
    /// New visibility.
    visible: bool,
}

impl TypedCommand for SetCellVisible {
    type Payload = SetCellVisiblePayload;

    fn name(&self) -> &'static str {
        "cell.setVisible"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "cell.setVisible";
        let target = access::cell_mut(document, NAME, payload.page, payload.layer, payload.cell)?;
        let before = target.clone();
        target.visible = payload.visible;
        Ok(Delta::Cell {
            page: payload.page,
            layer: payload.layer,
            cell: payload.cell,
            before: Box::new(before),
        })
    }
}

/// `cell.setLocked` — lock or unlock one state's cell.
struct SetCellLocked;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetCellLockedPayload {
    /// Page index.
    page: usize,
    /// Layer index, bottom-first.
    layer: usize,
    /// Cell index (state number).
    cell: usize,
    /// New locked flag.
    locked: bool,
}

impl TypedCommand for SetCellLocked {
    type Payload = SetCellLockedPayload;

    fn name(&self) -> &'static str {
        "cell.setLocked"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "cell.setLocked";
        let target = access::cell_mut(document, NAME, payload.page, payload.layer, payload.cell)?;
        let before = target.clone();
        target.locked = payload.locked;
        Ok(Delta::Cell {
            page: payload.page,
            layer: payload.layer,
            cell: payload.cell,
            before: Box::new(before),
        })
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
    fn set_visible_round_trips() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 1, "layer": 0, "cell": 1, "visible": false });
        let delta = SetCellVisible.apply_json(&mut document, payload).unwrap();
        assert!(!document.pages[1].layers.layers[0].cells[1].visible);
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.pages[1].layers.layers[0].cells[1].visible);
    }

    #[test]
    fn set_locked_round_trips() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 0, "layer": 1, "cell": 0, "locked": true });
        let delta = SetCellLocked.apply_json(&mut document, payload).unwrap();
        assert!(document.pages[0].layers.layers[1].cells[0].locked);
        delta.apply_inverse(&mut document).unwrap();
        assert!(!document.pages[0].layers.layers[1].cells[0].locked);
    }

    #[test]
    fn out_of_range_cell_is_missing_target() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 0, "layer": 0, "cell": 9, "visible": false });
        let error = SetCellVisible
            .apply_json(&mut document, payload)
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
    }

    #[test]
    fn add_frames_splices_cells_and_bumps_counts() {
        let (mut document, ids) = testdoc::sample_document();
        // A per-frame visibility vec stays aligned through the splice.
        document.pages[0].layers.layers[1].visibility_per_frame = Some(vec![false, true]);
        let before = serde_json::to_vec(&document).unwrap();
        let delta = AddFrames
            .apply_json(
                &mut document,
                json!({ "page": 0, "how_many": 2, "location": "beginning" }),
            )
            .unwrap();
        let page = &document.pages[0];
        assert_eq!(page.info.frame_count, 4);
        assert_eq!(page.info.frame_count_duplicate, 4);
        for layer in &page.layers.layers {
            assert_eq!(layer.cells.len(), 4);
            // Both frames landed before the old content.
            assert!(layer.cells[0].elements.is_empty());
            assert!(layer.cells[1].elements.is_empty());
        }
        assert_eq!(
            page.layers.layers[0].cells[2].elements,
            vec![ids[0], ids[1]],
            "old frame 0 content sits at index 2"
        );
        assert_eq!(
            page.layers.layers[1].visibility_per_frame,
            Some(vec![true, true, false, true]),
            "overrides realign with the insertion"
        );
        assert_eq!(
            page.info.frame_durations.len(),
            0,
            "none recorded, none minted"
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);

        // "after current" inserts at current + 1: old frame 0 keeps its
        // slot, the new empty frame lands behind it.
        AddFrames
            .apply_json(
                &mut document,
                json!({
                    "page": 0, "how_many": 1,
                    "location": "after current", "current": 0,
                }),
            )
            .unwrap();
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![ids[0], ids[1]]
        );
        assert!(
            document.pages[0].layers.layers[0].cells[1]
                .elements
                .is_empty()
        );

        // Current-relative without an active frame is refused; so is an
        // out-of-range active frame, a bogus count, and a bad location.
        let before = serde_json::to_vec(&document).unwrap();
        let error = AddFrames
            .apply_json(
                &mut document,
                json!({ "page": 0, "how_many": 1, "location": "before current" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        let error = AddFrames
            .apply_json(
                &mut document,
                json!({
                    "page": 0, "how_many": 1,
                    "location": "after current", "current": 9,
                }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        let error = AddFrames
            .apply_json(
                &mut document,
                json!({ "page": 0, "how_many": 0, "location": "end" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        let error = AddFrames
            .apply_json(
                &mut document,
                json!({ "page": 0, "how_many": 1, "location": "middle" }),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            crate::error::EngineError::BadPayload { .. }
        ));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn delete_frames_guards_and_round_trips() {
        let (mut document, _ids) = testdoc::sample_document();
        // The fixture leaves frame 1 empty; give layer 0's frame 1 its
        // own content so the shift is observable.
        let spare1 = document
            .registry
            .insert(document.registry.get(_ids[0]).cloned().unwrap());
        let spare2 = document
            .registry
            .insert(document.registry.get(_ids[1]).cloned().unwrap());
        document.pages[0].layers.layers[0].cells[1].elements = vec![spare1, spare2];
        document.pages[0].info.frame_durations = vec![10, 20];
        let delta = DeleteFrames
            .apply_json(
                &mut document,
                json!({ "page": 0, "index": 0, "how_many": 1 }),
            )
            .unwrap();
        assert_eq!(document.pages[0].info.frame_count, 1);
        assert_eq!(document.pages[0].info.frame_durations, vec![20]);
        assert_eq!(document.pages[0].layers.layers[0].cells.len(), 1);
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![spare1, spare2],
            "old frame 1 content is now frame 0"
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].info.frame_durations, vec![10, 20]);

        // Guards: out of range, overrun, last frame, bogus count.
        let before = serde_json::to_vec(&document).unwrap();
        for payload in [
            json!({ "page": 0, "index": 5, "how_many": 1 }),
            json!({ "page": 0, "index": 1, "how_many": 5 }),
            json!({ "page": 0, "index": 0, "how_many": 2 }),
            json!({ "page": 0, "index": 0, "how_many": 0 }),
        ] {
            let error = DeleteFrames.apply_json(&mut document, payload).unwrap_err();
            assert!(
                matches!(error, EngineError::InvalidState { .. })
                    || matches!(error, EngineError::MissingTarget { .. })
            );
        }
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn duplicate_frame_deep_copies_groups() {
        let (mut document, ids) = testdoc::sample_document();
        // Fold the fixture's two paths into one group in frame 0.
        let group_id = document
            .registry
            .insert(doc::ObjectData::Group(doc::GroupObject {
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
        document.pages[0].info.frame_durations = vec![10, 25];

        let delta = DuplicateFrame
            .apply_json(
                &mut document,
                json!({
                    "page": 0, "index": 0, "how_many": 1, "location": "end",
                }),
            )
            .unwrap();
        let page = &document.pages[0];
        assert_eq!(page.info.frame_count, 3);
        let copy_id = page.layers.layers[0].cells[2].elements[0];
        let copy = match document.registry.get(copy_id) {
            Some(doc::ObjectData::Group(group)) => group.clone(),
            other => panic!("not a group: {other:?}"),
        };
        assert_ne!(
            copy.children,
            vec![ids[0], ids[1]],
            "the copy owns fresh slots, not the original's ids"
        );
        for child in &copy.children {
            assert!(document.registry.get(*child).is_some(), "and they are live");
        }
        assert_eq!(
            page.info.frame_durations,
            vec![10, 25, 10],
            "duration copied"
        );
        // The original frame is untouched.
        assert_eq!(page.layers.layers[0].cells[0].elements, vec![group_id]);
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.registry.get(copy_id).is_none(), "copies retracted");
        for child in &copy.children {
            assert!(document.registry.get(*child).is_none());
        }
        assert_eq!(document.pages[0].info.frame_count, 2);

        // bDupeSelectionOnly is refused (no engine object selection).
        let before = serde_json::to_vec(&document).unwrap();
        let error = DuplicateFrame
            .apply_json(
                &mut document,
                json!({
                    "page": 0, "index": 0, "how_many": 1, "location": "end",
                    "selection_only": true,
                }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn reorder_moves_then_copies_frames() {
        let (mut document, ids) = testdoc::sample_document();
        // The fixture leaves frame 1 empty; give it its own content so
        // the move is observable.
        let spare1 = document
            .registry
            .insert(document.registry.get(ids[0]).cloned().unwrap());
        let spare2 = document
            .registry
            .insert(document.registry.get(ids[1]).cloned().unwrap());
        document.pages[0].layers.layers[0].cells[1].elements = vec![spare1, spare2];
        document.pages[0].info.frame_durations = vec![10, 20];
        let delta = ReorderFrame
            .apply_json(
                &mut document,
                json!({ "page": 0, "from": 1, "before": 0, "make_copy": false }),
            )
            .unwrap();
        assert_eq!(
            document.pages[0].info.frame_durations,
            vec![20, 10],
            "moved"
        );
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![spare1, spare2],
            "old frame 1 content is now first"
        );
        assert_eq!(
            document.pages[0].layers.layers[0].cells[1].elements,
            vec![ids[0], ids[1]],
            "old frame 0 content moved behind"
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].info.frame_durations, vec![10, 20]);

        // A move onto its own slot is a no-op.
        let noop = ReorderFrame
            .apply_json(
                &mut document,
                json!({ "page": 0, "from": 0, "before": 1, "make_copy": false }),
            )
            .unwrap();
        assert!(matches!(noop, Delta::Noop));

        // Copy: a fresh frame with fresh slots.
        let delta = ReorderFrame
            .apply_json(
                &mut document,
                json!({ "page": 0, "from": 0, "before": 2, "make_copy": true }),
            )
            .unwrap();
        assert_eq!(document.pages[0].info.frame_count, 3);
        let original = document.pages[0].layers.layers[0].cells[0].elements[0];
        let copy = document.pages[0].layers.layers[0].cells[2].elements[0];
        assert_ne!(original, copy, "the copy mints fresh slots");
        assert!(document.registry.get(copy).is_some());
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.registry.get(copy).is_none());
        assert_eq!(document.pages[0].info.frame_count, 2);
    }

    #[test]
    fn distribute_layer_moves_items_per_frame() {
        let (mut document, ids) = testdoc::sample_document();
        // Two more items on layer 0, frame 0: four items, two frames.
        let extra1 = document
            .registry
            .insert(document.registry.get(ids[0]).cloned().unwrap());
        let extra2 = document
            .registry
            .insert(document.registry.get(ids[1]).cloned().unwrap());
        document.pages[0].layers.layers[0].cells[0].elements = vec![ids[0], ids[1], extra1, extra2];
        let before = serde_json::to_vec(&document).unwrap();

        let delta = DistributeLayerToFrames
            .apply_json(&mut document, json!({ "page": 0, "layer": 0, "cell": 0 }))
            .unwrap();
        let layer = &document.pages[0].layers.layers[0];
        assert_eq!(document.pages[0].info.frame_count, 4, "frames grew to fit");
        assert_eq!(layer.cells.len(), 4);
        assert_eq!(
            layer.cells[0].elements,
            vec![ids[0]],
            "item 0 keeps the slot"
        );
        assert_eq!(layer.cells[1].elements, vec![ids[1]]);
        assert_eq!(layer.cells[2].elements, vec![extra1]);
        assert_eq!(layer.cells[3].elements, vec![extra2]);
        // Other layers keep their content and just gained empty cells.
        assert_eq!(
            document.pages[0].layers.layers[1].cells[0].elements,
            vec![ids[2], ids[3]]
        );
        assert!(
            document.pages[0].layers.layers[1].cells[1..]
                .iter()
                .all(|cell| cell.elements.is_empty())
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);

        // A single item is CS6's documented no-op.
        document.pages[0].layers.layers[0].cells[0].elements = vec![ids[0]];
        let noop = DistributeLayerToFrames
            .apply_json(&mut document, json!({ "page": 0, "layer": 0, "cell": 0 }))
            .unwrap();
        assert!(matches!(noop, Delta::Noop));
    }
}
