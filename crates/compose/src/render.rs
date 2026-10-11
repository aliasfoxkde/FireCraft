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
use doc::{Document, GroupObject, Layer, ObjectData, PathObject};
use filters::Surface;
use std::fmt;
use std::sync::Arc;
use vello_cpu::kurbo::{Rect, Stroke};
use vello_cpu::peniko::{BlendMode, Brush, Compose, Fill, ImageBrush, ImageQuality, ImageSampler};
use vello_cpu::{ImageSource, Pixmap, RenderContext, Resources};

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
        }
    }
}

impl std::error::Error for ComposeError {}

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

/// [`render`] over a caller-owned effect cache: filtered object
/// rasters survive between calls keyed on content, so unchanged
/// objects skip their kernels on the next render.
pub fn render_with_cache(
    document: &Document,
    page: usize,
    frame: usize,
    cache: &mut EffectCache,
) -> Result<Render, ComposeError> {
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
    let (width, height) = (width as u16, height as u16);

    let mut ctx = RenderContext::new(width, height);
    let mut resources = Resources::new();
    walk_layers(
        &page.layers.layers,
        document,
        frame,
        (width, height),
        cache,
        &mut ctx,
    )?;
    ctx.flush();

    let mut pixmap = Pixmap::new(width, height);
    ctx.render(pixmap.as_mut(), &mut resources);
    Ok(Render {
        width,
        height,
        pixels: pixmap.data_as_u8_slice().to_vec(),
    })
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

/// Paints a layer list in file order (bottom-first). A layer is
/// skipped when its `DISb` display flag is off, its per-frame
/// visibility override hides this frame, or this frame's cell is
/// hidden or absent.
fn walk_layers(
    layers: &[Layer],
    document: &Document,
    frame: usize,
    canvas: (u16, u16),
    cache: &mut EffectCache,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    for layer in layers {
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
        for id in &cell.elements {
            render_object(document, *id, frame, canvas, cache, ctx)?;
        }
    }
    Ok(())
}

fn render_object(
    document: &Document,
    id: doc::ObjectId,
    frame: usize,
    canvas: (u16, u16),
    cache: &mut EffectCache,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    let object = document.registry.get(id).ok_or(ComposeError::DanglingId {
        index: id.index() as u32,
        generation: id.generation(),
    })?;
    match object {
        ObjectData::Path(path) => render_path(path, canvas, cache, ctx),
        ObjectData::Group(group) => render_group(document, group, frame, canvas, cache, ctx),
        // Images (no tile decode this milestone), slices, text (M6) and
        // symbol instances render as nothing — the M4 plan's
        // out-of-scope list.
        ObjectData::Image(_)
        | ObjectData::Slice(_)
        | ObjectData::Text(_)
        | ObjectData::SymbolInstance(_) => Ok(()),
    }
}

fn render_path(
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
        paint_path(path, paint, ctx);
        return Ok(());
    }

    // Content-addressed: a cache hit skips both the offscreen render
    // and the kernels. Blend/opacity are not part of the key — they
    // wrap the filtered image below and never touch the cached bytes.
    let key = crate::cache::hash_path(path, canvas, &plan);
    if let Some(pixels) = cache.get(key).map(|entry| entry.pixels.clone()) {
        return paint_surface(&pixels, canvas, path.blend, path.opacity, ctx);
    }

    let base = rasterize_path(path, paint, canvas)?;
    let pixels = apply_filtered(base, canvas, &plan, key, cache)?;
    paint_surface(&pixels, canvas, path.blend, path.opacity, ctx)
}

/// Rasterizes one path alone over transparent black — the filter
/// input.
fn rasterize_path(
    path: &PathObject,
    paint: &doc::PaintStyle,
    canvas: (u16, u16),
) -> Result<Vec<u8>, ComposeError> {
    let mut offscreen = RenderContext::new(canvas.0, canvas.1);
    paint_path(path, paint, &mut offscreen);
    flush_offscreen(offscreen, canvas)
}

/// Paints one path through its own layer (fill/stroke order per
/// `FOTb`). Runs against the live context or an offscreen one.
fn paint_path(path: &PathObject, paint: &doc::PaintStyle, ctx: &mut RenderContext) {
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
    if path.fill_on_top == Some(true) {
        paint_stroke(paint.stroke_raw, &geometry, ctx);
        paint_fill(paint.fill_raw, &geometry, ctx);
    } else {
        paint_fill(paint.fill_raw, &geometry, ctx);
        paint_stroke(paint.stroke_raw, &geometry, ctx);
    }
    ctx.pop_layer();
}

fn paint_fill(color: i64, geometry: &vello_cpu::kurbo::BezPath, ctx: &mut RenderContext) {
    ctx.set_paint(Brush::Solid(abgr(color)));
    ctx.fill_path(geometry);
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

fn render_group(
    document: &Document,
    group: &GroupObject,
    frame: usize,
    canvas: (u16, u16),
    cache: &mut EffectCache,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    if !group.visible {
        return Ok(());
    }
    let plan = effects::plan(&group.effects);
    if plan.ops.is_empty() {
        ctx.push_layer(
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
            render_object(document, *id, frame, canvas, cache, ctx)?;
        }
        ctx.pop_layer();
        return Ok(());
    }

    // The group's filters see the composite of its children (each
    // child's own effects already applied); the group's blend/opacity
    // wrap the filtered image, as on the unfiltered path.
    let mut offscreen = RenderContext::new(canvas.0, canvas.1);
    for id in &group.children {
        render_object(document, *id, frame, canvas, cache, &mut offscreen)?;
    }
    offscreen.flush();
    let mut pixmap = Pixmap::new(canvas.0, canvas.1);
    offscreen.render(pixmap.as_mut(), &mut Resources::new());
    let base = pixmap.data_as_u8_slice().to_vec();
    let key = crate::cache::hash_pixels(&base, canvas, &plan);
    let pixels = match cache.get(key).map(|entry| entry.pixels.clone()) {
        Some(pixels) => pixels,
        None => apply_filtered(base, canvas, &plan, key, cache)?,
    };
    paint_surface(&pixels, canvas, group.blend, group.opacity, ctx)
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
