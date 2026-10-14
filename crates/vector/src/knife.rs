//! Knife cuts (decision 0032): splitting contours against a click
//! point or a drag segment. Same pipeline contract as the booleans —
//! a flattened proxy *finds* the cut positions; the output geometry is
//! true curve math (kurbo `subsegment`), never the proxy.
//!
//! Drag semantics (`from ≠ to`): every crossing of the blade with a
//! contour cuts, and a contour that does not cross but lies within
//! `tolerance` of the blade is cut at its nearest position — the dom's
//! "cuts additional items within the specified tolerance". Click
//! semantics (`from == to`) are the degenerate blade: one cut at the
//! nearest contour position within tolerance.
//!
//! Deviations are recorded in the decision: parallel-segment distance
//! uses the four end-point projections (exact except near-parallel
//! near-touching chords, which the crossing pass catches anyway), and
//! a nearest position that lands on a span boundary is not a cut —
//! splitting at an anchor of an open contour changes nothing.

use crate::SNAP;
use crate::VectorError;
use crate::bridge::{Seg, bezpath_to_contour, contour_segments};
use crate::flatten::flatten_cubic;
use crate::intersect::{dedup_params, segment_intersection};
use crate::split::Piece;
use doc::Contour;
use geom::Point;
use kurbo::ParamCurve;

/// One cut piece of a contour: true geometry between cut positions.
struct Cut {
    piece: Piece,
    /// A blade cut sits immediately before this piece.
    cut: bool,
}

/// Cuts one contour against the blade `from → to`. Returns `None` when
/// the blade cuts nothing (the dom's `false`).
#[must_use]
pub fn knife_contour(
    contour: &Contour,
    from: Point,
    to: Point,
    tolerance: f64,
) -> Option<Vec<Contour>> {
    let segs = contour_segments(contour);
    if segs.is_empty() {
        return None;
    }
    let degenerate = from == to;
    // Per-span interior cut parameters, plus the nearest position on
    // the whole contour to the blade (the tolerance clause's target).
    let mut params: Vec<Vec<f64>> = segs.iter().map(|_| Vec::new()).collect();
    let mut nearest: Option<(f64, usize, f64)> = None;
    for (index, seg) in segs.iter().enumerate() {
        let chords = match *seg {
            Seg::Line(l) => vec![(0.0, l.p0), (1.0, l.p1)],
            Seg::Cubic(c) => flatten_cubic(c).ok()?,
        };
        for pair in chords.windows(2) {
            let (ta, a) = (pair[0].0, pair[0].1);
            let (tb, b) = (pair[1].0, pair[1].1);
            let Some((t, u)) = segment_intersection(a, b, from, to) else {
                continue;
            };
            if t > 0.0 && t < 1.0 && u > 0.0 && u < 1.0 {
                params[index].push(ta + t * (tb - ta));
            }
        }
        let (distance, position) = nearest_on_blade(&chords, from, to, degenerate);
        if nearest.is_none_or(|(best, _, _)| distance < best) {
            nearest = Some((distance, index, position));
        }
    }
    for ts in &mut params {
        *ts = dedup_params(std::mem::take(ts), SNAP);
    }
    if params.iter().all(|ts| ts.is_empty()) {
        // No crossing: the tolerance clause cuts the nearest contour
        // within reach, at the nearest position (span boundaries
        // excepted — cutting there changes nothing).
        let (distance, index, position) = nearest?;
        if distance > tolerance || !chord_interior(position) {
            return None;
        }
        params[index].push(position);
    }
    let pieces = cut_pieces(&segs, &params);
    let cut_count = pieces.iter().filter(|entry| entry.cut).count();
    if cut_count == 0 {
        return None;
    }
    let mut pieces = pieces;
    if contour.closed {
        // Open the ring at the first cut: the pieces after it run to
        // the pieces before it, and the head starts at the cut point.
        let head = pieces.iter().position(|entry| entry.cut)?;
        pieces.rotate_left(head);
        pieces[0].cut = false;
    }
    let mut runs: Vec<Vec<Piece>> = vec![Vec::new()];
    for entry in pieces {
        if entry.cut {
            runs.push(Vec::new());
        }
        let last = runs.len() - 1;
        runs[last].push(entry.piece);
    }
    runs.retain(|run| !run.is_empty());
    let contours = runs
        .iter()
        .map(|run| pieces_to_contour(run))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    Some(contours)
}

/// Cuts every contour of a path's contour list. Contours the blade
/// misses pass through untouched; `None` when the blade cuts nothing.
#[must_use]
pub fn knife_contours(
    contours: &[Contour],
    from: Point,
    to: Point,
    tolerance: f64,
) -> Option<Vec<Contour>> {
    let mut out = Vec::new();
    let mut cut_any = false;
    for contour in contours {
        match knife_contour(contour, from, to, tolerance) {
            Some(mut pieces) => {
                cut_any = true;
                out.append(&mut pieces);
            }
            None => out.push(contour.clone()),
        }
    }
    if cut_any { Some(out) } else { None }
}

/// Cuts strictly inside a span; a boundary cut changes nothing.
fn chord_interior(t: f64) -> bool {
    t > SNAP && t < 1.0 - SNAP
}

/// True-curve pieces with cut markers: boundaries between consecutive
/// pieces of one span are cuts (a dropped sliver hands its marker to
/// the next surviving piece of the same span).
fn cut_pieces(segs: &[Seg], params: &[Vec<f64>]) -> Vec<Cut> {
    let mut out = Vec::new();
    for (index, seg) in segs.iter().enumerate() {
        let ts = &params[index];
        if ts.is_empty() {
            out.push(Cut {
                piece: whole_piece(seg),
                cut: false,
            });
            continue;
        }
        let mut bounds = Vec::with_capacity(ts.len() + 2);
        bounds.push(0.0);
        bounds.extend_from_slice(ts);
        bounds.push(1.0);
        for (window_index, window) in bounds.windows(2).enumerate() {
            let geom = sub_piece(seg, window[0], window[1]);
            let piece = Piece {
                start: geom.start(),
                end: geom.end(),
                geom,
            };
            if chord(piece.start, piece.end) < SNAP {
                continue; // sliver: the marker rides the next survivor
            }
            out.push(Cut {
                piece,
                cut: window_index > 0,
            });
        }
    }
    out
}

fn whole_piece(seg: &Seg) -> Piece {
    Piece {
        start: seg.start(),
        end: seg.end(),
        geom: *seg,
    }
}

fn sub_piece(seg: &Seg, a: f64, b: f64) -> Seg {
    match *seg {
        Seg::Line(l) => Seg::Line(kurbo::Line::new(lerp(l.p0, l.p1, a), lerp(l.p0, l.p1, b))),
        Seg::Cubic(c) => Seg::Cubic(c.subsegment(a..b)),
    }
}

fn lerp(p0: kurbo::Point, p1: kurbo::Point, t: f64) -> kurbo::Point {
    kurbo::Point::new(p0.x + (p1.x - p0.x) * t, p0.y + (p1.y - p0.y) * t)
}

fn chord(a: Point, b: Point) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}

/// The nearest (distance, parameter) from the blade to a chord proxy.
/// A degenerate blade measures point distance with a clamped
/// projection; otherwise the four end-point projections bound the
/// segment distance (recorded approximation). The parameter is the
/// nearest chord's midpoint — chord granularity, which is the
/// flatten tolerance the whole crate already works at.
fn nearest_on_blade(
    chords: &[(f64, Point)],
    from: Point,
    to: Point,
    degenerate: bool,
) -> (f64, f64) {
    let mut best = (f64::INFINITY, 0.0);
    for pair in chords.windows(2) {
        let (ta, a) = (pair[0].0, pair[0].1);
        let (tb, b) = (pair[1].0, pair[1].1);
        let distance = if degenerate {
            segment_projection(from, a, b).distance(from)
        } else {
            a.distance(blade_projection(a, from, to))
                .min(b.distance(blade_projection(b, from, to)))
                .min(from.distance(segment_projection(from, a, b)))
                .min(to.distance(segment_projection(to, a, b)))
        };
        if distance < best.0 {
            best = (distance, (ta + tb) * 0.5);
        }
    }
    best
}

/// The clamped projection of `point` onto the segment `a → b`.
fn segment_projection(point: Point, a: Point, b: Point) -> Point {
    let ab = b - a;
    let length2 = ab.dot(ab);
    if length2 <= 0.0 {
        return a;
    }
    let s = ((point - a).dot(ab) / length2).clamp(0.0, 1.0);
    a + ab * s
}

/// The clamped projection of `point` onto the blade segment.
fn blade_projection(point: Point, from: Point, to: Point) -> Point {
    segment_projection(point, from, to)
}

/// Reassembles one run of cut pieces into an open contour.
fn pieces_to_contour(run: &[Piece]) -> Result<Contour, VectorError> {
    let Some(first) = run.first() else {
        return Err(VectorError::EmptyPath);
    };
    let mut path = kurbo::BezPath::new();
    path.move_to(first.geom.start());
    for piece in run {
        match piece.geom {
            Seg::Line(l) => path.line_to(l.p1),
            Seg::Cubic(c) => path.curve_to(c.p1, c.p2, c.p3),
        }
    }
    bezpath_to_contour(&path)
}
