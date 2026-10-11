//! Arrow and L-Shape tools (the shared `L-Shape.jsf` behavior — one
//! script draws both, switching on a flag and an `a = 3` node offset
//! for the arrow head).
//!
//! Both shapes are one closed ring over the same frame: a rounded
//! top-left corner, the top edge running right, the right side (plain
//! corner for the L, barbs + tip for the arrow), the shaft bottom at
//! `y + t`, and a stem down to `y + h`. Roundness `r` rounds the
//! top-left corner; the excess beyond the shaft thickness,
//! `r_ = max(0, r − t)`, rounds the inner corner — the script's morph
//! from an L into a rounded blob as the corner control keeps dragging.
//!
//! The canonical parameter→geometry mapping is the script's
//! `RedrawSmartShape` (the reload path: it rebuilds the whole ring
//! from the stored parameters alone), which is exactly this crate's
//! regenerate-don't-bake contract — and like it, the generators draw
//! verbatim: the drag-time rubber bands (roundness ≤ max(w,h), barb
//! reach, stem limits) are interactive-shell history, not geometry
//! constraints, so every parameter passes through untouched. Only the
//! inner-corner ternary `r_ = r > t ? r − t : 0` is part of the
//! mapping itself.

use super::{ELLIPSE_BCP_CONST, closed_contour, shape};
use doc::model::{PathObject, PathPoint};
use geom::Point;

/// L-Shape parameters (`RedrawSmartShape`, `isArrow = false`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LShapeParams {
    /// Frame origin: top-left of the shape's bounding box.
    pub origin: Point,
    /// Overall width (`width`) and height (`height`).
    pub width: f64,
    pub height: f64,
    /// Shaft thickness (`thickness`): the horizontal arm's height.
    pub thickness: f64,
    /// Corner roundness (`roundness`); beyond `thickness` it starts
    /// rounding the inner corner too.
    pub roundness: f64,
}

impl LShapeParams {
    /// The tool's insert defaults: 200×200 with a 100 arm, sharp.
    pub fn defaults(origin: Point) -> Self {
        Self {
            origin,
            width: 200.0,
            height: 200.0,
            thickness: 100.0,
            roundness: 0.0,
        }
    }
}

/// Arrow parameters (`RedrawSmartShape`, `isArrow = true`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowParams {
    /// Frame origin: top-left of the shape's bounding box.
    pub origin: Point,
    /// Overall width (`width`) and height (`height`).
    pub width: f64,
    pub height: f64,
    /// Shaft thickness (`thickness`).
    pub thickness: f64,
    /// Corner roundness (`roundness`).
    pub roundness: f64,
    /// Full barb span (`arrowWidth`): the head's vertical extent.
    pub arrow_width: f64,
    /// Barb setback from the right edge (`arrowLength`).
    pub arrow_length: f64,
    /// Tip reach past the right edge (`arrowTip`).
    pub arrow_tip: f64,
}

impl ArrowParams {
    /// The tool's insert defaults: 200 wide, 100 tall shaft, barbs
    /// spanning 200 (they overhang the shaft band by ±50), no setback,
    /// tip reach 100, sharp corners.
    pub fn defaults(origin: Point) -> Self {
        Self {
            origin,
            width: 200.0,
            height: 100.0,
            thickness: 100.0,
            roundness: 0.0,
            arrow_width: 200.0,
            arrow_length: 0.0,
            arrow_tip: 100.0,
        }
    }
}

/// A node with explicit handles; a handle coincident with the anchor
/// (how the script stores a clipped side) is the model's absent case.
fn node(x: f64, y: f64, pred: Option<(f64, f64)>, succ: Option<(f64, f64)>) -> PathPoint {
    let live = |h: Option<(f64, f64)>| {
        h.filter(|(hx, hy)| *hx != x || *hy != y)
            .map(|(hx, hy)| Point::new(hx, hy))
    };
    let (primary, secondary) = (live(pred), live(succ));
    PathPoint {
        anchor: Point::new(x, y),
        is_curve: primary.is_some() || secondary.is_some(),
        primary,
        secondary,
    }
}

/// The rounded top-left corner node (anchor on the top edge, in-handle
/// reaching back along the edge by `r·κ`).
fn top_left(x: f64, y: f64, r: f64) -> PathPoint {
    node(x + r, y, Some((x + r - r * ELLIPSE_BCP_CONST, y)), None)
}

/// The shared five-node tail: inner corner (filleted by `r_`), stem
/// down the left side, closed by the rounded bottom of the top-left
/// arc. This is the script's `nodes[3+a]`..`nodes[7+a]` block.
fn tail(x: f64, y: f64, t: f64, h: f64, r: f64, r_: f64) -> Vec<PathPoint> {
    vec![
        node(
            x + t + r_,
            y + t,
            None,
            Some((x + t + r_ * ELLIPSE_BCP_CONST, y + t)),
        ),
        node(
            x + t,
            y + t + r_,
            Some((x + t, y + t + r_ - r_ * ELLIPSE_BCP_CONST)),
            None,
        ),
        node(x + t, y + h, None, None),
        node(x, y + h, None, None),
        node(x, y + r, None, Some((x, y + r - r * ELLIPSE_BCP_CONST))),
    ]
}

/// Generates the L-Shape: one closed 8-node contour.
pub fn lshape(p: LShapeParams) -> PathObject {
    let (t, r, w, h) = (p.thickness, p.roundness, p.width, p.height);
    let r_ = if r > t { r - t } else { 0.0 };
    let (x, y) = (p.origin.x, p.origin.y);

    let mut nodes = Vec::with_capacity(8);
    nodes.push(top_left(x, y, r));
    nodes.push(node(x + w, y, None, None));
    nodes.push(node(x + w, y + t, None, None));
    nodes.extend(tail(x, y, t, h, r, r_));
    shape(vec![closed_contour(nodes)])
}

/// Generates the Arrow: one closed 11-node contour — the L's frame
/// with the right side replaced by barbs and tip.
pub fn arrow(p: ArrowParams) -> PathObject {
    let (t, r, w, h, aw, al, at) = (
        p.thickness,
        p.roundness,
        p.width,
        p.height,
        p.arrow_width,
        p.arrow_length,
        p.arrow_tip,
    );
    let r_ = if r > t { r - t } else { 0.0 };
    let (x, y) = (p.origin.x, p.origin.y);

    let mut nodes = Vec::with_capacity(11);
    nodes.push(top_left(x, y, r));
    nodes.push(node(x + w, y, None, None));
    nodes.push(node(x + w - al, y + (t - aw) / 2.0, None, None));
    nodes.push(node(x + w + at, y + t / 2.0, None, None));
    nodes.push(node(x + w - al, y + (t + aw) / 2.0, None, None));
    nodes.push(node(x + w, y + t, None, None));
    nodes.extend(tail(x, y, t, h, r, r_));
    shape(vec![closed_contour(nodes)])
}
