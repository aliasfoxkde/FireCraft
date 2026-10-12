//! Web export (M11 task 3 stopping condition): a sliced document
//! exports HTML+images whose pixels are exactly the composer's; the
//! shim-table structure, hotspot maps and behavior hooks follow the
//! SLICES.XTT behavioral spec; runs are byte-deterministic.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use raster::{TILE_BYTES, TILE_SIDE, Tile};
use webexport::{WebExportError, WebExportOptions, Workflow, export};

/// Opaque red, straight alpha.
const RED: [u8; 4] = [255, 0, 0, 255];
/// Opaque blue.
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// An 8x4 canvas split into left/right halves of solid ink, one web
/// slice per half.
fn sliced_document() -> doc::Document {
    let mut document = doc::Document::default();
    document.info.canvas.width = 8.0;
    document.info.canvas.height = 4.0;
    document.info.frame_count = 1;

    // Ink: red left half, blue right half, as one tile image.
    let mut tile_bytes = [0u8; TILE_BYTES];
    for row in 0..4usize {
        for col in 0..8usize {
            let pixel = if col < 4 { RED } else { BLUE };
            let at = (row * TILE_SIDE + col) * 4;
            tile_bytes[at..at + 4].copy_from_slice(&pixel);
        }
    }
    document.tiles.insert(1, Tile::from_rgba8(tile_bytes));
    let image = document
        .registry
        .insert(doc::ObjectData::Image(doc::ImageObject {
            location: geom::Point::new(0.0, 0.0),
            offset: geom::Point::new(0.0, 0.0),
            pixels: (8, 4),
            tile_size: TILE_SIDE as i64,
            tiles: vec![doc::TileRef::Tile(1)],
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }));

    let web_slice = |name: &str, x0: f64, x1: f64| {
        doc::ObjectData::Slice(doc::SliceObject {
            frame: geom::Rect::new(x0, 0.0, x1, 4.0),
            name: Some(name.to_owned()),
            slice_type: Some(0),
            ..doc::SliceObject::default()
        })
    };
    let left = document.registry.insert(web_slice("left", 0.0, 4.0));
    let right = document.registry.insert(web_slice("right", 4.0, 8.0));

    document.pages.push(doc::Page {
        info: doc::PageInfo {
            version: None,
            revision: None,
            frame_count: 1,
            frame_count_duplicate: 1,
            request_id: 0,
            canvas: document.info.canvas.clone(),
            onion_before: 0,
            onion_after: 0,
            frame_durations: vec![10],
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: doc::LayerList {
            layers: vec![
                doc::Layer {
                    name: "Layer 1".to_owned(),
                    display: true,
                    cells: vec![doc::Cell {
                        locked: false,
                        visible: true,
                        elements: vec![image],
                    }],
                    ..doc::Layer::default()
                },
                doc::Layer {
                    name: "Web Layer".to_owned(),
                    has_url: true,
                    is_special: true,
                    display: true,
                    cells: vec![doc::Cell {
                        locked: false,
                        visible: true,
                        elements: vec![left, right],
                    }],
                    ..doc::Layer::default()
                },
            ],
        },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    });
    document
}

/// The composer's straight-alpha screen for state 0.
fn composer_pixels(document: &doc::Document) -> Vec<u8> {
    let rendered = compose::render(document, 0, 0).unwrap();
    compose::straight_alpha(&rendered.pixels)
}

/// Decodes an exported PNG back to straight RGBA.
fn png_pixels(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoded = io::raster::decode(bytes).unwrap();
    (decoded.width, decoded.height, decoded.rgba)
}

#[test]
fn slice_regions_carry_the_composers_pixels() {
    let document = sliced_document();
    let options = WebExportOptions {
        doc_name: "doc".to_owned(),
        ..WebExportOptions::default()
    };
    let exported = export(&document, &options, Workflow::ImagesOnly).unwrap();
    let screen = composer_pixels(&document);

    let left_file = &exported.files[0];
    assert_eq!(left_file.name, "left.png");
    let left_bytes = &left_file.bytes;
    let (w, h, rgba) = png_pixels(left_bytes);
    assert_eq!((w, h), (4, 4));
    // Left half: four red rows.
    for row in 0..4usize {
        let at = (row * w as usize) * 4;
        assert_eq!(&rgba[at..at + 4], &RED);
    }
    // The exported top-left pixel is the screen's top-left pixel.
    assert_eq!(&screen[0..4], &rgba[0..4]);

    let right_file = &exported.files[1];
    assert_eq!(right_file.name, "right.png");
    let (w, h, rgba) = png_pixels(&right_file.bytes);
    assert_eq!((w, h), (4, 4));
    for row in 0..4usize {
        let at = (row * w as usize) * 4;
        assert_eq!(&rgba[at..at + 4], &BLUE);
    }
}

#[test]
fn html_and_images_follows_the_shim_table() {
    let document = sliced_document();
    let options = WebExportOptions::default();
    let exported = export(&document, &options, Workflow::HtmlAndImages).unwrap();
    let html = exported.html.as_deref().unwrap();

    assert!(html.starts_with("<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.01 Transitional//EN\""));
    assert!(html.contains("<title>doc</title>"));
    assert!(html.contains("<table border=\"0\" cellpadding=\"0\" cellspacing=\"0\" width=\"8\">"));
    // The shim edges.
    assert!(html.contains("src=\"shim.gif\" width=\"4\" height=\"1\""));
    assert!(html.contains("src=\"shim.gif\" width=\"1\" height=\"4\""));
    // The body cells carry name/src/size and the composer's files.
    assert!(
        html.contains("<img name=\"left\" src=\"left.png\" width=\"4\" height=\"4\" alt=\"\">")
    );
    assert!(
        html.contains("<img name=\"right\" src=\"right.png\" width=\"4\" height=\"4\" alt=\"\">")
    );
    // The shim file rides last, byte-stable.
    let shim = exported.files.last().unwrap();
    assert_eq!(shim.name, "shim.gif");
    assert_eq!(&shim.bytes[..6], b"GIF89a");
}

#[test]
fn hotspots_anchor_to_the_piece_under_their_center() {
    let mut document = sliced_document();
    let hotspot = document
        .registry
        .insert(doc::ObjectData::Slice(doc::SliceObject {
            // Center at (2, 2): the left piece.
            frame: geom::Rect::new(1.0, 1.0, 3.0, 3.0),
            slice_type: None,
            link: Some("https://example.test/".to_owned()),
            alt: Some("go".to_owned()),
            ..doc::SliceObject::default()
        }));
    document.pages[0].layers.layers[1].cells[0]
        .elements
        .push(hotspot);

    let exported = export(
        &document,
        &WebExportOptions::default(),
        Workflow::HtmlAndImages,
    )
    .unwrap();
    let html = exported.html.as_deref().unwrap();

    // The host image gains the map reference; the map carries the
    // area, coordinates relative to the host piece's origin.
    assert!(html.contains(
        "<img name=\"left\" src=\"left.png\" width=\"4\" height=\"4\" usemap=\"#m_left\""
    ));
    assert!(html.contains("<map name=\"m_left\">"));
    assert!(html.contains(
        "<area shape=\"rect\" coords=\"1,1,3,3\" href=\"https://example.test/\" title=\"go\" alt=\"go\">"
    ));
    // No runtime: the hotspot carries no behaviors.
    assert!(!html.contains("function MM_"));
}

#[test]
fn behaviors_emit_hooks_and_the_runtime() {
    let mut document = sliced_document();
    let button = document
        .registry
        .insert(doc::ObjectData::Slice(doc::SliceObject {
            frame: geom::Rect::new(4.0, 0.0, 8.0, 4.0),
            name: Some("right".to_owned()),
            slice_type: Some(0),
            behaviors: vec![doc::SliceBehavior { kind: 2 }],
            custom_data: {
                let mut map = std::collections::BTreeMap::new();
                map.insert("swapTarget".to_owned(), "swap.png".to_owned());
                map
            },
            ..doc::SliceObject::default()
        }));
    document.pages[0].layers.layers[1].cells[0].elements[1] = button;

    let exported = export(
        &document,
        &WebExportOptions::default(),
        Workflow::HtmlAndImages,
    )
    .unwrap();
    let html = exported.html.as_deref().unwrap();

    assert!(html.contains("function MM_swapImage()"));
    assert!(html.contains("onMouseOver=\"MM_swapImage('right','','swap.png',1);\""));
    assert!(html.contains("onMouseOut=\"MM_swapImgRestore();\""));
    assert!(html.contains("onload=\"MM_preloadImages('swap.png')\""));
}

#[test]
fn expv_overrides_choose_the_codec() {
    let mut document = sliced_document();
    let jpeg_half = document
        .registry
        .insert(doc::ObjectData::Slice(doc::SliceObject {
            frame: geom::Rect::new(4.0, 0.0, 8.0, 4.0),
            name: Some("right".to_owned()),
            slice_type: Some(0),
            export: Some(doc::ExportOptions {
                format: Some(1),
                jpeg_quality: Some(80),
                ..doc::ExportOptions::default()
            }),
            ..doc::SliceObject::default()
        }));
    document.pages[0].layers.layers[1].cells[0].elements[1] = jpeg_half;

    let exported = export(
        &document,
        &WebExportOptions::default(),
        Workflow::ImagesOnly,
    )
    .unwrap();
    assert_eq!(exported.files[1].name, "right.jpg");
    assert_eq!(&exported.files[1].bytes[..3], b"\xff\xd8\xff");
}

#[test]
fn export_runs_are_byte_deterministic() {
    let document = sliced_document();
    let options = WebExportOptions::default();
    let first = export(&document, &options, Workflow::HtmlAndImages).unwrap();
    let second = export(&document, &options, Workflow::HtmlAndImages).unwrap();
    assert_eq!(first, second);
}

#[test]
fn out_of_range_selections_are_refused() {
    let document = sliced_document();
    let options = WebExportOptions {
        page: 3,
        ..WebExportOptions::default()
    };
    assert_eq!(
        export(&document, &options, Workflow::ImagesOnly),
        Err(WebExportError::PageOutOfBounds { page: 3 })
    );
    let options = WebExportOptions {
        state: 5,
        ..WebExportOptions::default()
    };
    assert_eq!(
        export(&document, &options, Workflow::ImagesOnly),
        Err(WebExportError::StateOutOfBounds {
            state: 5,
            states: 1
        })
    );
}

#[test]
fn skipped_undefined_areas_produce_no_cell() {
    let document = sliced_document();
    let options = WebExportOptions {
        include_undefined: false,
        ..WebExportOptions::default()
    };
    let exported = export(&document, &options, Workflow::ImagesOnly).unwrap();
    // Every pixel is under a slice here, so the count is unchanged,
    // but the workflow must accept the flag: prove via a doc with a
    // gap — narrow the right slice.
    let mut gapped = sliced_document();
    let right = gapped
        .registry
        .insert(doc::ObjectData::Slice(doc::SliceObject {
            frame: geom::Rect::new(6.0, 0.0, 8.0, 4.0),
            name: Some("right".to_owned()),
            slice_type: Some(0),
            ..doc::SliceObject::default()
        }));
    gapped.pages[0].layers.layers[1].cells[0].elements[1] = right;

    let gapped_export = export(&gapped, &options, Workflow::ImagesOnly).unwrap();
    // Two slices, no filler for the 4..6 band.
    assert_eq!(gapped_export.files.len(), 2);

    let filled = export(&gapped, &WebExportOptions::default(), Workflow::ImagesOnly).unwrap();
    assert_eq!(filled.files.len(), 3);
    let _ = exported;
}

#[test]
fn css_and_images_positions_slices_as_divs() {
    let document = sliced_document();
    let exported = export(
        &document,
        &WebExportOptions::default(),
        Workflow::CssAndImages,
    )
    .unwrap();
    let html = exported.html.as_deref().unwrap();

    assert!(html.starts_with("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\""));
    // The body rule carries the canvas background.
    assert!(html.contains("body {\n\tbackground-color: #ffffff;"));
    // The #main wrap takes the canvas width.
    assert!(html.contains("#main {\n\tmargin: 0 auto;\n\twidth: 8px;\n}"));
    // The csspos absolute-branch order for the left piece.
    assert!(html.contains(
        "#left {\n\tleft: 0px;\n\tposition: absolute;\n\ttop: 0px;\n\tbackground-image: url(left.png);\n\toverflow: hidden;\n\theight: 4px;\n\tmargin-bottom: 0;\n\twidth: 4px;\n}"
    ));
    assert!(html.contains(
        "#right {\n\tleft: 4px;\n\tposition: absolute;\n\ttop: 0px;\n\tbackground-image: url(right.png);\n\toverflow: hidden;\n\theight: 4px;\n\tmargin-bottom: 0;\n\twidth: 4px;\n}"
    ));
    // The div tree references each piece exactly once.
    assert!(html.contains("<div id=\"left\"></div>"));
    assert!(html.contains("<div id=\"right\"></div>"));
    // No shim table in this workflow.
    assert!(!html.contains("shim.gif"));
    // Same image files as Images Only.
    assert_eq!(exported.files.len(), 2);
}

#[test]
fn css_sprites_build_one_sheet_with_offsets() {
    let document = sliced_document();
    let exported = export(
        &document,
        &WebExportOptions::default(),
        Workflow::CssSprites,
    )
    .unwrap();

    // One sheet spanning both pieces' union (the full 8x4 canvas).
    assert_eq!(exported.files.len(), 1);
    let sheet = &exported.files[0];
    assert_eq!(sheet.name, "doc_sprite.png");
    let (w, h, rgba) = png_pixels(&sheet.bytes);
    assert_eq!((w, h), (8, 4));
    // The sheet is the composer's canvas, verbatim.
    assert_eq!(rgba, composer_pixels(&document));

    let css = exported.html.as_deref().unwrap();
    assert!(css.contains("background-image: url(doc_sprite.png);"));
    assert!(
        css.contains(
            "#left {\n\tbackground-position: -0px -0px;\n\twidth: 4px;\n\theight: 4px;\n}"
        )
    );
    assert!(
        css.contains(
            "#right {\n\tbackground-position: -4px -0px;\n\twidth: 4px;\n\theight: 4px;\n}"
        )
    );
}

#[test]
fn css_properties_describes_geometry_fill_and_effects() {
    let mut document = sliced_document();
    let point = |x: f64, y: f64| doc::PathPoint {
        anchor: geom::Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    };
    let text = |raw: &str| doc::EffectValue::Text(raw.to_owned());
    let button = document
        .registry
        .insert(doc::ObjectData::Path(doc::PathObject {
            contours: vec![doc::Contour {
                closed: true,
                point_count: 4,
                points: vec![
                    point(1.0, 0.0),
                    point(3.0, 0.0),
                    point(3.0, 2.0),
                    point(1.0, 2.0),
                ],
            }],
            primitive: Some(doc::PrimitiveSource {
                rect: geom::Rect::new(1.0, 0.0, 3.0, 2.0),
                radius: 2.0,
                transform: geom::Affine::IDENTITY,
                maintain_radius: false,
            }),
            even_odd: false,
            paint: Some(doc::PaintStyle {
                stroke_raw: 0,
                fill_raw: 0x00_00_00_FF, // ABGR: red
            }),
            fill_start: None,
            fill_end: None,
            fill_third: None,
            brush_placement: None,
            fill_on_top: None,
            locked: false,
            visible: true,
            opacity: 500,
            blend: 0,
            name: None,
            effects: vec![doc::Effect {
                category: "Shadow and Glow".to_owned(),
                internal_name: "dropShadow".to_owned(),
                properties: vec![
                    doc::EffectProperty {
                        key: "gaussian_blur_radius".to_owned(),
                        value: text("4"),
                    },
                    doc::EffectProperty {
                        key: "x_offset".to_owned(),
                        value: text("3"),
                    },
                    doc::EffectProperty {
                        key: "y_offset".to_owned(),
                        value: text("2"),
                    },
                ],
            }],
        }));
    document.pages[0].layers.layers[0].cells[0]
        .elements
        .push(button);

    let exported = export(
        &document,
        &WebExportOptions::default(),
        Workflow::CssProperties,
    )
    .unwrap();
    assert!(exported.files.is_empty());
    let css = exported.html.as_deref().unwrap();

    // Geometry from the contour anchors.
    assert!(css.contains("#object"));
    assert!(css.contains("\tleft: 1px;\n\ttop: 0px;\n\twidth: 2px;\n\theight: 2px;\n"));
    // Fill (ABGR raw → hex), radius, opacity, shadow.
    assert!(css.contains("\tbackground-color: #ff0000;\n"));
    assert!(css.contains("\tborder-radius: 2px;\n"));
    assert!(css.contains("\topacity: 0.500;\n"));
    assert!(css.contains("\tbox-shadow: 3px 2px 4px #000000;\n"));
}

#[test]
fn batch_workflows_follow_their_naming() {
    let document = sliced_document();

    let pages = export(
        &document,
        &WebExportOptions::default(),
        Workflow::PagesToFiles,
    )
    .unwrap();
    assert_eq!(pages.files.len(), 1);
    assert_eq!(pages.files[0].name, "doc_p1.png");
    let (w, h, _) = png_pixels(&pages.files[0].bytes);
    assert_eq!((w, h), (8, 4));

    let states = export(
        &document,
        &WebExportOptions::default(),
        Workflow::StatesToFiles,
    )
    .unwrap();
    assert_eq!(states.files.len(), 1);
    assert_eq!(states.files[0].name, "doc_state1.png");

    let layers = export(
        &document,
        &WebExportOptions::default(),
        Workflow::LayersToFiles,
    )
    .unwrap();
    // Both layers are visible; each exports under its cleaned name.
    assert_eq!(
        layers
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Layer1.png", "WebLayer.png"]
    );
    // The ink layer renders the tile; the web layer renders only the
    // canvas background.
    let (_, _, rgba) = png_pixels(&layers.files[0].bytes);
    assert_eq!(&rgba[0..4], &RED);

    let gif = export(
        &document,
        &WebExportOptions::default(),
        Workflow::StatesToGif,
    )
    .unwrap();
    assert_eq!(gif.files.len(), 1);
    assert_eq!(gif.files[0].name, "doc.gif");
    assert_eq!(&gif.files[0].bytes[..6], b"GIF89a");
}

#[test]
fn every_workflow_is_byte_deterministic() {
    let document = sliced_document();
    let options = WebExportOptions::default();
    for workflow in [
        Workflow::ImagesOnly,
        Workflow::HtmlAndImages,
        Workflow::CssAndImages,
        Workflow::CssSprites,
        Workflow::CssProperties,
        Workflow::PagesToFiles,
        Workflow::StatesToFiles,
        Workflow::LayersToFiles,
        Workflow::StatesToGif,
    ] {
        let first = export(&document, &options, workflow).unwrap();
        let second = export(&document, &options, workflow).unwrap();
        assert_eq!(first, second, "{workflow:?} is not byte-deterministic");
    }
}
