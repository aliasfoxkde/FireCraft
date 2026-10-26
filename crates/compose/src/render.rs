//! The document walk: page canvas → layer list → state cell → objects,
//! painted bottom-first through per-object opacity/blend layers.
//!
//! Live effects (M7): an object whose stack interprets to kernels is
//! first rasterized alone into a full-canvas offscreen context, pushed
//! through `filters`, and blitted back as an image paint — inside the
//! same per-object opacity/blend layer as the unfiltered path, so the
//! two paths differ only by the filter math. The offscreen is the full
//! canvas (not the shape's bounding box), so a blur's spread past the
//! geometry survives and margin math reduces to the canvas edge
//! (decision 0005).

use crate::cache::EffectCache;
use crate::convert::{abgr, blend_mode, to_bezpath};
use crate::effects;
use doc::{
    Document, FillShape, Gradient, GroupObject, Layer, NineSliceGuides, ObjectData, PathObject,
    SymbolInstanceObject, SymbolType, TileBank, TileRef,
};
use filters::Surface;
use geom::{Affine, transform::map_rect};
use std::fmt;
use std::sync::Arc;
use vello_cpu::kurbo::{Rect, Stroke};
use vello_cpu::peniko::color::{AlphaColor, Srgb};
use vello_cpu::peniko::{
    BlendMode, Brush, Compose, Extend, Fill, Gradient as PenikoGradient, GradientKind, ImageBrush,
    ImageQuality, ImageSampler, InterpolationAlphaSpace, LinearGradientPosition,
    RadialGradientPosition, SweepGradientPosition,
};
use vello_cpu::{ImageSource, Pixmap, RenderContext, Resources};

/// The paint type the rasterizer's `set_paint` takes: the peniko
/// brush over the CPU pipeline's own image source.
type PaintBrush = Brush<ImageBrush<ImageSource>>;

/// A finished composition: premultiplied sRGB RGBA8, row-major,
/// top-left origin. Deterministic — the same document, page and frame
/// always produce the same bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Render {
    /// Canvas width in pixels.
    pub width: u16,
    /// Canvas height in pixels.
    pub height: u16,
    /// Premultiplied RGBA8, `width * height * 4` bytes.
    pub pixels: Vec<u8>,
}

/// Why a document cannot be composed.
#[derive(Clone, Debug, PartialEq)]
pub enum ComposeError {
    /// Page index outside `document.pages`.
    PageOutOfBounds {
        /// The offending index.
        page: usize,
    },
    /// Canvas rounds to fewer than one pixel on an axis.
    CanvasEmpty,
    /// Canvas exceeds the rasterizer's `u16` extent.
    CanvasTooLarge {
        /// Canvas width in document units.
        width: f64,
        /// Canvas height in document units.
        height: f64,
    },
    /// A cell references an object id absent from the registry.
    DanglingId {
        /// Registry slot the id pointed at.
        index: u32,
        /// Generation the id carried.
        generation: u32,
    },
    /// A live-effect raster could not move through the kernel bridge.
    /// Only reachable through an internal size mismatch — kernel math
    /// itself cannot fail.
    FilterPipeline {
        /// What mismatched.
        detail: String,
    },
    /// An image object references a tile id the document's bank does
    /// not hold — a document defect (a `.fw.png` whose `mkBT` chunks
    /// were never bridged into the bank, most often), never painted
    /// as a silent hole.
    TileBankMiss {
        /// The tile id the `TIDi` slot named.
        tile: u32,
    },
    /// An image object needs a tile geometry the bank cannot back
    /// (the bank's slots are fixed 128×128).
    TileSource {
        /// What geometry was requested.
        detail: String,
    },
    /// A symbol instance references a `MSNi` the document library does
    /// not define — a document defect, never a silent hole.
    MissingDefinition {
        /// The symbol number the instance named.
        symbol: i64,
    },
    /// A definition (transitively) instances itself.
    SymbolCycle {
        /// The symbol number re-entered during the walk.
        symbol: i64,
    },
    /// The definition nesting exceeded [`MAX_SYMBOL_DEPTH`] without an
    /// explicit cycle on the stack.
    SymbolDepth {
        /// The symbol number being entered.
        symbol: i64,
        /// The depth that was exceeded.
        depth: usize,
    },
}

impl fmt::Display for ComposeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComposeError::PageOutOfBounds { page } => {
                write!(f, "page {page} is outside the document")
            }
            ComposeError::CanvasEmpty => {
                write!(f, "canvas rounds to fewer than one pixel on an axis")
            }
            ComposeError::CanvasTooLarge { width, height } => {
                write!(f, "canvas {width}x{height} exceeds the u16 raster extent")
            }
            ComposeError::DanglingId { index, generation } => {
                write!(
                    f,
                    "cell references dangling object {index} (gen {generation})"
                )
            }
            ComposeError::FilterPipeline { detail } => {
                write!(f, "live-effect pipeline failed: {detail}")
            }
            ComposeError::TileBankMiss { tile } => {
                write!(f, "image references tile {tile}, absent from the tile bank")
            }
            ComposeError::TileSource { detail } => write!(f, "image tiles unsupported: {detail}"),
            ComposeError::MissingDefinition { symbol } => {
                write!(
                    f,
                    "instance references symbol {symbol}, absent from the library"
                )
            }
            ComposeError::SymbolCycle { symbol } => {
                write!(f, "symbol {symbol} instances itself")
            }
            ComposeError::SymbolDepth { symbol, depth } => {
                write!(f, "symbol {symbol} nests past the depth ceiling ({depth})")
            }
        }
    }
}

impl std::error::Error for ComposeError {}

/// The definition nesting ceiling. A library of finite definitions can
/// only nest this deep by cycling, so hitting the ceiling is refused
/// exactly like a cycle — the walk never recurses unboundedly.
const MAX_SYMBOL_DEPTH: usize = 32;

/// Composes one page in one state (frame) to pixels. The page canvas
/// (`WIDf`/`HITf`, rounded) decides the output size; frames beyond a
/// layer's cell list render that layer as empty. Document coordinates
/// are canvas pixels (identity transform); resolution scaling is an
/// export-time concern.
///
/// Uses a throwaway effect cache — one render pays for every filter
/// once. Hosts that re-render repeatedly between edits call
/// [`render_with_cache`] instead.
pub fn render(document: &Document, page: usize, frame: usize) -> Result<Render, ComposeError> {
    render_with_cache(document, page, frame, &mut EffectCache::new())
}

/// [`render`] with the canvas matte (`BGCi`) suppressed: bare pixels
/// stay transparent where nothing paints. For the export surfaces
/// whose own model is transparency-native — the GIF animator's
/// ink-crop/disposal screen and the PSD layer records ("alpha against
/// the empty backdrop") — where a baked matte would corrupt the
/// semantics (an empty state is a 1×1 placeholder, not a white
/// canvas; a layer record carries its own alpha, not a white box).
pub fn render_transparent(
    document: &Document,
    page: usize,
    frame: usize,
) -> Result<Render, ComposeError> {
    render_inner(document, page, frame, &mut EffectCache::new(), false)
}

/// [`render`] over a caller-owned effect cache: filtered object
/// rasters survive between calls keyed on content, so unchanged
/// objects skip their kernels on the next render.
pub fn render_with_cache(
    document: &Document,
    page: usize,
    frame: usize,
    cache: &mut EffectCache,
) -> Result<Render, ComposeError> {
    render_inner(document, page, frame, cache, true)
}

/// Renders only the listed object ids, in list order (bottom-first),
/// against a transparent canvas — the bitmap-bake surface for the
/// flatten family (engine decision 0031). No canvas matte: the pixels
/// carry object coverage only, so the document's canvas color stays a
/// live property after a flatten bakes this render into tiles. An id
/// absent from the registry is a [`ComposeError::DanglingId`]; hidden
/// and slice objects contribute nothing, exactly as in the full walk.
pub fn render_subset(
    document: &Document,
    page: usize,
    frame: usize,
    ids: &[doc::ObjectId],
) -> Result<Render, ComposeError> {
    let (width, height) = canvas_extent(document, page)?;
    let mut ctx = RenderContext::new(width, height);
    let resources = Resources::new();
    let mut text_engine = None;
    let mut walk = Walk {
        document,
        frame,
        canvas: (width, height),
        cache: &mut EffectCache::new(),
        text_engine: &mut text_engine,
        ctx: &mut ctx,
        stack: &[],
    };
    for id in ids {
        render_object(*id, &mut walk)?;
    }
    finish(ctx, resources, width, height)
}

/// Rounds and range-checks a page's canvas to the rasterizer's pixel
/// extent — the shared preamble of every render entry point.
fn canvas_extent(document: &Document, page: usize) -> Result<(u16, u16), ComposeError> {
    let page = document
        .pages
        .get(page)
        .ok_or(ComposeError::PageOutOfBounds { page })?;
    let canvas = &page.info.canvas;
    let width = canvas.width.round();
    let height = canvas.height.round();
    if width < 1.0 || height < 1.0 {
        return Err(ComposeError::CanvasEmpty);
    }
    if width > f64::from(u16::MAX) || height > f64::from(u16::MAX) {
        return Err(ComposeError::CanvasTooLarge {
            width: canvas.width,
            height: canvas.height,
        });
    }
    Ok((width as u16, height as u16))
}

/// Flushes the context into a finished [`Render`].
fn finish(
    mut ctx: RenderContext,
    mut resources: Resources,
    width: u16,
    height: u16,
) -> Result<Render, ComposeError> {
    ctx.flush();
    let mut pixmap = Pixmap::new(width, height);
    ctx.render(pixmap.as_mut(), &mut resources);
    Ok(Render {
        width,
        height,
        pixels: pixmap.data_as_u8_slice().to_vec(),
    })
}

fn render_inner(
    document: &Document,
    page: usize,
    frame: usize,
    cache: &mut EffectCache,
    matte: bool,
) -> Result<Render, ComposeError> {
    let (width, height) = canvas_extent(document, page)?;
    let mut ctx = RenderContext::new(width, height);
    let resources = Resources::new();
    // The canvas background (`BGCi`) paints under everything, before
    // the layer walk — CS6's matte shows wherever no object covers.
    // The raw is ABGR like every paint slot; its alpha byte rides
    // unpinned (webexport's `bg_hex` ignores it too), so the fill is
    // forced opaque. Transparency-native exports opt out via
    // [`render_transparent`].
    if matte {
        let raw = document.pages[page].info.canvas.background_raw as u32;
        ctx.set_paint(Brush::Solid(AlphaColor::from_rgba8(
            (raw & 0xFF) as u8,
            ((raw >> 8) & 0xFF) as u8,
            ((raw >> 16) & 0xFF) as u8,
            255,
        )));
        ctx.fill_rect(&Rect::new(0.0, 0.0, f64::from(width), f64::from(height)));
    }
    // The text shaping engine builds lazily — documents without text
    // never pay for the font database.
    let mut text_engine = None;
    let page_layers = &document.pages[page].layers.layers;
    walk_layers(
        page_layers,
        &mut Walk {
            document,
            frame,
            canvas: (width, height),
            cache,
            text_engine: &mut text_engine,
            ctx: &mut ctx,
            stack: &[],
        },
        true,
    )?;
    finish(ctx, resources, width, height)
}

/// Converts a [`Render`]'s premultiplied pixels to straight alpha —
/// the layout PNG (and most image encoders) expect. `Render` is
/// premultiplied so it uploads to the GPU without a conversion step;
/// export paths call this first.
#[must_use]
pub fn straight_alpha(premultiplied: &[u8]) -> Vec<u8> {
    premultiplied
        .chunks_exact(4)
        .flat_map(|pixel| {
            let [r, g, b, a] = [pixel[0], pixel[1], pixel[2], pixel[3]];
            if a == 0 || a == 255 {
                [r, g, b, a]
            } else {
                // u16 intermediates: r ≤ a, so each quotient fits u8.
                let un = |c: u8| (u16::from(c) * 255 / u16::from(a)) as u8;
                [un(r), un(g), un(b), a]
            }
        })
        .collect()
}

/// Everything the object walk carries: the per-render invariants plus
/// the raster state. One struct keeps the recursive signatures short —
/// symbol walks re-enter it for the definition's own layer list.
struct Walk<'a> {
    document: &'a Document,
    /// The state being viewed.
    frame: usize,
    canvas: (u16, u16),
    cache: &'a mut EffectCache,
    text_engine: &'a mut Option<text::TextEngine>,
    ctx: &'a mut RenderContext,
    /// Symbol numbers currently being rendered — cycle/depth refusal.
    stack: &'a [i64],
}

/// Paints a layer list in file order (bottom-first). A layer is
/// skipped when its `DISb` display flag is off, its per-frame
/// visibility override hides this frame, or this frame's cell is
/// hidden or absent.
///
/// Button and animation instances span forward from the cell that
/// places them: a placement in cell *j* paints in every state
/// *j..j+span*, so the walk also rescans earlier cells for spanning
/// instances. The latest placement wins — an id re-placed in a later
/// cell supersedes its earlier cells (and is rendered once).
/// Walks a layer list bottom-first. `resolve_links` gates the
/// shared-layer resolution (decision 0022): page walks resolve a link
/// layer through the definition its `UIDs` names; symbol-definition
/// walks keep links inert (sharing is a page-level property — a
/// definition's content is its own).
fn walk_layers(layers: &[Layer], walk: &mut Walk, resolve_links: bool) -> Result<(), ComposeError> {
    for layer in layers {
        if !layer.display {
            continue;
        }
        if let Some(flags) = &layer.visibility_per_frame {
            if !flags.get(walk.frame).copied().unwrap_or(true) {
                continue;
            }
        }
        if resolve_links {
            if let Some(link) = &layer.shared_link {
                // The definition renders once, in this layer's
                // z-position; its own display/visibility flags gate
                // the recursion. Links never nest — a definition
                // body's own shared_link stays inert, which also
                // rules out uid cycles. A dangling uid draws nothing
                // (the stale-reference walker rule).
                if let Some(def) = walk
                    .document
                    .shared_layer_defs
                    .iter()
                    .find(|def| def.uid == link.uid)
                {
                    walk_layers(std::slice::from_ref(&def.layer), walk, false)?;
                }
                continue;
            }
        }
        let placed: Vec<doc::ObjectId> = layer
            .cells
            .get(walk.frame)
            .filter(|cell| cell.visible)
            .map(|cell| cell.elements.clone())
            .unwrap_or_default();
        for id in &placed {
            render_object(*id, walk)?;
        }
        // Spanning instances from earlier placements, newest first.
        let mut spanned: Vec<doc::ObjectId> = Vec::new();
        for placed_at in (0..walk.frame).rev() {
            let Some(cell) = layer.cells.get(placed_at) else {
                continue;
            };
            if !cell.visible {
                continue;
            }
            for id in &cell.elements {
                if placed.contains(id) || spanned.contains(id) {
                    continue;
                }
                if instance_span(walk.document, *id) <= (walk.frame - placed_at) as i64 {
                    continue;
                }
                spanned.push(*id);
                render_spanned(*id, placed_at, walk)?;
            }
        }
    }
    Ok(())
}

/// The number of states an instance spans forward from its placement
/// cell: animation instances their `ANNi` frame count, instances of a
/// button definition the definition's state cells, plain instances
/// just their own cell.
fn instance_span(document: &Document, id: doc::ObjectId) -> i64 {
    let Some(ObjectData::SymbolInstance(instance)) = document.registry.get(id) else {
        return 1;
    };
    if let Some(anim) = &instance.animation {
        return anim.frames.max(1);
    }
    match document
        .symbols
        .iter()
        .find(|definition| definition.symbol_number == instance.symbol_number)
    {
        Some(definition) if definition.symbol_type == SymbolType::Button => {
            definition.frame_count.max(1)
        }
        _ => 1,
    }
}

fn render_object(id: doc::ObjectId, walk: &mut Walk) -> Result<(), ComposeError> {
    let object = walk
        .document
        .registry
        .get(id)
        .ok_or(ComposeError::DanglingId {
            index: id.index() as u32,
            generation: id.generation(),
        })?;
    match object {
        ObjectData::Path(path) => {
            render_path(walk.document, path, walk.canvas, walk.cache, walk.ctx)
        }
        ObjectData::Group(group) => render_group(group, walk),
        ObjectData::Text(object) => {
            let engine = walk
                .text_engine
                .get_or_insert_with(text::TextEngine::default);
            render_text(
                walk.document,
                object,
                engine,
                walk.canvas,
                walk.cache,
                walk.ctx,
            )
        }
        ObjectData::Image(image) => {
            render_image(walk.document, image, walk.canvas, walk.cache, walk.ctx)
        }
        // Slices still render as nothing — export-time markers (the
        // M4 plan's out-of-scope list).
        ObjectData::Slice(_) => Ok(()),
        ObjectData::SymbolInstance(instance) => render_instance(instance, walk.frame, walk),
    }
}

/// Renders a spanning instance discovered in an earlier cell: placed
/// in state `placed_at`, viewed in state [`Walk::frame`].
fn render_spanned(
    id: doc::ObjectId,
    placed_at: usize,
    walk: &mut Walk,
) -> Result<(), ComposeError> {
    match walk.document.registry.get(id) {
        Some(ObjectData::SymbolInstance(instance)) => render_instance(instance, placed_at, walk),
        // Non-instances never span; a dangling id in an earlier cell
        // stays silent (only the placement cell's own walk reports it).
        _ => Ok(()),
    }
}

/// Bitmaps paint from the tile bank: the `TILv` grid assembles into a
/// full-canvas premultiplied buffer that rides the same image-paint
/// blit as filtered surfaces — inside the object's own opacity/blend
/// layer, with the live effect stack applied to the assembled raster
/// exactly as it is to a rasterized path.
fn render_image(
    document: &Document,
    image: &doc::ImageObject,
    canvas: (u16, u16),
    cache: &mut EffectCache,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    if !image.visible {
        return Ok(());
    }
    let base = assemble_image(document, image, canvas)?;
    let plan = effects::plan(&image.effects);
    if plan.ops.is_empty() {
        return paint_surface(&base, canvas, image.blend, image.opacity, ctx);
    }
    // Content-addressed like paths and groups: blend/opacity stay out
    // of the key — they wrap the filtered image below.
    let key = crate::cache::hash_pixels(&base, canvas, &plan);
    let pixels = match cache.get(key).map(|entry| entry.pixels.clone()) {
        Some(pixels) => pixels,
        None => apply_filtered(base, canvas, &plan, key, cache)?,
    };
    paint_surface(&pixels, canvas, image.blend, image.opacity, ctx)
}

/// Assembles an image object's tile grid into a full-canvas
/// premultiplied buffer — the image-paint source and the filter
/// input. `TIDi` slots copy bank tiles (fixed 128×128 premultiplied
/// slots; `TSZi` 256 has no bank bridge yet and refuses loudly rather
/// than laying a wrong grid); `TMCi` slots fill solid ARGB (the
/// dictionary's recorded order here — the fill/stroke paint raws are
/// the opposite byte order, ABGR). The `WPXi`/`HPXi` extent sits at
/// location+offset, clipped to the canvas; a `TIDi` id absent from
/// the bank is a document defect, not a silent hole.
fn assemble_image(
    document: &Document,
    image: &doc::ImageObject,
    canvas: (u16, u16),
) -> Result<Vec<u8>, ComposeError> {
    let side = i64::from(raster::TILE_SIDE as u32);
    if image.tile_size != side {
        return Err(ComposeError::TileSource {
            detail: format!(
                "tile size {} has no tile-bank bridge yet (bank slots are {side})",
                image.tile_size
            ),
        });
    }
    let width = usize::from(canvas.0);
    let height = usize::from(canvas.1);
    let mut out = vec![0u8; width * height * 4];
    // Saturating everywhere: document locations are untrusted f64 (the
    // SVG importer accepts ±1e30), and i64 casts saturate rather than
    // wrap, so the clamps below see a sane rect either way.
    let left = (image.location.x + image.offset.x).round() as i64;
    let top = (image.location.y + image.offset.y).round() as i64;
    let (extent_w, extent_h) = image.pixels;
    if extent_w <= 0 || extent_h <= 0 {
        return Ok(out);
    }
    let right = left.saturating_add(extent_w);
    let bottom = top.saturating_add(extent_h);
    // Horizontal window of the extent that lies on the canvas.
    let visible = |a: i64, b: i64, limit: i64| a.clamp(0, limit)..b.clamp(0, limit);
    // Signed div_ceil is unstable; both operands are positive here.
    let cols = extent_w / side + i64::from(extent_w % side != 0);
    for (slot, tile_ref) in image.tiles.iter().enumerate() {
        let slot = slot as i64;
        let tile_left = left + (slot % cols) * side;
        let tile_top = top + (slot / cols) * side;
        let x = visible(tile_left, tile_left + side, width as i64);
        let y = visible(tile_top, tile_top + side, height as i64);
        if x.is_empty() || y.is_empty() {
            continue;
        }
        // The extent clips the grid's last row/column.
        let x = x.start.max(left)..x.end.min(right);
        let y = y.start.max(top)..y.end.min(bottom);
        if x.is_empty() || y.is_empty() {
            continue;
        }
        match tile_ref {
            TileRef::Tile(tile_id) => {
                let tile = document
                    .tiles
                    .get(*tile_id)
                    .ok_or(ComposeError::TileBankMiss { tile: *tile_id })?;
                blit_tile(&mut out, width, tile.pixels(), &x, &y, tile_left, tile_top);
            }
            TileRef::Matte(raw) => {
                let pixel = matte_premultiplied(*raw);
                for py in y.start as usize..y.end as usize {
                    for px in x.start as usize..x.end as usize {
                        let at = (py * width + px) * 4;
                        out[at..at + 4].copy_from_slice(&pixel);
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Copies the `x`/`y` window of a 128×128 tile into the canvas buffer
/// at the tile's absolute origin.
fn blit_tile(
    out: &mut [u8],
    width: usize,
    tile: &[u8; raster::TILE_BYTES],
    x: &std::ops::Range<i64>,
    y: &std::ops::Range<i64>,
    tile_left: i64,
    tile_top: i64,
) {
    for py in y.start..y.end {
        let source_y = (py - tile_top) as usize;
        for px in x.start..x.end {
            let source = (source_y * raster::TILE_SIDE + (px - tile_left) as usize) * 4;
            let target = (py as usize * width + px as usize) * 4;
            out[target..target + 4].copy_from_slice(&tile[source..source + 4]);
        }
    }
}

/// `TMCi` decodes ARGB per the dictionary, then premultiplies for the
/// canvas buffer. Public so bake paths outside the compositor (canvas
/// rotation re-tiles images the same way the renderer reads them) can
/// materialize matte slots to the exact bytes this renderer blits.
#[must_use]
pub fn matte_premultiplied(raw: i64) -> [u8; 4] {
    let raw = raw as u32;
    let (r, g, b, a) = (
        ((raw >> 16) & 0xFF) as u8,
        ((raw >> 8) & 0xFF) as u8,
        (raw & 0xFF) as u8,
        ((raw >> 24) & 0xFF) as u8,
    );
    let premul = |c: u8| ((u16::from(c) * u16::from(a) + 127) / 255) as u8;
    [premul(r), premul(g), premul(b), a]
}

/// Text renders through the path pipeline: glyphs lower to document
/// contours (the `text` crate), which ride a synthetic path object —
/// inheriting the fill paint, the blend/opacity layer, and the live
/// effect stack.
fn render_text(
    document: &Document,
    object: &doc::TextObject,
    engine: &mut text::TextEngine,
    canvas: (u16, u16),
    cache: &mut EffectCache,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    if !object.visible {
        return Ok(());
    }
    let geometry = engine.glyph_contours(object);
    // Fireworks' default text fill is opaque black; `FCLi` overrides.
    const DEFAULT_FILL: i64 = 0xFF00_0000; // ABGR: alpha 255, black
    let fill = object
        .style
        .as_ref()
        .and_then(|style| style.fill_raw)
        .unwrap_or(DEFAULT_FILL);
    let synthetic = PathObject {
        contours: geometry.contours,
        even_odd: false,
        primitive: None,
        paint: Some(doc::PaintStyle {
            stroke_raw: 0, // ABGR alpha 0 — text never strokes
            fill_raw: fill,
            fill: None,
        }),
        fill_start: None,
        fill_end: None,
        fill_third: None,
        brush_placement: None,
        fill_on_top: None,
        locked: object.locked,
        visible: true,
        opacity: object.opacity,
        blend: object.blend,
        name: None,
        effects: object.effects.clone(),
    };
    render_path(document, &synthetic, canvas, cache, ctx)
}

fn render_path(
    document: &Document,
    path: &PathObject,
    canvas: (u16, u16),
    cache: &mut EffectCache,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    if !path.visible {
        return Ok(());
    }
    let Some(paint) = &path.paint else {
        return Ok(());
    };
    let plan = effects::plan(&path.effects);
    if plan.ops.is_empty() {
        return paint_path(document, path, paint, ctx);
    }

    // Content-addressed: a cache hit skips both the offscreen render
    // and the kernels. Blend/opacity are not part of the key — they
    // wrap the filtered image below and never touch the cached bytes.
    let key = crate::cache::hash_path(path, canvas, &plan);
    if let Some(pixels) = cache.get(key).map(|entry| entry.pixels.clone()) {
        return paint_surface(&pixels, canvas, path.blend, path.opacity, ctx);
    }

    let base = rasterize_path(document, path, paint, canvas)?;
    let pixels = apply_filtered(base, canvas, &plan, key, cache)?;
    paint_surface(&pixels, canvas, path.blend, path.opacity, ctx)
}

/// Rasterizes one path alone over transparent black — the filter
/// input.
fn rasterize_path(
    document: &Document,
    path: &PathObject,
    paint: &doc::PaintStyle,
    canvas: (u16, u16),
) -> Result<Vec<u8>, ComposeError> {
    let mut offscreen = RenderContext::new(canvas.0, canvas.1);
    paint_path(document, path, paint, &mut offscreen)?;
    flush_offscreen(offscreen, canvas)
}

/// Paints one path through its own layer (fill/stroke order per
/// `FOTb`). Runs against the live context or an offscreen one.
fn paint_path(
    document: &Document,
    path: &PathObject,
    paint: &doc::PaintStyle,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    let geometry = to_bezpath(&path.contours);

    // Every object paints through its own layer so opacity and blend
    // take one uniform code path (no conditional divergence between
    // opaque and transparent objects).
    ctx.push_layer(
        None,
        Some(BlendMode::new(blend_mode(path.blend), Compose::SrcOver)),
        Some(path.opacity as f32 / 1000.0),
        None,
        None,
    );
    ctx.set_fill_rule(if path.even_odd {
        Fill::EvenOdd
    } else {
        Fill::NonZero
    });
    // `FOTb` — fill over stroke. Default (absent/false) paints the
    // fill first so the stroke lands on top.
    let filled = paint_fill(document, path, paint, &geometry, ctx);
    if path.fill_on_top == Some(true) {
        paint_stroke(paint.stroke_raw, &geometry, ctx);
        filled?;
    } else {
        filled?;
        paint_stroke(paint.stroke_raw, &geometry, ctx);
    }
    ctx.pop_layer();
    Ok(())
}

/// The fill paint: a modeled gradient rides its `FPLv` stops and the
/// fill vector handles, a modeled pattern tiles its `FPTv` raster,
/// and every other kind paints the `FCLi` colour.
fn paint_fill(
    document: &Document,
    path: &PathObject,
    paint: &doc::PaintStyle,
    geometry: &vello_cpu::kurbo::BezPath,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    let pattern = paint
        .fill
        .as_ref()
        .filter(|fill| fill.shape == FillShape::Pattern)
        .and_then(|fill| fill.pattern.as_ref());
    if let Some(pattern) = pattern {
        // The brush paints in pattern space: translate the geometry
        // into it, fill, and put the world transform back.
        let placement = pattern_placement(pattern, path);
        let local = placement.inverse() * geometry.clone();
        ctx.set_transform(placement);
        ctx.set_paint(pattern_brush(pattern, &document.tiles)?);
        ctx.fill_path(&local);
        ctx.reset_transform();
        return Ok(());
    }
    ctx.set_paint(fill_brush(path, paint));
    ctx.fill_path(geometry);
    Ok(())
}

/// The pattern placement from the fill vector handles (the same
/// handles the Gradient tool drags — `fdct` sets "the angle and size"
/// of gradient and pattern fills alike): the raster's width maps onto
/// the start→end vector, rotated with it, anchored at the start. No
/// handles: the raster sits at the canvas origin, unscaled.
fn pattern_placement(pattern: &doc::PatternTile, path: &PathObject) -> Affine {
    let (Some(start), Some(end)) = (path.fill_start, path.fill_end) else {
        return Affine::IDENTITY;
    };
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let length = (dx * dx + dy * dy).sqrt();
    if !length.is_finite() || length <= f64::EPSILON {
        return Affine::IDENTITY;
    }
    let native = pattern.pixels.0.max(1) as f64;
    Affine::translate((start.x, start.y))
        * Affine::rotate(dy.atan2(dx))
        * Affine::scale(length / native)
}

/// One pattern repeat unit: the `TILv` grid assembled at the raster's
/// native pixel size. Bank tiles are fixed 128×128 premultiplied
/// planes (`TSZi` 256 has no bank bridge and refuses, same posture as
/// `assemble_image`); matte slots fill their ARGB premultiplied.
fn pattern_brush(pattern: &doc::PatternTile, tiles: &TileBank) -> Result<PaintBrush, ComposeError> {
    let side = i64::from(raster::TILE_SIDE as u32);
    if pattern.tile_size != side {
        return Err(ComposeError::TileSource {
            detail: format!(
                "tile size {} has no tile-bank bridge yet (bank slots are {side})",
                pattern.tile_size
            ),
        });
    }
    let (width, height) = pattern.pixels;
    if width <= 0 || height <= 0 || width > 8192 || height > 8192 {
        return Err(ComposeError::TileSource {
            detail: format!("pattern extent {width}x{height} is not a usable raster"),
        });
    }
    let (width, height) = (width as usize, height as usize);
    let side = side as usize;
    let cols = width.div_ceil(side);
    let mut out = vec![0u8; width * height * 4];
    for (slot, tile_ref) in pattern.tiles.iter().enumerate() {
        let left = (slot % cols) * side;
        let top = (slot / cols) * side;
        let x = left.min(width)..(left + side).min(width);
        let y = top.min(height)..(top + side).min(height);
        if x.is_empty() || y.is_empty() {
            continue;
        }
        match *tile_ref {
            TileRef::Tile(id) => {
                let tile = tiles
                    .get(id)
                    .ok_or(ComposeError::TileBankMiss { tile: id })?;
                for py in y.clone() {
                    let source_y = py - top;
                    let source = (source_y * side + x.start - left) * 4;
                    let target = (py * width + x.start) * 4;
                    let span = (x.end - x.start) * 4;
                    out[target..target + span]
                        .copy_from_slice(&tile.pixels()[source..source + span]);
                }
            }
            TileRef::Matte(raw) => {
                let pixel = matte_premultiplied(raw);
                for py in y.clone() {
                    let target = (py * width + x.start) * 4;
                    for px in x.clone() {
                        out[target..target + 4].copy_from_slice(&pixel);
                        let _ = px;
                    }
                }
            }
        }
    }
    let mut pixmap = Pixmap::new(
        u16::try_from(width).unwrap_or(u16::MAX),
        u16::try_from(height).unwrap_or(u16::MAX),
    );
    let target = pixmap.data_as_u8_slice_mut();
    if target.len() != out.len() {
        return Err(ComposeError::TileSource {
            detail: format!(
                "pattern raster holds {} bytes, the pixmap takes {}",
                out.len(),
                target.len()
            ),
        });
    }
    target.copy_from_slice(&out);
    Ok(Brush::Image(ImageBrush {
        image: ImageSource::Pixmap(Arc::new(pixmap)),
        sampler: ImageSampler::new()
            .with_x_extend(Extend::Repeat)
            .with_y_extend(Extend::Repeat)
            .with_quality(ImageQuality::Low),
    }))
}

/// The fill brush for one path. Linear/radial/conical render natively
/// through the fill vector handles; the remaining families fall back
/// to the solid `FCLi` colour (recorded substitutions — CS6's math for
/// satin, pinch, folds, bars, ripple, waves, contour, the rectangular
/// rings and web dither is not observable from the format). Modeled
/// patterns take their own arm in [`paint_fill`]. Elliptical renders
/// as a circle: the third handle's squash needs an affine the radial
/// brush does not carry (same recorded-substitution ledger).
fn fill_brush(path: &PathObject, paint: &doc::PaintStyle) -> PaintBrush {
    let solid = || Brush::Solid(abgr(paint.fill_raw));
    let Some(fill) = &paint.fill else {
        return solid();
    };
    let Some(gradient) = &fill.gradient else {
        return solid();
    };
    let stops = gradient_stops(gradient, paint.fill_raw);
    let (Some(start), Some(end)) = (path.fill_start, path.fill_end) else {
        return solid();
    };
    let (sx, sy) = (start.x as f32, start.y as f32);
    let (ex, ey) = (end.x as f32, end.y as f32);
    let center = vello_cpu::kurbo::Point::new(start.x, start.y);
    let kind = match fill.shape {
        FillShape::Linear => GradientKind::Linear(LinearGradientPosition {
            start: center,
            end: vello_cpu::kurbo::Point::new(end.x, end.y),
        }),
        FillShape::Radial | FillShape::Elliptical => {
            let radius = ((ex - sx) * (ex - sx) + (ey - sy) * (ey - sy)).sqrt();
            GradientKind::Radial(RadialGradientPosition {
                start_center: center,
                start_radius: 0.0,
                end_center: center,
                end_radius: radius,
            })
        }
        // The conical sweep starts along the fill vector — the handle
        // the editor drags — and wraps a full turn (recorded
        // interpretation: CS6's zero-angle basis is not observable
        // from the format).
        FillShape::Conical => {
            let start_angle = (ey - sy).atan2(ex - sx);
            GradientKind::Sweep(SweepGradientPosition {
                center,
                start_angle,
                end_angle: start_angle + std::f32::consts::TAU,
            })
        }
        _ => return solid(),
    };
    Brush::Gradient(PenikoGradient {
        kind,
        // Beyond the ramp ends the outer colours hold, as in CS6.
        extend: Extend::Pad,
        stops,
        // `FG0v` (colour) and `FG1v` (coverage) are independent 1D
        // ramps: the colour rides the position regardless of coverage,
        // so stops interpolate unpremultiplied. The premultiplied
        // default would drag a transparent end stop's colour toward
        // black and shift the hue of every covered pixel (pinned by
        // the corpus gradient fixture's veil).
        interpolation_alpha_space: InterpolationAlphaSpace::Unpremultiplied,
        ..PenikoGradient::default()
    })
}

/// Merges the `FG0v` RGB ramp and `FG1v` alpha ramp into one stop
/// list: every distinct position samples both ramps, alpha setting
/// coverage. An empty ramp contributes nothing on its axis (all
/// opaque, or the solid colour when the RGB ramp is empty too).
fn gradient_stops(gradient: &Gradient, solid_raw: i64) -> vello_cpu::peniko::ColorStops {
    let mut positions: Vec<f64> = gradient
        .rgb
        .iter()
        .chain(gradient.alpha.iter())
        .map(|stop| stop.position)
        .collect();
    positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    positions.dedup();

    let solid = abgr(solid_raw);
    let mut stops = vello_cpu::peniko::ColorStops::new();
    for position in positions {
        let color = sample_stop(&gradient.rgb, position, solid);
        // `sample_stop` yields normalized components: the alpha axis is
        // already 0..1 coverage, exactly what `multiply_alpha` wants.
        let alpha = if gradient.alpha.is_empty() {
            1.0
        } else {
            let opaque = AlphaColor::from_rgba8(0, 0, 0, 255);
            sample_stop(&gradient.alpha, position, opaque).components[3]
        };
        stops.push(vello_cpu::peniko::ColorStop {
            offset: position as f32,
            color: color.multiply_alpha(alpha).into(),
        });
    }
    stops
}

/// Piecewise-linear sample of one ramp at `position` in ABGR layout;
/// outside the ends the nearest stop holds (matching `Extend::Pad`).
fn sample_stop(
    ramp: &[doc::GradientStop],
    position: f64,
    edge: AlphaColor<Srgb>,
) -> AlphaColor<Srgb> {
    let Some(first) = ramp.first() else {
        return edge;
    };
    if position <= first.position {
        return abgr(first.color_raw);
    }
    let Some(last) = ramp.last() else {
        return edge;
    };
    if position >= last.position {
        return abgr(last.color_raw);
    }
    for pair in ramp.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if position >= a.position && position <= b.position {
            let span = b.position - a.position;
            let t = if span > 0.0 {
                (position - a.position) / span
            } else {
                0.0
            };
            let (ca, cb) = (&abgr(a.color_raw).components, &abgr(b.color_raw).components);
            let mix = |i: usize| -> u8 {
                (ca[i] * (1.0 - t as f32) + cb[i] * t as f32).mul_add(255.0, 0.5) as u8
            };
            return AlphaColor::from_rgba8(mix(0), mix(1), mix(2), mix(3));
        }
    }
    edge
}

fn paint_stroke(color: i64, geometry: &vello_cpu::kurbo::BezPath, ctx: &mut RenderContext) {
    // Brush geometry — `BPLv` diameter and `BRPi` inside/outside
    // placement — is outside the M4 model: strokes are 1px wide and
    // centered regardless of brush placement (recorded limitation,
    // M4 plan).
    ctx.set_stroke(Stroke::new(1.0));
    ctx.set_paint(Brush::Solid(abgr(color)));
    ctx.stroke_path(geometry);
}

fn render_group(group: &GroupObject, walk: &mut Walk) -> Result<(), ComposeError> {
    if !group.visible {
        return Ok(());
    }
    let plan = effects::plan(&group.effects);
    if plan.ops.is_empty() {
        walk.ctx.push_layer(
            None,
            Some(BlendMode::new(blend_mode(group.blend), Compose::SrcOver)),
            Some(group.opacity as f32 / 1000.0),
            None,
            None,
        );
        // Mask-structured groups composite all children unmasked this
        // milestone (masks as pixel content are M6/M7; `MNAb` is
        // carried by the model only).
        for id in &group.children {
            render_object(*id, walk)?;
        }
        walk.ctx.pop_layer();
        return Ok(());
    }

    // The group's filters see the composite of its children (each
    // child's own effects already applied); the group's blend/opacity
    // wrap the filtered image, as on the unfiltered path.
    let mut offscreen = RenderContext::new(walk.canvas.0, walk.canvas.1);
    for id in &group.children {
        render_object(
            *id,
            &mut Walk {
                document: walk.document,
                frame: walk.frame,
                canvas: walk.canvas,
                cache: &mut *walk.cache,
                text_engine: &mut *walk.text_engine,
                ctx: &mut offscreen,
                stack: walk.stack,
            },
        )?;
    }
    offscreen.flush();
    let mut pixmap = Pixmap::new(walk.canvas.0, walk.canvas.1);
    offscreen.render(pixmap.as_mut(), &mut Resources::new());
    let base = pixmap.data_as_u8_slice().to_vec();
    let key = crate::cache::hash_pixels(&base, walk.canvas, &plan);
    let pixels = match walk.cache.get(key).map(|entry| entry.pixels.clone()) {
        Some(pixels) => pixels,
        None => apply_filtered(base, walk.canvas, &plan, key, walk.cache)?,
    };
    paint_surface(&pixels, walk.canvas, group.blend, group.opacity, walk.ctx)
}

/// Renders one symbol instance viewed in state [`Walk::frame`], placed
/// in state `placed_at` (equal when the walk is painting the
/// instance's own cell).
///
/// The definition's layers compose to a full-canvas raster exactly
/// like a page — definition content is authored in page coordinates
/// (CS6 symbols are created from on-page selections), so the raster is
/// canvas-anchored and the instance transform plus its animation
/// offset place it on the page. Nine-slice definitions re-grid that
/// raster by their guide insets instead of transforming it
/// ([`nine_slice_grid`]). Definition nesting is refused past
/// [`MAX_SYMBOL_DEPTH`]; a definition that instances itself is a
/// typed refusal, never a recursion.
fn render_instance(
    instance: &SymbolInstanceObject,
    placed_at: usize,
    walk: &mut Walk,
) -> Result<(), ComposeError> {
    if walk.stack.len() >= MAX_SYMBOL_DEPTH {
        return Err(ComposeError::SymbolDepth {
            symbol: instance.symbol_number,
            depth: walk.stack.len(),
        });
    }
    if walk.stack.contains(&instance.symbol_number) {
        return Err(ComposeError::SymbolCycle {
            symbol: instance.symbol_number,
        });
    }
    let definition = walk
        .document
        .symbols
        .iter()
        .find(|definition| definition.symbol_number == instance.symbol_number)
        .ok_or(ComposeError::MissingDefinition {
            symbol: instance.symbol_number,
        })?;

    // States beyond a definition's cells hold nothing; a button's
    // last state sticks for later frames (clamped, not empty).
    let max_cells = definition
        .layers
        .layers
        .iter()
        .map(|layer| layer.cells.len())
        .max()
        .unwrap_or(0);
    let definition_frame = (walk.frame - placed_at).min(max_cells.saturating_sub(1));

    let mut inner = walk.stack.to_vec();
    inner.push(instance.symbol_number);
    let mut offscreen = RenderContext::new(walk.canvas.0, walk.canvas.1);
    walk_layers(
        &definition.layers.layers,
        &mut Walk {
            document: walk.document,
            frame: definition_frame,
            canvas: walk.canvas,
            cache: &mut *walk.cache,
            text_engine: &mut *walk.text_engine,
            ctx: &mut offscreen,
            stack: &inner,
        },
        false,
    )?;
    offscreen.flush();
    let mut pixmap = Pixmap::new(walk.canvas.0, walk.canvas.1);
    offscreen.render(pixmap.as_mut(), &mut Resources::new());
    let base = pixmap.data_as_u8_slice().to_vec();

    let (transform, opacity) = animation_at(instance, walk.frame - placed_at);
    match &definition.nine_slice {
        Some(guides) => {
            let gridded = nine_slice_grid(&base, walk.canvas, guides, transform);
            paint_surface(&gridded, walk.canvas, 0, opacity, walk.ctx)
        }
        None => paint_transformed(&base, walk.canvas, transform, opacity, walk.ctx),
    }
}

/// The instance transform and opacity at animation step `step`
/// (frames since placement): the `ANI*` parameters interpolate
/// linearly across the span's `frames − 1` transitions — state 0 is
/// the untouched instance, the last state carries the full
/// shift/rotation/scale and the `AEOi` opacity. Rotation pivots on the
/// canvas origin; opacity defaults to full when its endpoint is
/// absent.
fn animation_at(instance: &SymbolInstanceObject, step: usize) -> (Affine, i64) {
    let Some(anim) = &instance.animation else {
        return (instance.transform, 1000);
    };
    let transitions = (anim.frames.max(2) - 1) as f64;
    let t = (step as f64 / transitions).clamp(0.0, 1.0);
    let mut total = Affine::translate((anim.x_shift * t, anim.y_shift * t));
    if let Some(degrees) = anim.rotation {
        total *= Affine::rotate(degrees as f64 * t * std::f64::consts::PI / 180.0);
    }
    let scale = 1.0 + (anim.scaling / 100.0 - 1.0) * t;
    total *= Affine::scale(scale);
    let start = anim.start_opacity.unwrap_or(1000) as f64;
    let end = anim.end_opacity.unwrap_or(1000) as f64;
    (
        total * instance.transform,
        (start + (end - start) * t).round() as i64,
    )
}

/// Composites the definition raster through the instance transform: a
/// canvas-anchored image paint under `transform`, inside the
/// instance's opacity layer. Nearest sampling keeps the output
/// deterministic.
fn paint_transformed(
    base: &[u8],
    canvas: (u16, u16),
    transform: Affine,
    opacity: i64,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    let mut pixmap = Pixmap::new(canvas.0, canvas.1);
    let target = pixmap.data_as_u8_slice_mut();
    if target.len() != base.len() {
        return Err(ComposeError::FilterPipeline {
            detail: format!(
                "definition raster holds {} bytes, canvas takes {}",
                base.len(),
                target.len()
            ),
        });
    }
    target.copy_from_slice(base);
    ctx.push_layer(
        None,
        Some(BlendMode::new(blend_mode(0), Compose::SrcOver)),
        Some(opacity as f32 / 1000.0),
        None,
        None,
    );
    ctx.set_transform(transform);
    ctx.set_paint(ImageBrush {
        image: ImageSource::Pixmap(Arc::new(pixmap)),
        sampler: ImageSampler::new().with_quality(ImageQuality::Low),
    });
    ctx.fill_rect(&Rect::new(
        0.0,
        0.0,
        f64::from(canvas.0),
        f64::from(canvas.1),
    ));
    ctx.reset_transform();
    ctx.pop_layer();
    Ok(())
}

/// Re-grids the definition raster through the nine-slice guides: the
/// shape bounds cut the raster into nine regions; corners move 1:1,
/// edges stretch along one axis, the center fills the rest. `NGCf`
/// order is top, bottom, left, right (the dictionary's). The
/// destination is the transform-mapped shape bounds; insets larger
/// than their extent shrink so the middle pieces never go negative.
/// Integer nearest-neighbor sampling — deterministic bytes.
fn nine_slice_grid(
    base: &[u8],
    canvas: (u16, u16),
    guides: &NineSliceGuides,
    transform: Affine,
) -> Vec<u8> {
    let width = i64::from(canvas.0);
    let height = i64::from(canvas.1);
    let mut out = vec![0u8; (width * height * 4) as usize];
    let bounds = guides.shape_bounds;
    let sx = bounds.x0.round() as i64;
    let sy = bounds.y0.round() as i64;
    let sw = (bounds.x1 - bounds.x0).round() as i64;
    let sh = (bounds.y1 - bounds.y0).round() as i64;
    if sw <= 0 || sh <= 0 {
        return out;
    }
    let mapped = map_rect(transform, bounds);
    let dx = mapped.x0.round() as i64;
    let dy = mapped.y0.round() as i64;
    let dw = mapped.width().round() as i64;
    let dh = mapped.height().round() as i64;
    if dw <= 0 || dh <= 0 {
        return out;
    }

    // Dictionary `NGCf` order: top, bottom, left, right.
    let mut top = guides.controls[0].round() as i64;
    let mut bottom = guides.controls[1].round() as i64;
    let mut left = guides.controls[2].round() as i64;
    let mut right = guides.controls[3].round() as i64;
    top = top.clamp(0, sh);
    bottom = bottom.clamp(0, sh - top);
    left = left.clamp(0, sw);
    right = right.clamp(0, sw - left);
    // The destination keeps corners unscaled at the inset size; when
    // the target is smaller than the corners, they shrink together.
    let mut dst_left = left;
    let mut dst_right = right;
    if left + right > dw {
        let factor = dw as f64 / (left + right) as f64;
        dst_left = (left as f64 * factor).floor() as i64;
        dst_right = dw - dst_left;
    }
    let mut dst_top = top;
    let mut dst_bottom = bottom;
    if top + bottom > dh {
        let factor = dh as f64 / (top + bottom) as f64;
        dst_top = (top as f64 * factor).floor() as i64;
        dst_bottom = dh - dst_top;
    }

    let src_widths = [left, sw - left - right, right];
    let src_heights = [top, sh - top - bottom, bottom];
    let dst_widths = [dst_left, dw - dst_left - dst_right, dst_right];
    let dst_heights = [dst_top, dh - dst_top - dst_bottom, dst_bottom];
    let prefix = |sizes: [i64; 3]| [0, sizes[0], sizes[0] + sizes[1]];
    let src_x = prefix(src_widths);
    let src_y = prefix(src_heights);
    let dst_x = prefix(dst_widths);
    let dst_y = prefix(dst_heights);

    for row in 0..3 {
        for col in 0..3 {
            let (piece_w, piece_h) = (src_widths[col], src_heights[row]);
            let (draw_w, draw_h) = (dst_widths[col], dst_heights[row]);
            if piece_w <= 0 || piece_h <= 0 || draw_w <= 0 || draw_h <= 0 {
                continue;
            }
            let piece_x = sx + src_x[col];
            let piece_y = sy + src_y[row];
            let draw_x = dx + dst_x[col];
            let draw_y = dy + dst_y[row];
            for py in 0..draw_h {
                let target_y = draw_y + py;
                if target_y < 0 || target_y >= height {
                    continue;
                }
                let sample_y = piece_y + (py * piece_h + piece_h / 2) / draw_h;
                if sample_y < 0 || sample_y >= height {
                    continue;
                }
                for px in 0..draw_w {
                    let target_x = draw_x + px;
                    if target_x < 0 || target_x >= width {
                        continue;
                    }
                    let sample_x = piece_x + (px * piece_w + piece_w / 2) / draw_w;
                    if sample_x < 0 || sample_x >= width {
                        continue;
                    }
                    let source = ((sample_y * width + sample_x) * 4) as usize;
                    let target = ((target_y * width + target_x) * 4) as usize;
                    out[target..target + 4].copy_from_slice(&base[source..source + 4]);
                }
            }
        }
    }
    out
}

/// Drives the rendered base through the kernel stack and stores the
/// result under `key`.
fn apply_filtered(
    base: Vec<u8>,
    canvas: (u16, u16),
    plan: &effects::Plan,
    key: u64,
    cache: &mut EffectCache,
) -> Result<Vec<u8>, ComposeError> {
    let surface = Surface::from_premultiplied(u32::from(canvas.0), u32::from(canvas.1), base)
        .map_err(|error| ComposeError::FilterPipeline {
            detail: error.to_string(),
        })?;
    let pixels = effects::apply_stack(surface, &plan.ops).into_pixels();
    cache.insert(key, canvas.0, canvas.1, pixels.clone());
    Ok(pixels)
}

/// Composites a filtered raster back onto the live context: the
/// object's own opacity/blend layer wraps a 1:1 canvas-sized image
/// paint (nearest sampling — the pixels are already at output scale).
fn paint_surface(
    pixels: &[u8],
    canvas: (u16, u16),
    blend: i64,
    opacity: i64,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    ctx.push_layer(
        None,
        Some(BlendMode::new(blend_mode(blend), Compose::SrcOver)),
        Some(opacity as f32 / 1000.0),
        None,
        None,
    );
    let mut pixmap = Pixmap::new(canvas.0, canvas.1);
    let target = pixmap.data_as_u8_slice_mut();
    if target.len() != pixels.len() {
        return Err(ComposeError::FilterPipeline {
            detail: format!(
                "filtered raster holds {} bytes, canvas takes {}",
                pixels.len(),
                target.len()
            ),
        });
    }
    target.copy_from_slice(pixels);
    ctx.set_paint(ImageBrush {
        image: ImageSource::Pixmap(Arc::new(pixmap)),
        sampler: ImageSampler::new().with_quality(ImageQuality::Low),
    });
    ctx.fill_rect(&Rect::new(
        0.0,
        0.0,
        f64::from(canvas.0),
        f64::from(canvas.1),
    ));
    ctx.pop_layer();
    Ok(())
}

/// Flushes an offscreen context into its pixel bytes.
fn flush_offscreen(mut ctx: RenderContext, canvas: (u16, u16)) -> Result<Vec<u8>, ComposeError> {
    ctx.flush();
    let mut pixmap = Pixmap::new(canvas.0, canvas.1);
    ctx.render(pixmap.as_mut(), &mut Resources::new());
    Ok(pixmap.data_as_u8_slice().to_vec())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn straight_alpha_round_trips_opaque_and_transparent() {
        // Opaque pixels are unchanged; fully transparent goes to zero.
        assert_eq!(
            straight_alpha(&[1, 2, 3, 255, 0, 0, 0, 0]),
            [1, 2, 3, 255, 0, 0, 0, 0]
        );
    }

    #[test]
    fn straight_alpha_unpremultiplies_partial_coverage() {
        // Half-alpha red: premultiplied (128, 0, 0, 128) → (255, 0, 0, 128).
        assert_eq!(straight_alpha(&[128, 0, 0, 128]), [255, 0, 0, 128]);
    }
}
