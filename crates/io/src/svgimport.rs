//! SVG 1.1 import: an SVG byte stream → a fresh [`doc::Document`]
//! through `usvg` (the parsing/layout half of resvg), mirrored against
//! [`crate::svg`] so an export→import round trip stays render-equal.
//!
//! What arrives where:
//! - paths → `PATv` objects; usvg's absolute transforms are **baked
//!   into the geometry** (the model carries no per-object transform
//!   spine), quads raise to cubics on the way in;
//! - groups → groups; opacity per-mille, blend values through
//!   `compose::blend_value` (the inverse of the export's table);
//! - solid fills/strokes → `FCLi` paint raws; gradient fill servers
//!   import onto the `FPLv` model (linear/radial onto
//!   [`doc::FillShape`] with the ramp split into `FG0v`/`FG1v`);
//!   pattern servers and SVG filters are dropped **with report
//!   entries** (the model has no pattern spine and no
//!   SVG-filter→`EFLv` mapping); strokes stay solid-only.
//! - clip paths and masks: children import **unmasked** with a report
//!   entry — compose's own group semantics (decision 0005 parity);
//! - embedded rasters → `IMGv` objects over the shared tile bank
//!   ([`crate::raster::attach_image`]);
//! - text → usvg's fontdb layout flattens glyphs to paths against the
//!   bundled `text` faces (the same binaries the compositor
//!   rasterizes), so rendering matches — but the text object itself is
//!   not reconstructed (editable-text import is a recorded limitation).
//!
//! Strokes: the model records 1px centered strokes only (the M4
//! limitation), so a foreign width imports at 1px **with a report
//! entry** rather than being dropped or faked.

use compose::{Mix, blend_value};
use doc::{
    Contour, Document, Fill, FillShape, Gradient, GradientStop, GroupObject, ObjectData,
    PaintStyle, PathObject, PathPoint,
};
use geom::Point;
use usvg::tiny_skia_path::{Point as TsPoint, Transform as TsTransform};

use crate::{ImportError, raster};

/// Everything the importer did besides the happy path — the
/// machine-checkable answer to "what did this import not carry?"
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SvgImportReport {
    /// Objects and features dropped or approximated, with reasons, in
    /// walk order.
    pub notes: Vec<String>,
    /// Text nodes flattened to glyph paths (editable text was lost).
    pub texts_flattened: usize,
    /// Paint roles lost to gradient/pattern servers.
    pub gradients_dropped: usize,
    /// Gradient fill servers imported onto the `FPLv` model.
    pub gradients_imported: usize,
    /// Strokes whose width flattened to the model's 1px.
    pub strokes_flattened: usize,
    /// Groups carrying a clip path or mask, imported unmasked.
    pub clips_dropped: usize,
    /// Groups carrying SVG filter effects, imported unfiltered.
    pub filters_dropped: usize,
    /// Embedded raster images imported as `IMGv` objects.
    pub images: usize,
}

/// The imported document plus its report.
#[derive(Clone, Debug)]
pub struct ImportedSvg {
    /// A fresh single-page document; canvas = the SVG's size.
    pub document: Document,
    /// What was skipped or approximated.
    pub report: SvgImportReport,
}

/// Parses an SVG stream into a fresh document.
///
/// # Errors
/// [`ImportError::Unsupported`] for a stream that is not SVG, a canvas
/// that rounds to fewer than one pixel, or an unparseable embedded
/// image; [`ImportError::Decode`] when usvg rejects the document.
pub fn decode(bytes: &[u8]) -> Result<ImportedSvg, ImportError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ImportError::Unsupported {
        format: "svg",
        reason: "stream is not UTF-8 text".to_owned(),
    })?;

    // fontdb over the bundled faces only — a hermetic, deterministic
    // font stack (no system font scan), identical to what compose
    // rasterizes text with.
    // usvg's tree converter (and the XML parser beneath it) recurse
    // per nesting level with no internal cap, so a hostile document
    // can overflow the stack inside the dependencies. Gate on our own
    // depth ceiling first, measured by a flat scan that is safe at any
    // hostile nesting.
    const MAX_ELEMENT_DEPTH: usize = 128;
    let deepest = max_element_depth(text);
    if deepest > MAX_ELEMENT_DEPTH {
        return Err(ImportError::Unsupported {
            format: "svg",
            reason: format!("element nesting deeper than {MAX_ELEMENT_DEPTH} levels"),
        });
    }

    let mut options = usvg::Options {
        font_family: text::SANS_FAMILY.to_owned(),
        ..usvg::Options::default()
    };
    {
        let db = options.fontdb_mut();
        for face in text::FACE_BINARIES {
            db.load_font_data(face.to_vec());
        }
    }

    let tree = usvg::Tree::from_str(text, &options).map_err(|error| ImportError::Decode {
        format: "svg",
        reason: error.to_string(),
    })?;

    let size = tree.size();
    let width = f64::from(size.width()).round();
    let height = f64::from(size.height()).round();
    if width < 1.0 || height < 1.0 {
        return Err(ImportError::Unsupported {
            format: "svg",
            reason: "canvas rounds to fewer than one pixel on an axis".to_owned(),
        });
    }
    // compose's `Render` carries u16 dimensions — a canvas wider than
    // that can never be rendered, so importing it would only relocate
    // the failure (same ceiling as the raster import path).
    if width > 65535.0 || height > 65535.0 {
        return Err(ImportError::Unsupported {
            format: "svg",
            reason: format!("canvas {width}×{height} exceeds the 65535px render ceiling"),
        });
    }

    let mut document = Document::default();
    document.info.frame_count = 1;
    document.info.canvas.width = width;
    document.info.canvas.height = height;
    let mut canvas = document.info.canvas.clone();

    // A leading full-canvas solid rect is the canvas matte our own
    // serializer emits (`BGCi` ↔ background rect), not an object — it
    // folds back into `background_raw` so the round trip carries no
    // phantom full-page shape.
    let root_children = tree.root().children();
    let nodes = match canvas_matte(root_children.first(), width, height) {
        Some(raw) => {
            document.info.canvas.background_raw = raw;
            canvas = document.info.canvas.clone();
            &root_children[1..]
        }
        None => root_children,
    };

    let (elements, report) = {
        let mut importer = Importer {
            document: &mut document,
            report: SvgImportReport::default(),
        };
        let mut elements = Vec::new();
        importer.walk_nodes(nodes, TsTransform::identity(), &mut elements);
        (elements, importer.report)
    };
    if elements.is_empty() {
        return Err(ImportError::Unsupported {
            format: "svg",
            reason: "document carries no importable geometry".to_owned(),
        });
    }

    raster::build_single_object_page(&mut document, canvas, elements);
    Ok(ImportedSvg { document, report })
}

struct Importer<'a> {
    document: &'a mut Document,
    report: SvgImportReport,
}

impl Importer<'_> {
    /// Walks one usvg group's children, appending the object id each
    /// child becomes (or nothing for dropped ones) to `elements`.
    /// `extra` composes with each node's absolute transform — identity
    /// for tree nodes, the text node's transform for `flattened()`
    /// subroots.
    fn walk_group(
        &mut self,
        group: &usvg::Group,
        extra: TsTransform,
        elements: &mut Vec<doc::ObjectId>,
    ) {
        self.walk_nodes(group.children(), extra, elements);
    }

    /// The group walk over an explicit node slice — the root call
    /// peels the canvas matte (a full-canvas solid rect, below) before
    /// handing usvg's children over.
    fn walk_nodes(
        &mut self,
        nodes: &[usvg::Node],
        extra: TsTransform,
        elements: &mut Vec<doc::ObjectId>,
    ) {
        for node in nodes {
            match node {
                usvg::Node::Group(group) => {
                    if group.clip_path().is_some() || group.mask().is_some() {
                        self.report.clips_dropped += 1;
                        self.report.notes.push(
                            "group with clip/mask imports unmasked (compose parity)".to_owned(),
                        );
                    }
                    if !group.filters().is_empty() {
                        self.report.filters_dropped += 1;
                        self.report
                            .notes
                            .push("group with SVG filters imports unfiltered".to_owned());
                    }
                    let mut children = Vec::new();
                    self.walk_group(group, extra, &mut children);
                    if children.is_empty() {
                        continue;
                    }
                    let opacity = per_mille(group.opacity());
                    let blend = blend_value(svg_blend(group.blend_mode()));
                    let structured = opacity != 1000
                        || blend != 0
                        || group.clip_path().is_some()
                        || group.mask().is_some()
                        || !group.filters().is_empty()
                        || !group.id().is_empty();
                    if structured {
                        let id = self
                            .document
                            .registry
                            .insert(ObjectData::Group(GroupObject {
                                group_type: 0,
                                name: non_empty(group.id()),
                                children,
                                opacity,
                                blend,
                                visible: true,
                                locked: false,
                                mask_hidden: false,
                                effects: Vec::new(),
                            }));
                        elements.push(id);
                    } else {
                        // A pass-through group flattens into its parent —
                        // the same shape the serializer emits nothing for.
                        elements.extend(children);
                    }
                }
                usvg::Node::Path(path) => {
                    if !path.is_visible() {
                        continue;
                    }
                    if let Some(object) = self.import_path(path, extra) {
                        elements.push(self.document.registry.insert(ObjectData::Path(object)));
                    }
                }
                usvg::Node::Image(image) => {
                    if !image.is_visible() {
                        continue;
                    }
                    match self.import_image(image, extra) {
                        Ok(id) => {
                            self.report.images += 1;
                            elements.push(id);
                        }
                        Err(reason) => self
                            .report
                            .notes
                            .push(format!("embedded image dropped: {reason}")),
                    }
                }
                usvg::Node::Text(node) => {
                    // Glyph paths via usvg's fontdb layout over our
                    // bundled faces — rendering matches the compositor;
                    // the editable text object does not survive.
                    self.report.texts_flattened += 1;
                    self.report.notes.push(
                        "text arrives as glyph paths (editable-text import is a recorded \
                         limitation)"
                            .to_owned(),
                    );
                    let mut children = Vec::new();
                    self.walk_group(node.flattened(), node.abs_transform(), &mut children);
                    elements.extend(children);
                }
            }
        }
    }

    fn import_path(&mut self, path: &usvg::Path, extra: TsTransform) -> Option<PathObject> {
        // Bake the absolute transform (plus any subroot `extra`) into
        // the geometry; the model has no transform spine.
        let transform = compose_transform(extra, path.abs_transform());
        let data = path.data().clone().transform(transform)?;

        let mut fill_raw = 0;
        let mut fill_model = None;
        let mut fill_handles = (None, None);
        let mut stroke_raw = 0;
        let mut even_odd = false;
        let mut fill_on_top = None;
        if let Some(fill) = path.fill() {
            match fill.paint() {
                usvg::Paint::Color(color) => {
                    fill_raw = paint_raw(color, fill.opacity());
                }
                other => match self.gradient_fill(other, transform) {
                    Some((edge_raw, model, start, end)) => {
                        // The edge colour doubles as the beyond-the-ramp
                        // hold; a fully transparent first stop still
                        // paints (the interior carries the coverage).
                        fill_raw = edge_raw;
                        fill_model = Some(model);
                        fill_handles = (Some(start), Some(end));
                    }
                    None => {
                        self.report.gradients_dropped += 1;
                        self.report
                            .notes
                            .push(format!("fill dropped: {} paint server", paint_kind(other)));
                    }
                },
            }
            even_odd = fill.rule() == usvg::FillRule::EvenOdd;
        }
        if let Some(stroke) = path.stroke() {
            match stroke.paint() {
                usvg::Paint::Color(color) => {
                    stroke_raw = paint_raw(color, stroke.opacity());
                    let width = f64::from(stroke.width().get());
                    if (width - 1.0).abs() > f64::EPSILON {
                        self.report.strokes_flattened += 1;
                        self.report.notes.push(format!(
                            "stroke width {width} flattens to 1px (the model records 1px \
                             centered strokes only)"
                        ));
                    }
                    if stroke.dasharray().is_some() {
                        self.report
                            .notes
                            .push("stroke dashing dropped (no model spine)".to_owned());
                    }
                }
                other => {
                    self.report.gradients_dropped += 1;
                    self.report.notes.push(format!(
                        "stroke dropped: {} paint server",
                        paint_kind(other)
                    ));
                }
            }
            // `StrokeAndFill` paints the stroke first, fill over — the
            // model's `FOTb`.
            if path.paint_order() == usvg::PaintOrder::StrokeAndFill {
                fill_on_top = Some(true);
            }
        }
        if fill_raw == 0 && stroke_raw == 0 && fill_model.is_none() {
            self.report
                .notes
                .push("path without paint renders as nothing (compose parity)".to_owned());
            return None;
        }

        let contours = segments_to_contours(&data);
        if contours.is_empty() {
            return None;
        }
        Some(PathObject {
            contours,
            primitive: None,
            even_odd,
            paint: Some(PaintStyle {
                stroke_raw,
                fill_raw,
                fill: fill_model,
            }),
            fill_start: fill_handles.0,
            fill_end: fill_handles.1,
            fill_third: None,
            brush_placement: None,
            fill_on_top,
            locked: false,
            visible: true,
            opacity: 1000,
            blend: 0,
            name: non_empty(path.id()),
            effects: Vec::new(),
        })
    }

    fn import_image(
        &mut self,
        image: &usvg::Image,
        extra: TsTransform,
    ) -> Result<doc::ObjectId, String> {
        let bytes: &[u8] = match image.kind() {
            usvg::ImageKind::PNG(data)
            | usvg::ImageKind::JPEG(data)
            | usvg::ImageKind::GIF(data)
            | usvg::ImageKind::WEBP(data) => data,
            usvg::ImageKind::SVG(_) => {
                return Err("nested SVG images are not imported".to_owned());
            }
        };
        let decoded =
            crate::raster::decode(bytes).map_err(|error| format!("embedded raster: {error}"))?;
        let transform = compose_transform(extra, image.abs_transform());
        if (transform.sx - 1.0).abs() > f32::EPSILON
            || (transform.sy - 1.0).abs() > f32::EPSILON
            || transform.kx != 0.0
            || transform.ky != 0.0
        {
            self.report.notes.push(
                "embedded image scale/skew dropped; placed at its translated origin".to_owned(),
            );
        }
        let id = raster::attach_image(
            self.document,
            decoded.width,
            decoded.height,
            &decoded.rgba,
            non_empty(image.id()),
        )
        .map_err(|error| error.to_string())?;
        // Placement: the translation component reaches `location` (the
        // model's only image placement spine).
        if let Some(ObjectData::Image(object)) = self.document.registry.get_mut(id) {
            object.location = geom::Point::new(f64::from(transform.tx), f64::from(transform.ty));
        }
        Ok(id)
    }
}

impl Importer<'_> {
    /// A gradient paint server onto the fill model: the ramp split
    /// into the `FG0v` colour and `FG1v` coverage lists, the fill
    /// kind, and canvas-space control handles (usvg reports the
    /// server in the element's user space, so the path's absolute
    /// transform applies here exactly as it did to the geometry).
    /// Returns the edge colour (`fill_raw` — the beyond-the-ramp hold
    /// the renderer falls back to), the fill, and the start/end
    /// handles. `None` for servers the model can't carry (pattern)
    /// and stop-less gradients, which render nothing anyway.
    fn gradient_fill(
        &mut self,
        paint: &usvg::Paint,
        transform: TsTransform,
    ) -> Option<(i64, Fill, Point, Point)> {
        // usvg normalizes objectBoundingBox units and the
        // `gradientTransform` into the server's own transform (raw
        // coordinates stay unit-space), so the full chain to canvas
        // space is the path's absolute transform ∘ the server's.
        let (shape, start, end, stops, spread) = match paint {
            usvg::Paint::LinearGradient(grad) => {
                let space = compose_transform(transform, grad.transform());
                (
                    FillShape::Linear,
                    apply_ts(space, f64::from(grad.x1()), f64::from(grad.y1())),
                    apply_ts(space, f64::from(grad.x2()), f64::from(grad.y2())),
                    grad.stops(),
                    grad.spread_method(),
                )
            }
            usvg::Paint::RadialGradient(grad) => {
                let space = compose_transform(transform, grad.transform());
                let center = apply_ts(space, f64::from(grad.cx()), f64::from(grad.cy()));
                let radius = f64::from(grad.r().get());
                // The rim along +x: compose reads the radius as the
                // centre→end handle distance.
                (
                    FillShape::Radial,
                    center,
                    Point::new(center.x + radius, center.y),
                    grad.stops(),
                    grad.spread_method(),
                )
            }
            _ => return None,
        };
        if stops.is_empty() {
            return None;
        }
        // The model's ramps hold at the edges (`Extend::Pad` in the
        // renderer); reflect/repeat approximates to that, noted.
        if spread != usvg::SpreadMethod::Pad {
            self.report.notes.push(format!(
                "gradient spread {spread:?} imports padded (the model's ramps hold at the edges)"
            ));
        }

        // SVG stops may arrive out of order; the ramps are positional.
        let mut ordered: Vec<usvg::Stop> = stops.to_vec();
        ordered.sort_by(|a, b| {
            a.offset()
                .get()
                .partial_cmp(&b.offset().get())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut rgb = Vec::with_capacity(ordered.len());
        let mut alpha = Vec::with_capacity(ordered.len());
        let mut edge_raw = 0;
        for (index, stop) in ordered.iter().enumerate() {
            let position = f64::from(stop.offset().get());
            let color = stop.color();
            let coverage = (f64::from(stop.opacity().get()) * 255.0).round() as i64;
            rgb.push(GradientStop {
                position,
                color_raw: 0xFF00_0000
                    | (i64::from(color.blue) << 16)
                    | (i64::from(color.green) << 8)
                    | i64::from(color.red),
            });
            alpha.push(GradientStop {
                position,
                color_raw: coverage << 24,
            });
            if index == 0 {
                edge_raw = paint_raw(&color, stop.opacity());
            }
        }
        // A fully-opaque ramp needs no coverage list — the renderer
        // treats a missing `FG1v` as opaque, and the file convention
        // omits the slot (the writer emits exactly one shape then).
        if alpha
            .iter()
            .all(|stop| (stop.color_raw >> 24) & 0xFF == 0xFF)
        {
            alpha.clear();
        }
        self.report.gradients_imported += 1;
        Some((
            edge_raw,
            Fill {
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
            },
            start,
            end,
        ))
    }
}

/// Applies an affine transform to a point (the tiny-skia transform
/// carries the row-major SVG matrix).
fn apply_ts(transform: TsTransform, x: f64, y: f64) -> Point {
    Point::new(
        f64::from(transform.sx) * x + f64::from(transform.kx) * y + f64::from(transform.tx),
        f64::from(transform.ky) * x + f64::from(transform.sy) * y + f64::from(transform.ty),
    )
}

/// Recognizes the canvas matte our own serializer emits: the first
/// root child, a rect-covering path over the exact canvas with an
/// opaque solid paint and no stroke. Returns the `BGCi`-style ABGR
/// raw (alpha forced opaque — the same unpinned-alpha reading the
/// renderer and `bg_hex` use) or `None` for any other geometry.
fn canvas_matte(node: Option<&usvg::Node>, width: f64, height: f64) -> Option<i64> {
    let usvg::Node::Path(path) = node? else {
        return None;
    };
    if path.stroke().is_some() {
        return None;
    }
    let fill = path.fill()?;
    if f64::from(fill.opacity().get()) < 1.0 {
        return None;
    }
    let usvg::Paint::Color(color) = &fill.paint() else {
        return None;
    };
    let bounds = path.data().bounds();
    let exact = |a: f64, b: f64| (a - b).abs() < 1e-9;
    if !exact(f64::from(bounds.left()), 0.0)
        || !exact(f64::from(bounds.top()), 0.0)
        || !exact(f64::from(bounds.right()), width)
        || !exact(f64::from(bounds.bottom()), height)
    {
        return None;
    }
    Some(
        i64::from(0xFF00_0000u32)
            | i64::from(u32::from(color.blue)) << 16
            | i64::from(u32::from(color.green)) << 8
            | i64::from(u32::from(color.red)),
    )
}

/// The deepest element nesting in an SVG text, measured by a flat,
/// stack-safe scan. Exact for well-formed XML (tag bodies with quoted
/// `>`, comments, CDATA, processing instructions, and DOCTYPE internal
/// subsets are all skipped); on exotic-but-valid documents it can only
/// overcount, which merely rejects — never undercounts into a deeper
/// parse than the gate allowed.
fn max_element_depth(text: &str) -> usize {
    let bytes = text.as_bytes();
    let (mut depth, mut deepest) = (0usize, 0usize);
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        match bytes.get(i + 1) {
            // Closing tag.
            Some(b'/') => {
                depth = depth.saturating_sub(1);
                i += 2;
            }
            // `<!--`, `<![CDATA[`, `<!DOCTYPE`, `<?` — skip the whole
            // construct so its text cannot distort the count.
            Some(b'!') | Some(b'?') => {
                i = skip_construct(bytes, i);
            }
            // Element start: measure to the tag end so quoted `>`
            // attribute text is handled, and a self-closing tag opens
            // and closes in one — it must not leave depth inflated
            // (long runs of `<stop/>`, as in a densified gradient,
            // otherwise trip the ceiling on well-formed input).
            Some(c) if c.is_ascii_alphabetic() || *c == b'_' || *c == b':' => {
                let (end, self_closing) = tag_end(bytes, i);
                if !self_closing {
                    depth += 1;
                    deepest = deepest.max(depth);
                }
                i = end;
            }
            _ => i += 1,
        }
    }
    deepest
}

/// The index just past the `>` closing the tag that starts at `start`
/// (which points at `<`), and whether that tag is self-closing (`/>`).
/// Quoted attribute values are honored; an unterminated tag consumes
/// the rest of the input.
fn tag_end(bytes: &[u8], start: usize) -> (usize, bool) {
    let mut i = start + 1;
    let mut quote = 0u8;
    let mut prev = 0u8;
    while i < bytes.len() {
        let b = bytes[i];
        if quote != 0 {
            if b == quote {
                quote = 0;
            }
            prev = 0;
        } else if b == b'"' || b == b'\'' {
            quote = b;
            prev = 0;
        } else if b == b'>' {
            return (i + 1, prev == b'/');
        } else {
            prev = b;
        }
        i += 1;
    }
    (bytes.len(), false)
}

/// Skips the XML construct starting at `start` (which points at `<`
/// followed by `!` or `?`), returning the index just past it.
fn skip_construct(bytes: &[u8], start: usize) -> usize {
    let rest = &bytes[start..];
    if rest.starts_with(b"<!--") {
        return rest
            .windows(3)
            .position(|w| w == b"-->")
            .map_or(bytes.len(), |at| start + at + 3);
    }
    if rest.starts_with(b"<![CDATA[") {
        return rest
            .windows(3)
            .position(|w| w == b"]]>")
            .map_or(bytes.len(), |at| start + at + 3);
    }
    if rest.starts_with(b"<!DOCTYPE") {
        // An internal subset `[ ... ]` may hold constructs with their
        // own `>`; skip to the subset's `]`, then the doctype's `>`.
        let mut i = 0;
        let mut subset = false;
        while i < rest.len() {
            match rest[i] {
                b'[' => subset = true,
                b']' => subset = false,
                b'>' if !subset => return start + i + 1,
                _ => {}
            }
            i += 1;
        }
        return bytes.len();
    }
    // `<?...?>` or unrecognized `<!...>`: skip to the first `>`.
    tag_end(bytes, start).0
}

/// `extra ∘ node` — apply the node's absolute transform first, then the
/// caller's (subroot) one.
fn compose_transform(extra: TsTransform, node: TsTransform) -> TsTransform {
    TsTransform::from_row(
        extra.sx * node.sx + extra.kx * node.ky,
        extra.ky * node.sx + extra.sy * node.ky,
        extra.sx * node.kx + extra.kx * node.sy,
        extra.ky * node.kx + extra.sy * node.sy,
        extra.sx * node.tx + extra.kx * node.ty + extra.tx,
        extra.ky * node.tx + extra.sy * node.ty + extra.ty,
    )
}

/// tiny-skia path segments → model contours: quads raise to cubics
/// (control points at the ⅔ marks), cubics land on the point spine
/// (`secondary` = outgoing control, `primary` = incoming control —
/// the same convention `svg::path_data` reads back out).
fn segments_to_contours(data: &usvg::tiny_skia_path::Path) -> Vec<Contour> {
    use usvg::tiny_skia_path::PathSegment as Segment;
    let mut contours: Vec<Contour> = Vec::new();
    let mut current: Vec<PathPoint> = Vec::new();
    let mut last: Option<TsPoint> = None;
    for segment in data.segments() {
        match segment {
            Segment::MoveTo(at) => {
                if !current.is_empty() {
                    contours.push(finish_contour(&mut current));
                }
                current = vec![PathPoint {
                    anchor: to_point(at),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                }];
                last = Some(at);
            }
            Segment::LineTo(at) => {
                current.push(PathPoint {
                    anchor: to_point(at),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                });
                last = Some(at);
            }
            Segment::QuadTo(control, to) => {
                // p0 + ⅔(q − p0) out, p + ⅔(q − p) in — the standard
                // quad→cubic elevation.
                let Some(from) = last else { continue };
                let two_thirds = |a: f32, b: f32| a + (b - a) * 2.0 / 3.0;
                if let Some(previous) = current.last_mut() {
                    previous.secondary = Some(geom::Point::new(
                        f64::from(two_thirds(from.x, control.x)),
                        f64::from(two_thirds(from.y, control.y)),
                    ));
                    previous.is_curve = true;
                }
                current.push(PathPoint {
                    anchor: to_point(to),
                    primary: Some(geom::Point::new(
                        f64::from(two_thirds(to.x, control.x)),
                        f64::from(two_thirds(to.y, control.y)),
                    )),
                    secondary: None,
                    is_curve: true,
                });
                last = Some(to);
            }
            Segment::CubicTo(control_out, control_in, to) => {
                if let Some(previous) = current.last_mut() {
                    previous.secondary = Some(geom::Point::new(
                        f64::from(control_out.x),
                        f64::from(control_out.y),
                    ));
                    previous.is_curve = true;
                }
                current.push(PathPoint {
                    anchor: to_point(to),
                    primary: Some(geom::Point::new(
                        f64::from(control_in.x),
                        f64::from(control_in.y),
                    )),
                    secondary: None,
                    is_curve: true,
                });
                last = Some(to);
            }
            Segment::Close => {
                if !current.is_empty() {
                    let mut contour = finish_contour(&mut current);
                    contour.closed = true;
                    contours.push(contour);
                    current = Vec::new();
                    last = None;
                }
            }
        }
    }
    if !current.is_empty() {
        contours.push(finish_contour(&mut current));
    }
    contours
}

fn finish_contour(current: &mut Vec<PathPoint>) -> Contour {
    let point_count = current.len();
    Contour {
        closed: false,
        point_count: i64::try_from(point_count).unwrap_or(i64::MAX),
        points: std::mem::take(current),
    }
}

fn to_point(value: TsPoint) -> geom::Point {
    geom::Point::new(f64::from(value.x), f64::from(value.y))
}

/// `Color × paint-opacity` → `FCLi` ABGR raw: usvg colors are opaque,
/// the paint opacity is the alpha channel.
fn paint_raw(color: &usvg::Color, opacity: usvg::Opacity) -> i64 {
    let alpha = (f64::from(opacity.get()) * 255.0).round() as i64;
    (alpha << 24)
        | (i64::from(color.blue) << 16)
        | (i64::from(color.green) << 8)
        | i64::from(color.red)
}

fn paint_kind(paint: &usvg::Paint) -> &'static str {
    match paint {
        usvg::Paint::Color(_) => "color",
        usvg::Paint::LinearGradient(_) => "linear-gradient",
        usvg::Paint::RadialGradient(_) => "radial-gradient",
        usvg::Paint::Pattern(_) => "pattern",
    }
}

/// usvg's blend enum → the rasterizer's mix vocabulary (both enumerate
/// the same 16 W3C modes; only the names differ).
fn svg_blend(mode: usvg::BlendMode) -> Mix {
    match mode {
        usvg::BlendMode::Normal => Mix::Normal,
        usvg::BlendMode::Multiply => Mix::Multiply,
        usvg::BlendMode::Screen => Mix::Screen,
        usvg::BlendMode::Overlay => Mix::Overlay,
        usvg::BlendMode::Darken => Mix::Darken,
        usvg::BlendMode::Lighten => Mix::Lighten,
        usvg::BlendMode::ColorDodge => Mix::ColorDodge,
        usvg::BlendMode::ColorBurn => Mix::ColorBurn,
        usvg::BlendMode::HardLight => Mix::HardLight,
        usvg::BlendMode::SoftLight => Mix::SoftLight,
        usvg::BlendMode::Difference => Mix::Difference,
        usvg::BlendMode::Exclusion => Mix::Exclusion,
        usvg::BlendMode::Hue => Mix::Hue,
        usvg::BlendMode::Saturation => Mix::Saturation,
        usvg::BlendMode::Color => Mix::Color,
        usvg::BlendMode::Luminosity => Mix::Luminosity,
    }
}

/// `Opacity` (f32 0–1) → the model's per-mille integer.
fn per_mille(opacity: usvg::Opacity) -> i64 {
    (f64::from(opacity.get()) * 1000.0).round() as i64
}

fn non_empty(id: &str) -> Option<String> {
    (!id.is_empty()).then(|| id.to_owned())
}
