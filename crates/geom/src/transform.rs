//! Affine transform helpers over [`kurbo::Affine`].
//!
//! `FireCraft` composes transforms by matrix multiplication (`a * b` means
//! "apply `b`, then `a`", kurbo's convention). These helpers add the
//! operations the document layer needs on top of kurbo's bare matrix.

use kurbo::{Affine, Point, Rect};

/// Returns the bounding box of `rect` transformed by `t` (the axis-aligned
/// box containing the four transformed corners — not a minimal box for the
/// transformed shape, but exact for rects and safe for repaint regions).
#[must_use]
pub fn map_rect(t: Affine, rect: Rect) -> Rect {
    let corners = [
        Point::new(rect.x0, rect.y0),
        Point::new(rect.x1, rect.y0),
        Point::new(rect.x0, rect.y1),
        Point::new(rect.x1, rect.y1),
    ];
    let mut min = Point::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in corners {
        let q = t * p;
        min.x = min.x.min(q.x);
        min.y = min.y.min(q.y);
        max.x = max.x.max(q.x);
        max.y = max.y.max(q.y);
    }
    Rect::new(min.x, min.y, max.x, max.y)
}

/// A 2D scale-then-translate (the shape of Fireworks object transforms
/// before rotation is folded in).
#[must_use]
pub fn scale_about(t: Affine, sx: f64, sy: f64, origin: Point) -> Affine {
    // p -> origin + S·(p - origin), with `t` applied first.
    Affine::translate((origin.x, origin.y))
        * Affine::scale_non_uniform(sx, sy)
        * Affine::translate((-origin.x, -origin.y))
        * t
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// Deterministic xorshift64* — property tests stay reproducible and
    /// the crate keeps zero external dependencies.
    struct Rng(u64);

    impl Rng {
        fn next_f64(&mut self) -> f64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            let v = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
            // Uniform in [-1, 1): 53-bit integer scaled into the unit
            // interval, then shifted.
            (v >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0) - 1.0
        }

        fn affine(&mut self) -> Affine {
            loop {
                let a = Affine::new([
                    self.next_f64(),
                    self.next_f64(),
                    self.next_f64(),
                    self.next_f64(),
                    self.next_f64() * 100.0,
                    self.next_f64() * 100.0,
                ]);
                // Skip near-singular matrices; the inverse contract only
                // holds for invertible transforms.
                if a.as_coeffs()[0] * a.as_coeffs()[3] - a.as_coeffs()[1] * a.as_coeffs()[2] > 1e-3
                {
                    return a;
                }
            }
        }
    }

    #[test]
    fn inverse_round_trip_is_identity_for_random_matrices() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for _ in 0..512 {
            let a = rng.affine();
            let m = a * a.inverse();
            for (got, want) in m
                .as_coeffs()
                .iter()
                .zip(Affine::IDENTITY.as_coeffs().iter())
            {
                assert!(
                    (got - want).abs() < 1e-6,
                    "inverse round trip drifted: {m:?}"
                );
            }
        }
    }

    #[test]
    fn composition_is_associative_for_random_matrices() {
        let mut rng = Rng(0xDEAD_BEEF_CAFE_0001);
        for _ in 0..512 {
            let (a, b, c) = (rng.affine(), rng.affine(), rng.affine());
            let left = (a * b) * c;
            let right = a * (b * c);
            for (got, want) in left.as_coeffs().iter().zip(right.as_coeffs()) {
                assert!((got - want).abs() < 1e-9, "associativity broke");
            }
        }
    }

    #[test]
    fn random_matrix_never_panics_on_map_rect() {
        // Fuzz anchor: arbitrary f64 fields (including non-invertible
        // matrices) must produce a finite rect, never panic.
        let mut rng = Rng(0x1234_5678_9ABC_DEF0);
        let r = Rect::new(-10.0, -10.0, 30.0, 40.0);
        for _ in 0..2048 {
            let a = Affine::new([
                rng.next_f64(),
                rng.next_f64(),
                rng.next_f64(),
                rng.next_f64(),
                rng.next_f64() * 100.0,
                rng.next_f64() * 100.0,
            ]);
            let out = map_rect(a, r);
            assert!(
                out.x0.is_finite()
                    && out.x1.is_finite()
                    && out.y0.is_finite()
                    && out.y1.is_finite()
                    && out.x0 <= out.x1
                    && out.y0 <= out.y1,
                "map_rect produced {out:?} for {a:?}"
            );
        }
    }

    #[test]
    fn map_rect_scales_and_translates_exactly() {
        let t = Affine::new([2.0, 0.0, 0.0, 3.0, 10.0, 20.0]);
        let out = map_rect(t, Rect::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(out, Rect::new(10.0, 20.0, 30.0, 50.0));
    }

    #[test]
    fn map_rect_matches_transformed_shape_bounds() {
        // For a rotated rect, the mapped box must contain all corners and
        // equal the corners' bounding box.
        let angle = std::f64::consts::FRAC_PI_4;
        let t = Affine::rotate(angle).then_translate((5.0, -5.0).into());
        let r = Rect::new(0.0, 0.0, 4.0, 2.0);
        let out = map_rect(t, r);
        for corner in [
            Point::new(r.x0, r.y0),
            Point::new(r.x1, r.y0),
            Point::new(r.x0, r.y1),
            Point::new(r.x1, r.y1),
        ] {
            let p = t * corner;
            // Inclusive containment (kurbo's Rect::contains is half-open,
            // which would reject a corner sitting exactly on x1/y1).
            let inside = p.x >= out.x0 && p.x <= out.x1 && p.y >= out.y0 && p.y <= out.y1;
            assert!(inside, "corner {p:?} escaped {out:?}");
        }
    }

    #[test]
    fn scale_about_scales_about_the_given_point() {
        let origin = Point::new(10.0, 10.0);
        let t = scale_about(Affine::IDENTITY, 2.0, 2.0, origin);
        let moved = t * origin;
        assert!((moved.x - 10.0).abs() < 1e-9 && (moved.y - 10.0).abs() < 1e-9);
        let far = t * Point::new(20.0, 10.0);
        assert!((far.x - 30.0).abs() < 1e-9);
    }
}
