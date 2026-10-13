//! Image-object rendering (M8 task 7): tile-bank pixels land at their
//! declared extent, clip to the canvas, honor matte substitution, and
//! route through the same opacity/blend/effects machinery as paths.
//! Every expected value is hand-derived premultiplied RGBA8.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compose::{ComposeError, render};
use doc::{Document, Layer, LayerList, ObjectData, Page, PageInfo, Registry, TileRef};
use geom::Point;
use raster::{TILE_BYTES, TILE_SIDE, Tile};

// --- document builders ---------------------------------------------------

fn page(width: f64, height: f64, layers: Vec<Layer>) -> Page {
    Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: 2,
            frame_count_duplicate: 2,
            request_id: 0,
            canvas: doc::Canvas {
                x: 0.0,
                y: 0.0,
                width,
                height,
                resolution: 72.0,
                resolution_unit: 0,
                background_raw: 0xFF_FF_FF,
            },
            onion_before: 0,
            onion_after: 0,
            frame_durations: Vec::new(),
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList { layers },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    }
}

fn layer(cells: Vec<doc::Cell>) -> Layer {
    Layer {
        name: String::new(),
        is_background: false,
        has_url: false,
        is_special: false,
        frame_changing: false,
        display: true,
        nesting_level: None,
        cells,
        visibility_per_frame: None,
    }
}

fn cell(ids: Vec<doc::ObjectId>) -> doc::Cell {
    doc::Cell {
        locked: false,
        visible: true,
        elements: ids,
    }
}

fn image_object(
    registry: &mut Registry,
    location: (f64, f64),
    extent: (i64, i64),
    tile_size: i64,
    tiles: Vec<TileRef>,
    opacity: i64,
) -> doc::ObjectId {
    registry.insert(ObjectData::Image(doc::ImageObject {
        location: Point::new(location.0, location.1),
        offset: Point::new(0.0, 0.0),
        pixels: extent,
        tile_size,
        tiles,
        name: None,
        opacity,
        blend: 0,
        visible: true,
        locked: false,
        effects: Vec::new(),
    }))
}

/// A tile with a `block`-sized opaque patch of `color` at its origin,
/// the rest transparent; `None` paints the whole tile.
fn tile(color: [u8; 4], block: Option<(usize, usize)>) -> Tile {
    let mut pixels = [0u8; TILE_BYTES];
    match block {
        Some((w, h)) => {
            for y in 0..h {
                for x in 0..w {
                    let i = (y * TILE_SIDE + x) * 4;
                    pixels[i..i + 4].copy_from_slice(&color);
                }
            }
        }
        None => {
            for px in pixels.chunks_exact_mut(4) {
                px.copy_from_slice(&color);
            }
        }
    }
    Tile::from_rgba8(pixels)
}

/// Opaque red, straight or premultiplied (identical at α=255).
const RED: [u8; 4] = [255, 0, 0, 255];
/// Opaque blue.
const BLUE: [u8; 4] = [0, 0, 255, 255];

fn pixel(render: &compose::Render, x: u32, y: u32) -> [u8; 4] {
    let index = (y * u32::from(render.width) + x) as usize * 4;
    render.pixels[index..index + 4].try_into().expect("pixel")
}

/// Asserts every pixel of the render is fully transparent.
fn assert_all_transparent(render: &compose::Render) {
    assert!(render.pixels.iter().all(|&b| b == 0));
}

// --- proofs --------------------------------------------------------------

/// A 2×2 extent at location (4, 6) paints exactly pixels (4..6, 6..8)
/// and nothing else — location + extent, no drift.
#[test]
fn image_pixels_land_at_their_extent() {
    let mut doc = Document::default();
    doc.tiles.insert(0, tile(RED, Some((2, 2))));
    let image = image_object(
        &mut doc.registry,
        (4.0, 6.0),
        (2, 2),
        128,
        vec![TileRef::Tile(0)],
        1000,
    );
    doc.pages
        .push(page(8.0, 8.0, vec![layer(vec![cell(vec![image])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!((render.width, render.height), (8, 8));
    assert_eq!(pixel(&render, 4, 6), RED, "top-left of the extent");
    assert_eq!(pixel(&render, 5, 7), RED, "bottom-right of the extent");
    assert_eq!(pixel(&render, 6, 6), [0, 0, 0, 0], "right of the extent");
    assert_eq!(pixel(&render, 4, 5), [0, 0, 0, 0], "above the extent");
}

/// A 130-wide extent spans two bank tiles row-major: slot 0 covers
/// x∈[0,128), slot 1 x∈[128,130); the extent's last column clips the
/// second tile at x=129.
#[test]
fn tile_grid_is_row_major_across_tiles() {
    let mut doc = Document::default();
    doc.tiles.insert(0, tile(RED, None));
    doc.tiles.insert(1, tile(BLUE, None));
    let image = image_object(
        &mut doc.registry,
        (0.0, 0.0),
        (130, 2),
        128,
        vec![TileRef::Tile(0), TileRef::Tile(1)],
        1000,
    );
    doc.pages
        .push(page(132.0, 2.0, vec![layer(vec![cell(vec![image])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), RED, "slot 0, first column");
    assert_eq!(pixel(&render, 127, 1), RED, "slot 0, last full column");
    assert_eq!(pixel(&render, 128, 0), BLUE, "slot 1, first column");
    assert_eq!(pixel(&render, 129, 1), BLUE, "slot 1, extent-clipped end");
    assert_eq!(pixel(&render, 130, 0), [0, 0, 0, 0], "past the extent");
}

/// An image at a negative origin contributes only its on-canvas
/// window: extent pixel (2, 3) is canvas pixel (0, 0).
#[test]
fn canvas_clips_negative_origins() {
    let mut doc = Document::default();
    doc.tiles.insert(0, tile(RED, None));
    let image = image_object(
        &mut doc.registry,
        (-2.0, -3.0),
        (4, 4),
        128,
        vec![TileRef::Tile(0)],
        1000,
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![image])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), RED, "extent (2,3)");
    assert_eq!(pixel(&render, 1, 0), RED, "extent (3,3)");
    assert_eq!(pixel(&render, 2, 0), [0, 0, 0, 0], "extent (4,3): past");
    assert_eq!(pixel(&render, 0, 1), [0, 0, 0, 0], "extent (2,4): past");
}

/// `TMCi` matte tiles substitute a flat color, decoded ARGB per the
/// dictionary (a=FF r=00 g=00 b=FF → blue) and premultiplied.
#[test]
fn matte_tiles_fill_solid_argb() {
    let mut doc = Document::default();
    // 0xFF_00_00_FF: alpha FF, red 00, green 00, blue FF.
    let image = image_object(
        &mut doc.registry,
        (0.0, 0.0),
        (3, 3),
        128,
        vec![TileRef::Matte(0xFF_00_00_FF)],
        1000,
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![image])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), BLUE, "matte corner");
    assert_eq!(pixel(&render, 2, 2), BLUE, "matte far corner");
    assert_eq!(pixel(&render, 3, 3), [0, 0, 0, 0], "past the extent");
}

/// A dangling `TIDi` is a document defect: render refuses with
/// `TileBankMiss` instead of silently dropping the bitmap.
#[test]
fn missing_tile_bank_entry_refuses() {
    let mut doc = Document::default();
    let image = image_object(
        &mut doc.registry,
        (0.0, 0.0),
        (4, 4),
        128,
        vec![TileRef::Tile(42)],
        1000,
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![image])])]));

    assert_eq!(
        render(&doc, 0, 0),
        Err(ComposeError::TileBankMiss { tile: 42 })
    );
}

/// `TSZi` 256 has no tile-bank bridge (bank slots are fixed 128²);
/// the refusal names both sizes.
#[test]
fn unsupported_tile_size_refuses() {
    let mut doc = Document::default();
    let image = image_object(
        &mut doc.registry,
        (0.0, 0.0),
        (4, 4),
        256,
        vec![TileRef::Matte(0xFF_00_00_FF)],
        1000,
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![image])])]));

    let error = render(&doc, 0, 0).expect_err("refused");
    assert!(
        error.to_string().contains("256") && error.to_string().contains("128"),
        "refusal names both sizes: {error}"
    );
}

/// An image entirely off the canvas composes to a transparent canvas,
/// not an error.
#[test]
fn fully_offscreen_image_renders_nothing() {
    let mut doc = Document::default();
    doc.tiles.insert(0, tile(RED, None));
    let image = image_object(
        &mut doc.registry,
        (100.0, 200.0),
        (4, 4),
        128,
        vec![TileRef::Tile(0)],
        1000,
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![image])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_all_transparent(&render);
}

/// A hidden image contributes nothing.
#[test]
fn hidden_image_renders_nothing() {
    let mut doc = Document::default();
    doc.tiles.insert(0, tile(RED, None));
    let image = image_object(
        &mut doc.registry,
        (0.0, 0.0),
        (4, 4),
        128,
        vec![TileRef::Tile(0)],
        1000,
    );
    if let ObjectData::Image(image) = doc.registry.get_mut(image).expect("registered") {
        image.visible = false;
    }
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![image])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_all_transparent(&render);
}

/// Image opacity rides the same compositing math as path opacity:
/// red at `OPAi=200` (α = 51/255 exact) over opaque white gives
/// out = 51 + 255·204/255 = 255 on red and 204 on green/blue —
/// integer-exact in the covered extent, white outside it.
#[test]
fn image_opacity_composes_over_a_path() {
    let mut doc = Document::default();
    let white = doc.registry.insert(ObjectData::Path(doc::PathObject {
        contours: vec![doc::Contour {
            closed: true,
            point_count: 4,
            points: vec![
                doc::PathPoint {
                    anchor: Point::new(0.0, 0.0),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                },
                doc::PathPoint {
                    anchor: Point::new(4.0, 0.0),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                },
                doc::PathPoint {
                    anchor: Point::new(4.0, 4.0),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                },
                doc::PathPoint {
                    anchor: Point::new(0.0, 4.0),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                },
            ],
        }],
        primitive: None,
        even_odd: false,
        paint: Some(doc::PaintStyle {
            stroke_raw: 0,
            fill_raw: 0xFF_FF_FF_FF,
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
    }));
    doc.tiles.insert(0, tile(RED, Some((2, 2))));
    let image = image_object(
        &mut doc.registry,
        (1.0, 1.0),
        (2, 2),
        128,
        vec![TileRef::Tile(0)],
        200,
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![white, image])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 1, 1), [255, 204, 204, 255], "50% exact mix");
    assert_eq!(pixel(&render, 0, 0), [255, 255, 255, 255], "outside: white");
    assert_eq!(pixel(&render, 3, 3), [255, 255, 255, 255], "outside: white");
}

/// Images route through the live-effect stack exactly like paths:
/// inverting an opaque red tile yields its exact complement.
#[test]
fn image_effects_route_through_the_filter_stack() {
    let mut doc = Document::default();
    doc.tiles.insert(0, tile(RED, Some((2, 2))));
    let image = image_object(
        &mut doc.registry,
        (0.0, 0.0),
        (2, 2),
        128,
        vec![TileRef::Tile(0)],
        1000,
    );
    if let ObjectData::Image(image) = doc.registry.get_mut(image).expect("registered") {
        image.effects = vec![doc::Effect {
            category: String::new(),
            internal_name: "invert".to_owned(),
            properties: Vec::new(),
        }];
    }
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![image])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), [0, 255, 255, 255], "invert(red)");
    assert_eq!(pixel(&render, 3, 3), [0, 0, 0, 0], "outside: untouched");
}

/// States isolate images: cell 0's red image paints frame 0, cell 1's
/// blue image paints frame 1, and neither leaks into the other.
#[test]
fn each_state_shows_only_its_own_cell() {
    let mut doc = Document::default();
    doc.tiles.insert(0, tile(RED, Some((2, 2))));
    doc.tiles.insert(1, tile(BLUE, Some((2, 2))));
    let red = image_object(
        &mut doc.registry,
        (0.0, 0.0),
        (2, 2),
        128,
        vec![TileRef::Tile(0)],
        1000,
    );
    let blue = image_object(
        &mut doc.registry,
        (0.0, 0.0),
        (2, 2),
        128,
        vec![TileRef::Tile(1)],
        1000,
    );
    doc.pages.push(page(
        4.0,
        4.0,
        vec![layer(vec![cell(vec![red]), cell(vec![blue])])],
    ));

    let frame0 = render(&doc, 0, 0).expect("frame 0");
    assert_eq!(pixel(&frame0, 0, 0), RED);
    let frame1 = render(&doc, 0, 1).expect("frame 1");
    assert_eq!(pixel(&frame1, 0, 0), BLUE);
    assert_eq!(pixel(&frame1, 2, 2), [0, 0, 0, 0]);
}
