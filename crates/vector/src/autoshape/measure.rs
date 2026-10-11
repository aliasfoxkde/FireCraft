//! Measure Tool (`Measure Tool.jsf` behavior): a red dimension readout
//! between two points — two arrowheads, two perpendicular end ticks,
//! and two extension lines that stop short of the middle to leave room
//! for the distance label.
//!
//! `RedrawSmartShape` plus `EndDragControlPoint` is the canonical
//! mapping. Two regimes split at a span of 80 units: beyond it the
//! arrowheads point along the line toward each end and the extension
//! lines cross the span, stopping `δ` short of the midpoint (`δ` is 30
//! for |slope| ≤ 1, else 20); below it the heads flip outward and the
//! extension lines run 15 units away past each endpoint. The distance
//! label itself is a document Text element positioned by the shell —
//! not part of this geometry.
//!
//! Contour order follows the script: start extension, end extension,
//! start head, end head, start tick, end tick. Every node is a plain
//! polyline vertex (the script's `addPathPoint` stores all handles
//! coincident).

use super::{open_contour, shape};
use doc::model::{Contour, PathObject, PathPoint};
use geom::Point;

/// Perpendicular tick half-length (`15`).
const TICK: f64 = 15.0;
/// Arrowhead arm reach (`8`).
const HEAD_ARM: f64 = 8.0;
/// Arrowhead half-angle (`rad30`).
const HEAD_HALF_ANGLE: f64 = std::f64::consts::FRAC_PI_6;
/// Span below which the heads flip and the extensions point away.
const SHORT_SPAN: f64 = 80.0;
/// Midpoint gap for a shallow span (`30`, |slope| ≤ 1).
const GAP_SHALLOW: f64 = 30.0;
/// Midpoint gap for a steep span (`20`).
const GAP_STEEP: f64 = 20.0;

/// Measure Tool parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasureParams {
    /// Measured start (`cps[0]`, `x1`/`y1`).
    pub start: Point,
    /// Measured end (`cps[1]`, `x2`/`y2`).
    pub end: Point,
}

impl MeasureParams {
    /// The tool's insert defaults: a 100-unit horizontal span.
    pub fn defaults(start: Point) -> Self {
        Self {
            start,
            end: Point::new(start.x + 100.0, start.y),
        }
    }
}

/// A polyline vertex (handles coincident → absent in our model).
fn plain(x: f64, y: f64) -> PathPoint {
    PathPoint {
        anchor: Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    }
}

fn line(a: Point, b: Point) -> Contour {
    open_contour(vec![plain(a.x, a.y), plain(b.x, b.y)])
}

/// A 3-node arrowhead at `at`: the endpoint between two arms at
/// `±HEAD_HALF_ANGLE`, reaching `HEAD_ARM` along `dir` (negative
/// reaches point away from the line direction).
fn head(at: Point, angle: f64, reach: f64) -> Contour {
    let a1 = angle + HEAD_HALF_ANGLE;
    let a2 = angle - HEAD_HALF_ANGLE;
    open_contour(vec![
        plain(at.x + reach * a1.cos(), at.y + reach * a1.sin()),
        plain(at.x, at.y),
        plain(at.x + reach * a2.cos(), at.y + reach * a2.sin()),
    ])
}

/// The perpendicular tick: `TICK` units each way along `angle + 90°`.
fn tick(at: Point, angle: f64) -> Contour {
    let a = angle + std::f64::consts::FRAC_PI_2;
    let (dx, dy) = (TICK * a.cos(), TICK * a.sin());
    open_contour(vec![
        plain(at.x + dx, at.y + dy),
        plain(at.x - dx, at.y - dy),
    ])
}

/// Generates the six measurement contours.
pub fn measure(p: MeasureParams) -> PathObject {
    let span = (p.end.x - p.start.x).hypot(p.end.y - p.start.y);
    let angle = (p.end.y - p.start.y).atan2(p.end.x - p.start.x);
    let (sin, cos) = angle.sin_cos();
    let mid = Point::new((p.start.x + p.end.x) / 2.0, (p.start.y + p.end.y) / 2.0);
    let slope = if p.end.x == p.start.x {
        f64::INFINITY
    } else {
        (p.end.y - p.start.y) / (p.end.x - p.start.x)
    };

    // Extension-line far ends: across the span with a midpoint gap, or
    // outward past the endpoints on the short regime.
    let (ext_start_end, ext_end_end, reach) = if span < SHORT_SPAN {
        (
            Point::new(p.start.x - TICK * cos, p.start.y - TICK * sin),
            Point::new(p.end.x + TICK * cos, p.end.y + TICK * sin),
            -HEAD_ARM,
        )
    } else {
        let gap = if slope.abs() <= 1.0 {
            GAP_SHALLOW
        } else {
            GAP_STEEP
        };
        (
            Point::new(mid.x - gap * cos, mid.y - gap * sin),
            Point::new(mid.x + gap * cos, mid.y + gap * sin),
            HEAD_ARM,
        )
    };

    shape(vec![
        line(p.start, ext_start_end),
        line(p.end, ext_end_end),
        // Heads aim along the line toward each endpoint; the short
        // regime reaches negative (flipped outward).
        head(p.start, angle, reach),
        head(p.end, angle, -reach),
        tick(p.start, angle),
        tick(p.end, angle),
    ])
}
