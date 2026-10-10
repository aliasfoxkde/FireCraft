//! The document walk: page canvas → layer list → state cell → objects,
//! painted bottom-first through per-object opacity/blend layers.

use crate::convert::{abgr, blend_mode, to_bezpath};
use doc::{Document, GroupObject, Layer, ObjectData, PathObject};
use std::fmt;
use vello_cpu::kurbo::Stroke;
use vello_cpu::peniko::{BlendMode, Brush, Compose, Fill};
use vello_cpu::{Pixmap, RenderContext, Resources};

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
        }
    }
}

impl std::error::Error for ComposeError {}

/// Composes one page in one state (frame) to pixels. The page canvas
/// (`WIDf`/`HITf`, rounded) decides the output size; frames beyond a
/// layer's cell list render that layer as empty. Document coordinates
/// are canvas pixels (identity transform); resolution scaling is an
/// export-time concern.
pub fn render(document: &Document, page: usize, frame: usize) -> Result<Render, ComposeError> {
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
    walk_layers(&page.layers.layers, document, frame, &mut ctx)?;
    ctx.flush();

    let mut pixmap = Pixmap::new(width, height);
    ctx.render(pixmap.as_mut(), &mut resources);
    Ok(Render {
        width,
        height,
        pixels: pixmap.data_as_u8_slice().to_vec(),
    })
}

/// Paints a layer list in file order (bottom-first). A layer is
/// skipped when its `DISb` display flag is off, its per-frame
/// visibility override hides this frame, or this frame's cell is
/// hidden or absent.
fn walk_layers(
    layers: &[Layer],
    document: &Document,
    frame: usize,
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
            render_object(document, *id, frame, ctx)?;
        }
    }
    Ok(())
}

fn render_object(
    document: &Document,
    id: doc::ObjectId,
    frame: usize,
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    let object = document.registry.get(id).ok_or(ComposeError::DanglingId {
        index: id.index() as u32,
        generation: id.generation(),
    })?;
    match object {
        ObjectData::Path(path) => render_path(path, ctx),
        ObjectData::Group(group) => render_group(document, group, frame, ctx),
        // Images (no tile decode this milestone), slices, text (M6) and
        // symbol instances render as nothing — the M4 plan's
        // out-of-scope list.
        ObjectData::Image(_)
        | ObjectData::Slice(_)
        | ObjectData::Text(_)
        | ObjectData::SymbolInstance(_) => Ok(()),
    }
}

fn render_path(path: &PathObject, ctx: &mut RenderContext) -> Result<(), ComposeError> {
    if !path.visible {
        return Ok(());
    }
    let Some(paint) = &path.paint else {
        return Ok(());
    };
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
    Ok(())
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
    ctx: &mut RenderContext,
) -> Result<(), ComposeError> {
    if !group.visible {
        return Ok(());
    }
    ctx.push_layer(
        None,
        Some(BlendMode::new(blend_mode(group.blend), Compose::SrcOver)),
        Some(group.opacity as f32 / 1000.0),
        None,
        None,
    );
    // Mask-structured groups composite all children unmasked this
    // milestone (masks as pixel content are M6/M7; `MNAb` is carried by
    // the model only).
    for id in &group.children {
        render_object(document, *id, frame, ctx)?;
    }
    ctx.pop_layer();
    Ok(())
}
