//! Bounding-box helpers over kurbo geometry.

use crate::{BezPath, Rect};
use kurbo::Shape;

/// Bounding box of a bezier path, or [`None`] for an empty path.
///
/// The box is the mathematically exact bound of the segments (kurbo's
/// `bounding_box`), which may slightly overshoot the on-curve extremes.
#[must_use]
pub fn path_bounds(path: &BezPath) -> Option<Rect> {
    if path.is_empty() {
        None
    } else {
        Some(path.bounding_box())
    }
}

/// Smallest rectangle covering every input rectangle, or [`None`] if the
/// iterator is empty.
#[must_use]
pub fn union_all(rects: impl IntoIterator<Item = Rect>) -> Option<Rect> {
    rects.into_iter().reduce(|acc, r| acc.union(r))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn union_all_covers_all() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 20.0, 12.0);
        let u = union_all([a, b]).expect("two rects");
        assert!((u.x0 - 0.0).abs() < 1e-9);
        assert!((u.x1 - 20.0).abs() < 1e-9);
        assert!((u.y1 - 12.0).abs() < 1e-9);
    }

    #[test]
    fn union_all_empty_is_none() {
        assert!(union_all(std::iter::empty::<Rect>()).is_none());
    }
}
