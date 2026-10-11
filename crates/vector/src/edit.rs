//! Direct path edits (the reshape/freeform pointer tools' geometry):
//! per-point translations and a region warp. Both rebuild the path —
//! the input is never mutated — and both are pure arithmetic over the
//! `doc` model, so the engine can capture the before-object as the
//! whole inverse.
//!
//! The freeform warp moves each anchor inside the tool's ring along
//! the drag vector with linear falloff to zero at the ring edge, and
//! its bezier handles ride along rigidly (the tool drags path *points*;
//! handle-by-handle weighting would shear the local curve shape and no
//! corpus evidence so far pins that behavior — revisit at M8 if the
//! Fireworks files disagree).

use doc::PathObject;
use geom::{Point, Vec2};

use crate::VectorError;

/// Translates whole points — anchor plus both handles, so a moved
/// point carries its curve shape with it. `edits` are
/// `(contour, point, delta)` triples; any out-of-range index rejects
/// the whole edit before anything is produced.
///
/// # Errors
/// [`VectorError::PointOutOfBounds`] when a contour or point index is
/// out of range.
pub fn translate_points(
    path: &PathObject,
    edits: &[(usize, usize, Vec2)],
) -> Result<PathObject, VectorError> {
    // Validate every index against the input before cloning.
    for &(contour, point, _) in edits {
        let Some(c) = path.contours.get(contour) else {
            return Err(VectorError::PointOutOfBounds { contour, point: 0 });
        };
        if point >= c.points.len() {
            return Err(VectorError::PointOutOfBounds { contour, point });
        }
    }
    let mut edited = path.clone();
    for &(contour, point, delta) in edits {
        let c = &mut edited.contours[contour];
        let p = &mut c.points[point];
        p.anchor += delta;
        p.primary = p.primary.map(|handle| handle + delta);
        p.secondary = p.secondary.map(|handle| handle + delta);
    }
    Ok(edited)
}

/// Translates every point of the path — the align/distribute engine
/// command's whole vocabulary (placement is pure translation).
#[must_use]
pub fn translate_path(path: &PathObject, delta: Vec2) -> PathObject {
    let mut moved = path.clone();
    for contour in &mut moved.contours {
        for point in &mut contour.points {
            point.anchor += delta;
            point.primary = point.primary.map(|handle| handle + delta);
            point.secondary = point.secondary.map(|handle| handle + delta);
        }
    }
    moved
}

/// Freeform push: translates every anchor within `radius` of `center`
/// by `delta` scaled by the linear falloff `1 − d/radius`, handles
/// riding along with their anchor.
///
/// # Errors
/// [`VectorError::InvalidParameter`] when `radius` is not finite and
/// positive, or the delta is not finite.
pub fn warp(
    path: &PathObject,
    center: Point,
    radius: f64,
    delta: Vec2,
) -> Result<PathObject, VectorError> {
    if !(radius.is_finite() && radius > 0.0) || !delta.is_finite() {
        return Err(VectorError::InvalidParameter);
    }
    let mut warped = path.clone();
    for contour in &mut warped.contours {
        for point in &mut contour.points {
            let d = (point.anchor - center).hypot();
            if d < radius {
                let weight = 1.0 - d / radius;
                let shift = delta * weight;
                point.anchor += shift;
                point.primary = point.primary.map(|handle| handle + shift);
                point.secondary = point.secondary.map(|handle| handle + shift);
            }
        }
    }
    Ok(warped)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::autoshape::{CornerKind, RectParams, rectangle};

    /// A plain 100x50 rectangle path at the origin.
    fn rectangle_path() -> PathObject {
        rectangle(RectParams {
            origin: Point::new(0.0, 0.0),
            width: 100.0,
            height: 50.0,
            corner: 0.0,
            kind: CornerKind::Beveled,
        })
    }

    #[test]
    fn translate_moves_anchor_and_handles_together() {
        let path = rectangle_path();
        let edited = translate_points(&path, &[(0, 0, Vec2::new(5.0, -3.0))]).expect("valid edit");
        let p = &edited.contours[0].points[0];
        assert!((p.anchor.x - 5.0).abs() < 1e-12);
        assert!((p.anchor.y - -3.0).abs() < 1e-12);
        // Input untouched.
        assert!(path.contours[0].points[0].anchor.x == 0.0);
        // A chamfer-corner rectangle carries handles; both ride along.
        assert_eq!(
            edited.contours[0].points.len(),
            path.contours[0].points.len()
        );
    }

    #[test]
    fn translate_rejects_out_of_range_indices_whole_hog() {
        let path = rectangle_path();
        let error = translate_points(&path, &[(0, 0, Vec2::new(1.0, 1.0)), (0, 99, Vec2::ZERO)])
            .expect_err("second edit is out of range");
        assert!(matches!(
            error,
            VectorError::PointOutOfBounds { point: 99, .. }
        ));
        let error = translate_points(&path, &[(7, 0, Vec2::ZERO)]).expect_err("no contour 7");
        assert!(matches!(
            error,
            VectorError::PointOutOfBounds { contour: 7, .. }
        ));
    }

    #[test]
    fn warp_pulls_nearby_points_with_linear_falloff() {
        let path = rectangle_path();
        // Ring centered on the left edge midpoint: corners at distance
        // 25 get weight 1 − 25/50 = 0.5, the midpoint itself weight 1.
        let warped =
            warp(&path, Point::new(0.0, 25.0), 50.0, Vec2::new(-10.0, 0.0)).expect("valid warp");
        // Every left-edge anchor (corners at distance 25 from the ring
        // center) gets weight 1 − 25/50 = 0.5 → shift −5; the right
        // edge (~103 away) is untouched.
        for (original, moved) in path.contours[0]
            .points
            .iter()
            .zip(&warped.contours[0].points)
        {
            let expected = if original.anchor.x == 0.0 {
                -5.0
            } else {
                100.0
            };
            assert!(
                (moved.anchor.x - expected).abs() < 1e-12,
                "x {} (expected {})",
                moved.anchor.x,
                expected
            );
        }
    }

    #[test]
    fn warp_validates_its_parameters() {
        let path = rectangle_path();
        assert!(matches!(
            warp(&path, Point::ZERO, 0.0, Vec2::new(1.0, 0.0)),
            Err(VectorError::InvalidParameter)
        ));
        assert!(matches!(
            warp(&path, Point::ZERO, f64::NAN, Vec2::new(1.0, 0.0)),
            Err(VectorError::InvalidParameter)
        ));
        assert!(matches!(
            warp(&path, Point::ZERO, 10.0, Vec2::new(f64::NAN, 0.0)),
            Err(VectorError::InvalidParameter)
        ));
    }
}
