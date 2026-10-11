//! Page commands: add, delete, canvas size.

use doc::{Cell, Document, Layer, LayerList, Page, PageInfo};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::Delta;
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the page commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(AddPage)?;
    registry.register(DeletePage)?;
    registry.register(SetPageCanvasSize)
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
}
