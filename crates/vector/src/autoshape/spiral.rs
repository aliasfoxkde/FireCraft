//! Spiral tool (`Spiral.jsf` behavior): a logarithmic (equiangular)
//! spiral `r = 0.5·exp(θ·cot b)` with a tightness parameter `b` in
//! 50°..=89°, an outer radius limit, a direction flag, and a closed
//! mode that retraces the whole curve back to the center as one
//! contour.
//!
//! Nodes land every `2π/ppc` radians (ppc = 4, doubled for tight
//! early turns); handles are quarter-circle-constant scaled by the
//! angular step (`r·κ·Δθ/(π/2)`), and the final node is clipped to
//! exactly reach the radius limit — the script's `createEquiangularSpiral`
//! arithmetic, including its reuse of the clipping state in closed
//! mode.

use super::{ELLIPSE_BCP_CONST, open_contour, shape};
use doc::model::{Contour, PathObject, PathPoint};
use geom::Point;

/// Tightness bounds (`k.minB`/`k.maxB`): the evalB control maps the
/// drag control onto this window; outside it `cot b ≤ 0` would stop
/// the radius from ever reaching the limit.
pub const MIN_TIGHTNESS_DEG: f64 = 50.0;
pub const MAX_TIGHTNESS_DEG: f64 = 89.0;

/// `k.ppc` — nodes per full turn on the open spiral.
const POINTS_PER_CYCLE: f64 = 4.0;

/// Starting radius (`a` in the curve equation).
const START_RADIUS: f64 = 0.5;

/// Spiral parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpiralParams {
    /// Center control point (`c0`).
    pub center: Point,
    /// Outer radius limit (`radius`; the curve stops at the first node
    /// past it, clipped back onto the limit).
    pub radius: f64,
    /// Tightness `b` in degrees (clamped 50..=89).
    pub tightness_deg: f64,
    /// Drawing direction (`clockwise`).
    pub clockwise: bool,
    /// Closed mode: retrace the curve back to the center, reversed.
    pub closed: bool,
}

impl SpiralParams {
    /// The tool's insert defaults: radius 100, tightness 80°, open,
    /// clockwise.
    #[must_use]
    pub fn defaults(center: Point) -> Self {
        Self {
            center,
            radius: 100.0,
            tightness_deg: 80.0,
            clockwise: true,
            closed: false,
        }
    }
}

/// `evalB`: the tightness window entry for a vertical drag-control
/// position (`c2.y` between `center.y − radius` and `center.y +
/// radius` maps onto 50°..=89° logarithmically). The shell uses this
/// to turn control-point drags into the parameter [`SpiralParams::
/// tightness_deg`].
#[must_use]
pub fn tightness_from_control(center_y: f64, radius: f64, control_y: f64) -> f64 {
    let linear = ((center_y + radius - control_y) / (2.0 * radius)) * 9.0;
    (linear + 1.0).ln() / 12f64.ln() * (MAX_TIGHTNESS_DEG - MIN_TIGHTNESS_DEG) + MIN_TIGHTNESS_DEG
}

/// One node on the curve (`addPointOnSpiral`): position at `(r, θ)`
/// mirrored by direction, in-handle clipped to the anchor at θ = 0,
/// handles `cpdist` along the tangent, and `cpdist` scaled by the
/// quarter-circle constant relative to a quarter turn.
fn point_on_spiral(p: &SpiralParams, r: f64, theta: f64, b: f64, cpdist: f64) -> PathPoint {
    let (x, y, a) = if p.clockwise {
        (
            p.center.x + r * theta.cos(),
            p.center.y + r * theta.sin(),
            theta - std::f64::consts::FRAC_PI_2 + b,
        )
    } else {
        (
            p.center.x - r * (-theta).cos(),
            p.center.y - r * (-theta).sin(),
            -theta + std::f64::consts::FRAC_PI_2 - b,
        )
    };
    let (dx, dy) = (cpdist * a.sin(), cpdist * a.cos());
    // The script stores the first in-handle coincident with the anchor
    // (`theta==0` exactly — the spiral's only exact-zero parameter);
    // an absent handle is our model's spelling of that zero handle.
    let (primary, is_curve) = if theta == 0.0 {
        (None, true)
    } else {
        (Some(Point::new(x + dx, y - dy)), true)
    };
    PathPoint {
        anchor: Point::new(x, y),
        primary,
        secondary: Some(Point::new(x - dx, y + dy)),
        is_curve,
    }
}

/// Generates the spiral as one open (or closed-retrace) contour. A
/// radius inside the first turn clips the whole curve onto the limit
/// (one node); a non-positive radius has no curve at all and yields
/// the empty path.
#[must_use]
pub fn spiral(p: SpiralParams) -> PathObject {
    let b_deg = p.tightness_deg.clamp(MIN_TIGHTNESS_DEG, MAX_TIGHTNESS_DEG);
    let b = b_deg.to_radians();
    let cotb = 1.0 / b.tan();
    let radius = if p.radius.is_finite() { p.radius } else { 0.0 };
    if radius <= 0.0 {
        return shape(Vec::new());
    }

    // Points per cycle doubles when the first four nodes already
    // overshoot the limit (tighter early curve needs the resolution).
    let ppc = if START_RADIUS * (std::f64::consts::PI * 4.0 * cotb).exp() < radius {
        POINTS_PER_CYCLE
    } else {
        POINTS_PER_CYCLE * 2.0
    };
    let a_inc = std::f64::consts::TAU / ppc;
    let quarter = std::f64::consts::FRAC_PI_2;

    let mut nodes: Vec<PathPoint> = Vec::new();
    let mut theta = 0.0;
    let mut r;
    // Clipping state from the final over-limit step (the closed mode
    // and the clip both lean on it, exactly as the script does).
    let (mut a0, mut a1, mut an) = (0.0, 0.0, 0.0);

    loop {
        r = START_RADIUS * (theta * cotb).exp();
        if r <= radius {
            let cpdist = r * ELLIPSE_BCP_CONST * a_inc / quarter;
            nodes.push(point_on_spiral(&p, r, theta, b, cpdist));
        } else {
            // Clip the final span onto the radius limit: solve for the
            // angle where the curve reaches it, and pin the previous
            // node's out-handle to the clipped tangent.
            a0 = theta - a_inc;
            a1 = (radius / START_RADIUS).ln() / cotb;
            let cpdist = radius * ELLIPSE_BCP_CONST * (a1 - a0) / quarter;
            theta = a1;
            an = a0 - quarter + b;
            let (dx, dy) = (cpdist * an.sin(), cpdist * an.cos());
            let sign = if p.clockwise { 1.0 } else { -1.0 };
            if let Some(last) = nodes.last_mut() {
                if let Some(succ) = last.secondary.as_mut() {
                    *succ = Point::new(last.anchor.x - sign * dx, last.anchor.y + dy);
                }
            }
            nodes.push(point_on_spiral(&p, radius, theta, b, cpdist));
            break;
        }
        theta += a_inc;
        if !theta.is_finite() {
            break;
        }
    }

    if nodes.is_empty() {
        return shape(Vec::new());
    }

    // Clip the final node's out-handle: the spiral ends there.
    if let Some(last) = nodes.last_mut() {
        last.secondary = Some(Point::new(last.anchor.x, last.anchor.y));
    }

    if p.closed {
        let open_len = nodes.len();
        // Turn back one full revolution plus a step and retrace.
        theta -= std::f64::consts::TAU + a_inc;
        r = START_RADIUS * (theta * cotb).exp();
        let cpdist = r * ELLIPSE_BCP_CONST * (a1 - a0) / quarter;
        nodes.push(point_on_spiral(&p, r, theta, b, cpdist));

        // Copy the tail in reverse with handles swapped (an absent
        // handle — the spiral head's zero in-handle — stays absent on
        // its mirror).
        let start = open_len as isize - 2 - ppc as isize;
        let mut i = start;
        while i > -1 {
            let n = &nodes[i as usize];
            nodes.push(PathPoint {
                anchor: n.anchor,
                primary: n.secondary,
                secondary: n.primary,
                is_curve: true,
            });
            i -= 1;
        }

        // The first retrace node mirrors its handles around the join.
        let join = open_len; // index of the just-added turning point
        if let Some(turn) = nodes.get_mut(join) {
            turn.secondary = turn.primary;
            turn.primary = Some(Point::new(turn.anchor.x, turn.anchor.y));
        }
        if open_len + 1 < nodes.len() {
            // Pull the next retrace node's in-handle onto the curve.
            theta -= a_inc;
            r = START_RADIUS * (theta * cotb).exp();
            let cpdist = r * ELLIPSE_BCP_CONST * (a1 - a0) / quarter;
            let (dx, dy) = (cpdist * an.sin(), cpdist * an.cos());
            let sign = if p.clockwise { 1.0 } else { -1.0 };
            if let Some(next) = nodes.get_mut(join + 1) {
                next.primary = Some(Point::new(next.anchor.x - sign * dx, next.anchor.y + dy));
            }
        }
        let n = nodes.len() as i64;
        shape(vec![Contour {
            closed: true,
            point_count: n,
            points: nodes,
        }])
    } else {
        shape(vec![open_contour(nodes)])
    }
}
