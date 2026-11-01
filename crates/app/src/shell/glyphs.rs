//! Tool labels, keys, and the hand-painted tool glyphs (original
//! art, never the Adobe icons).
//!
//! Pure egui drawing glue: exercised visually (and later by the
//! `egui_kittest` snapshot gate), not by line coverage.

use super::*;

/// The CS6 chrome look: near-black charcoal panels, square corners,
/// small type. Applied once at startup; `eww`-era Fireworks was dark
/// graphite against a mid-gray canvas.
///
/// Public because the web entry point builds the shell through
/// [`Shell::with_platform`] (which has no creation context), and a
/// web build that skipped this rendered egui's default light theme —
/// the single loudest difference from the CS6 look.
pub fn apply_cs6_theme(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        let visuals = &mut style.visuals;
        visuals.dark_mode = true;
        visuals.panel_fill = Color32::from_rgb(0x3c, 0x3c, 0x3c);
        visuals.window_fill = Color32::from_rgb(0x32, 0x32, 0x32);
        visuals.extreme_bg_color = Color32::from_rgb(0x28, 0x28, 0x28);
        visuals.faint_bg_color = Color32::from_rgb(0x44, 0x44, 0x44);
        // 0xdd against the darkest fill we put text on (faint 0x44)
        // is 7.2:1 — WCAG 2.1 AAA (≥7:1). 0xd9 was 6.9:1, a hair short.
        visuals.override_text_color = Some(Color32::from_rgb(0xdd, 0xdd, 0xdd));
        visuals.selection.bg_fill = Color32::from_rgb(0x2f, 0x5d, 0x8a);
        visuals.selection.stroke = (1.0, Color32::from_rgb(0x50, 0x7e, 0xab)).into();
        visuals.window_corner_radius = egui::CornerRadius::ZERO;
        visuals.menu_corner_radius = egui::CornerRadius::ZERO;
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = egui::CornerRadius::ZERO;
        }
        style.spacing.menu_margin = egui::Margin::same(2);
        style.spacing.button_padding = egui::Vec2::new(6.0, 2.0);
        style.spacing.item_spacing = egui::Vec2::new(6.0, 3.0);
    });
}

/// Display name for a tool (the CS6 tool names).
pub(crate) fn tool_label(tool: Tool) -> &'static str {
    match tool {
        Tool::Pointer => "Pointer",
        Tool::Subselect => "Subselection",
        Tool::Rect => "Rectangle",
        Tool::Ellipse => "Ellipse",
        Tool::Line => "Line",
        Tool::Text => "Text",
        Tool::Zoom => "Zoom",
        Tool::Hand => "Hand",
        Tool::Slice => "Rectangle Slice",
        Tool::Hotspot => "Web Hotspot",
        Tool::AutoShape(_) => "Auto Shape",
        Tool::Paintbrush => "Paintbrush",
        Tool::Pencil => "Pencil",
        Tool::Eraser => "Eraser",
        Tool::PaintBucket => "Paint Bucket",
        Tool::Gradient => "Gradient",
        Tool::Eyedropper => "Eyedropper",
        Tool::Marquee => "Marquee",
        Tool::EllipticalMarquee => "Elliptical Marquee",
        Tool::Lasso => "Lasso",
        Tool::PolygonalMarquee => "Polygonal Marquee",
        Tool::MagicWand => "Magic Wand",
    }
}

/// The CS6 one-key select for a tool ("" when unbound here).
pub(crate) fn tool_key(tool: Tool) -> &'static str {
    match tool {
        Tool::Pointer => "V",
        Tool::Rect => "R",
        Tool::Ellipse => "E",
        // CS6's keymap gives L to the Lasso tool; the Line tool has
        // no letter there, so FireCraft gives it O.
        Tool::Line => "O",
        Tool::Text => "T",
        Tool::Zoom => "Z",
        Tool::Hand => "H",
        Tool::Paintbrush => "B",
        Tool::Pencil => "N",
        Tool::Eraser => "Y",
        Tool::PaintBucket => "G",
        Tool::Gradient => "D",
        Tool::Eyedropper => "I",
        Tool::Marquee => "M",
        Tool::Lasso => "L",
        Tool::PolygonalMarquee => "P",
        Tool::MagicWand => "W",
        _ => "",
    }
}

/// Original glyph art for a tool, drawn from `Painter` primitives.
/// Silhouettes are `FireCraft`'s own — no Adobe icon bytes, no traces.
pub(crate) fn draw_tool_glyph(painter: &egui::Painter, rect: Rect, tool: Tool, color: Color32) {
    let c = rect.center();
    let stroke = (1.4, color);
    match tool {
        Tool::Pointer => {
            // filled head plus tail
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 4.0, c.y - 6.0),
                    Pos2::new(c.x + 2.5, c.y + 0.5),
                    Pos2::new(c.x - 1.5, c.y + 1.0),
                ],
                color,
                stroke,
            ));
            painter.line_segment(
                [
                    Pos2::new(c.x - 1.0, c.y + 2.0),
                    Pos2::new(c.x + 1.0, c.y + 6.0),
                ],
                stroke,
            );
        }
        Tool::Subselect => {
            // the same arrow, outline only
            let points = [
                Pos2::new(c.x - 4.0, c.y - 6.0),
                Pos2::new(c.x + 2.5, c.y + 0.5),
                Pos2::new(c.x - 1.5, c.y + 1.0),
                Pos2::new(c.x + 1.0, c.y + 6.0),
                Pos2::new(c.x - 1.5, c.y + 1.5),
                Pos2::new(c.x - 4.0, c.y + 3.5),
                Pos2::new(c.x - 4.0, c.y - 6.0),
            ];
            painter.add(egui::Shape::line(points.to_vec(), stroke));
        }
        Tool::Rect => {
            painter.rect_stroke(
                Rect::from_center_size(c, egui::Vec2::new(11.0, 9.0)),
                0.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Tool::Ellipse => {
            painter.circle_stroke(c, 5.5, stroke);
        }
        Tool::Line => {
            painter.line_segment(
                [
                    Pos2::new(c.x - 5.0, c.y + 5.0),
                    Pos2::new(c.x + 5.0, c.y - 5.0),
                ],
                stroke,
            );
        }
        Tool::Text => {
            painter.text(
                c,
                egui::Align2::CENTER_CENTER,
                "T",
                egui::FontId::proportional(13.0),
                color,
            );
        }
        Tool::Zoom => {
            painter.circle_stroke(Pos2::new(c.x - 1.5, c.y - 1.5), 4.0, stroke);
            painter.line_segment(
                [
                    Pos2::new(c.x + 1.5, c.y + 1.5),
                    Pos2::new(c.x + 6.0, c.y + 6.0),
                ],
                (2.0, color),
            );
        }
        Tool::Hand => {
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(c.x - 4.5, c.y - 1.5), egui::Vec2::new(9.0, 7.5)),
                3.0,
                color,
            );
            for (offset, height) in [(-3.0, 3.5), (0.0, 4.5), (3.0, 3.0)] {
                painter.line_segment(
                    [
                        Pos2::new(c.x + offset, c.y - 1.5),
                        Pos2::new(c.x + offset, c.y - 1.5 - height),
                    ],
                    stroke,
                );
            }
        }
        Tool::Slice => {
            painter.rect_stroke(
                Rect::from_center_size(c, egui::Vec2::splat(9.0)),
                0.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - 6.5, c.y - 3.0),
                    Pos2::new(c.x - 2.5, c.y - 3.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - 3.0, c.y - 6.5),
                    Pos2::new(c.x - 3.0, c.y - 2.5),
                ],
                stroke,
            );
        }
        Tool::Hotspot => {
            painter.circle_filled(c, 5.5, Color32::from_rgb(0x4f, 0x7c, 0xc0));
            painter.circle_stroke(c, 5.5, (1.0, Color32::from_rgb(0x9c, 0xb8, 0xe0)));
        }
        Tool::AutoShape(shape) => draw_autoshape_glyph(painter, rect, shape, color),
        Tool::Paintbrush => {
            // handle down-right, bristle tip up-left
            painter.line_segment(
                [
                    Pos2::new(c.x + 4.5, c.y + 4.5),
                    Pos2::new(c.x - 1.5, c.y - 1.5),
                ],
                (2.2, color),
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 5.5, c.y + 1.5),
                    Pos2::new(c.x - 1.5, c.y - 2.5),
                    Pos2::new(c.x + 1.5, c.y - 1.5),
                    Pos2::new(c.x - 2.5, c.y + 2.5),
                ],
                color,
                stroke,
            ));
        }
        Tool::Pencil => {
            painter.line_segment(
                [
                    Pos2::new(c.x + 5.0, c.y + 5.0),
                    Pos2::new(c.x - 2.0, c.y - 2.0),
                ],
                (2.2, color),
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 5.5, c.y - 1.5),
                    Pos2::new(c.x - 2.0, c.y - 5.0),
                    Pos2::new(c.x - 0.5, c.y - 2.5),
                    Pos2::new(c.x - 3.0, c.y + 1.0),
                ],
                color,
                stroke,
            ));
        }
        Tool::Eraser => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 5.5, c.y + 3.5),
                    Pos2::new(c.x - 1.5, c.y - 3.5),
                    Pos2::new(c.x + 5.5, c.y + 0.5),
                    Pos2::new(c.x + 2.0, c.y + 6.0),
                ],
                color,
                stroke,
            ));
        }
        Tool::PaintBucket => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 5.5, c.y - 3.5),
                    Pos2::new(c.x + 0.5, c.y - 3.5),
                    Pos2::new(c.x + 4.0, c.y + 4.0),
                    Pos2::new(c.x - 3.0, c.y + 4.0),
                ],
                color,
                stroke,
            ));
            painter.circle_filled(Pos2::new(c.x - 5.5, c.y - 4.5), 1.6, color);
        }
        Tool::Gradient => {
            // a ramp of bands from the ink color to the background
            let bands = 6;
            for band in 0..bands {
                let mix = band as f32 / (bands - 1) as f32;
                let shade = Color32::from_rgba_unmultiplied(
                    (color.r() as f32 * mix) as u8,
                    (color.g() as f32 * mix) as u8,
                    (color.b() as f32 * mix) as u8,
                    color.a(),
                );
                let x = c.x - 6.0 + band as f32 * 2.0;
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(x, c.y - 5.0), egui::Vec2::new(2.0, 10.0)),
                    0.0,
                    shade,
                );
            }
        }
        Tool::Eyedropper => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 4.5, c.y + 4.5),
                    Pos2::new(c.x - 1.0, c.y + 1.0),
                    Pos2::new(c.x + 1.5, c.y + 3.5),
                    Pos2::new(c.x - 2.0, c.y + 6.0),
                ],
                color,
                stroke,
            ));
            painter.line_segment(
                [
                    Pos2::new(c.x - 0.5, c.y + 1.5),
                    Pos2::new(c.x + 4.5, c.y - 3.5),
                ],
                (2.0, color),
            );
        }
        Tool::Marquee => {
            // the dashed rectangle of a marquee: two dashes per edge
            let frame = Rect::from_center_size(c, egui::Vec2::splat(11.0));
            let dashes = [
                [
                    Pos2::new(frame.left(), frame.top()),
                    Pos2::new(frame.center().x, frame.top()),
                ],
                [
                    Pos2::new(frame.center().x, frame.top()),
                    Pos2::new(frame.right(), frame.top()),
                ],
                [
                    Pos2::new(frame.right(), frame.top()),
                    Pos2::new(frame.right(), frame.center().y),
                ],
                [
                    Pos2::new(frame.right(), frame.center().y),
                    Pos2::new(frame.right(), frame.bottom()),
                ],
                [
                    Pos2::new(frame.right(), frame.bottom()),
                    Pos2::new(frame.center().x, frame.bottom()),
                ],
                [
                    Pos2::new(frame.center().x, frame.bottom()),
                    Pos2::new(frame.left(), frame.bottom()),
                ],
                [
                    Pos2::new(frame.left(), frame.bottom()),
                    Pos2::new(frame.left(), frame.center().y),
                ],
                [
                    Pos2::new(frame.left(), frame.center().y),
                    Pos2::new(frame.left(), frame.top()),
                ],
            ];
            for dash in dashes {
                painter.line_segment(dash, stroke);
            }
        }
        Tool::EllipticalMarquee => {
            // the same dashed idea, round
            painter.circle_stroke(c, 5.5, (1.0, color));
            painter.circle_stroke(c, 2.5, (1.0, color));
        }
        Tool::Lasso => {
            // a loop with a trailing tail
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 4.5, c.y - 1.0),
                    Pos2::new(c.x - 1.0, c.y - 4.5),
                    Pos2::new(c.x + 4.0, c.y - 2.0),
                    Pos2::new(c.x + 2.5, c.y + 3.0),
                    Pos2::new(c.x - 2.5, c.y + 3.5),
                ],
                color,
                stroke,
            ));
            painter.line_segment(
                [
                    Pos2::new(c.x + 2.5, c.y + 3.0),
                    Pos2::new(c.x + 5.5, c.y + 5.5),
                ],
                (1.2, color),
            );
        }
        Tool::PolygonalMarquee => {
            // a triangle of vertices with its handles
            let points = [
                Pos2::new(c.x - 5.0, c.y + 4.0),
                Pos2::new(c.x, c.y - 5.0),
                Pos2::new(c.x + 5.0, c.y + 4.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                Color32::TRANSPARENT,
                stroke,
            ));
            for point in points {
                painter.rect_filled(
                    Rect::from_center_size(point, egui::Vec2::splat(2.0)),
                    0.0,
                    color,
                );
            }
        }
        Tool::MagicWand => {
            // a wand with a spark at its tip
            painter.line_segment(
                [
                    Pos2::new(c.x - 4.5, c.y + 4.5),
                    Pos2::new(c.x + 2.5, c.y - 2.5),
                ],
                (2.0, color),
            );
            painter.circle_stroke(Pos2::new(c.x + 4.0, c.y - 4.0), 2.0, (1.0, color));
        }
    }
}

/// Preview glyphs for the Auto Shape flyout: each of the ten
/// generators' silhouette as a one-stroke vector sketch (original
/// art — see `docs/ATTRIBUTION.md`).
pub(crate) fn draw_autoshape_glyph(
    painter: &egui::Painter,
    rect: Rect,
    shape: AutoShape,
    color: Color32,
) {
    let c = rect.center();
    let stroke = (1.4, color);
    let loop_points = |radii: &[f64]| -> Vec<Pos2> {
        let mut points = Vec::with_capacity(radii.len() + 1);
        for (index, radius) in radii.iter().enumerate() {
            let angle = -std::f64::consts::FRAC_PI_2
                + index as f64 * std::f64::consts::TAU / radii.len() as f64;
            points.push(Pos2::new(
                c.x + (radius * angle.cos()) as f32,
                c.y + (radius * angle.sin()) as f32,
            ));
        }
        points.push(points[0]);
        points
    };
    match shape {
        AutoShape::Star => {
            painter.add(egui::Shape::line(
                loop_points(&[6.5, 3.0, 6.5, 3.0, 6.5, 3.0, 6.5, 3.0, 6.5, 3.0]),
                stroke,
            ));
        }
        AutoShape::Polygon => {
            painter.add(egui::Shape::line(
                loop_points(&[6.5, 6.5, 6.5, 6.5, 6.5, 6.5]),
                stroke,
            ));
        }
        AutoShape::Spiral => {
            let mut points = Vec::new();
            for step in 0..48 {
                let t = step as f64 / 47.0;
                let angle = t * 3.0 * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
                let radius = 1.0 + t * 5.5;
                points.push(Pos2::new(
                    c.x + (radius * angle.cos()) as f32,
                    c.y + (radius * angle.sin()) as f32,
                ));
            }
            painter.add(egui::Shape::line(points, stroke));
        }
        AutoShape::Pie => {
            let center = Pos2::new(c.x - 1.5, c.y + 1.5);
            let mut points = vec![center];
            for step in 0..9 {
                let angle =
                    std::f64::consts::FRAC_PI_2 - step as f64 * std::f64::consts::TAU * 0.75 / 8.0;
                points.push(Pos2::new(
                    center.x + (6.0 * angle.cos()) as f32,
                    center.y + (6.0 * angle.sin()) as f32,
                ));
            }
            points.push(center);
            painter.add(egui::Shape::line(points, stroke));
        }
        AutoShape::Doughnut => {
            painter.circle_stroke(c, 6.0, stroke);
            painter.circle_stroke(c, 2.5, stroke);
        }
        AutoShape::LShape => {
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - 5.0, c.y - 6.0),
                    Pos2::new(c.x - 5.0, c.y + 6.0),
                    Pos2::new(c.x + 6.0, c.y + 6.0),
                ],
                stroke,
            ));
        }
        AutoShape::Arrow => {
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - 6.0, c.y - 2.5),
                    Pos2::new(c.x + 0.5, c.y - 2.5),
                    Pos2::new(c.x + 0.5, c.y - 5.5),
                    Pos2::new(c.x + 6.0, c.y),
                    Pos2::new(c.x + 0.5, c.y + 5.5),
                    Pos2::new(c.x + 0.5, c.y + 2.5),
                    Pos2::new(c.x - 6.0, c.y + 2.5),
                    Pos2::new(c.x - 6.0, c.y - 2.5),
                ],
                stroke,
            ));
        }
        AutoShape::ArrowLine => {
            painter.line_segment(
                [
                    Pos2::new(c.x - 6.0, c.y + 4.0),
                    Pos2::new(c.x + 4.5, c.y - 3.5),
                ],
                stroke,
            );
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x + 6.0, c.y - 6.0),
                    Pos2::new(c.x + 4.5, c.y - 3.5),
                    Pos2::new(c.x + 1.0, c.y - 4.0),
                ],
                stroke,
            ));
        }
        AutoShape::Measure => {
            painter.line_segment(
                [Pos2::new(c.x - 6.0, c.y), Pos2::new(c.x + 6.0, c.y)],
                stroke,
            );
            for x in [-6.0, 6.0] {
                painter.line_segment(
                    [Pos2::new(c.x + x, c.y - 4.0), Pos2::new(c.x + x, c.y + 4.0)],
                    stroke,
                );
            }
        }
        AutoShape::ConnectorLine => {
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - 6.0, c.y - 5.0),
                    Pos2::new(c.x + 1.5, c.y - 5.0),
                    Pos2::new(c.x + 1.5, c.y + 5.0),
                    Pos2::new(c.x + 6.0, c.y + 5.0),
                ],
                stroke,
            ));
        }
    }
}

/// UI label for a symbol type (`MSYi`).
pub(crate) fn symbol_type_label(kind: &SymbolType) -> String {
    match kind {
        SymbolType::Graphic => "graphic".to_owned(),
        SymbolType::Button => "button".to_owned(),
        SymbolType::Animation => "animation".to_owned(),
        SymbolType::Other(raw) => format!("other ({raw})"),
    }
}
