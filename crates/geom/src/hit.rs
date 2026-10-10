//! Hit-testing helpers over kurbo geometry.

use crate::{BezPath, Point, Rect};
use kurbo::ParamCurveNearest;

/// Whether `pt` lies inside `r` (edges inclusive).
#[must_use]
pub fn point_in_rect(pt: Point, r: Rect) -> bool {
    pt.x >= r.x0 && pt.x <= r.x1 && pt.y >= r.y0 && pt.y <= r.y1
}

/// Approximate distance from `pt` to the nearest point on `path`.
///
/// Returns infinity for an empty path.
#[must_use]
pub fn distance_to_path(path: &BezPath, pt: Point) -> f64 {
    path.segments()
        .map(|seg| seg.nearest(pt, 1e-3).distance_sq.sqrt())
        .fold(f64::INFINITY, f64::min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_in_rect_edges() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(point_in_rect(Point { x: 0.0, y: 0.0 }, r));
        assert!(point_in_rect(Point { x: 5.0, y: 5.0 }, r));
        assert!(!point_in_rect(Point { x: 10.1, y: 5.0 }, r));
    }

    #[test]
    fn distance_to_line() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        let d = distance_to_path(&p, Point { x: 5.0, y: 3.0 });
        assert!((d - 3.0).abs() < 1e-3, "got {d}");
    }

    #[test]
    fn distance_to_empty_path_is_infinite() {
        let d = distance_to_path(&BezPath::new(), Point { x: 0.0, y: 0.0 });
        assert!(d.is_infinite());
    }
}
