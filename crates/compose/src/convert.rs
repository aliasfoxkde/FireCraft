//! Model → rasterizer conversions: ABGR colour ints, `BLDi` blend
//! values, and contour geometry.

use doc::{Contour, PathPoint};
use vello_cpu::kurbo::BezPath;
use vello_cpu::peniko::{Color, Mix};

/// Decodes a Fireworks `FCLi`/`BCLi` colour int (ABGR layout, per the
/// node dictionary) into a peniko colour.
pub(crate) fn abgr(raw: i64) -> Color {
    let raw = raw as u32;
    Color::from_rgba8(
        (raw & 0xFF) as u8,
        ((raw >> 8) & 0xFF) as u8,
        ((raw >> 16) & 0xFF) as u8,
        ((raw >> 24) & 0xFF) as u8,
    )
}

/// Maps a Fireworks `BLDi` value onto the rasterizer's mix modes. The
/// value table is the node dictionary's; 0 (and any value absent from
/// the dictionary) is normal painting. Documented values without a
/// rasterizer formula — dissolve, the extended burn/light family,
/// invert, erase, tint, average, negation, and the rest — fall back to
/// [`Mix::Normal`] until `compose` grows its own compositing passes
/// (M7); recorded in ADR 0002.
pub(crate) fn blend_mode(bldi: i64) -> Mix {
    match bldi {
        2 => Mix::Multiply,
        3 => Mix::Difference,
        4 => Mix::Overlay,
        5 => Mix::Screen,
        9 => Mix::Darken,
        10 => Mix::Lighten,
        11 => Mix::Hue,
        12 => Mix::Saturation,
        13 => Mix::Color,
        14 => Mix::Luminosity,
        32 => Mix::Exclusion,
        33 => Mix::HardLight,
        34 => Mix::SoftLight,
        36 => Mix::ColorDodge,
        37 => Mix::ColorBurn,
        _ => Mix::Normal,
    }
}

/// Emits the model's contours as a rasterizer path. One-sided bezier
/// controls collapse onto their segment endpoint (a missing control
/// contributes no curvature); contours with no points are skipped.
pub(crate) fn to_bezpath(contours: &[Contour]) -> BezPath {
    let mut path = BezPath::new();
    for contour in contours {
        let Some(first) = contour.points.first() else {
            continue;
        };
        path.move_to((first.anchor.x, first.anchor.y));
        for pair in contour.points.windows(2) {
            emit_segment(&mut path, &pair[0], &pair[1]);
        }
        if contour.closed && contour.points.len() > 1 {
            if let Some(last) = contour.points.last() {
                emit_segment(&mut path, last, first);
            }
            path.close_path();
        }
    }
    path
}

fn emit_segment(path: &mut BezPath, from: &PathPoint, to: &PathPoint) {
    match (&from.secondary, &to.primary) {
        (Some(out), Some(inc)) => {
            path.curve_to((out.x, out.y), (inc.x, inc.y), (to.anchor.x, to.anchor.y));
        }
        (Some(out), None) => {
            path.curve_to(
                (out.x, out.y),
                (to.anchor.x, to.anchor.y),
                (to.anchor.x, to.anchor.y),
            );
        }
        (None, Some(inc)) => {
            path.curve_to(
                (from.anchor.x, from.anchor.y),
                (inc.x, inc.y),
                (to.anchor.x, to.anchor.y),
            );
        }
        (None, None) => path.line_to((to.anchor.x, to.anchor.y)),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::{abgr, blend_mode, to_bezpath};
    use doc::{Contour, PathPoint};
    use geom::Point;
    use vello_cpu::kurbo::PathEl;
    use vello_cpu::peniko::Mix;

    fn point(x: f64, y: f64) -> PathPoint {
        PathPoint {
            anchor: Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        }
    }

    fn contour(closed: bool, points: Vec<PathPoint>) -> Contour {
        Contour {
            closed,
            point_count: points.len() as i64,
            points,
        }
    }

    #[test]
    fn abgr_decodes_channels_in_dictionary_layout() {
        // 0x20_40_80_FF: a=0x20, b=0x40, g=0x80, r=0xFF.
        let color = abgr(0x20_40_80_FF);
        let [r, g, b, a] = color.components;
        assert_eq!((r * 255.0).round() as u8, 255);
        assert_eq!((g * 255.0).round() as u8, 128);
        assert_eq!((b * 255.0).round() as u8, 64);
        assert_eq!((a * 255.0).round() as u8, 32);
    }

    #[test]
    fn blend_table_matches_dictionary_values() {
        assert_eq!(blend_mode(0), Mix::Normal);
        assert_eq!(blend_mode(2), Mix::Multiply);
        assert_eq!(blend_mode(3), Mix::Difference);
        assert_eq!(blend_mode(5), Mix::Screen);
        assert_eq!(blend_mode(9), Mix::Darken);
        assert_eq!(blend_mode(14), Mix::Luminosity);
        assert_eq!(blend_mode(37), Mix::ColorBurn);
        // Documented but not in the rasterizer's mix set: normal fallback.
        assert_eq!(blend_mode(7), Mix::Normal, "dissolve");
        assert_eq!(blend_mode(8), Mix::Normal, "linear burn");
        assert_eq!(blend_mode(18), Mix::Normal, "linear dodge");
        assert_eq!(blend_mode(25), Mix::Normal, "erase");
        assert_eq!(blend_mode(99), Mix::Normal, "undocumented");
    }

    #[test]
    fn closed_polygon_emits_closed_path() {
        let path = to_bezpath(&[contour(
            true,
            vec![
                point(0.0, 0.0),
                point(4.0, 0.0),
                point(4.0, 4.0),
                point(0.0, 4.0),
            ],
        )]);
        let elements: Vec<PathEl> = path.elements().to_vec();
        assert_eq!(
            elements,
            vec![
                PathEl::MoveTo((0.0, 0.0).into()),
                PathEl::LineTo((4.0, 0.0).into()),
                PathEl::LineTo((4.0, 4.0).into()),
                PathEl::LineTo((0.0, 4.0).into()),
                PathEl::LineTo((0.0, 0.0).into()),
                PathEl::ClosePath,
            ]
        );
    }

    #[test]
    fn open_path_stays_open() {
        let path = to_bezpath(&[contour(false, vec![point(0.0, 0.0), point(4.0, 0.0)])]);
        let elements: Vec<PathEl> = path.elements().to_vec();
        assert_eq!(
            elements,
            vec![
                PathEl::MoveTo((0.0, 0.0).into()),
                PathEl::LineTo((4.0, 0.0).into()),
            ]
        );
    }

    #[test]
    fn one_sided_controls_collapse_to_their_endpoint() {
        let mut from = point(0.0, 0.0);
        from.secondary = Some(Point::new(1.0, 0.0));
        let mut to = point(4.0, 0.0);
        to.primary = Some(Point::new(3.0, 0.0));
        let path = to_bezpath(&[contour(false, vec![from, to])]);
        let elements: Vec<PathEl> = path.elements().to_vec();
        assert_eq!(
            elements,
            vec![
                PathEl::MoveTo((0.0, 0.0).into()),
                PathEl::CurveTo((1.0, 0.0).into(), (3.0, 0.0).into(), (4.0, 0.0).into()),
            ]
        );
    }

    #[test]
    fn empty_contours_are_skipped() {
        let path = to_bezpath(&[contour(true, Vec::new())]);
        assert_eq!(path.elements().len(), 0);
    }
}
