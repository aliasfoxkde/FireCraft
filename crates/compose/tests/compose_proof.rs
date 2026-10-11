//! Hand-computed composition proofs (M4 task 2 stopping condition):
//! documents whose pixels are derivable by arithmetic — integer-exact
//! at every quantization point, so no rounding rule can move them —
//! compose to exactly those pixels.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compose::{ComposeError, render};
use doc::{
    Canvas, Cell, Contour, Document, Layer, LayerList, ObjectData, Page, PageInfo, PaintStyle,
    PathObject, PathPoint, Registry,
};
use geom::Point;

// --- document builders ---------------------------------------------------

fn canvas(width: f64, height: f64) -> Canvas {
    Canvas {
        x: 0.0,
        y: 0.0,
        width,
        height,
        resolution: 72.0,
        resolution_unit: 0,
        background_raw: 0xFF_FF_FF,
    }
}

fn page(width: f64, height: f64, layers: Vec<Layer>) -> Page {
    Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: 2,
            frame_count_duplicate: 2,
            request_id: 0,
            canvas: canvas(width, height),
            onion_before: 0,
            onion_after: 0,
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
    }
}

fn anchor(x: f64, y: f64) -> PathPoint {
    PathPoint {
        anchor: Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    }
}

fn rect_contour(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
    Contour {
        closed: true,
        point_count: 4,
        points: vec![
            anchor(x0, y0),
            anchor(x1, y0),
            anchor(x1, y1),
            anchor(x0, y1),
        ],
    }
}

/// A filled rectangle; the stroke colour is fully transparent so only
/// the fill shows (the 1px M4 stroke would otherwise ring the shape).
fn rect_object(
    registry: &mut Registry,
    contour: Contour,
    fill_abgr: i64,
    opacity: i64,
    blend: i64,
) -> doc::ObjectId {
    registry.insert(ObjectData::Path(PathObject {
        contours: vec![contour],
        primitive: None,
        even_odd: false,
        paint: Some(PaintStyle {
            stroke_raw: 0,
            fill_raw: fill_abgr,
        }),
        fill_start: None,
        fill_end: None,
        fill_third: None,
        brush_placement: None,
        fill_on_top: None,
        locked: false,
        visible: true,
        opacity,
        blend,
        name: None,
        effects: Vec::new(),
    }))
}

/// Opaque white ABGR.
const WHITE: i64 = 0xFF_FF_FF_FF;
/// Opaque black ABGR.
const BLACK: i64 = 0xFF_00_00_00;
/// Opaque red ABGR (a=FF b=00 g=00 r=FF).
const RED: i64 = 0xFF_00_00_FF;
/// Opaque blue ABGR (a=FF b=FF g=00 r=00).
const BLUE: i64 = 0xFF_FF_00_00;

fn layer(cells: Vec<Cell>) -> Layer {
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

fn cell(ids: Vec<doc::ObjectId>) -> Cell {
    Cell {
        locked: false,
        visible: true,
        elements: ids,
    }
}

/// Asserts every pixel of the render equals `expected` (premultiplied
/// RGBA8).
fn assert_solid(render: &compose::Render, expected: [u8; 4]) {
    assert_eq!(
        render.pixels.len(),
        usize::from(render.width) * usize::from(render.height) * 4
    );
    for (i, pixel) in render.pixels.chunks_exact(4).enumerate() {
        assert_eq!(
            pixel,
            expected,
            "pixel {i} ({}, {}) diverged",
            i % usize::from(render.width),
            i / usize::from(render.width)
        );
    }
}

fn pixel(render: &compose::Render, x: u32, y: u32) -> [u8; 4] {
    let index = (y * u32::from(render.width) + x) as usize * 4;
    render.pixels[index..index + 4].try_into().expect("pixel")
}

// --- proofs --------------------------------------------------------------

/// The stopping condition: painted paths on two layers with different
/// opacities. Black at 200‰ (α = 0.2 = 51/255 exactly) over opaque
/// white: out = 0·51/255 + 255·204/255 = 204 on every channel —
/// integer-exact in the u8 pipeline, whichever way any intermediate
/// rounds.
#[test]
fn two_layers_with_different_opacities_compose_to_hand_computed_pixels() {
    let mut doc = Document::default();
    let white = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        WHITE,
        1000,
        0,
    );
    let black = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        BLACK,
        200,
        0,
    );
    doc.pages.push(page(
        4.0,
        4.0,
        vec![
            layer(vec![cell(vec![white])]),
            layer(vec![cell(vec![black])]),
        ],
    ));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!((render.width, render.height), (4, 4));
    assert_solid(&render, [204, 204, 204, 255]);
}

/// Blue `BLDi=2` (multiply) over opaque red: out = src·dst = (0,0,0) —
/// integer-exact.
#[test]
fn multiply_blend_of_primaries_is_black() {
    let mut doc = Document::default();
    let red = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        RED,
        1000,
        0,
    );
    let blue = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        BLUE,
        1000,
        2,
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![red, blue])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_solid(&render, [0, 0, 0, 255]);
}

/// Even-odd fill rule: two same-winding rectangles overlapping on
/// x∈[2,4) leave the overlap unfilled, on an 8×4 canvas.
#[test]
fn even_odd_leaves_overlaps_unfilled() {
    let mut doc = Document::default();
    let both = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        RED,
        1000,
        0,
    );
    let ObjectData::Path(path) = doc.registry.get_mut(both).expect("registered") else {
        panic!("inserted a path");
    };
    path.even_odd = true;
    path.contours.push(rect_contour(2.0, 0.0, 6.0, 4.0));
    doc.pages
        .push(page(8.0, 4.0, vec![layer(vec![cell(vec![both])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(
        pixel(&render, 1, 1),
        [255, 0, 0, 255],
        "single cover: filled"
    );
    assert_eq!(pixel(&render, 3, 1), [0, 0, 0, 0], "double cover: hole");
    assert_eq!(
        pixel(&render, 5, 1),
        [255, 0, 0, 255],
        "single cover: filled"
    );
    assert_eq!(pixel(&render, 7, 1), [0, 0, 0, 0], "outside: empty");
}

/// Non-zero rule with the same winding fills the overlap.
#[test]
fn non_zero_fills_the_overlap() {
    let mut doc = Document::default();
    let both = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        RED,
        1000,
        0,
    );
    let ObjectData::Path(path) = doc.registry.get_mut(both).expect("registered") else {
        panic!("inserted a path");
    };
    path.contours.push(rect_contour(2.0, 0.0, 6.0, 4.0));
    doc.pages
        .push(page(8.0, 4.0, vec![layer(vec![cell(vec![both])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(
        pixel(&render, 3, 1),
        [255, 0, 0, 255],
        "double cover: filled"
    );
    assert_eq!(pixel(&render, 7, 1), [0, 0, 0, 0], "outside: empty");
}

/// A group at 200‰ around an opaque blue child over opaque white: the
/// child fills the group layer at full alpha, the group composites at
/// 51/255 → (204, 204, 255, 255) — every term integer-exact.
#[test]
fn group_opacity_composes_over_its_children() {
    let mut doc = Document::default();
    let white = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        WHITE,
        1000,
        0,
    );
    let blue = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        BLUE,
        1000,
        0,
    );
    let group = doc.registry.insert(ObjectData::Group(doc::GroupObject {
        group_type: 0,
        name: None,
        children: vec![blue],
        opacity: 200,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        effects: Vec::new(),
    }));
    doc.pages.push(page(
        4.0,
        4.0,
        vec![
            layer(vec![cell(vec![white])]),
            layer(vec![cell(vec![group])]),
        ],
    ));

    let render = render(&doc, 0, 0).expect("render");
    assert_solid(&render, [204, 204, 255, 255]);
}

/// Structural visibility: a hidden layer, a hidden cell in the
/// rendered frame, a per-frame override, a hidden path and a path
/// without paint all contribute nothing.
#[test]
fn structural_visibility_rules() {
    let mut doc = Document::default();
    let shown = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        RED,
        1000,
        0,
    );
    let hidden_layer_rect = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        BLACK,
        1000,
        0,
    );
    let hidden_cell_rect = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        BLACK,
        1000,
        0,
    );
    let frame_hidden_rect = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        BLACK,
        1000,
        0,
    );
    let invisible_rect = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        BLACK,
        1000,
        0,
    );
    let unpainted = doc.registry.insert(ObjectData::Path(PathObject {
        contours: vec![rect_contour(0.0, 0.0, 4.0, 4.0)],
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
    }));

    let mut hidden_layer = layer(vec![cell(vec![hidden_layer_rect])]);
    hidden_layer.display = false;
    let mut hidden_cell_layer = layer(vec![cell(vec![hidden_cell_rect])]);
    hidden_cell_layer.cells[0].visible = false;
    let mut frame_layer = layer(vec![cell(vec![frame_hidden_rect])]);
    frame_layer.visibility_per_frame = Some(vec![false, false]);
    let invisible_layer = layer(vec![cell(vec![invisible_rect])]);
    if let ObjectData::Path(path) = doc.registry.get_mut(invisible_rect).expect("registered") {
        path.visible = false;
    }

    doc.pages.push(page(
        4.0,
        4.0,
        vec![
            layer(vec![cell(vec![shown]), cell(vec![shown])]),
            hidden_layer,
            hidden_cell_layer,
            frame_layer,
            invisible_layer,
            layer(vec![cell(vec![unpainted])]),
        ],
    ));

    let frame0 = render(&doc, 0, 0).expect("render frame 0");
    assert_solid(&frame0, [255, 0, 0, 255]);

    // Frame 1: the per-frame override hides its layer, the hidden cell
    // layer stays hidden — still only the red base.
    let frame1 = render(&doc, 0, 1).expect("render frame 1");
    assert_solid(&frame1, [255, 0, 0, 255]);
}

/// A frame past a layer's cell list renders that layer as empty
/// without erroring.
#[test]
fn frame_past_cell_list_renders_empty() {
    let mut doc = Document::default();
    let rect = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        RED,
        1000,
        0,
    );
    doc.pages.push(page(
        4.0,
        4.0,
        vec![layer(vec![cell(vec![rect])])], // one cell only
    ));
    let render = render(&doc, 0, 5).expect("render");
    assert_solid(&render, [0, 0, 0, 0]);
}

/// An empty document composes to a transparent canvas.
#[test]
fn empty_document_is_transparent() {
    let mut doc = Document::default();
    doc.pages.push(page(3.0, 2.0, Vec::new()));
    let render = render(&doc, 0, 0).expect("render");
    assert_eq!((render.width, render.height), (3, 2));
    assert_solid(&render, [0, 0, 0, 0]);
}

/// Repeated renders are byte-identical (the determinism floor, at
/// unit scale; corpus-wide pinning lands with the golden machinery).
#[test]
fn repeated_renders_are_byte_identical() {
    let mut doc = Document::default();
    let white = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 8.0, 8.0),
        WHITE,
        1000,
        0,
    );
    let red = rect_object(
        &mut doc.registry,
        rect_contour(1.0, 1.0, 7.0, 7.0),
        RED,
        600,
        0,
    );
    let group = doc.registry.insert(ObjectData::Group(doc::GroupObject {
        group_type: 0,
        name: None,
        children: vec![red],
        opacity: 800,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        effects: Vec::new(),
    }));
    doc.pages.push(page(
        8.0,
        8.0,
        vec![
            layer(vec![cell(vec![white, group])]),
            layer(vec![cell(vec![])]),
        ],
    ));
    let first = render(&doc, 0, 0).expect("render");
    let second = render(&doc, 0, 0).expect("render");
    assert_eq!(first, second);
}

/// Strokes paint (smoke — exact stroke geometry is the rasterizer's
/// domain; the M4 model pins a 1px centered stroke).
#[test]
fn stroke_paints_along_the_path() {
    let mut doc = Document::default();
    let line = doc.registry.insert(ObjectData::Path(PathObject {
        contours: vec![Contour {
            closed: false,
            point_count: 2,
            points: vec![anchor(2.0, 0.0), anchor(2.0, 4.0)],
        }],
        primitive: None,
        even_odd: false,
        paint: Some(PaintStyle {
            stroke_raw: BLACK,
            fill_raw: 0,
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
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![line])])]));

    let render = render(&doc, 0, 0).expect("render");
    // A 1px stroke centered on x=2 straddles pixel columns 1 and 2;
    // both must carry paint, the outer columns must stay empty.
    for y in 0..4u32 {
        assert!(
            pixel(&render, 1, y)[3] > 0 && pixel(&render, 2, y)[3] > 0,
            "stroke missing at y={y}"
        );
        assert_eq!(pixel(&render, 0, y), [0, 0, 0, 0], "outside left");
        assert_eq!(pixel(&render, 3, y), [0, 0, 0, 0], "outside right");
    }
}

// --- errors --------------------------------------------------------------

#[test]
fn page_out_of_bounds_is_an_error() {
    let mut doc = Document::default();
    doc.pages.push(page(4.0, 4.0, Vec::new()));
    assert_eq!(
        render(&doc, 1, 0),
        Err(ComposeError::PageOutOfBounds { page: 1 })
    );
}

#[test]
fn dangling_object_id_is_an_error() {
    let mut doc = Document::default();
    let id = doc.registry.insert(ObjectData::Text(doc::TextObject {
        frame: geom::Rect::new(0.0, 0.0, 1.0, 1.0),
        transform: None,
        effects: Vec::new(),
    }));
    doc.registry.remove(id);
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![id])])]));
    assert_eq!(
        render(&doc, 0, 0),
        Err(ComposeError::DanglingId {
            index: id.index() as u32,
            generation: id.generation(),
        })
    );
}

#[test]
fn degenerate_canvas_is_an_error() {
    let mut doc = Document::default();
    doc.pages.push(page(0.0, 4.0, Vec::new()));
    assert_eq!(render(&doc, 0, 0), Err(ComposeError::CanvasEmpty));
}

#[test]
fn oversized_canvas_is_an_error() {
    let mut doc = Document::default();
    doc.pages.push(page(70_000.0, 4.0, Vec::new()));
    assert_eq!(
        render(&doc, 0, 0),
        Err(ComposeError::CanvasTooLarge {
            width: 70_000.0,
            height: 4.0,
        })
    );
}
