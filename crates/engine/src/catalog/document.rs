//! Document commands: canvas color, resolution, page rotation.
//!
//! CS6's `dom.setDocumentCanvas*`/`setDocumentResolution` verbs are
//! document-scoped: one setting in the UI, carried on the wire by both
//! the `MKBv` root canvas and every page's `PDCv` canvas copy. The
//! commands write both so the round-trip stays consistent (decision
//! 0023). `dom.rotateDocument` is page-scoped instead — canvas
//! dimensions live per page — so it re-bakes one page's geometry
//! (decision 0034).

use doc::{Document, TileRef};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the document commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(SetDocumentCanvasColor)?;
    registry.register(SetDocumentResolution)?;
    registry.register(DocumentRotate)
}

/// `document.setCanvasColor` — CS6's
/// `dom.setDocumentCanvasColor(color)`: sets the canvas matte to one
/// color across the document (`BGCi` on the `MKBv` root canvas and
/// every page's copy). The raw is ABGR like every paint slot
/// (`compose` paints `R = raw & 0xFF`); the scripting layer's color
/// string maps to this raw before the command runs.
struct SetDocumentCanvasColor;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetDocumentCanvasColorPayload {
    /// Canvas color, ABGR layout (0x00RRGGBB when fully opaque).
    color_raw: i64,
}

impl TypedCommand for SetDocumentCanvasColor {
    type Payload = SetDocumentCanvasColorPayload;

    fn name(&self) -> &'static str {
        "document.setCanvasColor"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "document.setCanvasColor";
        if !(0..=0xFF_FF_FF_FF).contains(&payload.color_raw) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("color_raw {} outside 0..=0xFFFFFFFF", payload.color_raw),
            });
        }
        // Capture every `before` before the first mutation (undo law).
        let mut deltas = vec![Delta::Info {
            before: Box::new(document.info.clone()),
        }];
        let mut pages = Vec::with_capacity(document.pages.len());
        for page in &document.pages {
            pages.push(page.info.clone());
        }
        document.info.canvas.background_raw = payload.color_raw;
        for (index, page) in document.pages.iter_mut().enumerate() {
            page.info.canvas.background_raw = payload.color_raw;
            deltas.push(Delta::PageInfo {
                page: index,
                before: Box::new(pages[index].clone()),
            });
        }
        Ok(Delta::Group(deltas))
    }
}

/// Resolution unit (CS6's `units` string; `RSUi` on the wire).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ResolutionUnit {
    /// `RSUi` 0 — pixels per inch (the default).
    Inch,
    /// `RSUi` 1 — pixels per centimeter.
    Cm,
}

impl ResolutionUnit {
    fn raw(self) -> i64 {
        match self {
            ResolutionUnit::Inch => 0,
            ResolutionUnit::Cm => 1,
        }
    }
}

/// `document.setResolution` — CS6's
/// `dom.setDocumentResolution({pixelsPerUnit, units})`: sets the
/// resolution and its unit across the document (`RESf`/`RSUi` on the
/// `MKBv` root canvas and every page's copy).
struct SetDocumentResolution;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetDocumentResolutionPayload {
    /// Pixels per unit; the UI's document resolution.
    resolution: f64,
    /// Unit the resolution is expressed in.
    unit: ResolutionUnit,
}

impl TypedCommand for SetDocumentResolution {
    type Payload = SetDocumentResolutionPayload;

    fn name(&self) -> &'static str {
        "document.setResolution"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "document.setResolution";
        if !payload.resolution.is_finite() || payload.resolution <= 0.0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "resolution {} must be finite and positive",
                    payload.resolution
                ),
            });
        }
        // Capture every `before` before the first mutation (undo law).
        let mut deltas = vec![Delta::Info {
            before: Box::new(document.info.clone()),
        }];
        let mut pages = Vec::with_capacity(document.pages.len());
        for page in &document.pages {
            pages.push(page.info.clone());
        }
        document.info.canvas.resolution = payload.resolution;
        document.info.canvas.resolution_unit = payload.unit.raw();
        for (index, page) in document.pages.iter_mut().enumerate() {
            page.info.canvas.resolution = payload.resolution;
            page.info.canvas.resolution_unit = payload.unit.raw();
            deltas.push(Delta::PageInfo {
                page: index,
                before: Box::new(pages[index].clone()),
            });
        }
        Ok(Delta::Group(deltas))
    }
}

/// `document.rotate` — CS6's `dom.rotateDocument(rotationAmount)`:
/// rotates one page 90/180/270 degrees clockwise with a full geometry
/// re-bake (decision 0034). The page is an explicit payload field —
/// the engine keeps no current-page state (`page.delete` precedent) —
/// while CS6 targets the active page.
///
/// Every object on the page re-bakes into the rotated space: paths and
/// slices map point-for-point, text/symbol transforms compose the
/// rotation, images assemble their tile grid, rotate the buffer, and
/// re-tile over fresh bank slots (matte slots materialize to the
/// renderer's premultiplied fill — the flatten bake debt).
struct DocumentRotate;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DocumentRotatePayload {
    /// Clockwise rotation in degrees: 90, 180, or 270.
    rotation: i64,
    /// Page index.
    page: usize,
}

/// The linear part of a clockwise (y-down) rotation step, built from
/// integers: trig-approximated 90-degree rotations smear `cos(pi/2)`
/// into every coordinate, and a quarter turn must be exact.
fn rotation_linear(rotation: i64) -> geom::Affine {
    match rotation {
        90 => geom::Affine::new([0.0, 1.0, -1.0, 0.0, 0.0, 0.0]),
        180 => geom::Affine::new([-1.0, 0.0, 0.0, -1.0, 0.0, 0.0]),
        _ => geom::Affine::new([0.0, -1.0, 1.0, 0.0, 0.0, 0.0]),
    }
}

/// The full old-canvas → new-canvas map: rotate about the old center,
/// then translate the rotated extent onto the new rect (dimensions
/// swapped for 90/270, origin anchored), so every object survives the
/// step and nothing is clipped or lost.
fn rotation_map(rotation: i64, old: geom::Rect, new: geom::Rect) -> geom::Affine {
    let linear = rotation_linear(rotation);
    let moved_center = linear * old.center();
    let slide = geom::Vec2::new(
        new.center().x - moved_center.x,
        new.center().y - moved_center.y,
    );
    geom::Affine::translate(slide) * linear
}

/// Every object on the page in paint order: cells bottom-first, group
/// children expanded depth-first after their parent (groups carry no
/// geometry — children hold world-space coordinates and rotate
/// individually). Each id appears exactly once.
fn page_object_ids(document: &Document, page: usize) -> Vec<doc::ObjectId> {
    let mut ids: Vec<doc::ObjectId> = document.pages[page]
        .layers
        .layers
        .iter()
        .flat_map(|layer| layer.cells.iter())
        .flat_map(|cell| cell.elements.iter().copied())
        .collect();
    let mut index = 0;
    while index < ids.len() {
        if let Some(doc::ObjectData::Group(group)) = document.registry.get(ids[index]) {
            ids.extend(group.children.iter().copied());
        }
        index += 1;
    }
    ids
}

/// Maps one curve point (anchor plus optional handles) through `map`.
fn rotate_point(point: &mut doc::PathPoint, map: geom::Affine) {
    point.anchor = map * point.anchor;
    if let Some(primary) = &mut point.primary {
        *primary = map * *primary;
    }
    if let Some(secondary) = &mut point.secondary {
        *secondary = map * *secondary;
    }
}

fn rotate_contours(contours: &mut [doc::Contour], map: geom::Affine) {
    for contour in contours {
        for point in &mut contour.points {
            rotate_point(point, map);
        }
    }
}

/// Rotates a row-major RGBA buffer 90/180/270 degrees clockwise by
/// index permutation (no resampling: 90-degree steps are exact).
fn rotate_pixels(flat: &[u8], width: i64, height: i64, rotation: i64) -> Vec<u8> {
    let new_width = if rotation == 180 { width } else { height };
    let mut out = vec![0u8; flat.len()];
    for y in 0..height {
        for x in 0..width {
            let (new_x, new_y) = match rotation {
                90 => (height - 1 - y, x),
                180 => (width - 1 - x, height - 1 - y),
                _ => (y, width - 1 - x),
            };
            let src = (y as usize * width as usize + x as usize) * 4;
            let dst = (new_y as usize * new_width as usize + new_x as usize) * 4;
            out[dst..dst + 4].copy_from_slice(&flat[src..src + 4]);
        }
    }
    out
}

/// Assembles the image's tile grid into one RGBA buffer, rotates the
/// buffer, and re-tiles it over fresh bank slots. Matte slots
/// materialize to the renderer's premultiplied fill — the exact bytes
/// `compose` blits — so a rotated image is the bake debt (same class
/// as flatten). Returns the rotated image and the minted tile ids.
/// Callers preflight the tile bank, so the assembly cannot miss.
fn rotate_image(
    document: &mut Document,
    image: &doc::ImageObject,
    rotation: i64,
    map: geom::Affine,
) -> Result<(doc::ImageObject, Vec<u32>), EngineError> {
    let (old_width, old_height) = image.pixels;
    // 1. Assemble: every slot becomes premultiplied RGBA bytes.
    let flat = crate::bitmap::assemble_rgba(document, image, "document.rotate")?;
    // 2. Rotate the buffer.
    let (new_width, new_height) = if rotation == 180 {
        (old_width, old_height)
    } else {
        (old_height, old_width)
    };
    let rotated = rotate_pixels(&flat, old_width, old_height, rotation);
    // 3. Re-tile over fresh slots (the flatten minting law).
    let (tiles, minted) = crate::bitmap::retile_rgba(document, &rotated, new_width, new_height);
    Ok((
        doc::ImageObject {
            location: map * image.location,
            offset: map * image.offset,
            pixels: (new_width, new_height),
            tile_size: image.tile_size,
            tiles,
            name: image.name.clone(),
            opacity: image.opacity,
            blend: image.blend,
            visible: image.visible,
            locked: image.locked,
            effects: image.effects.clone(),
        },
        minted,
    ))
}

impl TypedCommand for DocumentRotate {
    type Payload = DocumentRotatePayload;

    fn name(&self) -> &'static str {
        "document.rotate"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "document.rotate";
        if !matches!(payload.rotation, 90 | 180 | 270) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("rotation {} is not one of 90, 180, 270", payload.rotation),
            });
        }
        access::page(document, NAME, payload.page)?;
        // Content refusals run before any capture, so a refused
        // rotation leaves the document byte-identical.
        let ids = page_object_ids(document, payload.page);
        for id in &ids {
            if let doc::ObjectData::Image(image) = access::object(document, NAME, *id)? {
                if image.tile_size != i64::from(raster::TILE_SIDE as u32) {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: format!(
                            "image {} uses a {} px tile grid; only the {} px mkBT grid rotates",
                            id.index(),
                            image.tile_size,
                            raster::TILE_SIDE
                        ),
                    });
                }
                for tile_ref in &image.tiles {
                    if let TileRef::Tile(tile_id) = tile_ref {
                        if document.tiles.get(*tile_id).is_none() {
                            return Err(EngineError::InvalidState {
                                name: NAME.to_owned(),
                                detail: format!(
                                    "image {} references missing tile {}",
                                    id.index(),
                                    tile_id
                                ),
                            });
                        }
                    }
                }
            }
        }
        // The canvas rect drives the whole map; a degenerate canvas has
        // nothing to rotate (CS6 always has a real one).
        let old_canvas = document.pages[payload.page].info.canvas.clone();
        if !old_canvas.width.is_finite()
            || !old_canvas.height.is_finite()
            || old_canvas.width <= 0.0
            || old_canvas.height <= 0.0
        {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "canvas {}x{} cannot rotate",
                    old_canvas.width, old_canvas.height
                ),
            });
        }
        // Capture every `before` before the first mutation (undo law):
        // the whole page (canvas, guides, grid, layers) plus each
        // object.
        let before_page = Box::new(document.pages[payload.page].clone());
        let mut befores = Vec::with_capacity(ids.len());
        for id in &ids {
            befores.push((*id, access::object(document, NAME, *id)?.clone()));
        }
        let old_rect = geom::Rect::new(
            old_canvas.x,
            old_canvas.y,
            old_canvas.x + old_canvas.width,
            old_canvas.y + old_canvas.height,
        );
        let (new_width, new_height) = if payload.rotation == 180 {
            (old_canvas.width, old_canvas.height)
        } else {
            (old_canvas.height, old_canvas.width)
        };
        let new_rect = geom::Rect::new(
            old_canvas.x,
            old_canvas.y,
            old_canvas.x + new_width,
            old_canvas.y + new_height,
        );
        let map = rotation_map(payload.rotation, old_rect, new_rect);
        // Image re-bakes mint bank slots; computing them first keeps
        // the registry pass borrow-free. Nothing below can fail: the
        // preflight settled every tile reference.
        let mut bakes = Vec::new();
        let mut minted = Vec::new();
        for (id, before) in &befores {
            if let doc::ObjectData::Image(image) = before {
                let (rotated, added) = rotate_image(document, image, payload.rotation, map)?;
                bakes.push((*id, rotated));
                minted.extend(added);
            }
        }
        let mut deltas = vec![
            Delta::PageBody {
                page: payload.page,
                before: before_page,
            },
            Delta::Bank { added: minted },
        ];
        for (id, rotated) in bakes {
            let Some(doc::ObjectData::Image(slot)) = document.registry.get_mut(id) else {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!("image {} vanished mid-command", id.index()),
                });
            };
            *slot = rotated;
        }
        for (id, before) in befores {
            let Some(slot) = document.registry.get_mut(id) else {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!("object {} vanished mid-command", id.index()),
                });
            };
            match slot {
                // Images baked above; their delta rides with the rest.
                doc::ObjectData::Image(_) => {}
                doc::ObjectData::Path(object) => {
                    rotate_contours(&mut object.contours, map);
                    if let Some(primitive) = &mut object.primitive {
                        // 90-degree steps keep rects axis-aligned, so
                        // the mapped bounds are the rotated rect.
                        primitive.rect = geom::transform::map_rect(map, primitive.rect);
                        primitive.transform = map * primitive.transform;
                    }
                }
                doc::ObjectData::Text(object) => {
                    // Transform-only: the frame is the pre-rotation
                    // layout input, and `finish` rotates the laid-out
                    // output — re-wrapping would move the glyphs.
                    let base = object.transform.unwrap_or(geom::Affine::IDENTITY);
                    object.transform = Some(map * base);
                }
                doc::ObjectData::Slice(object) => {
                    object.frame = geom::transform::map_rect(map, object.frame);
                    rotate_contours(&mut object.contours, map);
                }
                doc::ObjectData::SymbolInstance(object) => {
                    object.transform = map * object.transform;
                }
                doc::ObjectData::Group(_) => {}
            }
            deltas.push(Delta::Object {
                id,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(before),
                },
            });
        }
        {
            let page = &mut document.pages[payload.page];
            page.info.canvas.width = new_width;
            page.info.canvas.height = new_height;
            // Guides: horizontal lines (y positions) and vertical lines
            // (x positions) swap under 90/270 and mirror under 180 —
            // read off the map at one point of each old line.
            let origin = geom::Point::new(old_canvas.x, old_canvas.y);
            let mut guides = std::mem::take(&mut page.guides);
            let old_horizontal = std::mem::take(&mut guides.horizontal);
            let old_vertical = std::mem::take(&mut guides.vertical);
            if payload.rotation == 180 {
                guides.horizontal = old_horizontal
                    .iter()
                    .map(|y| (map * geom::Point::new(origin.x, *y)).y)
                    .collect();
                guides.vertical = old_vertical
                    .iter()
                    .map(|x| (map * geom::Point::new(*x, origin.y)).x)
                    .collect();
            } else {
                guides.horizontal = old_vertical
                    .iter()
                    .map(|x| (map * geom::Point::new(*x, origin.y)).y)
                    .collect();
                guides.vertical = old_horizontal
                    .iter()
                    .map(|y| (map * geom::Point::new(origin.x, *y)).x)
                    .collect();
            }
            page.guides = guides;
            if let Some(grid) = &mut page.grid {
                grid.offset = map * grid.offset;
            }
        }
        Ok(Delta::Group(deltas))
    }
}

#[cfg(test)]
mod document_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::catalog::flatten::next_tile_id;
    use crate::catalog::testdoc;
    use crate::command::Command;

    #[test]
    fn canvas_color_is_document_wide_and_undoes() {
        let (mut document, _) = testdoc::sample_document();
        document.info.canvas.background_raw = 0x00_FF_FF;
        document.pages[0].info.canvas.background_raw = 0x11_22_33;
        document.pages[1].info.canvas.background_raw = 0x44_55_66;

        let delta = SetDocumentCanvasColor
            .apply_json(&mut document, serde_json::json!({"color_raw": 0x00_00_FF}))
            .expect("setCanvasColor applies");
        assert_eq!(document.info.canvas.background_raw, 0x00_00_FF);
        for page in &document.pages {
            assert_eq!(page.info.canvas.background_raw, 0x00_00_FF);
        }

        delta.apply_inverse(&mut document).expect("inverse applies");
        assert_eq!(document.info.canvas.background_raw, 0x00_FF_FF);
        assert_eq!(document.pages[0].info.canvas.background_raw, 0x11_22_33);
        assert_eq!(document.pages[1].info.canvas.background_raw, 0x44_55_66);
    }

    #[test]
    fn canvas_color_rejects_out_of_range_raw() {
        let (mut document, _) = testdoc::sample_document();
        let err = SetDocumentCanvasColor
            .apply_json(&mut document, serde_json::json!({"color_raw": -1}))
            .expect_err("negative raw is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::InvalidState { .. }
        ));
    }

    #[test]
    fn resolution_sets_unit_and_undoes() {
        let (mut document, _) = testdoc::sample_document();
        assert_eq!(document.info.canvas.resolution, 72.0);
        assert_eq!(document.info.canvas.resolution_unit, 0);

        SetDocumentResolution
            .apply_json(
                &mut document,
                serde_json::json!({"resolution": 96.0, "unit": "inch"}),
            )
            .expect("setResolution applies");
        assert_eq!(document.info.canvas.resolution, 96.0);
        assert_eq!(document.info.canvas.resolution_unit, 0);
        assert_eq!(document.pages[1].info.canvas.resolution, 96.0);

        let delta = SetDocumentResolution
            .apply_json(
                &mut document,
                serde_json::json!({"resolution": 28.35, "unit": "cm"}),
            )
            .expect("cm applies");
        assert_eq!(document.info.canvas.resolution_unit, 1);

        delta.apply_inverse(&mut document).expect("inverse applies");
        assert_eq!(document.info.canvas.resolution, 96.0);
        assert_eq!(document.info.canvas.resolution_unit, 0);
    }

    #[test]
    fn resolution_rejects_non_positive() {
        let (mut document, _) = testdoc::sample_document();
        for bad in [0.0, -72.0] {
            let err = SetDocumentResolution
                .apply_json(
                    &mut document,
                    serde_json::json!({"resolution": bad, "unit": "inch"}),
                )
                .expect_err("non-positive resolution is refused");
            assert!(matches!(
                err,
                crate::error::EngineError::InvalidState { .. }
            ));
        }
        // NaN/inf never reach the command: they serialize to JSON null
        // and the payload layer refuses first (same outcome).
        let err = SetDocumentResolution
            .apply_json(
                &mut document,
                serde_json::json!({"resolution": f64::NAN, "unit": "inch"}),
            )
            .expect_err("null resolution is refused");
        assert!(matches!(err, crate::error::EngineError::BadPayload { .. }));
    }

    #[test]
    fn page_info_is_captured_per_page_not_shared() {
        // Each page's inverse must restore its own pre-command info:
        // a shared `before` would stamp page 0's state onto page 1.
        let (mut document, _) = testdoc::sample_document();
        document.pages[0].info.onion_before = 3;
        document.pages[1].info.onion_after = 4;
        let delta = SetDocumentCanvasColor
            .apply_json(&mut document, serde_json::json!({"color_raw": 1}))
            .expect("applies");
        delta.apply_inverse(&mut document).expect("inverse applies");
        assert_eq!(document.pages[0].info.onion_before, 3);
        assert_eq!(document.pages[1].info.onion_after, 4);
    }

    fn document_bytes(document: &Document) -> Vec<u8> {
        serde_json::to_vec(document).unwrap()
    }

    fn canvas(width: f64, height: f64) -> doc::Canvas {
        doc::Canvas {
            width,
            height,
            ..doc::Canvas::default()
        }
    }

    fn anchors(document: &Document, id: doc::ObjectId) -> Vec<(f64, f64)> {
        let doc::ObjectData::Path(path) = document.registry.get(id).unwrap() else {
            panic!("expected a path object")
        };
        path.contours[0]
            .points
            .iter()
            .map(|point| (point.anchor.x, point.anchor.y))
            .collect()
    }

    #[test]
    fn rotate_90_rebakes_geometry_and_undoes() {
        let (mut document, ids) = testdoc::sample_document();
        document.pages[0].info.canvas = canvas(64.0, 48.0);
        let before = document_bytes(&document);

        let delta = DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 90, "page": 0}),
            )
            .expect("rotate 90 applies");
        assert_eq!(document.pages[0].info.canvas.width, 48.0);
        assert_eq!(document.pages[0].info.canvas.height, 64.0);
        // The sample square (0,0)-(9,8) maps by (x, y) -> (48 - y, x).
        assert_eq!(
            anchors(&document, ids[0]),
            vec![(48.0, 0.0), (48.0, 9.0), (40.0, 9.0), (40.0, 0.0)]
        );
        // The other page never moves.
        assert_eq!(document.pages[1].info.canvas.width, 0.0);
        assert_eq!(document.pages[1].info.canvas.height, 0.0);

        delta.apply_inverse(&mut document).expect("inverse applies");
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn rotate_180_mirrors_and_keeps_dimensions() {
        let (mut document, ids) = testdoc::sample_document();
        document.pages[0].info.canvas = canvas(64.0, 48.0);
        document.pages[0].guides.horizontal = vec![10.0];
        document.pages[0].guides.vertical = vec![5.0];

        DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 180, "page": 0}),
            )
            .expect("rotate 180 applies");
        assert_eq!(document.pages[0].info.canvas.width, 64.0);
        assert_eq!(document.pages[0].info.canvas.height, 48.0);
        // (x, y) -> (64 - x, 48 - y).
        assert_eq!(
            anchors(&document, ids[0]),
            vec![(64.0, 48.0), (55.0, 48.0), (55.0, 40.0), (64.0, 40.0)]
        );
        // Guides mirror in place (no swap: the axes don't change).
        assert_eq!(document.pages[0].guides.horizontal, vec![38.0]);
        assert_eq!(document.pages[0].guides.vertical, vec![59.0]);
    }

    #[test]
    fn rotate_90_swaps_guides_and_rotates_the_grid() {
        let (mut document, _) = testdoc::sample_document();
        document.pages[0].info.canvas = canvas(64.0, 48.0);
        document.pages[0].guides.horizontal = vec![10.0, 20.0];
        document.pages[0].guides.vertical = vec![5.0];
        document.pages[0].grid = Some(doc::Grid {
            offset: geom::Point::new(3.0, 4.0),
            ..doc::Grid::default()
        });

        DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 90, "page": 0}),
            )
            .expect("applies");
        // (x, y) -> (48 - y, x): vertical 5 becomes horizontal 5;
        // horizontals 10/20 become verticals 38/28; the grid origin
        // rides the map, its cell size is rotation-free.
        assert_eq!(document.pages[0].guides.horizontal, vec![5.0]);
        assert_eq!(document.pages[0].guides.vertical, vec![38.0, 28.0]);
        let grid = document.pages[0].grid.as_ref().unwrap();
        assert_eq!((grid.offset.x, grid.offset.y), (44.0, 3.0));
        assert_eq!(grid.size, doc::Grid::default().size);
    }

    #[test]
    fn rotate_90_composes_text_and_symbol_transforms() {
        let (mut document, _) = testdoc::sample_document();
        document.pages[0].info.canvas = canvas(64.0, 48.0);
        let text_id = document
            .registry
            .insert(doc::ObjectData::Text(doc::TextObject::new(
                geom::Rect::new(0.0, 0.0, 20.0, 10.0),
            )));
        let instance_id =
            document
                .registry
                .insert(doc::ObjectData::SymbolInstance(doc::SymbolInstanceObject {
                    symbol_number: 1,
                    transform: geom::Affine::translate(geom::Vec2::new(4.0, 6.0)),
                    locked: false,
                    name: None,
                    animation: None,
                    custom_data: std::collections::BTreeMap::new(),
                }));
        {
            let cell = &mut document.pages[0].layers.layers[0].cells[0];
            cell.elements.push(text_id);
            cell.elements.push(instance_id);
        }

        DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 90, "page": 0}),
            )
            .expect("applies");
        let doc::ObjectData::Text(text) = document.registry.get(text_id).unwrap() else {
            panic!("text stays text")
        };
        // Transform-only: the layout frame is untouched, the rotation
        // lands in MTXv (identity -> the canvas map itself).
        assert_eq!(text.frame, geom::Rect::new(0.0, 0.0, 20.0, 10.0));
        let transform = text.transform.expect("rotation materializes MTXv");
        assert_eq!(transform.as_coeffs(), [0.0, 1.0, -1.0, 0.0, 48.0, 0.0]);
        let doc::ObjectData::SymbolInstance(instance) = document.registry.get(instance_id).unwrap()
        else {
            panic!("instance stays an instance")
        };
        // translate(4, 6) then the map: (x + 4, y + 6) -> (42 - y, x + 4).
        assert_eq!(
            instance.transform.as_coeffs(),
            [0.0, 1.0, -1.0, 0.0, 42.0, 4.0]
        );
    }

    #[test]
    fn rotate_90_retiles_image_pixels_and_undoes() {
        let (mut document, _) = testdoc::sample_document();
        document.pages[0].info.canvas = canvas(64.0, 48.0);
        let mut tile = [0u8; raster::TILE_BYTES];
        let put = |tile: &mut [u8; raster::TILE_BYTES], x: usize, y: usize, pixel: [u8; 4]| {
            let at = (y * raster::TILE_SIDE + x) * 4;
            tile[at..at + 4].copy_from_slice(&pixel);
        };
        put(&mut tile, 0, 0, [255, 0, 0, 255]);
        put(&mut tile, 15, 23, [0, 0, 255, 255]);
        put(&mut tile, 7, 11, [0, 255, 0, 255]);
        let tile_id = next_tile_id(&document);
        document
            .tiles
            .insert(tile_id, raster::Tile::from_rgba8(tile));
        let image_id = document
            .registry
            .insert(doc::ObjectData::Image(doc::ImageObject {
                location: geom::Point::new(10.0, 20.0),
                offset: geom::Point::ZERO,
                pixels: (16, 24),
                tile_size: i64::from(raster::TILE_SIDE as u32),
                tiles: vec![doc::TileRef::Tile(tile_id)],
                name: Some("photo".into()),
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                effects: Vec::new(),
            }));
        document.pages[0].layers.layers[0].cells[0]
            .elements
            .push(image_id);
        let before = document_bytes(&document);

        let delta = DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 90, "page": 0}),
            )
            .expect("applies");
        let doc::ObjectData::Image(image) = document.registry.get(image_id).unwrap() else {
            panic!("image stays an image")
        };
        assert_eq!(image.pixels, (24, 16)); // extent swapped
        assert_eq!(image.location, geom::Point::new(28.0, 10.0));
        assert_eq!(image.tiles.len(), 1);
        let doc::TileRef::Tile(new_tile) = image.tiles[0] else {
            panic!("a real tile rides the bake")
        };
        assert_ne!(new_tile, tile_id);
        let pixels = document.tiles.get(new_tile).unwrap().pixels();
        let at = |x: usize, y: usize| &pixels[(y * raster::TILE_SIDE + x) * 4..][..4];
        // (x, y) -> (23 - y, x): corners and center ride the
        // permutation.
        assert_eq!(at(23, 0), &[255, 0, 0, 255]);
        assert_eq!(at(0, 15), &[0, 0, 255, 255]);
        assert_eq!(at(12, 7), &[0, 255, 0, 255]);

        delta.apply_inverse(&mut document).expect("inverse applies");
        assert_eq!(document_bytes(&document), before);
        // The rotated tile left the bank with the undo.
        assert!(document.tiles.get(new_tile).is_none());
    }

    #[test]
    fn rotate_90_materializes_matte_slots_premultiplied() {
        let (mut document, _) = testdoc::sample_document();
        document.pages[0].info.canvas = canvas(64.0, 48.0);
        let image_id = document
            .registry
            .insert(doc::ObjectData::Image(doc::ImageObject {
                location: geom::Point::ZERO,
                offset: geom::Point::ZERO,
                pixels: (8, 8),
                tile_size: i64::from(raster::TILE_SIDE as u32),
                tiles: vec![doc::TileRef::Matte(0x80_FF_00_00)], // a=0x80, r=0xFF
                name: None,
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                effects: Vec::new(),
            }));
        document.pages[0].layers.layers[0].cells[0]
            .elements
            .push(image_id);

        DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 90, "page": 0}),
            )
            .expect("applies");
        let doc::ObjectData::Image(image) = document.registry.get(image_id).unwrap() else {
            panic!("image stays an image")
        };
        let doc::TileRef::Tile(new_tile) = image.tiles[0] else {
            panic!("matte baked into a real tile")
        };
        let pixels = document.tiles.get(new_tile).unwrap().pixels();
        // Image pixel (0, 0) lands at (7, 0) under the 90-degree map,
        // premultiplied: 255 * 128 / 255 = 128 red at alpha 128.
        assert_eq!(&pixels[28..32], &[128, 0, 0, 128]);
    }

    #[test]
    fn rotate_90_rotates_group_children() {
        let (mut document, ids) = testdoc::sample_document();
        document.pages[0].info.canvas = canvas(64.0, 48.0);
        let group_id = document
            .registry
            .insert(doc::ObjectData::Group(doc::GroupObject {
                group_type: 0,
                name: None,
                children: vec![ids[0]],
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                mask_hidden: false,
                mask_linked: None,
                effects: Vec::new(),
            }));
        // Pull the child out of the cell so only the group sits there
        // — the walk must reach through the group to the child.
        document.pages[0].layers.layers[0].cells[0].elements = vec![ids[1], group_id];

        DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 90, "page": 0}),
            )
            .expect("applies");
        assert_eq!(
            anchors(&document, ids[0]),
            vec![(48.0, 0.0), (48.0, 9.0), (40.0, 9.0), (40.0, 0.0)]
        );
    }

    #[test]
    fn rotate_refusals_leave_the_document_untouched() {
        let (mut document, _) = testdoc::sample_document();
        document.pages[0].info.canvas = canvas(64.0, 48.0);
        let image_id = document
            .registry
            .insert(doc::ObjectData::Image(doc::ImageObject {
                location: geom::Point::ZERO,
                offset: geom::Point::ZERO,
                pixels: (16, 16),
                tile_size: 64,
                tiles: vec![doc::TileRef::Tile(999)],
                name: None,
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                effects: Vec::new(),
            }));
        document.pages[0].layers.layers[0].cells[0]
            .elements
            .push(image_id);
        let before = document_bytes(&document);

        for bad in [45, 0, -90] {
            let err = DocumentRotate
                .apply_json(
                    &mut document,
                    serde_json::json!({"rotation": bad, "page": 0}),
                )
                .expect_err("non-step rotation is refused");
            assert!(matches!(
                err,
                crate::error::EngineError::InvalidState { .. }
            ));
        }
        let err = DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 90, "page": 7}),
            )
            .expect_err("missing page is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::MissingTarget { .. }
        ));
        let err = DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 90, "page": 1}),
            )
            .expect_err("a degenerate (0x0) canvas is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::InvalidState { .. }
        ));
        let err = DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 90, "page": 0}),
            )
            .expect_err("an odd tile grid is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::InvalidState { .. }
        ));
        assert_eq!(document_bytes(&document), before);

        // With the grid fixed the dangling tile still refuses.
        if let Some(doc::ObjectData::Image(image)) = document.registry.get_mut(image_id) {
            image.tile_size = i64::from(raster::TILE_SIDE as u32);
        }
        let after_grid_fix = document_bytes(&document);
        let err = DocumentRotate
            .apply_json(
                &mut document,
                serde_json::json!({"rotation": 90, "page": 0}),
            )
            .expect_err("a dangling tile is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::InvalidState { .. }
        ));
        assert_eq!(document_bytes(&document), after_grid_fix);
    }
}
