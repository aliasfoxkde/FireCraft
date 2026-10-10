//! Shared test support: a structural sample document and byte-level
//! document equality.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use doc::{
    Cell, Contour, Document, Layer, LayerList, ObjectData, Page, PageInfo, PathObject, PathPoint,
};

pub fn point(x: f64, y: f64) -> PathPoint {
    PathPoint {
        anchor: geom::Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    }
}

pub fn path_object(seed: u64) -> ObjectData {
    ObjectData::Path(PathObject {
        contours: vec![Contour {
            closed: true,
            point_count: 4,
            points: vec![
                point(0.0, 0.0),
                point(seed as f64 + 8.0, 0.0),
                point(seed as f64 + 8.0, 8.0),
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
    })
}

/// Two pages, two layers per page (two cells each — two states), two
/// path objects per cell: page/layer/cell indices are all live.
pub fn sample_document() -> (Document, Vec<doc::ObjectId>) {
    let mut document = Document::default();
    let mut ids = Vec::new();
    for page_index in 0..2 {
        let mut layers = LayerList::default();
        for layer_index in 0..2 {
            let mut layer = Layer {
                name: format!("Layer {layer_index}"),
                display: true,
                ..Layer::default()
            };
            for _ in 0..2 {
                let mut cell = Cell {
                    locked: false,
                    visible: true,
                    elements: Vec::new(),
                };
                for _ in 0..2 {
                    let id = document.registry.insert(path_object(ids.len() as u64 + 1));
                    cell.elements.push(id);
                    ids.push(id);
                }
                layer.cells.push(cell);
            }
            layers.layers.push(layer);
        }
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
            },
            name: Some(format!("Page {page_index}")),
            is_master: false,
            is_linked: false,
            page_id: None,
            last_export_dir: None,
            last_export_file: None,
            layers,
            grid: None,
            guides: Default::default(),
        });
    }
    (document, ids)
}

/// Byte-level document equality (`serde_json`, field order is
/// declaration order — deterministic).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentBytes(Vec<u8>);

impl DocumentBytes {
    pub fn of(document: &Document) -> Self {
        Self(serde_json::to_vec(document).expect("document serializes"))
    }
}
