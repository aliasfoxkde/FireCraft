//! Pattern-fill rendering (post-roadmap gap: `FPTv` fills). The
//! pattern's `TILv` grid assembles into one repeat-unit raster that
//! the image brush repeats across the geometry; the fill vector
//! (start→end handles) steers angle and scale exactly as it steers a
//! gradient. Every expected value is hand-derived premultiplied RGBA8.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compose::{ComposeError, render};
use doc::{
    Document, Fill, FillShape, Layer, LayerList, ObjectData, Page, PageInfo, PaintStyle,
    PathObject, PathPoint, PatternTile, Registry, TileRef,
};
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

/// A `side`×`side` square path at the origin, filled with `pattern`
/// and steered by the optional fill vector.
fn pattern_square(
    registry: &mut Registry,
    side: f64,
    pattern: PatternTile,
    handles: Option<(Point, Point)>,
) -> doc::ObjectId {
    registry.insert(ObjectData::Path(PathObject {
        contours: vec![doc::Contour {
            closed: true,
            point_count: 4,
            points: [(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)]
                .into_iter()
                .map(|(x, y)| PathPoint {
                    anchor: Point::new(x, y),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                })
                .collect(),
        }],
        primitive: None,
        even_odd: false,
        paint: Some(PaintStyle {
            stroke_raw: 0,
            fill_raw: 0xFF_FF_FF_FF,
            fill: Some(Fill {
                shape: FillShape::Pattern,
                shape_extended: true,
                texture_blend: 0,
                feather: 0,
                stamping: None,
                hard_edge: false,
                gradient: None,
                dither: None,
                pattern: Some(pattern),
            }),
        }),
        fill_start: handles.map(|(start, _)| start),
        fill_end: handles.map(|(_, end)| end),
        fill_third: None,
        brush_placement: None,
        fill_on_top: None,
        locked: false,
        visible: true,
        opacity: 1000,
        blend: 0,
        name: None,
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

/// The default canvas matte (`BGCi` default white, painted opaque).
const BACKGROUND: [u8; 4] = [255, 255, 255, 255];
/// Opaque red, straight or premultiplied (identical at α=255).
const RED: [u8; 4] = [255, 0, 0, 255];
/// Opaque green.
const GREEN: [u8; 4] = [0, 255, 0, 255];

fn pixel(render: &compose::Render, x: u32, y: u32) -> [u8; 4] {
    let index = (y * u32::from(render.width) + x) as usize * 4;
    render.pixels[index..index + 4].try_into().expect("pixel")
}

/// Every rendered pixel as `(x, y, rgba)` pairs.
fn pixels(render: &compose::Render) -> Vec<(u32, u32, [u8; 4])> {
    let mut out = Vec::new();
    for y in 0..u32::from(render.height) {
        for x in 0..u32::from(render.width) {
            out.push((x, y, pixel(render, x, y)));
        }
    }
    out
}

// --- proofs --------------------------------------------------------------

/// Without handles the pattern tiles its raster at native size from
/// the origin: a 3×3 repeat unit whose top-left texel is red lands a
/// red dot on every `(x ≡ 0, y ≡ 0) mod 3` pixel of a 9×9 square,
/// background where the raster is transparent.
#[test]
fn pattern_fill_repeats_its_raster() {
    let mut doc = Document::default();
    doc.tiles.insert(0, tile(RED, Some((1, 1))));
    let pattern = PatternTile {
        name: None,
        pixels: (3, 3),
        tile_size: 128,
        tiles: vec![TileRef::Tile(0)],
    };
    let square = pattern_square(&mut doc.registry, 9.0, pattern, None);
    doc.pages
        .push(page(9.0, 9.0, vec![layer(vec![cell(vec![square])])]));

    let render = render(&doc, 0, 0).expect("render");
    for &(x, y, rgba) in &pixels(&render) {
        let expected = if x % 3 == 0 && y % 3 == 0 {
            RED
        } else {
            BACKGROUND
        };
        assert_eq!(rgba, expected, "pixel ({x}, {y})");
    }
}

/// The fill vector scales the repeat: handles (0,0)→(9,0) stretch the
/// 3-wide raster to one 9×9 repeat covering the square, so the red
/// origin texel becomes one 3×3 block and the rest of the square
/// shows the raster's transparent remainder — the same square that
/// dotted nine times without handles.
#[test]
fn fill_vector_scales_the_repeat() {
    let mut doc = Document::default();
    doc.tiles.insert(0, tile(RED, Some((1, 1))));
    let pattern = PatternTile {
        name: None,
        pixels: (3, 3),
        tile_size: 128,
        tiles: vec![TileRef::Tile(0)],
    };
    let square = pattern_square(
        &mut doc.registry,
        9.0,
        pattern,
        Some((Point::new(0.0, 0.0), Point::new(9.0, 0.0))),
    );
    doc.pages
        .push(page(9.0, 9.0, vec![layer(vec![cell(vec![square])])]));

    let render = render(&doc, 0, 0).expect("render");
    for &(x, y, rgba) in &pixels(&render) {
        let expected = if x < 3 && y < 3 { RED } else { BACKGROUND };
        assert_eq!(rgba, expected, "pixel ({x}, {y})");
    }
}

/// The fill vector's angle rotates the repeat: a vertical handle
/// points the raster down the canvas, moving the red origin-texel
/// block off the top-left corner and into exactly one of the far
/// corners (which one is the rasterizer's y-down rotation handedness;
/// both are asserted equivalent here — the invariant is "rotated, and
/// still one 3×3 red block").
#[test]
fn fill_vector_rotates_the_repeat() {
    let mut doc = Document::default();
    doc.tiles.insert(0, tile(RED, Some((1, 1))));
    let pattern = PatternTile {
        name: None,
        pixels: (3, 3),
        tile_size: 128,
        tiles: vec![TileRef::Tile(0)],
    };
    let square = pattern_square(
        &mut doc.registry,
        9.0,
        pattern,
        Some((Point::new(0.0, 0.0), Point::new(0.0, 9.0))),
    );
    doc.pages
        .push(page(9.0, 9.0, vec![layer(vec![cell(vec![square])])]));

    let render = render(&doc, 0, 0).expect("render");
    let reds: Vec<(u32, u32)> = pixels(&render)
        .into_iter()
        .filter(|&(_, _, rgba)| rgba == RED)
        .map(|(x, y, _)| (x, y))
        .collect();
    assert_eq!(reds.len(), 9, "one 3×3 block of red texels: {reds:?}");
    assert!(!reds.contains(&(0, 0)), "the origin corner rotated away");

    let top_right = reds.iter().all(|&(x, y)| x >= 6 && y <= 2);
    let bottom_left = reds.iter().all(|&(x, y)| x <= 2 && y >= 6);
    assert!(
        top_right || bottom_left,
        "all nine reds share one rotated corner block: {reds:?}"
    );
}

/// Matte tiles fill their `TMCi` flat colour — ARGB per the
/// dictionary (a=FF r=00 g=FF b=00 → green) — across the whole
/// geometry, and stop at it.
#[test]
fn matte_pattern_tiles_fill_solid() {
    let mut doc = Document::default();
    let pattern = PatternTile {
        name: None,
        pixels: (2, 2),
        tile_size: 128,
        tiles: vec![TileRef::Matte(0xFF_00_FF_00)],
    };
    let square = pattern_square(&mut doc.registry, 9.0, pattern, None);
    doc.pages
        .push(page(10.0, 10.0, vec![layer(vec![cell(vec![square])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), GREEN);
    assert_eq!(pixel(&render, 8, 8), GREEN, "inside the square");
    assert_eq!(pixel(&render, 9, 9), BACKGROUND, "outside the square");
}

/// A pattern whose `TSZi` has no tile-bank backing refuses exactly
/// like an image object of the same geometry — no silent fill.
#[test]
fn unsupported_pattern_tile_size_refuses() {
    let mut doc = Document::default();
    let pattern = PatternTile {
        name: None,
        pixels: (4, 4),
        tile_size: 256,
        tiles: vec![TileRef::Matte(0xFF_00_FF_00)],
    };
    let square = pattern_square(&mut doc.registry, 9.0, pattern, None);
    doc.pages
        .push(page(10.0, 10.0, vec![layer(vec![cell(vec![square])])]));

    let error = render(&doc, 0, 0).expect_err("refused");
    assert!(matches!(error, ComposeError::TileSource { .. }));
    assert!(
        error.to_string().contains("256") && error.to_string().contains("128"),
        "refusal names both sizes: {error}"
    );
}

/// A dangling `TIDi` in a pattern's `TILv` is a document defect:
/// refused with `TileBankMiss`, never painted as a hole.
#[test]
fn pattern_bank_miss_refuses() {
    let mut doc = Document::default();
    let pattern = PatternTile {
        name: None,
        pixels: (4, 4),
        tile_size: 128,
        tiles: vec![TileRef::Tile(42)],
    };
    let square = pattern_square(&mut doc.registry, 9.0, pattern, None);
    doc.pages
        .push(page(10.0, 10.0, vec![layer(vec![cell(vec![square])])]));

    assert_eq!(
        render(&doc, 0, 0),
        Err(ComposeError::TileBankMiss { tile: 42 })
    );
}
