//! The master page's layers stand behind every other page's
//! (decision 0115): CS6's master page is the document's shared base.
//! The base walks first, so the page's own content paints over it;
//! the master page renders its own layers once.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compose::{Render, render};
use doc::{
    Canvas, Cell, Document, Layer, LayerList, ObjectData, Page, PageInfo, PaintStyle, PathObject,
    PathPoint,
};
use geom::Point;

fn page(layers: Vec<Layer>) -> Page {
    Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: 1,
            frame_count_duplicate: 1,
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

fn layer(name: &str, cells: Vec<Cell>) -> Layer {
    Layer {
        name: name.to_owned(),
        is_background: false,
        has_url: false,
        is_special: false,
        frame_changing: false,
        display: true,
        nesting_level: None,
        cells,
        visibility_per_frame: None,
        shared_link: None,
    }
}

fn cell(ids: Vec<doc::ObjectId>) -> Cell {
    Cell {
        locked: false,
        visible: true,
        elements: ids,
    }
}

/// A `side`×`side` square at (2,2), solid-filled `raw`.
fn filled_square(registry: &mut doc::Registry, side: f64, raw: i64) -> doc::ObjectId {
    registry.insert(ObjectData::Path(PathObject {
        contours: vec![doc::Contour {
            closed: true,
            point_count: 4,
            points: [
                (2.0, 2.0),
                (2.0 + side, 2.0),
                (2.0 + side, 2.0 + side),
                (2.0, 2.0 + side),
            ]
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

/// A master page carrying one red square, and an ordinary page.
fn document_with_master() -> Document {
    let mut document = Document::default();
    let red = filled_square(&mut document.registry, 8.0, RED_RAW);
    document.pages = vec![page(vec![layer("master", vec![cell(vec![red])])])];
    document.pages[0].is_master = true;
    document
        .pages
        .push(page(vec![layer("base", vec![cell(Vec::new())])]));
    document
}

#[test]
fn the_master_pages_layers_stand_behind_every_other_page() {
    let document = document_with_master();
    let rendered = render(&document, 1, 0).expect("renders");
    assert_eq!(
        pixel(&rendered, 5, 5),
        [255, 0, 0, 255],
        "the master's content shows on the other page"
    );
}

#[test]
fn the_master_page_renders_its_own_layers_once() {
    let document = document_with_master();
    let rendered = render(&document, 0, 0).expect("renders");
    assert_eq!(
        pixel(&rendered, 5, 5),
        [255, 0, 0, 255],
        "the master page draws its own content"
    );
}

#[test]
fn the_pages_own_content_paints_over_the_master() {
    let mut document = document_with_master();
    let green = filled_square(&mut document.registry, 8.0, GREEN_RAW);
    document.pages[1].layers.layers[0].cells = vec![cell(vec![green])];
    let rendered = render(&document, 1, 0).expect("renders");
    assert_eq!(
        pixel(&rendered, 5, 5),
        [0, 255, 0, 255],
        "the base stands in front of the master"
    );
}

#[test]
fn a_document_without_a_master_renders_only_its_own_layers() {
    let mut document = document_with_master();
    document.pages[0].is_master = false;
    let rendered = render(&document, 1, 0).expect("renders");
    assert_eq!(
        pixel(&rendered, 5, 5),
        [255, 255, 255, 255],
        "no master, no shared base"
    );
}
