//! Smart Polygon tool (`Smart Polygon.jsf` behavior): a regular
//! N-gon with an optional concentric inner ring (hole) or a split
//! "pinwheel" of S closed sections.
//!
//! Parameters map to the script's `customData`: `nodes` (N, corner
//! count, 3..=25), the section count (S, 1..=N — the arc length drawn
//! in whole sides), `radius0`/`radius1` (outer/inner radius), and the
//! start angle (the center→corner-0 direction). When N equals S the
//! outer ring closes; otherwise the open outer arc plus the inner arc
//! walked back (one anchor further, to the start radius line) forms a
//! closed sector ring in one contour — the script's exact anchor
//! layout. An inner radius with a closed outer ring becomes a second,
//! reversed contour: a hole under the nonzero rule.

use super::{closed_contour, corner, shape};
use doc::model::{Contour, PathObject, PathPoint};
use geom::Point;

/// The script's constants: corner-count floor/ceiling.
pub const MIN_POINTS: u32 = 3;
pub const MAX_POINTS: u32 = 25;

/// Smart Polygon parameters (`createPolygon(R, r, N, S)` /
/// `createSplitPolygon(R, r, N, S)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolygonParams {
    /// Center control point (`c0`).
    pub center: Point,
    /// Outer radius (`R`).
    pub outer: f64,
    /// Inner radius (`r`; 0 disables the ring / makes solid wedges).
    pub inner: f64,
    /// Corner count (`N`, clamped 3..=25).
    pub points: u32,
    /// Sides drawn / split sections (`S`, clamped 1..=points).
    pub sections: u32,
    /// Direction of the first corner from `center`, radians.
    pub start_angle: f64,
    /// Split mode: one closed 4-anchor section per side instead of
    /// concentric rings.
    pub split: bool,
}

impl PolygonParams {
    /// The tool's insert defaults: heptagon (N=7), all sides, no inner
    /// ring.
    #[must_use]
    pub fn defaults(center: Point) -> Self {
        Self {
            center,
            outer: 100.0,
            inner: 0.0,
            points: 7,
            sections: 7,
            start_angle: -std::f64::consts::FRAC_PI_2,
            split: false,
        }
    }
}

/// Generates the polygon (ring or pinwheel).
#[must_use]
pub fn smart_polygon(p: PolygonParams) -> PathObject {
    let n = (p.points as f64).clamp(MIN_POINTS as f64, MAX_POINTS as f64) as u32;
    let s = p.sections.clamp(1, n);
    if p.split {
        shape(split_contours(p, n, s))
    } else {
        shape(ring_contours(p, n, s))
    }
}

fn at(center: Point, radius: f64, angle: f64) -> PathPoint {
    let (s, c) = angle.sin_cos();
    corner(center.x + radius * c, center.y + radius * s)
}

/// `createPolygon`. `sides` is `S` when the ring closes (`N == S`),
/// else `S + 1` — the open arc draws one anchor past the last full
/// side so the inner walk can return to the start radius line.
fn ring_contours(p: PolygonParams, n: u32, s: u32) -> Vec<Contour> {
    let da = std::f64::consts::TAU / n as f64;
    let closed = n == s;
    let sides = if closed { s } else { s + 1 };

    let mut outer: Vec<PathPoint> = Vec::with_capacity(sides as usize);
    for i in 0..sides {
        outer.push(at(p.center, p.outer, p.start_angle + da * i as f64));
    }

    if p.inner > 0.0 {
        // Inner arc, walked backward from the last outer angle. The
        // script pushes `sides` anchors here too (the full polygon
        // when closed, the return arc when open).
        let mut inner: Vec<PathPoint> = Vec::with_capacity(sides as usize);
        for i in (0..sides).rev() {
            inner.push(at(p.center, p.inner, p.start_angle + da * i as f64));
        }
        if closed {
            vec![closed_contour(outer), closed_contour(inner)]
        } else {
            let mut points = outer;
            points.extend(inner);
            vec![closed_contour(points)]
        }
    } else if closed {
        vec![closed_contour(outer)]
    } else {
        outer.push(corner(p.center.x, p.center.y));
        vec![closed_contour(outer)]
    }
}

/// `createSplitPolygon`: `S` closed sections, each outer-a → outer-b →
/// inner-b → inner-a (degenerate to a triangle fan when the inner
/// radius is 0).
fn split_contours(p: PolygonParams, n: u32, s: u32) -> Vec<Contour> {
    let da = std::f64::consts::TAU / n as f64;
    let mut contours = Vec::with_capacity(s as usize);
    for i in 0..s {
        let (a0, a1) = (
            p.start_angle + da * i as f64,
            p.start_angle + da * (i + 1) as f64,
        );
        contours.push(closed_contour(vec![
            at(p.center, p.outer, a0),
            at(p.center, p.outer, a1),
            at(p.center, p.inner, a1),
            at(p.center, p.inner, a0),
        ]));
    }
    contours
}
