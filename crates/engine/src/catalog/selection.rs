//! Object-selection commands (CS6's `fw.selection`; decision 0024).
//!
//! The selection is document-level editing state —
//! [`Document::object_selection`] — stored in canonical document walk
//! order so `select*` results are order-deterministic regardless of
//! how they were picked. Group children are not cell-listed, so they
//! canonicalize by registry slot (documented deviation, decision
//! 0024).

use doc::{Document, ObjectId};

use super::cell::{DEFAULT_FRAME_DURATION, check_closure, clone_element, splice_cells};
use super::object::{Owner, find_owner};
use crate::access;
use crate::command::TypedCommand;
use crate::delta::Delta;
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the selection commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(SetSelection)?;
    registry.register(SelectAll)?;
    registry.register(SelectInverse)?;
    registry.register(SelectAllOnLayer)?;
    registry.register(SelectParents)?;
    registry.register(SelectChildren)?;
    registry.register(SelectNone)?;
    registry.register(DuplicateToFrames)?;
    registry.register(DuplicateToFrameRange)?;
    registry.register(DistributeToFrames)
}

/// Validates every id and returns them deduplicated in canonical
/// document walk order (page, layer, cell, element position; ids not
/// reachable from a cell — group children — order by registry slot).
fn canonicalize(
    document: &Document,
    name: &'static str,
    ids: &[ObjectId],
) -> Result<Vec<ObjectId>, EngineError> {
    for id in ids {
        if document.registry.get(*id).is_none() {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!(
                    "object {}:{} is not in the registry (stale or wrong generation)",
                    id.index(),
                    id.generation()
                ),
            });
        }
    }
    // Walk position for cell-listed ids; registry slot otherwise.
    // Keys: (walked, packed page/layer/cell or slot, element slot) —
    // the `walked` flag keeps the fallback in one coherent order
    // (after every cell-listed id) instead of colliding with packed
    // page bits.
    let mut walk_rank: std::collections::HashMap<ObjectId, (bool, u64, u64)> =
        std::collections::HashMap::new();
    for (page_index, page) in document.pages.iter().enumerate() {
        for (layer_index, layer) in page.layers.layers.iter().enumerate() {
            for (cell_index, cell) in layer.cells.iter().enumerate() {
                for (slot, id) in cell.elements.iter().enumerate() {
                    walk_rank.entry(*id).or_insert((
                        false,
                        (u64::from(page_index as u32)) << 32
                            | (u64::from(layer_index as u32)) << 16
                            | u64::from(cell_index as u32),
                        slot as u64,
                    ));
                }
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::with_capacity(ids.len());
    for id in ids {
        if seen.insert(*id) {
            unique.push(*id);
        }
    }
    unique.sort_by_key(|id| {
        walk_rank
            .get(id)
            .copied()
            .unwrap_or((true, u64::from(id.index() as u32), 0))
    });
    Ok(unique)
}

/// The element ids of one page+frame, in walk order, over every layer.
/// Layers shorter than `frame` contribute nothing; a `frame` beyond
/// every layer's cell list is refused by the caller.
fn scope_of_page_frame(document: &Document, page: usize, frame: usize) -> Vec<ObjectId> {
    let mut ids = Vec::new();
    let Some(page) = document.pages.get(page) else {
        return ids;
    };
    for layer in &page.layers.layers {
        if let Some(cell) = layer.cells.get(frame) {
            ids.extend(cell.elements.iter().copied());
        }
    }
    ids
}

/// `selection.set` — installs an explicit selection (the `fw.selection`
/// assignment surface and the UI's direct picks). Ids are validated
/// and stored in canonical order; duplicates collapse.
struct SetSelection;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetSelectionPayload {
    /// Objects to select, in canonical order.
    #[serde(default)]
    ids: Vec<ObjectId>,
}

impl TypedCommand for SetSelection {
    type Payload = SetSelectionPayload;

    fn name(&self) -> &'static str {
        "selection.set"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "selection.set";
        let next = canonicalize(document, NAME, &payload.ids)?;
        if next == document.object_selection {
            return Ok(Delta::Noop);
        }
        Ok(Delta::ObjectSelection {
            before: std::mem::replace(&mut document.object_selection, next),
        })
    }
}

/// `selection.selectAll` — CS6's `dom.selectAll()`: every element of
/// the page in the given frame ("all elements on all layers" — the
/// single-layer-editing variant is a UI concern, decision 0024).
struct SelectAll;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectAllPayload {
    /// Page to select on.
    page: usize,
    /// Frame (state) whose elements are selectable.
    frame: usize,
}

impl TypedCommand for SelectAll {
    type Payload = SelectAllPayload;

    fn name(&self) -> &'static str {
        "selection.selectAll"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "selection.selectAll";
        access::page(document, NAME, payload.page)?;
        frame_in_scope(document, NAME, payload.page, payload.frame)?;
        let ids = scope_of_page_frame(document, payload.page, payload.frame);
        let next = canonicalize(document, NAME, &ids)?;
        if next == document.object_selection {
            return Ok(Delta::Noop);
        }
        Ok(Delta::ObjectSelection {
            before: std::mem::replace(&mut document.object_selection, next),
        })
    }
}

/// `selection.selectInverse` — the Select > Inverse menu on the object
/// selection: everything in the page+frame scope the selection does
/// not already cover. Ids outside the scope drop out (CS6 text
/// documents only the bitmap-mode case; the object-mode complement is
/// the menu behavior — decision 0024).
struct SelectInverse;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectInversePayload {
    /// Page whose scope inverts.
    page: usize,
    /// Frame (state) whose scope inverts.
    frame: usize,
}

impl TypedCommand for SelectInverse {
    type Payload = SelectInversePayload;

    fn name(&self) -> &'static str {
        "selection.selectInverse"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "selection.selectInverse";
        access::page(document, NAME, payload.page)?;
        frame_in_scope(document, NAME, payload.page, payload.frame)?;
        let scope = scope_of_page_frame(document, payload.page, payload.frame);
        let complement: Vec<ObjectId> = scope
            .into_iter()
            .filter(|id| !document.object_selection.contains(id))
            .collect();
        let next = canonicalize(document, NAME, &complement)?;
        if next == document.object_selection {
            return Ok(Delta::Noop);
        }
        Ok(Delta::ObjectSelection {
            before: std::mem::replace(&mut document.object_selection, next),
        })
    }
}

/// `selection.selectAllOnLayer` — CS6's
/// `dom.selectAllOnLayer(layerIndex, bRememberSelection,
/// bToggleSelection)`: selects the layer's elements in the frame.
/// `remember_selection` appends instead of replacing; with it,
/// `toggle_selection` flips membership instead of adding.
struct SelectAllOnLayer;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectAllOnLayerPayload {
    /// Page index.
    page: usize,
    /// Layer index.
    layer: usize,
    /// Frame (state) whose elements are selectable.
    frame: usize,
    /// Append to the current selection instead of replacing
    /// (CS6's `bRememberSelection`).
    #[serde(default)]
    remember_selection: bool,
    /// Toggle membership instead of adding (CS6's
    /// `bToggleSelection`; meaningful only with
    /// `remember_selection`).
    #[serde(default)]
    toggle_selection: bool,
}

impl TypedCommand for SelectAllOnLayer {
    type Payload = SelectAllOnLayerPayload;

    fn name(&self) -> &'static str {
        "selection.selectAllOnLayer"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "selection.selectAllOnLayer";
        access::page(document, NAME, payload.page)?;
        let layer = document.pages[payload.page]
            .layers
            .layers
            .get(payload.layer)
            .ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("layer {} out of range", payload.layer),
            })?;
        frame_in_scope(document, NAME, payload.page, payload.frame)?;
        let ids: Vec<ObjectId> = layer
            .cells
            .get(payload.frame)
            .map(|cell| cell.elements.to_vec())
            .unwrap_or_default();
        let mut next = if payload.remember_selection {
            let mut combined = document.object_selection.clone();
            combined.extend(ids.iter().copied());
            combined
        } else {
            ids.clone()
        };
        if payload.remember_selection && payload.toggle_selection {
            // Toggle: selected ∆ layer ids — layer members drop out,
            // unselected layer members join.
            let layer_set: std::collections::HashSet<ObjectId> = ids.into_iter().collect();
            next.retain(|id| !layer_set.contains(id));
            next.extend(
                scope_of_layer_frame(document, payload.page, payload.layer, payload.frame)
                    .into_iter()
                    .filter(|id| !document.object_selection.contains(id)),
            );
        }
        let next = canonicalize(document, NAME, &next)?;
        if next == document.object_selection {
            return Ok(Delta::Noop);
        }
        Ok(Delta::ObjectSelection {
            before: std::mem::replace(&mut document.object_selection, next),
        })
    }
}

/// Element ids of one layer+frame in walk order.
fn scope_of_layer_frame(
    document: &Document,
    page: usize,
    layer: usize,
    frame: usize,
) -> Vec<ObjectId> {
    document
        .pages
        .get(page)
        .and_then(|page| page.layers.layers.get(layer))
        .and_then(|layer| layer.cells.get(frame))
        .map(|cell| cell.elements.to_vec())
        .unwrap_or_default()
}

/// `selection.selectParents` — CS6's `dom.selectParents()`: "if all
/// the members of a group are selected, the individual members are
/// deselected, and the group is selected." Nested groups resolve
/// innermost-first, iterating to a fixpoint (depth-capped — corrupt
/// cycles must not spin).
struct SelectParents;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectParentsPayload {}

impl TypedCommand for SelectParents {
    type Payload = SelectParentsPayload;

    fn name(&self) -> &'static str {
        "selection.selectParents"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "selection.selectParents";
        if document.object_selection.is_empty() {
            return Ok(Delta::Noop);
        }
        let mut selected: std::collections::HashSet<ObjectId> =
            document.object_selection.iter().copied().collect();
        // Collapse any group whose members are all selected; repeat
        // so a collapsed inner group can complete an outer one.
        for _ in 0..64 {
            let mut changed = false;
            for (id, data) in document.registry.iter() {
                if let doc::ObjectData::Group(group) = data {
                    if group.children.is_empty() {
                        continue;
                    }
                    if group.children.iter().all(|child| selected.contains(child)) {
                        for child in &group.children {
                            selected.remove(child);
                        }
                        selected.insert(id);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let next = canonicalize(document, NAME, &selected.into_iter().collect::<Vec<_>>())?;
        if next == document.object_selection {
            return Ok(Delta::Noop);
        }
        Ok(Delta::ObjectSelection {
            before: std::mem::replace(&mut document.object_selection, next),
        })
    }
}

/// `selection.selectChildren` — CS6's `dom.selectChildren()`: "if a
/// group is selected, the selection changes from the group to the
/// individual members of the group." One level per invocation, as
/// documented; non-group entries keep their places.
struct SelectChildren;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectChildrenPayload {}

impl TypedCommand for SelectChildren {
    type Payload = SelectChildrenPayload;

    fn name(&self) -> &'static str {
        "selection.selectChildren"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "selection.selectChildren";
        if document.object_selection.is_empty() {
            return Ok(Delta::Noop);
        }
        let mut next = Vec::with_capacity(document.object_selection.len());
        let mut expanded = false;
        for id in &document.object_selection {
            match document.registry.get(*id) {
                Some(doc::ObjectData::Group(group)) => {
                    next.extend(group.children.iter().copied());
                    expanded = true;
                }
                // Dangling entries drop, as in the projection: deleted
                // objects stop being selected.
                Some(_) | None => next.push(*id),
            }
        }
        if !expanded {
            return Ok(Delta::Noop);
        }
        let next = canonicalize(document, NAME, &next)?;
        Ok(Delta::ObjectSelection {
            before: std::mem::replace(&mut document.object_selection, next),
        })
    }
}

/// `selection.selectNone` — CS6's `dom.selectNone()`: clears the
/// object selection (the pixel mask is a separate mode and is left
/// alone).
struct SelectNone;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectNonePayload {}

impl TypedCommand for SelectNone {
    type Payload = SelectNonePayload;

    fn name(&self) -> &'static str {
        "selection.selectNone"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        if document.object_selection.is_empty() {
            return Ok(Delta::Noop);
        }
        Ok(Delta::ObjectSelection {
            before: std::mem::take(&mut document.object_selection),
        })
    }
}

/// Refuses a frame index beyond every layer's cell list on the page
/// (layers shorter than `frame` are simply skipped by the scope walk).
fn frame_in_scope(
    document: &Document,
    name: &'static str,
    page: usize,
    frame: usize,
) -> Result<(), EngineError> {
    let page = access::page(document, name, page)?;
    let max = page
        .layers
        .layers
        .iter()
        .map(|layer| layer.cells.len())
        .max()
        .unwrap_or(0);
    if frame >= max {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("frame {frame} out of range (page carries {max})"),
        });
    }
    Ok(())
}

// --- Selection-to-frames fans (CS6 `dom.cuplicateSelectionToFrames`,
// `dom.duplicateSelectionToFrameRange`, `dom.distributeSelectionToFrames`;
// decision 0026). CS6 aims these at "the current frame" — editor state
// the engine does not carry — so every command names its source frame
// explicitly, mirroring `frame.add`'s current-relative form.

/// Which frames `duplicateToFrames` fans to (CS6's `whichFrames`
/// strings, decision 0026).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum FrameFan {
    All,
    Previous,
    Next,
    End,
}

/// The selection members that live on one page+frame, in canonical
/// order. Group children are skipped: frame fans move whole top-level
/// items, so a group is fanned by selecting the group itself.
fn fan_items(
    document: &Document,
    name: &'static str,
    page: usize,
    frame: usize,
) -> Result<Vec<ObjectId>, EngineError> {
    access::page(document, name, page)?;
    frame_in_scope(document, name, page, frame)?;
    Ok(document
        .object_selection
        .iter()
        .copied()
        .filter(|id| {
            matches!(
                find_owner(document, *id),
                Some(Owner::Cell {
                    page: p,
                    cell: f,
                    ..
                }) if p == page && f == frame
            )
        })
        .collect())
}

/// Clones `items` onto `targets` (frame indices on `page`), one
/// independent deep copy per (item, target). Copies append to the
/// target cell (top of z-order — recorded deviation from Clone's
/// above-original placement). Returns the minted ids in insertion
/// order; closures are checked before any mint so a refused fan leaves
/// nothing behind.
fn fan_clones(
    document: &mut Document,
    name: &'static str,
    page: usize,
    items: &[ObjectId],
    targets: &[usize],
) -> Result<Vec<ObjectId>, EngineError> {
    for &id in items {
        check_closure(document, name, id, 0)?;
    }
    let mut minted = Vec::new();
    for &target in targets {
        for &id in items {
            let Some(Owner::Cell { layer, .. }) = find_owner(document, id) else {
                return Err(EngineError::MissingTarget {
                    name: name.to_owned(),
                    detail: format!(
                        "selected object {} (generation {}) left its cell mid-fan",
                        id.index(),
                        id.generation()
                    ),
                });
            };
            let mut remap = std::collections::HashMap::new();
            let copy = clone_element(document, id, &mut remap, &mut minted);
            access::cell_mut(document, name, page, layer, target)?
                .elements
                .push(copy);
        }
    }
    Ok(minted)
}

/// Appends `count` empty frames to the page (cells on every layer plus
/// the `FRCi` counts and, when aligned, the duration vec), the same
/// shape `frame.add` uses for end-appended frames.
fn append_frames(document: &mut Document, page: usize, count: usize) {
    let Some(target) = document.pages.get_mut(page) else {
        return;
    };
    let old_count = target.info.frame_count.max(0) as usize;
    let aligned =
        target.info.frame_durations.len() == old_count && !target.info.frame_durations.is_empty();
    splice_cells(
        target,
        old_count,
        || doc::Cell {
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

/// `selection.duplicateToFrames` — CS6
/// `dom.cuplicateSelectionToFrames(whichFrames)` (the original's own
/// typo lives in the parity matrix; the command carries the clean
/// name). "all" covers every existing frame including the source (an
/// in-place clone — the text documents no exclusion); "next" past the
/// last frame adds one; "end" never adds.
struct DuplicateToFrames;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DuplicateToFramesPayload {
    /// Page whose frames receive the copies.
    page: usize,
    /// Source ("current") frame.
    frame: usize,
    /// Which frames to fan to.
    which: FrameFan,
}

impl TypedCommand for DuplicateToFrames {
    type Payload = DuplicateToFramesPayload;

    fn name(&self) -> &'static str {
        "selection.duplicateToFrames"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "selection.duplicateToFrames";
        let items = fan_items(document, NAME, payload.page, payload.frame)?;
        let count = document.pages[payload.page].info.frame_count.max(0) as usize;
        let (targets, extra) = match payload.which {
            FrameFan::All => ((0..count).collect(), 0),
            FrameFan::Previous => {
                if payload.frame == 0 {
                    return Ok(Delta::Noop);
                }
                (vec![payload.frame - 1], 0)
            }
            FrameFan::Next => {
                let extra = usize::from(payload.frame + 1 == count);
                (vec![payload.frame + 1], extra)
            }
            FrameFan::End => {
                if count == 0 {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: "page carries no frames".to_owned(),
                    });
                }
                (vec![count - 1], 0)
            }
        };
        if items.is_empty() {
            return Ok(Delta::Noop);
        }
        let before = Box::new(document.pages[payload.page].clone());
        if extra > 0 {
            append_frames(document, payload.page, extra);
        }
        let minted = fan_clones(document, NAME, payload.page, &items, &targets)?;
        let mut deltas = vec![Delta::PageBody {
            page: payload.page,
            before,
        }];
        deltas.extend(minted.into_iter().map(|id| Delta::Object {
            id,
            inverse: crate::delta::ObjectInverse::Created,
        }));
        Ok(Delta::Group(deltas))
    }
}

/// `selection.duplicateToFrameRange` — CS6
/// `dom.duplicateSelectionToFrameRange(first, last)`, inclusive. `-1`
/// keeps its CS6 meaning ("the current frame") and names the payload's
/// source frame. When the range spans, the source frame is skipped
/// ("duplicates are not placed on that frame"); when `first == last`
/// the single frame is copied to even if it is the source. Ranges
/// never create frames (the text grants creation only to `distribute`
/// and "next").
struct DuplicateToFrameRange;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DuplicateToFrameRangePayload {
    /// Page whose frames receive the copies.
    page: usize,
    /// Source ("current") frame; `-1` in `first`/`last` names it.
    frame: usize,
    /// First frame of the inclusive range (`-1` = the source frame).
    first: i64,
    /// Last frame of the inclusive range (`-1` = the source frame).
    last: i64,
}

impl TypedCommand for DuplicateToFrameRange {
    type Payload = DuplicateToFrameRangePayload;

    fn name(&self) -> &'static str {
        "selection.duplicateToFrameRange"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "selection.duplicateToFrameRange";
        let items = fan_items(document, NAME, payload.page, payload.frame)?;
        if items.is_empty() {
            return Ok(Delta::Noop);
        }
        let resolve = |value: i64| -> Result<usize, EngineError> {
            if value < -1 {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("frame index {value} below -1"),
                });
            }
            Ok(if value < 0 {
                payload.frame
            } else {
                value as usize
            })
        };
        let first = resolve(payload.first)?;
        let last = resolve(payload.last)?;
        let count = document.pages[payload.page].info.frame_count.max(0) as usize;
        if first > last {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("range {first}..={last} is empty"),
            });
        }
        if last >= count {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("frame {last} out of range (page carries {count})"),
            });
        }
        let targets: Vec<usize> = (first..=last)
            .filter(|&target| first == last || target != payload.frame)
            .collect();
        if targets.is_empty() {
            return Ok(Delta::Noop);
        }
        let before = Box::new(document.pages[payload.page].clone());
        let minted = fan_clones(document, NAME, payload.page, &items, &targets)?;
        let mut deltas = vec![Delta::PageBody {
            page: payload.page,
            before,
        }];
        deltas.extend(minted.into_iter().map(|id| Delta::Object {
            id,
            inverse: crate::delta::ObjectInverse::Created,
        }));
        Ok(Delta::Group(deltas))
    }
}

/// `selection.distributeToFrames` — CS6
/// `dom.distributeSelectionToFrames()`: moves the selected items onto
/// consecutive frames starting at the source frame ("the first item
/// goes to the current frame"), adding frames when the run outruns the
/// page. One item — or none — is the documented no-effect case.
struct DistributeToFrames;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DistributeToFramesPayload {
    /// Page whose frames receive the items.
    page: usize,
    /// Source ("current") frame.
    frame: usize,
}

impl TypedCommand for DistributeToFrames {
    type Payload = DistributeToFramesPayload;

    fn name(&self) -> &'static str {
        "selection.distributeToFrames"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "selection.distributeToFrames";
        let items = fan_items(document, NAME, payload.page, payload.frame)?;
        if items.len() <= 1 {
            return Ok(Delta::Noop);
        }
        let count = document.pages[payload.page].info.frame_count.max(0) as usize;
        let needed = (payload.frame + items.len()).saturating_sub(count);
        let before = Box::new(document.pages[payload.page].clone());
        if needed > 0 {
            append_frames(document, payload.page, needed);
        }
        // Item 0 stays (it is already on the source frame); the rest
        // move within their own layer, one frame further each.
        for (step, &id) in items.iter().enumerate().skip(1) {
            let target = payload.frame + step;
            let Some(Owner::Cell { layer, .. }) = find_owner(document, id) else {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "selected object {} (generation {}) left its cell mid-fan",
                        id.index(),
                        id.generation()
                    ),
                });
            };
            let source = access::cell_mut(document, NAME, payload.page, layer, payload.frame)?;
            let Some(position) = source.elements.iter().position(|&element| element == id) else {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "selected object {} (generation {}) is not in the source cell",
                        id.index(),
                        id.generation()
                    ),
                });
            };
            source.elements.remove(position);
            access::cell_mut(document, NAME, payload.page, layer, target)?
                .elements
                .push(id);
        }
        Ok(Delta::PageBody {
            page: payload.page,
            before,
        })
    }
}

#[cfg(test)]
mod selection_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::catalog::testdoc;
    use crate::command::Command;

    /// The fixture's two ids of page 0, layer 0, cell 0 (walk order).
    fn page0_layer0_ids(document: &Document) -> Vec<ObjectId> {
        document.pages[0].layers.layers[0].cells[0].elements.clone()
    }

    #[test]
    fn set_validates_canonicalizes_and_dedupes() {
        let (mut document, ids) = testdoc::sample_document();
        let a = ids[0];
        let b = ids[1];
        // Reversed order with a duplicate canonicalizes to walk order.
        let delta = SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [b, a, b]}))
            .expect("set applies");
        assert_eq!(document.object_selection, vec![a, b]);
        delta.apply_inverse(&mut document).expect("undo");
        assert!(document.object_selection.is_empty());

        // A stale id (wrong generation) is a refusal.
        let stale = ObjectId::from_parts(a.index() as u32, a.generation() + 1);
        let err = SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [stale]}))
            .expect_err("stale id is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::InvalidState { .. }
        ));
    }

    #[test]
    fn set_to_same_selection_is_a_noop() {
        let (mut document, ids) = testdoc::sample_document();
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [ids[0]]}))
            .expect("first set applies");
        let delta = SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [ids[0]]}))
            .expect("second set applies");
        assert!(matches!(delta, Delta::Noop));
    }

    #[test]
    fn select_all_scopes_the_page_and_frame() {
        let (mut document, _ids) = testdoc::sample_document();
        let delta = SelectAll
            .apply_json(&mut document, serde_json::json!({"page": 0, "frame": 0}))
            .expect("selectAll applies");
        // Page 0, frame 0: two cells-worth? Only cell 0 carries the
        // fixture's four ids (two per layer).
        let expected = [
            page0_layer0_ids(&document),
            document.pages[0].layers.layers[1].cells[0].elements.clone(),
        ]
        .concat();
        assert_eq!(document.object_selection, expected);
        assert_eq!(document.object_selection.len(), 4);
        delta.apply_inverse(&mut document).expect("undo");
        assert!(document.object_selection.is_empty());

        // Frame 1 has no elements in the fixture: selectAll lands as a
        // Noop (nothing to select).
        let delta = SelectAll
            .apply_json(&mut document, serde_json::json!({"page": 0, "frame": 1}))
            .expect("selectAll frame 1 applies");
        assert!(matches!(delta, Delta::Noop));

        // Out-of-range page and frame are refusals (the page guard is
        // `access::page`'s MissingTarget; the frame guard is our own
        // InvalidState).
        let err = SelectAll
            .apply_json(&mut document, serde_json::json!({"page": 9, "frame": 0}))
            .expect_err("page out of range is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::MissingTarget { .. }
        ));
        let err = SelectAll
            .apply_json(&mut document, serde_json::json!({"page": 0, "frame": 9}))
            .expect_err("frame out of range is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::InvalidState { .. }
        ));
    }

    #[test]
    fn select_inverse_complements_within_the_scope() {
        let (mut document, ids) = testdoc::sample_document();
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [ids[0], ids[1]]}))
            .expect("seed selection");
        SelectInverse
            .apply_json(&mut document, serde_json::json!({"page": 0, "frame": 0}))
            .expect("selectInverse applies");
        let layer1 = document.pages[0].layers.layers[1].cells[0].elements.clone();
        assert_eq!(document.object_selection, layer1);
    }

    #[test]
    fn select_all_on_layer_replaces_appends_and_toggles() {
        let (mut document, _ids) = testdoc::sample_document();
        let layer0 = page0_layer0_ids(&document);
        let layer1 = document.pages[0].layers.layers[1].cells[0].elements.clone();

        // Replace (default).
        SelectAllOnLayer
            .apply_json(
                &mut document,
                serde_json::json!({"page": 0, "layer": 0, "frame": 0}),
            )
            .expect("replace applies");
        assert_eq!(document.object_selection, layer0);

        // Remember: append layer 1's ids.
        SelectAllOnLayer
            .apply_json(
                &mut document,
                serde_json::json!({
                    "page": 0, "layer": 1, "frame": 0,
                    "remember_selection": true,
                }),
            )
            .expect("remember applies");
        let mut combined = layer0.clone();
        combined.extend(layer1.iter().copied());
        assert_eq!(document.object_selection, combined);

        // Remember + toggle: layer 1's members flip off.
        SelectAllOnLayer
            .apply_json(
                &mut document,
                serde_json::json!({
                    "page": 0, "layer": 1, "frame": 0,
                    "remember_selection": true,
                    "toggle_selection": true,
                }),
            )
            .expect("toggle applies");
        assert_eq!(document.object_selection, layer0);

        // Layer out of range is a refusal.
        let err = SelectAllOnLayer
            .apply_json(
                &mut document,
                serde_json::json!({"page": 0, "layer": 9, "frame": 0}),
            )
            .expect_err("layer out of range is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::InvalidState { .. }
        ));
    }

    #[test]
    fn select_none_clears_and_noops_when_empty() {
        let (mut document, ids) = testdoc::sample_document();
        // Empty already: a Noop, not a delta.
        let delta = SelectNone
            .apply_json(&mut document, serde_json::json!({}))
            .expect("selectNone applies");
        assert!(matches!(delta, Delta::Noop));

        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": ids.clone()}))
            .expect("seed selection");
        let delta = SelectNone
            .apply_json(&mut document, serde_json::json!({}))
            .expect("selectNone applies");
        assert!(document.object_selection.is_empty());
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(document.object_selection, ids);
    }

    #[test]
    fn canonical_order_follows_the_document_walk() {
        // Ids from page 1 sort after page 0 ids regardless of pick
        // order; within a page, layer then cell then slot.
        let (mut document, ids) = testdoc::sample_document();
        let page1 = document.pages[1].layers.layers[0].cells[0].elements.clone();
        let mut picks = vec![page1[0], ids[3], ids[0]];
        picks.reverse();
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": picks}))
            .expect("set applies");
        assert_eq!(document.object_selection, vec![ids[0], ids[3], page1[0]]);
    }

    /// A document with one group holding the two paths of page 0
    /// layer 0 cell 0 (the group id replaces them in the cell list).
    fn grouped_document() -> (Document, Vec<ObjectId>, ObjectId) {
        let (mut document, ids) = testdoc::sample_document();
        let group = document
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
        let cell = &mut document.pages[0].layers.layers[0].cells[0];
        cell.elements = vec![group];
        (document, ids, group)
    }

    #[test]
    fn select_children_expands_a_group_one_level() {
        let (mut document, ids, group) = grouped_document();
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [group]}))
            .expect("select group");
        SelectChildren
            .apply_json(&mut document, serde_json::json!({}))
            .expect("selectChildren applies");
        assert_eq!(document.object_selection, vec![ids[0], ids[1]]);

        // A second invocation with no groups selected is a Noop.
        let delta = SelectChildren
            .apply_json(&mut document, serde_json::json!({}))
            .expect("selectChildren applies");
        assert!(matches!(delta, Delta::Noop));

        // Undo restores the group selection.
        let delta = SelectParents
            .apply_json(&mut document, serde_json::json!({}))
            .expect("selectParents applies");
        assert_eq!(document.object_selection, vec![group]);
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(document.object_selection, vec![ids[0], ids[1]]);
    }

    #[test]
    fn select_parents_needs_every_member_selected() {
        let (mut document, ids, group) = grouped_document();
        // Only one member selected: nothing collapses (Noop).
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [ids[0]]}))
            .expect("select member");
        let delta = SelectParents
            .apply_json(&mut document, serde_json::json!({}))
            .expect("selectParents applies");
        assert!(matches!(delta, Delta::Noop));
        assert_eq!(document.object_selection, vec![ids[0]]);

        // A selected non-member alongside the full set does not block
        // the collapse: both members in → the group replaces them.
        SetSelection
            .apply_json(
                &mut document,
                serde_json::json!({"ids": [ids[0], ids[1], ids[2]]}),
            )
            .expect("select members + outsider");
        SelectParents
            .apply_json(&mut document, serde_json::json!({}))
            .expect("selectParents applies");
        assert_eq!(document.object_selection.len(), 2);
        assert!(document.object_selection.contains(&group));
        assert!(document.object_selection.contains(&ids[2]));
    }

    #[test]
    fn select_parents_and_children_round_trip_nested() {
        // group2 holds [group1, outsider]; selecting all of group2's
        // members collapses both levels; selectChildren then expands
        // exactly one level.
        let (mut document, ids, group1) = grouped_document();
        let outsider = ids[2];
        let group2 = document
            .registry
            .insert(doc::ObjectData::Group(doc::GroupObject {
                group_type: 0,
                name: None,
                children: vec![group1, outsider],
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                mask_hidden: false,
                mask_linked: None,
                effects: Vec::new(),
            }));
        document.pages[0].layers.layers[0].cells[0].elements = vec![group2];

        SetSelection
            .apply_json(
                &mut document,
                serde_json::json!({"ids": [group1, outsider]}),
            )
            .expect("select inner group + outsider");
        SelectParents
            .apply_json(&mut document, serde_json::json!({}))
            .expect("selectParents applies");
        assert_eq!(document.object_selection, vec![group2]);

        SelectChildren
            .apply_json(&mut document, serde_json::json!({}))
            .expect("selectChildren applies");
        // One level: group1 stays a group; the outsider joins. Neither
        // is cell-listed any more, so canonical order falls back to
        // registry slots (outsider first).
        assert_eq!(document.object_selection, vec![outsider, group1]);
    }

    /// The sample's page 0 reshaped for frame fans: two layers, two
    /// frames; layer 0 frame 0 holds `[a, group, c]` (the group wraps
    /// `child`), frame 1 holds `[f2]`, layer 1 is empty. Returned ids:
    /// `[a, child, group, c, f2]`.
    fn fan_document() -> (Document, Vec<ObjectId>) {
        let (mut document, ids) = testdoc::sample_document();
        document.pages.truncate(1);
        let (a, child, f2, c) = (ids[0], ids[1], ids[2], ids[3]);
        let group = document
            .registry
            .insert(doc::ObjectData::Group(doc::GroupObject {
                group_type: 0,
                name: None,
                children: vec![child],
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                mask_hidden: false,
                mask_linked: None,
                effects: Vec::new(),
            }));
        let page = &mut document.pages[0];
        page.info.frame_count = 2;
        page.info.frame_count_duplicate = 2;
        page.layers.layers[0].cells[0].elements = vec![a, group, c];
        page.layers.layers[0].cells[1].elements = vec![f2];
        page.layers.layers[1].cells[0].elements.clear();
        (document, vec![a, child, group, c, f2])
    }

    #[test]
    fn duplicate_to_frames_all_covers_every_frame() {
        let (mut document, ids) = fan_document();
        let (a, group) = (ids[0], ids[2]);
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [a, group]}))
            .expect("select frame-0 items");
        let live_before = document.registry.len();
        let layer0 = |document: &Document, frame: usize| -> Vec<ObjectId> {
            document.pages[0].layers.layers[0].cells[frame]
                .elements
                .clone()
        };
        let delta = DuplicateToFrames
            .apply_json(
                &mut document,
                serde_json::json!({"page": 0, "frame": 0, "which": "all"}),
            )
            .expect("all frames fan applies");
        // Copies append to every frame including the source; each
        // group copy brings its own child (3 mints per frame, two
        // frames in the fixture).
        assert_eq!(document.registry.len(), live_before + 6);
        assert_eq!(layer0(&document, 0).len(), 5);
        assert_eq!(layer0(&document, 1).len(), 3);
        assert_eq!(layer0(&document, 1)[0], ids[4]);
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(document.registry.len(), live_before);
        assert_eq!(layer0(&document, 0), vec![a, group, ids[3]]);
        assert_eq!(layer0(&document, 1), vec![ids[4]]);
        assert_eq!(document.object_selection, vec![a, group]);
    }

    #[test]
    fn duplicate_to_frame_range_skips_the_source_frame() {
        let (mut document, ids) = fan_document();
        let (a, f2) = (ids[0], ids[4]);
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [a]}))
            .expect("select a");
        let live_before = document.registry.len();
        let delta = DuplicateToFrameRange
            .apply_json(
                &mut document,
                serde_json::json!({"page": 0, "frame": 0, "first": 0, "last": 1}),
            )
            .expect("range fan applies");
        // The spanning range drops the source frame; frame 1 gains one
        // copy on the item's own layer, behind its resident.
        assert_eq!(document.registry.len(), live_before + 1);
        assert_eq!(document.pages[0].layers.layers[0].cells[0].elements[0], a);
        let target = document.pages[0].layers.layers[0].cells[1].elements.clone();
        assert_eq!(target.len(), 2);
        assert_eq!(target[0], f2, "the resident stays first");
        assert_ne!(target[1], f2, "the copy is a fresh slot");
        assert_eq!(
            document.pages[0].layers.layers[1].cells[1].elements.len(),
            0
        );
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(
            document.pages[0].layers.layers[0].cells[1].elements,
            vec![f2]
        );
        assert_eq!(document.registry.len(), live_before);
    }

    #[test]
    fn duplicate_to_frame_range_single_frame_copies_in_place() {
        let (mut document, ids) = fan_document();
        let (a, group, c) = (ids[0], ids[2], ids[3]);
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [a]}))
            .expect("select a");
        let live_before = document.registry.len();
        // first == last on the source frame: the documented exception
        // to the skip rule.
        let delta = DuplicateToFrameRange
            .apply_json(
                &mut document,
                serde_json::json!({"page": 0, "frame": 0, "first": 0, "last": 0}),
            )
            .expect("single-frame fan applies");
        assert_eq!(document.registry.len(), live_before + 1);
        let cell = document.pages[0].layers.layers[0].cells[0].elements.clone();
        assert_eq!(cell.len(), 4);
        assert_eq!(&cell[..3], &[a, group, c]);
        assert_ne!(cell[3], a, "the copy is a fresh slot");
        delta.apply_inverse(&mut document).expect("undo");

        // Refusals: empty range, out-of-range last (no creation).
        for payload in [
            serde_json::json!({"page": 0, "frame": 0, "first": 1, "last": 0}),
            serde_json::json!({"page": 0, "frame": 0, "first": 0, "last": 2}),
            serde_json::json!({"page": 0, "frame": 0, "first": -3, "last": 1}),
        ] {
            assert!(
                DuplicateToFrameRange
                    .apply_json(&mut document, payload)
                    .is_err(),
                "range fan must refuse"
            );
        }
        assert_eq!(document.registry.len(), live_before);
    }

    #[test]
    fn duplicate_to_frames_next_adds_a_frame() {
        let (mut document, ids) = fan_document();
        let f2 = ids[4];
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [f2]}))
            .expect("select the last-frame item");
        let live_before = document.registry.len();
        let count_before = document.pages[0].info.frame_count;
        let delta = DuplicateToFrames
            .apply_json(
                &mut document,
                serde_json::json!({"page": 0, "frame": 1, "which": "next"}),
            )
            .expect("next fan applies");
        assert_eq!(document.pages[0].info.frame_count, count_before + 1);
        assert_eq!(document.registry.len(), live_before + 1);
        // "next" past the last frame appended one; every layer got a
        // cell for it.
        assert_eq!(
            document.pages[0].layers.layers[0].cells[2].elements.len(),
            1
        );
        assert_eq!(
            document.pages[0].layers.layers[1].cells[2].elements.len(),
            0
        );
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(document.pages[0].info.frame_count, count_before);
        assert_eq!(document.registry.len(), live_before);
        assert_eq!(document.pages[0].layers.layers[0].cells.len(), 2);
        assert_eq!(
            document.pages[0].layers.layers[0].cells[1].elements,
            vec![f2]
        );
    }

    #[test]
    fn duplicate_to_frames_previous_noops_at_frame_zero() {
        let (mut document, ids) = fan_document();
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [ids[0]]}))
            .expect("select a");
        let live_before = document.registry.len();
        let delta = DuplicateToFrames
            .apply_json(
                &mut document,
                serde_json::json!({"page": 0, "frame": 0, "which": "previous"}),
            )
            .expect("previous at 0 applies");
        assert_eq!(delta, Delta::Noop);
        assert_eq!(document.registry.len(), live_before);
        // An empty frame-0 selection is the other no-effect case.
        let (mut document, _ids) = fan_document();
        let delta = DuplicateToFrames
            .apply_json(
                &mut document,
                serde_json::json!({"page": 0, "frame": 1, "which": "all"}),
            )
            .expect("empty fan applies");
        assert_eq!(delta, Delta::Noop);
    }

    #[test]
    fn distribute_moves_items_onto_consecutive_frames() {
        let (mut document, ids) = fan_document();
        let (a, group, c) = (ids[0], ids[2], ids[3]);
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [a, group, c]}))
            .expect("select frame-0 items");
        let delta = DistributeToFrames
            .apply_json(&mut document, serde_json::json!({"page": 0, "frame": 0}))
            .expect("distribute applies");
        // a stays on frame 0; group and c move one and two frames out —
        // the run outruns the two-frame page, so a frame is added, and
        // group lands behind frame 1's resident. Ids are preserved,
        // selection intact.
        assert_eq!(document.pages[0].info.frame_count, 3);
        let layer0 = |document: &Document, frame: usize| -> Vec<ObjectId> {
            document.pages[0].layers.layers[0].cells[frame]
                .elements
                .clone()
        };
        assert_eq!(layer0(&document, 0), vec![a]);
        assert_eq!(layer0(&document, 1), vec![ids[4], group]);
        assert_eq!(layer0(&document, 2), vec![c]);
        assert_eq!(document.object_selection, vec![a, group, c]);
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(document.pages[0].info.frame_count, 2);
        assert_eq!(layer0(&document, 0), vec![a, group, c]);
        assert_eq!(layer0(&document, 1), vec![ids[4]]);
        assert_eq!(document.pages[0].layers.layers[0].cells.len(), 2);
    }

    #[test]
    fn distribute_single_item_noops() {
        let (mut document, ids) = fan_document();
        SetSelection
            .apply_json(&mut document, serde_json::json!({"ids": [ids[0]]}))
            .expect("select one");
        let delta = DistributeToFrames
            .apply_json(&mut document, serde_json::json!({"page": 0, "frame": 0}))
            .expect("distribute applies");
        assert_eq!(delta, Delta::Noop);
        // And with nothing selected at all.
        let (mut document, _ids) = fan_document();
        let delta = DistributeToFrames
            .apply_json(&mut document, serde_json::json!({"page": 0, "frame": 0}))
            .expect("distribute applies");
        assert_eq!(delta, Delta::Noop);
    }
}
