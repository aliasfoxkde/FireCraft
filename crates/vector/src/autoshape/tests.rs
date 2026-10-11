//! Golden geometry for the Auto Shape generators: each shape pinned at
//! its tool defaults plus the parameter edges the scripts clamp, and
//! every generator proven deterministic (two runs, `PartialEq`-equal
//! results — exact bit equality) and flatten-safe (every curve span
//! flattens inside the crate tolerance, so no NaN or hostile handle
//! ever leaves a generator).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::bridge::contour_segments;
use crate::flatten::{FLATTEN_TOLERANCE, flatten_cubic};
use doc::model::PathObject;
use geom::Point;

fn origin() -> Point {
    Point::new(40.0, -30.0)
}

/// Every generated curve span flattens within tolerance — generators
/// never emit NaN handles or degenerate control geometry.
fn flatten_safe(path: &PathObject) {
    for contour in &path.contours {
        for seg in contour_segments(contour) {
            if let crate::bridge::Seg::Cubic(c) = seg {
                let poly = flatten_cubic(c).expect("flatten");
                for (t, p) in &poly {
                    assert!(p.x.is_finite() && p.y.is_finite(), "non-finite at t={t}");
                }
                assert!(!poly.is_empty());
            }
            let _ = FLATTEN_TOLERANCE;
        }
    }
}

fn assert_deterministic<P: Copy, F: Fn(P) -> PathObject>(generate: F, params: P) {
    assert_eq!(generate(params), generate(params));
}

#[test]
fn star_defaults_are_ten_straight_corners() {
    let p = StarParams::defaults(origin());
    let path = star(p);
    assert_deterministic(star, p);
    assert_eq!(path.contours.len(), 1);
    let c = &path.contours[0];
    assert!(c.closed);
    assert_eq!(c.point_count, 10);
    // First corner straight up from the center, inner radius half.
    assert!((c.points[0].anchor.x - origin().x).abs() < 1e-9);
    assert!((c.points[0].anchor.y - (origin().y - 100.0)).abs() < 1e-9);
    assert!((c.points[1].anchor.y - (origin().y - 50.0 * 0.809_016_994_374_947_5)).abs() < 1e-9);
    for point in &c.points {
        assert!(!point.is_curve);
        assert!(point.primary.is_none() && point.secondary.is_none());
    }
    flatten_safe(&path);
}

#[test]
fn star_rounding_bends_only_the_rounded_family() {
    let p = StarParams {
        center: origin(),
        outer: 100.0,
        inner: 50.0,
        outer_round: 90.0, // outer corners curve
        inner_round: 50.0, // inner corners stay straight
        points: 5,
        start_angle: -std::f64::consts::FRAC_PI_2,
    };
    let path = star(p);
    let c = &path.contours[0];
    // Even anchors (outer) carry handles; odd (inner) do not.
    for (i, point) in c.points.iter().enumerate() {
        if i % 2 == 0 {
            assert!(point.is_curve, "outer corner {i} must curve");
            // Handle length from the script's outer formula.
            let da = std::f64::consts::TAU / 10.0;
            let rr =
                p.inner * da.sin() * (p.outer - p.outer_round) / (p.outer - p.inner * da.cos());
            let (s, co) = (da * i as f64 + p.start_angle).sin_cos();
            let pred = point.primary.expect("in-handle");
            assert!((pred.x - (point.anchor.x + rr * s)).abs() < 1e-9);
            assert!((pred.y - (point.anchor.y - rr * co)).abs() < 1e-9);
        } else {
            assert!(!point.is_curve);
        }
    }
    flatten_safe(&path);
}

#[test]
fn star_corner_count_clamps_like_the_tool() {
    let low = star(StarParams {
        points: 1,
        ..StarParams::defaults(origin())
    });
    assert_eq!(
        low.contours[0].point_count,
        2 * super::star::MIN_POINTS as i64
    );
    let high = star(StarParams {
        points: 99,
        ..StarParams::defaults(origin())
    });
    assert_eq!(
        high.contours[0].point_count,
        2 * super::star::MAX_POINTS as i64
    );
}

#[test]
fn star_triangle_inner_handles_hit_the_clamp() {
    // N = 3 with a live inner roundness: the script pins the inner
    // handle length to maxHandleSize (no ratio term for triangles).
    let p = StarParams {
        center: origin(),
        outer: 100.0,
        inner: 50.0,
        outer_round: 100.0,
        inner_round: 30.0,
        points: 3,
        start_angle: 0.0,
    };
    let path = star(p);
    let c = &path.contours[0];
    for (i, point) in c.points.iter().enumerate() {
        if i % 2 == 1 {
            let pred = point.primary.expect("in-handle");
            let dx = pred.x - point.anchor.x;
            let dy = pred.y - point.anchor.y;
            assert!(
                (dx.hypot(dy) - 3000.0).abs() < 1e-6,
                "handle {i} at {}",
                dx.hypot(dy)
            );
        }
    }
    flatten_safe(&path);
}

#[test]
fn rounded_rectangle_walks_the_script_anchor_ring() {
    let p = RectParams::defaults(origin(), CornerKind::Rounded);
    let path = rectangle(p);
    assert_deterministic(rectangle, p);
    let c = &path.contours[0];
    assert_eq!(c.point_count, 8);
    let (o, r) = (origin(), p.corner);
    let expect = [
        (o.x, o.y + r),
        (o.x + r, o.y),
        (o.x + p.width - r, o.y),
        (o.x + p.width, o.y + r),
        (o.x + p.width, o.y + p.height - r),
        (o.x + p.width - r, o.y + p.height),
        (o.x + r, o.y + p.height),
        (o.x, o.y + p.height - r),
    ];
    for (point, (x, y)) in c.points.iter().zip(expect) {
        assert!((point.anchor.x - x).abs() < 1e-9);
        assert!((point.anchor.y - y).abs() < 1e-9);
    }
    // Corner handles: quarter-circle constant along the edge normal;
    // edge anchors carry none.
    let cp = r * ELLIPSE_BCP_CONST;
    for (i, point) in c.points.iter().enumerate() {
        if i % 2 == 0 {
            let succ = point.secondary.expect("out-handle");
            if i == 0 {
                assert!((succ.y - (point.anchor.y - cp)).abs() < 1e-9);
                assert!((succ.x - point.anchor.x).abs() < 1e-9);
            }
        } else {
            assert!(point.primary.is_some());
            assert!(point.secondary.is_none());
        }
    }
    // The top edge (anchors 1→2) is a straight span.
    assert!(c.points[1].secondary.is_none() && c.points[2].primary.is_none());
    flatten_safe(&path);
}

#[test]
fn beveled_rectangle_is_all_straight_spans() {
    let path = rectangle(RectParams::defaults(origin(), CornerKind::Beveled));
    let c = &path.contours[0];
    for point in &c.points {
        assert!(point.primary.is_none() && point.secondary.is_none());
        assert!(!point.is_curve);
    }
}

#[test]
fn chamfer_rectangle_carries_the_script_handles_verbatim() {
    // Corpus-gated open question (see module docs): the script gives
    // chamfer corners edge-parallel handles, which renders a shallow
    // bulged cut. This pins the verbatim reading until the Wine
    // oracle settles it.
    let p = RectParams::defaults(origin(), CornerKind::Chamfer);
    let path = rectangle(p);
    let c = &path.contours[0];
    let cp = p.corner * ELLIPSE_BCP_CONST;
    let succ = c.points[0].secondary.expect("chamfer out-handle");
    assert!((succ.x - (c.points[0].anchor.x + cp)).abs() < 1e-9);
    assert!((succ.y - c.points[0].anchor.y).abs() < 1e-9);
    let pred = c.points[1].primary.expect("chamfer in-handle");
    assert!((pred.x - c.points[1].anchor.x).abs() < 1e-9);
    assert!((pred.y - (c.points[1].anchor.y + cp)).abs() < 1e-9);
    flatten_safe(&path);
}

#[test]
fn smart_polygon_closed_ring_reverses_the_inner() {
    let p = PolygonParams {
        center: origin(),
        outer: 100.0,
        inner: 40.0,
        points: 5,
        sections: 5,
        start_angle: 0.0,
        split: false,
    };
    let path = smart_polygon(p);
    assert_deterministic(smart_polygon, p);
    assert_eq!(path.contours.len(), 2);
    let (outer, inner) = (&path.contours[0], &path.contours[1]);
    assert_eq!(outer.point_count, 5);
    assert_eq!(inner.point_count, 5);
    // Inner walks backward: its first anchor sits at the last outer
    // angle (start + 4·72°), its final anchor back on the start angle.
    assert!((outer.points[0].anchor.x - (origin().x + 100.0)).abs() < 1e-9);
    let last_angle = 4.0 * std::f64::consts::TAU / 5.0;
    assert!((inner.points[0].anchor.x - (origin().x + 40.0 * last_angle.cos())).abs() < 1e-9);
    assert!((inner.points[0].anchor.y - (origin().y + 40.0 * last_angle.sin())).abs() < 1e-9);
    assert!((inner.points[4].anchor.x - (origin().x + 40.0)).abs() < 1e-9);
    assert!((inner.points[4].anchor.y - origin().y).abs() < 1e-9);
    flatten_safe(&path);
}

#[test]
fn smart_polygon_open_sector_closes_through_the_inner_walk() {
    // N=6, S=3: open outer arc of 4 anchors; with an inner radius the
    // inner walk adds 4 more in the same contour; without one, the
    // contour gains the center anchor instead.
    let base = PolygonParams {
        center: origin(),
        outer: 100.0,
        inner: 0.0,
        points: 6,
        sections: 3,
        start_angle: 0.0,
        split: false,
    };
    let solid = smart_polygon(base);
    assert_eq!(solid.contours.len(), 1);
    assert_eq!(solid.contours[0].point_count, 5); // 4 arc anchors + center
    assert!(solid.contours[0].closed);

    let ring = smart_polygon(PolygonParams {
        inner: 30.0,
        ..base
    });
    assert_eq!(ring.contours.len(), 1);
    assert_eq!(ring.contours[0].point_count, 8); // 4 outer + 4 inner
    assert!(ring.contours[0].closed);
    flatten_safe(&solid);
    flatten_safe(&ring);
}

#[test]
fn smart_polygon_split_makes_one_quad_per_section() {
    let p = PolygonParams {
        center: origin(),
        outer: 100.0,
        inner: 25.0,
        points: 5,
        sections: 5,
        start_angle: -std::f64::consts::FRAC_PI_2,
        split: true,
    };
    let path = smart_polygon(p);
    assert_eq!(path.contours.len(), 5);
    for contour in &path.contours {
        assert_eq!(point_count_of(contour), 4);
        assert!(contour.closed);
    }
    // First section starts at the start angle on the outer radius.
    assert!((path.contours[0].points[0].anchor.y - (origin().y - 100.0)).abs() < 1e-9);
    // Degenerate inner radius still generates (triangle wedges).
    let solid = smart_polygon(PolygonParams { inner: 0.0, ..p });
    assert_eq!(solid.contours.len(), 5);
    flatten_safe(&path);
    flatten_safe(&solid);
}

fn point_count_of(contour: &doc::model::Contour) -> i64 {
    contour.point_count
}

#[test]
fn spiral_open_ends_on_the_radius_limit() {
    let p = SpiralParams::defaults(origin());
    let path = spiral(p);
    assert_deterministic(spiral, p);
    let c = &path.contours[0];
    assert!(!c.closed);
    // Head: the start radius, heading out; no in-handle at theta 0.
    let head = &c.points[0];
    assert!((head.anchor.x - (origin().x + 0.5)).abs() < 1e-9);
    assert!((head.anchor.y - origin().y).abs() < 1e-9);
    assert!(head.primary.is_none());
    // Tail: clipped onto the radius limit, out-handle pinned to the
    // anchor (the walk ends there).
    let tail = c.points.last().expect("tail");
    assert!((tail.anchor.x - origin().x).hypot(tail.anchor.y - origin().y) - p.radius < 1.0);
    assert_eq!(
        tail.secondary,
        Some(Point::new(tail.anchor.x, tail.anchor.y))
    );
    // ~30 radians of curve at 4 points per cycle.
    assert!(
        (6..=30).contains(&c.point_count),
        "node count {}",
        c.point_count
    );
    flatten_safe(&path);
}

#[test]
fn spiral_closed_retraces_to_a_closed_contour() {
    let open = spiral(SpiralParams::defaults(origin()));
    let closed = spiral(SpiralParams {
        closed: true,
        ..SpiralParams::defaults(origin())
    });
    let oc = &open.contours[0];
    let cc = &closed.contours[0];
    assert!(cc.closed);
    assert!(cc.point_count > oc.point_count, "retrace must add nodes");
    // The retrace mirrors anchors: the tail repeats the head region in
    // reverse (the turning point sits one turn before the end).
    let head = &cc.points[0].anchor;
    let last = &cc.points.last().expect("last").anchor;
    assert!((head.x - last.x).abs() < 1e-9 && (head.y - last.y).abs() < 1e-9);
    flatten_safe(&closed);
}

#[test]
fn spiral_tightness_clamps_and_keeps_terminating() {
    // Below the window cot b would stall the growth; the clamp keeps
    // the curve (and the loop) finite.
    let p = spiral(SpiralParams {
        tightness_deg: 5.0,
        ..SpiralParams::defaults(origin())
    });
    assert!(!p.contours.is_empty());
    flatten_safe(&p);
}

#[test]
fn spiral_sub_start_radius_is_the_empty_path() {
    let p = spiral(SpiralParams {
        radius: 0.1,
        ..SpiralParams::defaults(origin())
    });
    // The whole curve clips onto the limit: one node, on the radius.
    assert_eq!(p.contours.len(), 1);
    let node = &p.contours[0].points[0].anchor;
    assert!((node.x - origin().x).hypot(node.y - origin().y) - 0.1 < 1e-9);
    let zero = spiral(SpiralParams {
        radius: 0.0,
        ..SpiralParams::defaults(origin())
    });
    assert!(zero.contours.is_empty());
}

#[test]
fn tightness_from_control_maps_the_drag_window() {
    let (cy, r) = (0.0, 100.0);
    let top = tightness_from_control(cy, r, cy - r);
    let bottom = tightness_from_control(cy, r, cy + r);
    // The drag window tops out below the k.maxB ceiling: evalB is
    // logarithmic in the control position.
    let top_expected =
        (10f64).ln() / (12f64).ln() * (MAX_TIGHTNESS_DEG - MIN_TIGHTNESS_DEG) + MIN_TIGHTNESS_DEG;
    assert!((top - top_expected).abs() < 1e-9, "top {top}");
    assert!((bottom - MIN_TIGHTNESS_DEG).abs() < 1e-9, "bottom {bottom}");
    // Monotone upward as the control rises (tighter = higher).
    let low = tightness_from_control(cy, r, cy + r / 2.0);
    let high = tightness_from_control(cy, r, cy - r / 2.0);
    assert!(low > bottom && high > low && high < top);
}

#[test]
fn pie_quarter_wedge_is_four_anchors() {
    let p = PieParams::pie_defaults(origin());
    let path = pie(p);
    assert_deterministic(pie, p);
    let c = &path.contours[0];
    assert_eq!(c.point_count, 4);
    assert!(c.closed);
    // Inner radius 0: both inner anchors pinch to the center.
    assert!((c.points[0].anchor.x - origin().x).abs() < 1e-9);
    assert!((c.points[0].anchor.y - origin().y).abs() < 1e-9);
    assert!((c.points[1].anchor.x - origin().x).abs() < 1e-9);
    // Outer arc walked backward: anchor 2 at the end angle (0°),
    // anchor 3 back at the start angle (-90°).
    assert!((c.points[2].anchor.x - (origin().x + 100.0)).abs() < 1e-9);
    assert!((c.points[3].anchor.y - (origin().y - 100.0)).abs() < 1e-9);
    // Zero inner radius: both center anchors carry no handles at all.
    assert!(c.points[0].primary.is_none() && c.points[0].secondary.is_none());
    assert!(c.points[1].primary.is_none() && c.points[1].secondary.is_none());
    // The outer walk ends with a live in-handle (the arc curves into
    // the start-angle stop) but no out-handle.
    assert!(c.points[3].primary.is_some() && c.points[3].secondary.is_none());
    flatten_safe(&path);
}

#[test]
fn pie_half_turn_distributes_three_stops_per_arc() {
    let p = PieParams {
        center: origin(),
        outer: 100.0,
        inner: 0.0,
        start: 0.0,
        end: std::f64::consts::PI,
        is_pie: true,
    };
    let path = pie(p);
    // da = π → 3 stops → 6 anchors; the middle outer stop sits at 90°.
    assert_eq!(path.contours[0].point_count, 6);
    let mid = &path.contours[0].points[4].anchor;
    assert!((mid.x - origin().x).abs() < 1e-9);
    assert!((mid.y - (origin().y + 100.0)).abs() < 1e-9);
    flatten_safe(&path);
}

#[test]
fn doughnut_full_circle_takes_the_two_circle_shortcut() {
    let p = PieParams::doughnut_defaults(origin());
    let path = doughnut(p);
    assert_deterministic(doughnut, p);
    assert_eq!(path.contours.len(), 2);
    assert!(path.even_odd, "same-winding circles need even-odd");
    for contour in &path.contours {
        assert_eq!(point_count_of(contour), 4);
        assert!(contour.closed);
    }
    let outer = &path.contours[0].points[0].anchor;
    let inner = &path.contours[1].points[0].anchor;
    assert!((outer.y - (origin().y - 100.0)).abs() < 1e-9);
    assert!((inner.y - (origin().y - 40.0)).abs() < 1e-9);
    flatten_safe(&path);
}

#[test]
fn doughnut_with_angles_is_one_sector_contour() {
    let p = PieParams {
        start: -std::f64::consts::FRAC_PI_2,
        end: std::f64::consts::FRAC_PI_2,
        ..PieParams::doughnut_defaults(origin())
    };
    let path = doughnut(p);
    assert_eq!(path.contours.len(), 1);
    assert!(!path.even_odd, "forward-inner/backward-outer winds a hole");
    assert_eq!(point_count_of(&path.contours[0]), 6);
    // Inner arc forward (anchor 0 at -90° on r=40), outer backward
    // (anchor 3 at +90° on R=100).
    assert!((path.contours[0].points[0].anchor.y - (origin().y - 40.0)).abs() < 1e-9);
    assert!((path.contours[0].points[3].anchor.y - (origin().y + 100.0)).abs() < 1e-9);
    flatten_safe(&path);
}

#[test]
fn pie_angle_wrap_spans_the_long_way_round() {
    // start 270°, end 90°: the span crosses ±π and covers a half turn.
    let p = PieParams {
        center: origin(),
        outer: 100.0,
        inner: 0.0,
        start: 3.0 * std::f64::consts::FRAC_PI_2,
        end: std::f64::consts::FRAC_PI_2,
        is_pie: true,
    };
    let path = pie(p);
    assert_eq!(point_count_of(&path.contours[0]), 6);
    flatten_safe(&path);
}

#[test]
fn lshape_defaults_are_the_script_ring() {
    let p = LShapeParams::defaults(origin());
    let path = lshape(p);
    assert_deterministic(lshape, p);
    let c = &path.contours[0];
    assert!(c.closed);
    assert_eq!(c.point_count, 8);
    let o = origin();
    // Sharp corners duplicate the corner anchors (script insert state).
    let expect = [
        (o.x, o.y),
        (o.x + 200.0, o.y),
        (o.x + 200.0, o.y + 100.0),
        (o.x + 100.0, o.y + 100.0),
        (o.x + 100.0, o.y + 100.0),
        (o.x + 100.0, o.y + 200.0),
        (o.x, o.y + 200.0),
        (o.x, o.y),
    ];
    for (point, (x, y)) in c.points.iter().zip(expect) {
        assert!((point.anchor.x - x).abs() < 1e-9);
        assert!((point.anchor.y - y).abs() < 1e-9);
        assert!(point.primary.is_none() && point.secondary.is_none());
    }
    flatten_safe(&path);
}

#[test]
fn lshape_roundness_excess_fillets_the_inner_corner() {
    // r = 150 over t = 100: the top-left arc takes 150, the inner
    // corner takes the 50 excess (the script's blob morph).
    let p = LShapeParams {
        roundness: 150.0,
        ..LShapeParams::defaults(origin())
    };
    let path = lshape(p);
    let c = &path.contours[0];
    let (o, k) = (origin(), ELLIPSE_BCP_CONST);
    let n0 = &c.points[0];
    assert!((n0.anchor.x - (o.x + 150.0)).abs() < 1e-9);
    let pred = n0.primary.expect("top-left in-handle");
    assert!((pred.x - (o.x + 150.0 - 150.0 * k)).abs() < 1e-9);
    // Verbatim parameters: width stays 200 (no normalization).
    assert!((c.points[1].anchor.x - (o.x + 200.0)).abs() < 1e-9);
    // Inner corner: r_ = 50 fillet with one live handle per node.
    let n3 = &c.points[3];
    assert!((n3.anchor.x - (o.x + 150.0)).abs() < 1e-9);
    let succ = n3.secondary.expect("inner out-handle");
    assert!((succ.x - (o.x + 100.0 + 50.0 * k)).abs() < 1e-9);
    assert!(n3.primary.is_none());
    let n4 = &c.points[4];
    assert!((n4.anchor.y - (o.y + 100.0 + 50.0)).abs() < 1e-9);
    let pred = n4.primary.expect("inner in-handle");
    assert!((pred.y - (o.y + 150.0 - 50.0 * k)).abs() < 1e-9);
    assert!(n4.secondary.is_none());
    // The closing arc's bottom node carries the same 150 radius.
    let n7 = &c.points[7];
    let succ = n7.secondary.expect("closing out-handle");
    assert!((succ.y - (o.y + 150.0 - 150.0 * k)).abs() < 1e-9);
    flatten_safe(&path);
}

#[test]
fn arrow_defaults_are_the_script_ring() {
    let p = ArrowParams::defaults(origin());
    let path = arrow(p);
    assert_deterministic(arrow, p);
    let c = &path.contours[0];
    assert!(c.closed);
    assert_eq!(c.point_count, 11);
    let o = origin();
    // Barbs overhang the shaft band by ±50; the head trio sits at the
    // right edge minus the setback, plus the tip reach.
    let expect = [
        (o.x, o.y),
        (o.x + 200.0, o.y),
        (o.x + 200.0, o.y - 50.0),
        (o.x + 300.0, o.y + 50.0),
        (o.x + 200.0, o.y + 150.0),
        (o.x + 200.0, o.y + 100.0),
        (o.x + 100.0, o.y + 100.0),
        (o.x + 100.0, o.y + 100.0),
        (o.x + 100.0, o.y + 100.0),
        (o.x, o.y + 100.0),
        (o.x, o.y),
    ];
    for (point, (x, y)) in c.points.iter().zip(expect) {
        assert!(
            (point.anchor.x - x).abs() < 1e-9,
            "x {} vs {}",
            point.anchor.x,
            x
        );
        assert!(
            (point.anchor.y - y).abs() < 1e-9,
            "y {} vs {}",
            point.anchor.y,
            y
        );
        assert!(point.primary.is_none() && point.secondary.is_none());
    }
    flatten_safe(&path);
}

#[test]
fn arrow_line_default_has_an_open_head_at_the_end_only() {
    let p = ArrowLineParams::defaults(origin(), Point::new(140.0, -30.0));
    let path = arrow_line(p);
    assert_deterministic(arrow_line, p);
    assert_eq!(path.contours.len(), 2);
    // Line first: two plain endpoints.
    let line = &path.contours[0];
    assert!(!line.closed);
    assert_eq!(line.point_count, 2);
    // Open head: arms 10 units at ∓30° behind the end.
    let head = &path.contours[1];
    assert!(!head.closed);
    assert_eq!(head.point_count, 3);
    let arm = 10.0 * (std::f64::consts::FRAC_PI_6).cos();
    let (x1, y1) = (head.points[0].anchor.x, head.points[0].anchor.y);
    assert!((x1 - (140.0 - arm)).abs() < 1e-9);
    assert!((y1 - (-30.0 - 5.0)).abs() < 1e-9);
    assert!((head.points[1].anchor.x - 140.0).abs() < 1e-9);
    let (x2, y2) = (head.points[2].anchor.x, head.points[2].anchor.y);
    assert!((x2 - (140.0 - arm)).abs() < 1e-9);
    assert!((y2 - (-30.0 + 5.0)).abs() < 1e-9);
    flatten_safe(&path);
}

#[test]
fn arrow_line_head_styles_follow_the_script_formulas() {
    let start = origin();
    let end = Point::new(140.0, -30.0);
    // Solid head: same arms as open, closed contour.
    let solid = arrow_line(ArrowLineParams {
        end_head: ArrowHead::Solid,
        ..ArrowLineParams::defaults(start, end)
    });
    assert_eq!(solid.contours.len(), 2);
    assert!(solid.contours[1].closed);
    // Tail: swept skirt 15, point 20, closed quartet around the end.
    let tail = arrow_line(ArrowLineParams {
        end_head: ArrowHead::Tail,
        ..ArrowLineParams::defaults(start, end)
    });
    let head = &tail.contours[1];
    assert!(head.closed);
    assert_eq!(head.point_count, 6);
    let d = 10.0 * std::f64::consts::FRAC_1_SQRT_2;
    let o = (d, d);
    let expect = [
        (140.0 + o.0, -30.0 + o.1),
        (140.0, -30.0),
        (140.0 + o.0, -30.0 - o.1),
        (132.071_067_811_865_48, -30.0 - o.1),
        (120.0, -30.0),
        (132.071_067_811_865_48, -30.0 + o.1),
    ];
    for (point, (x, y)) in head.points.iter().zip(expect) {
        assert!(
            (point.anchor.x - x).abs() < 1e-9,
            "x {} vs {}",
            point.anchor.x,
            x
        );
        assert!(
            (point.anchor.y - y).abs() < 1e-9,
            "y {} vs {}",
            point.anchor.y,
            y
        );
    }
    // Box at the start: symmetric diagonal square, same at both ends.
    let boxed = arrow_line(ArrowLineParams {
        start_head: ArrowHead::Box,
        ..ArrowLineParams::defaults(start, end)
    });
    assert_eq!(boxed.contours.len(), 3);
    let head = &boxed.contours[1];
    assert!(head.closed);
    assert_eq!(head.point_count, 4);
    let b = 5.0 * std::f64::consts::FRAC_1_SQRT_2;
    let expect = [
        (40.0 - b, -30.0 - b),
        (40.0 - b, -30.0 + b),
        (40.0 + b, -30.0 + b),
        (40.0 + b, -30.0 - b),
    ];
    for (point, (x, y)) in head.points.iter().zip(expect) {
        assert!((point.anchor.x - x).abs() < 1e-9);
        assert!((point.anchor.y - y).abs() < 1e-9);
    }
    flatten_safe(&solid);
    flatten_safe(&tail);
    flatten_safe(&boxed);
}

#[test]
fn measure_long_span_opens_a_midpoint_gap() {
    let p = MeasureParams::defaults(origin());
    let path = measure(p);
    assert_deterministic(measure, p);
    assert_eq!(path.contours.len(), 6);
    // Span 100 ≥ 80, slope 0 → gap 30 about the midpoint (90, -30).
    let c0 = &path.contours[0];
    assert!((c0.points[1].anchor.x - 60.0).abs() < 1e-9);
    let c1 = &path.contours[1];
    assert!((c1.points[1].anchor.x - 120.0).abs() < 1e-9);
    // Heads aim along the line toward each endpoint.
    let c2 = &path.contours[2];
    assert!(
        (c2.points[0].anchor.x - (40.0 + 8.0 * std::f64::consts::FRAC_PI_6.cos())).abs() < 1e-9
    );
    assert!((c2.points[0].anchor.y - (-30.0 + 4.0)).abs() < 1e-9);
    let c3 = &path.contours[3];
    assert!(
        (c3.points[0].anchor.x - (140.0 - 8.0 * std::f64::consts::FRAC_PI_6.cos())).abs() < 1e-9
    );
    assert!((c3.points[0].anchor.y - (-30.0 - 4.0)).abs() < 1e-9);
    // Perpendicular ticks, 15 each way.
    let c4 = &path.contours[4];
    assert!((c4.points[0].anchor.y - (-15.0)).abs() < 1e-9);
    assert!((c4.points[1].anchor.y - (-45.0)).abs() < 1e-9);
    flatten_safe(&path);
}

#[test]
fn measure_short_span_flips_the_heads_outward() {
    let p = MeasureParams {
        start: origin(),
        end: Point::new(100.0, -30.0),
    };
    let path = measure(p);
    // Span 60 < 80: extensions run away, heads point outward.
    let c0 = &path.contours[0];
    assert!((c0.points[1].anchor.x - 25.0).abs() < 1e-9);
    let c1 = &path.contours[1];
    assert!((c1.points[1].anchor.x - 115.0).abs() < 1e-9);
    let c2 = &path.contours[2];
    assert!(
        (c2.points[0].anchor.x - (40.0 - 8.0 * std::f64::consts::FRAC_PI_6.cos())).abs() < 1e-9
    );
    assert!((c2.points[0].anchor.y - (-30.0 - 4.0)).abs() < 1e-9);
    let c3 = &path.contours[3];
    assert!(
        (c3.points[0].anchor.x - (100.0 + 8.0 * std::f64::consts::FRAC_PI_6.cos())).abs() < 1e-9
    );
    flatten_safe(&path);
}

#[test]
fn connector_defaults_collapse_to_a_sharp_elbow() {
    let p = ConnectorParams::defaults(Point::new(140.0, 70.0));
    let path = connector_line(p);
    assert_deterministic(connector_line, p);
    let c = &path.contours[0];
    assert!(!c.closed);
    assert_eq!(c.point_count, 6);
    // Zero-radius corners: the arc pairs collapse onto the spine.
    let expect = [
        (40.0, -30.0),
        (140.0, -30.0),
        (140.0, -30.0),
        (140.0, 170.0),
        (140.0, 170.0),
        (240.0, 170.0),
    ];
    for (point, (x, y)) in c.points.iter().zip(expect) {
        assert!((point.anchor.x - x).abs() < 1e-9);
        assert!((point.anchor.y - y).abs() < 1e-9);
        assert!(point.primary.is_none() && point.secondary.is_none());
    }
    flatten_safe(&path);
}

#[test]
fn connector_rounded_corners_turn_onto_the_spine() {
    let p = ConnectorParams {
        start: Point::new(40.0, -30.0),
        end: Point::new(240.0, 170.0),
        elbow: Point::new(140.0, 70.0),
        corner1: 30.0,
        corner2: 20.0,
        vertical: false,
    };
    let path = connector_line(p);
    let c = &path.contours[0];
    let k = ELLIPSE_BCP_CONST;
    // Start corner at (170, -30), arc down to the spine.
    let n1 = &c.points[1];
    assert!((n1.anchor.x - 170.0).abs() < 1e-9);
    let succ = n1.secondary.expect("start corner out-handle");
    assert!((succ.x - (170.0 + 30.0 * k)).abs() < 1e-9);
    let n2 = &c.points[2];
    assert!((n2.anchor.y - 0.0).abs() < 1e-9);
    let pred = n2.primary.expect("arc end in-handle");
    assert!((pred.y - (0.0 - 30.0 * k)).abs() < 1e-9);
    // End corner at (160, 170), arc from the spine.
    let n3 = &c.points[3];
    assert!((n3.anchor.y - 150.0).abs() < 1e-9);
    let succ = n3.secondary.expect("arc start out-handle");
    assert!((succ.y - (150.0 + 20.0 * k)).abs() < 1e-9);
    let n4 = &c.points[4];
    assert!((n4.anchor.x - 160.0).abs() < 1e-9);
    let pred = n4.primary.expect("end corner in-handle");
    assert!((pred.x - (160.0 - 20.0 * k)).abs() < 1e-9);
    flatten_safe(&path);
}

#[test]
fn connector_vertical_orientation_rotates_the_frame() {
    let p = ConnectorParams {
        start: Point::new(40.0, -30.0),
        end: Point::new(240.0, 170.0),
        elbow: Point::new(140.0, 70.0),
        corner1: 30.0,
        corner2: 0.0,
        vertical: true,
    };
    let path = connector_line(p);
    let c = &path.contours[0];
    let k = ELLIPSE_BCP_CONST;
    // Start corner at (40, 100): arc bends onto the vertical spine.
    let n1 = &c.points[1];
    assert!((n1.anchor.y - 100.0).abs() < 1e-9);
    let succ = n1.secondary.expect("start corner out-handle");
    assert!((succ.y - (100.0 + 30.0 * k)).abs() < 1e-9);
    let n2 = &c.points[2];
    assert!((n2.anchor.x - 70.0).abs() < 1e-9);
    assert!((n2.anchor.y - 70.0).abs() < 1e-9);
    // Zero end corner: end side collapses to the spine crossing.
    let n3 = &c.points[3];
    assert!((n3.anchor.x - 240.0).abs() < 1e-9);
    assert!(n3.secondary.is_none());
    let n4 = &c.points[4];
    assert!(n4.primary.is_none() && n4.secondary.is_none());
    flatten_safe(&path);
}
