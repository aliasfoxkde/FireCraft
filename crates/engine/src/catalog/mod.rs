//! Built-in command catalog.
//!
//! Every command id is a stable surface for UI, CLI, journal, and (M9)
//! scripting. Payloads use `deny_unknown_fields` — a typo'd key is a
//! rejected payload, never a silently ignored one.

pub(crate) mod arrange;
pub(crate) mod autoshape;
pub(crate) mod bitmap;
pub(crate) mod cell;
pub(crate) mod display;
pub(crate) mod document;
pub(crate) mod filter;
pub(crate) mod layer;
pub(crate) mod mask;
pub(crate) mod object;
pub(crate) mod page;
pub(crate) mod path;
pub(crate) mod retouch;
pub(crate) mod style;
pub(crate) mod symbol;
pub(crate) mod text;
pub(crate) mod web;

use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the built-in catalog into `registry`.
pub fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    object::install(registry)?;
    arrange::install(registry)?;
    bitmap::install(registry)?;
    retouch::install(registry)?;
    autoshape::install(registry)?;
    filter::install(registry)?;
    path::install(registry)?;
    cell::install(registry)?;
    layer::install(registry)?;
    page::install(registry)?;
    document::install(registry)?;
    text::install(registry)?;
    symbol::install(registry)?;
    web::install(registry)?;
    mask::install(registry)?;
    style::install(registry)
}

/// Every registered built-in, sorted (surfaces enumerate this).
#[must_use]
pub fn ids() -> Vec<&'static str> {
    let mut registry = CommandRegistry::new();
    let _ = install(&mut registry);
    registry.names()
}

/// Unit-test fixture shared by the catalog modules: two pages, two
/// layers each, two visible cells (states) per layer, two path objects
/// per first cell. Integration tests keep their own copy in
/// `tests/support` (unit fixtures are `pub(crate)`, invisible across
/// the crate boundary).
#[cfg(test)]
pub(crate) mod testdoc {
    use doc::{
        Cell, Contour, Document, Layer, LayerList, ObjectData, Page, PageInfo, PathObject,
        PathPoint,
    };

    fn point(x: f64, y: f64) -> PathPoint {
        PathPoint {
            anchor: geom::Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        }
    }

    fn path_object(seed: u64) -> ObjectData {
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
            effects: Vec::new(),
        })
    }

    /// Two pages, two layers per page (two cells each — two states),
    /// two path objects in each first cell.
    pub(crate) fn sample_document() -> (Document, Vec<doc::ObjectId>) {
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
                for cell_index in 0..2 {
                    let mut cell = Cell {
                        locked: false,
                        visible: true,
                        elements: Vec::new(),
                    };
                    if cell_index == 0 {
                        for _ in 0..2 {
                            let id = document.registry.insert(path_object(ids.len() as u64 + 1));
                            cell.elements.push(id);
                            ids.push(id);
                        }
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
                    frame_durations: Vec::new(),
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
                export: None,
            });
        }
        (document, ids)
    }
}
