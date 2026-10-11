//! Star tool (`Star.jsf` behavior): an alternating outer/inner radius
//! polygon with optional roundness on either corner family.
//!
//! Parameters map to the script's `customData`: `radius0`/`radius1`
//! (outer/inner radius), `round0`/`round1` (the roundness radii the
//! shift-drag controls move), `nodes` (corner count, 3..=50), and the
//! start angle (the center→outer-corner control-point direction, −π/2
//! at insert = straight up). A corner whose rounded radius rounds to
//! the same integer as its plain radius stays straight — the script's
//! exact criterion (`Math.round` equality).

use super::{closed_contour, corner, curved, js_round, shape};
use doc::model::{PathObject, PathPoint};
use geom::Point;

/// The script's constants: corner-count floor/ceiling and the handle
/// clamp (`maxHandleSize`).
pub const MIN_POINTS: u32 = 3;
pub const MAX_POINTS: u32 = 50;
const MAX_HANDLE: f64 = 3000.0;

/// Star parameters (`createStar(R, R_, r, r_, N)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarParams {
    /// Center control point (hidden in the UI; `c5`).
    pub center: Point,
    /// Outer corner radius (`radius0`, `R`).
    pub outer: f64,
    /// Inner corner radius (`radius1`, `r`).
    pub inner: f64,
    /// Outer roundness radius (`round0`, `R_`).
    pub outer_round: f64,
    /// Inner roundness radius (`round1`, `r_`).
    pub inner_round: f64,
    /// Corner count (`nodes`, clamped 3..=50).
    pub points: u32,
    /// Direction of the first (outer) corner from `center`, radians.
    pub start_angle: f64,
}

impl StarParams {
    /// The tool's insert defaults: 100/50 radii, five points, straight
    /// corners (roundness radii equal the plain radii), first corner up.
    pub fn defaults(center: Point) -> Self {
        Self {
            center,
            outer: 100.0,
            inner: 50.0,
            outer_round: 100.0,
            inner_round: 50.0,
            points: 5,
            start_angle: -std::f64::consts::FRAC_PI_2,
        }
    }
}

/// Generates the star as one closed contour of `2·points` anchors.
pub fn star(p: StarParams) -> PathObject {
    let n = (p.points as f64).clamp(MIN_POINTS as f64, MAX_POINTS as f64) as usize;
    let da = std::f64::consts::TAU / (2.0 * n as f64);
    let mut a = p.start_angle;
    let mut points: Vec<PathPoint> = Vec::with_capacity(2 * n);

    for i in 0..2 * n {
        // The script normalizes into (−π, π] purely for the trig calls,
        // which are 2π-periodic — the raw angle is equivalent.
        let (s, c) = a.sin_cos();
        let outer_side = i & 1 == 0;
        let (radius, round) = if outer_side {
            (p.outer, p.outer_round)
        } else {
            (p.inner, p.inner_round)
        };

        if js_round(round) == js_round(radius) {
            let (x, y) = (p.center.x + radius * c, p.center.y + radius * s);
            points.push(corner(x, y));
        } else {
            // Handle length for the rounded corner, straight from the
            // script: the inner family divides by the outer/inner ratio
            // term (with the triangle case pinned to the clamp), the
            // outer family by the center-distance term.
            let mut rr = if outer_side {
                p.inner * da.sin() * (p.outer - p.outer_round) / (p.outer - p.inner * da.cos())
            } else if n > 3 {
                (p.inner_round - p.inner) * p.outer * da.sin() / (p.outer * da.cos() - p.inner)
            } else {
                MAX_HANDLE
            };
            if !rr.is_finite() {
                // The script's clamp only guards the upper bound; a
                // non-finite length (denominator exactly zero) would
                // poison every downstream consumer with NaN handles,
                // so it takes the clamp value instead.
                rr = MAX_HANDLE;
            }
            if rr > MAX_HANDLE {
                rr = MAX_HANDLE;
            }

            let (x, y) = (p.center.x + round * c, p.center.y + round * s);
            // Handles sit perpendicular to the radius, `rr` each way.
            points.push(curved(
                x,
                y,
                (x + rr * s, y - rr * c),
                (x - rr * s, y + rr * c),
            ));
        }

        a += da;
    }

    shape(vec![closed_contour(points)])
}
