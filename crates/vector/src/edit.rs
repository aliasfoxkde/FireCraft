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
use geom::{Point, Rect, Vec2};

use crate::VectorError;

/// How far `p` sits outside `rect`: 0 inside or on the rect, else the
/// Euclidean distance to it (the reshape-area falloff metric).
fn outside_distance(rect: Rect, p: Point) -> f64 {
    let (dx, dy) = (
        (rect.x0 - p.x).max(p.x - rect.x1).max(0.0),
        (rect.y0 - p.y).max(p.y - rect.y1).max(0.0),
    );
    (dx * dx + dy * dy).sqrt()
}

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

/// Reshape-area drag: every anchor inside the rectangle moves by the
/// full drag delta (a rigid drag of the enclosed region), then falls
/// off linearly across the `falloff` band outside the rect, reaching
/// zero at `falloff` past the edge — the CS6 Reshape Area tool's
/// "grab an area, drag it" read, with the feathered rim so the edit
/// never tears the path at the boundary.
///
/// # Errors
/// [`VectorError::InvalidParameter`] when `falloff` is negative or
/// non-finite, or the delta is not finite.
pub fn reshape_area(
    path: &PathObject,
    rect: Rect,
    falloff: f64,
    delta: Vec2,
) -> Result<PathObject, VectorError> {
    if !(falloff.is_finite() && falloff >= 0.0) || !delta.is_finite() {
        return Err(VectorError::InvalidParameter);
    }
    let mut shaped = path.clone();
    for contour in &mut shaped.contours {
        for point in &mut contour.points {
            let d = outside_distance(rect, point.anchor);
            if d == 0.0 {
                let shift = delta;
                point.anchor += shift;
                point.primary = point.primary.map(|handle| handle + shift);
                point.secondary = point.secondary.map(|handle| handle + shift);
            } else if d < falloff {
                let weight = 1.0 - d / falloff;
                let shift = delta * weight;
                point.anchor += shift;
                point.primary = point.primary.map(|handle| handle + shift);
                point.secondary = point.secondary.map(|handle| handle + shift);
            }
        }
    }
    Ok(shaped)
}

/// Redraws a contour span: replaces the points `from..to` with the
/// drawn replacement (the drawn path supplies its own endpoints), so
/// the redraw tool's fresh stroke swaps in wholesale. Indices are
/// validated against the input before anything is produced.
///
/// # Errors
/// [`VectorError::PointOutOfBounds`] when the contour or either span
/// index does not exist, [`VectorError::InvalidParameter`] when the
/// span is empty and the replacement is not (or vice versa).
pub fn redraw_span(
    path: &PathObject,
    contour: usize,
    from: usize,
    to: usize,
    drawn: Vec<doc::PathPoint>,
) -> Result<PathObject, VectorError> {
    let Some(c) = path.contours.get(contour) else {
        return Err(VectorError::PointOutOfBounds { contour, point: 0 });
    };
    if from >= to || to > c.points.len() {
        return Err(VectorError::PointOutOfBounds {
            contour,
            point: if from >= to { from } else { to },
        });
    }
    if drawn.is_empty() {
        return Err(VectorError::InvalidParameter);
    }
    let mut redrawn = path.clone();
    let target = &mut redrawn.contours[contour];
    target.points.splice(from..to, drawn);
    target.point_count = target.points.len() as i64;
    Ok(redrawn)
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
    use doc::PathPoint;

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

    /// A plain 4-point square 100x50 at the origin (no autoshape
    /// corner splits — point indices stay corner-shaped).
    fn square_path() -> PathObject {
        let pt = |x: f64, y: f64| PathPoint {
            anchor: Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        };
        PathObject {
            contours: vec![doc::Contour {
                closed: true,
                point_count: 4,
                points: vec![pt(0.0, 0.0), pt(100.0, 0.0), pt(100.0, 50.0), pt(0.0, 50.0)],
            }],
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

    #[test]
    fn reshape_area_drags_the_enclosed_region_rigidly() {
        let path = square_path();
        // Rect covering the left half of the 100x50 rect; drag +20,+10.
        let area = Rect::new(-10.0, -10.0, 60.0, 60.0);
        let shaped = reshape_area(&path, area, 0.0, Vec2::new(20.0, 10.0)).expect("valid reshape");
        // Points 0 and 3 (x = 0) are inside: full delta, exact.
        let inside = &shaped.contours[0].points[0];
        assert!((inside.anchor.x - 20.0).abs() < 1e-12);
        assert!((inside.anchor.y - 10.0).abs() < 1e-12);
        // Points 1 and 2 (x = 100) are far outside and falloff 0: they
        // hold still.
        let outside = &shaped.contours[0].points[1];
        assert!((outside.anchor.x - 100.0).abs() < 1e-12);
        assert!(outside.anchor.y == 0.0);
        // Input untouched.
        assert!(path.contours[0].points[0].anchor.x == 0.0);
    }

    #[test]
    fn reshape_area_falls_off_across_the_band() {
        let path = square_path();
        // Rect ending at x = 40 with a 20-wide band: the point at
        // x = 50 sits 10 into the band → half weight.
        let area = Rect::new(-1000.0, -1000.0, 40.0, 1000.0);
        let shaped = reshape_area(&path, area, 20.0, Vec2::new(30.0, 0.0)).expect("valid reshape");
        let banded = &shaped.contours[0].points[1]; // anchor (100, 0)
        // d = 100 − 40 = 60 ≥ falloff 20: still zero.
        assert!(banded.anchor.x == 100.0);
        // A point exactly at the band's midpoint: use the corner
        // rectangle with the rect ending at x = 95 — point (100, 0) is
        // d = 5 into a 10-wide band → half the delta.
        let area = Rect::new(-1000.0, -1000.0, 95.0, 1000.0);
        let shaped = reshape_area(&path, area, 10.0, Vec2::new(30.0, 0.0)).expect("valid reshape");
        let banded = &shaped.contours[0].points[1];
        assert!((banded.anchor.x - 115.0).abs() < 1e-12, "half of 30");
    }

    #[test]
    fn reshape_area_rejects_bad_falloff() {
        let path = square_path();
        let area = Rect::new(0.0, 0.0, 10.0, 10.0);
        let error = reshape_area(&path, area, -1.0, Vec2::new(1.0, 0.0))
            .expect_err("negative falloff refused");
        assert!(matches!(error, VectorError::InvalidParameter));
        let error = reshape_area(&path, area, f64::NAN, Vec2::new(1.0, 0.0))
            .expect_err("NaN falloff refused");
        assert!(matches!(error, VectorError::InvalidParameter));
    }

    #[test]
    fn redraw_span_swaps_the_drawn_points_in() {
        let path = square_path();
        // Replace the top edge (points 0..2) with a drawn bulge.
        let drawn = vec![
            PathPoint {
                anchor: Point::new(0.0, 0.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
            PathPoint {
                anchor: Point::new(50.0, -25.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
            PathPoint {
                anchor: Point::new(100.0, 0.0),
                primary: None,
                secondary: None,
                is_curve: false,
            },
        ];
        let redrawn = redraw_span(&path, 0, 0, 2, drawn).expect("valid redraw");
        let contour = &redrawn.contours[0];
        assert_eq!(contour.points.len(), 5, "two corners + one bulge point");
        assert_eq!(contour.point_count, 5);
        assert!(
            (contour.points[1].anchor.y - -25.0).abs() < 1e-12,
            "the drawn point sits mid-span"
        );
        // The untouched corners keep their geometry.
        assert!((contour.points[3].anchor.x - 100.0).abs() < 1e-12);
        assert!((contour.points[3].anchor.y - 50.0).abs() < 1e-12);
    }

    #[test]
    fn redraw_span_validates_its_span() {
        let path = square_path();
        let drawn = vec![PathPoint {
            anchor: Point::new(1.0, 1.0),
            primary: None,
            secondary: None,
            is_curve: false,
        }];
        let error = redraw_span(&path, 9, 0, 2, drawn.clone()).expect_err("no contour 9");
        assert!(matches!(
            error,
            VectorError::PointOutOfBounds { contour: 9, .. }
        ));
        let error = redraw_span(&path, 0, 2, 2, drawn.clone()).expect_err("empty span refused");
        assert!(matches!(
            error,
            VectorError::PointOutOfBounds { point: 2, .. }
        ));
        let error =
            redraw_span(&path, 0, 0, 99, drawn.clone()).expect_err("to past the end refused");
        assert!(matches!(
            error,
            VectorError::PointOutOfBounds { point: 99, .. }
        ));
        let error = redraw_span(&path, 0, 0, 2, Vec::new()).expect_err("empty drawn refused");
        assert!(matches!(error, VectorError::InvalidParameter));
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
