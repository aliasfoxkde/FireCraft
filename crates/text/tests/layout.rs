//! Text layout tests: pinned geometry for a pinned font + string (the
//! M7 task-7 stopping condition), determinism, and every pinned
//! attribute mapping from the crate header.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use doc::{TextObject, TextRun, TextStyle};
use geom::{Affine, Rect};
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
