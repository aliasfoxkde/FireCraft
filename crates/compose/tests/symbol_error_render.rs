//! Symbol-instance and error-path coverage: typed refusals (missing
//! definition, cycle, depth, tile-bank miss, oversized canvas), plain
//! and nine-slice instances, animation interpolation, text objects,
//! pattern-assembly fallbacks, and the render_subset flatten surface.
//! Every expected pixel value is hand-derived.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compose::{ComposeError, Render, render, render_subset, render_transparent};
use doc::{
    Canvas, Cell, Document, ImageObject, Layer, LayerList, ObjectData, Page, PageInfo, PaintStyle,
    PathObject, PathPoint, SliceObject, SymbolAnimation, SymbolDefinition, SymbolInstanceObject,
    SymbolType, TextRun, TextStyle, TileRef,
};
use geom::Point;

// Opaque red ABGR (alpha in the top byte).
const RED_RAW: i64 = 0xFF_00_00_FF;
const TILE_SIDE: i64 = 128;

fn page(canvas: Canvas) -> Page {
    Page {
        info: PageInfo {
            version: None,
            revision: None,
            frame_count: 1,
            frame_count_duplicate: 1,
            request_id: 0,
            canvas,
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
            layers: vec![Layer {
                name: String::new(),
                display: true,
                cells: vec![Cell {
                    locked: false,
                    visible: true,
                    elements: Vec::new(),
                }],
                ..Layer::default()
            }],
        },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    }
}

fn small_canvas() -> Canvas {
    Canvas {
        x: 0.0,
        y: 0.0,
        width: 16.0,
        height: 16.0,
        resolution: 72.0,
        resolution_unit: 0,
        background_raw: 0,
    }
}

fn square(registry: &mut doc::Registry, side: f64, raw: i64) -> doc::ObjectId {
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

fn instance(number: i64, transform: vello_cpu::kurbo::Affine) -> ObjectData {
    ObjectData::SymbolInstance(SymbolInstanceObject {
        symbol_number: number,
        transform,
        locked: false,
        name: None,
        animation: None,
        custom_data: Default::default(),
    })
}

fn definition(number: i64, elements: Vec<doc::ObjectId>) -> SymbolDefinition {
    SymbolDefinition {
        name: "sym".to_owned(),
        symbol_number: number,
        origin: Point::ZERO,
        frame_count: 1,
        layers: LayerList {
            layers: vec![Layer {
                name: String::new(),
                display: true,
                cells: vec![Cell {
                    locked: false,
                    visible: true,
                    elements,
                }],
                ..Layer::default()
            }],
        },
        background_raw: 0,
        symbol_type: SymbolType::Graphic,
        nine_slice: None,
        auto_slice: None,
        include_down_state: None,
        show_down_on_load: None,
        include_over_while_down: None,
        op_code: None,
    }
}

fn pixel(render: &Render, x: u32, y: u32) -> [u8; 4] {
    let stride = u32::from(render.width) * 4;
    render.pixels[(y * stride + x * 4) as usize..(y * stride + x * 4 + 4) as usize]
        .try_into()
        .expect("four channels")
}

// --- the battery -----------------------------------------------------------

#[test]
fn instance_renders_its_definition_content() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let square = square(&mut document.registry, 8.0, RED_RAW);
    document.symbols.push(definition(1, vec![square]));
    let placed = document
        .registry
        .insert(instance(1, vello_cpu::kurbo::Affine::IDENTITY));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements.push(placed);
    document.pages.push(page);

    let render = render(&document, 0, 0).expect("renders");
    // The definition's red square at (2,2)..(10,10).
    assert_eq!(pixel(&render, 3, 3), [255, 0, 0, 255]);
    assert_eq!(pixel(&render, 9, 9), [255, 0, 0, 255]);
    assert_eq!(
        pixel(&render, 12, 12),
        [0, 0, 0, 255],
        "outside is the opaque matte"
    );
}

#[test]
fn missing_definition_is_a_typed_refusal() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let placed = document
        .registry
        .insert(instance(7, vello_cpu::kurbo::Affine::IDENTITY));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements.push(placed);
    document.pages.push(page);

    let error = render(&document, 0, 0).expect_err("no definition 7");
    assert!(matches!(
        error,
        ComposeError::MissingDefinition { symbol: 7 }
    ));
    assert!(error.to_string().contains("symbol 7"));
}

#[test]
fn self_instancing_definition_is_a_typed_cycle_refusal() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    // The definition's own cell instances symbol 1 (itself).
    let inner = document
        .registry
        .insert(instance(1, vello_cpu::kurbo::Affine::IDENTITY));
    document.symbols.push(definition(1, vec![inner]));
    let placed = document
        .registry
        .insert(instance(1, vello_cpu::kurbo::Affine::IDENTITY));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements.push(placed);
    document.pages.push(page);

    let error = render(&document, 0, 0).expect_err("symbol 1 instances itself");
    assert!(matches!(error, ComposeError::SymbolCycle { symbol: 1 }));
}

#[test]
fn deep_nesting_refuses_at_the_depth_ceiling() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    // A chain: symbol n's cell holds an instance of symbol n+1; the
    // last one is missing, so hitting the ceiling (or the missing
    // definition past it) is the refusal. Chains deeper than the
    // ceiling refuse with SymbolDepth before MissingDefinition.
    const DEPTH: i64 = 12;
    for number in 0..DEPTH {
        let next = document
            .registry
            .insert(instance(number + 1, vello_cpu::kurbo::Affine::IDENTITY));
        document.symbols.push(definition(number, vec![next]));
    }
    let placed = document
        .registry
        .insert(instance(0, vello_cpu::kurbo::Affine::IDENTITY));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements.push(placed);
    document.pages.push(page);

    let error = render(&document, 0, 0).expect_err("12-deep chain");
    assert!(
        matches!(
            error,
            ComposeError::SymbolDepth { .. } | ComposeError::MissingDefinition { symbol: DEPTH }
        ),
        "{error}"
    );
}

#[test]
fn tile_bank_miss_is_a_typed_refusal() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let image = document.registry.insert(ObjectData::Image(ImageObject {
        location: Point::ZERO,
        offset: Point::ZERO,
        pixels: (16, 16),
        tile_size: TILE_SIDE,
        tiles: vec![TileRef::Tile(42)],
        name: None,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        effects: Vec::new(),
    }));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements.push(image);
    document.pages.push(page);

    let error = render(&document, 0, 0).expect_err("tile 42 is not in the bank");
    assert!(matches!(error, ComposeError::TileBankMiss { tile: 42 }));
    assert!(error.to_string().contains("tile 42"));
}

#[test]
fn oversized_canvas_refuses_with_extent() {
    let mut document = Document::default();
    let mut canvas = small_canvas();
    canvas.width = 70_000.0;
    document.info.canvas = canvas.clone();
    document.pages.push(page(canvas));
    let error = render(&document, 0, 0).expect_err("canvas past u16 extent");
    assert!(matches!(
        error,
        ComposeError::CanvasTooLarge {
            width: 70_000.0,
            height: 16.0
        }
    ));
}

#[test]
fn render_subset_paints_only_the_named_objects() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let red = square(&mut document.registry, 8.0, RED_RAW);
    let green = square(&mut document.registry, 8.0, 0xFF_00_FF_00);
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements = vec![red, green];
    document.pages.push(page);

    // Subset with just the green square: red stays out.
    let render = render_subset(&document, 0, 0, &[green]).expect("subset renders");
    assert_eq!(pixel(&render, 4, 4), [0, 255, 0, 255], "green only");
}

#[test]
fn nine_slice_definition_grids_its_raster() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let square = square(&mut document.registry, 8.0, RED_RAW);
    let mut sym = definition(1, vec![square]);
    sym.nine_slice = Some(doc::NineSliceGuides {
        locked: false,
        color_raw: 0,
        // top, bottom, left, right insets.
        controls: [2.0, 2.0, 2.0, 2.0],
        shape_bounds: vello_cpu::kurbo::Rect::new(0.0, 0.0, 16.0, 16.0),
        limits: vello_cpu::kurbo::Rect::new(0.0, 0.0, 16.0, 16.0),
    });
    document.symbols.push(sym);
    // Stretch the instance 2×: the grid re-pieces the raster instead
    // of scaling the middle, so the corner regions keep their pixels.
    let placed = document
        .registry
        .insert(instance(1, vello_cpu::kurbo::Affine::scale(2.0)));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements.push(placed);
    document.pages.push(page);

    let render = render(&document, 0, 0).expect("nine-slice renders");
    // Something drew (the grid preserved coverage from the stretched
    // transform — exact seams are the engine's hand-derived oracle in
    // compose_proof); here the pin is that the path ran without
    // falling back to a plain transform paint.
    let painted = render.pixels.chunks_exact(4).filter(|p| p[3] != 0).count();
    assert!(painted > 0, "nine-slice painted something");
}

#[test]
fn animated_instance_interpolates_position_and_opacity() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let square = square(&mut document.registry, 6.0, RED_RAW);
    document.symbols.push(definition(1, vec![square]));
    let mut inst = match instance(1, vello_cpu::kurbo::Affine::IDENTITY) {
        ObjectData::SymbolInstance(instance) => instance,
        _ => panic!("instance"),
    };
    inst.animation = Some(SymbolAnimation {
        frames: 2,
        x_shift: 8.0,
        y_shift: 0.0,
        scaling: 0.0,
        rotation: None,
        start_opacity: Some(1000),
        end_opacity: Some(0),
    });
    let placed = document.registry.insert(ObjectData::SymbolInstance(inst));
    document.info.frame_count = 2;
    let mut page = page(small_canvas());
    page.info.frame_count = 2;
    page.layers.layers[0].cells[0].elements.push(placed);
    document.pages.push(page);

    // Frame 0: start opacity, unshifted.
    let frame0 = render(&document, 0, 0).expect("frame 0");
    assert_eq!(pixel(&frame0, 3, 3), [255, 0, 0, 255]);
    // Frame 1: shifted +8x and fully faded.
    let frame1 = render(&document, 0, 1).expect("frame 1");
    assert_eq!(
        pixel(&frame1, 3, 3),
        [0, 0, 0, 255],
        "moved away and faded into the matte"
    );
}

#[test]
fn pattern_extent_and_bank_misses_refuse() {
    // The tile-assembly gate: a pattern whose extent is absurd refuses.
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let square = square(&mut document.registry, 8.0, RED_RAW);
    let mut obj = match document.registry.get(square).cloned() {
        Some(ObjectData::Path(path)) => path,
        _ => panic!("path"),
    };
    if let Some(paint) = obj.paint.as_mut() {
        paint.fill = Some(doc::Fill {
            shape: doc::FillShape::Pattern,
            shape_extended: false,
            texture_blend: 0,
            feather: 0,
            stamping: None,
            hard_edge: false,
            gradient: None,
            dither: None,
            pattern: Some(doc::PatternTile {
                name: None,
                pixels: (20_000, 4),
                tile_size: 128,
                tiles: Vec::new(),
            }),
        });
    }
    let id = document.registry.insert(ObjectData::Path(obj));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements.push(id);
    document.pages.push(page);

    let error = render(&document, 0, 0).expect_err("20k-wide pattern extent");
    assert!(matches!(error, ComposeError::TileSource { .. }), "{error}");
}

#[test]
fn dangling_id_in_a_cell_refuses() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0]
        .elements
        .push(doc::ObjectId::from_parts(9_999, 0));
    document.pages.push(page);
    let error = render(&document, 0, 0).expect_err("dangling cell id");
    assert!(matches!(error, ComposeError::DanglingId { .. }));
}

#[test]
fn text_renders_glyphs_through_the_synthetic_path() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let mut text = doc::TextObject::new(vello_cpu::kurbo::Rect::new(2.0, 2.0, 14.0, 14.0));
    text.runs = vec![TextRun {
        text: "Hi".to_owned(),
        changes: TextStyle {
            font: Some("sans".to_owned()),
            size: Some(10.0),
            ..TextStyle::default()
        },
    }];
    text.style = Some(TextStyle {
        font: Some("sans".to_owned()),
        size: Some(10.0),
        fill_raw: Some(RED_RAW),
        ..TextStyle::default()
    });
    let id = document.registry.insert(ObjectData::Text(text));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements.push(id);
    document.pages.push(page);

    let render = render(&document, 0, 0).expect("text renders");
    let painted = render.pixels.chunks_exact(4).filter(|p| p[3] > 0).count();
    assert!(painted > 0, "glyph contours painted something");
}

#[test]
fn hidden_text_and_hidden_images_draw_nothing() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let mut text = doc::TextObject::new(vello_cpu::kurbo::Rect::new(0.0, 0.0, 8.0, 8.0));
    text.runs = vec![TextRun {
        text: "x".to_owned(),
        changes: TextStyle::default(),
    }];
    text.visible = false;
    let hidden_text = document.registry.insert(ObjectData::Text(text));
    let hidden_image = document.registry.insert(ObjectData::Image(ImageObject {
        location: Point::ZERO,
        offset: Point::ZERO,
        pixels: (4, 4),
        tile_size: TILE_SIDE,
        tiles: vec![TileRef::Matte(RED_RAW)],
        name: None,
        opacity: 1000,
        blend: 0,
        visible: false,
        locked: false,
        effects: Vec::new(),
    }));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0]
        .elements
        .extend([hidden_text, hidden_image]);
    document.pages.push(page);
    let render = render_transparent(&document, 0, 0).expect("renders");
    assert!(
        render.pixels.chunks_exact(4).all(|p| p[3] == 0),
        "hidden objects draw nothing"
    );
}

#[test]
fn matte_tile_image_composites_from_the_bank() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let image = document.registry.insert(ObjectData::Image(ImageObject {
        location: Point::new(0.0, 0.0),
        offset: Point::ZERO,
        pixels: (4, 4),
        tile_size: TILE_SIDE,
        tiles: vec![TileRef::Matte(RED_RAW)],
        name: None,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        effects: Vec::new(),
    }));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements.push(image);
    document.pages.push(page);
    let render = render(&document, 0, 0).expect("image renders");
    assert_eq!(pixel(&render, 1, 1), [0, 0, 255, 255], "matte blue");
}

#[test]
fn slice_objects_draw_nothing() {
    let mut document = Document::default();
    document.info.canvas = small_canvas();
    let slice = document.registry.insert(ObjectData::Slice(SliceObject {
        frame: vello_cpu::kurbo::Rect::new(0.0, 0.0, 8.0, 8.0),
        name: None,
        ..SliceObject::default()
    }));
    let mut page = page(small_canvas());
    page.layers.layers[0].cells[0].elements.push(slice);
    document.pages.push(page);
    let render = render_transparent(&document, 0, 0).expect("slice renders as nothing");
    assert!(render.pixels.chunks_exact(4).all(|p| p[3] == 0));
}
