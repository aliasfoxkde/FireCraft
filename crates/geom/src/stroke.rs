//! Stroke-to-outline expansion, delegated to [`kurbo::stroke`].
//!
//! Fireworks renders strokes as expanded outlines; FireCraft keeps the
//! same model (a stroke produces a filled path), so this module is a thin
//! convenience wrapper that pins our default tolerance and options.

use kurbo::{BezPath, Stroke, StrokeOpts};

/// Default flattening tolerance for stroke expansion (pixels).
pub const STROKE_TOLERANCE: f64 = 0.1;

/// Expands `path` stroked with `style` into a filled outline path.
#[must_use]
pub fn stroke_outline(path: &BezPath, style: &Stroke) -> BezPath {
    kurbo::stroke(path.iter(), style, &StrokeOpts::default(), STROKE_TOLERANCE)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::bounds::path_bounds;
    use kurbo::{Cap, Join};

    #[test]
    fn line_expands_to_width_plus_caps() {
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((100.0, 0.0));

        let style = Stroke {
            width: 8.0,
            join: Join::Miter,
            miter_limit: 4.0,
            start_cap: Cap::Butt,
            end_cap: Cap::Butt,
            dash_pattern: Default::default(),
            dash_offset: 0.0,
        };
        let outline = stroke_outline(&line, &style);
        let bb = path_bounds(&outline).expect("stroke of a line has bounds");
        assert!(
            (bb.height() - 8.0).abs() < 0.5,
            "expected ~8 tall, got {bb:?}"
        );
        assert!(bb.width() >= 100.0, "butt-capped stroke covers the segment");
    }

    #[test]
    fn round_cap_extends_past_endpoints() {
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((100.0, 0.0));

        let style = Stroke {
            width: 10.0,
            join: Join::Round,
            miter_limit: 4.0,
            start_cap: Cap::Round,
            end_cap: Cap::Round,
            dash_pattern: Default::default(),
            dash_offset: 0.0,
        };
        let bb = path_bounds(&stroke_outline(&line, &style)).expect("stroke of a line has bounds");
        assert!(bb.x0 <= -5.0 + 0.1 && bb.x1 >= 105.0 - 0.1);
    }

    #[test]
    fn empty_path_strokes_to_empty() {
        let outline = stroke_outline(&BezPath::new(), &Stroke::new(1.0));
        assert!(outline.elements().is_empty());
    }
}
