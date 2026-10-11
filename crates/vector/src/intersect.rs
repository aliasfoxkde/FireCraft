//! Intersection of flattened segments — the probe that locates where
//! true curve pieces must split.
//!
//! Parallel/collinear segment pairs return `None`: a shared edge never
//! *crosses*, and region assembly (winding classification at a side
//! offset) resolves shared-boundary cases without splits. This is the
//! documented tolerance-pinned limitation for exact tangency (decision
//! doc, "Limitations").

use geom::Point;

/// Crossing gate for the denominator: below this the segments are
/// treated as parallel (no transversal intersection).
const EPS_PARALLEL: f64 = 1e-12;

/// Parametric intersection of segments p1→p2 and p3→p4.
///
/// Returns `(t, u)` with the crossing at `p1 + t·(p2−p1)` =
/// `p3 + u·(p4−p3)`, both in `[0, 1]`, or `None` (including
/// parallel/collinear pairs and endpoints-only touching is still
/// reported — splitting at a shared endpoint is harmless: it is
/// deduplicated and produces a zero-length piece that drops out).
#[must_use]
pub fn segment_intersection(p1: Point, p2: Point, p3: Point, p4: Point) -> Option<(f64, f64)> {
    let d1x = p2.x - p1.x;
    let d1y = p2.y - p1.y;
    let d2x = p4.x - p3.x;
    let d2y = p4.y - p3.y;
    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < EPS_PARALLEL {
        return None;
    }
    let ex = p3.x - p1.x;
    let ey = p3.y - p1.y;
    let t = (ex * d2y - ey * d2x) / denom;
    let u = (ex * d1y - ey * d1x) / denom;
    if (-1e-9..=1.0 + 1e-9).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&u) {
        Some((t.clamp(0.0, 1.0), u.clamp(0.0, 1.0)))
    } else {
        None
    }
}

/// Sorts and deduplicates split parameters, dropping duplicates within
/// `eps` and parameters at the span ends (no split there). Deterministic:
/// total order on f64, stable merge.
#[must_use]
pub fn dedup_params(mut ts: Vec<f64>, eps: f64) -> Vec<f64> {
    ts.sort_by(f64::total_cmp);
    let mut out: Vec<f64> = Vec::new();
    for t in ts {
        if !(eps..=1.0 - eps).contains(&t) {
            continue;
        }
        match out.last() {
            Some(prev) if (t - *prev).abs() < eps => {}
            _ => out.push(t),
        }
    }
    out
}
