//! Tolerance-pinned proofs for the boolean pipeline and the bridge.
//!
//! Square/circle cases assert exact anchor sets (splits land on integer
//! coordinates by construction); curve cases assert topology plus
//! determinism. The adversarial set follows the M6 plan: overlapping
//! loops, shared/collinear edges, tangent contact, self-intersection,
//! disjoint and concentric operands, even-odd rules.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::boolean::{BooleanOp, boolean};
use crate::bridge::{
    bezpath_to_contour, canonicalize_contour, contour_segments, contour_to_bezpath,
};
use crate::flatten::{FLATTEN_TOLERANCE, flatten_cubic};
use crate::intersect::{dedup_params, segment_intersection};
use crate::split::split_seg;
use doc::model::{Contour, PathObject, PathPoint};
use geom::Point;
use kurbo::{CubicBez, ParamCurve, PathEl};
use std::collections::BTreeSet;

// ---------------------------------------------------------------------
// fixtures

fn anchor(x: f64, y: f64) -> PathPoint {
    PathPoint {
        anchor: Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    }
}

fn neutral_path(contours: Vec<Contour>) -> PathObject {
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
    }
}

/// Axis-aligned rectangle as one closed all-line contour.
fn rect_path(x0: f64, y0: f64, x1: f64, y1: f64) -> PathObject {
    let points = vec![
        anchor(x0, y0),
        anchor(x1, y0),
        anchor(x1, y1),
        anchor(x0, y1),
    ];
    neutral_path(vec![Contour {
        closed: true,
        point_count: 4,
        points,
    }])
}

/// Circle as one closed 4-point contour with kappa handles.
fn circle_path(cx: f64, cy: f64, r: f64) -> PathObject {
    const K: f64 = 0.552_284_749_830_793_6;
    let (kx, ky) = (K * r, K * r);
    let pts = vec![
        PathPoint {
            anchor: Point::new(cx + r, cy),
            primary: Some(Point::new(cx + r, cy - ky)),
            secondary: Some(Point::new(cx + r, cy + ky)),
            is_curve: true,
        },
        PathPoint {
            anchor: Point::new(cx, cy + r),
            primary: Some(Point::new(cx + kx, cy + r)),
            secondary: Some(Point::new(cx - kx, cy + r)),
            is_curve: true,
        },
        PathPoint {
            anchor: Point::new(cx - r, cy),
            primary: Some(Point::new(cx - r, cy + ky)),
            secondary: Some(Point::new(cx - r, cy - ky)),
            is_curve: true,
        },
        PathPoint {
            anchor: Point::new(cx, cy - r),
            primary: Some(Point::new(cx - kx, cy - r)),
            secondary: Some(Point::new(cx + kx, cy - r)),
            is_curve: true,
        },
    ];
    neutral_path(vec![Contour {
        closed: true,
        point_count: 4,
        points: pts,
    }])
}

/// Self-intersecting "bowtie": two anchors offset by `(dx, dy)` so the
/// crossings land strictly inside spans (exact-corner crossings are the
/// documented tolerance-pinned limitation). The (0.37, 0.11) case also
/// leaves a ~0.38 px wedge between the square corner and the bowtie's
/// anti-diagonal — a deliberate regression pin for
/// [`crate::SIDE_OFFSET`] = 0.1 (a 0.5 offset overshoots the wedge and
/// drops its bounding edge, stranding the stitcher).
fn bowtie_path(dx: f64, dy: f64) -> PathObject {
    let (s, c) = (10.0, 10.0);
    let pts = vec![
        anchor(c - s + dx, c - s + dy),
        anchor(c + s + dx, c + s + dy),
        anchor(c + s + dx, c - s + dy),
        anchor(c - s + dx, c + s + dy),
    ];
    neutral_path(vec![Contour {
        closed: true,
        point_count: 4,
        points: pts,
    }])
}

/// Quantized anchor set of the result (1 px grid — split points in the
/// pinned square cases are integers).
fn anchor_set(p: &PathObject) -> BTreeSet<(i64, i64)> {
    p.contours
        .iter()
        .flat_map(|c| {
            c.points
                .iter()
                .map(|pt| (pt.anchor.x.round() as i64, pt.anchor.y.round() as i64))
        })
        .collect()
}

fn set(items: &[(i64, i64)]) -> BTreeSet<(i64, i64)> {
    items.iter().copied().collect()
}

// ---------------------------------------------------------------------
// bridge

#[test]
fn bridge_rect_round_trips_exactly() {
    let p = rect_path(0.0, 0.0, 10.0, 8.0);
    let contour = &p.contours[0];
    let back = bezpath_to_contour(&contour_to_bezpath(contour)).unwrap();
    assert_eq!(&back, contour);
}

#[test]
fn bridge_circle_round_trips_exactly() {
    let p = circle_path(0.0, 0.0, 7.5);
    let contour = &p.contours[0];
    let back = bezpath_to_contour(&contour_to_bezpath(contour)).unwrap();
    assert_eq!(&back, contour);
}

#[test]
fn bridge_open_polyline_round_trips() {
    let contour = Contour {
        closed: false,
        point_count: 3,
        points: vec![anchor(0.0, 0.0), anchor(5.0, 9.0), anchor(12.0, -3.0)],
    };
    let path = contour_to_bezpath(&contour);
    assert!(!matches!(path.iter().last(), Some(PathEl::ClosePath)));
    let back = bezpath_to_contour(&path).unwrap();
    assert_eq!(back, contour);
}

#[test]
fn bridge_dangling_handle_is_inert_and_dropped() {
    // Point 0 carries an in-handle, but its incoming span (3→0) is a
    // line (point 3 has no out-handle): the handle drives no geometry.
    let mut c = rect_contour_x();
    c.points[0].primary = Some(Point::new(-1.0, 4.0));
    let canon = canonicalize_contour(&c);
    assert_eq!(canon.points[0].primary, None);
    assert!(!canon.points[0].is_curve);
    // Idempotent.
    assert_eq!(canonicalize_contour(&canon), canon);
    // Round trip through kurbo equals the canonical form exactly.
    let back = bezpath_to_contour(&contour_to_bezpath(&canon)).unwrap();
    assert_eq!(back, canon);
}

fn rect_contour_x() -> Contour {
    let points = vec![
        anchor(0.0, 0.0),
        anchor(10.0, 0.0),
        anchor(10.0, 6.0),
        anchor(0.0, 6.0),
    ];
    Contour {
        closed: true,
        point_count: 4,
        points,
    }
}

#[test]
fn bridge_implicit_close_recovers_three_point_contour() {
    let mut path = geom::BezPath::new();
    path.move_to((0.0, 0.0));
    path.line_to((10.0, 0.0));
    path.line_to((10.0, 10.0));
    path.close_path();
    let c = bezpath_to_contour(&path).unwrap();
    assert!(c.closed);
    assert_eq!(c.point_count, 3);
    // Wrap span implied by the close: three spans total.
    assert_eq!(contour_segments(&c).len(), 3);
}

#[test]
fn bridge_rejects_multi_subpath_and_empty() {
    let mut path = geom::BezPath::new();
    path.move_to((0.0, 0.0));
    path.line_to((1.0, 1.0));
    path.move_to((5.0, 5.0));
    assert_eq!(
        bezpath_to_contour(&path),
        Err(crate::VectorError::MultiSubpath)
    );
    assert_eq!(
        bezpath_to_contour(&geom::BezPath::new()),
        Err(crate::VectorError::EmptyPath)
    );
}

// ---------------------------------------------------------------------
// flatten

#[test]
fn flatten_circle_stays_within_tolerance() {
    let c = CubicBez::new(
        (1000.0, 0.0),
        (1000.0, 552.2847498307936),
        (552.2847498307936, 1000.0),
        (0.0, 1000.0),
    );
    let poly = flatten_cubic(c);
    assert_eq!(poly.first().unwrap().0, 0.0);
    assert_eq!(poly.last().unwrap().0, 1.0);
    // Parameters strictly ascending.
    for w in poly.windows(2) {
        assert!(w[1].0 > w[0].0);
    }
    // Every probe chord's midpoint of the *true curve* lies within the
    // tolerance of the chord line (convex-hull bound).
    for w in poly.windows(2) {
        let tm = 0.5 * (w[0].0 + w[1].0);
        let true_pt = c.eval(tm);
        let ax = w[1].1.x - w[0].1.x;
        let ay = w[1].1.y - w[0].1.y;
        let len = ax.hypot(ay);
        let dist = (ax * (true_pt.y - w[0].1.y) - ay * (true_pt.x - w[0].1.x)).abs() / len;
        assert!(
            dist <= FLATTEN_TOLERANCE,
            "deviation {dist} exceeds {FLATTEN_TOLERANCE}"
        );
    }
}

// ---------------------------------------------------------------------
// intersect + split primitives

#[test]
fn segment_intersection_cross_and_parallel() {
    let hit = segment_intersection(
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(5.0, -5.0),
        Point::new(5.0, 5.0),
    );
    let (t, u) = hit.unwrap();
    assert!((t - 0.5).abs() < 1e-12 && (u - 0.5).abs() < 1e-12);
    // Parallel: None.
    assert!(
        segment_intersection(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(0.0, 1.0),
            Point::new(10.0, 1.0),
        )
        .is_none()
    );
    // Non-crossing: None.
    assert!(
        segment_intersection(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(5.0, 1.0),
            Point::new(5.0, 5.0),
        )
        .is_none()
    );
}

#[test]
fn dedup_params_sorts_and_trims() {
    let got = dedup_params(vec![0.5, 0.5 + 5e-10, 0.9, 1.0, -0.5, 1.5, 0.7], 1e-9);
    let want = [0.5, 0.7, 0.9];
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(want.iter()) {
        assert!((g - w).abs() < 1e-12);
    }
}

#[test]
fn split_line_pieces_chain() {
    let seg = crate::bridge::Seg::Line(kurbo::Line::new((0.0, 0.0), (10.0, 0.0)));
    let pieces = split_seg(&seg, &[0.25, 0.5, 0.75]);
    assert_eq!(pieces.len(), 4);
    assert_eq!(pieces[0].start, Point::new(0.0, 0.0));
    assert_eq!(pieces[3].end, Point::new(10.0, 0.0));
    for w in pieces.windows(2) {
        assert_eq!(w[0].end, w[1].start);
    }
    assert_eq!(pieces[1].start, Point::new(2.5, 0.0));
}

#[test]
fn split_cubic_pieces_chain() {
    let c = CubicBez::new((0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0));
    let seg = crate::bridge::Seg::Cubic(c);
    let pieces = split_seg(&seg, &[0.5]);
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0].start, Point::new(0.0, 0.0));
    assert_eq!(pieces[1].end, Point::new(10.0, 0.0));
    let gap = ((pieces[0].end.x - pieces[1].start.x).powi(2)
        + (pieces[0].end.y - pieces[1].start.y).powi(2))
    .sqrt();
    assert!(gap < 1e-9, "split pieces must chain (gap {gap})");
    // The piece midpoints lie on the original curve.
    for p in &pieces {
        let mid = match p.geom {
            crate::bridge::Seg::Cubic(pc) => pc.eval(0.5),
            crate::bridge::Seg::Line(l) => l.eval(0.5),
        };
        // Within 1e-6 of some point of the original curve: distance to
        // the original evaluated at the matching parameter.
        let t = if p.geom.start().x < 5.0 { 0.25 } else { 0.75 };
        let on = c.eval(t);
        let d = ((mid.x - on.x).powi(2) + (mid.y - on.y).powi(2)).sqrt();
        assert!(d < 1e-6, "piece midpoint off the original curve (d {d})");
    }
}

// ---------------------------------------------------------------------
// booleans: overlapping squares (exact)

/// A = [0,10]², B = [5,15]² — the canonical overlapping pair.
fn overlapping() -> (PathObject, PathObject) {
    (
        rect_path(0.0, 0.0, 10.0, 10.0),
        rect_path(5.0, 5.0, 15.0, 15.0),
    )
}

#[test]
fn union_overlapping_squares_exact_anchor_ring() {
    let (a, b) = overlapping();
    let r = boolean(&a, &b, BooleanOp::Union).unwrap();
    assert_eq!(r.contours.len(), 1);
    assert_eq!(
        anchor_set(&r),
        set(&[
            (0, 0),
            (10, 0),
            (10, 5),
            (15, 5),
            (15, 15),
            (5, 15),
            (5, 10),
            (0, 10),
        ])
    );
}

#[test]
fn intersect_and_crop_overlapping_squares_exact() {
    let (a, b) = overlapping();
    for op in [BooleanOp::Intersect, BooleanOp::Crop] {
        let r = boolean(&a, &b, op).unwrap();
        assert_eq!(r.contours.len(), 1, "{op:?}");
        assert_eq!(
            anchor_set(&r),
            set(&[(5, 5), (10, 5), (10, 10), (5, 10)]),
            "{op:?}"
        );
    }
}

#[test]
fn punch_overlapping_squares_exact_notch() {
    let (a, b) = overlapping();
    let r = boolean(&a, &b, BooleanOp::Punch).unwrap();
    assert_eq!(r.contours.len(), 1);
    assert_eq!(
        anchor_set(&r),
        set(&[(0, 0), (10, 0), (10, 5), (5, 5), (5, 10), (0, 10)])
    );
    // Punch is directional: b − a removes the other corner.
    let r2 = boolean(&b, &a, BooleanOp::Punch).unwrap();
    assert_eq!(r2.contours.len(), 1);
    assert_eq!(
        anchor_set(&r2),
        set(&[(5, 15), (15, 15), (15, 5), (10, 5), (10, 10), (5, 10)])
    );
}

#[test]
fn boolean_all_ops_deterministic_on_overlapping() {
    let (a, b) = overlapping();
    for op in [
        BooleanOp::Union,
        BooleanOp::Intersect,
        BooleanOp::Punch,
        BooleanOp::Crop,
    ] {
        let r1 = boolean(&a, &b, op).unwrap();
        let r2 = boolean(&a, &b, op).unwrap();
        assert_eq!(r1, r2, "{op:?} must be byte-deterministic");
    }
}

// ---------------------------------------------------------------------
// booleans: disjoint / concentric / shared edge (exact)

#[test]
fn disjoint_operands() {
    let a = rect_path(0.0, 0.0, 10.0, 10.0);
    let b = rect_path(20.0, 20.0, 30.0, 30.0);
    let inter = boolean(&a, &b, BooleanOp::Intersect).unwrap();
    assert!(inter.contours.is_empty(), "disjoint intersect is empty");
    let union = boolean(&a, &b, BooleanOp::Union).unwrap();
    assert_eq!(union.contours.len(), 2, "disjoint union keeps both loops");
}

#[test]
fn concentric_punch_makes_hole() {
    let a = rect_path(0.0, 0.0, 20.0, 20.0);
    let b = rect_path(5.0, 5.0, 15.0, 15.0);
    let r = boolean(&a, &b, BooleanOp::Punch).unwrap();
    assert_eq!(r.contours.len(), 2, "outer ring plus hole");
    let anchors = anchor_set(&r);
    assert!(anchors.contains(&(0, 0)) && anchors.contains(&(20, 20)));
    assert!(anchors.contains(&(5, 5)) && anchors.contains(&(15, 15)));
    // Union swallows the contained shape; intersect is the contained shape.
    assert_eq!(boolean(&a, &b, BooleanOp::Union).unwrap().contours.len(), 1);
    let inter = boolean(&a, &b, BooleanOp::Intersect).unwrap();
    assert_eq!(
        anchor_set(&inter),
        set(&[(5, 5), (15, 5), (15, 15), (5, 15)])
    );
}

#[test]
fn shared_edge_union_merges_exactly() {
    let a = rect_path(0.0, 0.0, 10.0, 10.0);
    let b = rect_path(10.0, 0.0, 20.0, 10.0);
    let r = boolean(&a, &b, BooleanOp::Union).unwrap();
    assert_eq!(r.contours.len(), 1);
    assert_eq!(
        anchor_set(&r),
        set(&[(0, 0), (10, 0), (20, 0), (20, 10), (10, 10), (0, 10)])
    );
}

// ---------------------------------------------------------------------
// booleans: even-odd rule (exact)

#[test]
fn even_odd_ring_intersects_by_parity() {
    // Ring: outer [0,20]², inner [5,15]², even-odd — the inner square is
    // a hole by parity.
    let mut ring = rect_path(0.0, 0.0, 20.0, 20.0);
    ring.contours.push(Contour {
        closed: true,
        point_count: 4,
        points: vec![
            anchor(5.0, 5.0),
            anchor(15.0, 5.0),
            anchor(15.0, 15.0),
            anchor(5.0, 15.0),
        ],
    });
    ring.even_odd = true;
    // Probe fully inside the hole: parity 0 → intersect is empty.
    let probe = rect_path(8.0, 8.0, 12.0, 12.0);
    let r = boolean(&ring, &probe, BooleanOp::Intersect).unwrap();
    assert!(r.contours.is_empty(), "hole region must not intersect");
    // Probe in the band: parity 1 → the probe survives intact.
    let band = rect_path(16.0, 8.0, 18.0, 12.0);
    let r2 = boolean(&ring, &band, BooleanOp::Intersect).unwrap();
    assert_eq!(r2.contours.len(), 1);
    assert_eq!(
        anchor_set(&r2),
        set(&[(16, 8), (18, 8), (18, 12), (16, 12)])
    );
}

// ---------------------------------------------------------------------
// booleans: curve cases (topology + determinism)

#[test]
fn circle_square_intersect_topology() {
    let c = circle_path(10.0, 10.0, 5.0);
    let s = rect_path(5.0, 5.0, 15.0, 15.0);
    let r = boolean(&c, &s, BooleanOp::Intersect).unwrap();
    assert_eq!(
        r.contours.len(),
        1,
        "inscribed circle ∩ square is the circle"
    );
    assert!(r.contours[0].point_count >= 4);
    assert!(r.contours[0].closed);
    // Every result anchor is a curve point (the square's corners are
    // outside the circle, the circle's anchors inside the square).
    assert!(r.contours[0].points.iter().all(|p| p.is_curve));
    let r2 = boolean(&c, &s, BooleanOp::Intersect).unwrap();
    assert_eq!(r, r2);
}

#[test]
fn circle_square_union_deterministic() {
    let c = circle_path(10.0, 10.0, 5.0);
    let s = rect_path(5.0, 5.0, 15.0, 15.0);
    // The inscribed circle kisses all four edges — a non-manifold
    // contact (loops touch without crossing). The pinned contract is
    // determinism, whether the stitcher closes the loops or reports
    // OpenChain.
    let r1 = boolean(&c, &s, BooleanOp::Union);
    let r2 = boolean(&c, &s, BooleanOp::Union);
    assert_eq!(r1, r2);
    if let Ok(p) = r1 {
        assert!(!p.contours.is_empty());
    }
}

#[test]
fn punch_circle_from_square_topology() {
    let s = rect_path(0.0, 0.0, 20.0, 20.0);
    let c = circle_path(10.0, 10.0, 4.0);
    let r = boolean(&s, &c, BooleanOp::Punch).unwrap();
    assert_eq!(r.contours.len(), 2, "square ring plus circular hole");
    let hole = r
        .contours
        .iter()
        .find(|ct| ct.points.iter().all(|p| p.is_curve))
        .expect("expected an all-curve (hole) contour");
    assert_eq!(hole.point_count, 4, "hole keeps the circle's four spans");
}

// ---------------------------------------------------------------------
// booleans: adversarial (no-panic + determinism contracts)

#[test]
fn self_intersecting_operand_stays_bounded() {
    let b = bowtie_path(0.37, 0.11);
    let s = rect_path(5.0, 5.0, 15.0, 15.0);
    for op in [BooleanOp::Union, BooleanOp::Intersect, BooleanOp::Punch] {
        let r1 = boolean(&b, &s, op).unwrap();
        let r2 = boolean(&b, &s, op).unwrap();
        assert_eq!(r1, r2, "{op:?} deterministic");
        for contour in &r1.contours {
            assert!(
                contour.closed && contour.point_count >= 3,
                "{op:?} loops well-formed"
            );
        }
    }
}

#[test]
fn tangent_contact_never_panics_and_is_stable() {
    // Circle tangent to the square's right edge at (10, 5): the two
    // boundary loops touch without crossing (non-manifold kiss). The
    // pinned contract is no-panic plus determinism; OpenChain is the
    // documented outcome for exactly-tangent contact.
    let s = rect_path(0.0, 0.0, 10.0, 10.0);
    let c = circle_path(15.0, 5.0, 5.0);
    let r1 = boolean(&s, &c, BooleanOp::Union);
    let r2 = boolean(&s, &c, BooleanOp::Union);
    assert_eq!(r1, r2);
}

#[test]
fn empty_operand_acts_as_empty_region() {
    let empty = neutral_path(Vec::new());
    let a = rect_path(0.0, 0.0, 5.0, 5.0);
    let corners = set(&[(0, 0), (5, 0), (5, 5), (0, 5)]);
    // a ∪ ∅ = a (either order)
    for (x, y) in [(&empty, &a), (&a, &empty)] {
        let r = boolean(x, y, BooleanOp::Union).unwrap();
        assert_eq!(r.contours.len(), 1);
        assert_eq!(anchor_set(&r), corners);
    }
    // a − ∅ = a
    let r = boolean(&a, &empty, BooleanOp::Punch).unwrap();
    assert_eq!(r.contours.len(), 1);
    assert_eq!(anchor_set(&r), corners);
    // ∅ − a, ∅ ∩ a, a ∩ ∅, crop: all empty
    for (x, y, op) in [
        (&empty, &a, BooleanOp::Punch),
        (&empty, &a, BooleanOp::Intersect),
        (&a, &empty, BooleanOp::Intersect),
        (&empty, &a, BooleanOp::Crop),
        (&a, &empty, BooleanOp::Crop),
    ] {
        let r = boolean(x, y, op).unwrap();
        assert!(r.contours.is_empty(), "{op:?} with empty operand");
    }
}

#[test]
fn result_metadata_is_neutral_and_nonzero() {
    let (a, b) = overlapping();
    let mut odd = a.clone();
    odd.even_odd = true;
    let r = boolean(&odd, &b, BooleanOp::Union).unwrap();
    assert!(
        !r.even_odd,
        "result loops are self-wound; nonzero rule suffices"
    );
    assert_eq!(r.opacity, 1000);
    assert_eq!(r.blend, 0);
    assert!(r.visible && !r.locked);
    assert!(r.name.is_none() && r.primitive.is_none());
}
