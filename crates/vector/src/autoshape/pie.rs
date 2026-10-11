//! Pie / Doughnut tools (`Pie.jsf` / `Doughnut.jsf` behavior — one
//! shared generator, `createShape`, split by the shape name): an annular
//! sector from a start/end angle pair with per-span quarter-circle
//! handles on both arcs.
//!
//! The arcs are node-distributed (`distributeAngles`): 2 nodes per arc
//! for spans up to 90°, 3 up to 180°, 4 up to 270°, else 5 — handles
//! scaled by each node's angular distance to its neighbors
//! (`r·κ·Δ/(π/2)`). The inner arc is walked forward and the outer arc
//! backward, so under the nonzero rule the sector is solid and an
//! annulus (inner > 0) carries a hole. A doughnut whose angles coincide
//! takes the script's circle shortcut: two full four-node circles —
//! same winding both, so that one result carries the even-odd fill
//! flag (a documented port decision: the script relies on the
//! renderer's fill semantics here; corpus-gated).

use super::{ELLIPSE_BCP_CONST, closed_contour, curved, shape};
use doc::model::{PathObject, PathPoint};
use geom::Point;

/// Pie / doughnut parameters (`createShape(a0, a1, R, r)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PieParams {
    /// Center control point (`c0`).
    pub center: Point,
    /// Outer radius (`radius0`, `R`).
    pub outer: f64,
    /// Inner radius (`radius1`, `r`; 0 for a pie wedge).
    pub inner: f64,
    /// Sector start angle (`a0`), radians.
    pub start: f64,
    /// Sector end angle (`a1`), radians.
    pub end: f64,
    /// Pie semantics (`shapeName == "pie"`): a zero-length angle span
    /// draws the distributed arc shape instead of the circle shortcut.
    pub is_pie: bool,
}

impl PieParams {
    /// The pie tool's insert defaults: radius 100, no inner radius,
    /// quarter sector from −90° to 0°.
    pub fn pie_defaults(center: Point) -> Self {
        Self {
            center,
            outer: 100.0,
            inner: 0.0,
            start: -std::f64::consts::FRAC_PI_2,
            end: 0.0,
            is_pie: true,
        }
    }

    /// The doughnut tool's insert defaults: radius 100, inner 40,
    /// coincident angles (the full-circle shortcut).
    pub fn doughnut_defaults(center: Point) -> Self {
        Self {
            center,
            outer: 100.0,
            inner: 40.0,
            start: -std::f64::consts::FRAC_PI_2,
            end: -std::f64::consts::FRAC_PI_2,
            is_pie: false,
        }
    }
}

/// `distributeAngles`: the node angles on the arc — `a[0] = a0`,
/// `a[N−1] = a1`, evenly spaced between, normalized into (−π, π] per
/// stop. Node count by span: 2 up to a quarter turn, 3 up to a half,
/// 4 up to three quarters, else 5 (a coincident span spreads over the
/// full turn).
fn distribute_angles(a0: f64, a1: f64) -> Vec<f64> {
    let tau = std::f64::consts::TAU;
    let da = if a0 == a1 {
        tau
    } else if a1 < a0 {
        tau - a0 + a1
    } else {
        (a1 - a0).abs()
    };
    let n = if da <= std::f64::consts::FRAC_PI_2 {
        2
    } else if da <= std::f64::consts::PI {
        3
    } else if da <= 3.0 * std::f64::consts::FRAC_PI_2 {
        4
    } else {
        5
    };
    let mut a = vec![a0];
    for i in 1..n - 1 {
        let stop = a0 + da / (n - 1) as f64 * i as f64;
        a.push(if stop >= tau { stop - tau } else { stop });
    }
    a.push(a1);
    a
}

/// Angular distance from `prev` to `cur` with the script's wraparound
/// correction (a crossing past ±π measured the long way round).
fn angle_delta(cur: f64, prev: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    if cur < prev {
        if prev > 0.0 {
            tau - (prev - cur)
        } else {
            prev - cur
        }
    } else {
        cur - prev
    }
}

/// One arc node (`moveNodePos`): position on the radius at `angle`,
/// handles scaled by the angular distance to each neighbor, oriented
/// by which walk produced the node (`inner` = the forward walk, w = 0;
/// outer = the backward walk). A zero delta stores an absent handle.
fn arc_node(
    center: Point,
    radius: f64,
    angle: f64,
    pred_delta: f64,
    succ_delta: f64,
    inner: bool,
) -> PathPoint {
    let quarter = std::f64::consts::FRAC_PI_2;
    let (s, c) = angle.sin_cos();
    let x = center.x + radius * c;
    let y = center.y + radius * s;
    let kappa = ELLIPSE_BCP_CONST / quarter;
    let (dxp, dyp) = (
        radius * kappa * pred_delta * s,
        radius * kappa * pred_delta * c,
    );
    let (dxs, dys) = (
        radius * kappa * succ_delta * s,
        radius * kappa * succ_delta * c,
    );
    // A zero radius collapses every handle onto the anchor — the
    // model's absent-handle case (pinched pie centers stay clean).
    let live = radius != 0.0;
    let (primary, secondary) = if inner {
        (
            (live && pred_delta != 0.0).then(|| Point::new(x + dxp, y - dyp)),
            (live && succ_delta != 0.0).then(|| Point::new(x - dxs, y + dys)),
        )
    } else {
        (
            (live && pred_delta != 0.0).then(|| Point::new(x - dxp, y + dyp)),
            (live && succ_delta != 0.0).then(|| Point::new(x + dxs, y - dys)),
        )
    };
    PathPoint {
        anchor: Point::new(x, y),
        primary,
        secondary,
        is_curve: primary.is_some() || secondary.is_some(),
    }
}

/// A full four-node circle (the script's shortcut layout: anchors at
/// top/right/bottom/left, quarter-circle handles).
fn circle_nodes(center: Point, radius: f64) -> Vec<PathPoint> {
    let cp = radius * ELLIPSE_BCP_CONST;
    let (cx, cy) = (center.x, center.y);
    vec![
        curved(
            cx,
            cy - radius,
            (cx - cp, cy - radius),
            (cx + cp, cy - radius),
        ),
        curved(
            cx + radius,
            cy,
            (cx + radius, cy - cp),
            (cx + radius, cy + cp),
        ),
        curved(
            cx,
            cy + radius,
            (cx + cp, cy + radius),
            (cx - cp, cy + radius),
        ),
        curved(
            cx - radius,
            cy,
            (cx - radius, cy + cp),
            (cx - radius, cy - cp),
        ),
    ]
}

/// The doughnut tool: the shared generator pinned to pie = false.
pub fn doughnut(mut p: PieParams) -> PathObject {
    p.is_pie = false;
    pie(p)
}

/// Generates the pie wedge / doughnut sector.
pub fn pie(p: PieParams) -> PathObject {
    if !p.is_pie && p.start == p.end {
        // Doughnut circle shortcut: outer circle, inner circle when the
        // inner radius is live. Same winding on both — even-odd carries
        // the hole (see module doc).
        let mut contours = vec![closed_contour(circle_nodes(p.center, p.outer))];
        if p.inner > 0.0 {
            contours.push(closed_contour(circle_nodes(p.center, p.inner)));
        }
        let mut result = shape(contours);
        result.even_odd = true;
        return result;
    }

    let a = distribute_angles(p.start, p.end);
    let n = a.len();
    let mut nodes: Vec<PathPoint> = Vec::with_capacity(2 * n);

    // Inner arc, forward (w = 0): in-handle to the previous stop, out
    // to the next; both clipped at the walk's ends.
    for (i, stop) in a.iter().enumerate() {
        let pred = if i == 0 {
            0.0
        } else {
            angle_delta(*stop, a[i - 1])
        };
        let succ = if i == n - 1 {
            0.0
        } else {
            angle_delta(a[i + 1], *stop)
        };
        nodes.push(arc_node(p.center, p.inner, *stop, pred, succ, true));
    }
    // Outer arc, backward (w = 5): the walk runs from the last stop to
    // the first, so the neighbor roles swap.
    for i in (0..n).rev() {
        let pred = if i == n - 1 {
            0.0
        } else {
            angle_delta(a[i + 1], a[i])
        };
        let succ = if i == 0 {
            0.0
        } else {
            angle_delta(a[i], a[i - 1])
        };
        nodes.push(arc_node(p.center, p.outer, a[i], pred, succ, false));
    }

    shape(vec![closed_contour(nodes)])
}
