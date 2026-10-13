//! The brush stamp (M15): the CS6 Edit Brush dialog's slots — Shape,
//! Size, Edge, Aspect, Angle, Square (`DIALOG 5964`, the
//! `CEditBrushDialog` family) — as one deterministic integer-exact
//! footprint shared by every brush-based tool (paint, eraser, retouch).
//!
//! Coverage discipline (the M7 contract): a pixel's coverage 0..=255
//! is computed in `f64` from basic operations only and rounded
//! half-up, so the same dab produces the same bytes on every run and
//! undo is byte-exact. Pressure is the per-dab sensitivity multiplier
//! (the Sensitivity tab); the caller varies it per dab, the engine
//! folds it into the effect like strength.

/// Which footprint the brush stamps. The dialog's "Square" checkbox is
/// the square shape; `Aspect`/`Angle` apply to both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BrushShape {
    /// Soft circular footprint.
    Round,
    /// Hard-edged square footprint (the dialog's Square flag).
    Square,
}

/// The dialog's option slots: `edge` is the softness percent 0..=100
/// (0 = hard rim, 100 = falloff from the center — 50 matches the M7
/// half-radius falloff); `aspect` is the minor axis as a percent of
/// the diameter (1..=100); `angle` is the rotation in degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrushOptions {
    pub(crate) edge: u8,
    pub(crate) shape: BrushShape,
    pub(crate) aspect: u8,
    pub(crate) angle: i16,
}

/// Shared limits, in one place so every tool refuses identically.
pub(crate) const MAX_DIAMETER: u32 = 1024;
pub(crate) const MAX_EDGE: u8 = 100;
pub(crate) const MAX_ASPECT: u8 = 100;

impl BrushOptions {
    /// Validates the slots; `Err` carries the calling command's name
    /// so refusals read from the surface they came from.
    pub(crate) fn new(
        name: &'static str,
        edge: u8,
        shape: BrushShape,
        aspect: u8,
        angle: i16,
    ) -> Result<Self, EngineError> {
        if edge > MAX_EDGE {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("brush edge {edge} is not in 0..={MAX_EDGE}"),
            });
        }
        if aspect == 0 || aspect > MAX_ASPECT {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("brush aspect {aspect} is not in 1..={MAX_ASPECT}"),
            });
        }
        Ok(Self {
            edge,
            shape,
            aspect,
            angle,
        })
    }
}

/// A brush dab: center and size (the dialog's Size slot — the brush
/// **diameter** in pixels) plus the [`BrushOptions`] slots. Pressure
/// is the per-dab sensitivity multiplier percent 1..=100 and rides
/// with the effect, not the footprint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Brush {
    center_x: f64,
    center_y: f64,
    size: u32,
    options: BrushOptions,
}

impl Brush {
    /// Validates and folds a dab.
    pub(crate) fn new(
        name: &'static str,
        center_x: f64,
        center_y: f64,
        size: u32,
        options: BrushOptions,
    ) -> Result<Self, EngineError> {
        if size == 0 || size > MAX_DIAMETER {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("brush size {size} is not in 1..={MAX_DIAMETER}"),
            });
        }
        Ok(Self {
            center_x,
            center_y,
            size,
            options,
        })
    }

    /// The stamp's radius (half the diameter), in pixels.
    fn radius(&self) -> f64 {
        f64::from(self.size) / 2.0
    }

    /// The pixel-center offset rotated into the brush's own frame:
    /// angle applied first (the brush rotates), then the aspect
    /// squeeze along the brush's minor axis (y in brush space).
    fn brush_space(&self, px: u32, py: u32) -> (f64, f64) {
        let dx = f64::from(px) + 0.5 - self.center_x;
        let dy = f64::from(py) + 0.5 - self.center_y;
        let radians = f64::from(self.options.angle) * std::f64::consts::PI / 180.0;
        let (sin, cos) = radians.sin_cos();
        let rotated_x = dx * cos + dy * sin;
        let rotated_y = -dx * sin + dy * cos;
        let squeeze = f64::from(self.options.aspect) / 100.0;
        (rotated_x, rotated_y / squeeze)
    }

    /// Normalized distance from the center in brush units: 0 at the
    /// center, 1 at the rim (the ellipse for round, the rotated square
    /// boundary for square).
    fn distance(&self, px: u32, py: u32) -> f64 {
        let (x, y) = self.brush_space(px, py);
        match self.options.shape {
            BrushShape::Round => (x * x + y * y).sqrt() / self.radius(),
            BrushShape::Square => {
                let half = self.radius();
                (x.abs().max(y.abs())) / half
            }
        }
    }

    /// Coverage 0..=255 of the pixel at `px, py`: full strength out to
    /// the inner bound set by `edge`, then a linear falloff to the rim
    /// — the M7 profile generalized from its hardcoded half-radius
    /// (edge 50) to the dialog's whole softness range.
    pub(crate) fn coverage(&self, px: u32, py: u32) -> u8 {
        let distance = self.distance(px, py);
        if distance >= 1.0 {
            return 0;
        }
        let inner = f64::from(MAX_EDGE - self.options.edge) / f64::from(MAX_EDGE);
        if distance <= inner {
            return 255;
        }
        let falloff = 1.0 - (distance - inner) / (1.0 - inner);
        ((255.0 * falloff) + 0.5) as u8
    }

    /// The per-dab effect 0..=255: coverage × strength × pressure ×
    /// selection, each percent folded with half-up rounding — the
    /// same algebra the M7 tools apply, pressure included.
    pub(crate) fn effect(
        &self,
        px: u32,
        py: u32,
        strength: u32,
        pressure: u32,
        selection: u8,
    ) -> u8 {
        let scaled = (u32::from(self.coverage(px, py)) * strength * pressure + 5_000) / 10_000;
        ((scaled * u32::from(selection) + 127) / 255) as u8
    }
}

use crate::error::EngineError;

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn dab(size: u32, edge: u8) -> Brush {
        Brush::new(
            "test",
            10.0,
            10.0,
            size,
            opts(edge, BrushShape::Round, 100, 0),
        )
        .unwrap()
    }

    fn opts(edge: u8, shape: BrushShape, aspect: u8, angle: i16) -> BrushOptions {
        BrushOptions::new("test", edge, shape, aspect, angle).unwrap()
    }

    /// Sums coverage over a square window around the center.
    fn footprint(brush: &Brush, window: u32) -> u64 {
        let mut total = 0u64;
        for py in 0..window {
            for px in 0..window {
                total += u64::from(brush.coverage(px, py));
            }
        }
        total
    }

    #[test]
    fn edge_zero_is_a_hard_brush() {
        let brush = dab(8, 0);
        // Rim pixels (distance just under 1) are full strength.
        assert_eq!(brush.coverage(10, 13), 255, "near-rim stays hard");
        assert_eq!(brush.coverage(14, 10), 0, "outside is nothing");
    }

    #[test]
    fn edge_fifty_matches_the_m7_falloff() {
        // The M7 profile: full to half the radius, linear to the rim.
        // Center (10.5-ish): probe a pixel in the falloff band and pin
        // the exact byte the old formula produced.
        let brush = dab(8, 50);
        // Pixel (13, 10): offset (3.5, 0.5) from the center — the
        // M7 distance is the full hypotenuse, not just dx.
        let value = brush.coverage(13, 10);
        let legacy = {
            let dx = f64::from(13) + 0.5 - 10.0;
            let dy = f64::from(10) + 0.5 - 10.0;
            let distance = (dx * dx + dy * dy).sqrt();
            let radius = 4.0;
            let inner = radius / 2.0;
            ((255.0 * (radius - distance) / inner) + 0.5) as u8
        };
        assert_eq!(value, legacy, "edge 50 is byte-identical to the M7 falloff");
    }

    #[test]
    fn edge_hundred_falls_off_from_the_center() {
        let brush = dab(8, 100);
        // The whole radius is falloff: the mid-band pixel is a
        // quarter-covered where edge 0 keeps it hard.
        assert_eq!(brush.coverage(13, 10), 30);
        let hard = dab(8, 0);
        assert_eq!(hard.coverage(13, 10), 255);
        assert!(
            footprint(&brush, 16) < footprint(&hard, 16),
            "more edge means less total coverage"
        );
    }

    #[test]
    fn aspect_squeezes_the_minor_axis() {
        let wide = Brush::new("test", 10.0, 10.0, 8, opts(0, BrushShape::Round, 50, 0)).unwrap();
        // Half aspect: the vertical reach halves, the horizontal holds.
        assert_eq!(wide.coverage(13, 10), 255, "horizontal reach holds");
        assert_eq!(wide.coverage(10, 12), 0, "vertical reach squeezes");
    }

    #[test]
    fn angle_rotates_the_footprint() {
        let tilted = Brush::new("test", 10.0, 10.0, 8, opts(0, BrushShape::Round, 50, 90)).unwrap();
        // Aspect 50 squeezed along brush-y; a 90° angle turns that
        // squeeze horizontal.
        assert_eq!(
            tilted.coverage(10, 12),
            255,
            "the wide axis is vertical now"
        );
        assert_eq!(
            tilted.coverage(13, 10),
            0,
            "the squeezed axis is horizontal now"
        );
    }

    #[test]
    fn square_shape_reaches_the_corners() {
        let round = dab(8, 0);
        let square =
            Brush::new("test", 10.0, 10.0, 8, opts(0, BrushShape::Square, 100, 0)).unwrap();
        // Corner of the bounding box: inside the square's Chebyshev
        // rim, outside the circle.
        let (px, py) = (13u32, 13u32);
        assert_eq!(square.coverage(px, py), 255);
        assert_eq!(round.coverage(px, py), 0);
    }

    #[test]
    fn effect_folds_strength_pressure_and_selection() {
        let brush = dab(8, 0);
        // Full coverage: strength 50 × pressure 50 = a quarter effect.
        assert_eq!(brush.effect(10, 10, 50, 50, 255), 64);
        // Selection halves it again.
        assert_eq!(brush.effect(10, 10, 50, 50, 128), 32);
        assert_eq!(brush.effect(14, 10, 100, 100, 255), 0, "outside is nothing");
    }

    #[test]
    fn refusals_carry_the_calling_command() {
        let error = Brush::new(
            "retouch.blur",
            0.0,
            0.0,
            0,
            opts(0, BrushShape::Round, 100, 0),
        )
        .expect_err("size 0 refused");
        assert!(error.to_string().contains("retouch.blur"));
        let error = BrushOptions::new("retouch.blur", 101, BrushShape::Round, 100, 0)
            .expect_err("edge >100 refused");
        assert!(error.to_string().contains("edge"));
        let error = BrushOptions::new("retouch.blur", 0, BrushShape::Round, 0, 0)
            .expect_err("aspect 0 refused");
        assert!(error.to_string().contains("aspect"));
    }
}
