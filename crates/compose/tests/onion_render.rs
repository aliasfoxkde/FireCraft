//! Onion-skin compositing (decision 0081): the states adjacent to the
//! current one show through as flat 50% ghosts behind it, and the
//! current state's own opaque pixels are untouched by them. Every
//! expected value is hand-derived premultiplied RGBA8.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compose::{Render, render, render_onion};
use doc::{
    Canvas, Cell, Contour, Document, Layer, LayerList, ObjectData, Page, PageInfo, PaintStyle,
    PathObject, PathPoint,
};
use geom::Point;

// --- document builders ---------------------------------------------------

/// A page of `states` cells on one layer, with the given onion
/// settings (`OSBi` / `OSAi`).
fn page(states: Vec<Cell>, onion_before: i64, onion_after: i64) -> Page {
    Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: states.len() as i64,
            frame_count_duplicate: states.len() as i64,
            request_id: 0,
            canvas: Canvas {
                x: 0.0,
                y: 0.0,
                width: 16.0,
                height: 16.0,
                resolution: 72.0,
                resolution_unit: 0,
                background_raw: 0xFF_FF_FF,
            },
            onion_before,
            onion_after,
            frame_durations: Vec::new(),
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList {
            layers: vec![Layer {
                name: "states".to_owned(),
                is_background: false,
                has_url: false,
                is_special: false,
                frame_changing: true,
                display: true,
                nesting_level: None,
                cells: states,
                visibility_per_frame: None,
                shared_link: None,
            }],
        },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    }
}

fn cell(ids: Vec<doc::ObjectId>) -> Cell {
    Cell {
        locked: false,
        visible: true,
        elements: ids,
    }
}

/// A solid-filled rectangle whose corners are `(x, y)` and
/// `(x + width, y + height)`.
fn filled_rect(
    registry: &mut doc::Registry,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    raw: i64,
) -> doc::ObjectId {
    registry.insert(ObjectData::Path(PathObject {
        contours: vec![Contour {
            closed: true,
            point_count: 4,
            points: [
                (x, y),
                (x + width, y),
                (x + width, y + height),
                (x, y + height),
            ]
            .into_iter()
            .map(|(px, py)| PathPoint {
                anchor: Point::new(px, py),
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
            fill_raw: raw,
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
    }))
}

/// Opaque red ABGR (`FCLi` layout: alpha in the top byte).
const RED_RAW: i64 = 0xFF_00_00_FF;
/// Opaque green.
const GREEN_RAW: i64 = 0xFF_00_FF_00;

fn pixel(render: &Render, x: u32, y: u32) -> [u8; 4] {
    let stride = u32::from(render.width) * 4;
    let at = (y * stride + x * 4) as usize;
    render.pixels[at..at + 4].try_into().expect("four channels")
}

/// Three states: a wide red band in state 0, a green square inside
/// that band in state 1, and an empty state 2.
fn document_with_states(onion_before: i64, onion_after: i64) -> Document {
    let mut document = Document::default();
    let red = filled_rect(&mut document.registry, 2.0, 2.0, 12.0, 4.0, RED_RAW);
    let green = filled_rect(&mut document.registry, 10.0, 2.0, 4.0, 4.0, GREEN_RAW);
    document.pages = vec![page(
        vec![cell(vec![red]), cell(vec![green]), cell(Vec::new())],
        onion_before,
        onion_after,
    )];
    document
}

// --- tests ---------------------------------------------------------------

#[test]
fn a_neighbouring_state_shows_through_as_a_ghost() {
    let document = document_with_states(1, 1);
    let onion = render_onion(&document, 0, 1, 1.0).expect("onion render");
    // State 0's red band lies outside the current state's green square,
    // so the current state is transparent there and the ghost shows:
    // red at half opacity over the white matte is 127 + 128 for the red
    // channel and 0 + 128 for the other two.
    assert_eq!(
        pixel(&onion, 3, 3),
        [255, 128, 128, 255],
        "the neighbouring state is ghosted where the current state is transparent"
    );
}

#[test]
fn the_current_state_is_unmodified_where_it_is_opaque() {
    let document = document_with_states(1, 1);
    let onion = render_onion(&document, 0, 1, 1.0).expect("onion render");
    let plain = render(&document, 0, 1).expect("plain render");
    // The ghost band covers the green square too, but the current state
    // is composited last at its own alpha: its pixels are the plain
    // render's, untouched.
    assert_eq!(
        pixel(&onion, 11, 3),
        [0, 255, 0, 255],
        "the current state stays opaque over the ghost"
    );
    assert_eq!(
        pixel(&onion, 11, 3),
        pixel(&plain, 11, 3),
        "the current state's pixels match the plain render"
    );
}

#[test]
fn the_matte_shows_where_neither_state_paints() {
    let document = document_with_states(1, 1);
    let onion = render_onion(&document, 0, 1, 1.0).expect("onion render");
    assert_eq!(
        pixel(&onion, 1, 1),
        [255, 255, 255, 255],
        "the canvas matte shows through"
    );
}

#[test]
fn without_onion_skinning_the_render_matches_the_plain_one() {
    let document = document_with_states(0, 0);
    let onion = render_onion(&document, 0, 1, 1.0).expect("onion render");
    let plain = render(&document, 0, 1).expect("plain render");
    for (x, y) in [(1_u32, 1_u32), (3, 3), (11, 3)] {
        assert_eq!(
            pixel(&onion, x, y),
            pixel(&plain, x, y),
            "no ghost, no difference at ({x}, {y})"
        );
    }
}

#[test]
fn the_ghost_set_follows_the_page_settings() {
    // Only the history ghosts: state 0's red band ghosts in, and the
    // current state still covers its own pixels.
    let document = document_with_states(1, 0);
    let onion = render_onion(&document, 0, 1, 1.0).expect("onion render");
    assert_eq!(
        pixel(&onion, 3, 3),
        [255, 128, 128, 255],
        "the state before the current one ghosts"
    );
    assert_eq!(
        pixel(&onion, 11, 3),
        [0, 255, 0, 255],
        "the current state still wins"
    );
}

#[test]
fn a_page_out_of_range_is_refused() {
    let document = document_with_states(1, 1);
    let error = render_onion(&document, 9, 0, 1.0).expect_err("page 9 does not exist");
    assert!(
        matches!(error, compose::ComposeError::PageOutOfBounds { page: 9 }),
        "unexpected error: {error}"
    );
}
