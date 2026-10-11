//! Text layout tests: pinned geometry for a pinned font + string (the
//! M7 task-7 stopping condition), determinism, and every pinned
//! attribute mapping from the crate header.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use doc::{Contour, PathPoint, TextObject, TextOnPath, TextRun, TextStyle};
use geom::{Affine, Point, Rect};
use text::{TextEngine, TextGeometry};

/// The golden object: Liberation Sans, 24px, left-aligned, in a 200×40
/// frame at (10, 20).
fn pinned() -> TextObject {
    let mut object = TextObject::new(Rect::new(10.0, 20.0, 210.0, 60.0));
    object.style = Some(TextStyle {
        font: Some("Liberation Sans".into()),
        size: Some(24.0),
        justification: Some(0),
        ..TextStyle::default()
    });
    object.runs = vec![TextRun {
        text: "FireCraft".into(),
    }];
    object
}

fn with_style(style: TextStyle) -> TextObject {
    let mut object = pinned();
    object.style = Some(style);
    object
}

fn layout(object: &TextObject) -> TextGeometry {
    TextEngine::new().glyph_contours(object)
}

/// An open two-anchor rail from `from` to `to` — the simplest guide
/// path with a constant tangent.
fn rail(from: (f64, f64), to: (f64, f64)) -> Vec<Contour> {
    vec![Contour {
        closed: false,
        point_count: 2,
        points: vec![
            PathPoint {
                anchor: Point::new(from.0, from.1),
                primary: None,
                secondary: None,
                is_curve: false,
            },
            PathPoint {
                anchor: Point::new(to.0, to.1),
                primary: None,
                secondary: None,
                is_curve: false,
            },
        ],
    }]
}

/// The pinned text riding a guide path: 10px (so a short rail holds the
/// whole string), flow 2.
fn on_path(path: Vec<Contour>, alignment: Option<i64>, offset: f64) -> TextObject {
    let mut object = TextObject::new(Rect::new(10.0, 20.0, 30.0, 24.0));
    object.style = Some(TextStyle {
        font: Some("Liberation Sans".into()),
        size: Some(10.0),
        justification: Some(0),
        ..TextStyle::default()
    });
    object.runs = vec![TextRun {
        text: "FireCraft".into(),
    }];
    object.flow = Some(2);
    object.on_path = Some(TextOnPath {
        alignment,
        offset,
        path,
    });
    object
}

fn ink(geometry: &TextGeometry) -> Rect {
    geometry.ink.expect("on-path text must produce ink")
}

/// Pinned render: pinned font + pinned string → pinned geometry. These
/// values come from the bundled Liberation Sans outlines; any change
/// here is a layout-engine or font-asset change and must be reviewed.
#[test]
fn pinned_render_is_stable() {
    let geometry = layout(&pinned());
    assert_eq!(
        geometry.ink,
        Some(Rect::new(
            11.96875,
            25.329689025878906,
            103.171875,
            42.954689025878906,
        ))
    );
    assert_eq!(geometry.block, (93.33984375, 28.80000114440918));
    assert_eq!(geometry.contours.len(), 12);
    let first = &geometry.contours[0].points[0].anchor;
    assert_eq!((first.x, first.y), (14.203125, 28.032814025878906));
    let second = &geometry.contours[1].points[0].anchor;
    assert_eq!((second.x, second.y), (26.26953125, 27.345314025878906));
}

/// The determinism contract: two engines from scratch shape the same
/// string into identical geometry.
#[test]
fn two_engines_agree() {
    let a = TextEngine::new().glyph_contours(&pinned());
    let b = TextEngine::new().glyph_contours(&pinned());
    assert_eq!(a, b);
}

#[test]
fn contour_invariants_hold() {
    let geometry = layout(&pinned());
    for contour in &geometry.contours {
        assert_eq!(contour.point_count as usize, contour.points.len());
        assert!(contour.points.len() >= 2);
        for point in &contour.points {
            let curved = point.primary.is_some() || point.secondary.is_some();
            assert_eq!(point.is_curve, curved);
        }
    }
}

#[test]
fn empty_runs_produce_nothing() {
    let mut object = pinned();
    object.runs.clear();
    let geometry = layout(&object);
    assert!(geometry.contours.is_empty());
    assert_eq!(geometry.ink, None);
    assert_eq!(geometry.block, (0.0, 0.0));

    object.runs = vec![TextRun {
        text: "  \n\t".into(),
    }];
    let whitespace = layout(&object);
    assert!(whitespace.contours.is_empty());
    assert_eq!(whitespace.ink, None);
}

#[test]
fn long_text_wraps_at_frame_width() {
    let mut object = pinned();
    object.runs = vec![TextRun {
        text: "the quick brown fox jumps over the lazy dog".into(),
    }];
    let wrapped = layout(&object);
    // Three line boxes stack: the block is three line heights tall.
    assert!(wrapped.block.1 > 2.5 * 28.8);

    // The same text in a frame wide enough never wraps.
    object.frame = Rect::new(10.0, 20.0, 1410.0, 60.0);
    let single = layout(&object);
    assert!((single.block.1 - 28.80000114440918).abs() < 0.001);
}

#[test]
fn point_size_scales_ink() {
    let mut object = pinned();
    object.style.as_mut().unwrap().size = Some(48.0);
    let doubled = layout(&object);
    let base = layout(&pinned());
    let base_height = base.ink.unwrap().height();
    let doubled_height = doubled.ink.unwrap().height();
    assert!((doubled_height / base_height - 2.0).abs() < 0.01);
    assert!(doubled.block.0 > base.block.0 * 1.9);
}

#[test]
fn justification_moves_lines() {
    let left = layout(&pinned()).ink.unwrap();
    let centre = layout(&with_style(TextStyle {
        font: Some("Liberation Sans".into()),
        size: Some(24.0),
        justification: Some(1),
        ..TextStyle::default()
    }))
    .ink
    .unwrap();
    let right = layout(&with_style(TextStyle {
        font: Some("Liberation Sans".into()),
        size: Some(24.0),
        justification: Some(2),
        ..TextStyle::default()
    }))
    .ink
    .unwrap();
    // Same ink width, different placement inside the 200px frame: left
    // hugs the frame origin, right hugs the frame's right edge.
    let width = left.width();
    assert!((centre.width() - width).abs() < 0.001);
    assert!((right.width() - width).abs() < 0.001);
    assert!(centre.x0 > left.x0 + 40.0);
    assert!(right.x1 > left.x1 + 80.0);
    assert!((right.x1 - 210.0).abs() < 1.0);
}

#[test]
fn underline_adds_one_rule_per_line() {
    let plain = layout(&pinned());
    let underlined = layout(&with_style(TextStyle {
        font: Some("Liberation Sans".into()),
        size: Some(24.0),
        underline: Some(true),
        ..TextStyle::default()
    }));
    assert_eq!(underlined.contours.len(), plain.contours.len() + 1);
    // The rule is the one wide 4-point closed contour — glyph parts can
    // be 4-point too (the 'i' dot), but nothing glyph-sized spans the
    // laid-out width. Every glyph contour is closed as well.
    let rule = underlined
        .contours
        .iter()
        .find(|contour| {
            contour.points.len() == 4
                && contour.points[2].anchor.x - contour.points[0].anchor.x > plain.block.0 / 2.0
        })
        .expect("underline rectangle");
    assert!(rule.closed);
    let y0 = rule
        .points
        .iter()
        .map(|p| p.anchor.y)
        .fold(f64::INFINITY, f64::min);
    assert!(y0 >= plain.ink.unwrap().y1 - 0.001);
    // And it spans the laid-out width.
    let x0 = rule
        .points
        .iter()
        .map(|p| p.anchor.x)
        .fold(f64::INFINITY, f64::min);
    let x1 = rule
        .points
        .iter()
        .map(|p| p.anchor.x)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((x1 - x0 - plain.block.0).abs() < 0.001);
}

#[test]
fn bold_changes_ink() {
    let base = layout(&pinned()).block.0;
    let bold = layout(&with_style(TextStyle {
        font: Some("Liberation Sans".into()),
        size: Some(24.0),
        bold: Some(true),
        ..TextStyle::default()
    }))
    .block
    .0;
    assert!(bold > base * 1.02);
}

#[test]
fn tracking_spreads_glyphs() {
    let base = layout(&pinned());
    let tracked = layout(&with_style(TextStyle {
        font: Some("Liberation Sans".into()),
        size: Some(24.0),
        kerning: Some(1000.0),
        ..TextStyle::default()
    }));
    // RKNf 1000 = one em per glyph index: the block grows by 8em on a
    // 9-glyph string while the first glyph stays put.
    let size = 24.0;
    assert!((tracked.block.0 - base.block.0 - 8.0 * size).abs() < 0.001);
    let base_first = &base.contours[0].points[0].anchor;
    let tracked_first = &tracked.contours[0].points[0].anchor;
    assert_eq!(base_first.x, tracked_first.x);
    // And the shift accumulates: the last glyph sits 8em further right
    // than in the untracked layout.
    let base_last = base.contours.last().unwrap().points[0].anchor.x;
    let tracked_last = tracked.contours.last().unwrap().points[0].anchor.x;
    assert!((tracked_last - base_last - 8.0 * size).abs() < 0.5);
}

#[test]
fn horizontal_scale_doubles_width() {
    let base = layout(&pinned()).block.0;
    let scaled = layout(&with_style(TextStyle {
        font: Some("Liberation Sans".into()),
        size: Some(24.0),
        horizontal_scale: Some(200.0),
        ..TextStyle::default()
    }))
    .block
    .0;
    assert!((scaled - 2.0 * base).abs() < 0.001);
}

#[test]
fn leading_pixel_unit_sets_line_height() {
    // LDMi 1 = pixels: LEDf 40 → a 40px line box.
    let geometry = layout(&with_style(TextStyle {
        font: Some("Liberation Sans".into()),
        size: Some(24.0),
        leading: Some(40.0),
        leading_unit: Some(1),
        ..TextStyle::default()
    }));
    assert!((geometry.block.1 - 40.0).abs() < 0.001);
}

#[test]
fn baseline_shift_raises_ink() {
    let base = layout(&pinned()).ink.unwrap();
    let raised = layout(&with_style(TextStyle {
        font: Some("Liberation Sans".into()),
        size: Some(24.0),
        baseline_shift: Some(10.0),
        ..TextStyle::default()
    }))
    .ink
    .unwrap();
    assert!((base.y0 - raised.y0 - 10.0).abs() < 0.001);
    assert!((base.y1 - raised.y1 - 10.0).abs() < 0.001);
    assert!((base.x0 - raised.x0).abs() < 0.001);
}

#[test]
fn unknown_family_substitutes_the_bundled_sans() {
    let mut object = pinned();
    object.style.as_mut().unwrap().font = Some("Arial Black".into());
    assert_eq!(layout(&object), layout(&pinned()));
}

#[test]
fn missing_style_uses_defaults() {
    let mut object = pinned();
    object.style = None;
    // No style → 12px over the bundled sans, laid out at the frame
    // origin. Nothing here may panic; the geometry just gets smaller.
    let geometry = layout(&object);
    let ink = geometry.ink.expect("default layout produces ink");
    assert!(ink.x0 >= object.frame.x0);
    assert!(geometry.block.0 > 0.0);
}

#[test]
fn transform_moves_geometry() {
    let transform = Affine::translate((100.0, -25.0));
    let mut object = pinned();
    object.transform = Some(transform);
    let moved = layout(&object).ink.unwrap();
    let base = layout(&pinned()).ink.unwrap();
    assert!((moved.x0 - base.x0 - 100.0).abs() < 0.001);
    assert!((moved.y0 - base.y0 + 25.0).abs() < 0.001);
}

#[test]
fn flows_two_and_three_fall_back_to_normal_flow() {
    for flow in [2, 3] {
        let mut object = pinned();
        object.flow = Some(flow);
        let geometry = layout(&object);
        assert_eq!(geometry, layout(&pinned()));
    }
}

#[test]
fn flow_two_without_a_path_still_falls_back() {
    let mut object = on_path(Vec::new(), Some(0), 0.0);
    object.flow = Some(2);
    let mut linear = on_path(Vec::new(), Some(0), 0.0);
    linear.flow = Some(0);
    linear.on_path = None;
    assert_eq!(layout(&object), layout(&linear));
}

/// A horizontal rail lays text exactly like rotate and upright agree —
/// both bases are the identity at tangent (1, 0).
#[test]
fn on_horizontal_rail_rotate_and_upright_agree() {
    let path = rail((10.0, 40.0), (210.0, 40.0));
    assert_eq!(
        layout(&on_path(path.clone(), Some(0), 0.0)),
        layout(&on_path(path, Some(1), 0.0))
    );
}

/// On-path text ignores the frame: a 30px-wide frame would wrap the
/// plain flow into many lines, but the rail holds the whole string on
/// one baseline.
#[test]
fn on_path_ignores_the_frame_width() {
    // The same text in plain flow inside the same tiny frame wraps.
    let mut wrapped = on_path(Vec::new(), Some(0), 0.0);
    wrapped.flow = Some(0);
    wrapped.on_path = None;
    assert!(layout(&wrapped).block.1 > 2.0 * 12.0, "plain flow wraps");

    let geometry = layout(&on_path(rail((10.0, 40.0), (210.0, 40.0)), Some(0), 0.0));
    let bounds = ink(&geometry);
    assert!(bounds.height() < 10.0, "one line expected, got {bounds:?}");
}

/// The baseline rides the rail: ink bottoms sit at the rail's y.
#[test]
fn baseline_rides_the_rail() {
    let geometry = layout(&on_path(rail((10.0, 40.0), (210.0, 40.0)), Some(0), 0.0));
    let bounds = ink(&geometry);
    // "FireCraft" has no descenders: ink ends within ~1.5px of the rail.
    assert!((bounds.y1 - 40.0).abs() < 1.5, "{bounds:?}");
    assert!(bounds.y0 > 30.0, "caps rise above the rail: {bounds:?}");
}

/// `CHOf` starts the text `offset` pixels along the rail.
#[test]
fn character_offset_starts_farther_along() {
    let at_zero = ink(&layout(&on_path(
        rail((0.0, 40.0), (210.0, 40.0)),
        Some(0),
        0.0,
    )));
    let at_thirty = ink(&layout(&on_path(
        rail((0.0, 40.0), (210.0, 40.0)),
        Some(0),
        30.0,
    )));
    assert!((at_thirty.x0 - at_zero.x0 - 30.0).abs() < 0.75);
}

/// A vertical rail + rotate runs the text downward: the ink is tall and
/// narrow, and starts at the rail's x.
#[test]
fn rotate_follows_a_vertical_rail_downward() {
    let geometry = layout(&on_path(rail((100.0, 10.0), (100.0, 210.0)), Some(0), 0.0));
    let bounds = ink(&geometry);
    assert!(bounds.height() > bounds.width() * 3.0, "{bounds:?}");
    // First glyph's baseline starts at the rail: the rotated F's stem
    // hangs left of the rail, its caps reach right — the box straddles
    // x=100 within a cap height.
    assert!(bounds.x0 > 100.0 - 8.0 && bounds.x0 < 100.0, "{bounds:?}");
}

/// Upright on a vertical rail stacks unrotated glyphs to the right of
/// the rail (left side bearing, ascent above each baseline), while the
/// rotated case straddles it — caps reach left of the rail.
#[test]
fn upright_on_a_vertical_rail_stays_unrotated() {
    let rotated = ink(&layout(&on_path(
        rail((100.0, 10.0), (100.0, 210.0)),
        Some(0),
        0.0,
    )));
    let upright = ink(&layout(&on_path(
        rail((100.0, 10.0), (100.0, 210.0)),
        Some(1),
        0.0,
    )));
    assert!(upright.x0 > 100.0, "upright hangs right: {upright:?}");
    assert!(rotated.x0 < 100.0, "rotated straddles: {rotated:?}");
    // Upright glyphs stack within a line height of the rail.
    assert!(upright.width() < 12.0, "{upright:?}");
}

/// Glyphs beyond the rail's end are dropped: a 12px rail holds at most
/// a couple of glyphs of the pinned string.
#[test]
fn glyphs_past_the_rail_end_are_dropped() {
    let full = layout(&on_path(rail((0.0, 40.0), (210.0, 40.0)), Some(0), 0.0));
    let short = layout(&on_path(rail((0.0, 40.0), (12.0, 40.0)), Some(0), 0.0));
    assert!(short.contours.len() < full.contours.len());
    // block.0 reports the rail length, not the laid-out width.
    assert!((short.block.0 - 12.0).abs() < 0.001);
    let bounds = ink(&short);
    assert!(
        bounds.x1 < 12.0 + 8.0,
        "overhang limited to one glyph: {bounds:?}"
    );
}

/// Negative offsets land before the rail's start — those glyphs drop
/// too.
#[test]
fn glyphs_before_the_rail_start_are_dropped() {
    let shifted = layout(&on_path(rail((0.0, 40.0), (110.0, 40.0)), Some(0), -6.0));
    let at_zero = layout(&on_path(rail((0.0, 40.0), (110.0, 40.0)), Some(0), 0.0));
    assert!(shifted.contours.len() < at_zero.contours.len());
}

/// A second line continues along the same rail past the first line's
/// extent (the recorded multi-line mapping).
#[test]
fn second_line_continues_along_the_rail() {
    let mut object = on_path(rail((0.0, 40.0), (410.0, 40.0)), Some(0), 0.0);
    object.runs = vec![TextRun {
        text: "FireCraft\nFireCraft".into(),
    }];
    let wrapped = ink(&layout(&object));
    let single = ink(&layout(&on_path(
        rail((0.0, 40.0), (410.0, 40.0)),
        Some(0),
        0.0,
    )));
    assert!(
        wrapped.x1 > single.x1 + 5.0,
        "second line must land past the first: {wrapped:?} vs {single:?}"
    );
}

/// Skew modes agree with rotate on a 45° rail in kind, not in bytes:
/// each preserves a different axis, so the ink boxes differ.
#[test]
fn skew_modes_differ_from_rotation_on_a_diagonal() {
    let diagonal = rail((0.0, 0.0), (100.0, 100.0));
    let rotate = ink(&layout(&on_path(diagonal.clone(), Some(0), 0.0)));
    let skew_v = ink(&layout(&on_path(diagonal.clone(), Some(2), 0.0)));
    let skew_h = ink(&layout(&on_path(diagonal.clone(), Some(3), 0.0)));
    assert!(
        (rotate.width() - skew_v.width()).abs() > 0.5,
        "{rotate:?} vs {skew_v:?}"
    );
    assert!(
        (rotate.height() - skew_h.height()).abs() > 0.5,
        "{rotate:?} vs {skew_h:?}"
    );
    // All three produce real ink (the bases are invertible at 45°).
    assert!(skew_v.width() > 20.0 && skew_h.height() > 5.0);
}

/// On-path layout is deterministic across engines, like linear flow.
#[test]
fn on_path_two_engines_agree() {
    let object = on_path(rail((10.0, 40.0), (110.0, 140.0)), Some(0), 5.0);
    assert_eq!(layout(&object), layout(&object));
}
