//! The PSD writer battery: encode → decode semantic equality on
//! scripted documents (the ticket's write→read gate), both blend
//! tables, visibility, the flat-composite fallback, and the refusal
//! gates. Every byte the reader accepted must be producible by our
//! own writer, and every document we write must open as the same
//! tree we wrote.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use doc::{
    Cell, Contour, Document, GroupObject, ImageObject, Layer, LayerList, ObjectData, Page,
    PageInfo, PaintStyle, PathObject, PathPoint, TileRef,
};
use geom::Point;
use io::psd;

/// One opaque premultiplied tile with a 2×2 color block at its origin.
fn color_tile(block: [u8; 3]) -> raster::Tile {
    let mut pixels = [0u8; raster::TILE_BYTES];
    for (x, y) in [(0usize, 0usize), (1, 0), (0, 1), (1, 1)] {
        let at = (y * 128 + x) * 4;
        pixels[at..at + 3].copy_from_slice(&block);
        pixels[at + 3] = 255;
    }
    raster::Tile::from_rgba8(pixels)
}

/// An `IMGv` over `tile` at `location`, named `name`.
fn image_object(
    document: &mut Document,
    tile: raster::Tile,
    location: Point,
    name: &str,
    opacity: i64,
    blend: i64,
    visible: bool,
) -> doc::ObjectId {
    let id = u32::try_from(document.tiles.len()).expect("tile id fits");
    document.tiles.insert(id, tile);
    document.registry.insert(ObjectData::Image(ImageObject {
        location,
        offset: Point::ZERO,
        pixels: (2, 2),
        tile_size: 128,
        tiles: vec![TileRef::Tile(id)],
        name: Some(name.to_owned()),
        opacity,
        blend,
        visible,
        locked: false,
        effects: Vec::new(),
    }))
}

/// A filled square path named `name`. `fill_raw` is ABGR — alpha in
/// the high byte; pass e.g. `0xFF_FF_00_00` for opaque blue.
fn path_object(name: &str, fill_raw: i64) -> ObjectData {
    let square = Contour {
        closed: true,
        point_count: 4,
        points: vec![
            PathPoint {
                anchor: Point::new(1.0, 1.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
            PathPoint {
                anchor: Point::new(9.0, 1.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
            PathPoint {
                anchor: Point::new(9.0, 9.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
            PathPoint {
                anchor: Point::new(1.0, 9.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
        ],
    };
    ObjectData::Path(PathObject {
        contours: vec![square],
        primitive: None,
        even_odd: false,
        paint: Some(PaintStyle {
            stroke_raw: 0,
            fill_raw,
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
        name: Some(name.to_owned()),
        effects: Vec::new(),
    })
}

/// A one-page 16×16 document with `layers`, each holding `elements`.
fn document(layers: Vec<(&str, bool, Vec<doc::ObjectId>)>) -> Document {
    let mut document = Document::default();
    document.info.frame_count = 1;
    document.info.canvas.width = 16.0;
    document.info.canvas.height = 16.0;
    document.pages.push(Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: 1,
            frame_count_duplicate: 1,
            request_id: 0,
            canvas: document.info.canvas.clone(),
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
        layers: LayerList {
            layers: layers
                .into_iter()
                .map(|(name, display, elements)| Layer {
                    name: name.to_owned(),
                    display,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements,
                    }],
                    ..Layer::default()
                })
                .collect(),
        },
        grid: None,
        guides: Default::default(),
        export: None,
    });
    document
}

/// The layer names of a decoded PSD, in read-back (bottom-first) order.
fn layer_names(imported: &psd::ImportedPsd) -> Vec<String> {
    imported.document.pages[0]
        .layers
        .layers
        .iter()
        .map(|layer| layer.name.clone())
        .collect()
}

/// The registry entry for `id`, failing the test when it went missing.
fn object(imported: &psd::ImportedPsd, id: doc::ObjectId) -> &doc::ObjectData {
    imported
        .document
        .registry
        .get(id)
        .expect("scripted object survives the round trip")
}

// ------------------------------------------------------------------ //
// The battery.
// ------------------------------------------------------------------ //

#[test]
fn scripted_document_round_trips_semantically() {
    // Bottom-first: red bitmap, a filled path, a group of two rasters,
    // and a hidden raster — names, order, opacity, blend, visibility,
    // and the bitmap's pixels must survive write → read.
    let mut document = document(vec![
        ("dot", true, Vec::new()),
        ("shape", true, Vec::new()),
        ("pair", true, Vec::new()),
        ("ghost", false, Vec::new()),
    ]);
    let dot = image_object(
        &mut document,
        color_tile([255, 0, 0]),
        Point::new(2.0, 3.0),
        "dot",
        1000,
        0,
        true,
    );
    let shape = document
        .registry
        .insert(path_object("shape", 0xFF_FF_00_00));
    let first = image_object(
        &mut document,
        color_tile([0, 255, 0]),
        Point::new(4.0, 4.0),
        "first",
        1000,
        2, // multiply
        true,
    );
    let second = image_object(
        &mut document,
        color_tile([0, 0, 255]),
        Point::new(6.0, 6.0),
        "second",
        502,
        5, // screen
        true,
    );
    let group = document.registry.insert(ObjectData::Group(GroupObject {
        group_type: 0,
        name: Some("pair".to_owned()),
        children: vec![first, second],
        opacity: 700,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        effects: Vec::new(),
    }));
    let ghost = image_object(
        &mut document,
        color_tile([255, 255, 0]),
        Point::ZERO,
        "ghost",
        1000,
        0,
        false,
    );
    // Attach: the model layer's cell holds each element.
    for (name, id) in [
        ("dot", dot),
        ("shape", shape),
        ("pair", group),
        ("ghost", ghost),
    ] {
        let layer = document.pages[0]
            .layers
            .layers
            .iter_mut()
            .find(|layer| layer.name == name)
            .expect("scripted layer");
        layer.cells[0].elements.push(id);
    }

    let bytes = psd::encode(&document, 0).expect("encode");
    let imported = psd::decode(&bytes).expect("round-trips through our own reader");

    // Structure: five raster records (the group's two children
    // included), the group pairing once, four model layers — dot,
    // shape, pair, ghost — in written order.
    assert_eq!(imported.layers, 5);
    assert_eq!(imported.groups, 1);
    assert!(imported.notes.is_empty(), "{:?}", imported.notes);
    assert_eq!(
        layer_names(&imported),
        vec!["dot", "shape", "pair", "ghost"]
    );

    let page = &imported.document.pages[0];
    // The bitmap: pixels extracted at the object's bounds.
    let doc::ObjectData::Image(image) =
        object(&imported, page.layers.layers[0].cells[0].elements[0])
    else {
        panic!("dot imports as an image object");
    };
    assert_eq!(image.location, Point::new(2.0, 3.0));
    assert_eq!(image.pixels, (2, 2));
    let tile = imported.document.tiles.get(0).expect("dot tile");
    let px = |x: usize, y: usize| {
        let at = (y * 128 + x) * 4;
        &tile.pixels()[at..at + 4]
    };
    // Straight alpha on read-back: opaque red is its own premultiply.
    assert_eq!(px(0, 0), &[255, 0, 0, 255]);
    assert_eq!(px(2, 2), &[0, 0, 0, 0], "outside the 2x2 block");
    // The group: opacity 700 → 179/255 → 702‰ (the u8 round trip is
    // within ±2‰), children carry their blends.
    let doc::ObjectData::Group(group) =
        object(&imported, page.layers.layers[2].cells[0].elements[0])
    else {
        panic!("pair imports as a group object");
    };
    assert_eq!(group.children.len(), 2);
    assert_eq!(group.opacity, 702);
    let doc::ObjectData::Image(second) = object(&imported, group.children[1]) else {
        panic!("second imports as an image object");
    };
    assert_eq!(second.blend, 5);
    assert_eq!(second.opacity, 502);
    // Hidden layer stays hidden.
    assert!(!page.layers.layers[3].display);
}

#[test]
fn blend_table_round_trips_through_the_record_bytes() {
    for value in [0i64, 2, 3, 4, 5, 9, 10, 11, 12, 13, 14, 32, 33, 34, 36, 37] {
        let mut document = document(vec![("blend", true, Vec::new())]);
        let id = image_object(
            &mut document,
            color_tile([255, 0, 0]),
            Point::ZERO,
            "blend",
            1000,
            value,
            true,
        );
        document.pages[0].layers.layers[0].cells[0]
            .elements
            .push(id);
        let bytes = psd::encode(&document, 0).expect("encode");
        let imported = psd::decode(&bytes).expect("decode");
        assert!(
            imported.notes.is_empty(),
            "blend {value}: {:?}",
            imported.notes
        );
        assert_eq!(imported.unknown_blends, 0, "blend {value}");
        let id = imported.document.pages[0].layers.layers[0].cells[0].elements[0];
        let doc::ObjectData::Image(image) = object(&imported, id) else {
            panic!("blend {value}: layer imports");
        };
        assert_eq!(image.blend, value, "blend {value} round-trips");
    }
}

#[test]
fn flattened_path_carries_compose_pixels_and_blend() {
    let mut document = document(vec![("shape", true, Vec::new())]);
    let shape = document
        .registry
        .insert(path_object("shape", 0xFF_FF_00_00));
    document.pages[0].layers.layers[0].cells[0]
        .elements
        .push(shape);

    let bytes = psd::encode(&document, 0).expect("encode");
    let imported = psd::decode(&bytes).expect("decode");
    assert_eq!(imported.layers, 1);
    assert!(imported.notes.is_empty(), "{:?}", imported.notes);
    // The flattened layer's pixels are exactly compose's render of the
    // object alone — the same renderer, so the equality is exact.
    let id = imported.document.pages[0].layers.layers[0].cells[0].elements[0];
    let doc::ObjectData::Image(image) = object(&imported, id) else {
        panic!("flattened path imports as an image object");
    };
    assert_eq!(image.location, Point::ZERO);
    assert_eq!(image.pixels, (16, 16), "full-canvas flatten");

    // Pixel oracle: the writer's probe render, un-premultiplied.
    let mut probe = document.clone();
    probe.pages.clear();
    io::raster::build_single_object_page(&mut probe, document.info.canvas.clone(), vec![shape]);
    let rendered = compose::render(&probe, 0, 0).expect("probe render");
    let straight = compose::straight_alpha(&rendered.pixels);
    let tile = imported.document.tiles.get(0).expect("flatten tile");
    for y in 0..16usize {
        for x in 0..16usize {
            let at = (y * 16 + x) * 4;
            let target = &straight[at..at + 4];
            let got = {
                let t = (y * 128 + x) * 4;
                &tile.pixels()[t..t + 4]
            };
            assert_eq!(got, target, "pixel ({x}, {y})");
        }
    }
}

#[test]
fn nested_groups_nest_through_the_pair_walk() {
    let mut document = document(vec![("outer", true, Vec::new())]);
    let leaf_a = image_object(
        &mut document,
        color_tile([0, 255, 0]),
        Point::ZERO,
        "a",
        1000,
        0,
        true,
    );
    let leaf_b = image_object(
        &mut document,
        color_tile([0, 0, 255]),
        Point::ZERO,
        "b",
        1000,
        0,
        true,
    );
    let inner = document.registry.insert(ObjectData::Group(GroupObject {
        group_type: 0,
        name: Some("inner".to_owned()),
        children: vec![leaf_a, leaf_b],
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        effects: Vec::new(),
    }));
    let outer = document.registry.insert(ObjectData::Group(GroupObject {
        group_type: 0,
        name: Some("outer".to_owned()),
        children: vec![inner],
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        effects: Vec::new(),
    }));
    document.pages[0].layers.layers[0].cells[0]
        .elements
        .push(outer);

    let bytes = psd::encode(&document, 0).expect("encode");
    let imported = psd::decode(&bytes).expect("decode");
    assert_eq!(imported.groups, 2);
    assert!(imported.notes.is_empty(), "{:?}", imported.notes);
    let id = imported.document.pages[0].layers.layers[0].cells[0].elements[0];
    let doc::ObjectData::Group(outer) = object(&imported, id) else {
        panic!("outer imports as a group object");
    };
    assert_eq!(outer.children.len(), 1);
    let doc::ObjectData::Group(inner) = object(&imported, outer.children[0]) else {
        panic!("inner imports as a group object");
    };
    assert_eq!(inner.children.len(), 2);
}

#[test]
fn empty_document_falls_back_to_its_composite() {
    let document = document(Vec::new());
    let bytes = psd::encode(&document, 0).expect("encode");
    let imported = psd::decode(&bytes).expect("decode");
    assert!(imported.composite);
    assert_eq!(imported.layers, 0);
    assert_eq!(imported.document.pages[0].layers.layers.len(), 1);
}

#[test]
fn refusals_name_their_reason() {
    let document = document(Vec::new());
    // Page out of range.
    let error = psd::encode(&document, 3).expect_err("page 3 refuses");
    assert!(
        error.to_string().contains("outside the document"),
        "{error}"
    );
    // Canvas beyond the u16 extent.
    let mut huge = document.clone();
    huge.pages[0].info.canvas.width = 70_000.0;
    huge.info.canvas.width = 70_000.0;
    let error = psd::encode(&huge, 0).expect_err("huge canvas refuses");
    assert!(error.to_string().contains("u16 extent"), "{error}");
    // Sub-pixel canvas.
    let mut tiny = document.clone();
    tiny.pages[0].info.canvas.width = 0.2;
    let error = psd::encode(&tiny, 0).expect_err("tiny canvas refuses");
    assert!(
        error.to_string().contains("fewer than one pixel"),
        "{error}"
    );
}

#[test]
fn structural_gates_refuse_with_reasons() {
    // A group child that points outside the registry.
    let mut broken = document(vec![("broken", true, Vec::new())]);
    let dangling = doc::ObjectId::from_parts(9_999, 0);
    let group = broken.registry.insert(ObjectData::Group(GroupObject {
        group_type: 0,
        name: Some("broken".to_owned()),
        children: vec![dangling],
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        effects: Vec::new(),
    }));
    broken.pages[0].layers.layers[0].cells[0]
        .elements
        .push(group);
    let error = psd::encode(&broken, 0).expect_err("dangling child refuses");
    assert!(error.to_string().contains("dangling"), "{error}");

    // Nesting past the group-depth ceiling.
    let mut deep = document(vec![("deep", true, Vec::new())]);
    let leaf = image_object(
        &mut deep,
        color_tile([255, 0, 0]),
        Point::ZERO,
        "leaf",
        1000,
        0,
        true,
    );
    let mut child = leaf;
    for level in 0..70 {
        child = deep.registry.insert(ObjectData::Group(GroupObject {
            group_type: 0,
            name: Some(format!("shell-{level}")),
            children: vec![child],
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            mask_hidden: false,
            effects: Vec::new(),
        }));
    }
    deep.pages[0].layers.layers[0].cells[0].elements.push(child);
    let error = psd::encode(&deep, 0).expect_err("70-deep nesting refuses");
    assert!(error.to_string().contains("nest deeper than 64"), "{error}");
}

#[test]
fn writer_output_is_deterministic() {
    let mut document = document(vec![("dot", true, Vec::new())]);
    let dot = image_object(
        &mut document,
        color_tile([12, 34, 56]),
        Point::ZERO,
        "dot",
        1000,
        0,
        true,
    );
    document.pages[0].layers.layers[0].cells[0]
        .elements
        .push(dot);
    let first = psd::encode(&document, 0).expect("first encode");
    let second = psd::encode(&document, 0).expect("second encode");
    assert_eq!(first, second);
}
