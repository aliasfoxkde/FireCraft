//! SVG import battery: export→import round trips proven at the model
//! level (geometry, paint, group semantics) and at the render level
//! (compose before vs compose after — the same oracle on both sides,
//! so any delta is import loss), plus the limitation report and a
//! hostile-stream arm.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compose::render;
use doc::{
    Contour, Document, Fill, FillShape, Gradient, GradientStop, GroupObject, ObjectData,
    PaintStyle, PathObject, PathPoint,
};
use geom::Point;
use io::svg::{self, SvgOptions};
use io::svgimport::{self};
use raster::Tile;

fn point(x: f64, y: f64) -> PathPoint {
    PathPoint {
        anchor: Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    }
}

/// A closed unit-ish square with an opaque red fill (`FCLi` ABGR
/// 0xFF_00_00_FF) and no stroke.
fn square() -> PathObject {
    PathObject {
        contours: vec![Contour {
            closed: true,
            point_count: 4,
            points: vec![
                point(0.0, 0.0),
                point(4.0, 0.0),
                point(4.0, 4.0),
                point(0.0, 4.0),
            ],
        }],
        primitive: None,
        even_odd: false,
        paint: Some(PaintStyle {
            stroke_raw: 0,
            fill_raw: 0xFF_00_00_FF,
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
    }
}

/// A one-page document with one layer holding `objects`.
fn document(objects: Vec<ObjectData>) -> Document {
    let mut document = Document::default();
    document.info.frame_count = 1;
    document.info.canvas.width = 64.0;
    document.info.canvas.height = 32.0;
    let ids: Vec<doc::ObjectId> = objects
        .into_iter()
        .map(|object| document.registry.insert(object))
        .collect();
    let cell = doc::Cell {
        locked: false,
        visible: true,
        elements: ids,
    };
    let layer = doc::Layer {
        name: "Layer 1".to_owned(),
        display: true,
        cells: vec![cell],
        ..doc::Layer::default()
    };
    document.pages.push(doc::Page {
        info: doc::PageInfo {
            version: document.info.version,
            revision: document.info.revision,
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
        layers: doc::LayerList {
            layers: vec![layer],
        },
        grid: None,
        guides: Default::default(),
        export: None,
    });
    document
}

fn encode(document: &Document) -> Vec<u8> {
    svg::encode(document, &SvgOptions::default())
        .expect("svg export")
        .svg
        .into_bytes()
}

/// Live `PathObject`s of the imported document, in registry order.
fn paths(imported: &Document) -> Vec<&PathObject> {
    imported
        .registry
        .iter()
        .filter_map(|(_, object)| match object {
            ObjectData::Path(path) => Some(path),
            _ => None,
        })
        .collect()
}

fn groups(imported: &Document) -> Vec<&GroupObject> {
    imported
        .registry
        .iter()
        .filter_map(|(_, object)| match object {
            ObjectData::Group(group) => Some(group),
            _ => None,
        })
        .collect()
}

/// The premultiplied max channel delta between two renders.
fn max_delta(a: &[u8], b: &[u8]) -> i64 {
    assert_eq!(a.len(), b.len(), "render sizes disagree");
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| i64::from(x.abs_diff(*y)))
        .max()
        .unwrap_or(0)
}

/// Renders a document's first page/state; empty documents render to a
/// fully transparent plane.
fn rendered(document: &Document) -> Vec<u8> {
    let width = document.info.canvas.width as usize;
    let height = document.info.canvas.height as usize;
    match render(document, 0, 0) {
        Ok(r) => r.pixels,
        Err(_) => vec![0; width * height * 4],
    }
}

/// A gradient-painted rect over (8,8)-(56,24): the M15 `FPLv` model
/// filled in by hand — two opaque stops, `shape_extended` like the
/// engine commands write, both handles set.
fn gradient_path(shape: FillShape, start: (f64, f64), end: (f64, f64)) -> PathObject {
    PathObject {
        contours: vec![Contour {
            closed: true,
            point_count: 4,
            points: vec![
                point(8.0, 8.0),
                point(56.0, 8.0),
                point(56.0, 24.0),
                point(8.0, 24.0),
            ],
        }],
        primitive: None,
        even_odd: false,
        paint: Some(PaintStyle {
            stroke_raw: 0,
            fill_raw: 0xFF18_1F2B,
            fill: Some(Fill {
                shape,
                shape_extended: true,
                texture_blend: 0,
                feather: 0,
                stamping: None,
                hard_edge: false,
                gradient: Some(Gradient {
                    vertices: Vec::new(),
                    rgb: vec![
                        GradientStop {
                            position: 0.0,
                            color_raw: 0xFF18_1F2B,
                        },
                        GradientStop {
                            position: 1.0,
                            color_raw: 0xFFE8_9B2C,
                        },
                    ],
                    alpha: Vec::new(),
                }),
                dither: None,
                pattern: None,
            }),
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
    }
}

/// A linear-gradient fill round-trips: the `FPLv` model reconstructs
/// (ramp, handles, kind), the report counts one import and zero
/// drops, and the renders agree bit-exact — export and import speak
/// the same userSpaceOnUse/pad dialect.
#[test]
fn linear_gradient_fill_round_trips_render_exact() {
    let original = document(vec![ObjectData::Path(gradient_path(
        FillShape::Linear,
        (12.0, 16.0),
        (52.0, 16.0),
    ))]);
    let imported = svgimport::decode(&encode(&original)).expect("import");
    assert_eq!(imported.report.gradients_imported, 1);
    assert_eq!(imported.report.gradients_dropped, 0);
    assert!(
        imported.report.notes.is_empty(),
        "{:?}",
        imported.report.notes
    );

    let imported = imported.document;
    let path = &paths(&imported)[0];
    let paint = path.paint.as_ref().expect("paint survives");
    let fill = paint.fill.as_ref().expect("the gradient model lands");
    assert_eq!(fill.shape, FillShape::Linear);
    assert!(fill.shape_extended, "imports write the FSXi slot");
    let gradient = fill.gradient.as_ref().expect("the ramp lands");
    assert!(
        gradient.alpha.is_empty(),
        "opaque stops omit the coverage ramp"
    );
    let knots: Vec<(f64, i64)> = gradient
        .rgb
        .iter()
        .map(|stop| (stop.position, stop.color_raw))
        .collect();
    assert_eq!(knots.len(), 2, "both stops land");
    // First stop = the ink, second = the amber; the low byte is red.
    assert_eq!(
        knots,
        vec![(0.0, 0xFF18_1F2B), (1.0, 0xFFE8_9B2C)],
        "ramp verbatim"
    );
    // Handles: the export's userSpaceOnUse vector, back verbatim.
    assert_eq!(path.fill_start.map(|p| (p.x, p.y)), Some((12.0, 16.0)));
    assert_eq!(path.fill_end.map(|p| (p.x, p.y)), Some((52.0, 16.0)));
    assert_eq!(max_delta(&rendered(&original), &rendered(&imported)), 0);

    // The imported gradient survives the file boundary: fwmap write →
    // read keeps the fill model and the ramp verbatim.
    let (nodes, stats) =
        doc::fwmap::write(&imported, &doc::fwmap::UnknownBag::default()).expect("fwmap write");
    assert_eq!(stats.unconsumed_unknown, 0, "the writer ate every node");
    let (reread, _) = doc::fwmap::read(&nodes).expect("fwmap read");
    let reopened = &paths(&reread)[0];
    assert_eq!(
        reopened
            .paint
            .as_ref()
            .and_then(|paint| paint.fill.as_ref()),
        path.paint.as_ref().and_then(|paint| paint.fill.as_ref()),
        "the gradient model round-trips the file"
    );
    assert_eq!(
        (reopened.fill_start, reopened.fill_end),
        (path.fill_start, path.fill_end),
        "the handles round-trip the file"
    );
}

/// A radial fill round-trips through cx/cy/r: the centre handle and
/// the rim-along-+x end reconstruct the same circle compose draws.
#[test]
fn radial_gradient_fill_round_trips_render_exact() {
    let original = document(vec![ObjectData::Path(gradient_path(
        FillShape::Radial,
        (32.0, 16.0),
        (52.0, 16.0),
    ))]);
    let imported = svgimport::decode(&encode(&original)).expect("import");
    assert_eq!(imported.report.gradients_imported, 1);
    let imported = imported.document;
    let path = &paths(&imported)[0];
    assert_eq!(
        path.fill_start.map(|p| (p.x, p.y)),
        Some((32.0, 16.0)),
        "centre handle"
    );
    assert_eq!(
        path.fill_end.map(|p| (p.x, p.y)),
        Some((52.0, 16.0)),
        "rim handle reconstructs the radius"
    );
    assert_eq!(max_delta(&rendered(&original), &rendered(&imported)), 0);
}

/// A foreign SVG's objectBoundingBox gradient (with a transform) is
/// normalized by usvg to absolute user space; the import lands it on
/// the model with the absolute handles and paints — the render leaves
/// the matte at the ramp's ink end.
#[test]
fn foreign_object_bbox_gradient_imports_painted() {
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"32\" height=\"16\">\
               <linearGradient id=\"g\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"0\">\
               <stop offset=\"0\" stop-color=\"#000000\"/>\
               <stop offset=\"1\" stop-color=\"#ffffff\"/></linearGradient>\
               <rect x=\"4\" y=\"4\" width=\"24\" height=\"8\" fill=\"url(#g)\"/></svg>";
    let imported = svgimport::decode(svg.as_bytes()).expect("import");
    assert_eq!(imported.report.gradients_imported, 1);
    assert_eq!(imported.report.gradients_dropped, 0);
    let path = &paths(&imported.document)[0];
    assert_eq!(
        path.fill_start.map(|p| (p.x, p.y)),
        Some((4.0, 4.0)),
        "objectBoundingBox x1=0 resolves to the rect's left edge"
    );
    assert_eq!(
        path.fill_end.map(|p| (p.x, p.y)),
        Some((28.0, 4.0)),
        "x2=1 resolves to the right edge"
    );
    // Ink at the black end, off the matte.
    let pixels = rendered(&imported.document);
    assert!(
        pixels.chunks(4).any(|px| px != [255, 255, 255, 255]),
        "the gradient paints"
    );
}

/// A gradient STROKE still drops — the model's stroke is a raw colour
/// only — while a gradient fill on the same path imports.
#[test]
fn gradient_stroke_still_drops_with_a_note() {
    // Two stops — a single-stop gradient collapses to a solid colour
    // in usvg, which would make the stroke importable.
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"32\" height=\"16\">\
               <linearGradient id=\"g\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"0\">\
               <stop offset=\"0\" stop-color=\"#000000\"/>\
               <stop offset=\"1\" stop-color=\"#ffffff\"/></linearGradient>\
               <rect x=\"4\" y=\"4\" width=\"24\" height=\"8\" fill=\"url(#g)\" \
               stroke=\"url(#g)\" stroke-width=\"2\"/></svg>";
    let imported = svgimport::decode(svg.as_bytes()).expect("import");
    assert_eq!(imported.report.gradients_imported, 1, "the fill imports");
    assert_eq!(imported.report.gradients_dropped, 1, "the stroke drops");
    assert!(
        imported
            .report
            .notes
            .iter()
            .any(|note| note.contains("stroke dropped: linear-gradient paint server")),
        "{:?}",
        imported.report.notes
    );
}

/// The canvas matte round-trips through the background slot, not the
/// object list: a non-default `BGCi` exports as the full-canvas rect,
/// imports back into `background_raw` with the exact color, and puts
/// no phantom full-page shape on the layer.
#[test]
fn canvas_matte_round_trips_through_the_background_slot() {
    let mut original = document(vec![ObjectData::Path(square())]);
    // ABGR: a<<24 | b<<16 | g<<8 | r — a deep ink, not the default.
    // The page's canvas is what the exporter reads.
    original.pages[0].info.canvas.background_raw = 0xFF18_1F2B;
    let imported = svgimport::decode(&encode(&original))
        .expect("import")
        .document;
    assert_eq!(
        imported.info.canvas.background_raw, 0xFF18_1F2B,
        "the matte color lands in the background slot"
    );
    assert_eq!(paths(&imported).len(), 1, "only the square is an object");
    // And the renders agree — the same matte, painted the same way.
    assert_eq!(max_delta(&rendered(&original), &rendered(&imported)), 0);

    // A foreign full-canvas rect folds in too, while anything short of
    // the exact canvas stays an object.
    let partial = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"32\" height=\"16\">\
                   <rect x=\"0\" y=\"0\" width=\"30\" height=\"16\" fill=\"#336699\"/>\
                   <rect x=\"0\" y=\"0\" width=\"4\" height=\"4\" fill=\"#ff0000\"/></svg>";
    let imported = svgimport::decode(partial.as_bytes())
        .expect("import")
        .document;
    assert_eq!(
        imported.info.canvas.background_raw, 0xFF_FF_FF,
        "a non-covering rect is not a matte"
    );
    assert_eq!(paths(&imported).len(), 2, "both rects stay objects");
}

#[test]
fn square_round_trips_bit_exact() {
    let original = document(vec![ObjectData::Path(square())]);
    let svg = encode(&original);
    let imported = svgimport::decode(&svg).expect("import").document;

    let imported_paths = paths(&imported);
    assert_eq!(imported_paths.len(), 1);
    let contour = &imported_paths[0].contours[0];
    // The serializer closes with an explicit line back to the start
    // point before `Z`, so the contour carries 5 points.
    assert_eq!(contour.point_count, 5);
    let anchors: Vec<(f64, f64)> = contour
        .points
        .iter()
        .map(|p| (p.anchor.x, p.anchor.y))
        .collect();
    assert_eq!(
        anchors,
        vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0), (0.0, 0.0)]
    );
    let paint = imported_paths[0].paint.as_ref().expect("paint survives");
    assert_eq!(paint.fill_raw, 0xFF_00_00_FF);
    assert_eq!(paint.stroke_raw, 0);

    // Same oracle both sides: bit-exact for axis-aligned solid fills.
    assert_eq!(max_delta(&rendered(&original), &rendered(&imported)), 0);
}

#[test]
fn opacity_and_blend_wrapper_round_trips() {
    let group = GroupObject {
        group_type: 0,
        name: None,
        children: vec![],
        opacity: 500,
        blend: 2, // multiply
        visible: true,
        locked: false,
        mask_hidden: false,
        mask_linked: None,
        effects: Vec::new(),
    };
    let original = document(vec![ObjectData::Path(square()), ObjectData::Group(group)]);
    // Put the child id into the group before building.
    let mut original = original;
    let child_id = doc::ObjectId::from_parts(1, 0);
    if let Some(ObjectData::Group(g)) = original.registry.get_mut(child_id) {
        g.children = vec![doc::ObjectId::from_parts(0, 0)];
    }
    let svg = encode(&original);
    let imported = svgimport::decode(&svg).expect("import");

    let imported_groups = groups(&imported.document);
    // Two: the serializer's `layer-1` wrapper (id → structured) plus
    // the opacity/blend group itself.
    assert_eq!(imported_groups.len(), 2);
    let wrapped = imported_groups
        .iter()
        .find(|g| g.blend != 0)
        .expect("blend group survives");
    assert_eq!(wrapped.opacity, 500);
    assert_eq!(wrapped.blend, 2);
    assert_eq!(wrapped.children.len(), 1);
    assert!(
        imported.report.notes.is_empty(),
        "{:?}",
        imported.report.notes
    );
    assert_eq!(
        max_delta(&rendered(&original), &rendered(&imported.document)),
        0
    );
}

#[test]
fn even_odd_and_fill_on_top_round_trip() {
    // Donut: outer square clockwise, inner square — same winding, so
    // only even-odd punches the hole.
    let mut donut = square();
    donut.even_odd = true;
    donut.contours.push(Contour {
        closed: true,
        point_count: 4,
        points: vec![
            point(1.0, 1.0),
            point(3.0, 1.0),
            point(3.0, 3.0),
            point(1.0, 3.0),
        ],
    });
    let mut stroked = square();
    stroked.paint = Some(PaintStyle {
        stroke_raw: 0xFF_00_00_FF,
        fill_raw: 0xFF_00_00_FF,
        fill: None,
    });
    stroked.fill_on_top = Some(true);

    let original = document(vec![ObjectData::Path(donut), ObjectData::Path(stroked)]);
    let imported = svgimport::decode(&encode(&original)).expect("import");
    let imported_paths = paths(&imported.document);
    // Three: SVG has no fill-over-stroke flag, so the export emits the
    // FOTb square as two same-geometry paths (stroke first, fill over)
    // and import sees them as two objects.
    assert_eq!(imported_paths.len(), 3);
    assert!(imported_paths[0].even_odd, "even-odd survives");
    assert_eq!(
        imported_paths[0].contours.len(),
        2,
        "both donut contours survive"
    );
    // The FOTb halves: stroke-only, then fill-only, in paint order.
    assert_eq!(
        imported_paths[1]
            .paint
            .as_ref()
            .map(|p| (p.stroke_raw, p.fill_raw)),
        Some((0xFF_00_00_FF, 0))
    );
    assert_eq!(
        imported_paths[2]
            .paint
            .as_ref()
            .map(|p| (p.stroke_raw, p.fill_raw)),
        Some((0, 0xFF_00_00_FF))
    );
    assert_eq!(
        max_delta(&rendered(&original), &rendered(&imported.document)),
        0
    );
}

#[test]
fn pass_through_groups_flatten_into_the_parent() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                <g><g><path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"#00ff00\"/></g></g>\
                </svg>";
    let imported = svgimport::decode(svg).expect("import");
    assert!(groups(&imported.document).is_empty(), "no phantom groups");
    assert_eq!(paths(&imported.document).len(), 1);
    let cell_elements = &imported.document.pages[0].layers.layers[0].cells[0].elements;
    assert_eq!(cell_elements.len(), 1, "path sits directly in the cell");
}

#[test]
fn multiple_root_children_all_land_in_the_cell() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                <path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"#ff0000\"/>\
                <path d=\"M 10 0 L 14 0 L 14 4 L 10 4 Z\" fill=\"#0000ff\"/>\
                </svg>";
    let imported = svgimport::decode(svg).expect("import");
    let cell_elements = &imported.document.pages[0].layers.layers[0].cells[0].elements;
    assert_eq!(cell_elements.len(), 2);
    assert_eq!(paths(&imported.document).len(), 2);
}

#[test]
fn transforms_bake_into_geometry() {
    // translate(10, 5) then the path at (0..4): anchors land at
    // (10..14, 5..9). Rotation would introduce f32 noise, so pin the
    // translate case exactly and a rotate case within epsilon.
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                <g transform=\"translate(10 5)\">\
                <path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"#ff0000\"/>\
                </g></svg>";
    let imported = svgimport::decode(svg).expect("import");
    let anchors: Vec<(f64, f64)> = paths(&imported.document)[0].contours[0]
        .points
        .iter()
        .map(|p| (p.anchor.x, p.anchor.y))
        .collect();
    assert_eq!(
        anchors,
        vec![(10.0, 5.0), (14.0, 5.0), (14.0, 9.0), (10.0, 9.0)]
    );

    let rotated = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                    <g transform=\"rotate(90 2 2)\">\
                    <path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"#ff0000\"/>\
                    </g></svg>";
    let imported = svgimport::decode(rotated).expect("import");
    for p in &paths(&imported.document)[0].contours[0].points {
        let (x, y) = (p.anchor.x, p.anchor.y);
        assert!(
            (0.0..=4.0).contains(&x) && (0.0..=4.0).contains(&y),
            "anchor ({x}, {y}) stays inside the rotated square"
        );
    }
}

#[test]
fn quads_raise_to_cubics_at_the_two_third_marks() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                <path d=\"M 0 0 Q 5 10 10 0\" stroke=\"#ff0000\" fill=\"none\"/>\
                </svg>";
    let imported = svgimport::decode(svg).expect("import");
    let contour = &paths(&imported.document)[0].contours[0];
    assert_eq!(contour.point_count, 2);
    let from = &contour.points[0];
    let to = &contour.points[1];
    // f32 path data, so compare against the ⅔ marks within epsilon.
    let close = |a: f64, b: f64| (a - b).abs() < 1e-3;
    let out = from.secondary.as_ref().expect("out control");
    let inc = to.primary.as_ref().expect("in control");
    assert!(close(out.x, 10.0 / 3.0) && close(out.y, 20.0 / 3.0));
    assert!(close(inc.x, 20.0 / 3.0) && close(inc.y, 20.0 / 3.0));
    assert!(from.is_curve && to.is_curve);
}

#[test]
fn text_flattens_to_glyph_paths_with_a_report_note() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                <text x=\"4\" y=\"20\" font-size=\"16\" fill=\"#000000\">Hi</text>\
                </svg>";
    let imported = svgimport::decode(svg).expect("import");
    assert_eq!(imported.report.texts_flattened, 1);
    assert!(
        imported
            .report
            .notes
            .iter()
            .any(|note| note.contains("editable-text")),
        "limitation note present: {:?}",
        imported.report.notes
    );
    assert!(
        !paths(&imported.document).is_empty(),
        "glyph outlines arrive as paths"
    );
    // The glyphs actually rasterize (the fontdb bridge fed usvg our
    // bundled faces).
    let pixels = rendered(&imported.document);
    assert!(
        pixels.chunks(4).any(|px| px != [255, 255, 255, 255]),
        "glyphs are visible (some pixel leaves the matte)"
    );
}

#[test]
fn embedded_image_imports_into_the_tile_bank() {
    // A 4×4 opaque blue PNG as a data URI at (8, 6).
    let plane_rgba = vec![0u8; 4 * 4 * 4];
    let mut rgba = plane_rgba;
    for px in rgba.chunks_mut(4) {
        px.copy_from_slice(&[0, 0, 255, 255]);
    }
    let png = io::png::encode(&io::Plane {
        width: 4,
        height: 4,
        rgba: &rgba,
    })
    .expect("png");
    use base64::Engine as _;
    let uri = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&png)
    );
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
         <image x=\"8\" y=\"6\" width=\"4\" height=\"4\" href=\"{uri}\"/>\
         </svg>"
    );
    let imported = svgimport::decode(svg.as_bytes()).expect("import");
    assert_eq!(imported.report.images, 1);

    let images: Vec<&doc::ImageObject> = imported
        .document
        .registry
        .iter()
        .filter_map(|(_, object)| match object {
            ObjectData::Image(image) => Some(image),
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].pixels, (4, 4));
    assert_eq!(images[0].location.x, 8.0);
    assert_eq!(images[0].location.y, 6.0);
    assert_eq!(imported.document.tiles.len(), 1, "one tile for a 4×4 image");
}

#[test]
fn gradient_fill_imports_beside_a_solid_stroke() {
    // Two stops (a single-stop gradient collapses to a solid color in
    // usvg) plus a solid stroke: the fill lands on the model, the
    // stroke keeps its raw.
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                <linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#ff0000\"/> \
                <stop offset=\"1\" stop-color=\"#0000ff\"/></linearGradient>\
                <path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"url(#g)\" stroke=\"#00ff00\"/>\
                </svg>";
    let imported = svgimport::decode(svg).expect("import");
    assert_eq!(imported.report.gradients_imported, 1);
    assert_eq!(imported.report.gradients_dropped, 0);
    assert!(
        imported.report.notes.is_empty(),
        "{:?}",
        imported.report.notes
    );
    let imported_paths = paths(&imported.document);
    assert_eq!(imported_paths.len(), 1);
    assert_eq!(
        imported_paths[0]
            .paint
            .as_ref()
            .map(|p| (p.stroke_raw, p.fill_raw)),
        Some((0xFF_00_FF_00, 0xFF_00_00_FF)),
        "stroke kept; the fill edge holds the first stop (ABGR red)"
    );
    let fill = imported_paths[0]
        .paint
        .as_ref()
        .and_then(|paint| paint.fill.as_ref())
        .expect("the gradient model lands");
    let gradient = fill.gradient.as_ref().expect("the ramp lands");
    assert_eq!(gradient.rgb.len(), 2, "both stops land");
}

#[test]
fn gradient_only_document_imports() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                <linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#ff0000\"/> \
                <stop offset=\"1\" stop-color=\"#0000ff\"/></linearGradient>\
                <path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"url(#g)\"/>\
                </svg>";
    let imported = svgimport::decode(svg).expect("a painted path is importable");
    assert_eq!(imported.report.gradients_imported, 1);
    assert_eq!(paths(&imported.document).len(), 1);
}

/// Regression: the depth guard counted every self-closing element as a
/// permanently open one, so a long run of `<stop/>` — exactly what the
/// exporter's densified gradient fills emit — tripped the 128-level
/// ceiling on well-formed input.
#[test]
fn self_closing_siblings_do_not_accumulate_depth() {
    let stops = "<stop offset=\"0\" stop-color=\"#000000\"/>".repeat(257);
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
         <linearGradient id=\"g\" x1=\"0\" y1=\"0\" x2=\"8\" y2=\"0\">{stops}\
         </linearGradient>\
         <path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"url(#g)\" stroke=\"#00ff00\"/>\
         </svg>"
    );
    let imported =
        svgimport::decode(svg.as_bytes()).expect("well-formed input clears the depth gate");
    assert_eq!(
        imported.report.gradients_imported, 1,
        "the depth gate holds and the densified gradient imports"
    );
    assert_eq!(
        paths(&imported.document).len(),
        1,
        "the path survives on its stroke"
    );
}

#[test]
fn stroke_width_and_dashing_flatten_with_notes() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                <path d=\"M 0 0 L 10 0\" stroke=\"#ff0000\" stroke-width=\"3\" \
                stroke-dasharray=\"2 1\" fill=\"none\"/>\
                </svg>";
    let imported = svgimport::decode(svg).expect("import");
    assert_eq!(imported.report.strokes_flattened, 1);
    assert!(
        imported
            .report
            .notes
            .iter()
            .any(|note| note.contains("1px") && note.contains("stroke width 3")),
        "{:?}",
        imported.report.notes
    );
    assert!(
        imported
            .report
            .notes
            .iter()
            .any(|note| note.contains("dashing")),
        "{:?}",
        imported.report.notes
    );
}

#[test]
fn clips_masks_and_filters_drop_with_report_entries() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                <clipPath id=\"c\"><rect width=\"4\" height=\"4\"/></clipPath>\
                <mask id=\"m\"><rect width=\"4\" height=\"4\" fill=\"#fff\"/></mask>\
                <filter id=\"f\"><feGaussianBlur stdDeviation=\"1\"/></filter>\
                <g clip-path=\"url(#c)\"><path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"#ff0000\"/></g>\
                <g mask=\"url(#m)\"><path d=\"M 8 0 L 12 0 L 12 4 L 8 4 Z\" fill=\"#00ff00\"/></g>\
                <g filter=\"url(#f)\"><path d=\"M 16 0 L 20 0 L 20 4 L 16 4 Z\" fill=\"#0000ff\"/></g>\
                </svg>";
    let imported = svgimport::decode(svg).expect("import");
    assert_eq!(imported.report.clips_dropped, 2);
    assert_eq!(imported.report.filters_dropped, 1);
    assert_eq!(
        paths(&imported.document).len(),
        3,
        "children import unmasked"
    );
}

#[test]
fn hostile_streams_refuse_without_panicking() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", Vec::new()),
        ("not text", vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0xFF]),
        ("not xml", b"just some words".to_vec()),
        (
            "truncated",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"8\"".to_vec(),
        ),
        (
            "empty svg",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_vec(),
        ),
        (
            "zero canvas",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"0\" height=\"0\"/>".to_vec(),
        ),
        (
            "negative canvas",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"-4\" height=\"-4\"/>".to_vec(),
        ),
        (
            "gigantic canvas",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1e9\" height=\"1e9\"\
              ><path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"#f00\"/></svg>"
                .to_vec(),
        ),
        (
            "entity expansion",
            b"<?xml version=\"1.0\"?><!DOCTYPE svg [<!ENTITY a 'aaaa'>\
              <!ENTITY b '&a;&a;&a;&a;'>]><svg xmlns=\"http://www.w3.org/2000/svg\"\
              width=\"8\" height=\"8\"><text>&b;</text></svg>"
                .to_vec(),
        ),
        (
            "deep nesting",
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"8\" height=\"8\">{}path\
                 </svg>{}",
                "<g>".repeat(4096),
                "</g>".repeat(4096)
            )
            .into_bytes(),
        ),
        (
            "attribute soup",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"8\" height=\"8\"\
               viewBox=\"NaN 0 8 8\" preserveAspectRatio=\"weird\"><path d=\"M garbage\"\
               fill=\"rgb(nonsense)\" stroke=\"url(#missing)\"/></svg>"
                .to_vec(),
        ),
        (
            "unclosed groups",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"8\" height=\"8\"><g><g><path\
              d=\"M 0 0 L 4 0\"/>"
                .to_vec(),
        ),
    ];
    for (name, bytes) in cases {
        // Refusal matters, not the specific variant; the invariant is
        // that no input panics, stacks, or aborts.
        let _ = svgimport::decode(&bytes).expect_err(name);
    }
}

#[test]
fn absurd_but_valid_geometry_imports_without_panicking() {
    // Valid SVG with absurd magnitudes: usvg accepts it, the f64 model
    // carries it, and the render clips it — importing is correct, the
    // invariant is only "no panic, no hang".
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"8\" height=\"8\"\
                ><path d=\"M -1e30 -1e30 L 1e30 1e30 L 0 0 Z\" fill=\"#f00\"/></svg>";
    let imported = svgimport::decode(svg).expect("valid SVG imports");
    let contour = &paths(&imported.document)[0].contours[0];
    assert_eq!(contour.point_count, 3);
    let pixels = rendered(&imported.document);
    assert_eq!(pixels.len(), 8 * 8 * 4);
}

#[test]
fn valid_hostile_document_imports_sane() {
    // Well-formed but adversarial: a zero-opacity group and a path
    // with paint explicitly stripped (`fill="none"` — SVG's default
    // fill is black, so omitting the attribute is *not* unpainted).
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\">\
                <g opacity=\"0\"><path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"#ff0000\"/></g>\
                <path d=\"M 0 0 L 4 0 L 4 4 L 0 4 Z\" fill=\"none\"/>\
                </svg>";
    let imported = svgimport::decode(svg).expect("import");
    // The zero-opacity group still imports (structured, opacity 0).
    let imported_groups = groups(&imported.document);
    assert_eq!(imported_groups.len(), 1);
    assert_eq!(imported_groups[0].opacity, 0);
    assert_eq!(imported_groups[0].children.len(), 1);
    // The paint-stripped path never arrives: usvg culls `fill="none"`
    // paths during tree construction, so nothing registers and no
    // note fires (the importer's no-paint branch stays as defense).
    assert!(
        imported.report.notes.is_empty(),
        "{:?}",
        imported.report.notes
    );
    assert_eq!(paths(&imported.document).len(), 1, "only the group child");
}

// --- `FPTv` pattern fills: single-raster tiles round-trip (export's
// `<pattern>` def imports back onto the model), vector-content tiles
// drop with a report entry like every other un-carried server.

/// An opaque red 128×128 bank tile seeded at slot 0 (the bank starts
/// empty, so the first minted id is deterministic).
fn red_tile() -> Tile {
    let mut plane = [0u8; 128 * 128 * 4];
    for px in plane.chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 0, 0, 255]);
    }
    Tile::from_rgba8(plane)
}

/// A 24×24 square filled entirely by the bank tile in slot 0.
fn pattern_square() -> PathObject {
    PathObject {
        contours: vec![Contour {
            closed: true,
            point_count: 4,
            points: vec![
                point(4.0, 4.0),
                point(28.0, 4.0),
                point(28.0, 28.0),
                point(4.0, 28.0),
            ],
        }],
        primitive: None,
        even_odd: false,
        paint: Some(PaintStyle {
            stroke_raw: 0,
            fill_raw: 0xFF_00_00_FF,
            fill: Some(Fill {
                shape: FillShape::Pattern,
                shape_extended: true,
                texture_blend: 0,
                feather: 0,
                stamping: None,
                hard_edge: false,
                gradient: None,
                dither: None,
                pattern: Some(doc::PatternTile {
                    name: None,
                    pixels: (128, 128),
                    tile_size: 128,
                    tiles: vec![doc::TileRef::Tile(0)],
                }),
            }),
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
    }
}

fn pattern_document() -> Document {
    let mut doc = document(vec![ObjectData::Path(pattern_square())]);
    doc.tiles.insert(0, red_tile());
    doc
}

/// A pattern fill round-trips: the `<pattern>` def imports back onto
/// the `FPTv` model with the tile re-minted into the fresh bank, the
/// report counts one import, and the renders agree bit-exact.
#[test]
fn pattern_fill_round_trips_render_exact() {
    let original = pattern_document();
    let imported = svgimport::decode(&encode(&original)).expect("import");
    assert_eq!(imported.report.patterns_imported, 1);
    assert_eq!(imported.report.gradients_dropped, 0);
    assert!(
        imported.report.notes.is_empty(),
        "{:?}",
        imported.report.notes
    );
    let imported = imported.document;
    let path = paths(&imported)[0];
    let fill = path
        .paint
        .as_ref()
        .expect("paint")
        .fill
        .as_ref()
        .expect("fill");
    let pattern = fill.pattern.as_ref().expect("pattern");
    assert_eq!(fill.shape, FillShape::Pattern);
    assert_eq!(pattern.pixels, (128, 128));
    assert_eq!(pattern.tile_size, 128);
    let doc::TileRef::Tile(id) = pattern.tiles[0] else {
        panic!("expected a bank tile");
    };
    assert!(
        imported.tiles.get(id).is_some(),
        "the tile re-minted into the fresh bank"
    );
    let before = render(&original, 0, 0).expect("original render");
    let after = render(&imported, 0, 0).expect("import render");
    assert_eq!(before.pixels, after.pixels, "render-equal round trip");
}

/// A foreign pattern whose tile is vector content has no raster to
/// mint: the fill drops with the report entry, the path keeps the
/// solid colour.
#[test]
fn foreign_vector_pattern_drops_with_a_report_entry() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
      <defs>
        <pattern id="p" width="8" height="8" patternUnits="userSpaceOnUse">
          <circle cx="4" cy="4" r="2" fill="#00ff00"/>
        </pattern>
      </defs>
      <rect x="1" y="1" width="8" height="8" fill="url(#p)"/>
      <rect x="9" y="1" width="4" height="4" fill="#ff0000"/>
    </svg>"##;
    let imported = svgimport::decode(svg.as_bytes()).expect("import");
    assert_eq!(imported.report.patterns_imported, 0);
    assert_eq!(imported.report.gradients_dropped, 1);
    assert!(
        imported
            .report
            .notes
            .iter()
            .any(|note| note.contains("pattern paint server")),
        "{:?}",
        imported.report.notes
    );
    // The unpainted pattern rect drops with its fill (compose parity:
    // no model paint means nothing renders), the solid sibling lands.
    assert_eq!(paths(&imported.document).len(), 1);
}
