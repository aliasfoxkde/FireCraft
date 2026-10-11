//! Tolerance-driven point reduction (Fireworks: Modify ▸ Alter Path ▸
//! Simplify).
//!
//! Scope, pinned by the tests: simplification reduces redundant corner
//! anchors in *linear runs* — maximal chains of line spans between curve
//! anchors — by Ramer–Douglas–Peucker at `tolerance` px. Curve spans and
//! their bounding anchors pass through untouched: merging cubic spans
//! requires curve fitting, which has no recorded CS6 ground truth yet
//! (corpus-gated; revisit when `corpus/incoming` carries simplify
//! evidence). Curve geometry therefore survives simplification exactly.
//!
//! Idempotence: RDP's stop condition is "every interior deviation is
//! within tolerance", checked before anything new is kept, so running
//! the reduction on its own output keeps the same anchors byte-for-byte
//! (proved by a test).

use crate::VectorError;
use doc::model::{Contour, PathObject, PathPoint};
use geom::Point;

/// Reduces each contour's linear runs to the anchors RDP keeps at
/// `tolerance` px. `tolerance <= 0` reduces nothing (any deviation is
/// "over" a non-positive limit only when exactly zero-deviation points
/// are dropped — refused instead, so callers pin the tolerance).
///
/// # Errors
/// [`VectorError::DegenerateSegment`] when a linear run's chord is
/// degenerate (cannot measure deviation).
pub fn simplify(path: &PathObject, tolerance: f64) -> Result<PathObject, VectorError> {
    if tolerance <= 0.0 || !tolerance.is_finite() {
        return Err(VectorError::DegenerateSegment);
    }
    let contours = path
        .contours
        .iter()
        .map(|contour| simplify_contour(contour, tolerance))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PathObject {
        contours,
        ..path.clone()
    })
}

fn simplify_contour(contour: &Contour, tolerance: f64) -> Result<Contour, VectorError> {
    let points = &contour.points;
    if points.len() < 3 {
        return Ok(contour.clone());
    }
    let n = points.len();
    let spans = if contour.closed { n } else { n - 1 };
    // A span is a line when its paired handles are absent (the bridge
    // rule); reads the same working data the bridge reads.
    let span_is_line = |i: usize| {
        let a = &points[i];
        let b = &points[(i + 1) % n];
        a.secondary.is_none() || b.primary.is_none()
    };

    // Keep the anchors bounding curve spans; reduce only between them.
    let mut keep = vec![false; n];
    for i in 0..spans {
        if !span_is_line(i) {
            keep[i] = true;
            keep[(i + 1) % n] = true;
        }
    }

    // Maximal runs of consecutive line spans, in path order. A run's
    // bounding anchors survive by construction (they bound a curve span
    // or are the contour's ends); only interior anchors compete.
    let mut out = vec![false; n];
    let mut i = 0usize;
    while i < spans {
        if !span_is_line(i) {
            i += 1;
            continue;
        }
        let start = i;
        let mut end = i;
        while end + 1 < spans && span_is_line(end + 1) {
            end += 1;
        }
        // A run reaching the wrap span of a closed contour would need
        // wrap-aware RDP (its last anchor is anchor 0 again) — left
        // unchanged for now (corpus-gated, same as curve fitting).
        if contour.closed && end + 1 == n {
            return Ok(contour.clone());
        }
        // The run covers spans start..=end: anchors start..=end+1.
        reduce_run(points, start, end + 1, tolerance, &mut out)?;
        i = end + 1;
    }

    let kept_points: Vec<PathPoint> = points
        .iter()
        .enumerate()
        .filter(|(idx, _)| out[*idx] || keep[*idx])
        .map(|(_, p)| p.clone())
        .collect();
    // Never collapse below the well-formed floor: a contour that would
    // lose its closure (fewer than 3 anchors on a closed contour, 2 on
    // an open one) passes through unchanged.
    let floor = if contour.closed { 3 } else { 2 };
    if kept_points.len() < floor {
        return Ok(contour.clone());
    }
    Ok(Contour {
        closed: contour.closed,
        point_count: kept_points.len() as i64,
        points: kept_points,
    })
}

/// RDP over the anchor polyline `points[first..=last]` (consecutive
/// line spans). Keeps `first`/`last` and every anchor whose deviation
/// from its chord exceeds `tolerance`; marks survivors in `out`.
fn reduce_run(
    points: &[PathPoint],
    first: usize,
    last: usize,
    tolerance: f64,
    out: &mut [bool],
) -> Result<(), VectorError> {
    out[first] = true;
    out[last] = true;
    if last - first < 2 {
        return Ok(());
    }
    // Explicit stack (recursion depth on adversarial collinear input is
    // unbounded otherwise).
    let mut stack = vec![(first, last)];
    while let Some((a, b)) = stack.pop() {
        if b - a < 2 {
            continue;
        }
        let (pa, pb) = (points[a].anchor, points[b].anchor);
        let (mut best_idx, mut best_dist) = (a, -1.0_f64);
        for (offset, candidate) in points[a + 1..b].iter().enumerate() {
            let d = point_segment_distance(candidate.anchor, pa, pb);
            if d > best_dist {
                best_dist = d;
                best_idx = a + 1 + offset;
            }
        }
        if best_dist > tolerance {
            out[best_idx] = true;
            stack.push((a, best_idx));
            stack.push((best_idx, b));
        }
    }
    Ok(())
}

/// Perpendicular distance from `p` to the segment `a`–`b` (for a
/// degenerate chord: the distance to `a`).
fn point_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let (vx, vy) = (b.x - a.x, b.y - a.y);
    let len2 = vx * vx + vy * vy;
    if len2 < 1e-18 {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = (((p.x - a.x) * vx + (p.y - a.y) * vy) / len2).clamp(0.0, 1.0);
    let (cx, cy) = (a.x + t * vx, a.y + t * vy);
    (p.x - cx).hypot(p.y - cy)
}
