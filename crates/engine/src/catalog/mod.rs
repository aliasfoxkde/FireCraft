//! Built-in command catalog.
//!
//! Every command id is a stable surface for UI, CLI, journal, and (M9)
//! scripting. Payloads use `deny_unknown_fields` — a typo'd key is a
//! rejected payload, never a silently ignored one.

pub(crate) mod arrange;
pub(crate) mod autoshape;
pub(crate) mod bake;
pub(crate) mod bitmap;
pub(crate) mod cell;
pub(crate) mod clipboard;
pub(crate) mod display;
pub(crate) mod document;
pub(crate) mod filter;
pub(crate) mod flatten;
pub(crate) mod layer;
pub(crate) mod mask;
pub(crate) mod object;
pub(crate) mod page;
pub(crate) mod path;
pub(crate) mod primitive;
pub(crate) mod retouch;
pub(crate) mod selection;
pub(crate) mod style;
pub(crate) mod symbol;
pub(crate) mod text;
pub(crate) mod web;

use crate::error::EngineError;
use crate::registry::CommandRegistry;
use doc::{Document, ObjectId};

/// Registers the built-in catalog into `registry`.
pub fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    object::install(registry)?;
    arrange::install(registry)?;
    bitmap::install(registry)?;
    clipboard::install(registry)?;
    retouch::install(registry)?;
    autoshape::install(registry)?;
    filter::install(registry)?;
    bake::install(registry)?;
    flatten::install(registry)?;
    path::install(registry)?;
    cell::install(registry)?;
    layer::install(registry)?;
    page::install(registry)?;
    primitive::install(registry)?;
    document::install(registry)?;
    selection::install(registry)?;
    text::install(registry)?;
    symbol::install(registry)?;
    web::install(registry)?;
    mask::install(registry)?;
    style::install(registry)
}

/// The element-mask state of `id`: `Some(true)` when `id` carries an
/// enabled element mask, `Some(false)` when the mask is present but
/// disabled, `None` when `id` carries no element mask.
///
/// The mask family's read side. CS6's Modify ▸ Mask enable/disable is
/// one row whose visible label flips with the mask's state, so a
/// shell needs to read the state to render that row and to pick the
/// direction of the toggle.
#[must_use]
pub fn element_mask_state(document: &Document, id: ObjectId) -> Option<bool> {
    mask::element_mask_state(document, id)
}

/// The mask group behind `id` and the combine mode of its level.
///
/// The mask stack's read side: `id` is the mask group itself, the
/// mask it holds, or the object it masks. `None` when `id` carries no
/// element mask. The combine mode is the level's mask-stack mode —
/// `None` is the default union.
#[must_use]
pub fn element_mask_stack_mode(
    document: &Document,
    id: ObjectId,
) -> Option<(ObjectId, Option<doc::MaskCombineMode>)> {
    mask::element_mask_stack_mode(document, id)
}

/// One level of an element mask stack.
pub use mask::MaskLevel;

/// The mask stack behind `id`, top level first.
///
/// `id` is the stack's top group, any level's group, the mask a level
/// holds, or the object the stack restricts. Empty when `id` carries
/// no element mask. A disabled level is skipped — it contributes no
/// coverage, so the level below it takes its place.
#[must_use]
pub fn element_mask_stack(document: &Document, id: ObjectId) -> Vec<MaskLevel> {
    mask::element_mask_stack(document, id)
}

/// Every registered built-in, sorted (surfaces enumerate this).
#[must_use]
pub fn ids() -> Vec<&'static str> {
    let mut registry = CommandRegistry::new();
    let _ = install(&mut registry);
    registry.names()
}

/// The session-scoped command ids, sorted.
///
/// These are *not* in [`ids`]: they act on the session's undo
/// history rather than on the document, so the registry cannot own
/// them. A surface that enumerates commands (the command palette, the
/// CLI, the MCP bridge) offers both lists.
#[must_use]
pub fn session_ids() -> Vec<&'static str> {
    vec![crate::session::REWIND_ID]
}

/// Every command a surface can run: the registry catalog plus the
/// session-scoped ids, sorted.
#[must_use]
pub fn all_ids() -> Vec<&'static str> {
    let mut ids = ids();
    ids.extend(session_ids());
    ids.sort_unstable();
    ids
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

    pub(crate) fn path_object(seed: u64) -> ObjectData {
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

    /// A graphic definition numbered `number`, carrying the CS6
    /// button-behavior defaults (decision 0036) and no content.
    pub(crate) fn definition(number: i64) -> doc::SymbolDefinition {
        doc::SymbolDefinition {
            name: format!("definition {number}"),
            symbol_number: number,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList::default(),
            background_raw: 0,
            symbol_type: doc::SymbolType::Graphic,
            nine_slice: None,
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
        }
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
