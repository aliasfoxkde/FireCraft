//! The flatten family (decision 0031): CS6's bake-to-bitmap commands
//! — Merge Down, Flatten Selection, Flatten Document — backed by the
//! compositor. Each command renders its participants through
//! `compose::render_subset` (object pixels only, no canvas matte: the
//! document's canvas color stays a live property after the bake),
//! writes the pixels into fresh `mkBT` bank tiles, and replaces the
//! participants with one canvas-sized [`doc::ImageObject`] — Fireworks
//! bitmaps are canvas-sized objects, so the extent is faithful.
//!
//! Baked pixels carry everything the compositor saw: paint, opacity,
//! blend, live effects. The result therefore carries the default
//! weight (opacity 1000, blend 0, no effects) — re-applying the source
//! object's weight would double it.

use doc::{Cell, Document, ImageObject, Layer, LayerList, ObjectData, ObjectId, TileRef};
use raster::{TILE_BYTES, TILE_SIDE};

use crate::access;
use crate::catalog::object::{Owner, attach_cell, detach, find_owner};
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the flatten family.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(MergeDown)?;
    registry.register(FlattenSelection)?;
    registry.register(FlattenDocument)
}

/// The next free bank slot: one above the highest live id (0 for an
/// empty bank) — the same replay-stable law as symbol numbers.
pub(crate) fn next_tile_id(document: &Document) -> u32 {
    document.tiles.ids().max().map_or(0, |id| id + 1)
}

/// Crops a finished render into the `TILE_SIDE` bank grid, tail slots
/// padded transparent, inserting each tile under a fresh id. Returns
/// the minted ids (the caller captures them in a [`Delta::Bank`]) and
/// the canvas extent in pixels.
fn bake_from_render(document: &mut Document, render: &compose::Render) -> (Vec<u32>, i64, i64) {
    let (width, height) = (i64::from(render.width), i64::from(render.height));
    let side = i64::from(TILE_SIDE as u32);
    let cols = (width / side + i64::from(width % side != 0)) as usize;
    let rows = (height / side + i64::from(height % side != 0)) as usize;
    let stride = usize::from(render.width) * 4;
    let mut minted = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            let mut pixels = [0u8; TILE_BYTES];
            let (base_x, base_y) = (col as i64 * side, row as i64 * side);
            let copy_w = (width - base_x).min(side) as usize;
            let copy_h = (height - base_y).min(side) as usize;
            for line in 0..copy_h {
                let src = (base_y + line as i64) as usize * stride + base_x as usize * 4;
                let dst = line * TILE_SIDE * 4;
                pixels[dst..dst + copy_w * 4]
                    .copy_from_slice(&render.pixels[src..src + copy_w * 4]);
            }
            let id = next_tile_id(document);
            document.tiles.insert(id, raster::Tile::from_rgba8(pixels));
            minted.push(id);
        }
    }
    (minted, width, height)
}

/// Renders the listed ids (list order = paint order, bottom-first)
/// and bakes the result into fresh bank tiles.
fn bake_subset(
    document: &mut Document,
    page: usize,
    ids: &[ObjectId],
    name: &'static str,
) -> Result<(Vec<u32>, i64, i64), EngineError> {
    let render = compose::render_subset(document, page, 0, ids).map_err(|error| {
        EngineError::InvalidState {
            name: name.to_owned(),
            detail: error.to_string(),
        }
    })?;
    Ok(bake_from_render(document, &render))
}

/// The canvas-sized bitmap over freshly baked tiles.
fn baked_image(minted: &[u32], width: i64, height: i64) -> ImageObject {
    ImageObject {
        location: geom::Point::ZERO,
        offset: geom::Point::ZERO,
        pixels: (width, height),
        tile_size: i64::from(TILE_SIDE as u32),
        tiles: minted.iter().map(|id| TileRef::Tile(*id)).collect(),
        name: None,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        effects: Vec::new(),
    }
}

/// One selected object's cell placement.
#[derive(Clone, Copy)]
struct Placement {
    id: ObjectId,
    page: usize,
    layer: usize,
    cell: usize,
    position: usize,
}

/// Resolves every selected object to a cell placement; a group-nested
/// (or unplaced) selection refuses — the bake needs a z-position.
fn placements(document: &Document, name: &'static str) -> Result<Vec<Placement>, EngineError> {
    let mut placements = Vec::new();
    for id in &document.object_selection {
        match find_owner(document, *id) {
            Some(Owner::Cell {
                page,
                layer,
                cell,
                position,
            }) => placements.push(Placement {
                id: *id,
                page,
                layer,
                cell,
                position,
            }),
            _ => {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: format!(
                        "selected object {} must sit in a cell (not nested in groups)",
                        id.index()
                    ),
                });
            }
        }
    }
    Ok(placements)
}

/// Detaches and removes each captured member, pushing the removal
/// deltas onto `deltas` (undo revives and re-places them LIFO).
fn remove_members(
    document: &mut Document,
    name: &'static str,
    members: &[(ObjectId, ObjectData)],
    deltas: &mut Vec<Delta>,
) -> Result<(), EngineError> {
    for (id, before) in members {
        deltas.push(detach(document, name, *id)?);
        if document.registry.remove(*id).is_none() {
            return Err(EngineError::MissingTarget {
                name: name.to_owned(),
                detail: format!("object {} vanished mid-flatten", id.index()),
            });
        }
        deltas.push(Delta::Object {
            id: *id,
            inverse: ObjectInverse::Removed {
                object: Box::new(before.clone()),
            },
        });
    }
    Ok(())
}

/// Captures every member's current object data.
fn capture_members(
    document: &Document,
    members: &[ObjectId],
    name: &'static str,
) -> Result<Vec<(ObjectId, ObjectData)>, EngineError> {
    members
        .iter()
        .map(|id| Ok((*id, access::object(document, name, *id)?.clone())))
        .collect()
}

/// `flatten.mergeDown` — CS6 `dom.mergeDown()`: "Merges selected
/// objects to the bitmap directly below the selected objects. Succeeds
/// only if the object immediately below the selection is a bitmap."
/// The selection and the bitmap must share one cell (Fireworks'
/// z-order is per-state); the merged pixels replace the bitmap in
/// place, so its identity survives the way `path.boolean` keeps `a`.
struct MergeDown;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct MergeDownPayload {}

impl TypedCommand for MergeDown {
    type Payload = MergeDownPayload;

    fn name(&self) -> &'static str {
        "flatten.mergeDown"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "flatten.mergeDown";
        if document.object_selection.is_empty() {
            return Ok(Delta::Noop);
        }
        let placements = placements(document, NAME)?;
        let first = placements[0];
        if placements.iter().any(|placement| {
            (placement.page, placement.layer, placement.cell)
                != (first.page, first.layer, first.cell)
        }) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the selection and the bitmap below must share one cell".to_owned(),
            });
        }
        let min_position = placements
            .iter()
            .map(|placement| placement.position)
            .min()
            .unwrap_or(0);
        if min_position == 0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "nothing sits below the selection".to_owned(),
            });
        }
        // Everything below the lowest selection is unselected by
        // construction, so the immediate neighbor is the bitmap.
        let below_id = document.pages[first.page].layers.layers[first.layer].cells[first.cell]
            .elements[min_position - 1];
        if !matches!(document.registry.get(below_id), Some(ObjectData::Image(_))) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the object immediately below the selection is not a bitmap".to_owned(),
            });
        }
        // Paint order: the bitmap, then the selection bottom-first.
        let mut order: Vec<(usize, ObjectId)> = placements
            .iter()
            .map(|placement| (placement.position, placement.id))
            .collect();
        order.sort_unstable_by_key(|(position, _)| *position);
        let mut ids = Vec::with_capacity(order.len() + 1);
        ids.push(below_id);
        ids.extend(order.into_iter().map(|(_, id)| id));

        let before_below = access::object(document, NAME, below_id)?.clone();
        let member_ids: Vec<ObjectId> = placements.iter().map(|placement| placement.id).collect();
        let members = capture_members(document, &member_ids, NAME)?;
        let (minted, width, height) = bake_subset(document, first.page, &ids, NAME)?;
        match document.registry.get_mut(below_id) {
            Some(ObjectData::Image(image)) => {
                image.location = geom::Point::ZERO;
                image.offset = geom::Point::ZERO;
                image.pixels = (width, height);
                image.tile_size = i64::from(TILE_SIDE as u32);
                image.tiles = minted.iter().map(|id| TileRef::Tile(*id)).collect();
                // The baked pixels carry the old weight already.
                image.opacity = 1000;
                image.blend = 0;
                image.effects = Vec::new();
            }
            _ => {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "bitmap {} (generation {}) vanished between validation and bake",
                        below_id.index(),
                        below_id.generation()
                    ),
                });
            }
        }
        let mut deltas = vec![
            Delta::Bank { added: minted },
            Delta::Object {
                id: below_id,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(before_below),
                },
            },
        ];
        remove_members(document, NAME, &members, &mut deltas)?;
        Ok(Delta::Group(deltas))
    }
}

/// `flatten.selection` — CS6 `dom.flattenSelection()`: "Flattens the
/// selection into a single pixel image. This action is the same
/// behavior as the Merge Images command." The selection may span
/// layers of one page; the result lands where the topmost selected
/// object sat.
struct FlattenSelection;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct FlattenSelectionPayload {}

impl TypedCommand for FlattenSelection {
    type Payload = FlattenSelectionPayload;

    fn name(&self) -> &'static str {
        "flatten.selection"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "flatten.selection";
        if document.object_selection.is_empty() {
            return Ok(Delta::Noop);
        }
        let placements = placements(document, NAME)?;
        let page = placements[0].page;
        if placements.iter().any(|placement| placement.page != page) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the selection must sit on one page".to_owned(),
            });
        }
        // The bake paints the canonical selection order (bottom-first).
        let members = capture_members(document, &document.object_selection, NAME)?;
        let ids = document.object_selection.clone();
        let (minted, width, height) = bake_subset(document, page, &ids, NAME)?;

        // The result takes the topmost member's z-position, minus the
        // same-cell members below it that leave with the bake.
        let top = placements.last().map(|placement| {
            let below = placements
                .iter()
                .filter(|other| {
                    other.id != placement.id
                        && (other.page, other.layer, other.cell)
                            == (placement.page, placement.layer, placement.cell)
                        && other.position < placement.position
                })
                .count();
            (*placement, below)
        });
        let mut deltas = Vec::new();
        remove_members(document, NAME, &members, &mut deltas)?;
        let image = baked_image(&minted, width, height);
        let image_id = document.registry.insert(ObjectData::Image(image));
        if let Some((placement, below)) = top {
            deltas.push(attach_cell(
                document,
                NAME,
                placement.page,
                placement.layer,
                placement.cell,
                image_id,
                Some(placement.position - below),
            )?);
        }
        deltas.push(Delta::Object {
            id: image_id,
            inverse: ObjectInverse::Created,
        });
        deltas.push(Delta::Bank { added: minted });
        Ok(Delta::Group(deltas))
    }
}

/// `flatten.document` — CS6 `dom.flattenDocument()`: "Flattens the
/// entire document into a single pixel image. This is the same
/// behavior as the Merge Layers command." Every content layer of the
/// page collapses into one bitmap on a single bottom content layer;
/// special (web) layers survive untouched, as does the canvas color.
struct FlattenDocument;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct FlattenDocumentPayload {
    /// The page to flatten.
    page: usize,
}

impl TypedCommand for FlattenDocument {
    type Payload = FlattenDocumentPayload;

    fn name(&self) -> &'static str {
        "flatten.document"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "flatten.document";
        let Some(page_data) = document.pages.get(payload.page) else {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!("no page {}", payload.page),
            });
        };
        if page_data.info.frame_count > 1 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "page {} carries {} states; document flatten takes a single-state page",
                    payload.page, page_data.info.frame_count
                ),
            });
        }
        // Content ids in paint order (the layer list is bottom-first).
        let mut content_ids = Vec::new();
        let mut content_name: Option<String> = None;
        for layer in &page_data.layers.layers {
            if layer.is_special {
                continue;
            }
            if content_name.is_none() {
                content_name = Some(layer.name.clone());
            }
            for cell in &layer.cells {
                content_ids.extend(cell.elements.iter().copied());
            }
        }
        if content_ids.is_empty() {
            return Ok(Delta::Noop);
        }
        let render = compose::render_transparent(document, payload.page, 0).map_err(|error| {
            EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: error.to_string(),
            }
        })?;
        let (minted, width, height) = bake_from_render(document, &render);

        let before = Box::new(document.pages[payload.page].clone());
        let image = baked_image(&minted, width, height);
        let image_id = document.registry.insert(ObjectData::Image(image));
        let flat_layer = Layer {
            name: content_name.unwrap_or_default(),
            display: true,
            cells: vec![Cell {
                locked: false,
                visible: true,
                elements: vec![image_id],
            }],
            ..Layer::default()
        };
        // Content collapses to one bottom layer; special layers keep
        // their relative order above it.
        let mut layers = vec![flat_layer];
        layers.extend(
            document.pages[payload.page]
                .layers
                .layers
                .iter()
                .filter(|layer| layer.is_special)
                .cloned(),
        );
        document.pages[payload.page].layers = LayerList { layers };

        let mut deltas = vec![
            Delta::PageBody {
                page: payload.page,
                before,
            },
            Delta::Bank { added: minted },
            Delta::Object {
                id: image_id,
                inverse: ObjectInverse::Created,
            },
        ];
        for id in &content_ids {
            let Some(object) = document.registry.get(*id) else {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!("content object {} vanished mid-flatten", id.index()),
                });
            };
            let object = object.clone();
            if document.registry.remove(*id).is_none() {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!("content object {} vanished mid-flatten", id.index()),
                });
            }
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use doc::{Canvas, Contour, Page, PageInfo, PathObject, PathPoint};
    use serde_json::json;

    fn point(x: f64, y: f64) -> PathPoint {
        PathPoint {
            anchor: geom::Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        }
    }

    /// An opaque square path painting solid `fill_raw`.
    fn square(x0: f64, y0: f64, x1: f64, y1: f64, fill_raw: i64) -> ObjectData {
        ObjectData::Path(PathObject {
            contours: vec![Contour {
                closed: true,
                point_count: 4,
                points: vec![point(x0, y0), point(x1, y0), point(x1, y1), point(x0, y1)],
            }],
            primitive: None,
            even_odd: false,
            paint: Some(doc::PaintStyle {
                stroke_raw: 0,
                fill_raw,
                fill: None,
            }),
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

    /// A 24×24 opaque red bitmap over bank tile 0.
    fn red_bitmap() -> ObjectData {
        ObjectData::Image(ImageObject {
            location: geom::Point::ZERO,
            offset: geom::Point::ZERO,
            pixels: (24, 24),
            tile_size: i64::from(TILE_SIDE as u32),
            tiles: vec![TileRef::Tile(0)],
            name: Some("below".to_owned()),
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        })
    }

    /// One page, one layer, `frames` cells (frame 0 holds `objects`),
    /// a 24×24 canvas, and the red tile loaded at bank slot 0.
    fn bake_document(objects: Vec<ObjectData>, frames: usize) -> (Document, Vec<ObjectId>) {
        let mut document = Document::default();
        let mut ids = Vec::new();
        for object in objects {
            ids.push(document.registry.insert(object));
        }
        let mut cells = Vec::new();
        for (index, cell_elements) in std::iter::once(ids.clone())
            .chain(std::iter::repeat(Vec::new()))
            .take(frames)
            .enumerate()
        {
            let _ = index;
            cells.push(Cell {
                locked: false,
                visible: true,
                elements: cell_elements,
            });
        }
        let layer = Layer {
            name: "Layer 0".to_owned(),
            display: true,
            cells,
            ..Layer::default()
        };
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: frames as i64,
                frame_count_duplicate: frames as i64,
                request_id: 0,
                canvas: Canvas {
                    width: 24.0,
                    height: 24.0,
                    ..Canvas::default()
                },
                onion_before: 0,
                onion_after: 0,
                frame_durations: Vec::new(),
            },
            name: Some("Page 0".to_owned()),
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
        });
        document.tiles.insert(0, red_tile());
        (document, ids)
    }

    fn red_tile() -> raster::Tile {
        let mut pixels = [0u8; TILE_BYTES];
        for rgba in pixels.chunks_exact_mut(4) {
            rgba.copy_from_slice(&[255, 0, 0, 255]);
        }
        raster::Tile::from_rgba8(pixels)
    }

    fn apply_by_name(
        document: &mut Document,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<Delta, EngineError> {
        let mut registry = CommandRegistry::new();
        crate::catalog::install(&mut registry).unwrap();
        registry.execute(document, name, payload)
    }

    fn document_bytes(document: &Document) -> Vec<u8> {
        serde_json::to_vec(document).unwrap()
    }

    /// Premultiplied RGBA8 at (x, y) of the baked bitmap's first tile.
    fn image_pixel(document: &Document, image_id: ObjectId, x: i64, y: i64) -> [u8; 4] {
        match document.registry.get(image_id).unwrap() {
            ObjectData::Image(image) => match image.tiles[0] {
                TileRef::Tile(tile_id) => {
                    let tile = document.tiles.get(tile_id).unwrap().pixels();
                    tile[(y as usize * TILE_SIDE + x as usize) * 4..][..4]
                        .try_into()
                        .unwrap()
                }
                TileRef::Matte(_) => panic!("baked bitmap carries a matte"),
            },
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    const GREEN: i64 = 0xFF_00_FF_00;
    const BLUE: i64 = 0xFF_FF_00_00;

    #[test]
    fn merge_down_bakes_the_selection_into_the_bitmap_below() {
        let (mut document, ids) = bake_document(
            vec![
                red_bitmap(),
                square(0.0, 0.0, 8.0, 8.0, GREEN),
                square(4.0, 4.0, 12.0, 12.0, BLUE),
            ],
            1,
        );
        let below = ids[0];
        document.object_selection = vec![ids[1], ids[2]];
        let before = document_bytes(&document);
        let delta = apply_by_name(&mut document, "flatten.mergeDown", json!({})).unwrap();

        // The bitmap keeps its identity and name; the pixels are fresh.
        let (tiles, name) = match document.registry.get(below).unwrap() {
            ObjectData::Image(image) => (image.tiles.clone(), image.name.clone()),
            other => panic!("wrong object kind: {other:?}"),
        };
        assert_eq!(name.as_deref(), Some("below"));
        assert_eq!(tiles.len(), 1, "one 24×24 tile");
        assert!(matches!(tiles[0], TileRef::Tile(id) if id != 0));
        assert!(
            document.registry.get(ids[1]).is_none(),
            "selection member removed"
        );
        assert!(
            document.registry.get(ids[2]).is_none(),
            "selection member removed"
        );
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![below]
        );
        // Paint order: green below blue; the bitmap shows through.
        assert_eq!(image_pixel(&document, below, 2, 2), [0, 255, 0, 255]);
        assert_eq!(image_pixel(&document, below, 6, 6), [0, 0, 255, 255]);
        assert_eq!(image_pixel(&document, below, 20, 20), [255, 0, 0, 255]);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn merge_down_refusals() {
        // Empty selection is an explicit no-op.
        let (mut document, _) = bake_document(vec![red_bitmap()], 1);
        assert!(matches!(
            apply_by_name(&mut document, "flatten.mergeDown", json!({})).unwrap(),
            Delta::Noop
        ));

        // Nothing below the selection: the bottom object is selected.
        let (mut document, ids) =
            bake_document(vec![red_bitmap(), square(0.0, 0.0, 4.0, 4.0, GREEN)], 1);
        document.object_selection = vec![ids[0]];
        assert!(matches!(
            apply_by_name(&mut document, "flatten.mergeDown", json!({})),
            Err(EngineError::InvalidState { .. })
        ));

        // The object below is a path, not a bitmap.
        let (mut document, ids) = bake_document(
            vec![
                square(0.0, 0.0, 4.0, 4.0, GREEN),
                square(2.0, 2.0, 6.0, 6.0, BLUE),
            ],
            1,
        );
        document.object_selection = vec![ids[1]];
        assert!(matches!(
            apply_by_name(&mut document, "flatten.mergeDown", json!({})),
            Err(EngineError::InvalidState { .. })
        ));

        // The selection spans two cells (states).
        let (mut document, ids) =
            bake_document(vec![red_bitmap(), square(0.0, 0.0, 4.0, 4.0, GREEN)], 2);
        document.pages[0].layers.layers[0].cells[0]
            .elements
            .remove(1);
        document.pages[0].layers.layers[0].cells[1]
            .elements
            .push(ids[1]);
        document.object_selection = vec![ids[0], ids[1]];
        assert!(matches!(
            apply_by_name(&mut document, "flatten.mergeDown", json!({})),
            Err(EngineError::InvalidState { .. })
        ));

        // A selected object that sits in no cell refuses.
        let (mut document, ids) = bake_document(vec![red_bitmap()], 1);
        let orphan = document.registry.insert(square(0.0, 0.0, 4.0, 4.0, GREEN));
        let _ = ids;
        document.object_selection = vec![orphan];
        assert!(matches!(
            apply_by_name(&mut document, "flatten.mergeDown", json!({})),
            Err(EngineError::InvalidState { .. })
        ));
    }

    #[test]
    fn flatten_selection_bakes_across_layers_into_one_bitmap() {
        let mut document = Document::default();
        let green = document.registry.insert(square(0.0, 0.0, 8.0, 8.0, GREEN));
        let blue = document.registry.insert(square(4.0, 4.0, 12.0, 12.0, BLUE));
        let layer_at = |elements: Vec<ObjectId>| Layer {
            name: "Layer".to_owned(),
            display: true,
            cells: vec![Cell {
                locked: false,
                visible: true,
                elements,
            }],
            ..Layer::default()
        };
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: 1,
                frame_count_duplicate: 1,
                request_id: 0,
                canvas: Canvas {
                    width: 24.0,
                    height: 24.0,
                    ..Canvas::default()
                },
                onion_before: 0,
                onion_after: 0,
                frame_durations: Vec::new(),
            },
            name: Some("Page 0".to_owned()),
            is_master: false,
            is_linked: false,
            page_id: None,
            last_export_dir: None,
            last_export_file: None,
            layers: LayerList {
                layers: vec![layer_at(vec![green]), layer_at(vec![blue])],
            },
            grid: None,
            guides: Default::default(),
            export: None,
        });
        document.object_selection = vec![green, blue];
        let before = document_bytes(&document);
        let delta = apply_by_name(&mut document, "flatten.selection", json!({})).unwrap();

        // One fresh image where the topmost member sat (layer 1).
        let image_id = match document.pages[0].layers.layers[1].cells[0]
            .elements
            .first()
            .copied()
        {
            Some(id) => id,
            None => panic!("no baked image"),
        };
        match document.registry.get(image_id).unwrap() {
            ObjectData::Image(image) => {
                assert_eq!(image.pixels, (24, 24));
                assert_eq!(image.opacity, 1000);
                assert_eq!(image.tiles.len(), 1);
                assert!(matches!(image.tiles[0], TileRef::Tile(id) if id != u32::MAX));
            }
            other => panic!("wrong object kind: {other:?}"),
        }
        assert!(document.registry.get(green).is_none());
        assert!(document.registry.get(blue).is_none());
        assert!(
            document.pages[0].layers.layers[0].cells[0]
                .elements
                .is_empty()
        );
        assert_eq!(image_pixel(&document, image_id, 2, 2), [0, 255, 0, 255]);
        assert_eq!(image_pixel(&document, image_id, 6, 6), [0, 0, 255, 255]);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn flatten_selection_noop_and_refusals() {
        let (mut document, _) = bake_document(vec![square(0.0, 0.0, 4.0, 4.0, GREEN)], 1);
        assert!(matches!(
            apply_by_name(&mut document, "flatten.selection", json!({})).unwrap(),
            Delta::Noop
        ));

        // A selection spanning two pages refuses.
        let (mut document, ids) = bake_document(vec![square(0.0, 0.0, 4.0, 4.0, GREEN)], 1);
        let elsewhere = document.registry.insert(square(0.0, 0.0, 4.0, 4.0, BLUE));
        let mut second = document.pages[0].clone();
        second.layers.layers[0].cells[0].elements = vec![elsewhere];
        document.pages.push(second);
        document.object_selection = vec![ids[0], elsewhere];
        assert!(matches!(
            apply_by_name(&mut document, "flatten.selection", json!({})),
            Err(EngineError::InvalidState { .. })
        ));

        // No canvas, no bake.
        let (mut document, ids) = bake_document(vec![square(0.0, 0.0, 4.0, 4.0, GREEN)], 1);
        document.pages[0].info.canvas.width = 0.0;
        document.object_selection = vec![ids[0]];
        assert!(matches!(
            apply_by_name(&mut document, "flatten.selection", json!({})),
            Err(EngineError::InvalidState { .. })
        ));
    }

    #[test]
    fn flatten_document_collapses_content_layers() {
        let mut document = Document::default();
        let green = document.registry.insert(square(0.0, 0.0, 8.0, 8.0, GREEN));
        let blue = document.registry.insert(square(4.0, 4.0, 12.0, 12.0, BLUE));
        let layer_at = |name: &str, elements: Vec<ObjectId>| Layer {
            name: name.to_owned(),
            display: true,
            cells: vec![Cell {
                locked: false,
                visible: true,
                elements,
            }],
            ..Layer::default()
        };
        let mut web_layer = layer_at("Web Layer", Vec::new());
        web_layer.is_special = true;
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: 1,
                frame_count_duplicate: 1,
                request_id: 0,
                canvas: Canvas {
                    width: 24.0,
                    height: 24.0,
                    ..Canvas::default()
                },
                onion_before: 0,
                onion_after: 0,
                frame_durations: Vec::new(),
            },
            name: Some("Page 0".to_owned()),
            is_master: false,
            is_linked: false,
            page_id: None,
            last_export_dir: None,
            last_export_file: None,
            layers: LayerList {
                layers: vec![
                    layer_at("Bottom", vec![green]),
                    layer_at("Top", vec![blue]),
                    web_layer,
                ],
            },
            grid: None,
            guides: Default::default(),
            export: None,
        });
        let before = document_bytes(&document);
        let delta = apply_by_name(&mut document, "flatten.document", json!({ "page": 0 })).unwrap();

        // One flat content layer below the surviving special layer.
        let layers = &document.pages[0].layers.layers;
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].name, "Bottom");
        assert_eq!(layers[0].cells.len(), 1);
        assert!(!layers[0].is_special);
        assert!(layers[1].is_special);
        assert_eq!(layers[1].name, "Web Layer");
        let image_id = layers[0].cells[0].elements[0];
        assert!(document.registry.get(green).is_none());
        assert!(document.registry.get(blue).is_none());
        assert_eq!(image_pixel(&document, image_id, 2, 2), [0, 255, 0, 255]);
        assert_eq!(image_pixel(&document, image_id, 6, 6), [0, 0, 255, 255]);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn flatten_document_refusals() {
        // Multi-state pages refuse (recorded deviation).
        let (mut document, _) = bake_document(vec![square(0.0, 0.0, 4.0, 4.0, GREEN)], 2);
        assert!(matches!(
            apply_by_name(&mut document, "flatten.document", json!({ "page": 0 })),
            Err(EngineError::InvalidState { .. })
        ));

        // An empty page is a no-op.
        let (mut document, _) = bake_document(Vec::new(), 1);
        assert!(matches!(
            apply_by_name(&mut document, "flatten.document", json!({ "page": 0 })).unwrap(),
            Delta::Noop
        ));

        // An unknown page is a missing target.
        let (mut document, _) = bake_document(vec![square(0.0, 0.0, 4.0, 4.0, GREEN)], 1);
        assert!(matches!(
            apply_by_name(&mut document, "flatten.document", json!({ "page": 3 })),
            Err(EngineError::MissingTarget { .. })
        ));
    }
}
