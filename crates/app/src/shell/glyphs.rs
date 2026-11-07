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
    // The tool-strip icons are embedded Lucide SVGs; the loader
    // rasterizes them at the exact on-screen size. Idempotent.
    egui_extras::install_image_loaders(ctx);
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
///
/// The names live in [`crate::toolstrip`], next to the strip's order
/// and the groups they belong to.
pub(crate) fn tool_label(tool: Tool) -> &'static str {
    match tool {
        Tool::AutoShape(shape) => shape.label(),
        other => crate::toolstrip::label_of(other),
    }
}

/// The CS6 one-key select for a tool ("" when unbound here).
///
/// CS6 gives a tool *group* one shortcut and cycles through the group
/// on repeated presses, so every member of a group carries that
/// group's key.
pub(crate) fn tool_key(tool: Tool) -> &'static str {
    crate::toolstrip::key_of(tool)
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
