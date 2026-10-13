//! SVG 1.1 export: a [`doc::Document`] page/state → deterministic SVG
//! text, element-for-element mirroring `compose::render`'s semantics so
//! the two renderers stay comparable (the `xtask golden` resvg loop).
//!
//! The mapping is read off the compositor, not re-invented: blend
//! values go through [`compose::blend_mode`] (the dictionary table
//! stays in one place; unknown values fall back to `normal` exactly
//! where compose falls back to `Mix::Normal`), live-effect stacks go
//! through [`compose::effects::plan`], and the per-channel kernels map
//! to `feComponentTransfer` tables derived by running the *real*
//! `filters` kernels on a 256-step ramp — exact LUTs with zero
//! duplicated math. Unmappable kernels drop the object's whole filter
//! (compose's own behavior for uninterpretable stacks) and land in the
//! report, never silently.
//!
//! Coverage per compose's M4/M7 surface: paths (fill/stroke/`FOTb`),
//! groups, per-object opacity/blend, the mapped filter set. Text emits
//! a real `<text>` — pixel fidelity then depends on the viewer's font
//! stack, because our own rasterization lowers glyphs through the
//! `text` crate (the golden loop accounts for this). Images embed as
//! data-URI PNGs. Slices render as nothing (compose parity); symbol
//! instances compose *does* draw (decision 0014), so their absence is
//! a recorded divergence — both are reported, never silent.

use base64::Engine as _;

use compose::{KernelOp, Mix, blend_mode, plan, straight_alpha};
use doc::{Document, FillShape, Gradient, GradientStop, Layer, ObjectData, PathObject, TileRef};
use filters::Surface;

use crate::{ExportError, Plane};

/// Which page/state to export.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SvgOptions {
    /// Page index into `document.pages`.
    pub page: usize,
    /// Frame (state) index.
    pub frame: usize,
}

/// Everything the exporter did *besides* the happy path — the
/// machine-checkable answer to "what did this SVG not carry?"
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SvgReport {
    /// Objects and effects dropped or approximated, with reasons, in
    /// walk order.
    pub notes: Vec<String>,
    /// Text objects emitted as SVG `<text>` (compose lowers glyphs —
    /// pixel parity depends on the viewer's font stack).
    pub text_objects: usize,
    /// Image objects embedded as data-URI PNGs (compose renders images
    /// as nothing yet — an SVG/compose render pair diverges on these).
    pub image_objects: usize,
    /// Filters dropped whole because one op mapped to no primitive.
    pub dropped_filters: usize,
    /// Paths painted from a modeled `FPLv` gradient def. compose
    /// renders these (the svg-golden gate holds the two rasters
    /// together); the importer drops gradient paint servers today, so
    /// the round-trip gate skips such documents — recorded limitation,
    /// not silent loss.
    pub gradient_fills: usize,
}

/// The SVG text plus its report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgDocument {
    /// The SVG 1.1 document text.
    pub svg: String,
    /// What was skipped or approximated.
    pub report: SvgReport,
}

/// Serializes one page in one state to SVG.
///
/// # Errors
/// [`ExportError::Unsupported`] for a page index outside the document
/// or a canvas that rounds to fewer than one pixel,
/// [`ExportError::TooLarge`] beyond the u16 raster extent, and
/// [`ExportError::Encode`] when an embedded image cannot be encoded.
pub fn encode(document: &Document, options: &SvgOptions) -> Result<SvgDocument, ExportError> {
    let page = document
        .pages
        .get(options.page)
        .ok_or_else(|| ExportError::Unsupported {
            format: "svg",
            reason: format!("page {} is outside the document", options.page),
        })?;
    let canvas = &page.info.canvas;
    let width = canvas.width.round();
    let height = canvas.height.round();
    if width < 1.0 || height < 1.0 {
        return Err(ExportError::Unsupported {
            format: "svg",
            reason: "canvas rounds to fewer than one pixel on an axis".to_owned(),
        });
    }
    if width > f64::from(u16::MAX) || height > f64::from(u16::MAX) {
        return Err(ExportError::TooLarge {
            format: "svg",
            width: width as u32,
            height: height as u32,
            limit: u32::from(u16::MAX),
        });
    }
    let (width, height) = (width as u16, height as u16);

    let mut writer = SvgWriter {
        out: String::new(),
        defs: String::new(),
        report: SvgReport::default(),
        document,
        canvas: (f64::from(width), f64::from(height)),
        filter_seq: 0,
        gradient_seq: 0,
    };
    writer.out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    // The canvas background (`BGCi`): the same matte compose paints,
    // so a viewer's raster matches the renderer byte-for-byte at the
    // bare pixels. Alpha byte ignored (unpinned; `bg_hex` precedent).
    let bg = page.info.canvas.background_raw as u32;
    writer.out.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" fill=\"#{:02x}{:02x}{:02x}\"/>\n",
        bg & 0xFF,
        (bg >> 8) & 0xFF,
        (bg >> 16) & 0xFF
    ));
    writer.walk_layers(&page.layers.layers, options.frame);
    if !writer.defs.is_empty() {
        writer.out.push_str("<defs>\n");
        writer.out.push_str(&std::mem::take(&mut writer.defs));
        writer.out.push_str("</defs>\n");
    }
    writer.out.push_str("</svg>\n");
    Ok(SvgDocument {
        svg: writer.out,
        report: writer.report,
    })
}

struct SvgWriter<'a> {
    out: String,
    defs: String,
    report: SvgReport,
    document: &'a Document,
    canvas: (f64, f64),
    filter_seq: usize,
    gradient_seq: usize,
}

impl SvgWriter<'_> {
    fn walk_layers(&mut self, layers: &[Layer], frame: usize) {
        for (index, layer) in layers.iter().enumerate() {
            if !layer.display {
                continue;
            }
            if let Some(flags) = &layer.visibility_per_frame {
                if !flags.get(frame).copied().unwrap_or(true) {
                    continue;
                }
            }
            let Some(cell) = layer.cells.get(frame) else {
                continue;
            };
            if !cell.visible {
                continue;
            }
            self.out
                .push_str(&format!("<g id=\"layer-{}\">\n", index + 1));
            for id in &cell.elements {
                self.write_object(*id);
            }
            self.out.push_str("</g>\n");
        }
    }

    fn write_object(&mut self, id: doc::ObjectId) {
        match self.document.registry.get(id) {
            Some(ObjectData::Path(path)) => self.write_path(path),
            Some(ObjectData::Group(group)) => {
                let opened = self.open_wrapper(group.opacity, group.blend, &group.effects);
                // Mask-structured groups composite children unmasked
                // (compose parity), so `mask_hidden` changes nothing.
                for child in &group.children {
                    self.write_object(*child);
                }
                self.close_wrapper(opened);
            }
            Some(ObjectData::Text(text)) => self.write_text(text),
            Some(ObjectData::Image(image)) => self.write_image(image),
            // Slices render as nothing in both stacks (compose parity);
            // symbol instances compose *does* draw (decision 0014), so
            // the gap is a recorded divergence, not parity.
            Some(ObjectData::Slice(_)) => self
                .report
                .notes
                .push("slice renders as nothing (compose parity)".to_owned()),
            Some(ObjectData::SymbolInstance(_)) => self
                .report
                .notes
                .push("symbol instance renders as nothing (compose draws it)".to_owned()),
            // A dangling element id draws nothing; registry integrity
            // is the document layer's contract, not the serializer's.
            None => {}
        }
    }

    /// Opens the blend/opacity/filter wrapper a group paints through,
    /// returning whether an element was actually opened — a pass-through
    /// group (default attrs, no filter) emits nothing and must not be
    /// closed either.
    /// Filter drop notes are the filter builder's; the wrapper carries
    /// only what remains.
    fn open_wrapper(&mut self, opacity: i64, blend: i64, effects: &[doc::Effect]) -> bool {
        let filter = self.write_filter(effects);
        let mode = mix_name(blend_mode(blend));
        let attrs = wrap_attrs(opacity, mode, filter.as_deref());
        if attrs.is_empty() {
            return false;
        }
        self.out.push_str(&format!("<g{attrs}>\n"));
        true
    }

    fn close_wrapper(&mut self, opened: bool) {
        if opened {
            self.out.push_str("</g>\n");
        }
    }

    fn write_path(&mut self, path: &PathObject) {
        if !path.visible {
            return;
        }
        let Some(paint) = &path.paint else {
            self.report
                .notes
                .push("path without paint renders as nothing (compose parity)".to_owned());
            return;
        };
        let filter = self.write_filter(&path.effects);
        let mode = mix_name(blend_mode(path.blend));
        let attrs = wrap_attrs(path.opacity, mode, filter.as_deref());

        let fill = self.fill_paint(path, paint);
        let stroke = paint_attr("stroke", paint.stroke_raw);
        let rule = if path.even_odd { "evenodd" } else { "nonzero" };
        let d = path_data(&path.contours);
        let element = if path.fill_on_top == Some(true) {
            // Stroke first, fill over it: one element cannot order its
            // own paint, so two paths ride the wrapper.
            let mut both = String::new();
            if stroke != "stroke=\"none\"" {
                both.push_str(&format!(
                    "<path d=\"{d}\" fill=\"none\" {stroke} stroke-width=\"1\"/>\n"
                ));
            }
            if fill != "fill=\"none\"" {
                both.push_str(&format!(
                    "<path d=\"{d}\" {fill} fill-rule=\"{rule}\" stroke=\"none\"/>\n"
                ));
            }
            both
        } else {
            format!("<path d=\"{d}\" {fill} fill-rule=\"{rule}\" {stroke} stroke-width=\"1\"/>\n")
        };
        if attrs.is_empty() {
            self.out.push_str(&element);
        } else {
            self.out.push_str(&format!("<g{attrs}>\n{element}</g>\n"));
        }
    }

    /// The fill paint for one path: a modeled `FPLv` gradient becomes a
    /// `linearGradient`/`radialGradient` def plus a `url(#…)` reference.
    /// The kind mapping is compose's `fill_brush` verbatim — the
    /// svg-golden gate rasters the two against each other, so both must
    /// agree: linear rides the start→end fill vector, radial/elliptical
    /// is a circle centered on the start handle (the third handle's
    /// ellipse squash is a recorded substitution), and every family
    /// compose substitutes to the solid `FCLi` colour exports solid too.
    /// The conical sweep is the one asymmetry: compose renders it
    /// natively, SVG has no sweep primitive, so it drops to the solid
    /// colour under a recorded note (the encoder contract: nothing
    /// diverges silently).
    fn fill_paint(&mut self, path: &PathObject, paint: &doc::PaintStyle) -> String {
        let solid = paint_attr("fill", paint.fill_raw);
        let Some(style) = &paint.fill else {
            return solid;
        };
        let Some(gradient) = &style.gradient else {
            return solid;
        };
        let (Some(start), Some(end)) = (path.fill_start, path.fill_end) else {
            // Without both handles compose paints the solid colour too.
            return solid;
        };
        let (sx, sy, ex, ey) = (start.x, start.y, end.x, end.y);
        let (element, geometry) = match style.shape {
            FillShape::Linear => (
                "linearGradient",
                format!(
                    "x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
                    num(sx),
                    num(sy),
                    num(ex),
                    num(ey)
                ),
            ),
            FillShape::Radial | FillShape::Elliptical => {
                let radius = ((ex - sx) * (ex - sx) + (ey - sy) * (ey - sy)).sqrt();
                (
                    "radialGradient",
                    format!(
                        "cx=\"{}\" cy=\"{}\" r=\"{}\"",
                        num(sx),
                        num(sy),
                        num(radius)
                    ),
                )
            }
            FillShape::Conical => {
                self.report
                    .notes
                    .push("conical gradient exports as the solid colour (SVG has no sweep primitive; compose renders it)".to_owned());
                return solid;
            }
            _ => return solid,
        };
        let Some(stops) = gradient_stops(gradient, paint.fill_raw) else {
            return solid;
        };
        self.gradient_seq += 1;
        let id = format!("grad-{}", self.gradient_seq);
        self.defs.push_str(&format!(
            "<{element} id=\"{id}\" gradientUnits=\"userSpaceOnUse\" {geometry} spreadMethod=\"pad\">\n"
        ));
        for (position, color_raw, alpha) in stops {
            let (r, g, b, _) = decode_abgr(color_raw);
            self.defs.push_str(&format!(
                "<stop offset=\"{}\" stop-color=\"#{r:02x}{g:02x}{b:02x}\" stop-opacity=\"{}\"/>\n",
                num(position),
                num(alpha),
            ));
        }
        self.defs.push_str(&format!("</{element}>\n"));
        self.report.gradient_fills += 1;
        format!("fill=\"url(#{id})\"")
    }

    fn write_text(&mut self, text: &doc::TextObject) {
        if !text.visible {
            return;
        }
        let filter = self.write_filter(&text.effects);
        let mode = mix_name(blend_mode(text.blend));
        let attrs = wrap_attrs(text.opacity, mode, filter.as_deref());
        self.report.notes.push(
            "text exports as SVG text; pixel fidelity depends on the viewer's font stack"
                .to_owned(),
        );
        self.report.text_objects += 1;
        // compose lowers glyphs through the `text` crate and fills
        // them; `FCLi` overrides, opaque black is the Fireworks
        // default.
        const DEFAULT_FILL: i64 = 0xFF00_0000;
        let fill = text
            .style
            .as_ref()
            .and_then(|style| style.fill_raw)
            .unwrap_or(DEFAULT_FILL);
        let fill = paint_attr("fill", fill);

        let body: String = text.runs.iter().map(|run| escape(&run.text)).collect();
        let mut font = String::new();
        if let Some(style) = &text.style {
            if let Some(family) = &style.font {
                font.push_str(&format!(" font-family=\"{}\"", escape(family)));
            }
            if let Some(size) = style.size {
                font.push_str(&format!(" font-size=\"{}\"", num(size)));
            }
            if style.bold == Some(true) {
                font.push_str(" font-weight=\"bold\"");
            }
            if style.italic == Some(true) {
                font.push_str(" font-style=\"italic\"");
            }
        }
        self.out.push_str(&format!("<g{attrs}>\n"));
        self.out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" dominant-baseline=\"text-before-edge\"{font} {fill}>{body}</text>\n",
            num(text.frame.x0),
            num(text.frame.y0),
        ));
        self.out.push_str("</g>\n");
    }

    fn write_image(&mut self, image: &doc::ImageObject) {
        if !image.visible {
            return;
        }
        self.report.notes.push(
            "image embedded as a data-URI PNG (compose does not render images yet)".to_owned(),
        );
        self.report.image_objects += 1;
        let png = match self.image_png(image) {
            Ok(png) => png,
            Err(error) => {
                self.report
                    .notes
                    .push(format!("image dropped, embed failed: {error}"));
                return;
            }
        };
        let filter = self.write_filter(&image.effects);
        let mode = mix_name(blend_mode(image.blend));
        let attrs = wrap_attrs(image.opacity, mode, filter.as_deref());
        let (x, y) = (
            image.location.x + image.offset.x,
            image.location.y + image.offset.y,
        );
        self.out.push_str(&format!("<g{attrs}>\n"));
        self.out.push_str(&format!(
            "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"data:image/png;base64,{}\"/>\n",
            num(x),
            num(y),
            num(image.pixels.0 as f64),
            num(image.pixels.1 as f64),
            base64::engine::general_purpose::STANDARD.encode(png),
        ));
        self.out.push_str("</g>\n");
    }

    /// Gathers an image's tiles into a straight-alpha RGBA plane and
    /// encodes it as PNG (the same house un-premultiply
    /// `compose::straight_alpha` uses everywhere else).
    fn image_png(&self, image: &doc::ImageObject) -> Result<Vec<u8>, ExportError> {
        let width = image.pixels.0.max(0) as usize;
        let height = image.pixels.1.max(0) as usize;
        let tile_side = image.tile_size.max(1) as usize;
        let expected = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| ExportError::Encode {
                format: "svg",
                reason: "image size overflows usize".to_owned(),
            })?;
        let (cols, rows) = (width.div_ceil(tile_side), height.div_ceil(tile_side));
        if width == 0 || height == 0 || image.tiles.len() != cols * rows {
            return Err(ExportError::Encode {
                format: "svg",
                reason: format!(
                    "image is {width}x{height} over {tile_side}px tiles but carries {} tile refs",
                    image.tiles.len()
                ),
            });
        }
        let mut premultiplied = vec![0u8; expected];
        for (index, tile_ref) in image.tiles.iter().enumerate() {
            let (col, row) = (index % cols, index / cols);
            let tile_bytes: Vec<u8> = match tile_ref {
                TileRef::Tile(id) => {
                    let Some(tile) = self.document.tiles.get(*id) else {
                        return Err(ExportError::Encode {
                            format: "svg",
                            reason: format!("tile {id} is missing from the bank"),
                        });
                    };
                    tile.pixels().to_vec()
                }
                TileRef::Matte(raw) => {
                    let (r, g, b, a) = decode_abgr(*raw);
                    let mut bytes = Vec::with_capacity(tile_side * tile_side * 4);
                    for _ in 0..tile_side * tile_side {
                        bytes.extend_from_slice(&[r, g, b, a]);
                    }
                    bytes
                }
            };
            for py in 0..tile_side {
                let y = row * tile_side + py;
                if y >= height {
                    break;
                }
                for px in 0..tile_side {
                    let x = col * tile_side + px;
                    if x >= width {
                        break;
                    }
                    let src = (py * tile_side + px) * 4;
                    let dst = (y * width + x) * 4;
                    premultiplied[dst..dst + 4].copy_from_slice(&tile_bytes[src..src + 4]);
                }
            }
        }
        let straight = straight_alpha(&premultiplied);
        let plane = Plane::new(width as u32, height as u32, &straight).map_err(|error| {
            ExportError::Encode {
                format: "svg",
                reason: error.to_string(),
            }
        })?;
        crate::png::encode(&plane)
    }

    /// Emits a `<filter>` for an interpretable effect stack, or `None`
    /// when the stack is empty or dropped whole (any unmappable op
    /// poisons the chain — compose applies all its ops, and a partial
    /// chain would lie about which math ran). Drop reasons land in the
    /// report here, so callers never repeat them.
    fn write_filter(&mut self, effects: &[doc::Effect]) -> Option<String> {
        let interpreted = plan(effects);
        if interpreted.ops.is_empty() {
            return None;
        }
        let mut primitives = String::new();
        let mut unmapped: Vec<String> = Vec::new();
        for op in &interpreted.ops {
            match op {
                KernelOp::BrightnessContrast {
                    brightness,
                    contrast,
                } => {
                    if let Some(transfer) = lut_transfer(|| {
                        ramp_lut(|surface| {
                            filters::brightness_contrast(surface, *brightness, *contrast)
                        })
                    }) {
                        primitives.push_str(&transfer);
                    } else {
                        unmapped.push(kernel_name(op).to_owned());
                    }
                }
                KernelOp::Levels {
                    master,
                    red,
                    green,
                    blue,
                } => {
                    // The gray ramp hits each channel with its own
                    // override-else-master LUT, so the three exported
                    // tables carry the per-channel curves directly.
                    if let Some(transfer) = lut_transfer(|| {
                        ramp_lut(|surface| {
                            let pick = |over: &Option<filters::LevelsControls>| match over {
                                Some(c) => *c,
                                None => *master,
                            };
                            filters::levels_channels(surface, pick(red), pick(green), pick(blue))
                        })
                    }) {
                        primitives.push_str(&transfer);
                    } else {
                        unmapped.push(kernel_name(op).to_owned());
                    }
                }
                KernelOp::Invert => match lut_transfer(|| ramp_lut(filters::invert)) {
                    Some(transfer) => primitives.push_str(&transfer),
                    None => unmapped.push(kernel_name(op).to_owned()),
                },
                KernelOp::GaussianBlur { radius } => {
                    primitives.push_str(&format!(
                        "<feGaussianBlur stdDeviation=\"{}\"/>\n",
                        num(*radius)
                    ));
                }
                KernelOp::BoxBlur { radius } => {
                    let side = 2 * (*radius) + 1;
                    let cells = i64::from(side * side);
                    let matrix = std::iter::repeat_n("1", (side * side) as usize)
                        .collect::<Vec<_>>()
                        .join(" ");
                    primitives.push_str(&format!(
                        "<feConvolveMatrix order=\"{side}\" kernelMatrix=\"{matrix}\" divisor=\"{cells}\" edgeMode=\"duplicate\" preserveAlpha=\"false\"/>\n"
                    ));
                }
                KernelOp::UnsharpMask {
                    radius,
                    amount,
                    threshold,
                } if *threshold == 0 => {
                    // out = orig + amount% · (orig − blur), as an
                    // arithmetic composite over the blurred input. A
                    // raised threshold cannot be expressed here — it
                    // falls through to the poison arm below rather
                    // than exporting a filter that overshoots flat
                    // areas.
                    let a = f64::from((*amount).min(500)) / 100.0;
                    primitives.push_str(&format!(
                        "<feGaussianBlur stdDeviation=\"{}\" result=\"blur\"/><feComposite operator=\"arithmetic\" in=\"SourceGraphic\" in2=\"blur\" k1=\"0\" k2=\"{}\" k3=\"{}\" k4=\"0\"/>\n",
                        num(*radius),
                        num(1.0 + a),
                        num(-a),
                    ));
                }
                KernelOp::Curves { knots } => {
                    if let Some(transfer) =
                        lut_transfer(|| ramp_lut(|surface| filters::curves(surface, knots)))
                    {
                        primitives.push_str(&transfer);
                    } else {
                        unmapped.push(kernel_name(op).to_owned());
                    }
                }
                KernelOp::Sharpen | KernelOp::SharpenMore => {
                    let matrix = if matches!(op, KernelOp::Sharpen) {
                        "0 -1 0 -1 5 -1 0 -1 0"
                    } else {
                        "-1 -1 -1 -1 9 -1 -1 -1 -1"
                    };
                    primitives.push_str(&format!(
                        "<feConvolveMatrix order=\"3\" kernelMatrix=\"{matrix}\" divisor=\"1\" edgeMode=\"duplicate\" preserveAlpha=\"false\"/>\n"
                    ));
                }
                other => unmapped.push(kernel_name(other).to_owned()),
            }
        }
        if !unmapped.is_empty() {
            self.report.dropped_filters += 1;
            for effect in effects {
                self.report.notes.push(format!(
                    "filter '{}' dropped: {} maps to no SVG primitive",
                    effect.internal_name,
                    unmapped.join(", ")
                ));
            }
            return None;
        }
        let id = format!("fc{}", self.filter_seq);
        self.filter_seq += 1;
        // Full-canvas filter region: compose rasterizes filtered
        // objects over a full-canvas offscreen, so blur spread reaches
        // the canvas edge (decision 0005) — the SVG region matches.
        self.defs.push_str(&format!(
            "<filter id=\"{id}\" filterUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" color-interpolation-filters=\"sRGB\">\n{primitives}</filter>\n",
            num(self.canvas.0),
            num(self.canvas.1),
        ));
        Some(id)
    }
}

/// Wrapper attributes: opacity per mille → SVG opacity, blend →
/// mix-blend-mode, filter → url reference. Empty when everything is
/// default.
fn wrap_attrs(opacity: i64, mode: &str, filter: Option<&str>) -> String {
    let mut attrs = String::new();
    if opacity != 1000 {
        attrs.push_str(&format!(" opacity=\"{}\"", num(opacity as f64 / 1000.0)));
    }
    if mode != "normal" {
        attrs.push_str(&format!(" style=\"mix-blend-mode:{mode}\""));
    }
    if let Some(filter) = filter {
        attrs.push_str(&format!(" filter=\"url(#{filter})\""));
    }
    attrs
}

/// `fill`/`stroke` presentation for an ABGR color int: `none` at
/// alpha 0 (compose paints nothing visible), `#rrggbb` plus a
/// `-opacity` attribute otherwise.
fn paint_attr(kind: &str, raw: i64) -> String {
    let (r, g, b, a) = decode_abgr(raw);
    if a == 0 {
        return format!("{kind}=\"none\"");
    }
    if a == 255 {
        return format!("{kind}=\"#{r:02x}{g:02x}{b:02x}\"");
    }
    format!(
        "{kind}=\"#{r:02x}{g:02x}{b:02x}\" {kind}-opacity=\"{}\"",
        num(a as f64 / 255.0)
    )
}

/// ABGR layout per the node dictionary (`FCLi`/`BCLi`).
fn decode_abgr(raw: i64) -> (u8, u8, u8, u8) {
    let raw = raw as u32;
    (
        (raw & 0xFF) as u8,
        ((raw >> 8) & 0xFF) as u8,
        ((raw >> 16) & 0xFF) as u8,
        ((raw >> 24) & 0xFF) as u8,
    )
}

/// Stop density when a coverage ramp rides along (see
/// [`gradient_stops`]).
const GRADIENT_SAMPLES: usize = 256;

/// Merges the `FG0v` RGB ramp and the `FG1v` coverage ramp into SVG
/// `(offset, stop-color, stop-opacity)` triples, mirroring compose's
/// `gradient_stops`: every sample point evaluates both ramps
/// independently and coverage rides `stop-opacity`.
///
/// Sample points are the ramp knots when the gradient is opaque —
/// straight and premultiplied stop interpolation agree there, so the
/// sparse form is exact and the SVG stays small. With a coverage ramp
/// the stops densify onto a uniform grid: SVG viewers interpolate
/// stop pairs in different spaces (straight per SVG 1.1, premultiplied
/// per CSS Images — resvg does the latter), and over long transparent
/// stretches that choice visibly shifts colour, not just alpha. At
/// 1/256 spacing any viewer reconstructs compose's independent-ramp
/// values within the AA noise the svg-golden gate measures.
fn gradient_stops(gradient: &Gradient, solid_raw: i64) -> Option<Vec<(f64, i64, f64)>> {
    if gradient.rgb.is_empty() && gradient.alpha.is_empty() {
        return None;
    }
    let mut positions: Vec<f64> = if gradient.alpha.is_empty() {
        gradient.rgb.iter().map(|stop| stop.position).collect()
    } else {
        (0..=GRADIENT_SAMPLES)
            .map(|step| f64::from(step as u32) / f64::from(GRADIENT_SAMPLES as u32))
            .chain(
                gradient
                    .rgb
                    .iter()
                    .chain(gradient.alpha.iter())
                    .map(|stop| stop.position),
            )
            .collect()
    };
    positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    positions.dedup();

    let mut stops = Vec::with_capacity(positions.len());
    for position in positions {
        let color_raw = sample_ramp(&gradient.rgb, position, solid_raw);
        // An empty coverage ramp is fully opaque; otherwise the `FGCi`
        // alpha byte is the coverage.
        let alpha = if gradient.alpha.is_empty() {
            1.0
        } else {
            let (_, _, _, a) = decode_abgr(sample_ramp(&gradient.alpha, position, 0xFF00_0000));
            f64::from(a) / 255.0
        };
        stops.push((position, color_raw, alpha));
    }
    Some(stops)
}

/// Piecewise-linear sample of one ramp at `position` in ABGR layout;
/// outside the ends the nearest stop holds. Parity twin of compose's
/// `sample_stop` — the svg-golden gate keeps the two honest.
fn sample_ramp(ramp: &[GradientStop], position: f64, edge: i64) -> i64 {
    let Some(first) = ramp.first() else {
        return edge;
    };
    if position <= first.position {
        return first.color_raw;
    }
    let Some(last) = ramp.last() else {
        return edge;
    };
    if position >= last.position {
        return last.color_raw;
    }
    for pair in ramp.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if position <= b.position && b.position > a.position {
            let t = (position - a.position) / (b.position - a.position);
            let mix = |x: i64, y: i64| -> i64 {
                (f64::from(x as i32) * (1.0 - t) + f64::from(y as i32) * t + 0.5) as i64
            };
            let (ar, ag, ab, aa) = decode_abgr(a.color_raw);
            let (br, bg, bb, ba) = decode_abgr(b.color_raw);
            let (ar, ag, ab, aa) = (i64::from(ar), i64::from(ag), i64::from(ab), i64::from(aa));
            let (br, bg, bb, ba) = (i64::from(br), i64::from(bg), i64::from(bb), i64::from(ba));
            return mix(ar, br) | mix(ag, bg) << 8 | mix(ab, bb) << 16 | mix(aa, ba) << 24;
        }
    }
    edge
}

/// Rasterizer mix → SVG mix-blend-mode name. The value table itself is
/// compose's; undocumented values never reach here (they arrive as
/// `Mix::Normal`).
#[must_use]
fn mix_name(mix: Mix) -> &'static str {
    match mix {
        Mix::Normal => "normal",
        Mix::Multiply => "multiply",
        Mix::Screen => "screen",
        Mix::Overlay => "overlay",
        Mix::Darken => "darken",
        Mix::Lighten => "lighten",
        Mix::ColorDodge => "color-dodge",
        Mix::ColorBurn => "color-burn",
        Mix::HardLight => "hard-light",
        Mix::SoftLight => "soft-light",
        Mix::Difference => "difference",
        Mix::Exclusion => "exclusion",
        Mix::Hue => "hue",
        Mix::Saturation => "saturation",
        Mix::Color => "color",
        Mix::Luminosity => "luminosity",
    }
}

/// Human-readable kernel names for the drop report.
fn kernel_name(op: &KernelOp) -> &'static str {
    match op {
        KernelOp::BrightnessContrast { .. } => "brightness/contrast",
        KernelOp::Levels { .. } => "levels",
        KernelOp::Invert => "invert",
        KernelOp::HueSaturation { .. } => "hue/saturation",
        KernelOp::GaussianBlur { .. } => "gaussian blur",
        KernelOp::BoxBlur { .. } => "blur",
        KernelOp::UnsharpMask { .. } => "unsharp mask",
        KernelOp::Curves { .. } => "curves",
        KernelOp::AutoLevels => "auto levels",
        KernelOp::ColorFill { .. } => "color fill",
        KernelOp::MotionBlur { .. } => "motion blur",
        KernelOp::RadialBlur { .. } => "radial blur",
        KernelOp::ZoomBlur { .. } => "zoom blur",
        KernelOp::Sharpen => "sharpen",
        KernelOp::SharpenMore => "sharpen more",
        KernelOp::ConvertToAlpha => "convert to alpha",
        KernelOp::AddNoise { .. } => "add noise",
        KernelOp::FindEdges => "find edges",
        KernelOp::DropShadow { .. } => "drop shadow",
        KernelOp::InnerShadow { .. } => "inner shadow",
        KernelOp::SolidShadow { .. } => "solid shadow",
        KernelOp::OuterGlow { .. } => "outer glow",
        KernelOp::InnerGlow { .. } => "inner glow",
        KernelOp::Bevel { .. } => "bevel emboss",
        KernelOp::Clouds { .. } => "clouds",
        KernelOp::DifferenceClouds { .. } => "difference clouds",
    }
}

/// A per-channel `feComponentTransfer` with the same 256-entry table
/// on R, G and B (alpha passes through — the identity these kernels
/// keep). `None` when the LUT could not be derived.
fn lut_transfer(lut: impl FnOnce() -> Option<[[u8; 256]; 3]>) -> Option<String> {
    let [r, g, b] = lut()?;
    let table = |channel: &[u8; 256]| {
        channel
            .iter()
            .map(|&v| num(f64::from(v) / 255.0))
            .collect::<Vec<_>>()
            .join(" ")
    };
    Some(format!(
        "<feComponentTransfer>\n<feFuncR type=\"table\" tableValues=\"{}\"/>\n<feFuncG type=\"table\" tableValues=\"{}\"/>\n<feFuncB type=\"table\" tableValues=\"{}\"/>\n</feComponentTransfer>\n",
        table(&r),
        table(&g),
        table(&b)
    ))
}

/// Runs a per-channel kernel over a 256-step opaque gray ramp and
/// reads the per-channel output tables. The ramp is opaque, so
/// premultiplied == straight and the LUT is the kernel's exact
/// transfer curve — no formula duplication, exact by construction.
fn ramp_lut(kernel: impl Fn(&Surface) -> Surface) -> Option<[[u8; 256]; 3]> {
    let mut ramp = Vec::with_capacity(256 * 4);
    for i in 0..=255u8 {
        ramp.extend_from_slice(&[i, i, i, 255]);
    }
    let surface = Surface::from_straight(256, 1, &ramp).ok()?;
    let out = kernel(&surface);
    let mut tables = [[0u8; 256]; 3];
    for (i, px) in out.pixels().chunks_exact(4).take(256).enumerate() {
        tables[0][i] = px[0];
        tables[1][i] = px[1];
        tables[2][i] = px[2];
    }
    Some(tables)
}

/// Contours → SVG path data, mirroring `compose`'s `to_bezpath`
/// segment-for-segment: one-sided bezier controls collapse onto their
/// segment endpoint, empty contours are skipped, closed contours emit
/// the closing segment then `Z`.
fn path_data(contours: &[doc::Contour]) -> String {
    let mut d = String::new();
    for contour in contours {
        let Some(first) = contour.points.first() else {
            continue;
        };
        d.push_str(&format!(
            "M {} {}",
            num(first.anchor.x),
            num(first.anchor.y)
        ));
        for pair in contour.points.windows(2) {
            segment(&mut d, &pair[0].anchor, &pair[0].secondary, &pair[1]);
        }
        if contour.closed && contour.points.len() > 1 {
            if let Some(last) = contour.points.last() {
                segment(&mut d, &last.anchor, &last.secondary, first);
            }
            d.push_str(" Z");
        }
        d.push('\n');
    }
    d.trim_end().to_owned()
}

fn segment(d: &mut String, from: &geom::Point, out: &Option<geom::Point>, to: &doc::PathPoint) {
    match (out, &to.primary) {
        (Some(out), Some(inc)) => d.push_str(&format!(
            " C {} {} {} {} {} {}",
            num(out.x),
            num(out.y),
            num(inc.x),
            num(inc.y),
            num(to.anchor.x),
            num(to.anchor.y)
        )),
        (Some(out), None) => d.push_str(&format!(
            " C {} {} {} {} {} {}",
            num(out.x),
            num(out.y),
            num(to.anchor.x),
            num(to.anchor.y),
            num(to.anchor.x),
            num(to.anchor.y)
        )),
        (None, Some(inc)) => d.push_str(&format!(
            " C {} {} {} {} {} {}",
            num(from.x),
            num(from.y),
            num(inc.x),
            num(inc.y),
            num(to.anchor.x),
            num(to.anchor.y)
        )),
        (None, None) => {
            d.push_str(&format!(" L {} {}", num(to.anchor.x), num(to.anchor.y)));
        }
    }
}

/// Deterministic shortest-round-trip float formatting (SVG accepts
/// exponent notation for tiny magnitudes).
fn num(value: f64) -> String {
    format!("{value}")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
