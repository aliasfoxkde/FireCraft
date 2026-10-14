//! Page commands: add, delete, canvas size, guides, grid, onion
//! skinning.

use doc::{Cell, Document, Grid, Layer, LayerList, Page, PageInfo};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::Delta;
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the page commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(AddPage)?;
    registry.register(DeletePage)?;
    registry.register(SetPageCanvasSize)?;
    registry.register(AddGuide)?;
    registry.register(ChangeGuide)?;
    registry.register(RemoveGuide)?;
    registry.register(RemoveAllGuides)?;
    registry.register(SetGridOrigin)?;
    registry.register(SetGridSize)?;
    registry.register(SetOnionSkinning)
}

/// Which axis a guide runs along (CS6's `guidekind` string).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum GuideKind {
    /// A horizontal guide — positions are y coordinates.
    Horizontal,
    /// A vertical guide — positions are x coordinates.
    Vertical,
}

impl GuideKind {
    /// The matching position list of a `Guides`.
    fn slot(self, guides: &doc::Guides) -> &Vec<f64> {
        match self {
            GuideKind::Horizontal => &guides.horizontal,
            GuideKind::Vertical => &guides.vertical,
        }
    }

    fn slot_mut(self, guides: &mut doc::Guides) -> &mut Vec<f64> {
        match self {
            GuideKind::Horizontal => &mut guides.horizontal,
            GuideKind::Vertical => &mut guides.vertical,
        }
    }
}

/// Rejects non-finite guide coordinates (NaN/inf would poison the
/// wire and every consumer's snapping).
fn finite_position(name: &'static str, position: f64) -> Result<(), EngineError> {
    if position.is_finite() {
        return Ok(());
    }
    Err(EngineError::InvalidState {
        name: name.to_owned(),
        detail: format!("guide position {position} is not finite"),
    })
}

/// The grid a page currently renders with: its own override, or the
/// document grid when the page carries none.
fn effective_grid(document: &Document, page: usize) -> Grid {
    document.pages[page]
        .grid
        .clone()
        .unwrap_or_else(|| document.grid.clone())
}

/// `page.add` — appends (or inserts at `index`) a fresh page: one
/// visible layer whose cell count matches the document's frame count,
/// canvas copied from the document node.
struct AddPage;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddPagePayload {
    /// Insert position; absent appends.
    index: Option<usize>,
}

impl TypedCommand for AddPage {
    type Payload = AddPagePayload;

    fn name(&self) -> &'static str {
        "page.add"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.add";
        let len = document.pages.len();
        let at = payload.index.unwrap_or(len);
        if at > len {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("insert index {at} beyond {len} pages"),
            });
        }
        // At least one state: a page with no cells has nothing to edit.
        let frame_count = document.info.frame_count.max(1);
        let cells = (0..frame_count)
            .map(|_| Cell {
                locked: false,
                visible: true,
                elements: Vec::new(),
            })
            .collect();
        let layer = Layer {
            name: "Layer 1".to_owned(),
            display: true,
            cells,
            ..Layer::default()
        };
        let page = Page {
            info: PageInfo {
                version: document.info.version,
                revision: document.info.revision,
                frame_count,
                frame_count_duplicate: frame_count,
                request_id: 0,
                canvas: document.info.canvas.clone(),
                onion_before: document.info.onion_before,
                onion_after: document.info.onion_after,
                frame_durations: Vec::new(),
            },
            name: None,
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
        };
        document.pages.insert(at, page);
        Ok(Delta::Page {
            index: at,
            before: None,
        })
    }
}

/// `page.delete` — removes a page. The last page cannot be deleted: a
/// Fireworks document always has at least one.
struct DeletePage;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DeletePagePayload {
    /// Page index.
    page: usize,
}

impl TypedCommand for DeletePage {
    type Payload = DeletePagePayload;

    fn name(&self) -> &'static str {
        "page.delete"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.delete";
        if document.pages.len() <= 1 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "cannot delete the last page".to_owned(),
            });
        }
        let len = document.pages.len();
        let before = document.pages.get(payload.page).cloned().ok_or_else(|| {
            EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("page {} out of range ({len} pages)", payload.page),
            }
        })?;
        document.pages.remove(payload.page);
        Ok(Delta::Page {
            index: payload.page,
            before: Some(Box::new(before)),
        })
    }
}

/// `page.setCanvasSize` — sets one page's canvas width and height.
/// Values must be finite and within 1..65536 (Fireworks' pixel bound).
struct SetPageCanvasSize;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetPageCanvasSizePayload {
    /// Page index.
    page: usize,
    /// New canvas width, pixels.
    width: f64,
    /// New canvas height, pixels.
    height: f64,
}

impl TypedCommand for SetPageCanvasSize {
    type Payload = SetPageCanvasSizePayload;

    fn name(&self) -> &'static str {
        "page.setCanvasSize"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.setCanvasSize";
        const BOUND: f64 = 65535.0;
        for (label, value) in [("width", payload.width), ("height", payload.height)] {
            if !value.is_finite() || !(1.0..=BOUND).contains(&value) {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("{label} {value} outside 1..=65535"),
                });
            }
        }
        let target = access::page_mut(document, NAME, payload.page)?;
        let before = target.info.clone();
        target.info.canvas.width = payload.width;
        target.info.canvas.height = payload.height;
        Ok(Delta::PageInfo {
            page: payload.page,
            before: Box::new(before),
        })
    }
}

/// `page.addGuide` — CS6's `dom.addGuide(position, guidekind)`: one
/// page-scoped guide line at `position` along the chosen axis.
/// Duplicates are allowed (CS6 appends without dedup).
struct AddGuide;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddGuidePayload {
    /// Page index.
    page: usize,
    /// Position in canvas coordinates (y for horizontal, x for
    /// vertical).
    position: f64,
    /// Which axis the guide runs along.
    kind: GuideKind,
}

impl TypedCommand for AddGuide {
    type Payload = AddGuidePayload;

    fn name(&self) -> &'static str {
        "page.addGuide"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.addGuide";
        finite_position(NAME, payload.position)?;
        let target = access::page_mut(document, NAME, payload.page)?;
        let before = target.guides.clone();
        payload
            .kind
            .slot_mut(&mut target.guides)
            .push(payload.position);
        Ok(Delta::Guides {
            page: payload.page,
            before,
        })
    }
}

/// `page.changeGuide` — CS6's `dom.changeGuide(currentPosition,
/// newPosition, guidekind)`: moves the first guide sitting at
/// `current` to `new`. Unlike `removeGuide`, CS6 documents no
/// absent-position no-op here, so a miss is a refusal.
struct ChangeGuide;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ChangeGuidePayload {
    /// Page index.
    page: usize,
    /// Position of the guide to move.
    current: f64,
    /// New position for that guide.
    new: f64,
    /// Which axis the guide runs along.
    kind: GuideKind,
}

impl TypedCommand for ChangeGuide {
    type Payload = ChangeGuidePayload;

    fn name(&self) -> &'static str {
        "page.changeGuide"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.changeGuide";
        finite_position(NAME, payload.current)?;
        finite_position(NAME, payload.new)?;
        let target = access::page_mut(document, NAME, payload.page)?;
        let slot = payload.kind.slot(&target.guides);
        let at = slot
            .iter()
            .position(|position| *position == payload.current)
            .ok_or_else(|| EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!(
                    "no {} guide at {}",
                    match payload.kind {
                        GuideKind::Horizontal => "horizontal",
                        GuideKind::Vertical => "vertical",
                    },
                    payload.current
                ),
            })?;
        let before = target.guides.clone();
        payload.kind.slot_mut(&mut target.guides)[at] = payload.new;
        Ok(Delta::Guides {
            page: payload.page,
            before,
        })
    }
}

/// `page.removeGuide` — CS6's `dom.removeGuide(position, guidekind)`.
/// CS6 documents the absent case as a no-op ("has no effect"), so a
/// miss succeeds without touching the document.
struct RemoveGuide;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RemoveGuidePayload {
    /// Page index.
    page: usize,
    /// Position of the guide to remove.
    position: f64,
    /// Which axis the guide runs along.
    kind: GuideKind,
}

impl TypedCommand for RemoveGuide {
    type Payload = RemoveGuidePayload;

    fn name(&self) -> &'static str {
        "page.removeGuide"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.removeGuide";
        finite_position(NAME, payload.position)?;
        let target = access::page_mut(document, NAME, payload.page)?;
        let slot = payload.kind.slot(&target.guides);
        let Some(at) = slot
            .iter()
            .position(|position| *position == payload.position)
        else {
            return Ok(Delta::Noop);
        };
        let before = target.guides.clone();
        payload.kind.slot_mut(&mut target.guides).remove(at);
        Ok(Delta::Guides {
            page: payload.page,
            before,
        })
    }
}

/// `page.removeAllGuides` — CS6's `dom.removeAllGuides([guidekind])`:
/// clears one axis, or both when the kind is absent (the no-arg form).
/// Already-empty targets are a no-op.
struct RemoveAllGuides;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RemoveAllGuidesPayload {
    /// Page index.
    page: usize,
    /// Clear only this axis; absent clears both.
    kind: Option<GuideKind>,
}

impl TypedCommand for RemoveAllGuides {
    type Payload = RemoveAllGuidesPayload;

    fn name(&self) -> &'static str {
        "page.removeAllGuides"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.removeAllGuides";
        let target = access::page_mut(document, NAME, payload.page)?;
        let before = target.guides.clone();
        match payload.kind {
            Some(kind) => *kind.slot_mut(&mut target.guides) = Vec::new(),
            None => {
                target.guides.horizontal = Vec::new();
                target.guides.vertical = Vec::new();
            }
        }
        if target.guides == before {
            return Ok(Delta::Noop);
        }
        Ok(Delta::Guides {
            page: payload.page,
            before,
        })
    }
}

/// `page.setGridOrigin` — CS6's `dom.setGridOrigin(point)`. Sets the
/// page's grid override (created from the effective grid when the page
/// carries none); the document grid is untouched.
struct SetGridOrigin;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetGridOriginPayload {
    /// Page index.
    page: usize,
    /// Grid origin, world coordinates.
    x: f64,
    /// Grid origin, world coordinates.
    y: f64,
}

impl TypedCommand for SetGridOrigin {
    type Payload = SetGridOriginPayload;

    fn name(&self) -> &'static str {
        "page.setGridOrigin"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.setGridOrigin";
        finite_position(NAME, payload.x)?;
        finite_position(NAME, payload.y)?;
        access::page(document, NAME, payload.page)?;
        let mut grid = effective_grid(document, payload.page);
        let target = access::page_mut(document, NAME, payload.page)?;
        let before = target.grid.clone();
        grid.offset = geom::Point::new(payload.x, payload.y);
        target.grid = Some(grid);
        Ok(Delta::PageGrid {
            page: payload.page,
            before: before.map(Box::new),
        })
    }
}

/// `page.setGridSize` — CS6's `dom.setGridSize(point)`. Both extents
/// must be finite and positive (a zero or negative cell is degenerate
/// — CS6's Edit Grid dialog cannot produce one).
struct SetGridSize;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetGridSizePayload {
    /// Page index.
    page: usize,
    /// Cell width.
    x: f64,
    /// Cell height.
    y: f64,
}

impl TypedCommand for SetGridSize {
    type Payload = SetGridSizePayload;

    fn name(&self) -> &'static str {
        "page.setGridSize"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.setGridSize";
        for (label, value) in [("x", payload.x), ("y", payload.y)] {
            if !value.is_finite() || value <= 0.0 {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("grid size {label} {value} must be finite and > 0"),
                });
            }
        }
        access::page(document, NAME, payload.page)?;
        let mut grid = effective_grid(document, payload.page);
        let target = access::page_mut(document, NAME, payload.page)?;
        let before = target.grid.clone();
        grid.size = geom::Point::new(payload.x, payload.y);
        target.grid = Some(grid);
        Ok(Delta::PageGrid {
            page: payload.page,
            before: before.map(Box::new),
        })
    }
}

/// `page.setOnionSkinning` — CS6's `dom.setOnionSkinning(before,
/// after)`: frames shown before/after the current one. `0, 0`
/// disables; negative counts are refused.
struct SetOnionSkinning;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetOnionSkinningPayload {
    /// Page index.
    page: usize,
    /// `OSBi` — frames shown before the current one.
    before: i64,
    /// `OSAi` — frames shown after the current one.
    after: i64,
}

impl TypedCommand for SetOnionSkinning {
    type Payload = SetOnionSkinningPayload;

    fn name(&self) -> &'static str {
        "page.setOnionSkinning"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "page.setOnionSkinning";
        if payload.before < 0 || payload.after < 0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "onion frame counts must be >= 0, got {}/{}",
                    payload.before, payload.after
                ),
            });
        }
        let target = access::page_mut(document, NAME, payload.page)?;
        let before_info = target.info.clone();
        target.info.onion_before = payload.before;
        target.info.onion_after = payload.after;
        Ok(Delta::PageInfo {
            page: payload.page,
            before: Box::new(before_info),
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
    fn add_appends_a_page_and_undo_removes_it() {
        let (mut document, ids) = testdoc::sample_document();
        // The document node's frame count is authoritative for the
        // new page's state count (the fixture's pages say 2, the
        // document node says 1 — the node wins).
        let expected_states = document.info.frame_count.max(1) as usize;
        let original = serde_json::to_vec(&document).unwrap();
        let delta = AddPage.apply_json(&mut document, json!({})).unwrap();
        assert_eq!(document.pages.len(), 3);
        let added = &document.pages[2];
        assert_eq!(
            added.info.frame_count as usize, expected_states,
            "cells match document states"
        );
        assert_eq!(added.layers.layers[0].cells.len(), expected_states);
        assert!(added.layers.layers[0].cells.iter().all(|cell| cell.visible));
        assert_eq!(added.layers.layers[0].cells[0].elements.len(), 0);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages.len(), 2);
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
        assert!(document.registry.get(ids[0]).is_some());
    }

    #[test]
    fn add_at_an_index_inserts() {
        let (mut document, _) = testdoc::sample_document();
        AddPage
            .apply_json(&mut document, json!({ "index": 1 }))
            .unwrap();
        assert_eq!(document.pages[1].name, None);
        assert_eq!(document.pages[2].name, Some("Page 1".to_owned()));
        // Beyond the end is refused.
        let error = AddPage
            .apply_json(&mut document, json!({ "index": 9 }))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }

    #[test]
    fn delete_removes_and_undo_restores() {
        let (mut document, _) = testdoc::sample_document();
        let delta = DeletePage
            .apply_json(&mut document, json!({ "page": 0 }))
            .unwrap();
        assert_eq!(document.pages.len(), 1);
        assert_eq!(document.pages[0].name, Some("Page 1".to_owned()));
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages.len(), 2);
        assert_eq!(document.pages[0].name, Some("Page 0".to_owned()));
    }

    #[test]
    fn the_last_page_cannot_be_deleted() {
        let (mut document, _) = testdoc::sample_document();
        DeletePage
            .apply_json(&mut document, json!({ "page": 1 }))
            .unwrap();
        let error = DeletePage
            .apply_json(&mut document, json!({ "page": 0 }))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(document.pages.len(), 1, "document untouched");
    }

    #[test]
    fn canvas_size_sets_and_validates() {
        let (mut document, _) = testdoc::sample_document();
        let delta = SetPageCanvasSize
            .apply_json(
                &mut document,
                json!({ "page": 0, "width": 640.0, "height": 480.0 }),
            )
            .unwrap();
        assert_eq!(document.pages[0].info.canvas.width, 640.0);
        assert_eq!(
            document.pages[1].info.canvas.width, 0.0,
            "other pages untouched"
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].info.canvas.width, 0.0);

        for (width, height) in [(0.0, 100.0), (-5.0, 100.0), (100.0, 70000.0)] {
            let error = SetPageCanvasSize
                .apply_json(
                    &mut document,
                    json!({ "page": 0, "width": width, "height": height }),
                )
                .unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }), "{width}");
        }
    }

    #[test]
    fn out_of_range_page_is_missing_target() {
        let (mut document, _) = testdoc::sample_document();
        let payload = json!({ "page": 5, "width": 100.0, "height": 100.0 });
        let error = SetPageCanvasSize
            .apply_json(&mut document, payload)
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
    }

    #[test]
    fn guides_add_change_remove_round_trip() {
        let (mut document, _) = testdoc::sample_document();
        let add_h = AddGuide
            .apply_json(
                &mut document,
                json!({ "page": 0, "position": 32.0, "kind": "horizontal" }),
            )
            .unwrap();
        AddGuide
            .apply_json(
                &mut document,
                json!({ "page": 0, "position": 64.0, "kind": "vertical" }),
            )
            .unwrap();
        assert_eq!(document.pages[0].guides.horizontal, vec![32.0]);
        assert_eq!(document.pages[0].guides.vertical, vec![64.0]);
        assert!(
            document.pages[1].guides.horizontal.is_empty(),
            "page-scoped"
        );

        let moved = ChangeGuide
            .apply_json(
                &mut document,
                json!({ "page": 0, "current": 32.0, "new": 48.0, "kind": "horizontal" }),
            )
            .unwrap();
        assert_eq!(document.pages[0].guides.horizontal, vec![48.0]);
        moved.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].guides.horizontal, vec![32.0]);

        let removed = RemoveGuide
            .apply_json(
                &mut document,
                json!({ "page": 0, "position": 32.0, "kind": "horizontal" }),
            )
            .unwrap();
        assert!(document.pages[0].guides.horizontal.is_empty());
        removed.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].guides.horizontal, vec![32.0]);

        // CS6: removing an absent guide "has no effect" — a no-op, not
        // a refusal.
        let absent = RemoveGuide
            .apply_json(
                &mut document,
                json!({ "page": 0, "position": 999.0, "kind": "horizontal" }),
            )
            .unwrap();
        assert!(matches!(absent, Delta::Noop));
        add_h.apply_inverse(&mut document).unwrap();
        assert!(document.pages[0].guides.horizontal.is_empty());

        // A move with no guide at `current` is refused without
        // mutating (CS6 documents no no-op for changeGuide).
        let before = serde_json::to_vec(&document).unwrap();
        let error = ChangeGuide
            .apply_json(
                &mut document,
                json!({ "page": 0, "current": 999.0, "new": 1.0, "kind": "horizontal" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn remove_all_guides_clears_one_axis_or_both() {
        let (mut document, _) = testdoc::sample_document();
        for kind in ["horizontal", "vertical"] {
            AddGuide
                .apply_json(
                    &mut document,
                    json!({ "page": 0, "position": 10.0, "kind": kind }),
                )
                .unwrap();
        }
        let cleared = RemoveAllGuides
            .apply_json(&mut document, json!({ "page": 0, "kind": "vertical" }))
            .unwrap();
        assert!(document.pages[0].guides.vertical.is_empty());
        assert_eq!(document.pages[0].guides.horizontal, vec![10.0]);
        cleared.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].guides.vertical, vec![10.0]);

        let cleared = RemoveAllGuides
            .apply_json(&mut document, json!({ "page": 0 }))
            .unwrap();
        assert!(document.pages[0].guides.horizontal.is_empty());
        assert!(document.pages[0].guides.vertical.is_empty());
        cleared.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].guides.horizontal, vec![10.0]);

        // Already empty: a no-op.
        RemoveAllGuides
            .apply_json(&mut document, json!({ "page": 0 }))
            .unwrap();
        let again = RemoveAllGuides
            .apply_json(&mut document, json!({ "page": 0 }))
            .unwrap();
        assert!(matches!(again, Delta::Noop));
    }

    #[test]
    fn grid_commands_write_the_page_override() {
        let (mut document, _) = testdoc::sample_document();
        assert!(
            document.pages[0].grid.is_none(),
            "page inherits the doc grid"
        );
        let origin = SetGridOrigin
            .apply_json(&mut document, json!({ "page": 0, "x": 4.0, "y": 8.0 }))
            .unwrap();
        let grid = document.pages[0].grid.as_ref().unwrap();
        assert_eq!(grid.offset, geom::Point::new(4.0, 8.0));
        assert_eq!(
            grid.size, document.grid.size,
            "the override starts from the effective grid"
        );
        origin.apply_inverse(&mut document).unwrap();
        assert!(document.pages[0].grid.is_none(), "undo drops the override");

        let size = SetGridSize
            .apply_json(&mut document, json!({ "page": 0, "x": 32.0, "y": 16.0 }))
            .unwrap();
        let grid = document.pages[0].grid.as_ref().unwrap();
        assert_eq!(grid.size, geom::Point::new(32.0, 16.0));
        assert_eq!(grid.offset, document.grid.offset);
        size.apply_inverse(&mut document).unwrap();
        assert!(document.pages[0].grid.is_none());

        // Degenerate sizes are refused without mutating.
        let before = serde_json::to_vec(&document).unwrap();
        for (x, y) in [(0.0, 10.0), (-4.0, 10.0)] {
            let error = SetGridSize
                .apply_json(&mut document, json!({ "page": 0, "x": x, "y": y }))
                .unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }), "{x}");
        }
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);

        // An out-of-range page is a refusal, never a panic (the
        // effective grid reads the page before the mutation does).
        let error = SetGridOrigin
            .apply_json(&mut document, json!({ "page": 9, "x": 0.0, "y": 0.0 }))
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
    }

    #[test]
    fn onion_skinning_sets_frame_counts() {
        let (mut document, _) = testdoc::sample_document();
        let delta = SetOnionSkinning
            .apply_json(&mut document, json!({ "page": 0, "before": 2, "after": 3 }))
            .unwrap();
        assert_eq!(document.pages[0].info.onion_before, 2);
        assert_eq!(document.pages[0].info.onion_after, 3);
        assert_eq!(document.pages[1].info.onion_before, 0, "page-scoped");
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.pages[0].info.onion_before, 0);

        // Zero/zero is CS6's "disable" form; negatives are refused.
        SetOnionSkinning
            .apply_json(&mut document, json!({ "page": 0, "before": 0, "after": 0 }))
            .unwrap();
        let before = serde_json::to_vec(&document).unwrap();
        let error = SetOnionSkinning
            .apply_json(
                &mut document,
                json!({ "page": 0, "before": -1, "after": 0 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }
}
