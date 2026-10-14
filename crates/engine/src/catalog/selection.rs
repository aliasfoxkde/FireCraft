//! Object-selection commands (CS6's `fw.selection`; decision 0024).
//!
//! The selection is document-level editing state —
//! [`Document::object_selection`] — stored in canonical document walk
//! order so `select*` results are order-deterministic regardless of
//! how they were picked. Group children are not cell-listed, so they
//! canonicalize by registry slot (documented deviation, decision
//! 0024).

use doc::{Document, ObjectId};

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
    registry.register(SelectNone)
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
    let mut walk_rank: std::collections::HashMap<ObjectId, (u64, u64)> =
        std::collections::HashMap::new();
    for (page_index, page) in document.pages.iter().enumerate() {
        for (layer_index, layer) in page.layers.layers.iter().enumerate() {
            for (cell_index, cell) in layer.cells.iter().enumerate() {
                for (slot, id) in cell.elements.iter().enumerate() {
                    walk_rank.entry(*id).or_insert((
                        (u64::from(page_index as u32)) << 48
                            | (u64::from(layer_index as u32)) << 32
                            | (u64::from(cell_index as u32)) << 16,
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
        *walk_rank
            .get(id)
            .unwrap_or(&(u64::from(id.index() as u32), 0))
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
}
