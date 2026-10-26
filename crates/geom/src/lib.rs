//! Geometry foundations for `FireCraft`: affine transforms, bezier paths,
//! bounds, and hit testing built on [`kurbo`].
//!
//! Layer L0: depends on nothing from this workspace.

pub mod bounds;
pub mod hit;
pub mod stroke;
pub mod transform;

pub use kurbo::{Affine, BezPath, Point, Rect, Vec2};
pub use stroke::{STROKE_TOLERANCE, stroke_outline};
pub use transform::{map_rect, scale_about};

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::bounds::path_bounds;

    #[test]
    fn affine_inverse_round_trip() {
        let a = Affine::new([1.5, 0.5, -0.25, 2.0, 10.0, -3.0]);
        let inv = a.inverse();
        let m = a * inv;
        for (got, want) in m
            .as_coeffs()
            .iter()
            .zip(Affine::IDENTITY.as_coeffs().iter())
        {
            assert!((got - want).abs() < 1e-9, "coefficient {got} != {want}");
        }
    }

    #[test]
    fn affine_transforms_points() {
        let t = Affine::translate((10.0, 20.0));
        let p = t * Point { x: 1.0, y: 2.0 };
        assert!((p.x - 11.0).abs() < 1e-9 && (p.y - 22.0).abs() < 1e-9);
    }

    #[test]
    fn square_path_bounds() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.line_to((10.0, 10.0));
        p.line_to((0.0, 10.0));
        p.close_path();
        let bb = path_bounds(&p).expect("non-empty path has bounds");
        assert!((bb.x0 - 0.0).abs() < 1e-9);
        assert!((bb.y0 - 0.0).abs() < 1e-9);
        assert!(
            (bb.x1 - 10.0).abs() < 1e-6,
            "bezier bound may overshoot slightly"
        );
        assert!((bb.y1 - 10.0).abs() < 1e-6);
    }

    #[test]
    fn empty_path_has_no_bounds() {
        assert!(path_bounds(&BezPath::new()).is_none());
    }
}
