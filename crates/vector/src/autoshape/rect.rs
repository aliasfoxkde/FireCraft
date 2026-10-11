//! The rectangle corner family (`Rounded|Chamfer|Beveled Rectangle.jsf`
//! behavior): one shared generator — `createRectangle(c, w, h, r, t)` —
//! over a corner kind `t` (0 beveled, 1 rounded, 2 chamfered).
//!
//! Eight anchors: four "corner start" anchors on one edge and four
//! "corner end" anchors on the next, with the corner spans carrying
//! handles and the straight edge spans none. Rounded corners take the
//! quarter-circle handle `r·ELLIPSE_BCP_CONST` perpendicular to the
//! radius; beveled corners take none; chamfered corners take the same
//! length along the adjacent edge directions, exactly as the script
//! writes them — which renders a shallow bulged cut, not a straight
//! chamfer. That reading is taken verbatim and pinned by the tests;
//! confirming it against a Wine-oracle render is a recorded open
//! question (corpus-gated).

use super::{ELLIPSE_BCP_CONST, closed_contour, shape};
use doc::model::{PathObject, PathPoint};
use geom::Point;

/// Corner treatment (`cornerType` in the scripts; the three tools are
/// the same generator with different `t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerKind {
    /// Straight diagonal cuts (`t` = 0).
    Beveled,
    /// Quarter-circle arcs (`t` = 1).
    Rounded,
    /// Edge-parallel handles per the script (`t` = 2; see module doc).
    Chamfer,
}

impl CornerKind {
    fn index(self) -> i32 {
        match self {
            CornerKind::Beveled => 0,
            CornerKind::Rounded => 1,
            CornerKind::Chamfer => 2,
        }
    }
}

/// Rectangle parameters. `corner` is the single cut radius the shared
/// generator takes (the scripts store `corner0..3` but drive this
/// function with one value; per-corner radii are a drag-time concern,
/// corpus-gated).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectParams {
    /// Top-left corner (`c` before the first `c.y += r` shift).
    pub origin: Point,
    /// Full width (`W`).
    pub width: f64,
    /// Full height (`H`).
    pub height: f64,
    /// Corner cut radius (`r`; insert default `H/4`).
    pub corner: f64,
    /// Corner treatment.
    pub kind: CornerKind,
}

impl RectParams {
    /// The tool's insert defaults: 200×150, corner `H/4`.
    pub fn defaults(origin: Point, kind: CornerKind) -> Self {
        Self {
            origin,
            width: 200.0,
            height: 150.0,
            corner: 150.0 / 4.0,
            kind,
        }
    }
}

/// Generates the rectangle as one closed 8-anchor contour.
pub fn rectangle(p: RectParams) -> PathObject {
    let t = p.kind.index();
    let cpdist = p.corner * ELLIPSE_BCP_CONST;
    let mut c = p.origin;
    let mut points: Vec<PathPoint> = Vec::with_capacity(8);

    // Zero handles are expressed as (anchor, anchor) in the scripts and
    // as absent handles here; each anchor below carries at most one
    // potentially non-zero handle, alternating succ/pred. The ternaries
    // are transcribed from the script lines one for one.
    let mut push = |at: Point, succ_off: (f64, f64), pred_off: (f64, f64)| {
        let anchor = at;
        let (primary, secondary) = if pred_off == (0.0, 0.0) && succ_off == (0.0, 0.0) {
            (None, None)
        } else if pred_off == (0.0, 0.0) {
            // Corner start: the out-handle carries the curve.
            (None, Some(Point::new(at.x + succ_off.0, at.y + succ_off.1)))
        } else {
            // Corner end: the in-handle carries the curve (succ and
            // pred are never both non-zero in this generator).
            (Some(Point::new(at.x + pred_off.0, at.y + pred_off.1)), None)
        };
        points.push(PathPoint {
            anchor,
            primary,
            secondary,
            is_curve: primary.is_some() || secondary.is_some(),
        });
    };

    // 1. left edge, top (corner start)
    c.y += p.corner;
    push(
        c,
        (
            if t == 2 { cpdist } else { 0.0 },
            if t == 1 { -cpdist } else { 0.0 },
        ),
        (0.0, 0.0),
    );
    // 2. top-left corner end
    c.x += p.corner;
    c.y -= p.corner;
    push(
        c,
        (0.0, 0.0),
        (
            if t == 1 { -cpdist } else { 0.0 },
            if t == 2 { cpdist } else { 0.0 },
        ),
    );
    // 3. top edge, right (corner start)
    c.x += p.width - p.corner * 2.0;
    push(
        c,
        (
            if t == 1 { cpdist } else { 0.0 },
            if t == 2 { cpdist } else { 0.0 },
        ),
        (0.0, 0.0),
    );
    // 4. top-right corner end
    c.x += p.corner;
    c.y += p.corner;
    push(
        c,
        (0.0, 0.0),
        (
            if t == 2 { -cpdist } else { 0.0 },
            if t == 1 { -cpdist } else { 0.0 },
        ),
    );
    // 5. right edge, bottom (corner start)
    c.y += p.height - p.corner * 2.0;
    push(
        c,
        (
            if t == 2 { -cpdist } else { 0.0 },
            if t == 1 { cpdist } else { 0.0 },
        ),
        (0.0, 0.0),
    );
    // 6. bottom-right corner end
    c.x -= p.corner;
    c.y += p.corner;
    push(
        c,
        (0.0, 0.0),
        (
            if t == 1 { cpdist } else { 0.0 },
            if t == 2 { -cpdist } else { 0.0 },
        ),
    );
    // 7. bottom edge, left (corner start)
    c.x -= p.width - p.corner * 2.0;
    push(
        c,
        (
            if t == 1 { -cpdist } else { 0.0 },
            if t == 2 { -cpdist } else { 0.0 },
        ),
        (0.0, 0.0),
    );
    // 8. bottom-left corner end
    c.x -= p.corner;
    c.y -= p.corner;
    push(
        c,
        (0.0, 0.0),
        (
            if t == 2 { cpdist } else { 0.0 },
            if t == 1 { cpdist } else { 0.0 },
        ),
    );

    shape(vec![closed_contour(points)])
}
