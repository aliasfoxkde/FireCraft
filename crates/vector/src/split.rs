//! Splitting of true curve spans at found intersection parameters.
//!
//! Output geometry is exact curve math (kurbo `subcurve` — de Casteljau
//! re-parameterization), never the flattened proxy.

use crate::SNAP;
use crate::bridge::Seg;
use geom::Point;
use kurbo::ParamCurve;

/// Pieces shorter than this (chord length, px) are dropped: they are
/// intersection artifacts at shared endpoints, not geometry, and would
/// stitch to themselves on the [`SNAP`] grid.
const MIN_PIECE: f64 = SNAP;

/// One true-geometry span between consecutive split parameters.
#[derive(Clone, Debug)]
pub struct Piece {
    pub geom: Seg,
    pub start: Point,
    pub end: Point,
}

impl Piece {
    /// The same piece traversed in the opposite direction.
    #[must_use]
    pub fn reversed(&self) -> Self {
        let geom = self.geom.reversed();
        Piece {
            start: geom.start(),
            end: geom.end(),
            geom,
        }
    }
}

/// Splits `seg` at `ts` (must be sorted, deduplicated, strictly
/// interior) into pieces. Adjacent parameters closer than
/// [`MIN_PIECE`] collapse (the sliver piece is skipped), so the pieces
/// still chain: each piece starts where the previous one ends.
pub fn split_seg(seg: &Seg, ts: &[f64]) -> Vec<Piece> {
    let mut bounds = Vec::with_capacity(ts.len() + 2);
    bounds.push(0.0);
    bounds.extend_from_slice(ts);
    bounds.push(1.0);

    let mut pieces = Vec::with_capacity(bounds.len() - 1);
    for pair in bounds.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if b <= a {
            continue; // duplicate bound: zero-width span
        }
        let geom = match *seg {
            Seg::Line(l) => Seg::Line(kurbo::Line::new(lerp(l.p0, l.p1, a), lerp(l.p0, l.p1, b))),
            Seg::Cubic(c) => Seg::Cubic(c.subsegment(a..b)),
        };
        let (start, end) = (geom.start(), geom.end());
        if chord(start, end) < MIN_PIECE {
            continue;
        }
        pieces.push(Piece { geom, start, end });
    }
    pieces
}

/// Endpoint distance (chord length) of a piece.
fn chord(a: Point, b: Point) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}

#[must_use]
fn lerp(p0: kurbo::Point, p1: kurbo::Point, t: f64) -> kurbo::Point {
    kurbo::Point::new(p0.x + (p1.x - p0.x) * t, p0.y + (p1.y - p0.y) * t)
}
