//! Flattening of curve spans to polylines — used **only** to locate
//! intersections. Output geometry is built from true curve pieces, so
//! the flatten tolerance bounds intersection *detection*, not output
//! fidelity (see the decision doc, `docs/decisions/0003-path-booleans.md`).
//!
//! Deviation bound: the curve lies in the convex hull of its control
//! points, so when both inner control points sit within
//! [`FLATTEN_TOLERANCE`] of the chord, the whole arc does too.
//!
//! Emission scheme: the caller emits the root start `(0, p0)` and root
//! end `(1, p3)`; every node with `t0 > 0` emits its own start. Every
//! span end is either the next sibling's start or the root end, and
//! the leftmost chain's `t0`-duplicate is skipped — the parameter
//! sequence is complete and strictly ascending with no duplicates.

use geom::Point;
use kurbo::{CubicBez, ParamCurve, Vec2};

/// Maximum deviation of the flattened proxy from the true curve (px).
pub const FLATTEN_TOLERANCE: f64 = 0.05;

/// Recursion ceiling: each level halves the parameter span; 24 levels
/// resolve any curve representable in f64 coordinates well past f64
/// precision. Reached only for degenerate input, where the remaining
/// control polygon is emitted as a chord.
const MAX_DEPTH: u32 = 24;

/// A flattened span: `(t, point)` pairs at ascending `t`, always
/// starting at `(0.0, p0)` and ending at `(1.0, p3)`.
pub type Polyline = Vec<(f64, Point)>;

/// Flattens one cubic to within [`FLATTEN_TOLERANCE`].
#[must_use]
pub fn flatten_cubic(c: CubicBez) -> Polyline {
    let mut out = Polyline::new();
    out.push((0.0, Point::new(c.p0.x, c.p0.y)));
    flatten_rec(&c, 0.0, 1.0, 0, &mut out);
    out.push((1.0, Point::new(c.p3.x, c.p3.y)));
    out
}

fn flatten_rec(c: &CubicBez, t0: f64, t1: f64, depth: u32, out: &mut Polyline) {
    if depth >= MAX_DEPTH || is_flat(c, FLATTEN_TOLERANCE) {
        // `t0 > 0` skips the leftmost subdivision chain: its start is
        // the root start the caller already emitted.
        if t0 > 0.0 {
            out.push((t0, Point::new(c.p0.x, c.p0.y)));
        }
        return;
    }
    let tm = 0.5 * (t0 + t1);
    let (a, b) = c.subdivide();
    flatten_rec(&a, t0, tm, depth + 1, out);
    flatten_rec(&b, tm, t1, depth + 1, out);
}

/// Both inner control points within `tol` of the chord p0→p3.
fn is_flat(c: &CubicBez, tol: f64) -> bool {
    let chord = Vec2::new(c.p3.x - c.p0.x, c.p3.y - c.p0.y);
    let len = chord.hypot();
    if len < 1e-12 {
        // Degenerate chord: measure against the start point itself.
        let d1 = (c.p1 - c.p0).hypot();
        let d2 = (c.p2 - c.p0).hypot();
        return d1 <= tol && d2 <= tol;
    }
    let within = |p: kurbo::Point| chord_cross(chord, p - c.p0).abs() / len <= tol;
    within(c.p1) && within(c.p2)
}

/// 2D cross product of two vectors.
fn chord_cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}
