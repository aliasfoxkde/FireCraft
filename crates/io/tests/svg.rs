//! SVG export battery: the serializer is proven element-for-element
//! against `compose`'s semantics — path data, paint attributes, the
//! blend/opacity wrapper, the filter chain (LUT endpoints pinned from
//! the real kernels), text, embedded images, and the drop report.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use compose::straight_alpha;
use doc::{
    Cell, Contour, Document, Effect, EffectProperty, EffectValue, GroupObject, ImageObject, Layer,
    LayerList, ObjectData, Page, PageInfo, PaintStyle, PathObject, PathPoint, TileRef,
};
use geom::Point;
use io::svg::{self, SvgOptions};

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
    document.pages.push(Page {
        info: PageInfo {
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
        layers: LayerList {
            layers: vec![Layer {
                name: "Layer 1".to_owned(),
                display: true,
                cells: vec![Cell {
                    locked: false,
                    visible: true,
                    elements: ids,
                }],
                ..Layer::default()
            }],
        },
        grid: None,
        guides: Default::default(),
        export: None,
    });
    document
}

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

fn encode(document: &Document) -> (String, svg::SvgReport) {
    let out = svg::encode(document, &SvgOptions::default()).expect("svg");
    (out.svg, out.report)
}

#[test]
fn export_is_deterministic() {
    let document = document(vec![ObjectData::Path(square())]);
    let first = encode(&document).0;
    let second = encode(&document).0;
    assert_eq!(first, second);
}

#[test]
fn path_serializes_geometry_and_paint() {
    let (svg, report) = encode(&document(vec![ObjectData::Path(square())]));
    assert!(report.notes.is_empty(), "{:?}", report.notes);
    // Closed contour: three window segments, the closing segment, Z.
    assert!(svg.contains(
        "<path d=\"M 0 0 L 4 0 L 4 4 L 0 4 L 0 0 Z\" fill=\"#ff0000\" fill-rule=\"nonzero\" stroke=\"none\" stroke-width=\"1\"/>"
    ), "{svg}");
}

#[test]
fn bezier_controls_collapse_like_the_compositor() {
    let mut from = point(0.0, 0.0);
    from.secondary = Some(Point::new(1.0, 0.0));
    let mut to = point(4.0, 0.0);
    to.primary = Some(Point::new(3.0, 0.0));
    let mut path = square();
    path.contours = vec![Contour {
        closed: false,
        point_count: 2,
        points: vec![from, to],
    }];
    let (svg, _) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(
        svg.contains("M 0 0 C 1 0 3 0 4 0"),
        "one-sided controls collapse onto the segment endpoint: {svg}"
    );
}

#[test]
fn even_odd_fill_rule_is_carried() {
    let mut path = square();
    path.even_odd = true;
    let (svg, _) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(svg.contains("fill-rule=\"evenodd\""), "{svg}");
}

#[test]
fn fill_on_top_orders_two_paths() {
    let mut path = square();
    path.fill_on_top = Some(true);
    path.paint = Some(PaintStyle {
        stroke_raw: 0xFF_00_00_00,
        fill_raw: 0xFF_00_00_FF,
        fill: None,
    });
    let (svg, _) = encode(&document(vec![ObjectData::Path(path)]));
    let d = "M 0 0 L 4 0 L 4 4 L 0 4 L 0 0 Z";
    let stroke_at = svg
        .find(&format!("<path d=\"{d}\" fill=\"none\" stroke=\"#000000\""))
        .expect("stroke path");
    let fill_at = svg
        .find(&format!("<path d=\"{d}\" fill=\"#ff0000\" fill-rule"))
        .expect("fill path");
    assert!(stroke_at < fill_at, "stroke paints before the fill: {svg}");
}

#[test]
fn opacity_and_blend_wrap_in_a_group() {
    let mut path = square();
    path.opacity = 500;
    path.blend = 2; // multiply
    let (svg, _) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(
        svg.contains("<g opacity=\"0.5\" style=\"mix-blend-mode:multiply\">"),
        "{svg}"
    );
}

#[test]
fn semi_transparent_paint_gets_a_paint_opacity() {
    let mut path = square();
    path.paint = Some(PaintStyle {
        stroke_raw: 0,
        fill_raw: 0x80_00_00_FF, // alpha 128 red
        fill: None,
    });
    let (svg, _) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(
        svg.contains("fill=\"#ff0000\" fill-opacity=\"0.5019607843137255\""),
        "{svg}"
    );
}

#[test]
fn invisible_and_unpainted_paths_follow_compose() {
    let mut hidden = square();
    hidden.visible = false;
    let unpainted = PathObject {
        paint: None,
        ..square()
    };
    let (svg, report) = encode(&document(vec![
        ObjectData::Path(hidden),
        ObjectData::Path(unpainted),
    ]));
    assert!(!svg.contains("<path"), "{svg}");
    assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
    assert!(report.notes[0].contains("without paint"));
}

#[test]
fn groups_nest_their_children() {
    let mut inner = square();
    inner.opacity = 250;
    let mut group = GroupObject {
        group_type: 0,
        name: None,
        children: Vec::new(),
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        mask_hidden: false,
        mask_linked: None,
        effects: Vec::new(),
    };
    let mut document = document(Vec::new());
    let inner_id = document.registry.insert(ObjectData::Path(inner));
    group.children.push(inner_id);
    let group_id = document.registry.insert(ObjectData::Group(group));
    document.pages[0].layers.layers[0].cells[0].elements = vec![group_id];

    let (svg, _) = encode(&document);
    // An attribute-less group is a pass-through (no wrapper — and no
    // closing tag either); the opacity-carrying group gets one,
    // nesting the path inside the layer group. The document must stay
    // well-formed: every `<g` opened is closed exactly once.
    assert_eq!(svg.matches("<g ").count(), 2, "{svg}");
    assert_eq!(svg.matches("</g>").count(), 2, "{svg}");
    let layer_at = svg.find("<g id=\"layer-1\">").expect("layer");
    let wrap_at = svg.find("<g opacity=\"0.25\">").expect("opacity wrapper");
    let path_at = svg.find("<path").expect("path");
    assert!(layer_at < wrap_at && wrap_at < path_at, "{svg}");
    let wrap_close = svg[wrap_at..].find("</g>").expect("wrapper close") + wrap_at;
    assert!(path_at < wrap_close, "path lives inside the wrapper: {svg}");
}

#[test]
fn invert_filter_maps_to_the_exact_lut() {
    let mut path = square();
    path.effects = vec![effect("invert", &[])];
    let (svg, report) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(report.notes.is_empty(), "{:?}", report.notes);
    assert!(svg.contains("<filter id=\"fc0\" filterUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"64\" height=\"32\" color-interpolation-filters=\"sRGB\">"), "{svg}");
    assert!(svg.contains("<g filter=\"url(#fc0)\">"), "{svg}");
    // The ramp LUT of invert: input 0 → 1.0, input 255 → 0.0.
    let values = svg
        .split("tableValues=\"")
        .nth(1)
        .expect("table values")
        .split('"')
        .next()
        .expect("closing quote");
    let mut tokens = values.split(' ');
    assert_eq!(tokens.next(), Some("1"));
    assert_eq!(tokens.next_back(), Some("0"));
    assert_eq!(values.split(' ').count(), 256);
}

#[test]
fn brightness_contrast_derives_its_lut_from_the_kernel() {
    let mut path = square();
    path.effects = vec![effect(
        "brightnesscontrast",
        &[("brightness_amount", "100"), ("contrast_amount", "0")],
    )];
    let (svg, _) = encode(&document(vec![ObjectData::Path(path)]));
    let values = svg
        .split("tableValues=\"")
        .nth(1)
        .expect("table values")
        .split('"')
        .next()
        .expect("closing quote");
    let parsed: Vec<f64> = values
        .split(' ')
        .map(|token| token.parse::<f64>().expect("table entry"))
        .collect();
    assert_eq!(parsed.len(), 256);
    // The LUT must be the real kernel curve: non-decreasing, and the
    // endpoints pinned by the `filters` crate's documented behavior
    // ("brightness +100 → +128 levels", top of ramp saturates).
    for pair in parsed.windows(2) {
        assert!(pair[0] <= pair[1], "ramp must be non-decreasing");
    }
    assert!(parsed[255] > 0.99, "bright end saturates: {}", parsed[255]);
    let dark = 128.0_f64 / 255.0 - 1e-9;
    assert!(
        (parsed[0] - dark).abs() < 1e-9,
        "input 0 lifts to 128: {}",
        parsed[0]
    );
}

#[test]
fn unmappable_kernels_drop_the_whole_filter_with_a_note() {
    let mut path = square();
    path.effects = vec![effect("findedges", &[]), effect("invert", &[])];
    let (svg, report) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(!svg.contains("<filter"), "no filter may be emitted: {svg}");
    assert!(!svg.contains("filter="), "{svg}");
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("'findedges'") && note.contains("find edges")),
        "{:?}",
        report.notes
    );
    // The invert step still happened in compose — the path itself
    // remains in the output, unfiltered.
    assert!(svg.contains("<path"), "{svg}");
}

#[test]
fn blur_filters_map_to_their_primitives() {
    let mut gaussian = square();
    gaussian.effects = vec![effect("gaussianblur", &[("gaussian_blur_radius", "2.5")])];
    let mut boxy = square();
    boxy.effects = vec![effect("blur", &[])];
    let mut sharp = square();
    sharp.effects = vec![effect(
        "unsharpmask",
        &[("unsharp_mask_radius", "2"), ("unsharp_mask_amount", "50")],
    )];
    let (svg, _) = encode(&document(vec![
        ObjectData::Path(gaussian),
        ObjectData::Path(boxy),
        ObjectData::Path(sharp),
    ]));
    assert!(
        svg.contains("<feGaussianBlur stdDeviation=\"2.5\"/>"),
        "{svg}"
    );
    assert!(
        svg.contains(
            "order=\"3\" kernelMatrix=\"1 1 1 1 1 1 1 1 1\" divisor=\"9\" edgeMode=\"duplicate\""
        ),
        "{svg}"
    );
    assert!(
        svg.contains("k2=\"1.5\" k3=\"-0.5\""),
        "unsharp arithmetic composite: {svg}"
    );
    assert_eq!(svg.matches("<filter id=").count(), 3, "{svg}");
}

#[test]
fn text_carries_style_and_runs() {
    let mut text = doc::TextObject::new(geom::Rect::new(2.0, 4.0, 30.0, 20.0));
    text.style = Some(doc::TextStyle {
        font: Some("Liberation Sans".to_owned()),
        size: Some(12.0),
        bold: Some(true),
        fill_raw: Some(0xFF_00_00_FF),
        ..doc::TextStyle::default()
    });
    text.runs = vec![doc::TextRun {
        text: "hi & bye".to_owned(),
        changes: doc::TextStyle::default(),
    }];
    let (svg, report) = encode(&document(vec![ObjectData::Text(text)]));
    assert!(
        svg.contains(
            "<text x=\"2\" y=\"4\" dominant-baseline=\"text-before-edge\" font-family=\"Liberation Sans\" font-size=\"12\" font-weight=\"bold\" fill=\"#ff0000\">hi &amp; bye</text>"
        ),
        "{svg}"
    );
    assert_eq!(report.notes.len(), 1);
    assert!(report.notes[0].contains("font stack"));
}

#[test]
fn images_embed_as_png_data_uris() {
    let mut document = document(Vec::new());
    // A 2×1 tile image: tile 0 red opaque, tile 1 (matte) blue.
    let mut red = raster::Tile::new();
    for px in red.pixels_mut().chunks_exact_mut(4) {
        px.copy_from_slice(&[255, 0, 0, 255]);
    }
    document.tiles.insert(0, red);
    let image = ImageObject {
        location: Point::ZERO,
        offset: Point::ZERO,
        pixels: (256, 128),
        tile_size: 128,
        tiles: vec![TileRef::Tile(0), TileRef::Matte(0xFF_FF_00_00)],
        name: None,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        effects: Vec::new(),
    };
    let id = document.registry.insert(ObjectData::Image(image));
    document.pages[0].layers.layers[0].cells[0].elements = vec![id];

    let (svg, report) = encode(&document);
    assert!(report.notes.iter().any(|note| note.contains("data-URI")));
    let uri_at = svg.find("href=\"data:image/png;base64,").expect("data uri");
    let payload = &svg[uri_at + "href=\"data:image/png;base64,".len()..];
    let end = payload.find('"').expect("closing quote");
    use base64::Engine as _;
    let png = base64::engine::general_purpose::STANDARD
        .decode(&payload[..end])
        .expect("base64");
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']), "png signature");
    // Decode and check the matte half is blue after the un-premultiply.
    let img = image::load_from_memory(&png).expect("decode").to_rgba8();
    assert_eq!((img.width(), img.height()), (256, 128));
    let first = img.get_pixel(0, 0);
    assert_eq!((first[0], first[1], first[2], first[3]), (255, 0, 0, 255));
    let matte = img.get_pixel(200, 0);
    assert_eq!(
        (matte[0], matte[1], matte[2], matte[3]),
        (0, 0, 255, 255),
        "matte tile decodes as blue"
    );
}

#[test]
fn export_matches_compose_on_the_pinned_document() {
    // The red square over the white matte: SVG text is not rendered
    // here, but the geometry+paint it describes must equal what compose
    // rasterizes — proven at the pixel level via the xtask golden
    // resvg loop; this test pins the cheap invariant (a non-empty,
    // deterministic, well-formed document).
    let document = document(vec![ObjectData::Path(square())]);
    let (svg, _) = encode(&document);
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"32\" viewBox=\"0 0 64 32\">"));
    assert!(svg.ends_with("</svg>\n"));
    // Well-formedness floor: opens balance closes at every level (the
    // canvas-matte `<rect>` is self-closing like every path).
    assert_eq!(svg.matches("<g").count(), svg.matches("</g>").count());
    assert_eq!(
        svg.matches("<path").count() + svg.matches("<rect").count(),
        svg.matches("/>").count()
    );
    assert_eq!(svg.matches("<g id=\"layer-1\">").count(), 1);
    // Sanity: the compositor's own render of the same document is the
    // bytes the golden loop compares the SVG against.
    let render = compose::render(&document, 0, 0).expect("compose");
    let straight = straight_alpha(&render.pixels);
    assert_eq!(straight.len(), 64 * 32 * 4);
    assert_ne!(
        straight[0..4],
        [255, 255, 255, 255],
        "square covers the origin (the matte would mean it drew nothing)"
    );
}

#[test]
fn hostile_documents_are_refused_cleanly() {
    let mut document = document(Vec::new());
    document.info.canvas.width = 0.4;
    document.info.canvas.height = 0.4;
    document.pages[0].info.canvas = document.info.canvas.clone();
    let out = svg::encode(&document, &SvgOptions::default()).expect_err("empty canvas");
    assert!(matches!(
        out,
        io::ExportError::Unsupported { format: "svg", .. }
    ));

    let out = svg::encode(&Document::default(), &SvgOptions { page: 3, frame: 0 })
        .expect_err("no such page");
    assert!(matches!(
        out,
        io::ExportError::Unsupported { format: "svg", .. }
    ));
}

// --- `FPLv` gradient fills (M15 task 5): the def + reference form is
// compose's `fill_brush` mapping transcribed; the svg-golden gate
// holds the two rasters together, these tests pin the serializer.

use doc::{Fill, FillShape, Gradient, GradientStop, PatternTile};

fn ramp(a: i64, b: i64) -> Vec<GradientStop> {
    vec![
        GradientStop {
            position: 0.0,
            color_raw: a,
        },
        GradientStop {
            position: 1.0,
            color_raw: b,
        },
    ]
}

fn gradient_path(
    shape: FillShape,
    rgb: Vec<GradientStop>,
    alpha: Vec<GradientStop>,
    handles: Option<(Point, Point)>,
) -> PathObject {
    let mut path = square();
    if let Some((start, end)) = handles {
        path.fill_start = Some(start);
        path.fill_end = Some(end);
    }
    if let Some(paint) = path.paint.as_mut() {
        paint.fill = Some(Fill {
            shape,
            shape_extended: true,
            texture_blend: 0,
            feather: 0,
            stamping: None,
            hard_edge: false,
            gradient: Some(Gradient {
                vertices: Vec::new(),
                rgb,
                alpha,
            }),
            dither: None,
            pattern: None,
        });
    }
    path
}

#[test]
fn linear_gradient_exports_a_def_and_reference() {
    let path = gradient_path(
        FillShape::Linear,
        ramp(0xFF_00_00_00, 0xFF_FF_FF_FF),
        Vec::new(),
        Some((Point::new(0.0, 0.0), Point::new(8.0, 0.0))),
    );
    let (svg, report) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(
        svg.contains(
            "<linearGradient id=\"grad-1\" gradientUnits=\"userSpaceOnUse\" \
             x1=\"0\" y1=\"0\" x2=\"8\" y2=\"0\" spreadMethod=\"pad\">\n\
             <stop offset=\"0\" stop-color=\"#000000\" stop-opacity=\"1\"/>\n\
             <stop offset=\"1\" stop-color=\"#ffffff\" stop-opacity=\"1\"/>\n\
             </linearGradient>"
        ),
        "linear def rides the start→end vector: {svg}"
    );
    assert!(
        svg.contains("<path d=\"M 0 0 L 4 0 L 4 4 L 0 4 L 0 0 Z\" fill=\"url(#grad-1)\""),
        "the path references the def: {svg}"
    );
    assert!(report.notes.is_empty(), "opaque linear is exact, no notes");
    assert_eq!(report.gradient_fills, 1, "the def is counted for the gates");
}

#[test]
fn radial_gradient_centers_on_the_start_handle() {
    let path = gradient_path(
        FillShape::Radial,
        ramp(0xFF_00_00_00, 0xFF_FF_FF_FF),
        Vec::new(),
        Some((Point::new(4.0, 4.0), Point::new(12.0, 4.0))),
    );
    let (svg, _) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(
        svg.contains(
            "<radialGradient id=\"grad-1\" gradientUnits=\"userSpaceOnUse\" \
             cx=\"4\" cy=\"4\" r=\"8\" spreadMethod=\"pad\">"
        ),
        "radius is the start→end distance: {svg}"
    );
}

/// The M15 regression pin: with a coverage ramp the stops densify and
/// every interior stop must carry the straight-interpolated colour and
/// the independent coverage. A channel-lane swap in the ramp sampler
/// produced exactly the mirrored colours and zero opacities this
/// assertion rejects (the monochrome fixtures hid it; the gate caught
/// it on the veil).
#[test]
fn coverage_ramp_interpolates_straight_at_mid() {
    let path = gradient_path(
        FillShape::Linear,
        // `FG0v`: blue → yellow.
        ramp(0xFF_FF_00_00, 0xFF_00_FF_FF),
        // `FG1v`: transparent → opaque.
        ramp(0x00_00_00_00, 0xFF_00_00_00),
        Some((Point::new(0.0, 0.0), Point::new(8.0, 0.0))),
    );
    let (svg, _) = encode(&document(vec![ObjectData::Path(path)]));
    let stops = svg.matches("<stop ").count();
    assert!(
        stops > 128,
        "a coverage ramp densifies the stops (got {stops})"
    );
    assert!(
        svg.contains(
            "<stop offset=\"0.5\" stop-color=\"#808080\" \
             stop-opacity=\"0.5019607843137255\"/>"
        ),
        "mid-ramp: straight colour halfway blue→yellow, coverage halfway: {svg}"
    );
    assert!(
        svg.contains("<stop offset=\"0\" stop-color=\"#0000ff\" stop-opacity=\"0\"/>")
            && svg.contains("<stop offset=\"1\" stop-color=\"#ffff00\" stop-opacity=\"1\"/>"),
        "endpoints hold the ramp ends: {svg}"
    );
}

#[test]
fn conical_gradient_records_the_divergence_and_exports_solid() {
    let path = gradient_path(
        FillShape::Conical,
        ramp(0xFF_00_00_00, 0xFF_FF_FF_FF),
        Vec::new(),
        Some((Point::new(0.0, 0.0), Point::new(8.0, 0.0))),
    );
    let (svg, report) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(
        !svg.contains("Gradient"),
        "SVG has no sweep primitive: {svg}"
    );
    assert_eq!(report.notes.len(), 1, "the divergence is recorded");
    assert!(report.notes[0].contains("conical"), "{}", report.notes[0]);
    assert!(
        svg.contains("fill=\"#ff0000\""),
        "the solid `FCLi` colour rides instead: {svg}"
    );
}

#[test]
fn gradient_without_handles_exports_solid_like_compose() {
    let path = gradient_path(
        FillShape::Linear,
        ramp(0xFF_00_00_00, 0xFF_FF_FF_FF),
        Vec::new(),
        None,
    );
    let (svg, report) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(!svg.contains("Gradient"), "no def without handles: {svg}");
    assert!(report.notes.is_empty());
    assert!(svg.contains("fill=\"#ff0000\""), "solid `FCLi`: {svg}");
}

// --- `FPTv` pattern fills (post-roadmap gap): the def + reference
// form is compose's pattern-arm placement transcribed; the svg-golden
// gate holds the two rasters together, these tests pin the serializer.

fn pattern_path(pattern: PatternTile, handles: Option<(Point, Point)>) -> PathObject {
    let mut path = square();
    if let Some((start, end)) = handles {
        path.fill_start = Some(start);
        path.fill_end = Some(end);
    }
    if let Some(paint) = path.paint.as_mut() {
        paint.fill = Some(Fill {
            shape: FillShape::Pattern,
            shape_extended: true,
            texture_blend: 0,
            feather: 0,
            stamping: None,
            hard_edge: false,
            gradient: None,
            dither: None,
            pattern: Some(pattern),
        });
    }
    path
}

fn matte_pattern(raw: i64) -> PatternTile {
    PatternTile {
        name: None,
        pixels: (2, 2),
        tile_size: 128,
        tiles: vec![TileRef::Matte(raw)],
    }
}

#[test]
fn pattern_fill_exports_a_def_and_reference() {
    let path = pattern_path(matte_pattern(0xFF_00_FF_00), None);
    let (svg, report) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(
        svg.contains(
            "<pattern id=\"pat-1\" patternUnits=\"userSpaceOnUse\" \
             width=\"2\" height=\"2\">"
        ),
        "identity placement without handles: {svg}"
    );
    assert!(
        svg.contains(
            "<image x=\"0\" y=\"0\" width=\"2\" height=\"2\" \
                      href=\"data:image/png;base64,"
        ),
        "the repeat unit embeds as a data-URI PNG: {svg}"
    );
    assert!(
        svg.contains("fill=\"url(#pat-1)\""),
        "the path references the def: {svg}"
    );
    assert!(report.notes.is_empty());
    assert_eq!(report.pattern_fills, 1, "the def is counted");
}

#[test]
fn pattern_fill_transform_rides_the_fill_vector() {
    let path = pattern_path(
        matte_pattern(0xFF_00_FF_00),
        Some((Point::new(1.0, 2.0), Point::new(5.0, 2.0))),
    );
    let (svg, _) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(
        svg.contains("patternTransform=\"translate(1,2) rotate(0) scale(2)\""),
        "translate(start) rotate(angle) scale(length / native width): {svg}"
    );
}

#[test]
fn pattern_missing_tiles_drop_to_solid_with_a_note() {
    let pattern = PatternTile {
        name: None,
        pixels: (2, 2),
        tile_size: 128,
        tiles: vec![TileRef::Tile(9)],
    };
    let path = pattern_path(pattern, None);
    let (svg, report) = encode(&document(vec![ObjectData::Path(path)]));
    assert!(!svg.contains("<pattern"), "no def on a bank miss: {svg}");
    assert_eq!(report.pattern_fills, 0);
    assert_eq!(report.notes.len(), 1, "the divergence is recorded");
    assert!(
        report.notes[0].contains("solid colour") && report.notes[0].contains("tile 9"),
        "{}",
        report.notes[0]
    );
    assert!(svg.contains("fill=\"#ff0000\""), "solid `FCLi`: {svg}");
}
