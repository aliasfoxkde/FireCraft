//! Bridge between the `doc` path representation (`Contour` points with
//! per-point handles) and kurbo curve primitives.
//!
//! Segment rule: the span from point *i* to point *i+1* is a cubic iff
//! point *i* carries an out-handle (`secondary`) **and** point *i+1*
//! carries an in-handle (`primary`); otherwise it is a line. A stored
//! handle that fails the pairing (e.g. an in-handle on a point whose
//! incoming span is a line) is semantically inert — it drives no
//! geometry — so [`canonicalize_contour`] drops it and the doc→kurbo→doc
//! round trip is exact on canonical contours. The doc keeps its
//! format-mapped representation untouched; this bridge is vector-crate
//! internal working data.

use crate::VectorError;
use doc::model::{Contour, PathPoint};
use geom::{BezPath, Point};
use kurbo::{CubicBez, Line, PathEl};

/// One span of a contour: a true cubic or a straight line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Line(Line),
    Cubic(CubicBez),
}

impl Seg {
    /// Start point.
    #[must_use]
    pub fn start(&self) -> Point {
        match self {
            Self::Line(l) => Point::new(l.p0.x, l.p0.y),
            Self::Cubic(c) => Point::new(c.p0.x, c.p0.y),
        }
    }

    /// End point.
    #[must_use]
    pub fn end(&self) -> Point {
        match self {
            Self::Line(l) => Point::new(l.p1.x, l.p1.y),
            Self::Cubic(c) => Point::new(c.p3.x, c.p3.y),
        }
    }

    /// The same span traversed in the opposite direction.
    #[must_use]
    pub fn reversed(&self) -> Self {
        match self {
            Self::Line(l) => Self::Line(Line::new(l.p1, l.p0)),
            Self::Cubic(c) => Self::Cubic(CubicBez {
                p0: c.p3,
                p1: c.p2,
                p2: c.p1,
                p3: c.p0,
            }),
        }
    }
}

/// The spans of one contour, in path order. A contour with fewer than
/// two points produces no spans; a closed contour wraps last→first.
#[must_use]
pub fn contour_segments(contour: &Contour) -> Vec<Seg> {
    let pts = &contour.points;
    if pts.len() < 2 {
        return Vec::new();
    }
    let n = pts.len();
    let spans = if contour.closed { n } else { n - 1 };
    (0..spans)
        .map(|i| {
            let a = &pts[i];
            let b = &pts[(i + 1) % n];
            match (a.secondary, b.primary) {
                (Some(c1), Some(c2)) => Seg::Cubic(CubicBez {
                    p0: to_kurbo(a.anchor),
                    p1: to_kurbo(c1),
                    p2: to_kurbo(c2),
                    p3: to_kurbo(b.anchor),
                }),
                _ => Seg::Line(Line::new(to_kurbo(a.anchor), to_kurbo(b.anchor))),
            }
        })
        .collect()
}

/// Drops semantically inert handles: an in-handle whose incoming span
/// is a line, and an out-handle whose outgoing span is a line. Idempotent;
/// after this the doc→kurbo→doc round trip is exact.
#[must_use]
pub fn canonicalize_contour(contour: &Contour) -> Contour {
    let n = contour.points.len();
    if n < 2 {
        // No spans: every handle is inert.
        let points = contour
            .points
            .iter()
            .map(|p| PathPoint {
                anchor: p.anchor,
                primary: None,
                secondary: None,
                is_curve: false,
            })
            .collect();
        return Contour {
            points,
            ..contour.clone()
        };
    }
    let has_out = |i: usize| matches!(contour_segments_at(contour, i), Seg::Cubic(_));
    let points = contour
        .points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let prev = (i + n - 1) % n;
            let keep_primary = contour.closed || i > 0;
            let keep_secondary = contour.closed || i + 1 < n;
            let primary = if keep_primary && has_out(prev) {
                p.primary
            } else {
                None
            };
            let secondary = if keep_secondary && has_out(i) {
                p.secondary
            } else {
                None
            };
            PathPoint {
                anchor: p.anchor,
                primary,
                secondary,
                is_curve: primary.is_some() || secondary.is_some(),
            }
        })
        .collect();
    Contour {
        points,
        ..contour.clone()
    }
}

/// The span leaving point `i` (wrapping on closed contours).
fn contour_segments_at(contour: &Contour, i: usize) -> Seg {
    let pts = &contour.points;
    let n = pts.len();
    let a = &pts[i];
    let b = &pts[(i + 1) % n];
    match (a.secondary, b.primary) {
        (Some(c1), Some(c2)) => Seg::Cubic(CubicBez {
            p0: to_kurbo(a.anchor),
            p1: to_kurbo(c1),
            p2: to_kurbo(c2),
            p3: to_kurbo(b.anchor),
        }),
        _ => Seg::Line(Line::new(to_kurbo(a.anchor), to_kurbo(b.anchor))),
    }
}

/// Converts one contour to a kurbo `BezPath` (single subpath).
#[must_use]
pub fn contour_to_bezpath(contour: &Contour) -> BezPath {
    let mut path = BezPath::new();
    let mut spans = contour_segments(contour).into_iter();
    let Some(first) = spans.next() else {
        // Degenerate contour (0 or 1 points): anchor as a bare move.
        if let Some(p) = contour.points.first() {
            path.move_to(to_kurbo(p.anchor));
        }
        return path;
    };
    path.move_to(to_kurbo(first.start()));
    emit_span(&mut path, &first);
    for span in spans {
        emit_span(&mut path, &span);
    }
    if contour.closed {
        path.close_path();
    }
    path
}

fn emit_span(path: &mut BezPath, span: &Seg) {
    match *span {
        Seg::Line(l) => path.line_to(l.p1),
        Seg::Cubic(c) => path.curve_to(c.p1, c.p2, c.p3),
    }
}

/// Converts a single-subpath kurbo `BezPath` back to a contour. The
/// result is canonical: handles only where the paired span is a cubic.
///
/// # Errors
/// [`VectorError::EmptyPath`] for a path with no commands,
/// [`VectorError::NoCurrentPoint`] if it does not start with a move,
/// [`VectorError::MultiSubpath`] for more than one subpath.
pub fn bezpath_to_contour(path: &BezPath) -> Result<Contour, VectorError> {
    let mut points: Vec<PathPoint> = Vec::new();
    let mut closed = false;
    for el in path.iter() {
        match el {
            PathEl::MoveTo(p) => {
                if !points.is_empty() {
                    return Err(VectorError::MultiSubpath);
                }
                points.push(PathPoint {
                    anchor: from_kurbo(p),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                });
            }
            PathEl::LineTo(p) => {
                points.last_mut().ok_or(VectorError::NoCurrentPoint)?;
                points.push(PathPoint {
                    anchor: from_kurbo(p),
                    primary: None,
                    secondary: None,
                    is_curve: false,
                });
            }
            PathEl::CurveTo(c1, c2, p) => {
                let prev = points.last_mut().ok_or(VectorError::NoCurrentPoint)?;
                prev.secondary = Some(from_kurbo(c1));
                prev.is_curve = true;
                points.push(PathPoint {
                    anchor: from_kurbo(p),
                    primary: Some(from_kurbo(c2)),
                    secondary: None,
                    is_curve: true,
                });
            }
            PathEl::QuadTo(_, _) => return Err(VectorError::QuadraticSegment),
            PathEl::ClosePath => closed = true,
        }
    }
    if points.is_empty() {
        return Err(VectorError::EmptyPath);
    }
    // Our writer emits a closed contour's wrap span (last→first) as an
    // explicit segment before the ClosePath, so reading back yields a
    // duplicated start anchor. Merge it: the duplicate's in-handle is
    // the start point's incoming wrap handle.
    if closed && points.len() >= 2 && points[points.len() - 1].anchor == points[0].anchor {
        if let Some(wrap) = points.pop() {
            let first = &mut points[0];
            first.primary = wrap.primary;
            first.is_curve = first.secondary.is_some() || first.primary.is_some();
        }
    }
    Ok(Contour {
        closed,
        point_count: points.len() as i64,
        points,
    })
}

#[must_use]
fn to_kurbo(p: Point) -> kurbo::Point {
    kurbo::Point::new(p.x, p.y)
}

#[must_use]
fn from_kurbo(p: kurbo::Point) -> Point {
    Point::new(p.x, p.y)
}
