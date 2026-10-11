//! Curve-aware path booleans: union, intersect, punch, crop.
//!
//! Pipeline (decision doc `docs/decisions/0003-path-booleans.md`):
//!
//! 1. **Prepare** — each operand's contours become true segments
//!    ([`Seg`]) plus a flattened probe polyline per segment
//!    (tolerance [`FLATTEN_TOLERANCE`]).
//! 2. **Probe** — every cross-operand pair of flat segments is tested
//!    for a transversal crossing; each hit maps back to a parameter on
//!    the *true* segment via the probe's recorded `(t0, t1)` range.
//! 3. **Split** — true segments are split at those parameters with
//!    exact curve math (`kurbo::subcurve`), never the proxy.
//! 4. **Classify** — each piece is sampled at three stations, a
//!    [`SIDE_OFFSET`] to each side; a side's inside-ness under the op's
//!    predicate is the winding number of the corresponding operand at
//!    the sample (parity for even-odd operands). A piece is boundary
//!    when a majority of stations straddle with one orientation; it is
//!    directed so the result interior lies to its left. Majority voting
//!    both rejects pieces tangent to the region at a single point and
//!    keeps pieces that bound a sub-tolerance sliver at one end.
//! 5. **Stitch** — boundary pieces chain end-to-start on the [`SNAP`]
//!    grid into closed loops; failure is [`VectorError::OpenChain`],
//!    never a guess.
//! 6. **Assemble** — loops become closed contours in deterministic
//!    order (first-anchor `total_cmp`); metadata is neutral
//!    (opaque, normal blend, nonzero fill — the loops carry their own
//!    winding, so even-odd need not survive).

use crate::bridge::{Seg, contour_segments};
use crate::flatten::flatten_cubic;
use crate::intersect::{dedup_params, segment_intersection};
use crate::split::{Piece, split_seg};
use crate::{SIDE_OFFSET, VectorError, snap_key};
use doc::model::{Contour, PathObject, PathPoint};
use geom::Point;
use kurbo::ParamCurve;

/// Duplicate-parameter gate for split parameters (normalized 0–1 on a
/// segment). Flat-probe endpoints land within this of a shared anchor.
const EPS_PARAM: f64 = 1e-9;

/// A chord hit is a real crossing only when strictly interior to the
/// chord; endpoint touches are anchor artifacts (see the probe loops).
fn chord_interior(t: f64) -> bool {
    t > EPS_PARAM && t < 1.0 - EPS_PARAM
}

/// Which combine operation to perform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOp {
    /// Union of both operands' regions.
    Union,
    /// Intersection of both operands' regions.
    Intersect,
    /// `a` minus `b`: the second operand punches through the first
    /// (Fireworks order: front object punches back object).
    Punch,
    /// Crop: intersection, kept as a distinct op for command-name
    /// parity with Fireworks (Modify ▸ Combine Paths ▸ Crop).
    Crop,
}

/// One prepared operand: true segments, flattened probes (each pointing
/// back at its segment with its parameter range), and the fill rule.
struct Operand {
    segs: Vec<Seg>,
    flats: Vec<FlatSeg>,
    /// Per flat probe: `(contour index, span index)` of its true segment.
    span_owner: Vec<(usize, usize)>,
    /// Span count per contour (for adjacency exclusion).
    spans_per_contour: Vec<usize>,
    /// Whether each contour is closed (wrap-span adjacency).
    contour_closed: Vec<bool>,
    even_odd: bool,
}

struct FlatSeg {
    seg: usize,
    a: Point,
    b: Point,
    ta: f64,
    tb: f64,
}

fn prepare(path: &PathObject) -> Operand {
    let mut segs = Vec::new();
    let mut flats = Vec::new();
    let mut span_owner = Vec::new();
    let mut spans_per_contour = Vec::new();
    let mut contour_closed = Vec::new();
    for (contour_idx, contour) in path.contours.iter().enumerate() {
        let first_span = segs.len();
        for (local, seg) in contour_segments(contour).into_iter().enumerate() {
            let (a, b) = (seg.start(), seg.end());
            let poly = match seg {
                Seg::Cubic(c) => flatten_cubic(c),
                Seg::Line(_) => vec![(0.0, a), (1.0, b)],
            };
            let seg_idx = segs.len();
            for w in poly.windows(2) {
                flats.push(FlatSeg {
                    seg: seg_idx,
                    a: w[0].1,
                    b: w[1].1,
                    ta: w[0].0,
                    tb: w[1].0,
                });
                span_owner.push((contour_idx, local));
            }
            segs.push(seg);
        }
        spans_per_contour.push(segs.len() - first_span);
        contour_closed.push(contour.closed);
    }
    Operand {
        segs,
        flats,
        span_owner,
        spans_per_contour,
        contour_closed,
        even_odd: path.even_odd,
    }
}

/// Combines two paths. Operand order matters only for [`BooleanOp::Punch`]
/// (`a` − `b`); the other three are symmetric.
///
/// An empty operand participates as the empty region: union with it
/// reproduces the other operand's boundary, punch *by* it leaves the
/// other operand untouched, and intersect/crop/punch *of* it are empty.
/// (The registry command layer still enforces Fireworks'
/// two-selected-objects precondition before calling.)
///
/// # Errors
/// [`VectorError::OpenChain`] when boundary pieces fail to close (a
/// classification inconsistency or a numerically hostile case),
/// [`VectorError::DegenerateSegment`] when a piece has no usable
/// tangent for side sampling.
pub fn boolean(a: &PathObject, b: &PathObject, op: BooleanOp) -> Result<PathObject, VectorError> {
    let oa = prepare(a);
    let ob = prepare(b);

    let mut params_a: Vec<Vec<f64>> = vec![Vec::new(); oa.segs.len()];
    let mut params_b: Vec<Vec<f64>> = vec![Vec::new(); ob.segs.len()];
    for fa in &oa.flats {
        for fb in &ob.flats {
            if let Some((t, u)) = segment_intersection(fa.a, fa.b, fb.a, fb.b) {
                // Chord-endpoint touches (shared anchors between
                // consecutive probes, tangential kisses) must not
                // split: the filter is on the CHORD parameters —
                // mapping first would land at interior segment
                // params and shred the segment at every chord joint.
                if chord_interior(t) && chord_interior(u) {
                    params_a[fa.seg].push(fa.ta + t * (fa.tb - fa.ta));
                    params_b[fb.seg].push(fb.ta + u * (fb.tb - fb.ta));
                }
            }
        }
    }
    // Self-intersections split too: an unsplit self-crossing would force
    // one span to bound two region components with opposite orientation
    // (proved by the bowtie intersect case). Adjacent spans are skipped —
    // they touch at shared anchors, never transversally.
    probe_intra(&oa, &mut params_a);
    probe_intra(&ob, &mut params_b);

    let mut boundary: Vec<Piece> = Vec::new();
    // Both passes classify against the SAME global region predicate —
    // punching must see the second operand's boundary as hole edge, so
    // the predicate is never swapped per source.
    collect_boundary(&oa, &params_a, (&oa, &ob), op, &mut boundary)?;
    collect_boundary(&ob, &params_b, (&oa, &ob), op, &mut boundary)?;

    let loops = stitch(boundary)?;
    Ok(assemble(loops))
}

/// Probes an operand's flat segments against each other, recording
/// split parameters for non-adjacent span pairs.
fn probe_intra(op: &Operand, params: &mut [Vec<f64>]) {
    for i in 0..op.flats.len() {
        for j in (i + 1)..op.flats.len() {
            let (ci, si) = op.span_owner[i];
            let (cj, sj) = op.span_owner[j];
            if ci == cj && spans_adjacent(si, sj, op.spans_per_contour[ci], op.contour_closed[ci]) {
                continue;
            }
            let (fa, fb) = (&op.flats[i], &op.flats[j]);
            if let Some((t, u)) = segment_intersection(fa.a, fa.b, fb.a, fb.b) {
                if chord_interior(t) && chord_interior(u) {
                    params[fa.seg].push(fa.ta + t * (fa.tb - fa.ta));
                    params[fb.seg].push(fb.ta + u * (fb.tb - fb.ta));
                }
            }
        }
    }
}

/// True when two spans of one contour touch at a shared anchor (wrap
/// adjacency included for closed contours).
fn spans_adjacent(a: usize, b: usize, span_count: usize, closed: bool) -> bool {
    if span_count < 2 {
        return true;
    }
    if a + 1 == b || b + 1 == a {
        return true;
    }
    closed && (a == 0 && b == span_count - 1 || b == 0 && a == span_count - 1)
}

/// Splits one operand at its parameters, classifies each piece against
/// the global region, and appends directed boundary pieces to `out`.
fn collect_boundary(
    source: &Operand,
    params: &[Vec<f64>],
    region: (&Operand, &Operand),
    op: BooleanOp,
    out: &mut Vec<Piece>,
) -> Result<(), VectorError> {
    for (seg_idx, seg) in source.segs.iter().enumerate() {
        let ts = dedup_params(params[seg_idx].clone(), EPS_PARAM);
        for piece in split_seg(seg, &ts) {
            // Classify at three stations; the piece is boundary when a
            // MAJORITY of stations straddle (one side in, one out) with
            // one consistent orientation, and is directed so the result
            // interior lies to its left. Majority (not unanimity)
            // because a piece may legitimately bound a sub-tolerance
            // sliver at one end (the wedge regression: the sliver
            // between a lobe edge and the crossing narrows to zero at
            // the split point, so one station reads interior); majority
            // (not midpoint-only) because a piece tangent to the region
            // at a single point — the inscribed-circle kiss, where
            // each square edge's midpoint IS the kiss — straddles at
            // exactly one station and must drop.
            let mut left_votes = 0u32;
            let mut straddles = 0u32;
            for station in [0.25, 0.5, 0.75] {
                let (mid, tangent) = piece_frame(&piece, station)?;
                let nx = -tangent.y;
                let ny = tangent.x;
                let left = Point::new(mid.x + nx * SIDE_OFFSET, mid.y + ny * SIDE_OFFSET);
                let right = Point::new(mid.x - nx * SIDE_OFFSET, mid.y - ny * SIDE_OFFSET);
                let l_in = region_contains(region, op, left);
                let r_in = region_contains(region, op, right);
                if l_in != r_in {
                    straddles += 1;
                    if l_in {
                        left_votes += 1;
                    }
                }
            }
            if straddles >= 2 && (left_votes == 0 || left_votes == straddles) {
                if left_votes == straddles {
                    out.push(piece);
                } else {
                    out.push(piece.reversed());
                }
            }
        }
    }
    Ok(())
}

/// The op's containment predicate at `p`, over the global operand pair
/// (always `a` then `b`, regardless of which operand supplied the piece).
fn region_contains(region: (&Operand, &Operand), op: BooleanOp, p: Point) -> bool {
    let in_a = operand_contains(region.0, p);
    let in_b = operand_contains(region.1, p);
    match op {
        BooleanOp::Union => in_a || in_b,
        BooleanOp::Intersect | BooleanOp::Crop => in_a && in_b,
        BooleanOp::Punch => in_a && !in_b,
    }
}

/// Fill-rule containment of one operand at `p`.
fn operand_contains(op: &Operand, p: Point) -> bool {
    let w = winding(&op.flats, p);
    if op.even_odd { w % 2 != 0 } else { w != 0 }
}

/// Winding number of the flattened probe set at `p` (ray cast toward +x).
fn winding(flats: &[FlatSeg], p: Point) -> i32 {
    let mut w = 0i32;
    for f in flats {
        if (f.a.y > p.y) != (f.b.y > p.y) {
            let dy = f.b.y - f.a.y;
            let x_cross = f.a.x + (p.y - f.a.y) * (f.b.x - f.a.x) / dy;
            if x_cross > p.x {
                w += if dy > 0.0 { 1 } else { -1 };
            }
        }
    }
    w
}

/// Point and tangent direction of a piece at parameter `t`. The tangent
/// comes from a short chord so stationary points still yield a direction.
fn piece_frame(piece: &Piece, t: f64) -> Result<(Point, Point), VectorError> {
    let at = |u: f64| match piece.geom {
        Seg::Line(l) => l.eval(u),
        Seg::Cubic(c) => c.eval(u),
    };
    let m = at(t);
    let d = at(t + 0.1) - at(t - 0.1);
    let len = d.hypot();
    if len < 1e-12 {
        return Err(VectorError::DegenerateSegment);
    }
    Ok((Point::new(m.x, m.y), Point::new(d.x / len, d.y / len)))
}

/// Chains directed boundary pieces into closed loops on the [`SNAP`]
/// grid. At a junction where several pieces start (a degree-4 crossing),
/// the walk takes the first candidate counterclockwise from the reversed
/// incoming direction — the standard face-walk rule for keeping the
/// region interior on the walker's left. Ties break by piece index, so
/// the walk is deterministic.
fn stitch(pieces: Vec<Piece>) -> Result<Vec<Vec<Piece>>, VectorError> {
    let mut used = vec![false; pieces.len()];
    let mut loops = Vec::new();
    for start in 0..pieces.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let first_key = snap_key(pieces[start].start);
        let mut chain = vec![pieces[start].clone()];
        let mut cur_end = pieces[start].end;
        let mut cur_dir = piece_direction(&pieces[start]);
        while snap_key(cur_end) != first_key {
            let reverse = Point::new(-cur_dir.x, -cur_dir.y);
            let mut best: Option<(f64, usize)> = None;
            for (j, p) in pieces.iter().enumerate() {
                if used[j] || snap_key(p.start) != snap_key(cur_end) {
                    continue;
                }
                let angle = ccw_angle(reverse, piece_direction(p));
                if best.is_none() || angle < best.map_or(f64::INFINITY, |(a, _)| a) {
                    best = Some((angle, j));
                }
            }
            let (_, j) = best.ok_or(VectorError::OpenChain)?;
            used[j] = true;
            cur_end = pieces[j].end;
            cur_dir = piece_direction(&pieces[j]);
            chain.push(pieces[j].clone());
            if chain.len() > pieces.len() {
                return Err(VectorError::OpenChain);
            }
        }
        loops.push(chain);
    }
    Ok(loops)
}

/// Chord direction of a piece (never zero: slivers are dropped at
/// split time).
fn piece_direction(p: &Piece) -> Point {
    Point::new(p.end.x - p.start.x, p.end.y - p.start.y)
}

/// Counterclockwise angle from `r` to `d` in `[0, 2π)`.
fn ccw_angle(r: Point, d: Point) -> f64 {
    let dot = r.x * d.x + r.y * d.y;
    let cross = r.x * d.y - r.y * d.x;
    let a = f64::atan2(cross, dot);
    if a < 0.0 {
        a + std::f64::consts::TAU
    } else {
        a
    }
}

/// Loops → closed contours in deterministic order, wrapped in a neutral
/// [`PathObject`].
fn assemble(loops: Vec<Vec<Piece>>) -> PathObject {
    let mut contours: Vec<Contour> = loops
        .into_iter()
        .map(|chain| {
            let n = chain.len();
            let mut points = Vec::with_capacity(n);
            for (i, piece) in chain.iter().enumerate() {
                let prev = &chain[(i + n - 1) % n];
                let secondary = match piece.geom {
                    Seg::Cubic(c) => Some(Point::new(c.p1.x, c.p1.y)),
                    Seg::Line(_) => None,
                };
                let primary = match prev.geom {
                    Seg::Cubic(c) => Some(Point::new(c.p2.x, c.p2.y)),
                    Seg::Line(_) => None,
                };
                points.push(PathPoint {
                    anchor: piece.start,
                    primary,
                    secondary,
                    is_curve: primary.is_some() || secondary.is_some(),
                });
            }
            Contour {
                closed: true,
                point_count: n as i64,
                points,
            }
        })
        .collect();
    contours.sort_by(|x, y| {
        let (cx, cy) = (x.points[0].anchor, y.points[0].anchor);
        cx.x.total_cmp(&cy.x).then(cx.y.total_cmp(&cy.y))
    });
    PathObject {
        contours,
        primitive: None,
        // The loops are emitted closed and consistently wound, so the
        // nonzero rule reproduces the result region exactly.
        even_odd: false,
        paint: None,
        fill_start: None,
        fill_end: None,
        fill_third: None,
        brush_placement: None,
        fill_on_top: None,
        locked: false,
        visible: true,
        opacity: 1000,
        blend: 0,
        name: None,
    }
}
