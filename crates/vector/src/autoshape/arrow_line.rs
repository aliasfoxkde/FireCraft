//! Arrow Line tool (`Arrow Line.jsf` behavior): a straight segment
//! with an independently-styled head on each end.
//!
//! The script keeps three path elements — the line and one head
//! contour per end — rebuilt from the endpoints and a head-mode pair
//! (`DrawbyModecp0`/`DrawbyModecp1`, five modes each, cycled by
//! clicking the control point). Head geometry is fixed-size: open and
//! solid heads are 10-unit arms at ±30° about the line direction, the
//! tail adds a 15-unit skirt and a 20-unit point at 45°, the box is a
//! 5-unit diagonal square centered on the endpoint. A `none` head is
//! the script's degenerate zero-paint contour; here it is simply
//! absent. Head contours belonging to this path are emitted after the
//! line contour, in start-then-end order.

use super::{closed_contour, open_contour, shape};
use doc::model::{PathObject, PathPoint};
use geom::Point;

/// Head styles (`mode0`/`mode1`, the script's five-way cycle).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArrowHead {
    /// No head (`custdata.none`).
    #[default]
    None,
    /// Open V (`head1`): two arms, stroked only.
    Open,
    /// Closed triangle (`head2`): arms closed back to the endpoint.
    Solid,
    /// Barbed tail (`tail`): swept quartet closed around the endpoint.
    Tail,
    /// Diagonal square (`box`).
    Box,
}

/// Arm reach of the open/solid heads (`10` in the script).
const HEAD_ARM: f64 = 10.0;
/// Half-angle of the open/solid heads (`rad30`).
const HEAD_HALF_ANGLE: f64 = std::f64::consts::FRAC_PI_6;
/// Half-angle of the tail and box arms (`rad45`).
const OBLIQUE_HALF_ANGLE: f64 = std::f64::consts::FRAC_PI_4;
/// Tail skirt offset along the line (`15`).
const TAIL_SET: f64 = 15.0;
/// Tail point reach past the endpoint (`20`).
const TAIL_DEPTH: f64 = 20.0;
/// Box arm (`5`).
const BOX_ARM: f64 = 5.0;

/// Arrow Line parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowLineParams {
    /// Line start (`cps[0]`, `x1`/`y1`).
    pub start: Point,
    /// Line end (`cps[1]`, `x2`/`y2`).
    pub end: Point,
    /// Head style at `start`.
    pub start_head: ArrowHead,
    /// Head style at `end`.
    pub end_head: ArrowHead,
}

impl ArrowLineParams {
    /// The tool's insert defaults: a 100-unit horizontal line, no head
    /// at the start, an open head at the end.
    pub fn defaults(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
            start_head: ArrowHead::None,
            end_head: ArrowHead::Open,
        }
    }
}

/// A corner node with both handles coincident (a polyline vertex).
fn plain(x: f64, y: f64) -> PathPoint {
    PathPoint {
        anchor: Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    }
}

/// One head's nodes: the endpoint plus its arms, rotated by `angle`
/// (the line direction) and mirrored for the end head (`forward`).
fn head_nodes(at: Point, angle: f64, kind: ArrowHead, forward: bool) -> Vec<PathPoint> {
    // +1 at the line's start, −1 at its end: the script writes each
    // end's formulas separately, and the difference is exactly the
    // sign of the arm reach and the along-line skirt offsets.
    let d = if forward { 1.0 } else { -1.0 };
    let (cos, sin) = (angle.cos(), angle.sin());
    // Arm at line-angle + `delta` with a signed reach (the mirror
    // flips the reach, not the angular offset).
    let arm = |delta: f64, reach: f64| -> (f64, f64) {
        (
            at.x + reach * (angle + delta).cos(),
            at.y + reach * (angle + delta).sin(),
        )
    };
    // Step from `p` along the line direction by `len`.
    let along = |p: (f64, f64), len: f64| -> (f64, f64) { (p.0 + len * cos, p.1 + len * sin) };
    match kind {
        ArrowHead::None => Vec::new(),
        ArrowHead::Open | ArrowHead::Solid => {
            let (x1, y1) = arm(HEAD_HALF_ANGLE, HEAD_ARM * d);
            let (x2, y2) = arm(-HEAD_HALF_ANGLE, HEAD_ARM * d);
            vec![plain(x1, y1), plain(at.x, at.y), plain(x2, y2)]
        }
        ArrowHead::Tail => {
            let (x1, y1) = arm(OBLIQUE_HALF_ANGLE, -HEAD_ARM * d);
            let (x2, y2) = arm(-OBLIQUE_HALF_ANGLE, -HEAD_ARM * d);
            let (x3, y3) = along((x2, y2), TAIL_SET * d);
            let (x4, y4) = along((at.x, at.y), TAIL_DEPTH * d);
            let (x5, y5) = along((x1, y1), TAIL_SET * d);
            vec![
                plain(x1, y1),
                plain(at.x, at.y),
                plain(x2, y2),
                plain(x3, y3),
                plain(x4, y4),
                plain(x5, y5),
            ]
        }
        // The box is symmetric — the script's two ends agree exactly.
        ArrowHead::Box => {
            let (x1, y1) = arm(OBLIQUE_HALF_ANGLE, -BOX_ARM);
            let (x2, y2) = arm(-OBLIQUE_HALF_ANGLE, -BOX_ARM);
            let (x3, y3) = arm(OBLIQUE_HALF_ANGLE, BOX_ARM);
            let (x4, y4) = arm(-OBLIQUE_HALF_ANGLE, BOX_ARM);
            vec![plain(x1, y1), plain(x2, y2), plain(x3, y3), plain(x4, y4)]
        }
    }
}

/// Generates the arrow line: the open 2-node line contour, then each
/// present head as its own contour (open for `Open`, closed for the
/// filled styles).
pub fn arrow_line(p: ArrowLineParams) -> PathObject {
    let angle = (p.end.y - p.start.y).atan2(p.end.x - p.start.x);
    let mut contours = Vec::with_capacity(3);
    contours.push(open_contour(vec![
        plain(p.start.x, p.start.y),
        plain(p.end.x, p.end.y),
    ]));

    let (start_nodes, end_nodes) = (
        head_nodes(p.start, angle, p.start_head, true),
        head_nodes(p.end, angle, p.end_head, false),
    );
    if !start_nodes.is_empty() {
        let closed = p.start_head != ArrowHead::Open;
        contours.push(if closed {
            closed_contour(start_nodes)
        } else {
            open_contour(start_nodes)
        });
    }
    if !end_nodes.is_empty() {
        let closed = p.end_head != ArrowHead::Open;
        contours.push(if closed {
            closed_contour(end_nodes)
        } else {
            open_contour(end_nodes)
        });
    }
    shape(contours)
}
