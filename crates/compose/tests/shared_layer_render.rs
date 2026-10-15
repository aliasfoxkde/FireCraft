//! Shared-layer link resolution (decision 0060): a page's link layer
//! (`LSAv`) renders the shared definition's content (`LSEv`, decision
//! 0022) in the link's z-position. The definition's own display flags
//! gate; a dangling uid draws nothing; symbol-definition walks keep
//! links inert. Every expected value is hand-derived premultiplied
//! RGBA8.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compose::{Render, render};
use doc::{
    Canvas, Cell, Document, Layer, LayerList, ObjectData, Page, PageInfo, PaintStyle, PathObject,
    PathPoint, SharedLayerDef, SharedLayerLink,
};
use geom::Point;

// --- document builders ---------------------------------------------------

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

fn layer(name: &str, cells: Vec<Cell>, shared_link: Option<SharedLayerLink>) -> Layer {
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
        shared_link,
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

/// The shared definition: one layer, one cell, one red square.
fn document_with(link: Option<SharedLayerLink>) -> Document {
    let mut document = Document::default();
    let id = filled_square(&mut document.registry, 8.0, RED_RAW);
    document.shared_layer_defs = vec![SharedLayerDef {
        uid: "shared-red".to_owned(),
        is_primary: true,
        layer: layer("shared", vec![cell(vec![id])], None),
    }];
    // Page: an empty base layer, then the link.
    document.pages = vec![page(vec![
        layer("base", vec![cell(Vec::new())], None),
        layer("link", vec![cell(Vec::new())], link),
    ])];
    document
}

// --- tests ---------------------------------------------------------------

#[test]
fn a_link_layer_renders_the_shared_content() {
    let document = document_with(Some(SharedLayerLink {
        uid: "shared-red".to_owned(),
        is_primary: false,
    }));
    let rendered = render(&document, 0, 0).expect("renders");
    assert_eq!(pixel(&rendered, 5, 5), [255, 0, 0, 255], "shared red shows");
    assert_eq!(
        pixel(&rendered, 1, 1),
        [255, 255, 255, 255],
        "outside the square stays matte"
    );
}

#[test]
fn the_link_layer_flags_gate_the_resolution() {
    let mut document = document_with(Some(SharedLayerLink {
        uid: "shared-red".to_owned(),
        is_primary: false,
    }));
    // Hiding the page's link layer hides the shared content there.
    document.pages[0].layers.layers[1].display = false;
    let rendered = render(&document, 0, 0).expect("renders");
    assert_eq!(
        pixel(&rendered, 5, 5),
        [255, 255, 255, 255],
        "hidden link draws nothing"
    );

    // Hiding the definition's own layer hides it everywhere.
    document.pages[0].layers.layers[1].display = true;
    document.shared_layer_defs[0].layer.display = false;
    let rendered = render(&document, 0, 0).expect("renders");
    assert_eq!(
        pixel(&rendered, 5, 5),
        [255, 255, 255, 255],
        "hidden def draws nothing"
    );
}

#[test]
fn a_dangling_uid_draws_nothing_and_a_nested_link_stays_inert() {
    let mut document = document_with(Some(SharedLayerLink {
        uid: "missing".to_owned(),
        is_primary: false,
    }));
    let rendered = render(&document, 0, 0).expect("dangling uid is not an error");
    assert_eq!(pixel(&rendered, 5, 5), [255, 255, 255, 255]);

    // A definition whose own body carries a link back to itself stays
    // inert — no cycle, content renders exactly once.
    document.shared_layer_defs[0].uid = "self".to_owned();
    document.shared_layer_defs[0].layer.shared_link = Some(SharedLayerLink {
        uid: "self".to_owned(),
        is_primary: false,
    });
    document.pages[0].layers.layers[1].shared_link = Some(SharedLayerLink {
        uid: "self".to_owned(),
        is_primary: false,
    });
    let rendered = render(&document, 0, 0).expect("self-link does not cycle");
    assert_eq!(pixel(&rendered, 5, 5), [255, 0, 0, 255]);
}

#[test]
fn shared_content_keeps_its_z_position_and_merges_with_page_content() {
    let mut document = document_with(Some(SharedLayerLink {
        uid: "shared-red".to_owned(),
        is_primary: false,
    }));
    // A third layer above the link paints the same square green: the
    // opaque green covers at (5,5), proving the shared content
    // rendered at the link's z-position (below it) — bottom-first.
    let green = filled_square(&mut document.registry, 8.0, GREEN_RAW);
    document.pages[0]
        .layers
        .layers
        .push(layer("above", vec![cell(vec![green])], None));
    let rendered = render(&document, 0, 0).expect("renders");
    assert_eq!(
        pixel(&rendered, 5, 5),
        [0, 255, 0, 255],
        "the top layer covers the shared content"
    );
}
