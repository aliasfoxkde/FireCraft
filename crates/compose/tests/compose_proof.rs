//! Hand-computed composition proofs (M4 task 2 stopping condition):
//! documents whose pixels are derivable by arithmetic — integer-exact
//! at every quantization point, so no rounding rule can move them —
//! compose to exactly those pixels.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compose::{ComposeError, EffectCache, render, render_with_cache};
use doc::{
    Canvas, Cell, Contour, Document, Effect, EffectProperty, EffectValue, Layer, LayerList,
    ObjectData, Page, PageInfo, PaintStyle, PathObject, PathPoint, Registry,
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
            fill: None,
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

// --- live effects (M7 task 4) ---------------------------------------------

/// Opaque mid-gray ABGR (r=g=b=128 — the contrast pivot).
const GRAY: i64 = 0xFF_80_80_80;
/// Opaque level-100 gray ABGR (off the pivot, so contrast moves it).
const GRAY100: i64 = 0xFF64_6464;
/// Brightness/Contrast `EffectMoaID` (dictionary line 1497).
const GUID_BC: &str = "3439b08c-1921-11d3-9bde00e02910d580";
/// Gaussian Blur `EffectMoaID` (dictionary line 1508).
const GUID_GB: &str = "d04ef8c0-71b3-11d1-8c8200a024cdc039";
/// Invert `EffectMoaID` (dictionary line 1512).
const GUID_INVERT: &str = "d2541291-70d6-11d1-8c8000a024cdc039";

/// Builds one live effect from `DCKs`/`DCVs` pairs.
fn effect(name: &str, props: &[(&str, &str)]) -> Effect {
    Effect {
        category: String::new(),
        internal_name: name.to_owned(),
        properties: props
            .iter()
            .map(|(key, value)| EffectProperty {
                key: (*key).to_owned(),
                value: EffectValue::Text((*value).to_owned()),
            })
            .collect(),
    }
}

/// Attaches a live-effect stack to an existing path object.
fn set_effects(doc: &mut Document, id: doc::ObjectId, effects: Vec<Effect>) {
    let ObjectData::Path(path) = doc.registry.get_mut(id).expect("registered") else {
        panic!("inserted a path");
    };
    path.effects = effects;
}

/// Invert is alpha-passing and integer-exact: red (255,0,0) becomes
/// (0,255,255) on every covered pixel. Both identities — the CS6 GUID
/// and the bare internal name — must produce it.
#[test]
fn invert_filter_negates_solid_fill() {
    for stack in [
        vec![effect("dropShadow", &[("EffectMoaID", GUID_INVERT)])],
        vec![effect("invert", &[])],
    ] {
        let mut doc = Document::default();
        let rect = rect_object(
            &mut doc.registry,
            rect_contour(0.0, 0.0, 4.0, 4.0),
            RED,
            1000,
            0,
        );
        set_effects(&mut doc, rect, stack.clone());
        doc.pages
            .push(page(4.0, 4.0, vec![layer(vec![cell(vec![rect])])]));

        let render = render(&doc, 0, 0).expect("render");
        assert_solid(&render, [0, 255, 255, 255]);
    }
}

/// Brightness/Contrast against the pinned LUT arithmetic:
/// `brightness 25` lifts level 128 by `round(25·128/100) = 32` → 160;
/// `contrast 50` contracts level 100 about 128 by 1.5×:
/// `round((100−128)·150/100) = −42` → 86. Both exact integers.
#[test]
fn brightness_and_contrast_match_the_pinned_lut() {
    let mut doc = Document::default();
    let bright = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        GRAY,
        1000,
        0,
    );
    set_effects(
        &mut doc,
        bright,
        vec![effect(
            "brightnessContrast",
            &[
                ("EffectMoaID", GUID_BC),
                ("brightness_amount", "25"),
                ("contrast_amount", "0"),
            ],
        )],
    );
    let contrast = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 4.0, 4.0, 8.0),
        GRAY100,
        1000,
        0,
    );
    set_effects(
        &mut doc,
        contrast,
        vec![effect(
            "brightnessContrast",
            &[
                ("EffectMoaID", GUID_BC),
                ("brightness_amount", "0"),
                ("contrast_amount", "50"),
            ],
        )],
    );
    doc.pages.push(page(
        4.0,
        8.0,
        vec![layer(vec![cell(vec![bright, contrast])])],
    ));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 1, 1), [160, 160, 160, 255], "brightness");
    assert_eq!(pixel(&render, 1, 5), [86, 86, 86, 255], "contrast");
}

/// Stack order where the math says it matters: on level 128,
/// `brightness +25` then invert gives invert(160) = 95, while invert
/// then brightness gives 127 + 32 = 159. Both values pinned; the two
/// orders must differ.
#[test]
fn stack_order_changes_where_the_math_says() {
    let stack = |reversed: bool| {
        let mut effects = vec![
            effect(
                "brightnessContrast",
                &[
                    ("EffectMoaID", GUID_BC),
                    ("brightness_amount", "25"),
                    ("contrast_amount", "0"),
                ],
            ),
            effect("invert", &[("EffectMoaID", GUID_INVERT)]),
        ];
        if reversed {
            effects.reverse();
        }
        effects
    };
    let mut doc = Document::default();
    let first = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        GRAY,
        1000,
        0,
    );
    set_effects(&mut doc, first, stack(false));
    let second = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 4.0, 4.0, 8.0),
        GRAY,
        1000,
        0,
    );
    set_effects(&mut doc, second, stack(true));
    doc.pages
        .push(page(4.0, 8.0, vec![layer(vec![cell(vec![first, second])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(
        pixel(&render, 1, 1),
        [95, 95, 95, 255],
        "brightness then invert"
    );
    assert_eq!(
        pixel(&render, 1, 5),
        [159, 159, 159, 255],
        "invert then brightness"
    );
}

/// Point ops whose LUTs compose exactly commute: brightness +25 twice
/// is brightness +50 (32 + 32 = 64 levels both ways, no rounding
/// drift).
#[test]
fn commuting_brightness_shifts_compose_exactly() {
    let lift = |amount: &str| {
        vec![effect(
            "brightnessContrast",
            &[
                ("EffectMoaID", GUID_BC),
                ("brightness_amount", amount),
                ("contrast_amount", "0"),
            ],
        )]
    };
    let mut doc = Document::default();
    let twice = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        GRAY,
        1000,
        0,
    );
    let mut stacked = lift("25");
    stacked.extend(lift("25"));
    set_effects(&mut doc, twice, stacked);
    let once = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 4.0, 4.0, 8.0),
        GRAY,
        1000,
        0,
    );
    set_effects(&mut doc, once, lift("50"));
    doc.pages
        .push(page(4.0, 8.0, vec![layer(vec![cell(vec![twice, once])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 1, 1), [192, 192, 192, 255], "25 ∘ 25");
    assert_eq!(pixel(&render, 1, 5), [192, 192, 192, 255], "50");
}

/// Effects the kernel catalog cannot interpret (a GUIDed dropShadow)
/// and hidden effects (`EffectIsVisible` false) leave the object
/// rendered unfiltered — never dropped, never approximated.
#[test]
fn unsupported_and_hidden_effects_render_unfiltered() {
    let mut doc = Document::default();
    let plain = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        RED,
        1000,
        0,
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![plain])])]));
    let unfiltered = render(&doc, 0, 0).expect("render");

    let mut dressed = Document::default();
    let shadowed = rect_object(
        &mut dressed.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        RED,
        1000,
        0,
    );
    set_effects(
        &mut dressed,
        shadowed,
        vec![effect(
            "dropShadow",
            &[
                ("EffectMoaID", "deadbeef-0000-11d1-8c8000a024cdc039"),
                ("ShadowColor", "0"),
                ("ShadowBlur", "4"),
            ],
        )],
    );
    let hidden = rect_object(
        &mut dressed.registry,
        rect_contour(0.0, 4.0, 4.0, 8.0),
        RED,
        1000,
        0,
    );
    set_effects(
        &mut dressed,
        hidden,
        vec![effect(
            "invert",
            &[("EffectMoaID", GUID_INVERT), ("EffectIsVisible", "false")],
        )],
    );
    dressed.pages.push(page(
        4.0,
        8.0,
        vec![layer(vec![cell(vec![shadowed, hidden])])],
    ));
    let dressed_render = render(&dressed, 0, 0).expect("render");

    assert_eq!(pixel(&dressed_render, 1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(&dressed_render, 1, 5), [255, 0, 0, 255]);
    assert_eq!(dressed_render.pixels.len() / 2, unfiltered.pixels.len());
}

/// Gaussian blur must spread past the shape's bounding box (the
/// full-canvas offscreen, decision 0005): paint appears at (5,5) —
/// outside the 8..16 geometry — and the far corner stays empty.
#[test]
fn gaussian_blur_spreads_past_the_geometry() {
    let mut doc = Document::default();
    let rect = rect_object(
        &mut doc.registry,
        rect_contour(8.0, 8.0, 16.0, 16.0),
        RED,
        1000,
        0,
    );
    set_effects(
        &mut doc,
        rect,
        vec![effect(
            "gaussianBlur",
            &[("EffectMoaID", GUID_GB), ("gaussian_blur_radius", "2")],
        )],
    );
    doc.pages
        .push(page(24.0, 24.0, vec![layer(vec![cell(vec![rect])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert!(pixel(&render, 12, 12)[3] > 200, "interior stays painted");
    assert!(pixel(&render, 5, 5)[3] > 0, "bleed lands beyond the bbox");
    assert_eq!(pixel(&render, 0, 0), [0, 0, 0, 0], "far corner untouched");
}

/// The content-hash cache: repeat renders hit identical bytes without
/// new entries; an edit re-filters (new entry, new pixels); the
/// cache-less entry point agrees with the cached one.
#[test]
fn effect_cache_hits_are_byte_identical_and_content_tracked() {
    let mut doc = Document::default();
    let rect = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        GRAY,
        1000,
        0,
    );
    set_effects(
        &mut doc,
        rect,
        vec![effect(
            "brightnessContrast",
            &[
                ("EffectMoaID", GUID_BC),
                ("brightness_amount", "25"),
                ("contrast_amount", "0"),
            ],
        )],
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![rect])])]));

    let mut cache = EffectCache::new();
    let first = render_with_cache(&doc, 0, 0, &mut cache).expect("render");
    assert_eq!(cache.len(), 1, "one filtered object → one entry");
    let second = render_with_cache(&doc, 0, 0, &mut cache).expect("render");
    assert_eq!(first, second, "a hit must be byte-identical");
    assert_eq!(cache.len(), 1, "hits do not grow the cache");
    assert_eq!(
        first.pixels,
        render(&doc, 0, 0).expect("uncached").pixels,
        "cached and uncached agree"
    );

    let ObjectData::Path(path) = doc.registry.get_mut(rect).expect("registered") else {
        panic!("inserted a path");
    };
    path.contours[0].points[0].anchor = Point::new(1.0, 1.0);
    let third = render_with_cache(&doc, 0, 0, &mut cache).expect("render");
    assert_eq!(cache.len(), 2, "edited content re-filters");
    assert_ne!(third.pixels, first.pixels, "the edit changed the output");
}

/// A group's filters see the composite of its children: inverting a
/// half-red/half-blue group yields the exact complements per half.
#[test]
fn group_filters_apply_to_the_child_composite() {
    let mut doc = Document::default();
    let left = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 2.0, 4.0),
        RED,
        1000,
        0,
    );
    let right = rect_object(
        &mut doc.registry,
        rect_contour(2.0, 0.0, 4.0, 4.0),
        BLUE,
        1000,
        0,
    );
    let group = doc.registry.insert(ObjectData::Group(doc::GroupObject {
        group_type: 0,
        name: None,
        children: vec![left, right],
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        effects: vec![effect("invert", &[("EffectMoaID", GUID_INVERT)])],
    }));
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![group])])]));

    let render = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&render, 0, 0), [0, 255, 255, 255], "invert(red)");
    assert_eq!(pixel(&render, 3, 0), [255, 255, 0, 255], "invert(blue)");
}

/// Nested stacks order correctly: a child inverted inside a group
/// inverted composes to the identity on opaque pixels — byte-equal to
/// the unfiltered render.
#[test]
fn nested_inverts_cancel_exactly() {
    let build = |effects_on_child: bool, effects_on_group: bool| {
        let mut doc = Document::default();
        let rect = rect_object(
            &mut doc.registry,
            rect_contour(0.0, 0.0, 2.0, 4.0),
            RED,
            1000,
            0,
        );
        if effects_on_child {
            set_effects(&mut doc, rect, vec![effect("invert", &[])]);
        }
        let group = doc.registry.insert(ObjectData::Group(doc::GroupObject {
            group_type: 0,
            name: None,
            children: vec![rect],
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            mask_hidden: false,
            effects: if effects_on_group {
                vec![effect("invert", &[])]
            } else {
                Vec::new()
            },
        }));
        doc.pages
            .push(page(4.0, 4.0, vec![layer(vec![cell(vec![group])])]));
        doc
    };
    let plain = render(&build(false, false), 0, 0).expect("plain");
    let nested = render(&build(true, true), 0, 0).expect("nested");
    assert_eq!(nested, plain, "invert ∘ invert through a group = identity");
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
    let id = doc
        .registry
        .insert(ObjectData::Text(doc::TextObject::new(geom::Rect::new(
            0.0, 0.0, 1.0, 1.0,
        ))));
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

// --- symbol instances (M10 task 2) ----------------------------------------

use doc::{NineSliceGuides, SymbolAnimation, SymbolDefinition, SymbolInstanceObject, SymbolType};
use geom::{Affine, Rect};

/// Opaque green ABGR (a=FF b=00 g=FF r=00).
const GREEN: i64 = 0xFF_00_FF_00;
/// Opaque magenta ABGR.
const MAGENTA: i64 = 0xFF_FF_00_FF;
/// Opaque yellow ABGR.
const YELLOW: i64 = 0xFF_00_FF_FF;
/// Opaque cyan ABGR.
const CYAN: i64 = 0xFF_FF_FF_00;

/// A library definition named after `number`, whose layer holds the
/// given state cells.
fn symbol_definition(number: i64, symbol_type: SymbolType, cells: Vec<Cell>) -> SymbolDefinition {
    SymbolDefinition {
        name: format!("symbol-{number}"),
        symbol_number: number,
        origin: Point::ZERO,
        frame_count: cells.len() as i64,
        layers: LayerList {
            layers: vec![layer(cells)],
        },
        background_raw: 0,
        symbol_type,
        nine_slice: None,
        op_code: None,
    }
}

fn instance(
    number: i64,
    transform: Affine,
    animation: Option<SymbolAnimation>,
) -> SymbolInstanceObject {
    SymbolInstanceObject {
        symbol_number: number,
        transform,
        locked: false,
        name: None,
        animation,
        custom_data: std::collections::BTreeMap::new(),
    }
}

/// Transparent where nothing paints (the canvas carries no
/// background fill — proofs paint their own ground).
const CLEAR: [u8; 4] = [0, 0, 0, 0];
/// Premultiplied opaque red.
const PIXEL_RED: [u8; 4] = [255, 0, 0, 255];
/// Premultiplied opaque blue.
const PIXEL_BLUE: [u8; 4] = [0, 0, 255, 255];

/// A graphic definition holding one red 4×4 rect at the page origin;
/// an instance of it placed in the page's first cell.
fn red_dot_document() -> (Document, doc::ObjectId) {
    let mut doc = Document::default();
    let content = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        RED,
        1000,
        0,
    );
    doc.symbols.push(symbol_definition(
        7,
        SymbolType::Graphic,
        vec![cell(vec![content])],
    ));
    let placement = doc.registry.insert(ObjectData::SymbolInstance(instance(
        7,
        Affine::IDENTITY,
        None,
    )));
    (doc, placement)
}

#[test]
fn graphic_instance_renders_definition_through_its_transform() {
    let (mut doc, placement) = red_dot_document();
    doc.pages
        .push(page(8.0, 4.0, vec![layer(vec![cell(vec![placement])])]));

    // Identity placement: the definition raster lands where authored.
    let placed = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&placed, 0, 1), PIXEL_RED);
    assert_eq!(pixel(&placed, 3, 1), PIXEL_RED);
    assert_eq!(pixel(&placed, 4, 1), CLEAR);

    // Translate by 2: the same raster, shifted exactly.
    let placement = doc.registry.insert(ObjectData::SymbolInstance(instance(
        7,
        Affine::translate((2.0, 0.0)),
        None,
    )));
    doc.pages = vec![page(8.0, 4.0, vec![layer(vec![cell(vec![placement])])])];
    let shifted = render(&doc, 0, 0).expect("render");
    assert_eq!(pixel(&shifted, 1, 1), CLEAR);
    assert_eq!(pixel(&shifted, 2, 1), PIXEL_RED);
    assert_eq!(pixel(&shifted, 5, 1), PIXEL_RED);
    assert_eq!(pixel(&shifted, 6, 1), CLEAR);

    // Determinism: the same document renders identical bytes.
    let again = render(&doc, 0, 0).expect("render again");
    assert_eq!(shifted.pixels, again.pixels);
}

#[test]
fn nine_slice_instance_scales_by_guide_insets_at_two_sizes() {
    let mut doc = Document::default();
    // Source 8×8: corners RED / BLUE / CYAN / BLACK, edges MAGENTA
    // (top) and YELLOW (bottom), center GREEN. Integer-aligned
    // pieces, so every source pixel is exact.
    let pieces: [([f64; 4], i64); 9] = [
        ([0.0, 0.0, 2.0, 2.0], RED),
        ([2.0, 0.0, 6.0, 2.0], MAGENTA),
        ([6.0, 0.0, 8.0, 2.0], BLUE),
        ([0.0, 2.0, 2.0, 6.0], RED),
        ([2.0, 2.0, 6.0, 6.0], GREEN),
        ([6.0, 2.0, 8.0, 6.0], BLUE),
        ([0.0, 6.0, 2.0, 8.0], CYAN),
        ([2.0, 6.0, 6.0, 8.0], YELLOW),
        ([6.0, 6.0, 8.0, 8.0], BLACK),
    ];
    let content: Vec<doc::ObjectId> = pieces
        .iter()
        .map(|([x0, y0, x1, y1], fill)| {
            rect_object(
                &mut doc.registry,
                rect_contour(*x0, *y0, *x1, *y1),
                *fill,
                1000,
                0,
            )
        })
        .collect();
    let mut definition = symbol_definition(9, SymbolType::Graphic, vec![cell(content)]);
    definition.nine_slice = Some(NineSliceGuides {
        locked: false,
        color_raw: 0,
        // Dictionary `NGCf` order: top, bottom, left, right.
        controls: [2.0, 2.0, 2.0, 2.0],
        shape_bounds: Rect::new(0.0, 0.0, 8.0, 8.0),
        limits: Rect::new(-2.0, -2.0, 10.0, 10.0),
    });
    doc.symbols.push(definition);

    // Two sizes side by side: 16×8 (stretched wide) and 8×16
    // (stretched tall).
    let wide = doc.registry.insert(ObjectData::SymbolInstance(instance(
        9,
        Affine::scale_non_uniform(2.0, 1.0),
        None,
    )));
    let tall = doc.registry.insert(ObjectData::SymbolInstance(instance(
        9,
        Affine::translate((20.0, 0.0)) * Affine::scale_non_uniform(1.0, 2.0),
        None,
    )));
    doc.pages
        .push(page(28.0, 16.0, vec![layer(vec![cell(vec![wide, tall])])]));

    let render = render(&doc, 0, 0).expect("render");
    // Wide: corners 1:1, the center band stretches x 2..14. A plain
    // 2× scale would paint the right (BLUE) column from x=12 — the
    // GREEN at x=13 pins nine-slice behavior.
    assert_eq!(pixel(&render, 1, 1), [255, 0, 0, 255], "red corner");
    assert_eq!(pixel(&render, 8, 1), [255, 0, 255, 255], "magenta top edge");
    assert_eq!(pixel(&render, 15, 1), PIXEL_BLUE, "blue corner");
    assert_eq!(pixel(&render, 4, 4), [0, 255, 0, 255], "green center");
    assert_eq!(pixel(&render, 13, 4), [0, 255, 0, 255], "stretched center");
    assert_eq!(pixel(&render, 14, 4), PIXEL_BLUE, "right column at 14");
    assert_eq!(
        pixel(&render, 8, 7),
        [255, 255, 0, 255],
        "yellow bottom edge"
    );
    assert_eq!(pixel(&render, 15, 7), [0, 0, 0, 255], "black corner");
    // Tall: columns 1:1, the center band stretches y 2..14. Plain
    // scale(1,2) would sample the bottom band at y=13 — GREEN there
    // pins the edge stretch.
    assert_eq!(pixel(&render, 21, 1), PIXEL_RED);
    assert_eq!(pixel(&render, 27, 1), PIXEL_BLUE);
    assert_eq!(pixel(&render, 24, 8), [0, 255, 0, 255], "center");
    assert_eq!(pixel(&render, 23, 13), [0, 255, 0, 255], "stretched center");
    assert_eq!(pixel(&render, 21, 15), [0, 255, 255, 255], "cyan corner");
    assert_eq!(pixel(&render, 24, 15), [255, 255, 0, 255], "yellow edge");
    assert_eq!(pixel(&render, 27, 15), [0, 0, 0, 255], "black corner");
}

#[test]
fn nine_slice_instance_smaller_than_its_corners_shrinks_corners() {
    let mut doc = Document::default();
    // The same nine-piece 8x8 source as the two-size proof above:
    // corners RED/BLUE/CYAN/BLACK, center GREEN.
    let pieces: [([f64; 4], i64); 9] = [
        ([0.0, 0.0, 2.0, 2.0], RED),
        ([2.0, 0.0, 6.0, 2.0], MAGENTA),
        ([6.0, 0.0, 8.0, 2.0], BLUE),
        ([0.0, 2.0, 2.0, 6.0], RED),
        ([2.0, 2.0, 6.0, 6.0], GREEN),
        ([6.0, 2.0, 8.0, 6.0], BLUE),
        ([0.0, 6.0, 2.0, 8.0], CYAN),
        ([2.0, 6.0, 6.0, 8.0], YELLOW),
        ([6.0, 6.0, 8.0, 8.0], BLACK),
    ];
    let content: Vec<doc::ObjectId> = pieces
        .iter()
        .map(|([x0, y0, x1, y1], fill)| {
            rect_object(
                &mut doc.registry,
                rect_contour(*x0, *y0, *x1, *y1),
                *fill,
                1000,
                0,
            )
        })
        .collect();
    let mut definition = symbol_definition(9, SymbolType::Graphic, vec![cell(content)]);
    definition.nine_slice = Some(NineSliceGuides {
        locked: false,
        color_raw: 0,
        controls: [2.0, 2.0, 2.0, 2.0],
        shape_bounds: Rect::new(0.0, 0.0, 8.0, 8.0),
        limits: Rect::new(-2.0, -2.0, 10.0, 10.0),
    });
    doc.symbols.push(definition);

    // Destination 3x2: smaller than the 2+2 corner insets on both
    // axes. The corners shrink proportionally (2 -> 1), the center
    // pieces collapse to zero width/height, and nothing goes negative
    // - the output is four shrunken corners and no center.
    let tiny = doc.registry.insert(ObjectData::SymbolInstance(instance(
        9,
        Affine::scale_non_uniform(3.0 / 8.0, 2.0 / 8.0),
        None,
    )));
    // The page canvas hosts the 8x8 source raster; the *destination*
    // is what shrinks to 3x2.
    doc.pages
        .push(page(8.0, 8.0, vec![layer(vec![cell(vec![tiny])])]));

    let render = render(&doc, 0, 0).expect("render");
    // Corner widths resolve to 1 (left) and 2 (right), heights 1 and
    // 1; the middle pieces are zero-sized and paint nothing.
    assert_eq!(
        pixel(&render, 0, 0),
        [255, 0, 0, 255],
        "shrunken red corner"
    );
    assert_eq!(
        pixel(&render, 1, 0),
        PIXEL_BLUE,
        "blue corner fills the rest"
    );
    assert_eq!(pixel(&render, 2, 0), PIXEL_BLUE, "blue corner is 2 wide");
    assert_eq!(
        pixel(&render, 0, 1),
        [0, 255, 255, 255],
        "shrunken cyan corner"
    );
    assert_eq!(
        pixel(&render, 1, 1),
        [0, 0, 0, 255],
        "black corner fills the rest"
    );
    assert_eq!(
        pixel(&render, 2, 1),
        [0, 0, 0, 255],
        "black corner is 2 wide"
    );
}

#[test]
fn animation_instance_spans_states_with_shifted_placements() {
    let (mut doc, placement) = red_dot_document();
    let Some(ObjectData::SymbolInstance(animated)) = doc.registry.get_mut(placement) else {
        panic!("inserted a symbol instance");
    };
    animated.animation = Some(SymbolAnimation {
        frames: 3,
        x_shift: 4.0,
        y_shift: 0.0,
        scaling: 100.0,
        rotation: None,
        start_opacity: None,
        end_opacity: None,
    });
    // Four states; the span covers the first three.
    doc.pages.push(page(
        8.0,
        4.0,
        vec![layer(vec![
            cell(vec![placement]),
            cell(vec![]),
            cell(vec![]),
            cell(vec![]),
        ])],
    ));

    // t = step/2 over x_shift 4: states land at x offsets 0, 2, 4,
    // and the span ends — state 3 paints nothing.
    let frame0 = render(&doc, 0, 0).expect("frame 0");
    assert_eq!(pixel(&frame0, 0, 1), PIXEL_RED);
    assert_eq!(pixel(&frame0, 4, 1), CLEAR);
    let frame1 = render(&doc, 0, 1).expect("frame 1");
    assert_eq!(pixel(&frame1, 1, 1), CLEAR);
    assert_eq!(pixel(&frame1, 2, 1), PIXEL_RED);
    assert_eq!(pixel(&frame1, 5, 1), PIXEL_RED);
    assert_eq!(pixel(&frame1, 6, 1), CLEAR);
    let frame2 = render(&doc, 0, 2).expect("frame 2");
    assert_eq!(pixel(&frame2, 3, 1), CLEAR);
    assert_eq!(pixel(&frame2, 4, 1), PIXEL_RED);
    assert_eq!(pixel(&frame2, 7, 1), PIXEL_RED);
    let frame3 = render(&doc, 0, 3).expect("frame 3");
    assert_eq!(pixel(&frame3, 0, 1), CLEAR);
    assert_eq!(pixel(&frame3, 7, 1), CLEAR);
}

#[test]
fn button_instance_shows_definition_state_per_frame() {
    let mut doc = Document::default();
    let up = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        RED,
        1000,
        0,
    );
    let over = rect_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        BLUE,
        1000,
        0,
    );
    doc.symbols.push(symbol_definition(
        3,
        SymbolType::Button,
        vec![cell(vec![up]), cell(vec![over])],
    ));
    let placement = doc.registry.insert(ObjectData::SymbolInstance(instance(
        3,
        Affine::IDENTITY,
        None,
    )));
    doc.pages.push(page(
        8.0,
        4.0,
        vec![layer(vec![cell(vec![placement]), cell(vec![])])],
    ));

    // State 0 shows the Up art; state 1 shows Over even though the
    // placement cell only names state 0.
    let frame0 = render(&doc, 0, 0).expect("frame 0");
    assert_eq!(pixel(&frame0, 1, 1), PIXEL_RED);
    let frame1 = render(&doc, 0, 1).expect("frame 1");
    assert_eq!(pixel(&frame1, 1, 1), PIXEL_BLUE);
}

#[test]
fn dangling_instance_is_a_typed_refusal() {
    let mut doc = Document::default();
    let placement = doc.registry.insert(ObjectData::SymbolInstance(instance(
        42,
        Affine::IDENTITY,
        None,
    )));
    doc.pages
        .push(page(8.0, 4.0, vec![layer(vec![cell(vec![placement])])]));
    assert_eq!(
        render(&doc, 0, 0),
        Err(ComposeError::MissingDefinition { symbol: 42 })
    );
}

#[test]
fn self_referential_definition_is_a_typed_refusal() {
    let mut doc = Document::default();
    let inner = doc.registry.insert(ObjectData::SymbolInstance(instance(
        7,
        Affine::IDENTITY,
        None,
    )));
    doc.symbols.push(symbol_definition(
        7,
        SymbolType::Graphic,
        vec![cell(vec![inner])],
    ));
    let placement = doc.registry.insert(ObjectData::SymbolInstance(instance(
        7,
        Affine::IDENTITY,
        None,
    )));
    doc.pages
        .push(page(8.0, 4.0, vec![layer(vec![cell(vec![placement])])]));
    assert_eq!(
        render(&doc, 0, 0),
        Err(ComposeError::SymbolCycle { symbol: 7 })
    );
}

#[test]
fn nesting_past_the_depth_ceiling_is_a_typed_refusal() {
    let mut doc = Document::default();
    // 33 definitions in a chain: k instances k+1. Entering the 33rd
    // exceeds the ceiling without any repeated symbol number.
    for number in 0..=32i64 {
        let next = doc.registry.insert(ObjectData::SymbolInstance(instance(
            number + 1,
            Affine::IDENTITY,
            None,
        )));
        doc.symbols.push(symbol_definition(
            number,
            SymbolType::Graphic,
            vec![cell(vec![next])],
        ));
    }
    let placement = doc.registry.insert(ObjectData::SymbolInstance(instance(
        0,
        Affine::IDENTITY,
        None,
    )));
    doc.pages
        .push(page(8.0, 4.0, vec![layer(vec![cell(vec![placement])])]));
    assert_eq!(
        render(&doc, 0, 0),
        Err(ComposeError::SymbolDepth {
            symbol: 32,
            depth: 32,
        })
    );
}

// --- M15: gradient fills --------------------------------------------------

use doc::{Dither, Fill, FillShape, Gradient, GradientStop};

/// A path painting a modeled gradient through `fill` and the fill
/// vector handles.
fn gradient_object(
    registry: &mut Registry,
    contour: Contour,
    fill: Fill,
    start: (f64, f64),
    end: (f64, f64),
) -> doc::ObjectId {
    registry.insert(ObjectData::Path(PathObject {
        contours: vec![contour],
        primitive: None,
        even_odd: false,
        paint: Some(PaintStyle {
            stroke_raw: 0,
            fill_raw: BLACK,
            fill: Some(fill),
        }),
        fill_start: Some(Point::new(start.0, start.1)),
        fill_end: Some(Point::new(end.0, end.1)),
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

/// An opaque-black → opaque-white linear ramp with stops at 0 and 1.
fn black_white_fill(shape: FillShape) -> Fill {
    Fill {
        shape,
        shape_extended: true,
        texture_blend: 0,
        feather: 0,
        stamping: Some(1),
        hard_edge: false,
        gradient: Some(Gradient {
            vertices: Vec::new(),
            rgb: vec![
                GradientStop {
                    position: 0.0,
                    color_raw: BLACK,
                },
                GradientStop {
                    position: 1.0,
                    color_raw: WHITE,
                },
            ],
            alpha: Vec::new(),
        }),
        dither: None,
    }
}

#[test]
fn a_linear_gradient_runs_black_to_white_along_the_fill_vector() {
    let mut doc = Document::default();
    let gradient = gradient_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        black_white_fill(FillShape::Linear),
        (0.0, 2.0),
        (4.0, 2.0),
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![gradient])])]));
    let render = render(&doc, 0, 0).expect("render");

    // Pixel centers sample the ramp at x = 0.125, 0.375, 0.625, 0.875
    // of the 0..4 span — 31, 95, 159, 223 exactly.
    let columns: Vec<[u8; 4]> = (0..4).map(|x| pixel(&render, x, 2)).collect();
    for (i, pixel_rgba) in columns.iter().enumerate() {
        assert_eq!(
            pixel_rgba[0], pixel_rgba[1],
            "column {i}: a grey ramp moves all channels together"
        );
        assert_eq!(pixel_rgba[1], pixel_rgba[2], "column {i}");
    }
    let grey: Vec<u8> = columns.iter().map(|rgba| rgba[0]).collect();
    assert_eq!(grey, [31, 95, 159, 223], "grey ramp by pixel center");
}

#[test]
fn a_radial_gradient_runs_black_at_the_center_to_white_at_the_rim() {
    let mut doc = Document::default();
    let gradient = gradient_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        black_white_fill(FillShape::Radial),
        (2.0, 2.0),
        (2.0, 6.0),
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![gradient])])]));
    let render = render(&doc, 0, 0).expect("render");

    // Radius |start→end| = 4 from (2, 2). Pixel centers: (1,1) sits
    // √0.5 out → position 0.177 → 45; the (0,0) corner center (0.5,
    // 0.5) sits √4.5 ≈ 2.121 out → position 0.530 → 135.
    let center = pixel(&render, 1, 1);
    let corner = pixel(&render, 0, 0);
    assert_eq!(center[0], 45, "√0.5 of radius 4 into a black-white ramp");
    assert_eq!(corner[0], 135, "√4.5 of radius 4 into the ramp");
}

#[test]
fn gradient_shapes_without_render_math_fall_back_to_the_solid_colour() {
    // The substitution ledger: satin, pinch, folds, bars, ripple,
    // waves, contour, rectangular, web dither and pattern all paint
    // the FCLi colour until their math is pinned.
    for shape in [
        FillShape::Satin,
        FillShape::Pinch,
        FillShape::Folds,
        FillShape::Bars,
        FillShape::Ripple,
        FillShape::Waves,
        FillShape::Contour,
        FillShape::Rectangular,
        FillShape::WebDither,
        FillShape::Pattern,
    ] {
        let mut doc = Document::default();
        let mut fill = black_white_fill(shape);
        fill.dither = Some(Dither {
            colors: [None; 4],
            transparent: None,
        });
        let gradient = gradient_object(
            &mut doc.registry,
            rect_contour(0.0, 0.0, 4.0, 4.0),
            fill,
            (0.0, 2.0),
            (4.0, 2.0),
        );
        doc.pages
            .push(page(4.0, 4.0, vec![layer(vec![cell(vec![gradient])])]));
        let rendered = render(&doc, 0, 0).expect("render");
        // Premultiplied opaque black.
        assert_solid(&rendered, [0, 0, 0, 255]);
    }
}

#[test]
fn an_alpha_ramp_sets_coverage_across_the_gradient() {
    let mut doc = Document::default();
    let mut fill = black_white_fill(FillShape::Linear);
    if let Some(gradient) = &mut fill.gradient {
        gradient.alpha = vec![
            GradientStop {
                position: 0.0,
                color_raw: 0x00_00_00_00,
            },
            GradientStop {
                position: 1.0,
                color_raw: 0xFF_00_00_00,
            },
        ];
    }
    let gradient = gradient_object(
        &mut doc.registry,
        rect_contour(0.0, 0.0, 4.0, 4.0),
        fill,
        (0.0, 2.0),
        (4.0, 2.0),
    );
    doc.pages
        .push(page(4.0, 4.0, vec![layer(vec![cell(vec![gradient])])]));
    let rendered = render(&doc, 0, 0).expect("render");

    // Over the transparent canvas the coverage ramp (0 → 255) is the
    // visible thing. The target is premultiplied: at x=0.125 the ramp
    // samples grey 31 at coverage 31/255 → premultiplied 31×31/255 =
    // 3.8 → 4; at x=0.875 grey 223 at 223/255 → 223×223/255 = 195.1 →
    // 195. Colour and coverage ride the same position — the
    // unpremultiplied stop interpolation keeps the grey hue (the
    // premultiplied default would lerp the black end stop toward
    // nothing and read (0,0,0,31)).
    let left = pixel(&rendered, 0, 2);
    let right = pixel(&rendered, 3, 2);
    assert_eq!(left[3], 31, "the alpha ramp sets coverage: {left:?}");
    assert_eq!(left[0], 4, "the dark end rides the ramp: {left:?}");
    assert_eq!(right[3], 223, "coverage rides the same ramp");
    assert_eq!(right[0], 195, "the bright end rides it too: {right:?}");
}
