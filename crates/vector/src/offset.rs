//! Constant-distance offsetting: expand stroke (Fireworks: Modify ▸
//! Alter Path ▸ Expand Stroke) and inset/outset (▸ Inset Path).
//!
//! Method: each contour's spans are offset individually (lines translate;
//! cubics by one Tiller–Hanson pass). At each vertex the offset spans
//! either overlap (the contour's connecting chord carries the excursion
//! and the self-union resolves it at the transversal crossing) or spread
//! apart (miter/bevel/round join, with the spans trimmed or extended to
//! the join point). Caps close open outlines per the requested style.
//! The raw rings — which still overlap at reflex vertices and carry cusp
//! folds where curvature radius < distance — are cleaned by the boolean
//! pipeline's self-union ([`crate::boolean`] with both operands the same
//! path: identical operands cross-probe every non-adjacent span pair,
//! and the majority-vote classifier keeps exactly the true region
//! boundary, so the result is the offset region with a resolved,
//! editable outline).
//!
//! Tolerance-pinned limitations (documented behavior, proved by tests):
//! Tiller–Hanson is a first-order approximation — offset error grows
//! with curvature × distance; features narrower than the offset distance
//! collapse (the union resolves them, possibly to empty); offsets past a
//! contour's curvature radius fold (the union resolves the folds, and
//! hostile configurations surface as [`VectorError::OpenChain`]).

use crate::SNAP;
use crate::VectorError::{self, DegenerateSegment as Degenerate};
use crate::boolean::{BooleanOp, boolean, chord_interior};
use crate::bridge::Seg;
use crate::flatten::flatten_cubic;
use crate::intersect::segment_intersection;
use doc::model::{Contour, PathObject, PathPoint};
use geom::Point;
use kurbo::{CubicBez, Line, ParamCurve};

/// How an open contour's stroke ends are closed (Expand Stroke).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapKind {
    /// Flat end at the contour's endpoint.
    Butt,
    /// Semicircular end of stroke radius.
    Round,
    /// Square end extending the contour by stroke radius.
    Square,
}

/// How a join between two offset spans is filled (Expand Stroke and
/// inset corners on the join side).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinKind {
    /// Circular arc of offset radius.
    Round,
    /// Straight chord between the offset span ends.
    Bevel,
    /// Extend both spans to their intersection, up to `miter_limit`
    /// radii; bevel past it.
    Miter,
}

/// Miter length limit in radii used when no explicit style applies
/// (inset/outset); matches the Expand Stroke default.
pub const DEFAULT_MITER_LIMIT: f64 = 4.0;

/// Stroke geometry for [`expand_stroke`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    /// Full stroke width (px); offsets use `width / 2` per side.
    pub width: f64,
    /// End treatment for open contours.
    pub cap: CapKind,
    /// Corner treatment on the join side.
    pub join: JoinKind,
    /// Miter length limit, in radii, before falling back to bevel.
    pub miter_limit: f64,
}

/// The filled outline of stroking every contour of `path` with `style`.
/// Closed contours contribute two rings (one per side); open contours
/// contribute one closed outline (left side, end cap, right side, start
/// cap). The raw rings are resolved through the boolean self-union.
///
/// # Errors
/// [`VectorError::DegenerateSegment`] for a non-finite or non-positive
/// width or a span with no usable tangent, [`VectorError::OpenChain`]
/// when the self-union cannot close (hostile geometry).
pub fn expand_stroke(path: &PathObject, style: &StrokeStyle) -> Result<PathObject, VectorError> {
    let d = style.width / 2.0;
    validate_distance(d)?;
    validate_distance(style.miter_limit)?;

    let mut raw_contours = Vec::new();
    for contour in &path.contours {
        let spans = crate::contour_segments(contour);
        if spans.is_empty() {
            continue;
        }
        if contour.closed {
            raw_contours.extend(rings_closed(&spans, d, style)?);
        } else {
            raw_contours.push(ring_open(&spans, d, style)?);
        }
    }
    if raw_contours.is_empty() {
        return Ok(neutral(Vec::new()));
    }
    let raw = neutral(raw_contours);
    boolean(&raw, &raw, BooleanOp::Union)
}

/// Offsets every closed contour of `path` by `distance` px: positive
/// outsets (enlarges the region), negative insets (shrinks it). Open
/// contours are closed by their start–end chord first. Contour
/// orientation is honored: the offset always moves the boundary away
/// from the contour's interior for positive distances, so holes inset in
/// the same direction as the body they belong to.
///
/// # Errors
/// [`VectorError::DegenerateSegment`] for a non-finite distance or a
/// span with no usable tangent, [`VectorError::OpenChain`] when the
/// self-union cannot close (hostile geometry).
pub fn offset_path(path: &PathObject, distance: f64) -> Result<PathObject, VectorError> {
    validate_finite(distance)?;
    let mut raw_contours = Vec::new();
    for contour in &path.contours {
        let mut spans = crate::contour_segments(contour);
        if spans.is_empty() {
            continue;
        }
        if !contour.closed {
            // Implicit close: start–end chord completes the ring.
            if let (Some(first), Some(last)) = (spans.first(), spans.last()) {
                let (a, b) = (last.end(), first.start());
                if chord(a, b) >= SNAP {
                    spans.push(Seg::Line(Line::new(to_kurbo(a), to_kurbo(b))));
                }
            }
        }
        if spans.len() < 2 {
            continue;
        }
        // Outsetting moves the boundary away from the oriented interior
        // (right of travel); insetting moves it into the interior (left
        // of travel). Both directions are orientation-independent: a CW
        // hole's oriented interior is the body it belongs to, so an
        // inset thins the body and the hole grows — holes and bodies
        // move the same way, as one edit.
        let side = -distance;
        // Inset/outset keeps true-offset corner semantics: miter joins
        // preserve polygon corners exactly (an inset square stays a
        // square); the miter limit bevels spikes the way the join-side
        // analysis dictates.
        let chain = offset_chain(&spans, side, true, (JoinKind::Miter, DEFAULT_MITER_LIMIT))?;
        if let Some(ring) = chain_to_contour(&chain) {
            raw_contours.push(ring);
        }
    }
    if raw_contours.is_empty() {
        return Ok(neutral(Vec::new()));
    }
    let raw = neutral(raw_contours);
    boolean(&raw, &raw, BooleanOp::Union)
}

/// A signed offset distance: any finite value (negative insets).
fn validate_finite(d: f64) -> Result<(), VectorError> {
    if !d.is_finite() {
        return Err(VectorError::DegenerateSegment);
    }
    Ok(())
}

/// A radius-like quantity (half width, miter limit): finite and
/// non-negative.
fn validate_distance(d: f64) -> Result<(), VectorError> {
    if !d.is_finite() || d < 0.0 {
        return Err(VectorError::DegenerateSegment);
    }
    Ok(())
}

/// Offsets one closed contour into two rings, one per side, oriented as
/// a *band*: the inner ring (smaller |area|) is reversed, so the raw
/// region — nonzero winding of the pair — is the band itself. Both
/// rings in the same direction would wind the hole to ±2, fill it, and
/// blind the classifier to the inner boundary.
fn rings_closed(spans: &[Seg], d: f64, style: &StrokeStyle) -> Result<Vec<Contour>, VectorError> {
    let mut out = Vec::new();
    for side in [d, -d] {
        let (join, miter) = (style.join, style.miter_limit);
        let chain = offset_chain(spans, side, true, (join, miter))?;
        if let Some(ring) = chain_to_contour(&chain) {
            out.push(ring);
        }
    }
    if out.len() == 2 {
        let inner = if ring_area(&out[0]).abs() <= ring_area(&out[1]).abs() {
            0
        } else {
            1
        };
        out[inner] = reverse_contour(&out[inner]);
    }
    Ok(out)
}

/// Signed shoelace area over span endpoints (only the magnitude's
/// ordering is consumed).
fn ring_area(c: &Contour) -> f64 {
    let spans = crate::contour_segments(c);
    let mut area = 0.0;
    for span in spans {
        let (a, b) = (to_point(span.start()), to_point(span.end()));
        area += a.x * b.y - b.x * a.y;
    }
    area / 2.0
}

/// The same contour traversed in the opposite direction (handles swap
/// slots; correct for open and closed contours alike).
fn reverse_contour(c: &Contour) -> Contour {
    let points = c
        .points
        .iter()
        .rev()
        .map(|p| PathPoint {
            anchor: p.anchor,
            primary: p.secondary,
            secondary: p.primary,
            is_curve: p.is_curve,
        })
        .collect();
    Contour {
        closed: c.closed,
        point_count: c.point_count,
        points,
    }
}

/// Offsets one open contour into its single closed stroke outline:
/// left side forward, end cap, right side reversed, start cap.
fn ring_open(spans: &[Seg], d: f64, style: &StrokeStyle) -> Result<Contour, VectorError> {
    let (join, miter) = (style.join, style.miter_limit);
    let left = offset_chain(spans, d, false, (join, miter))?;
    let right = offset_chain(spans, -d, false, (join, miter))?;

    // End cap: left tail → right tail, bulging along the contour's end
    // tangent; then the right chain reversed (tail → head); then the
    // start cap back to the left chain's head.
    let end_vertex = spans[spans.len() - 1].end();
    let cap_end = cap_segs(
        end_vertex,
        chain_tail(&left)?,
        chain_tail(&right)?,
        span_tangent_at_end(&spans[spans.len() - 1])?,
        d,
        style.cap,
    )?;
    let left_head = chain_head(&left)?;
    let mut chain = left;
    chain.extend(cap_end);
    chain.extend(right.into_iter().rev().map(|seg| seg.reversed()));

    let start_vertex = spans[0].start();
    let cap_start = cap_segs(
        start_vertex,
        chain_tail(&chain)?,
        left_head,
        neg(span_tangent_at_start(&spans[0])?),
        d,
        style.cap,
    )?;
    chain.extend(cap_start);
    chain_to_contour(&chain).ok_or(VectorError::DegenerateSegment)
}

fn chain_tail(chain: &[Seg]) -> Result<Point, VectorError> {
    chain
        .last()
        .map(Seg::end)
        .ok_or(VectorError::DegenerateSegment)
}

fn chain_head(chain: &[Seg]) -> Result<Point, VectorError> {
    chain
        .first()
        .map(Seg::start)
        .ok_or(VectorError::DegenerateSegment)
}

/// Offsets every span of the chain by `dist` (signed: positive offsets
/// to the left of travel) and reconnects them. At each vertex the two
/// offset spans either *overlap* (their carrier lines cross inside both
/// spans — an explicit connecting chord carries the excursion, and the
/// self-union resolves it at the transversal crossing) or *spread
/// apart* (a join is needed). Joins retarget the span parameters:
/// miter trims or extends each span to the offset tangents'
/// intersection; bevel/round trim each span to its closest-to-vertex
/// point and bridge with a chord or arc.
fn offset_chain(
    spans: &[Seg],
    dist: f64,
    closed: bool,
    (join, miter): (JoinKind, f64),
) -> Result<Vec<Seg>, VectorError> {
    let count = spans.len();
    let offs: Vec<Seg> = spans
        .iter()
        .map(|span| offset_seg(span, dist))
        .collect::<Result<Vec<_>, _>>()?;
    let connections = if closed { count } else { count - 1 };

    // Per-span parameter window [start_param, end_param] (of the full
    // offset span) and per-connection bridge geometry from span i's
    // trimmed end to span i+1's trimmed start. Two passes: joins may
    // trim either neighbor, so all windows settle before bridges pick
    // up their endpoints.
    let mut start_param = vec![0.0_f64; count];
    let mut end_param = vec![1.0_f64; count];
    let mut joined = vec![false; count];
    let mut fillers: Vec<Vec<Seg>> = vec![Vec::new(); count];

    for i in 0..connections {
        let next = (i + 1) % count;
        let vertex = to_point(spans[i].end());
        let t_in = span_tangent_at_end(&offs[i])?;
        let t_out = span_tangent_at_start(&offs[next])?;
        let cross = t_in.x * t_out.y - t_in.y * t_out.x;
        // The offsets spread apart exactly when the turn is away from
        // the offset side: left offset (dist > 0) at a right turn, right
        // offset at a left turn. A 180° reversal (cross ≈ 0) overlaps
        // collinearly — no join, the union resolves it.
        if cross * dist < -1e-12 {
            let (ep, sp, fill) = join_at(
                &offs[i],
                &offs[next],
                vertex,
                t_in,
                t_out,
                dist.abs(),
                (join, miter),
            )?;
            end_param[i] = ep;
            start_param[next] = sp;
            joined[i] = true;
            fillers[i] = fill;
        } else if let Some((ta, tb)) = overlap_crossing(&offs[i], &offs[next])? {
            // Overlap side with a real transversal crossing: trim both
            // spans to it. The excursion never materializes, so there is
            // nothing for the union to clean at this vertex.
            end_param[i] = ta;
            start_param[next] = tb;
            joined[i] = true;
        }
    }

    let mut chain = Vec::with_capacity(count * 2);
    for i in 0..count {
        // A sub-unit window extrapolates lines to the join point (the
        // miter extension); kurbo only ever sees [0, 1] for cubics.
        chain.push(sub_seg(&offs[i], start_param[i]..end_param[i]));
        if closed || i + 1 < count {
            if joined[i] {
                chain.append(&mut fillers[i]);
            } else {
                // Overlap: carry the excursion as an explicit chord so
                // the contour model keeps both offset spans whole; the
                // self-union resolves it at the transversal crossing.
                let next = (i + 1) % count;
                let (e, s) = (
                    point_at(&offs[i], end_param[i]),
                    point_at(&offs[next], start_param[next]),
                );
                if chord(e, s) > SNAP {
                    chain.push(line_seg(e, s));
                }
            }
        }
    }
    Ok(chain)
}

/// Where two offset spans' geometry crosses, strictly interior to both,
/// as `(param_on_a, param_on_b)` — found on flattened proxies. `None`
/// when they do not cross (parallel, tangent-continuous, or the only
/// contact sits on an endpoint).
///
/// # Errors
/// [`VectorError::DegenerateSegment`] past the flatten chord ceiling.
fn overlap_crossing(a: &Seg, b: &Seg) -> Result<Option<(f64, f64)>, VectorError> {
    let (fa, fb) = (flatten_span(a)?, flatten_span(b)?);
    for wa in fa.windows(2) {
        for wb in fb.windows(2) {
            if let Some((t, u)) = segment_intersection(wa[0].1, wa[1].1, wb[0].1, wb[1].1) {
                if chord_interior(t) && chord_interior(u) {
                    return Ok(Some((
                        wa[0].0 + t * (wa[1].0 - wa[0].0),
                        wb[0].0 + u * (wb[1].0 - wb[0].0),
                    )));
                }
            }
        }
    }
    Ok(None)
}

/// `(parameter, point)` polyline of one span (a line is its own proxy).
///
/// # Errors
/// [`VectorError::DegenerateSegment`] past the flatten chord ceiling.
fn flatten_span(seg: &Seg) -> Result<Vec<(f64, Point)>, VectorError> {
    match *seg {
        Seg::Line(l) => Ok(vec![(0.0, to_point(l.p0)), (1.0, to_point(l.p1))]),
        Seg::Cubic(c) => flatten_cubic(c),
    }
}

/// Computes the join between offset span `incoming`'s end and offset
/// span `outgoing`'s start at `vertex`: returns the spans' new
/// (`end_param`, `start_param`) window bounds plus filler segments bridging
/// the trimmed ends.
fn join_at(
    incoming: &Seg,
    outgoing: &Seg,
    vertex: Point,
    t_in: Point,
    t_out: Point,
    d: f64,
    style: (JoinKind, f64),
) -> Result<(f64, f64, Vec<Seg>), VectorError> {
    let (join, miter_limit) = style;
    if join == JoinKind::Miter {
        // Intersect the offset tangents; trim the spans to the meeting
        // point when it lies inside them, extrapolate/extend when it
        // lies beyond. Parallel tangents (or past the miter limit) bevel.
        let denom = t_in.x * t_out.y - t_in.y * t_out.x;
        let e = point_at(incoming, 1.0);
        let s = point_at(outgoing, 0.0);
        let to_s = sub(s, e);
        let u = if denom.abs() < 1e-12 {
            f64::NAN
        } else {
            (to_s.x * t_out.y - to_s.y * t_out.x) / denom
        };
        if u.is_finite() {
            let m = add(e, scale(t_in, u));
            if chord(m, vertex) <= miter_limit * d {
                let (te, ts) = (param_of(incoming, m)?, param_of(outgoing, m)?);
                let pe = point_at(incoming, te);
                let ps = point_at(outgoing, ts);
                let mut fill = Vec::new();
                // Lines reach `m` exactly (param_of/point_at are exact
                // on their carrier line); cubics land within flattening
                // tolerance and these chords close the residual.
                if chord(pe, m) > SNAP {
                    fill.push(line_seg(pe, m));
                }
                if chord(m, ps) > SNAP {
                    fill.push(line_seg(m, ps));
                }
                return Ok((te, ts, fill));
            }
        }
    }
    // Bevel and round (and miter fallback): trim each span to its
    // closest-to-vertex point and bridge.
    let (te, ts) = (param_of(incoming, vertex)?, param_of(outgoing, vertex)?);
    let (pe, ps) = (point_at(incoming, te), point_at(outgoing, ts));
    let fill = if join == JoinKind::Round {
        let r = chord(pe, vertex).max(chord(ps, vertex)).max(d);
        let a0 = angle_of(sub(pe, vertex));
        let a1 = angle_of(sub(ps, vertex));
        arc_cubics(vertex, r, a0, short_delta(a0, a1))
    } else {
        vec![line_seg(pe, ps)]
    };
    Ok((te, ts, fill))
}

/// The point of `seg` at parameter `t` (unclamped: lines extrapolate;
/// cubic parameters stay in [0, 1] by construction of `param_of`).
fn point_at(seg: &Seg, t: f64) -> Point {
    match *seg {
        Seg::Line(l) => to_point(lerp(l.p0, l.p1, t)),
        Seg::Cubic(c) => to_point(c.eval(t)),
    }
}

/// The parameter of the point on `seg` closest to `target`: exact
/// projection for lines (may leave [0, 1] — that is the miter/round
/// extension), nearest flattened sample for cubics.
///
/// # Errors
/// [`VectorError::DegenerateSegment`] past the flatten chord ceiling.
fn param_of(seg: &Seg, target: Point) -> Result<f64, VectorError> {
    match *seg {
        Seg::Line(l) => {
            let (a, b) = (to_point(l.p0), to_point(l.p1));
            let (vx, vy) = (b.x - a.x, b.y - a.y);
            let len2 = vx * vx + vy * vy;
            Ok(if len2 < 1e-18 {
                0.0
            } else {
                ((target.x - a.x) * vx + (target.y - a.y) * vy) / len2
            })
        }
        Seg::Cubic(c) => {
            let (mut bt, mut bd) = (0.0_f64, f64::INFINITY);
            for (t, sample) in crate::flatten::flatten_cubic(c)? {
                let d = chord(sample, target);
                if d < bd {
                    bd = d;
                    bt = t;
                }
            }
            Ok(bt)
        }
    }
}

/// The sub-span of `seg` over parameter window `w`; lines lerp (so
/// windows outside [0, 1] extrapolate — the miter extension), cubics
/// subdivide exactly.
fn sub_seg(seg: &Seg, w: std::ops::Range<f64>) -> Seg {
    let (a, b) = (w.start, w.end.max(w.start + 1e-12));
    match *seg {
        Seg::Line(l) => Seg::Line(Line::new(lerp(l.p0, l.p1, a), lerp(l.p0, l.p1, b))),
        Seg::Cubic(c) => Seg::Cubic(c.subsegment(a.min(1.0)..b.min(1.0))),
    }
}

#[must_use]
fn lerp(p0: kurbo::Point, p1: kurbo::Point, t: f64) -> kurbo::Point {
    kurbo::Point::new(p0.x + (p1.x - p0.x) * t, p0.y + (p1.y - p0.y) * t)
}

/// Offsets one span by `dist` to the left of travel. Lines translate;
/// cubics take one Tiller–Hanson pass (each control point moves along
/// the normal of its adjacent control-polygon edge; inner points along
/// the averaged normal).
fn offset_seg(span: &Seg, dist: f64) -> Result<Seg, VectorError> {
    match *span {
        Seg::Line(l) => {
            let (a, b) = (to_point(l.p0), to_point(l.p1));
            let n = left_normal(unit_or(sub(b, a), VectorError::DegenerateSegment)?);
            let (da, db) = (scale(n, dist), scale(n, dist));
            Ok(Seg::Line(Line::new(
                to_kurbo(add(a, da)),
                to_kurbo(add(b, db)),
            )))
        }
        Seg::Cubic(c) => {
            let pts = [c.p0, c.p1, c.p2, c.p3];
            let mut moved = [kurbo::Point::ZERO; 4];
            for (i, p) in pts.iter().enumerate() {
                let n = match i {
                    0 => left_normal(unit_or(sub(pts[1], pts[0]), Degenerate)?),
                    3 => left_normal(unit_or(sub(pts[3], pts[2]), Degenerate)?),
                    i => {
                        let n1 = left_normal(unit_or(sub(pts[i], pts[i - 1]), Degenerate)?);
                        let n2 = left_normal(unit_or(sub(pts[i + 1], pts[i]), Degenerate)?);
                        unit_or(add(n1, n2), Degenerate)?
                    }
                };
                moved[i] = kurbo::Point::new(p.x + n.x * dist, p.y + n.y * dist);
            }
            Ok(Seg::Cubic(CubicBez::new(
                moved[0], moved[1], moved[2], moved[3],
            )))
        }
    }
}

/// Cap closing the outline around an open contour's endpoint `center`,
/// from the left-side offset end to the right-side offset end, bulging
/// toward `bulge` (the path tangent at the endpoint, pointing away from
/// the contour).
fn cap_segs(
    center: Point,
    from: Point,
    to: Point,
    bulge: Point,
    d: f64,
    cap: CapKind,
) -> Result<Vec<Seg>, VectorError> {
    match cap {
        CapKind::Butt => Ok(vec![line_seg(from, to)]),
        CapKind::Square => {
            let (f2, t2) = (add(from, scale(bulge, d)), add(to, scale(bulge, d)));
            Ok(vec![line_seg(from, f2), line_seg(f2, t2), line_seg(t2, to)])
        }
        CapKind::Round => {
            let a0 = angle_of(sub(from, center));
            let a1 = angle_of(sub(to, center));
            let delta = short_delta(a0, a1);
            // Take the arc whose midpoint radial agrees with `bulge`.
            let mid = a0 + delta / 2.0;
            let mid_radial = Point::new(mid.cos(), mid.sin());
            let good = mid_radial.x * bulge.x + mid_radial.y * bulge.y >= 0.0;
            let full = if good {
                delta
            } else {
                delta - std::f64::consts::TAU * delta.signum()
            };
            Ok(arc_cubics(center, d, a0, full))
        }
    }
}

/// Circular arc as cubics: `theta` (signed) from angle `a0`, split into
/// chunks of at most 90°, each with the exact tangent-handle constant
/// (4/3)·tan(θ/4).
fn arc_cubics(center: Point, r: f64, a0: f64, theta: f64) -> Vec<Seg> {
    let chunks = ((theta.abs() / (std::f64::consts::FRAC_PI_2)).ceil() as usize).max(1);
    let step = theta / chunks as f64;
    let mut out = Vec::with_capacity(chunks);
    let mut a = a0;
    for _ in 0..chunks {
        let next = a + step;
        let k = 4.0 / 3.0 * (step / 4.0).tan().abs() * r;
        let (ca, sa) = (a.cos(), a.sin());
        let (cb, sb) = (next.cos(), next.sin());
        let p0 = add(center, scale(Point::new(ca, sa), r));
        let p3 = add(center, scale(Point::new(cb, sb), r));
        let t0 = scale(Point::new(-sa, ca), step.signum());
        let t3 = scale(Point::new(-sb, cb), step.signum());
        out.push(Seg::Cubic(CubicBez::new(
            to_kurbo(p0),
            to_kurbo(add(p0, scale(t0, k))),
            to_kurbo(sub(p3, scale(t3, k))),
            to_kurbo(p3),
        )));
        a = next;
    }
    out
}

/// Angle difference normalized to (−π, π].
fn short_delta(a0: f64, a1: f64) -> f64 {
    let mut d = a1 - a0;
    while d > std::f64::consts::PI {
        d -= std::f64::consts::TAU;
    }
    while d <= -std::f64::consts::PI {
        d += std::f64::consts::TAU;
    }
    d
}

/// Builds one closed contour from a connected chain of segments.
/// Anchors closer than [`SNAP`] merge (a degenerate connection carries
/// no geometry); the wrap anchor inherits the last segment's in-handle.
fn chain_to_contour(chain: &[Seg]) -> Option<Contour> {
    let mut points: Vec<PathPoint> = Vec::new();
    let mut last_cubic_p2: Option<Point> = None;
    for seg in chain {
        let start = seg.start();
        if let Some(prev) = points.last() {
            if chord(prev.anchor, start) < SNAP {
                continue; // zero-length connection: no geometry
            }
        }
        let secondary = match seg {
            Seg::Cubic(c) => Some(to_point(c.p1)),
            Seg::Line(_) => None,
        };
        points.push(PathPoint {
            anchor: start,
            primary: last_cubic_p2,
            secondary,
            is_curve: last_cubic_p2.is_some() || secondary.is_some(),
        });
        last_cubic_p2 = match seg {
            Seg::Cubic(c) => Some(to_point(c.p2)),
            Seg::Line(_) => None,
        };
    }
    if points.is_empty() {
        return None;
    }
    if let Some(first) = points.first_mut() {
        first.primary = last_cubic_p2;
        first.is_curve = first.secondary.is_some() || first.primary.is_some();
    }
    Some(Contour {
        closed: true,
        point_count: points.len() as i64,
        points,
    })
}

/// Unit tangent of a span at its start (control-polygon direction).
fn span_tangent_at_start(span: &Seg) -> Result<Point, VectorError> {
    let d = match *span {
        Seg::Line(l) => sub(to_point(l.p1), to_point(l.p0)),
        Seg::Cubic(c) => sub(to_point(c.p1), to_point(c.p0)),
    };
    unit_or(d, VectorError::DegenerateSegment)
}

/// Unit tangent of a span at its end (control-polygon direction).
fn span_tangent_at_end(span: &Seg) -> Result<Point, VectorError> {
    let d = match *span {
        Seg::Line(l) => sub(to_point(l.p1), to_point(l.p0)),
        Seg::Cubic(c) => sub(to_point(c.p3), to_point(c.p2)),
    };
    unit_or(d, VectorError::DegenerateSegment)
}

/// A neutral result path carrying `contours` (the boolean pipeline
/// overwrites metadata on its own output; these raws only feed it).
fn neutral(contours: Vec<Contour>) -> PathObject {
    PathObject {
        contours,
        primitive: None,
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
        effects: Vec::new(),
    }
}

// -- small vector helpers (f64 pairs in `geom::Point` clothing) --------

fn to_point(p: kurbo::Point) -> Point {
    Point::new(p.x, p.y)
}

fn to_kurbo(p: Point) -> kurbo::Point {
    kurbo::Point::new(p.x, p.y)
}

fn sub(a: Point, b: Point) -> Point {
    Point::new(a.x - b.x, a.y - b.y)
}

fn add(a: Point, b: Point) -> Point {
    Point::new(a.x + b.x, a.y + b.y)
}

fn scale(a: Point, k: f64) -> Point {
    Point::new(a.x * k, a.y * k)
}

fn neg(a: Point) -> Point {
    Point::new(-a.x, -a.y)
}

fn chord(a: Point, b: Point) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}

fn angle_of(a: Point) -> f64 {
    f64::atan2(a.y, a.x)
}

fn left_normal(a: Point) -> Point {
    Point::new(-a.y, a.x)
}

/// Normalizes, or fails with `error` for a zero-length vector.
fn unit_or(a: Point, error: VectorError) -> Result<Point, VectorError> {
    let len = a.x.hypot(a.y);
    if len < 1e-12 {
        return Err(error);
    }
    Ok(Point::new(a.x / len, a.y / len))
}

fn line_seg(a: Point, b: Point) -> Seg {
    Seg::Line(Line::new(to_kurbo(a), to_kurbo(b)))
}
