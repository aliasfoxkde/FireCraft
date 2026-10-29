//! The egui face of the editor: menus, panels, canvas. Thin by rule —
//! every document change you see here is an [`Editor::apply`] call,
//! and every panel reads the document read-only. Desktop file routing
//! goes through the injected [`engine::platform`] traits (`EnvDialogs`
//! below; real modal dialogs land with desktop integration).

use doc::{ObjectData, PathObject, SymbolInstanceObject, SymbolType, TextObject};
use egui::{Color32, ColorImage, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2};

use base64::Engine as _;

use crate::canvas::View;
use crate::editor::{
    AutoShape, DragConstraint, Editor, NineSliceEdge, PixelSelectMode, StampShape, Tool,
};
use crate::menus;

/// The CS6 chrome look: near-black charcoal panels, square corners,
/// small type. Applied once at startup; `eww`-era Fireworks was dark
/// graphite against a mid-gray canvas.
fn apply_cs6_theme(ctx: &egui::Context) {
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
fn tool_label(tool: Tool) -> &'static str {
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
fn tool_key(tool: Tool) -> &'static str {
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
fn draw_tool_glyph(painter: &egui::Painter, rect: Rect, tool: Tool, color: Color32) {
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
fn draw_autoshape_glyph(painter: &egui::Painter, rect: Rect, shape: AutoShape, color: Color32) {
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
fn symbol_type_label(kind: &SymbolType) -> String {
    match kind {
        SymbolType::Graphic => "graphic".to_owned(),
        SymbolType::Button => "button".to_owned(),
        SymbolType::Animation => "animation".to_owned(),
        SymbolType::Other(raw) => format!("other ({raw})"),
    }
}

/// One quick-add preset of the Live Filters panel: the engine's
/// internal name — the registry id the Filters menu and this panel
/// share — the CS6 display label, and the `DCVs` defaults that give
/// each kernel its working shape (visible where a visible default
/// exists; `levels` ships its identity master so the rows are
/// editable from a known state, and the shadow family ships the CS6
/// defaults — the same four scalars its Inner Shadow preset
/// documents — because the kernel reads all four or renders nothing).
#[derive(Clone, Copy)]
pub(crate) struct FilterPreset {
    /// Engine internal name (`filter.add`'s `name`).
    pub name: &'static str,
    /// CS6 display label.
    pub label: &'static str,
    /// `DCVs` defaults: (key, decimal-text value).
    pub params: &'static [(&'static str, &'static str)],
    /// Curve knots for the `*_points` param — the engine's raw
    /// `GPLv` encoding; `value` must stay empty when these ride along.
    pub points: &'static [(f64, f64)],
}

/// The panel's quick-add registry, grouped the way the enumeration
/// table groups it (`docs/research/fw-code-notes.md`, M14 task 1).
/// CS6 builds its Filters menu at runtime from the same registry —
/// the static menu resource carries only Repeat Filter — which is
/// exactly the mirror `menus::FILTERS` casts.
pub(crate) static FILTER_FAMILIES: &[(&str, &[FilterPreset])] = &[
    (
        "Adjust Color",
        &[
            FilterPreset {
                name: "invert",
                label: "Invert",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "brightnessContrast",
                label: "Brightness/Contrast",
                params: &[("brightness_amount", "10"), ("contrast_amount", "10")],
                points: &[],
            },
            FilterPreset {
                name: "levels",
                label: "Levels",
                params: &[
                    ("source_low_rgb", "0"),
                    ("source_high_rgb", "255"),
                    ("gamma_rgb", "1"),
                    ("dest_low_rgb", "0"),
                    ("dest_high_rgb", "255"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "curves",
                label: "Curves",
                params: &[("rgb_points", "")],
                points: &[(0.0, 0.0), (128.0, 64.0), (255.0, 255.0)],
            },
            FilterPreset {
                name: "hueSaturation",
                label: "Hue/Saturation",
                params: &[
                    ("hue_amount", "15"),
                    ("saturation_amount", "10"),
                    ("lightness_amount", "0"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "autoLevels",
                label: "Auto Levels",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "colorFill",
                label: "Color Fill",
                params: &[("Color", "#336699"), ("Opacity", "100"), ("Blendmode", "0")],
                points: &[],
            },
            FilterPreset {
                name: "convertToAlpha",
                label: "Convert to Alpha",
                params: &[],
                points: &[],
            },
        ],
    ),
    (
        "Blur",
        &[
            FilterPreset {
                name: "blur",
                label: "Blur",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "blurMore",
                label: "Blur More",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "gaussianBlur",
                label: "Gaussian Blur",
                params: &[("gaussian_blur_radius", "2")],
                points: &[],
            },
            FilterPreset {
                name: "motionBlur",
                label: "Motion Blur",
                params: &[("motion_blur_angle", "0"), ("motion_blur_distance", "4")],
                points: &[],
            },
            FilterPreset {
                name: "radialBlur",
                label: "Radial Blur",
                params: &[("radial_blur_amount", "10"), ("radial_blur_quality", "3")],
                points: &[],
            },
            FilterPreset {
                name: "zoomBlur",
                label: "Zoom Blur",
                params: &[("zoom_blur_amount", "10"), ("zoom_blur_quality", "3")],
                points: &[],
            },
        ],
    ),
    (
        "Sharpen",
        &[
            FilterPreset {
                name: "sharpen",
                label: "Sharpen",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "sharpenMore",
                label: "Sharpen More",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "unsharpMask",
                label: "Unsharp Mask",
                params: &[
                    ("unsharp_mask_radius", "2"),
                    ("unsharp_mask_amount", "50"),
                    ("unsharp_mask_threshold", "0"),
                ],
                points: &[],
            },
        ],
    ),
    (
        "Stylize",
        &[
            FilterPreset {
                name: "addNoise",
                label: "Add Noise",
                params: &[("add_noise_amount", "10"), ("add_noise_use_color", "0")],
                points: &[],
            },
            FilterPreset {
                name: "findEdges",
                label: "Find Edges",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "clouds",
                label: "Clouds",
                params: &[("sky_color", "#0000FF"), ("cloud_color", "#FFFFFF")],
                points: &[],
            },
            FilterPreset {
                name: "differenceClouds",
                label: "Difference Clouds",
                params: &[("sky_color", "#0000FF"), ("cloud_color", "#FFFFFF")],
                points: &[],
            },
        ],
    ),
    (
        "Shadow and Glow",
        &[
            FilterPreset {
                name: "dropShadow",
                label: "Drop Shadow",
                params: &[
                    ("ShadowColor", "#000000"),
                    ("ShadowAngle", "135"),
                    ("ShadowBlur", "2"),
                    ("ShadowDistance", "3"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "innerShadow",
                label: "Inner Shadow",
                params: &[
                    ("ShadowColor", "#000000"),
                    ("ShadowAngle", "135"),
                    ("ShadowBlur", "2"),
                    ("ShadowDistance", "3"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "solidShadow",
                label: "Solid Shadow",
                params: &[("ShadowColor", "#804000"), ("ShowObject", "1")],
                points: &[],
            },
            FilterPreset {
                name: "innerGlow",
                label: "Inner Glow",
                params: &[("ShadowColor", "#ffcc00"), ("GlowWidth", "3")],
                points: &[],
            },
            FilterPreset {
                name: "glow",
                label: "Glow",
                params: &[
                    ("BevelType", "4"),
                    ("GlowWidth", "5"),
                    ("LightAngle", "120"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#804000"),
                ],
                points: &[],
            },
        ],
    ),
    (
        "Bevel and Emboss",
        &[
            FilterPreset {
                name: "innerBevel",
                label: "Inner Bevel",
                params: &[
                    ("BevelType", "0"),
                    ("BevelWidth", "6"),
                    ("BevelContrast", "70"),
                    ("LightAngle", "120"),
                    ("LightDistance", "150"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#808000"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "outerBevel",
                label: "Outer Bevel",
                params: &[
                    ("BevelType", "1"),
                    ("BevelWidth", "6"),
                    ("BevelContrast", "70"),
                    ("LightAngle", "120"),
                    ("LightDistance", "150"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#808000"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "emboss",
                label: "Raised Emboss",
                params: &[
                    ("BevelType", "2"),
                    ("BevelWidth", "6"),
                    ("BevelContrast", "70"),
                    ("LightAngle", "120"),
                    ("LightDistance", "150"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#808000"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "pillowEmboss",
                label: "Inset Emboss",
                params: &[
                    ("BevelType", "3"),
                    ("BevelWidth", "6"),
                    ("BevelContrast", "70"),
                    ("LightAngle", "120"),
                    ("LightDistance", "150"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#808000"),
                ],
                points: &[],
            },
        ],
    ),
];

/// Numeric edit-row metadata for applied effects: the documented
/// range (the enumeration table) plus the neutral value the row's
/// reset button restores. Keys absent here render as text rows; the
/// boolean keys render as checkboxes and `*_points` as knot lists.
struct FilterParamMeta {
    key: &'static str,
    /// Row label.
    label: &'static str,
    min: f64,
    max: f64,
    /// The reset value (the no-op where one exists).
    default: f64,
}

const FILTER_PARAMS: &[FilterParamMeta] = &[
    FilterParamMeta {
        key: "brightness_amount",
        label: "Brightness",
        min: -100.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "contrast_amount",
        label: "Contrast",
        min: -100.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "source_low_rgb",
        label: "In black",
        min: 0.0,
        max: 255.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "source_high_rgb",
        label: "In white",
        min: 0.0,
        max: 255.0,
        default: 255.0,
    },
    FilterParamMeta {
        key: "gamma_rgb",
        label: "Gamma",
        min: 0.1,
        max: 10.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "dest_low_rgb",
        label: "Out black",
        min: 0.0,
        max: 255.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "dest_high_rgb",
        label: "Out white",
        min: 0.0,
        max: 255.0,
        default: 255.0,
    },
    // In colorize mode the hue is absolute 0..360 — the row clamps to
    // the shift range; flipping the mode is one setParam away.
    FilterParamMeta {
        key: "hue_amount",
        label: "Hue",
        min: -180.0,
        max: 180.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "saturation_amount",
        label: "Saturation",
        min: -100.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "lightness_amount",
        label: "Lightness",
        min: -100.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "gaussian_blur_radius",
        label: "Radius",
        min: 0.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "motion_blur_angle",
        label: "Angle",
        min: 0.0,
        max: 360.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "motion_blur_distance",
        label: "Distance",
        min: 1.0,
        max: 100.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "radial_blur_amount",
        label: "Amount",
        min: 1.0,
        max: 100.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "radial_blur_quality",
        label: "Quality",
        min: 1.0,
        max: 100.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "zoom_blur_amount",
        label: "Amount",
        min: 1.0,
        max: 100.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "zoom_blur_quality",
        label: "Quality",
        min: 1.0,
        max: 100.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "unsharp_mask_radius",
        label: "Radius",
        min: 0.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "unsharp_mask_amount",
        label: "Amount",
        min: 0.0,
        max: 200.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "unsharp_mask_threshold",
        label: "Threshold",
        min: 0.0,
        max: 255.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "add_noise_amount",
        label: "Amount",
        min: 0.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "Opacity",
        label: "Opacity",
        min: 0.0,
        max: 100.0,
        default: 100.0,
    },
    FilterParamMeta {
        key: "ShadowAngle",
        label: "Angle",
        min: 0.0,
        max: 360.0,
        default: 135.0,
    },
    FilterParamMeta {
        key: "ShadowBlur",
        label: "Blur",
        min: 0.0,
        max: 250.0,
        default: 2.0,
    },
    FilterParamMeta {
        key: "ShadowDistance",
        label: "Distance",
        min: 0.0,
        max: 250.0,
        default: 3.0,
    },
    FilterParamMeta {
        key: "GlowWidth",
        label: "Width",
        min: 0.0,
        max: 35.0,
        default: 3.0,
    },
    FilterParamMeta {
        key: "GlowStartDistance",
        label: "Start",
        min: 0.0,
        max: 35.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "EdgeThreshold",
        label: "Edge threshold",
        min: 0.0,
        max: 1.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "BevelType",
        label: "Type",
        min: 0.0,
        max: 4.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "BevelWidth",
        label: "Width",
        min: 0.0,
        max: 35.0,
        default: 6.0,
    },
    FilterParamMeta {
        key: "BevelContrast",
        label: "Contrast",
        min: 0.0,
        max: 100.0,
        default: 70.0,
    },
    FilterParamMeta {
        key: "LightAngle",
        label: "Light angle",
        min: 0.0,
        max: 360.0,
        default: 120.0,
    },
    FilterParamMeta {
        key: "LightDistance",
        label: "Depth",
        min: 1.0,
        max: 200.0,
        default: 100.0,
    },
];

/// Checkbox keys (truthy = not "0"/"false", the compose read rule).
const FILTER_BOOL_KEYS: &[&str] = &["add_noise_use_color", "ShowObject"];

/// The preset for an internal name — the shared lookup behind the
/// panel's quick-add and the Filters menu's `Action::Filter`.
pub(crate) fn filter_preset(name: &str) -> Option<&'static FilterPreset> {
    FILTER_FAMILIES
        .iter()
        .flat_map(|(_, presets)| presets.iter())
        .find(|preset| preset.name == name)
}

/// The display quartet plus name, captured read-only from any
/// property-bearing kind.
struct DisplayState {
    opacity: i64,
    blend: i64,
    visible: bool,
    locked: bool,
    name: Option<String>,
}

/// One Web Layer panel row: id, display name, is-foreground-slice,
/// link, alt text, and the raw `TSLi` behind the foreground toggle.
type WebRow = (
    doc::ObjectId,
    String,
    bool,
    Option<String>,
    Option<String>,
    i64,
);

/// Desktop file routing for v1: the open/save path comes from
/// `FIRECRAFT_DOC` (default `untitled.fw.png` in the working
/// directory), messages go to stderr. No dialog library yet — the
/// trait boundary is what matters: the shell asks, the platform
/// answers, and the wasm build swaps this for browser file I/O.
pub struct EnvDialogs {
    /// The document path this build routes through.
    pub path: String,
}

impl Default for EnvDialogs {
    fn default() -> Self {
        Self {
            path: std::env::var("FIRECRAFT_DOC").unwrap_or_else(|_| "untitled.fw.png".to_owned()),
        }
    }
}

impl engine::platform::Dialogs for EnvDialogs {
    fn open_path(&self, _title: &str, _filters: &[(String, Vec<String>)]) -> Option<String> {
        Some(self.path.clone())
    }

    fn save_path(
        &self,
        _title: &str,
        _suggested_name: &str,
        _filters: &[(String, Vec<String>)],
    ) -> Option<String> {
        Some(self.path.clone())
    }

    fn message(&self, title: &str, text: &str) {
        eprintln!("[firecraft] {title}: {text}");
    }
}

/// The export formats the dialog offers, in menu order. The index is
/// the document's `FMTi` export-format id, so new formats append —
/// reordering would re-point every saved document's setting.
const EXPORT_FORMATS: &[&str] = &[
    "png", "jpg", "bmp", "tiff", "wbmp", "svg", "psd", "gif", "webp",
];

/// The web export workflows the dialog offers (the M11 set), in menu
/// order — the same names the CLI's `--workflow` flag takes.
const WEB_WORKFLOWS: &[&str] = &[
    "images only",
    "HTML + images",
    "CSS + images",
    "CSS sprites",
    "CSS properties",
    "pages to files",
    "states to files",
    "layers to files",
    "states to GIF",
];

/// The [`webexport::Workflow`] behind a dialog index — the two lists
/// move together.
fn web_workflow(index: usize) -> Option<webexport::Workflow> {
    if index >= WEB_WORKFLOWS.len() {
        return None;
    }
    Some(match index {
        0 => webexport::Workflow::ImagesOnly,
        1 => webexport::Workflow::HtmlAndImages,
        2 => webexport::Workflow::CssAndImages,
        3 => webexport::Workflow::CssSprites,
        4 => webexport::Workflow::CssProperties,
        5 => webexport::Workflow::PagesToFiles,
        6 => webexport::Workflow::StatesToFiles,
        7 => webexport::Workflow::LayersToFiles,
        _ => webexport::Workflow::StatesToGif,
    })
}

/// Export-dialog state (File → Export…): format, per-format controls,
/// and the cached 2-Up preview. The controls mirror the `EXPv` fields
/// the model maps; Export stamps them onto a cloned document and runs
/// the same codec path the CLI does. A web workflow replaces the
/// raster format entirely — the dialog then drives
/// [`app_cli::ops::export_web`], the same entry point the CLI verb
/// shares.
struct ExportDialog {
    /// Index into [`EXPORT_FORMATS`].
    format: usize,
    /// Selected web workflow, or `None` for the raster formats.
    web: Option<usize>,
    /// Emit the 1px shim row/column (web table workflows).
    web_shim: bool,
    /// Give areas without slices their own image (`include_undefined`).
    web_undefined: bool,
    /// JPEG quality 1..=100.
    quality: i32,
    /// Progressive JPEG scans.
    progressive: bool,
    /// Matte for alpha-less formats: false = white, true = black.
    matte_black: bool,
    /// GIF loop: 0 infinite, 1 never, 2 repeat n times.
    gif_loop: usize,
    /// Repeat count when `gif_loop == 2`.
    gif_times: i32,
    /// GIF per-frame ink crop (`AACb`).
    gif_crop: bool,
    /// GIF diff frames (`AADb`).
    gif_diff: bool,
    /// GIF Floyd–Steinberg amount (`PLDi`), percent.
    gif_dither: i32,
    /// Cached 2-Up panes and the settings stamp they were made from.
    preview: Option<(u64, TextureHandle, String, TextureHandle, String)>,
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self {
            format: 0,
            web: None,
            web_shim: true,
            web_undefined: true,
            quality: 90,
            progressive: false,
            matte_black: false,
            gif_loop: 0,
            gif_times: 3,
            gif_crop: false,
            gif_diff: false,
            gif_dither: 0,
            preview: None,
        }
    }
}

/// The eframe app. Holds the editor core plus everything window-shaped.
/// Platform services are trait objects: the desktop build injects
/// [`EnvDialogs`] + `StdFs`, the wasm build (crate `firecraft-web`)
/// injects browser implementations without touching this code.
pub struct Shell {
    /// The editor core (document, tools, selection).
    pub editor: Editor,
    /// Canvas view (pan/zoom).
    pub view: View,
    /// Cached page raster + the document stamp it was made from.
    raster: Option<(TextureHandle, (usize, usize, usize))>,
    /// Injected platform services.
    fs: Box<dyn engine::platform::FileSystem>,
    dialogs: Box<dyn engine::platform::Dialogs>,
    /// Last status line (errors, save confirmations).
    pub status: String,
    /// Open export dialog, if any.
    export_dialog: Option<ExportDialog>,
    /// Display name of the open document (tab strip).
    doc_name: String,
    /// Last canvas widget rect — the anchor for view commands that
    /// arrive outside the canvas (menu/shortcut zoom and fit).
    canvas_rect: Option<Rect>,
    /// Set by the Exit action; the frame loop closes the viewport.
    close_requested: bool,
    /// The Auto Shape flyout is open (toggled by its tool button).
    autoshape_flyout: bool,
    /// The keyboard-shortcuts help panel is open (F1 / Help menu).
    help_open: bool,
    /// The Styles panel's new-style name field.
    style_name: String,
    /// The layer whose name is being edited in the Layers panel
    /// (`None` = no inline rename open).
    rename_layer: Option<usize>,
    /// The live egui context, stashed each frame so code outside the
    /// frame loop (the web bridge) can request a repaint after it
    /// mutates the shell — without one, a bridge-loaded document sat
    /// behind a stale untitled canvas until the next input event.
    ctx: Option<egui::Context>,
}

impl Shell {
    /// Builds the shell with desktop platform defaults.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_cs6_theme(&cc.egui_ctx);
        Self::with_platform(
            Editor::untitled(),
            Box::new(app_cli::io::StdFs),
            Box::new(EnvDialogs::default()),
        )
    }

    /// Builds the shell over an editor and explicit platform services.
    #[must_use]
    pub fn with_platform(
        editor: Editor,
        fs: Box<dyn engine::platform::FileSystem>,
        dialogs: Box<dyn engine::platform::Dialogs>,
    ) -> Self {
        Self {
            editor,
            view: View::default(),
            raster: None,
            fs,
            dialogs,
            status: "ready".to_owned(),
            export_dialog: None,
            doc_name: "untitled".to_owned(),
            canvas_rect: None,
            close_requested: false,
            autoshape_flyout: false,
            help_open: false,
            style_name: String::new(),
            rename_layer: None,
            ctx: None,
        }
    }

    /// Wakes the frame loop — a no-op until the first `ui` frame has
    /// stashed the context. Bridge-side mutations (open/apply) must
    /// call this: they run between frames, where nothing else would
    /// repaint the stale canvas.
    pub fn request_repaint(&self) {
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }

    /// Replaces the document from raw `.fw.png` bytes — the same read
    /// path the desktop open uses (`io::open`, no registry side door).
    ///
    /// # Errors
    /// Whatever the reader refuses; the current document is kept.
    pub fn load_document_bytes(&mut self, bytes: &[u8]) -> Result<(), engine::EngineError> {
        let editor = Editor::open(bytes)?;
        self.editor = editor;
        self.view = View::default();
        self.raster = None;
        self.status = "opened document".to_owned();
        Ok(())
    }

    /// A change stamp over the session: any apply/undo/redo moves at
    /// least one component, so the page raster re-renders exactly when
    /// the document could have changed.
    fn stamp(&self) -> (usize, usize, usize) {
        (
            self.editor.session.undo_len(),
            self.editor.session.redo_len(),
            self.editor.session.journal_len(),
        )
    }

    /// File → Open through the injected traits. A Fireworks PNG
    /// opens as a document; any other supported raster opens as a
    /// document sized to the image — one door, no guessing.
    fn open_document(&mut self) {
        let Some(path) = self.dialogs.open_path(
            "Open document",
            &[
                ("Fireworks PNG".to_owned(), vec!["png".to_owned()]),
                (
                    "Images".to_owned(),
                    vec![
                        "png".to_owned(),
                        "jpg".to_owned(),
                        "jpeg".to_owned(),
                        "gif".to_owned(),
                        "bmp".to_owned(),
                        "tif".to_owned(),
                        "tiff".to_owned(),
                        "webp".to_owned(),
                    ],
                ),
            ],
        ) else {
            return;
        };
        let bytes = match self.fs.read_file(&path).map_err(|e| e.to_string()) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("open failed: {error}");
                return;
            }
        };
        match Editor::open(&bytes) {
            Ok(editor) => {
                self.status = format!("opened {path}");
                self.raster = None;
                self.editor = editor;
                self.view = View::default();
                self.doc_name = std::path::Path::new(&path)
                    .file_name()
                    .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
            }
            Err(document_error) => {
                // Not a Fireworks PNG — try a plain raster. A raster
                // opens as its own document, sized to the image, which
                // is what CS6 does with a JPEG.
                let name = std::path::Path::new(&path)
                    .file_name()
                    .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
                match self.open_raster_document(&bytes, &name) {
                    Ok(()) => {
                        self.status = format!("opened {path} as an image");
                        self.raster = None;
                        self.view = View::default();
                        self.doc_name = name;
                    }
                    Err(_) => {
                        self.status = format!("open failed: {document_error}");
                    }
                }
            }
        }
    }

    /// File → Import…: place a raster into the current document, on
    /// the active layer.
    fn import_raster_dialog(&mut self) {
        let Some(path) = self.dialogs.open_path(
            "Import image",
            &[(
                "Images".to_owned(),
                vec![
                    "png".to_owned(),
                    "jpg".to_owned(),
                    "jpeg".to_owned(),
                    "gif".to_owned(),
                    "bmp".to_owned(),
                    "tif".to_owned(),
                    "tiff".to_owned(),
                    "webp".to_owned(),
                ],
            )],
        ) else {
            return;
        };
        let bytes = match self.fs.read_file(&path).map_err(|e| e.to_string()) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("import failed: {error}");
                return;
            }
        };
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        match self.import_raster(&bytes, &name) {
            Ok(placed) => {
                self.status = format!("imported {name} ({placed})");
            }
            Err(error) => {
                self.status = format!("import failed: {error}");
            }
        }
    }

    /// Decodes `bytes` and places the raster on the active layer of
    /// the current page. A source larger than the canvas is scaled to
    /// fit it (aspect kept); a smaller one lands unscaled at the
    /// canvas corner, the way CS6 imports it.
    fn import_raster(&mut self, bytes: &[u8], name: &str) -> Result<String, String> {
        let decoded = io::raster::decode(bytes).map_err(|error| error.to_string())?;
        let (page, layer, cell) = (
            self.editor.page,
            self.editor.active_layer(),
            self.editor.frame,
        );
        let canvas = self
            .editor
            .document()
            .pages
            .get(page)
            .map(|page| page.info.canvas.clone())
            .ok_or_else(|| "the document has no such page".to_owned())?;
        let fits =
            f64::from(decoded.width) <= canvas.width && f64::from(decoded.height) <= canvas.height;
        let bounds = if fits {
            // A degenerate rectangle places the file unscaled with
            // its top-left corner at that point.
            serde_json::json!({
                "left": canvas.x,
                "top": canvas.y,
                "right": canvas.x,
                "bottom": canvas.y,
            })
        } else {
            serde_json::json!({
                "left": canvas.x,
                "top": canvas.y,
                "right": canvas.x + canvas.width,
                "bottom": canvas.y + canvas.height,
            })
        };
        let payload = serde_json::json!({
            "width": decoded.width,
            "height": decoded.height,
            "pixels": base64::engine::general_purpose::STANDARD.encode(&decoded.rgba),
            "bounds": bounds,
            "maintain_aspect_ratio": true,
            "page": page,
            "layer": layer,
            "cell": cell,
            "name": name,
        });
        self.editor
            .apply("bitmap.importFile", payload)
            .map_err(|error| error.to_string())?;
        Ok(format!("{}x{}", decoded.width, decoded.height))
    }

    /// Opens a plain raster as its own document: a fresh one-page
    /// document whose canvas matches the image, with the image on the
    /// page's layer.
    fn open_raster_document(&mut self, bytes: &[u8], name: &str) -> Result<(), String> {
        let decoded = io::raster::decode(bytes).map_err(|error| error.to_string())?;
        let mut editor = Editor::untitled();
        let _ = editor.apply(
            "page.setCanvasSize",
            serde_json::json!({
                "page": 0,
                "width": decoded.width,
                "height": decoded.height,
            }),
        );
        let payload = serde_json::json!({
            "width": decoded.width,
            "height": decoded.height,
            "pixels": base64::engine::general_purpose::STANDARD.encode(&decoded.rgba),
            "bounds": {
                "left": 0.0,
                "top": 0.0,
                "right": 0.0,
                "bottom": 0.0,
            },
            "maintain_aspect_ratio": true,
            "page": 0,
            "layer": 0,
            "cell": 0,
            "name": name,
        });
        editor
            .apply("bitmap.importFile", payload)
            .map_err(|error| error.to_string())?;
        self.editor = editor;
        Ok(())
    }

    /// File → Save through the injected traits.
    fn save_document(&mut self) {
        let Some(path) = self.dialogs.save_path(
            "Save document",
            "untitled.fw.png",
            &[("Fireworks PNG".to_owned(), vec!["png".to_owned()])],
        ) else {
            return;
        };
        match self.editor.save() {
            Ok(bytes) => match self.fs.write_file(&path, &bytes) {
                Ok(()) => {
                    self.editor.session.mark_clean();
                    self.doc_name = std::path::Path::new(&path)
                        .file_name()
                        .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
                    self.status = format!("saved {path} ({} bytes)", bytes.len());
                }
                Err(error) => self.status = format!("save failed: {error}"),
            },
            Err(error) => self.status = format!("save failed: {error}"),
        }
    }

    /// Applies through the editor and reports failures on the status
    /// line — the one idiom every menu/panel action shares.
    fn run_command(&mut self, name: &str, payload: serde_json::Value) {
        if let Err(error) = self.editor.apply(name, payload) {
            self.status = format!("{name} failed: {error}");
        }
    }

    /// History → Save Steps…: the session journal written as a
    /// `.fwsteps` command file (JSON lines) through the injected
    /// traits. The file replays through [`Session::reopen`] — the same
    /// path the CLI's `apply` verb drives — so a saved command is not
    /// an export but the session's own restart record.
    fn save_steps(&mut self) {
        let Some(path) = self.dialogs.save_path(
            "Save Steps As Command",
            "steps.fwsteps",
            &[("FireCraft steps".to_owned(), vec!["fwsteps".to_owned()])],
        ) else {
            return;
        };
        let text = self.editor.steps_text();
        match self.fs.write_file(&path, text.as_bytes()) {
            Ok(()) => {
                self.status = format!(
                    "saved steps {path} ({} rows)",
                    self.editor.session.journal_len()
                );
            }
            Err(error) => self.status = format!("save steps failed: {error}"),
        }
    }

    /// The Web Layer panel: the page's slices and hotspots with link
    /// editing (`slice.setLink`) and the foreground/background toggle
    /// (`slice.setType`). Snapshots the web objects first so the edit
    /// fields never borrow the document while commands run.
    fn web_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Web Layer");
        let page = self.editor.page;
        let frame = self.editor.frame;
        let rows: Vec<WebRow> = {
            let document = self.editor.document();
            let Some(layer) = document.pages.get(page).and_then(|page| {
                page.layers
                    .layers
                    .iter()
                    .rev()
                    .find(|layer| layer.has_url && layer.is_special)
            }) else {
                return;
            };
            let Some(cell) = layer.cells.get(frame) else {
                return;
            };
            cell.elements
                .iter()
                .filter_map(|id| match document.registry.get(*id) {
                    Some(ObjectData::Slice(slice)) => Some((
                        *id,
                        slice
                            .name
                            .clone()
                            .unwrap_or_else(|| format!("slice {}", id.index())),
                        slice.slice_type.is_some(),
                        slice.link.clone(),
                        slice.alt.clone(),
                        slice.slice_type.unwrap_or(0),
                    )),
                    _ => None,
                })
                .collect()
        };
        if rows.is_empty() {
            ui.label("no slices on this state");
            return;
        }
        for (id, name, is_slice, link, alt, slice_type) in rows {
            ui.horizontal(|ui| {
                ui.strong(&name);
                ui.label(if is_slice { "slice" } else { "hotspot" });
            });
            let mut edited = link.unwrap_or_default();
            let response = ui.add(
                egui::TextEdit::singleline(&mut edited)
                    .hint_text("link URL")
                    .desired_width(f32::INFINITY),
            );
            if response.changed() {
                // An emptied field clears the link (null, not "");
                // re-assert the existing alt so a link edit never
                // clears it.
                let link = (!edited.is_empty()).then_some(edited);
                self.run_command(
                    "slice.setLink",
                    serde_json::json!({ "id": id, "link": link, "alt": alt }),
                );
            }
            if is_slice {
                ui.horizontal(|ui| {
                    ui.label("type");
                    if ui.selectable_label(slice_type == 0, "foreground").clicked() {
                        self.run_command(
                            "slice.setType",
                            serde_json::json!({ "id": id, "slice_type": 0 }),
                        );
                    }
                    if ui.selectable_label(slice_type == 2, "background").clicked() {
                        self.run_command(
                            "slice.setType",
                            serde_json::json!({ "id": id, "slice_type": 2 }),
                        );
                    }
                });
            }
            ui.separator();
        }
    }

    /// History → Replay…: pick a `.fwsteps` file, then the base
    /// document it replays onto (the File → Open flow — a steps file
    /// is a journal prefix, so it needs the pre-steps document). The
    /// replayed session replaces the current one, undo history
    /// included.
    fn replay_steps(&mut self) {
        let steps_filters = [("FireCraft steps".to_owned(), vec!["fwsteps".to_owned()])];
        let Some(steps_path) = self.dialogs.open_path("Replay steps", &steps_filters) else {
            return;
        };
        let steps = match self
            .fs
            .read_file(&steps_path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| String::from_utf8(bytes).map_err(|error| error.to_string()))
        {
            Ok(text) => text,
            Err(error) => {
                self.status = format!("replay failed: {error}");
                return;
            }
        };
        let doc_filters = [("Fireworks PNG".to_owned(), vec!["png".to_owned()])];
        let Some(doc_path) = self
            .dialogs
            .open_path("Base document for replay", &doc_filters)
        else {
            return;
        };
        let replayed = self
            .fs
            .read_file(&doc_path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                Editor::open(&bytes)
                    .map_err(|e| e.to_string())
                    .and_then(|mut editor| {
                        editor
                            .replay_steps(&steps)
                            .map(|()| editor)
                            .map_err(|e| e.to_string())
                    })
            });
        match replayed {
            Ok(editor) => {
                let rows = editor.session.journal_len();
                self.editor = editor;
                self.view = View::default();
                self.raster = None;
                self.status = format!("replayed {steps_path} onto {doc_path} ({rows} rows)");
            }
            Err(error) => self.status = format!("replay failed: {error}"),
        }
    }

    /// The export dialog (File → Export…): format + per-format
    /// controls, a 2-Up preview of the chosen settings against a
    /// lossless PNG pane, and Export through the save dialog.
    fn export_dialog_ui(&mut self, ui: &mut egui::Ui) {
        let mut dialog = match self.export_dialog.take() {
            Some(dialog) => dialog,
            None => return,
        };
        egui::Grid::new("export_controls")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                egui::ComboBox::from_label("Web workflow")
                    .selected_text(match dialog.web {
                        Some(index) => WEB_WORKFLOWS[index],
                        None => "—",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut dialog.web, None, "—");
                        for (i, name) in WEB_WORKFLOWS.iter().enumerate() {
                            ui.selectable_value(&mut dialog.web, Some(i), *name);
                        }
                    });
                if dialog.web.is_some() {
                    ui.checkbox(&mut dialog.web_shim, "Shim edges");
                    ui.checkbox(&mut dialog.web_undefined, "Include areas without slices");
                }
                egui::ComboBox::from_label("Format")
                    .selected_text(EXPORT_FORMATS[dialog.format])
                    .show_ui(ui, |ui| {
                        for (i, name) in EXPORT_FORMATS.iter().enumerate() {
                            ui.selectable_value(&mut dialog.format, i, *name);
                        }
                    });
                let format = EXPORT_FORMATS[dialog.format];
                if format == "jpg" {
                    ui.add(egui::Slider::new(&mut dialog.quality, 1..=100).text("Quality"));
                    ui.checkbox(&mut dialog.progressive, "Progressive");
                    ui.horizontal(|ui| {
                        ui.label("Matte");
                        ui.selectable_value(&mut dialog.matte_black, false, "white");
                        ui.selectable_value(&mut dialog.matte_black, true, "black");
                    });
                }
                if format == "gif" {
                    egui::ComboBox::from_label("Loop")
                        .selected_text(match dialog.gif_loop {
                            0 => "infinite",
                            1 => "never",
                            _ => "repeat n times",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut dialog.gif_loop, 0, "infinite");
                            ui.selectable_value(&mut dialog.gif_loop, 1, "never");
                            ui.selectable_value(&mut dialog.gif_loop, 2, "repeat n times");
                        });
                    if dialog.gif_loop == 2 {
                        ui.add(egui::DragValue::new(&mut dialog.gif_times).range(1..=65535));
                    }
                    ui.checkbox(&mut dialog.gif_crop, "Crop frames");
                    ui.checkbox(&mut dialog.gif_diff, "Diff frames");
                    ui.add(egui::Slider::new(&mut dialog.gif_dither, 0..=100).text("Dither %"));
                }
            });

        // The 2-Up: left pane is the chosen settings, right pane the
        // lossless PNG reference. Regenerates when settings or the
        // document move. Web workflows write many files — no preview.
        let stamp = self.preview_stamp();
        let regenerate = dialog.web.is_none()
            && match &dialog.preview {
                Some((made, _, _, _, _)) => *made != stamp,
                None => true,
            };
        if regenerate {
            let fresh = self.render_preview(ui.ctx());
            dialog.preview = fresh;
        }
        if let Some((_, left, left_label, right, right_label)) = &dialog.preview {
            ui.separator();
            ui.columns(2, |columns| {
                columns[0].label(format!("export ({left_label})"));
                columns[0].add(egui::Image::new((left.id(), Vec2::splat(128.0))));
                columns[1].label(format!("png reference ({right_label})"));
                columns[1].add(egui::Image::new((right.id(), Vec2::splat(128.0))));
            });
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Export…").clicked() {
                self.export_via_dialog(&dialog);
            }
            if ui.button("Cancel").clicked() {
                self.status = "export cancelled".to_owned();
            }
        });
        self.export_dialog = Some(dialog);
    }

    /// A change stamp over the dialog settings plus the document, so
    /// the preview regenerates exactly when either could have moved.
    fn preview_stamp(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let Some(dialog) = &self.export_dialog else {
            return 0;
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (
            dialog.format,
            dialog.quality,
            dialog.progressive,
            dialog.matte_black,
            dialog.gif_loop,
            dialog.gif_times,
            dialog.gif_crop,
            dialog.gif_diff,
            dialog.gif_dither,
            self.stamp(),
        )
            .hash(&mut hasher);
        hasher.finish()
    }

    /// The dialog's settings as a model `ExportOptions` (what Export
    /// stamps onto the document; the 2-Up and the file share it).
    fn dialog_options(dialog: &ExportDialog) -> doc::ExportOptions {
        let jpg = EXPORT_FORMATS[dialog.format] == "jpg";
        let gif = EXPORT_FORMATS[dialog.format] == "gif";
        doc::ExportOptions {
            format: Some(i64::try_from(dialog.format).unwrap_or(0)),
            jpeg_quality: jpg.then_some(i64::from(dialog.quality)),
            jpeg_progressive: jpg.then_some(dialog.progressive),
            frame_loop: gif.then_some(match dialog.gif_loop {
                0 => 0,
                1 => -1,
                _ => i64::from(dialog.gif_times),
            }),
            anim_crop: gif.then_some(dialog.gif_crop),
            anim_diff: gif.then_some(dialog.gif_diff),
            dither: gif.then_some(i64::from(dialog.gif_dither)),
            ..doc::ExportOptions::default()
        }
    }

    /// The dialog's JPEG flags (the stated-over-recorded side of the
    /// same controls).
    fn dialog_flags(dialog: &ExportDialog) -> app_cli::ops::ExportFlags {
        app_cli::ops::ExportFlags {
            quality: if EXPORT_FORMATS[dialog.format] == "jpg" {
                Some(u8::try_from(dialog.quality).unwrap_or(90))
            } else {
                None
            },
            progressive: dialog.progressive,
            matte: if dialog.matte_black {
                io::Matte::Black
            } else {
                io::Matte::White
            },
        }
    }

    /// Encodes page 0 / state 0 in `format` with the dialog's settings
    /// stamped onto a cloned document — byte-identical to what Export
    /// writes.
    fn encode_with_dialog(&self, dialog: &ExportDialog, format: &str) -> Result<Vec<u8>, String> {
        let mut document = self.editor.document().clone();
        let export = Self::dialog_options(dialog);
        if let Some(page) = document.pages.first_mut() {
            page.export = Some(export);
        }
        let flags = Self::dialog_flags(dialog);
        let mapped = |result: Result<Vec<u8>, app_cli::CliError>| match result {
            Ok(bytes) => Ok(bytes),
            Err(error) => Err(error.to_string()),
        };
        match format {
            "svg" => mapped(app_cli::ops::export_svg(&document)),
            "psd" => mapped(app_cli::ops::export_psd(&document)),
            "gif" => mapped(app_cli::ops::export_gif(&document)),
            _ => mapped(app_cli::ops::export_raster(
                &document,
                &format!("x.{format}"),
                &flags,
            )),
        }
    }

    /// Renders state 0 once and builds the two 2-Up panes: chosen
    /// settings, and a lossless PNG for comparison. Each pane keeps
    /// its byte size in the label. `ctx` is the live egui context the
    /// textures are loaded onto.
    fn render_preview(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<(u64, TextureHandle, String, TextureHandle, String)> {
        let dialog = self.export_dialog.as_ref()?;
        let format = EXPORT_FORMATS[dialog.format];
        let chosen = match self.encode_with_dialog(dialog, format) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("preview failed: {error}");
                return None;
            }
        };
        let reference = match self.encode_with_dialog(dialog, "png") {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("preview failed: {error}");
                return None;
            }
        };
        let to_texture = |bytes: &[u8]| -> Option<(TextureHandle, String)> {
            let decoded = io::raster::decode(bytes).ok()?;
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [decoded.width as usize, decoded.height as usize],
                &decoded.rgba,
            );
            let label = format!("{} bytes", bytes.len());
            Some((
                ctx.load_texture("export-preview", image, TextureOptions::LINEAR),
                label,
            ))
        };
        let ((left, left_size), (right, right_size)) =
            (to_texture(&chosen)?, to_texture(&reference)?);
        let stamp = self.preview_stamp();
        Some((
            stamp,
            left,
            format!("{format}, {left_size}"),
            right,
            format!("png, {right_size}"),
        ))
    }

    /// Export through the save dialog: encode with the dialog's
    /// settings, write through the injected file system. A selected
    /// web workflow routes to [`Self::export_web_via_dialog`] instead.
    fn export_via_dialog(&mut self, dialog: &ExportDialog) {
        if let Some(index) = dialog.web {
            self.export_web_via_dialog(dialog, index);
            return;
        }
        let format = EXPORT_FORMATS[dialog.format];
        let encoded = match self.encode_with_dialog(dialog, format) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("export failed: {error}");
                return;
            }
        };
        let suggested = format!("export.{format}");
        let Some(path) = self.dialogs.save_path(
            "Export",
            &suggested,
            &[(format.to_owned(), vec![format.to_owned()])],
        ) else {
            return;
        };
        match self.fs.write_file(&path, &encoded) {
            Ok(()) => {
                self.status = format!("exported {path} ({} bytes)", encoded.len());
                self.export_dialog = None;
            }
            Err(error) => self.status = format!("export failed: {error}"),
        }
    }

    /// Web export through the save dialog: the save path names the
    /// primary text artifact (`export.html`, the sprite/properties
    /// CSS); every image file lands beside it under its export name.
    /// The pipeline is [`app_cli::ops::export_web`] — the CLI verb's
    /// exact path, so the dialog cannot drift from the CLI.
    fn export_web_via_dialog(&mut self, dialog: &ExportDialog, index: usize) {
        let Some(workflow) = web_workflow(index) else {
            return;
        };
        let options = webexport::WebExportOptions {
            page: self.editor.page,
            state: self.editor.frame,
            doc_name: "export".to_owned(),
            title: "export".to_owned(),
            shim: dialog.web_shim,
            include_undefined: dialog.web_undefined,
            ..webexport::WebExportOptions::default()
        };
        let export = match app_cli::ops::export_web(self.editor.document(), &options, workflow) {
            Ok(export) => export,
            Err(error) => {
                self.status = format!("export failed: {error}");
                return;
            }
        };
        let (suggested, filter) = match workflow {
            webexport::Workflow::CssSprites => ("sprites.css", "css"),
            webexport::Workflow::CssProperties => ("properties.css", "css"),
            webexport::Workflow::StatesToGif => ("export.gif", "gif"),
            _ => ("export.html", "html"),
        };
        let Some(path) = self.dialogs.save_path(
            "Export",
            suggested,
            &[(filter.to_owned(), vec![filter.to_owned()])],
        ) else {
            return;
        };
        let dir = std::path::Path::new(&path)
            .parent()
            .map_or_else(std::path::PathBuf::new, std::path::Path::to_path_buf);
        let mut count = 0usize;
        for file in &export.files {
            let target = dir.join(&file.name);
            match self.fs.write_file(&target.to_string_lossy(), &file.bytes) {
                Ok(()) => count += 1,
                Err(error) => {
                    self.status = format!("export failed: {error}");
                    return;
                }
            }
        }
        if let Some(text) = &export.html {
            if let Err(error) = self.fs.write_file(&path, text.as_bytes()) {
                self.status = format!("export failed: {error}");
                return;
            }
            count += 1;
        }
        self.status = format!("exported {count} files to {}", dir.to_string_lossy());
        self.export_dialog = None;
    }

    /// The CS6 menu bar: ten menus rendered from the [`crate::menus`]
    /// table. Items whose action is [`menus::Action::Inert`] render
    /// disabled — the bar shows the full CS6 surface while only the
    /// wired subset is actionable.
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            for menu in menus::MENUBAR {
                ui.menu_button(menu.label, |ui| self.entries_ui(ui, menu.entries));
            }
        });
    }

    /// Renders one menu-tree level; submenus recurse.
    fn entries_ui(&mut self, ui: &mut egui::Ui, entries: &'static [menus::Entry]) {
        for entry in entries {
            match entry {
                menus::Entry::Sep => {
                    ui.separator();
                }
                menus::Entry::Sub(label, sub) => {
                    ui.menu_button(*label, |ui| self.entries_ui(ui, sub));
                }
                menus::Entry::Item(item) => {
                    let label = if item.accel.is_empty() {
                        item.label.to_owned()
                    } else {
                        format!("{}\t{}", item.label, item.accel)
                    };
                    let wired = !matches!(item.action, menus::Action::Inert);
                    if ui.add_enabled(wired, egui::Button::new(label)).clicked() {
                        self.run_action(&item.action);
                    }
                }
            }
        }
    }

    /// Executes one menu/shortcut action. This is the only place an
    /// [`menus::Action`] becomes behavior.
    fn run_action(&mut self, action: &menus::Action) {
        match *action {
            menus::Action::Inert => {}
            menus::Action::Shell(shell) => self.run_shell(shell),
            menus::Action::Sel(sel) => self.run_sel(sel),
            menus::Action::Align(mode) => {
                let ids = self.editor.live_selection();
                if ids.len() >= 2 {
                    self.run_command(
                        "object.align",
                        serde_json::json!({ "ids": ids, "mode": mode }),
                    );
                } else {
                    self.status = "align needs two or more objects".to_owned();
                }
            }
            menus::Action::Distribute(axis) => {
                let ids = self.editor.live_selection();
                if ids.len() >= 3 {
                    self.run_command(
                        "object.distribute",
                        serde_json::json!({ "ids": ids, "axis": axis }),
                    );
                } else {
                    self.status = "distribute needs three or more objects".to_owned();
                }
            }
            menus::Action::Boolean(op) => match self.editor.boolean_pair() {
                Some((a, b)) => self.run_command(
                    "path.boolean",
                    serde_json::json!({ "a": a, "b": b, "op": op }),
                ),
                None => self.status = "booleans need exactly two live paths".to_owned(),
            },
            menus::Action::Arrange(direction) => self.run_arrange(direction),
            menus::Action::TextSize(size) => {
                if let Some(id) = self.editor.selection {
                    self.run_command(
                        "text.setAttrs",
                        serde_json::json!({ "id": id, "size": size }),
                    );
                }
            }
            menus::Action::TextFlag(flag) => {
                if let Some(id) = self.editor.selection {
                    self.run_command(
                        "text.setAttrs",
                        serde_json::json!({ "id": id, (flag): true }),
                    );
                }
            }
            menus::Action::Filter(name) => match self.editor.selection {
                Some(id) => match filter_preset(name) {
                    Some(preset) => self.apply_filter_preset(id, *preset),
                    None => self.status = format!("no such filter: {name}"),
                },
                None => self.status = "pick an object to filter".to_owned(),
            },
        }
    }

    /// Shell-level plumbing: file, history, selection, view.
    fn run_shell(&mut self, action: menus::ShellAction) {
        match action {
            menus::ShellAction::New => {
                self.editor = Editor::untitled();
                self.view = View::default();
                self.raster = None;
                self.doc_name = "untitled".to_owned();
                self.status = "new document".to_owned();
            }
            menus::ShellAction::Open => self.open_document(),
            menus::ShellAction::Import => self.import_raster_dialog(),
            menus::ShellAction::Save | menus::ShellAction::SaveAs => self.save_document(),
            menus::ShellAction::Export | menus::ShellAction::ExportPreview => {
                self.export_dialog = Some(ExportDialog::default());
            }
            menus::ShellAction::Quit => self.close_requested = true,
            menus::ShellAction::Undo => match self.editor.undo() {
                Ok(name) => self.status = format!("undid {name}"),
                Err(error) => self.status = format!("undo failed: {error}"),
            },
            menus::ShellAction::Redo => match self.editor.redo() {
                Ok(name) => self.status = format!("redid {name}"),
                Err(error) => self.status = format!("redo failed: {error}"),
            },
            menus::ShellAction::Deselect => {
                self.editor.set_selection(Vec::new());
                self.status = "deselected".to_owned();
            }
            menus::ShellAction::SelectAll => {
                let ids = self
                    .editor
                    .current_cell()
                    .map_or_else(Vec::new, |cell| cell.elements.clone());
                let picked = ids.len();
                self.editor.set_selection(ids);
                self.status = format!("selected {picked} object(s)");
            }
            menus::ShellAction::SelectInverse => self.select_inverse(),
            menus::ShellAction::Copy => self.clipboard_copy(false),
            menus::ShellAction::Cut => self.clipboard_copy(true),
            menus::ShellAction::Paste => self.clipboard_paste(),
            menus::ShellAction::PasteAsMask => self.paste_as_mask("clipboard.pasteAsMask"),
            menus::ShellAction::PasteInside => self.paste_as_mask("clipboard.pasteInside"),
            menus::ShellAction::ZoomIn => {
                let center = self
                    .canvas_rect
                    .map_or_else(|| Pos2::ZERO, |rect| rect.center());
                self.view.zoom_about(center, 1.25);
            }
            menus::ShellAction::ZoomOut => {
                let center = self
                    .canvas_rect
                    .map_or_else(|| Pos2::ZERO, |rect| rect.center());
                self.view.zoom_about(center, 1.0 / 1.25);
            }
            menus::ShellAction::ZoomTo(target) => {
                if self.view.zoom > 0.0 {
                    let center = self
                        .canvas_rect
                        .map_or_else(|| Pos2::ZERO, |rect| rect.center());
                    self.view.zoom_about(center, target / self.view.zoom);
                }
            }
            menus::ShellAction::FitAll => {
                let size = self
                    .canvas_rect
                    .map_or_else(|| egui::Vec2::ZERO, |rect| rect.size());
                if let Some(world) = self.page_world() {
                    self.view.fit(world, size);
                }
            }
            menus::ShellAction::About => {
                self.status = "FireCraft — a clean-room reimplementation of \
                    Adobe Fireworks CS6 (Apache-2.0)"
                    .to_owned();
            }
            menus::ShellAction::ShowShortcuts => self.help_open = !self.help_open,
            menus::ShellAction::PickTool(tool) => {
                self.editor.tool = tool;
                self.status = format!("tool: {}", tool_label(tool));
            }
        }
    }

    /// Select ▸ Select Inverse: everything in the current cell the
    /// selection does not cover. The cell is the editor's scope (one
    /// layer, one state) — the same scope Select All covers.
    fn select_inverse(&mut self) {
        let selected = self.editor.live_selection();
        let ids: Vec<doc::ObjectId> = self
            .editor
            .current_cell()
            .map_or_else(Vec::new, |cell| cell.elements.clone())
            .into_iter()
            .filter(|id| !selected.contains(id))
            .collect();
        let picked = ids.len();
        self.editor.set_selection(ids);
        self.status = format!("selected {picked} object(s)");
    }

    /// Edit ▸ Copy / Cut: the engine's clipboard commands read the
    /// document's selection, so the view's selection lands there
    /// first. A cut takes the objects with it, so the view's selection
    /// follows them out.
    fn clipboard_copy(&mut self, cut: bool) {
        if self.editor.live_selection().is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        let name = if cut {
            "clipboard.cut"
        } else {
            "clipboard.copy"
        };
        match self.editor.apply(name, serde_json::json!({})) {
            Ok(()) => {
                if cut {
                    self.editor.set_selection(Vec::new());
                    self.status = "cut".to_owned();
                } else {
                    self.status = "copied".to_owned();
                }
            }
            Err(error) => self.status = format!("{name} failed: {error}"),
        }
    }

    /// Edit ▸ Paste: the clipboard re-mints onto the active layer of
    /// the current page, in place (CS6's paste-in-place). The pasted
    /// objects become the selection, so a second paste rides the same
    /// place.
    fn clipboard_paste(&mut self) {
        let page = self.editor.page;
        let layer = self.editor.active_layer();
        let cell = self.editor.frame;
        let before: Vec<doc::ObjectId> = self
            .editor
            .current_cell()
            .map_or_else(Vec::new, |cell| cell.elements.clone());
        let payload = serde_json::json!({ "page": page, "layer": layer, "cell": cell });
        if let Err(error) = self.editor.apply("clipboard.paste", payload) {
            self.status = format!("clipboard.paste failed: {error}");
            return;
        }
        let after: Vec<doc::ObjectId> = self
            .editor
            .current_cell()
            .map_or_else(Vec::new, |cell| cell.elements.clone());
        let fresh: Vec<doc::ObjectId> = after
            .into_iter()
            .filter(|id| !before.contains(id))
            .collect();
        let pasted = fresh.len();
        self.editor.set_selection(fresh);
        self.status = format!("pasted {pasted} object(s)");
    }

    /// Edit ▸ Paste as Mask / Paste Inside: the clipboard becomes the
    /// selection's mask. Both commands read the document's selection,
    /// so it is synced first.
    fn paste_as_mask(&mut self, name: &str) {
        if self.editor.live_selection().is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command(name, serde_json::json!({}));
    }

    /// Engine commands whose payload derives from the current
    /// selection or document shape.
    fn run_sel(&mut self, action: menus::SelAction) {
        match action {
            menus::SelAction::Delete => {
                if let Some(id) = self.editor.selection {
                    self.run_command("object.delete", serde_json::json!({ "id": id }));
                }
            }
            menus::SelAction::Hide => {
                if let Some(id) = self.editor.selection {
                    self.run_command(
                        "object.setVisible",
                        serde_json::json!({ "id": id, "visible": false }),
                    );
                }
            }
            menus::SelAction::Simplify => {
                if let Some(id) = self.editor.selected_live() {
                    self.run_command(
                        "path.simplify",
                        serde_json::json!({ "id": id, "tolerance": 1.0 }),
                    );
                }
            }
            menus::SelAction::ExpandStroke => {
                if let Some(id) = self.editor.selected_live() {
                    self.run_command(
                        "path.expandStroke",
                        serde_json::json!({
                            "id": id, "width": 2.0,
                            "cap": "butt", "join": "miter", "miter_limit": 4.0,
                        }),
                    );
                }
            }
            menus::SelAction::Inset(distance) => {
                if let Some(id) = self.editor.selected_live() {
                    self.run_command(
                        "path.inset",
                        serde_json::json!({ "id": id, "distance": distance }),
                    );
                }
            }
            menus::SelAction::NewSymbol => {
                let count = self.editor.document().symbols.len();
                self.run_command(
                    "library.add",
                    serde_json::json!({
                        "name": format!("Symbol {}", count + 1),
                        "symbol_type": "graphic",
                    }),
                );
            }
            menus::SelAction::NewLayer => {
                self.run_command("layer.add", serde_json::json!({ "page": self.editor.page }));
                // CS6 makes a freshly added layer the active one, so
                // what the user draws next lands on it.
                let top = self.editor.layer_count().saturating_sub(1);
                self.editor.set_active_layer(top);
            }
            menus::SelAction::NewPage => {
                self.run_command("page.add", serde_json::json!({ "index": null }));
            }
            menus::SelAction::BreakApart => {
                if let Some(id) = self.editor.selection {
                    self.run_command("symbol.breakApart", serde_json::json!({ "id": id }));
                }
            }
            menus::SelAction::Group => {
                let ids = self.editor.live_selection();
                if ids.len() < 2 {
                    self.status = "group needs two or more objects".to_owned();
                    return;
                }
                self.run_command("object.group", serde_json::json!({ "ids": ids }));
            }
            menus::SelAction::Ungroup => {
                if let Some(id) = self.editor.selected_live() {
                    self.run_command("object.ungroup", serde_json::json!({ "id": id }));
                }
            }
            menus::SelAction::FlattenSelection => self.flatten_selection(),
            menus::SelAction::MergeDown => self.merge_down(),
            menus::SelAction::FlattenLayers => {
                let page = self.editor.page;
                self.run_command("flatten.document", serde_json::json!({ "page": page }));
            }
            menus::SelAction::Rotate(degrees) => {
                let ids = self.editor.live_selection();
                if ids.is_empty() {
                    self.status = "nothing selected".to_owned();
                    return;
                }
                self.run_command(
                    "object.rotate",
                    serde_json::json!({ "ids": ids, "degrees": degrees, "pivot": null }),
                );
            }
            menus::SelAction::Reflect {
                horizontal,
                vertical,
            } => {
                let ids = self.editor.live_selection();
                if ids.is_empty() {
                    self.status = "nothing selected".to_owned();
                    return;
                }
                self.run_command(
                    "object.reflect",
                    serde_json::json!({
                        "ids": ids, "horizontal": horizontal, "vertical": vertical,
                        "pivot": null,
                    }),
                );
            }
        }
    }

    /// Modify ▸ Flatten Selection: bakes the selection into one
    /// bitmap. The engine reads the document's selection, so the
    /// view's selection lands there first.
    fn flatten_selection(&mut self) {
        if self.editor.live_selection().is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command("flatten.selection", serde_json::json!({}));
    }

    /// Modify ▸ Merge Down: merges the selection into the bitmap
    /// directly below it. Same selection bridge as the flatten.
    fn merge_down(&mut self) {
        if self.editor.live_selection().is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command("flatten.mergeDown", serde_json::json!({}));
    }

    /// Modify ▸ Arrange: resolve the numeric `object.reorder` target
    /// from the object's position in the current cell. Payload indexes
    /// are post-removal, so "front" is `len - 2` of the pre-removal
    /// list.
    fn run_arrange(&mut self, direction: menus::Arrange) {
        let Some(id) = self.editor.selection else {
            self.status = "nothing selected".to_owned();
            return;
        };
        let Some(cell) = self.editor.current_cell() else {
            self.status = "no current cell".to_owned();
            return;
        };
        let Some(position) = cell.elements.iter().position(|&element| element == id) else {
            self.status = "selection is not in the current cell".to_owned();
            return;
        };
        let last = cell.elements.len().saturating_sub(2);
        let to = match direction {
            menus::Arrange::Front => last,
            menus::Arrange::Forward => (position + 1).min(last),
            menus::Arrange::Backward => position.saturating_sub(1),
            menus::Arrange::Back => 0,
        };
        if to == position {
            self.status = "already at that stacking position".to_owned();
            return;
        }
        self.run_command("object.reorder", serde_json::json!({ "id": id, "to": to }));
    }

    /// The keyboard-shortcuts help panel: every binding in the
    /// SHORTCUTS table, grouped by the table's section structure.
    fn help_panel(&mut self, ctx: &egui::Context) {
        let mut open = self.help_open;
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_width(340.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(
                                    "Plain keys do not fire while a text field has focus.",
                                )
                                .italics(),
                            )
                            .wrap(),
                        );
                        ui.separator();
                        // (title, row count) — mirrors the SHORTCUTS table's
                        // section comments; the sums are asserted in tests.
                        let groups: &[(&str, usize)] = &[
                            ("File", 7),
                            ("Edit", 6),
                            ("Clipboard", 3),
                            ("Modify ▸ Group", 3),
                            ("View", 3),
                            ("Modify ▸ Arrange", 4),
                            ("Modify ▸ Combine Paths", 1),
                            ("Text styles", 3),
                            ("Tools (one-key selects)", 14),
                            ("Pixel-selection tools", 4),
                            ("Help", 1),
                        ];
                        let mut row = 0usize;
                        for (title, count) in groups {
                            ui.heading(*title);
                            egui::Grid::new((*title).to_string())
                                .num_columns(2)
                                .spacing([12.0, 2.0])
                                .show(ui, |ui| {
                                    for s in &menus::SHORTCUTS[row..row + count] {
                                        ui.strong(s.label());
                                        ui.label(menus::action_name(s.action));
                                        ui.end_row();
                                    }
                                });
                            ui.separator();
                            row += count;
                        }
                    });
            });
        self.help_open = open;
    }

    /// The Fireworks.xml keymap bindings: one-shot key events mapped
    /// through the same [`Shell::run_action`] dispatch as the menus.
    /// Plain (modifier-free) keys are ignored while a text field has
    /// keyboard focus.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let text_focus = ctx.egui_wants_keyboard_input();
        let mut fired: Vec<menus::Action> = Vec::new();
        ctx.input(|input| {
            for event in &input.events {
                let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } = event
                else {
                    continue;
                };
                for shortcut in menus::SHORTCUTS {
                    if shortcut.key != *key
                        || shortcut.mods.ctrl != modifiers.ctrl
                        || shortcut.mods.alt != modifiers.alt
                        || shortcut.mods.shift != modifiers.shift
                    {
                        continue;
                    }
                    let plain = !shortcut.mods.ctrl && !shortcut.mods.alt && !shortcut.mods.shift;
                    if !(plain && text_focus) {
                        fired.push(shortcut.action);
                    }
                }
            }
        });
        for action in fired {
            self.run_action(&action);
        }
    }

    /// The right panel stack: the CS6 panel roster from the Window
    /// menu, as collapsible sections (the fixed-layout stand-in for
    /// CS6's floating docks).
    fn right_panels(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::CollapsingHeader::new("Pages")
                .default_open(true)
                .show(ui, |ui| self.pages_section(ui));
            egui::CollapsingHeader::new("States")
                .default_open(true)
                .show(ui, |ui| self.states_section(ui));
            egui::CollapsingHeader::new("Layers")
                .default_open(true)
                .show(ui, |ui| self.layers_section(ui));
            egui::CollapsingHeader::new("Styles")
                .default_open(false)
                .show(ui, |ui| self.styles_panel(ui));
            egui::CollapsingHeader::new("Document Library")
                .default_open(false)
                .show(ui, |ui| self.library_panel(ui));
            egui::CollapsingHeader::new("History")
                .default_open(false)
                .show(ui, |ui| self.history_section(ui));
            egui::CollapsingHeader::new("Web Layer")
                .default_open(false)
                .show(ui, |ui| self.web_panel(ui));
        });
    }

    /// Pages section: one selectable row per page.
    fn pages_section(&mut self, ui: &mut egui::Ui) {
        let pages = self.editor.document().pages.len();
        for index in 0..pages {
            let name = self.editor.document().pages[index]
                .name
                .clone()
                .unwrap_or_else(|| format!("Page {}", index + 1));
            if ui
                .selectable_label(self.editor.page == index, &name)
                .clicked()
            {
                self.editor.page = index;
                self.raster = None;
            }
        }
    }

    /// States section: one row per state of the tallest layer.
    fn states_section(&mut self, ui: &mut egui::Ui) {
        let states = self
            .editor
            .document()
            .pages
            .get(self.editor.page)
            .map_or(1, |page| {
                page.layers
                    .layers
                    .iter()
                    .map(|layer| layer.cells.len())
                    .max()
                    .unwrap_or(1)
            });
        for frame in 0..states {
            if ui
                .selectable_label(self.editor.frame == frame, format!("State {}", frame + 1))
                .clicked()
            {
                self.editor.frame = frame;
                self.raster = None;
            }
        }
    }

    /// Layers section: name + display toggle per layer, bottom-first.
    /// The active layer's row is highlighted and can be changed by
    /// clicking it — that row is where every placement command lands
    /// its object.
    fn layers_section(&mut self, ui: &mut egui::Ui) {
        // Snapshot what the panel shows so closures never borrow the
        // document while the editor is mutable.
        let Some(page) = self.editor.document().pages.get(self.editor.page) else {
            return;
        };
        let frame = self.editor.frame;
        let layers: Vec<(String, bool, bool)> = page
            .layers
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| {
                let label = if layer.name.is_empty() {
                    format!("Layer {}", index + 1)
                } else {
                    layer.name.clone()
                };
                // The lock is per state (CS6 locks a layer's cell in
                // one frame), so the row reads the current frame.
                let locked = layer.cells.get(frame).is_some_and(|cell| cell.locked);
                (label, layer.display, locked)
            })
            .collect();
        let count = layers.len();
        let active = self.editor.active_layer();
        for (layer_index, (label, display, locked)) in layers.iter().enumerate() {
            let mut rename = label.clone();
            ui.horizontal(|ui| {
                let eye = if *display { "👁" } else { "·" };
                if ui
                    .button(eye)
                    .on_hover_text("Toggle layer display")
                    .clicked()
                {
                    let _ = self.editor.apply(
                        "layer.setDisplay",
                        serde_json::json!({
                            "page": self.editor.page,
                            "layer": layer_index,
                            "display": !display,
                        }),
                    );
                }
                let lock = if *locked { "🔒" } else { "🔓" };
                if ui
                    .button(lock)
                    .on_hover_text("Lock this layer in the current state")
                    .clicked()
                {
                    let _ = self.editor.apply(
                        "layer.setLocked",
                        serde_json::json!({
                            "page": self.editor.page,
                            "layer": layer_index,
                            "frame": frame,
                            "locked": !locked,
                        }),
                    );
                }
                if self.rename_layer == Some(layer_index) {
                    let field = ui
                        .add(egui::TextEdit::singleline(&mut rename).desired_width(72.0))
                        .on_hover_text("rename (Enter applies)");
                    let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
                    if (field.lost_focus() || enter) && rename != *label {
                        self.run_command(
                            "layer.rename",
                            serde_json::json!({
                                "page": self.editor.page,
                                "layer": layer_index,
                                "name": rename,
                            }),
                        );
                    }
                    if field.lost_focus() || enter {
                        self.rename_layer = None;
                    }
                } else if ui
                    .selectable_label(active == layer_index, label)
                    .on_hover_text("Make this the active layer")
                    .clicked()
                {
                    self.editor.set_active_layer(layer_index);
                }
                if ui
                    .small_button("✎")
                    .on_hover_text("Rename this layer")
                    .clicked()
                {
                    self.rename_layer = Some(layer_index);
                }
                if ui
                    .small_button("↑")
                    .on_hover_text("Move this layer up")
                    .clicked()
                {
                    self.reorder_layer(layer_index, true);
                }
                if ui
                    .small_button("↓")
                    .on_hover_text("Move this layer down")
                    .clicked()
                {
                    self.reorder_layer(layer_index, false);
                }
            });
        }
        ui.horizontal(|ui| {
            if ui
                .button("+")
                .on_hover_text("Add a layer above the active one")
                .clicked()
            {
                self.run_command(
                    "layer.add",
                    serde_json::json!({ "page": self.editor.page, "index": null }),
                );
                let top = self.editor.layer_count().saturating_sub(1);
                self.editor.set_active_layer(top);
            }
            if ui
                .button("−")
                .on_hover_text("Delete the active layer")
                .clicked()
                && count > 1
            {
                self.run_command(
                    "layer.delete",
                    serde_json::json!({ "page": self.editor.page, "layer": active }),
                );
                self.editor.set_active_layer(active.saturating_sub(1));
            }
        });
    }

    /// Layers-panel reorder: one slot up (toward the top of the
    /// stack) or down. `before` is the pre-removal index the moved
    /// layer lands in front of, so moving up one slot is `index + 2`.
    fn reorder_layer(&mut self, index: usize, up: bool) {
        let count = self.editor.layer_count();
        if count == 0 {
            return;
        }
        let before = if up {
            (index + 2).min(count)
        } else {
            index.saturating_sub(1)
        };
        if before == index || before == index + 1 {
            return; // already at the end of the stack
        }
        let payload = serde_json::json!({
            "page": self.editor.page,
            "from": index,
            "before": before,
        });
        match self.editor.apply("layer.reorder", payload) {
            Ok(()) => {
                // The moved layer keeps the active role; its index
                // shifts by one slot in the direction of the move.
                let moved = if up { index + 1 } else { index - 1 };
                self.editor.set_active_layer(moved);
            }
            Err(error) => self.status = format!("layer.reorder failed: {error}"),
        }
    }

    /// History section: journal rows plus the steps save/replay pair.
    fn history_section(&mut self, ui: &mut egui::Ui) {
        // Snapshot the rows so closures never borrow the session while
        // the editor is mutable (same rule as the layer list above).
        let rows: Vec<String> = self
            .editor
            .session
            .journal_entries()
            .iter()
            .map(|entry| match &entry.txn {
                Some(txn) => format!("{}. {txn}: {}", entry.seq, entry.name),
                None => format!("{}. {}", entry.seq, entry.name),
            })
            .collect();
        egui::ScrollArea::vertical()
            .max_height(160.0)
            .show(ui, |ui| {
                for row in &rows {
                    ui.label(row);
                }
            });
        ui.horizontal(|ui| {
            if ui
                .button("Save Steps…")
                .on_hover_text("Write the journal as a .fwsteps command file")
                .clicked()
            {
                self.save_steps();
            }
            if ui
                .button("Replay…")
                .on_hover_text("Rebuild a session from a steps file over a base document")
                .clicked()
            {
                self.replay_steps();
            }
        });
    }

    /// The document library: one row per definition — click enters
    /// symbol editing, drag places an instance on the canvas, the
    /// field renames, the button removes (refused by the engine while
    /// instances remain). While a definition is entered, the banner
    /// says so and offers the way out.
    /// The Styles panel: the document style library (`MKBv`'s `ZSLv`).
    /// A row click applies the bundle onto the selection
    /// (`style.apply` — the CS6 panel's apply), the name field defines
    /// a new (or replaces a same-named) style from the selection's
    /// appearance, and Import merges a CS6 `.stl` library through
    /// [`Editor::import_styles`].
    fn styles_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Styles");
        // (index, label) — snapshotted so closures never borrow the
        // document while the editor is mutable (the panel rule).
        let rows: Vec<(usize, String)> = self
            .editor
            .document()
            .styles
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let label = if entry.kind == 0x1f {
                    format!("{} · text", entry.name)
                } else {
                    entry.name.clone()
                };
                (index, label)
            })
            .collect();
        let targets: Vec<doc::ObjectId> = self.editor.selection_set.clone();
        for (index, label) in &rows {
            if ui
                .selectable_label(false, label)
                .on_hover_text("apply onto the selection")
                .clicked()
            {
                self.run_command(
                    "style.apply",
                    serde_json::json!({ "index": index, "ids": targets }),
                );
            }
        }
        if rows.is_empty() {
            ui.label("(empty — define one from a selection or import a library)");
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("name");
            ui.add(egui::TextEdit::singleline(&mut self.style_name).desired_width(96.0));
            if ui
                .button("New From Selection")
                .on_hover_text("redefines a style of this name, or defines a new one")
                .clicked()
                && let Some(source) = targets.last().copied()
                && !self.style_name.is_empty()
            {
                self.run_command(
                    "style.redefineFromSelection",
                    serde_json::json!({ "name": self.style_name, "source": source }),
                );
            }
        });
        if ui.button("Import Style Library...").clicked() {
            self.import_styles_dialog();
        }
    }

    /// Styles panel ▸ Import: reads a CS6 style library (`.stl`)
    /// through the injected traits; entries join the document library,
    /// their preserved texture subtrees join the editor bag.
    fn import_styles_dialog(&mut self) {
        let Some(path) = self.dialogs.open_path(
            "Import style library",
            &[("Fireworks style library".to_owned(), vec!["stl".to_owned()])],
        ) else {
            return;
        };
        let bytes = match self.fs.read_file(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("import failed: {error}");
                return;
            }
        };
        match self.editor.import_styles(&bytes) {
            Ok(count) => self.status = format!("imported {count} styles from {path}"),
            Err(error) => self.status = format!("import failed: {error}"),
        }
    }

    fn library_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Library");
        let editing = self.editor.editing_definition().map(|d| d.name.clone());
        if let Some(name) = editing {
            ui.horizontal(|ui| {
                ui.label(format!("editing {name}"));
                if ui.button("Exit Symbol").clicked() {
                    self.editor.exit_symbol();
                }
            });
        }
        // (number, name, kind label, states, objects in state 1,
        // entered) — snapshotted so closures never borrow the document
        // while the editor is mutable (the panel rule).
        let rows: Vec<(i64, String, String, i64, usize, bool)> = self
            .editor
            .document()
            .symbols
            .iter()
            .map(|definition| {
                (
                    definition.symbol_number,
                    definition.name.clone(),
                    symbol_type_label(&definition.symbol_type),
                    definition.frame_count,
                    definition.layers.layers.first().map_or(0, |layer| {
                        layer.cells.first().map_or(0, |cell| cell.elements.len())
                    }),
                    self.editor.symbol_scope == Some(definition.symbol_number),
                )
            })
            .collect();
        for (number, name, kind, states, parts, entered) in &rows {
            ui.horizontal(|ui| {
                let row = ui.selectable_label(*entered, format!("{name} · {kind} · {states}"));
                if row.drag_started() {
                    // Drag-to-place: the canvas is the drop target.
                    egui::DragAndDrop::set_payload(ui.ctx(), *number);
                }
                if row.clicked()
                    && let Err(error) = self.editor.enter_symbol(*number)
                {
                    self.status = format!("enter symbol failed: {error}");
                }
                let mut edited = name.clone();
                let field = ui
                    .add(egui::TextEdit::singleline(&mut edited).desired_width(72.0))
                    .on_hover_text("rename (Enter applies)");
                if field.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    && edited != *name
                {
                    self.run_command(
                        "library.rename",
                        serde_json::json!({ "symbol": number, "name": edited }),
                    );
                }
                if ui
                    .small_button("remove")
                    .on_hover_text(format!("remove {name} from the library"))
                    .clicked()
                {
                    self.run_command("library.remove", serde_json::json!({ "symbol": number }));
                }
            });
            ui.label(format!("{} object(s) in state 1", parts));
        }
        if ui.button("New Symbol").clicked() {
            let count = self.editor.document().symbols.len();
            self.run_command(
                "library.add",
                serde_json::json!({
                    "name": format!("Symbol {}", count + 1),
                    "symbol_type": "graphic",
                }),
            );
        }
        if rows.is_empty() {
            ui.label("(empty — create a symbol from a selection)");
        }
    }

    /// Right panel: the align/distribute group, then properties of the
    /// selection.
    fn properties_panel(&mut self, ui: &mut egui::Ui) {
        if matches!(
            self.editor.tool,
            Tool::Paintbrush
                | Tool::Pencil
                | Tool::Eraser
                | Tool::PaintBucket
                | Tool::Gradient
                | Tool::Eyedropper
                | Tool::Marquee
                | Tool::EllipticalMarquee
                | Tool::Lasso
                | Tool::PolygonalMarquee
                | Tool::MagicWand
        ) {
            self.brush_panel(ui);
            ui.separator();
        }
        if matches!(
            self.editor.tool,
            Tool::Marquee
                | Tool::EllipticalMarquee
                | Tool::Lasso
                | Tool::PolygonalMarquee
                | Tool::MagicWand
        ) {
            self.selection_panel(ui);
            ui.separator();
        }
        ui.heading("Align");
        let ids = self.editor.live_selection();
        for row in [
            [
                ("left", "Left"),
                ("centerX", "Center X"),
                ("right", "Right"),
            ],
            [
                ("top", "Top"),
                ("centerY", "Center Y"),
                ("bottom", "Bottom"),
            ],
        ] {
            ui.horizontal(|ui| {
                for (mode, label) in row {
                    if ui
                        .add_enabled(ids.len() >= 2, egui::Button::new(label).small())
                        .clicked()
                    {
                        self.run_command(
                            "object.align",
                            serde_json::json!({ "ids": ids, "mode": mode }),
                        );
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            for (axis, label) in [("horizontal", "Space H"), ("vertical", "Space V")] {
                if ui
                    .add_enabled(ids.len() >= 3, egui::Button::new(label).small())
                    .clicked()
                {
                    self.run_command(
                        "object.distribute",
                        serde_json::json!({ "ids": ids, "axis": axis }),
                    );
                }
            }
        });
        ui.separator();

        ui.heading("Properties");
        let Some(id) = self.editor.selection else {
            ui.label("nothing selected");
            return;
        };
        let object = self.editor.document().registry.get(id).cloned();
        let effects = match object {
            Some(ObjectData::Path(path)) => {
                self.path_properties(ui, id, &path);
                path.effects
            }
            Some(ObjectData::Text(text)) => {
                self.text_properties(ui, id, &text);
                text.effects
            }
            Some(ObjectData::SymbolInstance(instance)) => {
                self.symbol_instance_properties(ui, id, &instance);
                return;
            }
            Some(_) => {
                ui.label("selection has no editable properties yet");
                return;
            }
            None => {
                ui.label("stale selection");
                return;
            }
        };
        // The live effect list: every kind that carries an `EFLv` spine.
        self.filter_panel(ui, id, &effects);
    }

    /// The paint tools' slots: the Edit Brush dialog's size, edge,
    /// shape, aspect, angle, pressure and strength, plus the paint
    /// color. The canvas paints with exactly these — the engine reads
    /// them from every stroke's payload.
    fn brush_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Brush");
        let brush = self.editor.brush;
        let mut size = f32::from(u16::try_from(brush.size.min(1024)).unwrap_or(u16::MAX));
        if ui
            .add(egui::Slider::new(&mut size, 1.0..=1024.0).text("size"))
            .changed()
        {
            self.editor.brush.size = size.round().clamp(1.0, 1024.0) as u32;
        }
        let mut edge = f32::from(brush.edge);
        if ui
            .add(egui::Slider::new(&mut edge, 0.0..=100.0).text("edge"))
            .changed()
        {
            self.editor.brush.edge = edge.round().clamp(0.0, 100.0) as u8;
        }
        let mut aspect = f32::from(brush.aspect);
        if ui
            .add(egui::Slider::new(&mut aspect, 1.0..=100.0).text("aspect"))
            .changed()
        {
            self.editor.brush.aspect = aspect.round().clamp(1.0, 100.0) as u8;
        }
        let mut angle = f32::from(brush.angle);
        if ui
            .add(egui::Slider::new(&mut angle, -180.0..=180.0).text("angle"))
            .changed()
        {
            self.editor.brush.angle = angle.round().clamp(-180.0, 180.0) as i16;
        }
        let mut pressure = f32::from(brush.pressure);
        if ui
            .add(egui::Slider::new(&mut pressure, 1.0..=100.0).text("pressure"))
            .changed()
        {
            self.editor.brush.pressure = pressure.round().clamp(1.0, 100.0) as u8;
        }
        let mut strength = f32::from(brush.strength);
        if ui
            .add(egui::Slider::new(&mut strength, 1.0..=100.0).text("strength"))
            .changed()
        {
            self.editor.brush.strength = strength.round().clamp(1.0, 100.0) as u8;
        }
        ui.horizontal(|ui| {
            ui.label("shape");
            for (shape, label) in [(StampShape::Round, "round"), (StampShape::Square, "square")] {
                if ui.selectable_label(brush.shape == shape, label).clicked() {
                    self.editor.brush.shape = shape;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("color");
            let mut color = egui::Color32::from_rgba_unmultiplied(
                brush.color[0],
                brush.color[1],
                brush.color[2],
                brush.color[3],
            );
            if ui.color_edit_button_srgba(&mut color).changed() {
                self.editor.brush.color = [color.r(), color.g(), color.b(), color.a()];
            }
        });
        if matches!(self.editor.tool, Tool::MagicWand) {
            let mut tolerance = f32::from(self.editor.wand_tolerance);
            if ui
                .add(egui::Slider::new(&mut tolerance, 0.0..=255.0).text("tolerance"))
                .changed()
            {
                self.editor.wand_tolerance = tolerance.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    /// The pixel-selection tools' panel: the mask's presence and
    /// statistics, the mode words, and the feather / adjust / move /
    /// erase actions on it.
    fn selection_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Select");
        let active = self.editor.has_pixel_selection();
        if active {
            let (pixels, coverage) =
                self.editor
                    .document()
                    .selection
                    .as_ref()
                    .map_or((0, 0), |mask| {
                        (
                            mask.alpha.iter().filter(|&&a| a != 0).count(),
                            u64::from(mask.alpha.iter().map(|&a| u16::from(a)).sum::<u16>()) * 100
                                / (255 * u64::try_from(mask.alpha.len().max(1)).unwrap_or(1)),
                        )
                    });
            ui.label(format!("{pixels} pixels selected ({coverage}% coverage)"));
        } else {
            ui.label("no pixel selection");
        }
        ui.horizontal(|ui| {
            ui.label("mode");
            for (mode, label) in [
                (PixelSelectMode::Replace, "new"),
                (PixelSelectMode::Add, "add"),
                (PixelSelectMode::Subtract, "subtract"),
                (PixelSelectMode::Intersect, "intersect"),
            ] {
                if ui
                    .selectable_label(self.editor.pixel_mode == mode, label)
                    .clicked()
                {
                    self.editor.pixel_mode = mode;
                }
            }
        });
        ui.separator();
        let mut feather = f32::from(self.editor.feather_radius);
        if ui
            .add(egui::Slider::new(&mut feather, 1.0..=64.0).text("feather"))
            .changed()
        {
            self.editor.feather_radius = feather.round().clamp(1.0, 64.0) as u8;
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(active, egui::Button::new("feather").small())
                .clicked()
            {
                let radius = self.editor.feather_radius;
                if let Err(error) = self.editor.feather_selection(radius) {
                    self.status = format!("feather failed: {error}");
                }
            }
            if ui
                .add_enabled(active, egui::Button::new("deselect").small())
                .clicked()
                && let Err(error) = self.editor.deselect_pixels()
            {
                self.status = format!("deselect failed: {error}");
            }
        });
        ui.horizontal(|ui| {
            for (what, label) in [
                ("expand", "grow"),
                ("contract", "shrink"),
                ("smooth", "smooth"),
            ] {
                if ui
                    .add_enabled(active, egui::Button::new(label).small())
                    .clicked()
                    && let Err(error) = self.editor.adjust_pixel_selection(what, 1)
                {
                    self.status = format!("{label} failed: {error}");
                }
            }
        });
        if matches!(self.editor.tool, Tool::MagicWand) && ui.button("select similar").clicked() {
            let tolerance = self.editor.wand_tolerance;
            if let Err(error) = self.editor.select_similar(tolerance) {
                self.status = format!("similar failed: {error}");
            }
        }
        ui.separator();
        ui.horizontal(|ui| {
            for (dx, dy, label) in [(-1, 0, "◀"), (0, -1, "▲"), (0, 1, "▼"), (1, 0, "▶")] {
                if ui
                    .add_enabled(active, egui::Button::new(label).small())
                    .clicked()
                {
                    if ui.input(|i| i.modifiers.shift) {
                        // Shift moves the pixels; the plain arrows
                        // move the selection frame.
                        let target_move = (dx, dy);
                        if let Err(error) = self
                            .editor
                            .move_selected_pixels(target_move.0, target_move.1)
                        {
                            self.status = format!("move failed: {error}");
                        }
                    } else if let Err(error) = self.editor.move_selection_mask(dx, dy) {
                        self.status = format!("move failed: {error}");
                    }
                }
            }
        });
        if ui
            .add_enabled(active, egui::Button::new("erase pixels").small())
            .clicked()
            && let Err(error) = self.editor.erase_selected()
        {
            self.status = format!("erase failed: {error}");
        }
    }

    /// Symbol-instance properties: the referenced definition, the
    /// button-state inspector (per-state content), the 9-slice
    /// toggle, and the actions on the instance or its definition.
    fn symbol_instance_properties(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        instance: &SymbolInstanceObject,
    ) {
        let number = instance.symbol_number;
        // Snapshot the definition facts so closures never borrow the
        // document while the editor is mutable.
        let facts = self.editor.definition_of(number).map(|definition| {
            (
                definition.name.clone(),
                symbol_type_label(&definition.symbol_type),
                definition.symbol_type,
                definition
                    .layers
                    .layers
                    .first()
                    .map_or_else(Vec::new, |layer| {
                        layer.cells.iter().map(|cell| cell.elements.len()).collect()
                    }),
                definition.nine_slice.is_some(),
            )
        });
        let Some((name, kind, kind_raw, state_parts, sliced)) = facts else {
            ui.label(format!("instance of missing symbol {number}"));
            return;
        };
        ui.label(format!("instance of {name} (symbol {number}, {kind})"));
        let placed = instance.transform.translation();
        ui.label(format!("placed at ({:.1}, {:.1})", placed.x, placed.y));

        ui.horizontal(|ui| {
            if ui.button("Edit Definition").clicked()
                && let Err(error) = self.editor.enter_symbol(number)
            {
                self.status = format!("enter symbol failed: {error}");
            }
            if ui
                .button("Break Apart")
                .on_hover_text("release the instance's content onto the page")
                .clicked()
            {
                self.run_command("symbol.breakApart", serde_json::json!({ "id": id }));
            }
        });

        // The type switches (button-state inspector wiring): the
        // engine keeps per-state content, the UI offers the observed
        // kinds.
        ui.horizontal(|ui| {
            ui.label("type");
            for (choice, label) in [
                ("graphic", "graphic"),
                ("button", "button"),
                ("animation", "animation"),
            ] {
                let current = match &kind_raw {
                    SymbolType::Graphic => "graphic",
                    SymbolType::Button => "button",
                    SymbolType::Animation => "animation",
                    SymbolType::Other(_) => "other",
                };
                if ui.selectable_label(current == choice, label).clicked() {
                    self.run_command(
                        "symbol.setType",
                        serde_json::json!({ "symbol": number, "symbol_type": choice }),
                    );
                }
            }
        });

        // States: one row per definition state with its object count —
        // the button inspector's read side.
        ui.heading("States");
        if state_parts.is_empty() {
            ui.label("(the definition has no states)");
        }
        for (index, parts) in state_parts.iter().enumerate() {
            ui.label(format!("State {}: {} object(s)", index + 1, parts));
        }

        // 9-slice guides: install defaults / clear, drag the on-canvas
        // handles to set the insets.
        let mut sliced_flag = sliced;
        if ui.checkbox(&mut sliced_flag, "nine-slice guides").changed() && sliced_flag != sliced {
            let outcome = if sliced_flag {
                self.editor.enable_nine_slice(number)
            } else {
                self.editor.disable_nine_slice(number)
            };
            if let Err(error) = outcome {
                self.status = format!("nine-slice failed: {error}");
            }
        }
    }

    /// Path-object properties, then the shared display controls.
    fn path_properties(&mut self, ui: &mut egui::Ui, id: doc::ObjectId, path: &PathObject) {
        ui.label(format!("object {} (gen {})", id.index(), id.generation()));
        self.display_controls(
            ui,
            id,
            DisplayState {
                opacity: path.opacity,
                blend: path.blend,
                visible: path.visible,
                locked: path.locked,
                name: path.name.clone(),
            },
        );
    }

    /// Text-object properties: characters, font, size, style flags,
    /// then the shared display controls.
    fn text_properties(&mut self, ui: &mut egui::Ui, id: doc::ObjectId, text: &TextObject) {
        ui.label(format!("object {} (gen {})", id.index(), id.generation()));
        // Characters: one run, applied on Enter like the name field.
        let mut content = text
            .runs
            .first()
            .map(|run| run.text.clone())
            .unwrap_or_default();
        let field =
            ui.add(egui::TextEdit::singleline(&mut content).hint_text("text (Enter applies)"));
        if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            let _ = self.editor.apply(
                "text.setText",
                serde_json::json!({ "id": id, "text": content }),
            );
        }
        // The family name round-trips verbatim; unknown families render
        // in the bundled sans, so the list is a convenience, not a
        // whitelist.
        let font = text
            .style
            .as_ref()
            .and_then(|style| style.font.clone())
            .unwrap_or_else(|| "Liberation Sans".to_owned());
        egui::ComboBox::from_label("font")
            .selected_text(font.clone())
            .show_ui(ui, |ui| {
                for family in [
                    "Liberation Sans",
                    "Arial",
                    "Verdana",
                    "Times New Roman",
                    "Courier New",
                ] {
                    if ui.selectable_label(font == family, family).clicked() {
                        let _ = self.editor.apply(
                            "text.setFont",
                            serde_json::json!({ "id": id, "font": family }),
                        );
                    }
                }
            });
        let mut size = text
            .style
            .as_ref()
            .and_then(|style| style.size)
            .unwrap_or(12.0);
        if ui
            .add(egui::Slider::new(&mut size, 4.0..=200.0).text("size"))
            .changed()
        {
            let _ = self.editor.apply(
                "text.setAttrs",
                serde_json::json!({ "id": id, "size": size }),
            );
        }
        ui.horizontal(|ui| {
            let mut bold = text
                .style
                .as_ref()
                .and_then(|style| style.bold)
                .unwrap_or(false);
            if ui.checkbox(&mut bold, "bold").changed() {
                let _ = self.editor.apply(
                    "text.setAttrs",
                    serde_json::json!({ "id": id, "bold": bold }),
                );
            }
            let mut italic = text
                .style
                .as_ref()
                .and_then(|style| style.italic)
                .unwrap_or(false);
            if ui.checkbox(&mut italic, "italic").changed() {
                let _ = self.editor.apply(
                    "text.setAttrs",
                    serde_json::json!({ "id": id, "italic": italic }),
                );
            }
            let mut underline = text
                .style
                .as_ref()
                .and_then(|style| style.underline)
                .unwrap_or(false);
            if ui.checkbox(&mut underline, "underline").changed() {
                let _ = self.editor.apply(
                    "text.setAttrs",
                    serde_json::json!({ "id": id, "underline": underline }),
                );
            }
        });
        self.display_controls(
            ui,
            id,
            DisplayState {
                opacity: text.opacity,
                blend: text.blend,
                visible: text.visible,
                locked: text.locked,
                name: text.name.clone(),
            },
        );
    }

    /// The display quartet plus the object name — shared by every
    /// property-bearing kind.
    fn display_controls(&mut self, ui: &mut egui::Ui, id: doc::ObjectId, state: DisplayState) {
        let DisplayState {
            opacity,
            blend,
            visible,
            locked,
            name,
        } = state;
        // Opacity, per mille.
        let mut opacity = opacity;
        if ui
            .add(egui::Slider::new(&mut opacity, 0..=1000).text("opacity"))
            .changed()
        {
            let _ = self.editor.apply(
                "object.setOpacity",
                serde_json::json!({ "id": id, "opacity": opacity }),
            );
        }
        // Blend mode, indexed into the UI-order table.
        let blend_names: Vec<&str> = color::BlendMode::ALL
            .iter()
            .map(|mode| mode.as_str())
            .collect();
        let blend = (blend as usize).min(blend_names.len() - 1);
        egui::ComboBox::from_label("blend")
            .selected_text(blend_names[blend])
            .show_ui(ui, |ui| {
                for (index, mode) in blend_names.iter().enumerate() {
                    if ui.selectable_label(index == blend, *mode).clicked() {
                        let _ = self.editor.apply(
                            "object.setBlend",
                            serde_json::json!({ "id": id, "blend": index }),
                        );
                    }
                }
            });
        // Visibility / lock.
        ui.horizontal(|ui| {
            if ui.checkbox(&mut { visible }, "visible").changed() {
                let _ = self.editor.apply(
                    "object.setVisible",
                    serde_json::json!({ "id": id, "visible": !visible }),
                );
            }
            if ui.checkbox(&mut { locked }, "locked").changed() {
                let _ = self.editor.apply(
                    "object.setLocked",
                    serde_json::json!({ "id": id, "locked": !locked }),
                );
            }
        });
        // Name.
        let mut name = name.unwrap_or_default();
        if ui
            .add(egui::TextEdit::singleline(&mut name).hint_text("object name"))
            .lost_focus()
            && ui.input(|input| input.key_pressed(egui::Key::Enter))
        {
            let value = if name.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::json!(name)
            };
            let _ = self.editor.apply(
                "object.rename",
                serde_json::json!({ "id": id, "name": value }),
            );
        }
    }

    /// The live effect list: one collapsible row per effect (remove +
    /// per-parameter edit rows), plus the family-grouped quick-add
    /// menu over the shared preset registry. Live preview needs
    /// nothing here — every command below invalidates the compose
    /// stamp and the canvas re-renders through the live stack.
    fn filter_panel(&mut self, ui: &mut egui::Ui, id: doc::ObjectId, effects: &[doc::Effect]) {
        ui.heading("Filters");
        if effects.is_empty() {
            ui.label("(none)");
        }
        for (index, effect) in effects.iter().enumerate() {
            egui::CollapsingHeader::new(format!("{}. {}", index + 1, effect.internal_name))
                .id_salt(("filter", id, index))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.small_button("remove").clicked() {
                            self.run_command(
                                "filter.remove",
                                serde_json::json!({ "id": id, "index": index }),
                            );
                        }
                    });
                    self.filter_param_rows(ui, id, index, effect);
                });
        }
        self.filter_add_menu(ui, id);
    }

    /// The family-grouped quick-add menu — the same table the Filters
    /// menu submenu casts as `Action::Filter` entries.
    fn filter_add_menu(&mut self, ui: &mut egui::Ui, id: doc::ObjectId) {
        ui.menu_button("add filter", |ui| {
            for (family, presets) in FILTER_FAMILIES {
                ui.menu_button(*family, |ui| {
                    for preset in *presets {
                        if ui.selectable_label(false, preset.label).clicked() {
                            self.apply_filter_preset(id, *preset);
                        }
                    }
                });
            }
        });
    }

    /// One quick-add: `filter.add` with the preset's `DCVs` defaults
    /// (curves ride the raw knot list — `value` stays empty there).
    fn apply_filter_preset(&mut self, id: doc::ObjectId, preset: FilterPreset) {
        let params: Vec<_> = preset
            .params
            .iter()
            .map(|(key, value)| {
                let mut entry = serde_json::json!({ "key": key, "value": value });
                if !preset.points.is_empty() {
                    entry["points"] = serde_json::json!(preset.points);
                }
                entry
            })
            .collect();
        self.run_command(
            "filter.add",
            serde_json::json!({ "id": id, "name": preset.name, "params": params }),
        );
        self.status = format!("filter: {}", preset.label);
    }

    /// One parameter edit on an applied effect.
    fn set_filter_param(&mut self, id: doc::ObjectId, index: usize, key: &str, value: &str) {
        self.run_command(
            "filter.setParam",
            serde_json::json!({ "id": id, "index": index, "key": key, "value": value }),
        );
    }

    /// The per-effect parameter rows: documented numeric keys get
    /// range sliders with a reset, boolean keys checkboxes, knot lists
    /// a read-only count with a reset-to-linear, everything else a
    /// text row (colors keep their `#rrggbb` text shape — the same
    /// form the file carries). Each edit is one `filter.setParam`
    /// through the journal, so undo undoes it exactly.
    fn filter_param_rows(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        index: usize,
        effect: &doc::Effect,
    ) {
        for property in &effect.properties {
            let key = property.key.as_str();
            if key == "EffectMoaID" {
                continue;
            }
            if key.ends_with("_points") {
                // Curve knots live as a raw `GPLv` node (or legacy
                // text); the rows show the count and a reset to the
                // identity line — full curve editing stays in the
                // engine's `points` JSON until a curve canvas exists.
                let knots = match &property.value {
                    doc::EffectValue::Raw(node) => node.as_children().map_or(0, <[doc::Node]>::len),
                    doc::EffectValue::Text(text) => text.split(';').count(),
                };
                ui.horizontal(|ui| {
                    ui.label(format!("{key}: {knots} knots"));
                    if ui.small_button("reset curve").clicked() {
                        let _ = self.editor.apply(
                            "filter.setParam",
                            serde_json::json!({
                                "id": id, "index": index, "key": key,
                                "value": "",
                                "points": [[0.0, 0.0], [255.0, 255.0]],
                            }),
                        );
                    }
                });
                continue;
            }
            let doc::EffectValue::Text(text) = &property.value else {
                // Raw nodes render read-only — a misread is not a value.
                ui.label(format!("{key}: (raw node)"));
                continue;
            };
            if FILTER_BOOL_KEYS.contains(&key) {
                let mut on = !matches!(text.trim(), "0" | "false");
                if ui.checkbox(&mut on, key).changed() {
                    self.set_filter_param(id, index, key, if on { "1" } else { "0" });
                }
                continue;
            }
            let Some(meta) = FILTER_PARAMS.iter().find(|meta| meta.key == key) else {
                // Text rows (colors, blend codes, file-borne keys):
                // commit on enter/blur so a keystroke is not a journal
                // entry.
                let mut edit = text.clone();
                let field = egui::TextEdit::singleline(&mut edit).desired_width(140.0);
                let response = ui.add(field);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.set_filter_param(id, index, key, &edit);
                } else {
                    ui.label(key);
                }
                continue;
            };
            let Ok(mut value) = text.trim().parse::<f64>() else {
                ui.label(format!("{key}: {text}"));
                continue;
            };
            ui.horizontal(|ui| {
                let slider =
                    ui.add(egui::Slider::new(&mut value, meta.min..=meta.max).text(meta.label));
                if slider.changed() {
                    self.set_filter_param(id, index, key, &format!("{value}"));
                }
                if ui.small_button("reset").clicked() {
                    self.set_filter_param(id, index, key, &format!("{}", meta.default));
                }
            });
        }
    }

    /// World-space page bounds — the Fit All target.
    fn page_world(&self) -> Option<geom::Rect> {
        self.editor
            .document()
            .pages
            .get(self.editor.page)
            .map(|page| {
                geom::Rect::new(
                    page.info.canvas.x,
                    page.info.canvas.y,
                    page.info.canvas.x + page.info.canvas.width,
                    page.info.canvas.y + page.info.canvas.height,
                )
            })
    }

    /// The CS6 tool strip: vertical, grouped, glyph buttons painted
    /// with `Painter` primitives (original art, never the Adobe icons).
    fn tools_panel(&mut self, ui: &mut egui::Ui) {
        let groups: &[&[Tool]] = &[
            &[Tool::Pointer, Tool::Subselect],
            &[Tool::Rect, Tool::Ellipse, Tool::Line, Tool::Text],
            &[
                Tool::Paintbrush,
                Tool::Pencil,
                Tool::Eraser,
                Tool::PaintBucket,
                Tool::Gradient,
                Tool::Eyedropper,
            ],
            &[
                Tool::Marquee,
                Tool::EllipticalMarquee,
                Tool::Lasso,
                Tool::PolygonalMarquee,
                Tool::MagicWand,
            ],
            &[Tool::Slice, Tool::Hotspot],
            &[Tool::Zoom, Tool::Hand],
        ];
        ui.add_space(4.0);
        for (index, group) in groups.iter().enumerate() {
            if index > 0 {
                ui.separator();
            }
            for tool in *group {
                self.tool_button(ui, *tool);
            }
        }
        ui.separator();
        self.autoshape_button(ui);
    }

    /// One painted tool button; clicking picks the tool.
    fn tool_button(&mut self, ui: &mut egui::Ui, tool: Tool) {
        let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(26.0), Sense::click());
        let active = self.editor.tool == tool;
        if response.clicked() {
            self.editor.tool = tool;
            self.status = format!("tool: {}", tool_label(tool));
        }
        // CS6's tool-button double-clicks: Zoom resets to 100%, Hand
        // fits the page in the window.
        if response.double_clicked() {
            match tool {
                Tool::Zoom => self.run_shell(menus::ShellAction::ZoomTo(1.0)),
                Tool::Hand => self.run_shell(menus::ShellAction::FitAll),
                _ => {}
            }
        }
        let painter = ui.painter();
        if active {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x2f, 0x5d, 0x8a));
        } else if response.hovered() {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
        }
        let ink = if active {
            Color32::from_rgb(0xe8, 0xe8, 0xe8)
        } else {
            Color32::from_rgb(0xc4, 0xc4, 0xc4)
        };
        draw_tool_glyph(painter, rect, tool, ink);
        let key = tool_key(tool);
        if key.is_empty() {
            response.on_hover_text(tool_label(tool));
        } else {
            response.on_hover_text(format!("{} ({key})", tool_label(tool)));
        }
    }

    /// The Auto Shape flyout: the star button toggles a popup listing
    /// every shape with its preview glyph; picking one selects the
    /// tool and closes the flyout.
    fn autoshape_button(&mut self, ui: &mut egui::Ui) {
        let shapes = AutoShape::ALL;
        let current = match self.editor.tool {
            Tool::AutoShape(shape) => Some(shape),
            _ => None,
        };
        let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(26.0), Sense::click());
        let mut open = self.autoshape_flyout;
        if response.clicked() {
            open = !open;
            if open && current.is_none() {
                self.editor.tool = Tool::AutoShape(shapes[0]);
            }
        }
        let painter = ui.painter();
        let active = current.is_some();
        if active {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x2f, 0x5d, 0x8a));
        } else if response.hovered() {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
        }
        let ink = if active {
            Color32::from_rgb(0xe8, 0xe8, 0xe8)
        } else {
            Color32::from_rgb(0xc4, 0xc4, 0xc4)
        };
        let shown = current.unwrap_or(shapes[0]);
        draw_tool_glyph(painter, rect, Tool::AutoShape(shown), ink);
        let tip = match current {
            Some(shape) => format!("Auto Shape — {}", shape.label()),
            None => "Auto Shape".to_owned(),
        };
        let response = response.on_hover_text(tip);

        let mut picked: Option<AutoShape> = None;
        egui::Popup::from_response(&response)
            .open_bool(&mut open)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                for shape in shapes {
                    let (row, row_response) =
                        ui.allocate_exact_size(egui::Vec2::new(150.0, 20.0), Sense::click());
                    let selected = current == Some(shape);
                    let rp = ui.painter();
                    if selected {
                        rp.rect_filled(row, 2.0, Color32::from_rgb(0x2f, 0x5d, 0x8a));
                    } else if row_response.hovered() {
                        rp.rect_filled(row, 2.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
                    }
                    let ink = if selected || row_response.hovered() {
                        Color32::from_rgb(0xe8, 0xe8, 0xe8)
                    } else {
                        Color32::from_rgb(0xdd, 0xdd, 0xdd)
                    };
                    draw_autoshape_glyph(
                        rp,
                        Rect::from_min_size(
                            Pos2::new(row.left() + 4.0, row.center().y - 8.0),
                            egui::Vec2::splat(16.0),
                        ),
                        shape,
                        ink,
                    );
                    rp.text(
                        Pos2::new(row.left() + 26.0, row.center().y),
                        egui::Align2::LEFT_CENTER,
                        shape.label(),
                        egui::FontId::proportional(12.0),
                        ink,
                    );
                    if row_response.clicked() {
                        picked = Some(shape);
                    }
                }
            });
        self.autoshape_flyout = open;
        if let Some(shape) = picked {
            self.editor.tool = Tool::AutoShape(shape);
            self.autoshape_flyout = false;
            self.status = format!("tool: Auto Shape — {}", shape.label());
        }
    }

    /// Canvas-local screen position of a world point.
    fn screen_of(&self, world: geom::Point) -> Pos2 {
        let screen = self.view.to_screen(world);
        Pos2::new(screen.x, screen.y)
    }

    /// The entered definition's nine-slice guides in canvas-local
    /// space: the shape bounds plus one drag handle per edge, at the
    /// current insets. `None` when not editing a guided symbol.
    fn nine_slice_overlay(&self) -> Option<(Rect, [(Rect, NineSliceEdge); 4])> {
        let guides = self.editor.editing_definition()?.nine_slice.clone()?;
        let bounds = guides.shape_bounds;
        let screen = Rect::from_min_max(
            self.screen_of(geom::Point::new(bounds.x0, bounds.y0)),
            self.screen_of(geom::Point::new(bounds.x1, bounds.y1)),
        );
        let [top, bottom, left, right] = guides.controls.map(|value| value as f32);
        let (mid_x, mid_y) = (screen.center().x, screen.center().y);
        let handle = |x: f32, y: f32| Rect::from_center_size(Pos2::new(x, y), Vec2::splat(10.0));
        Some((
            screen,
            [
                (handle(mid_x, screen.top() + top), NineSliceEdge::Top),
                (
                    handle(mid_x, screen.bottom() - bottom),
                    NineSliceEdge::Bottom,
                ),
                (handle(screen.left() + left, mid_y), NineSliceEdge::Left),
                (handle(screen.right() - right, mid_y), NineSliceEdge::Right),
            ],
        ))
    }

    /// The nine-slice guide edge under a canvas-local point, if any.
    fn nine_slice_edge_at(&self, local: Pos2) -> Option<NineSliceEdge> {
        let (_, handles) = self.nine_slice_overlay()?;
        handles
            .iter()
            .find(|(handle, _)| handle.contains(local))
            .map(|(_, edge)| *edge)
    }

    /// The canvas: compose renders the page, egui shows the texture,
    /// tools drive the editor through world-space input.
    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (available, response_rect) = {
            let size = ui.available_size();
            let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
            (size, (rect, response))
        };
        let (rect, response) = response_rect;
        // Remembered for view commands that arrive outside the canvas
        // (menu zoom/fit, keyboard shortcuts) — the anchor is the
        // canvas center of the last laid-out frame.
        self.canvas_rect = Some(rect);

        // Per-tool cursor icons (CS6 semantics); space held anywhere
        // switches to the temporary Hand (the CS6 space-drag pan).
        let space_held = ui.input(|input| input.key_down(egui::Key::Space));
        let cursor = if space_held {
            egui::CursorIcon::Grab
        } else {
            match self.editor.tool {
                Tool::Pointer | Tool::Subselect => egui::CursorIcon::Default,
                Tool::Rect | Tool::Ellipse | Tool::Line | Tool::Slice | Tool::Hotspot => {
                    egui::CursorIcon::Crosshair
                }
                Tool::Text => egui::CursorIcon::Text,
                Tool::Zoom => egui::CursorIcon::ZoomIn,
                Tool::Hand => egui::CursorIcon::Grab,
                Tool::AutoShape(_) => egui::CursorIcon::Crosshair,
                Tool::Paintbrush
                | Tool::Pencil
                | Tool::Eraser
                | Tool::PaintBucket
                | Tool::Gradient
                | Tool::Eyedropper => egui::CursorIcon::Crosshair,
                Tool::Marquee
                | Tool::EllipticalMarquee
                | Tool::Lasso
                | Tool::PolygonalMarquee
                | Tool::MagicWand => egui::CursorIcon::Crosshair,
            }
        };
        let response = response.on_hover_cursor(cursor);

        // First fit on a fresh raster keeps the document visible.
        if self.raster.is_none() {
            if let Some(page) = self.editor.document().pages.get(self.editor.page) {
                let world = geom::Rect::new(
                    page.info.canvas.x,
                    page.info.canvas.y,
                    page.info.canvas.x + page.info.canvas.width,
                    page.info.canvas.y + page.info.canvas.height,
                );
                if world.width() > 0.0 && world.height() > 0.0 {
                    self.view.fit(world, available);
                }
            }
        }

        // Raster cache: re-render only when the session changed.
        let stamp = self.stamp();
        let texture = match &self.raster {
            Some((texture, made)) if *made == stamp => texture.clone(),
            _ => match compose::render(self.editor.document(), self.editor.page, self.editor.frame)
            {
                Ok(rendered) => {
                    let image = ColorImage::from_rgba_premultiplied(
                        [rendered.width as usize, rendered.height as usize],
                        &rendered.pixels,
                    );
                    let texture = ui.ctx().load_texture("page", image, TextureOptions::LINEAR);
                    self.raster = Some((texture.clone(), stamp));
                    texture
                }
                Err(error) => {
                    ui.painter()
                        .rect_filled(rect, 0.0, Color32::from_rgb(24, 24, 24));
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("cannot compose: {error}"),
                        egui::FontId::proportional(14.0),
                        Color32::from_rgb(200, 120, 120),
                    );
                    return;
                }
            },
        };

        // Checker-dark backdrop, then the page texture through the view.
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from_rgb(32, 32, 32));
        let top_left = self.view.to_screen(geom::Point::new(0.0, 0.0));
        let size = self.view.vec_to_screen(geom::Vec2::new(
            texture.size()[0] as f64,
            texture.size()[1] as f64,
        ));
        let image_rect = Rect::from_min_size(
            Pos2::new(rect.left() + top_left.x, rect.top() + top_left.y),
            Vec2::new(size.x.max(1.0), size.y.max(1.0)),
        );
        ui.painter().image(
            texture.id(),
            image_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );

        // Nine-slice guide overlay while a guided definition is
        // entered: shape bounds, the four guide lines, one handle per
        // edge. Overlay geometry is canvas-local; shift by the widget
        // origin to paint.
        if let Some((bounds, handles)) = self.nine_slice_overlay() {
            let color = Color32::from_rgb(90, 200, 250);
            let shift = egui::Vec2::new(rect.left(), rect.top());
            let bounds = bounds.translate(shift);
            let painter = ui.painter();
            painter.rect_stroke(bounds, 0.0, (1.0, color), egui::StrokeKind::Outside);
            let handles = handles.map(|(handle, edge)| (handle.translate(shift), edge));
            let [(top, _), (bottom, _), (left, _), (right, _)] = handles;
            painter.line_segment(
                [
                    Pos2::new(bounds.left(), top.center().y),
                    Pos2::new(bounds.right(), top.center().y),
                ],
                (1.0, color),
            );
            painter.line_segment(
                [
                    Pos2::new(bounds.left(), bottom.center().y),
                    Pos2::new(bounds.right(), bottom.center().y),
                ],
                (1.0, color),
            );
            painter.line_segment(
                [
                    Pos2::new(left.center().x, bounds.top()),
                    Pos2::new(left.center().x, bounds.bottom()),
                ],
                (1.0, color),
            );
            painter.line_segment(
                [
                    Pos2::new(right.center().x, bounds.top()),
                    Pos2::new(right.center().x, bounds.bottom()),
                ],
                (1.0, color),
            );
            for (handle, _) in handles {
                painter.rect_filled(handle, 2.0, color);
            }
        }

        // Marching ants: the pixel selection's boundary, drawn over
        // the page. The in-progress marquee paints a dashed preview
        // band; the committed mask paints from `selection_edges`
        // (the image border is not a boundary).
        {
            const ANTS: Color32 = Color32::from_rgb(0, 0, 0);
            let to_canvas = |world: geom::Point| {
                let screen = self.view.to_screen(world);
                Pos2::new(rect.left() + screen.x, rect.top() + screen.y)
            };
            let painter = ui.painter();
            // The drag preview: the marquee's live frame, offset by
            // the target image's location.
            if let Some(selection) = self.editor.pixel_selection.as_ref() {
                let location = self.editor.image_location(selection.target);
                let offset = location.unwrap_or_default();
                let a = to_canvas(geom::Point::new(
                    offset.x + selection.start.x.min(selection.current.x),
                    offset.y + selection.start.y.min(selection.current.y),
                ));
                let b = to_canvas(geom::Point::new(
                    offset.x + selection.start.x.max(selection.current.x),
                    offset.y + selection.start.y.max(selection.current.y),
                ));
                let band = Rect::from_two_pos(a, b);
                for shape in egui::Shape::dashed_line(
                    &[
                        band.left_top(),
                        band.right_top(),
                        band.right_bottom(),
                        band.left_bottom(),
                        band.left_top(),
                    ],
                    (1.0, ANTS),
                    4.0,
                    3.0,
                ) {
                    painter.add(shape);
                }
            }
            // The committed selection's boundary.
            if let Some(edges) = self.editor.selection_outline() {
                for [x0, y0, x1, y1] in edges {
                    painter.line_segment(
                        [
                            to_canvas(geom::Point::new(x0, y0)),
                            to_canvas(geom::Point::new(x1, y1)),
                        ],
                        (1.0, ANTS),
                    );
                }
            }
        }

        // Rubber band for a creation drag.
        if let Some(drag) = &self.editor.drag {
            let a = self.view.to_screen(drag.start);
            let b = self.view.to_screen(drag.current);
            let band = Rect::from_two_pos(
                Pos2::new(rect.left() + a.x, rect.top() + a.y),
                Pos2::new(rect.left() + b.x, rect.top() + b.y),
            );
            ui.painter().rect_stroke(
                band,
                0.0,
                (1.0, Color32::from_rgb(120, 180, 255)),
                egui::StrokeKind::Outside,
            );
        }

        // Input routing, by tool. Pointer coordinates are widget-local;
        // the view works in the same local space (top_left origin).
        // A press on a nine-slice guide handle drags that guide
        // instead of the tool.
        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.left_top().to_vec2();
                let world = self.view.to_world(local);
                match self.nine_slice_edge_at(local) {
                    Some(edge) => {
                        if let Some(symbol) = self.editor.symbol_scope {
                            if let Err(error) = self.editor.begin_guide_drag(symbol, edge, world) {
                                self.status = format!("guide drag failed: {error}");
                            }
                        }
                    }
                    None => {
                        // Space-drag pan: no tool drag begins while the
                        // temporary Hand is active.
                        if !space_held {
                            match self.editor.tool {
                                Tool::Paintbrush | Tool::Pencil | Tool::Eraser | Tool::Gradient => {
                                    // A paint stroke: the dabs land on
                                    // the image under the pointer.
                                    if let Err(error) = self.editor.begin_stroke(world) {
                                        self.status = format!("paint failed: {error}");
                                    }
                                }
                                Tool::Pointer | Tool::Subselect => {
                                    // A press selects its target first,
                                    // then moves the selection live —
                                    // or (Alt) clones it, CS6 pointer
                                    // semantics. Empty canvas starts a
                                    // (no-op) rubber band.
                                    if let Some(hit) = self.editor.hit_at(world) {
                                        let ids = if self.editor.selection_set.contains(&hit) {
                                            self.editor.live_selection()
                                        } else {
                                            self.editor.selection = Some(hit);
                                            self.editor.selection_set = vec![hit];
                                            vec![hit]
                                        };
                                        let outcome = if ui.input(|i| i.modifiers.alt) {
                                            self.editor
                                                .begin_clone_drag(world, &ids)
                                                .map(|_created| ())
                                        } else {
                                            self.editor.begin_move_drag(world, ids)
                                        };
                                        if let Err(error) = outcome {
                                            self.status = format!("drag failed: {error}");
                                        }
                                    } else {
                                        self.editor.begin_drag(world);
                                    }
                                }
                                Tool::PaintBucket | Tool::Eyedropper => {
                                    // Click-only tools: the press
                                    // itself is the whole gesture.
                                }
                                Tool::Marquee
                                | Tool::EllipticalMarquee
                                | Tool::Lasso
                                | Tool::MagicWand => {
                                    // A pixel selection: the shape
                                    // lands on the image under the
                                    // pointer, folded by the
                                    // modifier keys.
                                    self.editor.pixel_mode =
                                        PixelSelectMode::from_modifiers(ui.input(|i| i.modifiers));
                                    if let Err(error) = self.editor.begin_pixel_selection(world) {
                                        self.status = format!("select failed: {error}");
                                    }
                                }
                                Tool::PolygonalMarquee => {
                                    // The polygon's vertices are
                                    // clicks, not a drag.
                                }
                                _ => self.editor.begin_drag(world),
                            }
                        }
                    }
                }
            }
        }
        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.left_top().to_vec2();
                let world = self.view.to_world(local);
                if self.editor.guide_drag.is_some() {
                    self.editor.drag_guide_to(world);
                } else if self.editor.pixel_selection.is_some() {
                    if let Err(error) = self.editor.extend_pixel_selection(world) {
                        self.status = format!("select failed: {error}");
                    }
                } else if self.editor.stroke.is_some() {
                    if let Err(error) = self.editor.stroke_to(world) {
                        self.status = format!("paint failed: {error}");
                    }
                } else if self.editor.drag.is_some() {
                    let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));
                    self.editor
                        .drag_to_with(world, DragConstraint { shift, alt });
                } else if self.editor.tool == Tool::Hand
                    || space_held
                    || response.dragged_by(egui::PointerButton::Middle)
                {
                    // Hand tool, space-drag pan, or middle-button pan.
                    self.view.pan_by(response.drag_delta());
                }
            }
        }
        if response.drag_stopped() {
            if self.editor.guide_drag.is_some()
                && let Err(error) = self.editor.end_guide_drag()
            {
                self.status = format!("nine-slice drag failed: {error}");
            }
            if let Err(error) = self.editor.end_drag() {
                self.status = format!("create failed: {error}");
            }
            if let Err(error) = self.editor.end_stroke() {
                self.status = format!("paint failed: {error}");
            }
            if let Err(error) = self.editor.end_pixel_selection() {
                self.status = format!("select failed: {error}");
            }
        }
        // Drop target for the Library panel's drag-to-place: the
        // payload clears itself at end of frame, release included.
        if response.hovered() && ui.input(|input| input.pointer.any_released()) {
            if let Some(symbol) = egui::DragAndDrop::take_payload::<i64>(ui.ctx()) {
                let placed_at = ui
                    .input(|input| input.pointer.latest_pos())
                    .map_or(rect.center(), |pos| pos - rect.left_top().to_vec2());
                if let Err(error) = self
                    .editor
                    .place_symbol(*symbol, self.view.to_world(placed_at))
                {
                    self.status = format!("place failed: {error}");
                }
            }
        }
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.left_top().to_vec2();
                match self.editor.tool {
                    Tool::Pointer | Tool::Subselect => {
                        let world = self.view.to_world(local);
                        if ui.input(|i| i.modifiers.shift) {
                            self.editor.toggle_select_at(world);
                        } else {
                            let _ = self.editor.select_at(world);
                        }
                    }
                    Tool::Hand => {}
                    Tool::PaintBucket => {
                        let world = self.view.to_world(local);
                        match self.editor.fill_target(world) {
                            Ok(true) => {}
                            Ok(false) => {
                                self.status = "no image under the pointer".to_owned();
                            }
                            Err(error) => self.status = format!("fill failed: {error}"),
                        }
                    }
                    Tool::Eyedropper => {
                        let world = self.view.to_world(local);
                        if self.editor.pick_color(world).is_none() {
                            self.status = "no image under the pointer".to_owned();
                        }
                    }
                    Tool::Marquee | Tool::EllipticalMarquee | Tool::Lasso | Tool::MagicWand => {
                        // A click without a drag is still a
                        // gesture: the wand selects, a collapsed
                        // marquee deselects (the engine folds an
                        // empty shape into no selection).
                        let world = self.view.to_world(local);
                        self.editor.pixel_mode =
                            PixelSelectMode::from_modifiers(ui.input(|i| i.modifiers));
                        if self.editor.pixel_selection.is_none()
                            && self.editor.begin_pixel_selection(world).is_ok()
                            && let Err(error) = self.editor.end_pixel_selection()
                        {
                            self.status = format!("select failed: {error}");
                        }
                    }
                    Tool::PolygonalMarquee => {
                        let world = self.view.to_world(local);
                        self.editor.pixel_mode =
                            PixelSelectMode::from_modifiers(ui.input(|i| i.modifiers));
                        if let Err(error) = self.editor.add_polygon_point(world) {
                            self.status = format!("polygon failed: {error}");
                        }
                    }
                    Tool::Zoom => {
                        let factor = if ui.input(|i| i.modifiers.alt) {
                            1.0 / 1.25
                        } else {
                            1.25
                        };
                        self.view.zoom_about(local, factor);
                    }
                    _ => {}
                }
            }
        }
        // Double-click enters a symbol instance's definition (CS6's
        // pointer double-click), like the inspector's Edit Definition.
        if response.double_clicked()
            && matches!(self.editor.tool, Tool::Pointer)
            && let Some(pos) = response.interact_pointer_pos()
        {
            let world = self.view.to_world(pos - rect.left_top().to_vec2());
            let entered = self.editor.hit_at(world).and_then(|id| {
                self.editor
                    .document()
                    .registry
                    .get(id)
                    .map(|data| (id, data))
            });
            if let Some((_id, ObjectData::SymbolInstance(instance))) = entered {
                let number = instance.symbol_number;
                match self.editor.enter_symbol(number) {
                    Ok(()) => self.status = format!("editing symbol {number}"),
                    Err(error) => self.status = format!("enter symbol failed: {error}"),
                }
            }
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let anchor = ui
                    .input(|i| i.pointer.latest_pos())
                    .map_or(rect.center(), |pos| pos - rect.left_top().to_vec2());
                self.view
                    .zoom_about(anchor, if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 });
            }
        }
    }
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Keep the context reachable for out-of-frame mutators (the
        // web bridge's request_repaint).
        self.ctx = Some(ui.ctx().clone());
        // Menu/shortcut actions run before layout so their effects are
        // visible the same frame.
        self.shortcuts(ui.ctx());
        // Panels first, CentralPanel last (egui's required order), in
        // the CS6 arrangement: menu, tool strip, panel stack,
        // bottom-docked Properties, status strip, document canvas.
        egui::Panel::top(egui::Id::new("menu")).show(ui, |ui| self.menu_bar(ui));
        egui::Panel::left(egui::Id::new("tools"))
            .exact_size(34.0)
            .show(ui, |ui| self.tools_panel(ui));
        egui::Panel::right(egui::Id::new("panels"))
            .default_size(230.0)
            .size_range(160.0..=420.0)
            .resizable(true)
            .show(ui, |ui| self.right_panels(ui));
        egui::Panel::bottom(egui::Id::new("properties"))
            .default_size(210.0)
            .size_range(120.0..=360.0)
            .resizable(true)
            .show(ui, |ui| self.properties_panel(ui));
        egui::Panel::bottom(egui::Id::new("status")).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "{} · page {} · state {} · undo {} · redo {} · journal {}",
                        self.doc_name,
                        self.editor.page + 1,
                        self.editor.frame + 1,
                        self.editor.session.undo_len(),
                        self.editor.session.redo_len(),
                        self.editor.session.journal_len(),
                    ));
                });
            });
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                // The document tab strip: a single-document window, so
                // the CS6 MDI tab strip reduced to its one member.
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Button::new(egui::RichText::new(&self.doc_name).size(11.0))
                            .selected(true),
                    );
                    ui.weak("FireCraft");
                });
                self.canvas(ui);
            });
        if self.export_dialog.is_some() {
            egui::Window::new("Export")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.export_dialog_ui(ui));
        }
        if self.help_open {
            self.help_panel(ui.ctx());
        }
        if self.close_requested {
            self.close_requested = false;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    // The web bridge downcasts through this (`WebRunner::app_mut`);
    // the trait default returns None and the runner panics on it,
    // which took down every browser open/apply/export until v0.2.1.
    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use engine::platform::{FileSystem, MemoryFs};

    /// Scripted dialogs: the steps path answers the replay/save
    /// picks, the document path the base pick — matched by title.
    struct FixedDialogs {
        steps: String,
        doc: String,
    }

    impl engine::platform::Dialogs for FixedDialogs {
        fn open_path(&self, title: &str, _filters: &[(String, Vec<String>)]) -> Option<String> {
            if title.starts_with("Replay") {
                Some(self.steps.clone())
            } else {
                Some(self.doc.clone())
            }
        }

        fn save_path(
            &self,
            _title: &str,
            _suggested_name: &str,
            _filters: &[(String, Vec<String>)],
        ) -> Option<String> {
            Some(self.steps.clone())
        }

        fn message(&self, title: &str, text: &str) {
            eprintln!("[firecraft-test] {title}: {text}");
        }
    }

    /// A GUI-shaped session: seeds + drag-created rect + a rename.
    fn gui_editor() -> Editor {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(90.0, 50.0));
        editor.end_drag().unwrap();
        let id = editor.current_cell().unwrap().elements[0];
        editor
            .apply(
                "object.rename",
                serde_json::json!({ "id": id, "name": "plate" }),
            )
            .unwrap();
        editor
    }

    /// A shell with scripted dialogs over a memory filesystem.
    fn gui_shell(editor: Editor) -> Shell {
        let dialogs = FixedDialogs {
            steps: "/mem/steps.fwsteps".to_owned(),
            doc: "/mem/doc.fw.png".to_owned(),
        };
        Shell::with_platform(editor, Box::new(MemoryFs::new()), Box::new(dialogs))
    }

    /// The action dispatch wiring: Select All picks up the cell's
    /// objects, Delete removes through the registry, Arrange reorders
    /// with a post-removal index, and Inert renders but does nothing.
    #[test]
    fn menu_actions_drive_the_editor() {
        let mut shell = gui_shell(gui_editor());
        let first = shell.editor.current_cell().unwrap().elements[0];

        // Select All selects everything in the current cell.
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SelectAll));
        assert_eq!(shell.editor.selection_set.len(), 1);
        assert_eq!(shell.editor.selection, Some(first));

        // Arrange with a single object is a no-op with a status line,
        // not a command.
        shell.run_action(&menus::Action::Arrange(menus::Arrange::Front));
        assert!(shell.status.contains("stacking position"));

        // Inert actions are accepted by the table but change nothing.
        let before = shell.editor.session.journal_len();
        shell.run_action(&menus::Action::Inert);
        assert_eq!(shell.editor.session.journal_len(), before);

        // Delete drives object.delete and empties the cell.
        shell.run_action(&menus::Action::Sel(menus::SelAction::Delete));
        assert!(shell.editor.current_cell().unwrap().elements.is_empty());
    }

    /// The CS6 shortcut table fires through the same dispatch —
    /// Ctrl+Z undoes, plain V picks the Pointer tool.
    #[test]
    fn shortcut_actions_share_the_dispatch() {
        let mut shell = gui_shell(gui_editor());
        let undoable = shell.editor.session.undo_len();
        assert!(undoable >= 1, "the seeded session has history");
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Undo));
        assert_eq!(shell.editor.session.undo_len(), undoable - 1);
        assert_eq!(shell.editor.session.redo_len(), 1);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Redo));
        assert_eq!(shell.editor.session.undo_len(), undoable);
        assert_eq!(shell.editor.session.redo_len(), 0);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::PickTool(
            Tool::Pointer,
        )));
        assert_eq!(shell.editor.tool, Tool::Pointer);
    }

    /// The preset registry is internally consistent: names unique,
    /// every numeric default parses inside its documented range, the
    /// knot preset rides raw points with an empty value, and the
    /// slider metadata's reset values stay in range.
    #[test]
    fn filter_preset_registry_is_consistent() {
        let mut names = Vec::new();
        for (family, presets) in FILTER_FAMILIES {
            assert!(!presets.is_empty(), "{family} is empty");
            for preset in *presets {
                names.push(preset.name);
                let points_preset = !preset.points.is_empty();
                assert_eq!(
                    points_preset,
                    preset
                        .params
                        .iter()
                        .any(|(key, _)| key.ends_with("_points")),
                    "{}: points xor text",
                    preset.name
                );
                for (key, value) in preset.params {
                    if FILTER_BOOL_KEYS.contains(key) {
                        assert!(matches!(*value, "0" | "1"), "{key}: {value}");
                        continue;
                    }
                    if key.ends_with("_points") {
                        assert!(value.is_empty(), "{key}: knots carry no text");
                        continue;
                    }
                    if let Some(meta) = FILTER_PARAMS.iter().find(|meta| meta.key == *key) {
                        let parsed: f64 =
                            value.parse().unwrap_or_else(|_| panic!("{key}: {value}"));
                        assert!(
                            (meta.min..=meta.max).contains(&parsed),
                            "{key}: {parsed} outside {}..{}",
                            meta.min,
                            meta.max
                        );
                    }
                }
            }
        }
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate preset names");
        for meta in FILTER_PARAMS {
            assert!(
                (meta.min..=meta.max).contains(&meta.default),
                "{}: default outside range",
                meta.key
            );
        }
    }

    /// The Filters menu dispatch and the panel preset path drive the
    /// same commands: a menu pick adds the preset's `DCVs`, the curves
    /// preset lands as the raw knot node, a param edit rides
    /// `filter.setParam` through the journal (so undo restores
    /// it), and a pick with nothing selected is a status line.
    #[test]
    fn filter_actions_flow_through_the_journal() {
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SelectAll));
        let id = shell.editor.selection.unwrap();

        shell.run_action(&menus::Action::Filter("gaussianBlur"));
        let effects = shell_effect_list(&shell, id);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].internal_name, "gaussianBlur");

        let curves = filter_preset("curves").unwrap();
        shell.apply_filter_preset(id, *curves);
        let effects = shell_effect_list(&shell, id);
        assert_eq!(effects.len(), 2);
        let doc::EffectValue::Raw(node) = &effects[1].properties[0].value else {
            panic!("curves knots encode as a raw node");
        };
        assert_eq!(node.as_children().map_or(0, <[doc::Node]>::len), 3);

        shell.set_filter_param(id, 0, "gaussian_blur_radius", "4");
        let mut radius = shell_param(&shell, id, 0, "gaussian_blur_radius");
        assert_eq!(radius, "4");
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Undo));
        radius = shell_param(&shell, id, 0, "gaussian_blur_radius");
        assert_eq!(radius, "2", "undo restores the preset default");

        let mut bare = gui_shell(gui_editor());
        bare.run_action(&menus::Action::Filter("invert"));
        assert!(bare.status.contains("pick an object"), "{}", bare.status);
    }

    /// The applied effects of an object, read the way the properties
    /// panel reads them.
    fn shell_effect_list(shell: &Shell, id: doc::ObjectId) -> Vec<doc::Effect> {
        match shell.editor.document().registry.get(id) {
            Some(ObjectData::Path(path)) => path.effects.clone(),
            Some(ObjectData::Text(text)) => text.effects.clone(),
            _ => Vec::new(),
        }
    }

    /// The three presets the audit found dead now reach a kernel: the
    /// drop shadow carries the four scalars its kernel reads (all four
    /// or nothing renders), and the two emboss presets reach the bevel
    /// kernel under their panel spellings. Each preset must change
    /// the rendered page — a preset that lands as an effect stack the
    /// kernel skips leaves the render untouched.
    #[test]
    fn the_shadowed_and_embossed_presets_reach_the_renderer() {
        for name in ["dropShadow", "emboss", "pillowEmboss"] {
            let mut shell = gui_shell(gui_editor());
            let id = shell.editor.current_cell().unwrap().elements[0];
            let before = compose::render(
                shell.editor.document(),
                shell.editor.page,
                shell.editor.frame,
            )
            .expect("renders");
            shell.apply_filter_preset(id, *filter_preset(name).expect("preset exists"));
            let after = compose::render(
                shell.editor.document(),
                shell.editor.page,
                shell.editor.frame,
            )
            .expect("renders");
            assert_ne!(
                before.pixels, after.pixels,
                "{name}: the preset must change the render"
            );
        }
    }

    /// An editor holding two rectangles on the active layer.
    fn gui_editor_two() -> Editor {
        let mut editor = gui_editor();
        editor.begin_drag(geom::Point::new(110.0, 100.0));
        editor.drag_to(geom::Point::new(180.0, 140.0));
        editor.end_drag().unwrap();
        editor
    }

    /// The path object behind an id, for the transform assertions.
    fn shell_path(shell: &Shell, id: doc::ObjectId) -> PathObject {
        match shell.editor.document().registry.get(id) {
            Some(ObjectData::Path(path)) => path.clone(),
            _ => panic!("object {id:?} is not a path"),
        }
    }

    /// Edit ▸ Copy / Paste: the clipboard crosses the engine's command
    /// door — the view's selection lands in the document first, the
    /// paste re-mints onto the active layer, and the pasted object
    /// becomes the selection.
    #[test]
    fn copy_and_paste_round_trip_through_the_clipboard() {
        let mut shell = gui_shell(gui_editor());
        let id = shell.editor.current_cell().unwrap().elements[0];
        shell.editor.set_selection(vec![id]);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Copy));
        assert!(shell.status.contains("copied"), "{}", shell.status);
        let before = shell.editor.current_cell().unwrap().elements.len();
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Paste));
        assert!(shell.status.contains("pasted"), "{}", shell.status);
        let cell = shell.editor.current_cell().unwrap();
        assert_eq!(cell.elements.len(), before + 1, "the paste adds one object");
        assert_eq!(
            shell.editor.selection_set,
            vec![cell.elements[before]],
            "the pasted object is the selection"
        );
    }

    /// Edit ▸ Cut: the objects leave the document, and the view's
    /// selection follows them out.
    #[test]
    fn cut_takes_the_objects_with_it() {
        let mut shell = gui_shell(gui_editor());
        let id = shell.editor.current_cell().unwrap().elements[0];
        shell.editor.set_selection(vec![id]);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Cut));
        assert!(
            shell.editor.current_cell().unwrap().elements.is_empty(),
            "the cell is empty"
        );
        assert!(
            shell.editor.document().registry.get(id).is_none(),
            "the object left the registry"
        );
        assert!(
            shell.editor.selection_set.is_empty(),
            "the selection follows the cut objects out"
        );
    }

    /// Select ▸ Select Inverse: the complement of the selection within
    /// the cell the editor edits.
    #[test]
    fn select_inverse_complements_the_cell() {
        let mut shell = gui_shell(gui_editor_two());
        let ids = shell.editor.current_cell().unwrap().elements.clone();
        shell.editor.set_selection(vec![ids[0]]);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SelectInverse));
        assert_eq!(shell.editor.selection_set, vec![ids[1]]);
    }

    /// Modify ▸ Group / Ungroup: the group lands in the cell where the
    /// members were, and ungroup puts the members back.
    #[test]
    fn group_and_ungroup_fold_the_selection() {
        let mut shell = gui_shell(gui_editor_two());
        let ids = shell.editor.current_cell().unwrap().elements.clone();
        shell.editor.set_selection(ids);
        shell.run_action(&menus::Action::Sel(menus::SelAction::Group));
        let cell = shell.editor.current_cell().unwrap();
        assert_eq!(cell.elements.len(), 1, "one group replaces the members");
        let group = cell.elements[0];
        assert!(
            matches!(
                shell.editor.document().registry.get(group),
                Some(ObjectData::Group(_))
            ),
            "the folded object is a group"
        );
        shell.editor.set_selection(vec![group]);
        shell.run_action(&menus::Action::Sel(menus::SelAction::Ungroup));
        assert_eq!(
            shell.editor.current_cell().unwrap().elements.len(),
            2,
            "the group dissolves"
        );
    }

    /// Modify ▸ Transform: a quarter turn swaps the object's extents,
    /// and a flip moves its points.
    #[test]
    fn transform_rotates_and_flips_the_selection() {
        let mut shell = gui_shell(gui_editor());
        let id = shell.editor.current_cell().unwrap().elements[0];
        shell.editor.set_selection(vec![id]);
        let before = crate::editor::bounds_of(&shell_path(&shell, id));
        shell.run_action(&menus::Action::Sel(menus::SelAction::Rotate(90.0)));
        let after = crate::editor::bounds_of(&shell_path(&shell, id));
        assert!(
            (after.width() - before.height()).abs() < 0.001
                && (after.height() - before.width()).abs() < 0.001,
            "a quarter turn swaps the extents: {before:?} -> {after:?}"
        );
        let anchors = |shell: &Shell| {
            shell_path(shell, id)
                .contours
                .iter()
                .flat_map(|contour| contour.points.iter().map(|point| point.anchor))
                .collect::<Vec<_>>()
        };
        let before_points = anchors(&shell);
        shell.run_action(&menus::Action::Sel(menus::SelAction::Reflect {
            horizontal: true,
            vertical: false,
        }));
        assert_ne!(anchors(&shell), before_points, "the flip moves the points");
    }

    /// The Layers panel's three mutating surfaces: rename, the
    /// per-state lock, and the reorder. The reorder keeps the moved
    /// layer active — the user keeps editing the layer they moved.
    #[test]
    fn the_layers_panel_renames_locks_and_reorders() {
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Sel(menus::SelAction::NewLayer));
        assert_eq!(shell.editor.layer_count(), 2, "the panel adds a layer");

        shell.run_command(
            "layer.rename",
            serde_json::json!({ "page": 0, "layer": 0, "name": "base" }),
        );
        assert_eq!(
            shell.editor.document().pages[0].layers.layers[0].name,
            "base"
        );

        shell.run_command(
            "layer.setLocked",
            serde_json::json!({
                "page": 0, "layer": 0, "frame": 0, "locked": true,
            }),
        );
        assert!(
            shell.editor.document().pages[0].layers.layers[0].cells[0].locked,
            "the layer is locked in this state"
        );

        shell.editor.set_active_layer(0);
        shell.reorder_layer(0, true);
        let layers = &shell.editor.document().pages[0].layers.layers;
        assert_eq!(layers[1].name, "base", "the layer moved up");
        assert_eq!(
            shell.editor.active_layer(),
            1,
            "the moved layer stays active"
        );
    }

    /// Modify ▸ Flatten Selection: the selection bakes into one
    /// bitmap, in place of the members.
    #[test]
    fn flatten_selection_bakes_into_one_bitmap() {
        let mut shell = gui_shell(gui_editor_two());
        let ids = shell.editor.current_cell().unwrap().elements.clone();
        shell.editor.set_selection(ids);
        shell.run_action(&menus::Action::Sel(menus::SelAction::FlattenSelection));
        let cell = shell.editor.current_cell().unwrap();
        assert_eq!(cell.elements.len(), 1, "the bake leaves one object");
        assert!(
            matches!(
                shell.editor.document().registry.get(cell.elements[0]),
                Some(ObjectData::Image(_))
            ),
            "the baked object is a bitmap"
        );
    }

    /// One text param of one applied effect (the `DCVs` form the
    /// panel's rows edit).
    fn shell_param(shell: &Shell, id: doc::ObjectId, index: usize, key: &str) -> String {
        let effects = shell_effect_list(shell, id);
        let doc::EffectValue::Text(value) = &effects[index]
            .properties
            .iter()
            .find(|property| property.key == key)
            .unwrap_or_else(|| panic!("{key}: absent"))
            .value
        else {
            panic!("{key}: not text");
        };
        value.clone()
    }

    #[test]
    fn history_panel_saves_and_replays_steps() {
        let editor = gui_editor();
        let steps = editor.steps_text();
        // The replay base is the PRE-steps document — what the user
        // opens before running the recorded steps — not the edited one.
        let base = Editor::from_document(doc::Document::default())
            .save()
            .unwrap();

        // Save through the panel action.
        let dialogs = FixedDialogs {
            steps: "session.fwsteps".to_owned(),
            doc: "base.fw.png".to_owned(),
        };
        let mut saver = Shell::with_platform(
            gui_editor(),
            Box::new(MemoryFs::new()),
            Box::new(FixedDialogs {
                steps: dialogs.steps.clone(),
                doc: dialogs.doc.clone(),
            }),
        );
        saver.save_steps();
        assert!(saver.status.contains("saved steps"), "{}", saver.status);

        // Replay through the panel action: fresh FS carrying the base
        // document and the steps file, base opened from disk.
        let fs = MemoryFs::new();
        fs.write_file("session.fwsteps", steps.as_bytes()).unwrap();
        fs.write_file("base.fw.png", &base).unwrap();
        let mut replayer =
            Shell::with_platform(Editor::untitled(), Box::new(fs), Box::new(dialogs));
        replayer.replay_steps();
        assert!(replayer.status.contains("replayed"), "{}", replayer.status);
        assert_eq!(
            serde_json::to_vec(replayer.editor.session.document()).unwrap(),
            serde_json::to_vec(editor.session.document()).unwrap(),
            "the replayed session equals the original, byte for byte"
        );
    }

    #[test]
    fn web_export_dialog_writes_html_and_files() {
        // A slice dragged on the canvas, then the dialog pointed at
        // the HTML+Images workflow.
        let mut editor = Editor::untitled();
        editor.tool = Tool::Slice;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(90.0, 50.0));
        editor.end_drag().unwrap();
        let id = editor.document().pages[0]
            .layers
            .layers
            .iter()
            .find(|layer| layer.has_url && layer.is_special)
            .and_then(|layer| layer.cells[0].elements.first())
            .copied()
            .unwrap();
        editor
            .apply(
                "slice.setLink",
                serde_json::json!({ "id": id, "link": "https://example.test/", "alt": "go" }),
            )
            .unwrap();

        let dialogs = FixedDialogs {
            steps: "export.html".to_owned(),
            doc: String::new(),
        };
        let mut shell = Shell::with_platform(editor, Box::new(MemoryFs::new()), Box::new(dialogs));
        shell.export_dialog = Some(ExportDialog {
            web: Some(1), // HTML + images
            ..ExportDialog::default()
        });
        let dialog = shell.export_dialog.take().unwrap();
        shell.export_via_dialog(&dialog);

        assert!(shell.status.contains("exported"), "{}", shell.status);
        assert!(shell.export_dialog.is_none(), "the dialog closes");
        let html = String::from_utf8(shell.fs.read_file("export.html").unwrap()).unwrap();
        assert!(html.contains(r#"href="https://example.test/""#));
        // The slice's image landed beside the HTML under its auto
        // name; the dialog's doc base is "export".
        let image = shell.fs.read_file("nexport_1_1.png").unwrap();
        assert_eq!(&image[..4], b"\x89PNG");
    }

    /// A 4×4 opaque red PNG, written through the project's own encoder.
    fn red_png() -> Vec<u8> {
        let rgba = [255u8, 0, 0, 255].repeat(16);
        let plane = io::Plane::new(4, 4, &rgba).expect("4x4 plane");
        io::png::encode(&plane).expect("png bytes")
    }

    /// File ▸ Import… places the raster on the active layer, and the
    /// object that lands there is a real image object.
    #[test]
    fn import_places_a_raster_on_the_active_layer() {
        let mut shell = gui_shell(Editor::untitled());
        shell.import_raster(&red_png(), "red.png").expect("import");
        let cell = shell.editor.current_cell().expect("the page's cell");
        assert_eq!(cell.elements.len(), 1, "the import added one object");
        let id = cell.elements[0];
        let document = shell.editor.document();
        let Some(ObjectData::Image(image)) = document.registry.get(id) else {
            panic!("the imported object is an image")
        };
        assert_eq!(image.pixels, (4, 4), "the source extent is kept");
        assert_eq!(image.name.as_deref(), Some("red.png"));
    }

    /// The active layer decides where the import lands: with a second
    /// layer added and selected, the object goes there — not to the
    /// layer underneath.
    #[test]
    fn import_lands_on_the_selected_layer() {
        let mut shell = gui_shell(Editor::untitled());
        shell
            .editor
            .apply("layer.add", serde_json::json!({ "page": 0 }))
            .expect("second layer");
        assert_eq!(shell.editor.layer_count(), 2);
        shell.editor.set_active_layer(1);
        shell.import_raster(&red_png(), "red.png").expect("import");
        let document = shell.editor.document();
        assert!(
            document.pages[0].layers.layers[0].cells[0]
                .elements
                .is_empty(),
            "the lower layer stays empty"
        );
        assert_eq!(
            document.pages[0].layers.layers[1].cells[0].elements.len(),
            1,
            "the active layer took the import"
        );
    }

    /// The pixel at image coordinates (x, y) — the paint tools' read
    /// path, through the engine's own sampler.
    fn image_pixel(shell: &Shell, id: doc::ObjectId, x: i64, y: i64) -> Option<[u8; 4]> {
        let document = shell.editor.document();
        let Some(ObjectData::Image(image)) = document.registry.get(id) else {
            return None;
        };
        engine::bitmap::sample_pixel(image, &document.tiles, x, y)
    }

    /// A shell with a 4×4 red raster imported on the active layer,
    /// and the imported image's id.
    fn shell_with_raster() -> (Shell, doc::ObjectId) {
        let mut shell = gui_shell(Editor::untitled());
        shell.import_raster(&red_png(), "red.png").expect("import");
        let id = shell
            .editor
            .current_cell()
            .expect("the page's cell")
            .elements[0];
        (shell, id)
    }

    /// The brush lays its dabs on the image under the pointer, and the
    /// whole stroke undoes as one step.
    #[test]
    fn the_brush_paints_a_stroke_on_the_image() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::Paintbrush;
        shell.editor.brush.color = [0, 0, 255, 255];
        shell.editor.brush.size = 2;
        shell.editor.brush.edge = 0;

        shell
            .editor
            .begin_stroke(geom::Point::new(0.5, 1.5))
            .expect("the stroke starts");
        shell
            .editor
            .stroke_to(geom::Point::new(3.5, 1.5))
            .expect("the stroke continues");
        shell.editor.end_stroke().expect("the stroke ends");

        assert_eq!(
            image_pixel(&shell, id, 0, 1),
            Some([0, 0, 255, 255]),
            "the dab painted the row"
        );
        assert_eq!(
            image_pixel(&shell, id, 0, 0),
            Some([255, 0, 0, 255]),
            "the rows above the stroke keep their color"
        );

        shell.editor.undo().expect("undo");
        assert_eq!(
            image_pixel(&shell, id, 0, 1),
            Some([255, 0, 0, 255]),
            "one undo takes the whole stroke back"
        );
    }

    /// The pencil draws a hard one-pixel line — the dab carries no
    /// size, edge or shape slot.
    #[test]
    fn the_pencil_draws_a_hard_line() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::Pencil;
        shell.editor.brush.color = [0, 255, 0, 255];

        shell
            .editor
            .begin_stroke(geom::Point::new(0.0, 2.0))
            .expect("the stroke starts");
        shell
            .editor
            .stroke_to(geom::Point::new(3.0, 2.0))
            .expect("the stroke continues");
        shell.editor.end_stroke().expect("the stroke ends");

        assert_eq!(image_pixel(&shell, id, 1, 2), Some([0, 255, 0, 255]));
        assert_eq!(
            image_pixel(&shell, id, 1, 1),
            Some([255, 0, 0, 255]),
            "the pencil leaves its neighbours alone"
        );
    }

    /// The paint bucket fills the image under the pointer with the
    /// paint color.
    #[test]
    fn the_paint_bucket_fills_the_image() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::PaintBucket;
        shell.editor.brush.color = [255, 255, 0, 255];
        assert!(
            shell
                .editor
                .fill_target(geom::Point::new(1.0, 1.0))
                .expect("fill")
        );
        assert_eq!(image_pixel(&shell, id, 3, 3), Some([255, 255, 0, 255]));
    }

    /// The eyedropper reads the pixel under the pointer into the
    /// brush model — the color model, not the document.
    #[test]
    fn the_eyedropper_picks_the_color_under_the_pointer() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::Eyedropper;
        let picked = shell.editor.pick_color(geom::Point::new(2.0, 2.0));
        assert_eq!(picked, Some([255, 0, 0, 255]));
        assert_eq!(shell.editor.brush.color, [255, 0, 0, 255]);
        assert_eq!(
            image_pixel(&shell, id, 2, 2),
            Some([255, 0, 0, 255]),
            "the pick does not touch the document"
        );
    }

    /// The gradient tool paints a ramp between the press and the
    /// release points.
    #[test]
    fn the_gradient_paints_a_ramp_between_its_handles() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::Gradient;
        shell.editor.brush.color = [0, 0, 0, 255];

        shell
            .editor
            .begin_stroke(geom::Point::new(0.0, 0.0))
            .expect("the gradient starts");
        shell
            .editor
            .stroke_to(geom::Point::new(3.0, 0.0))
            .expect("the gradient follows");
        shell.editor.end_stroke().expect("the gradient paints");

        assert_eq!(image_pixel(&shell, id, 0, 0), Some([0, 0, 0, 255]));
        assert_eq!(
            image_pixel(&shell, id, 3, 0),
            Some([255, 255, 255, 255]),
            "the ramp ends in white"
        );
    }

    /// File ▸ Open on a plain raster opens it as its own document,
    /// with the canvas sized to the image.
    #[test]
    fn open_raster_sizes_the_document_to_the_image() {
        let mut shell = gui_shell(Editor::untitled());
        shell
            .open_raster_document(&red_png(), "red.png")
            .expect("open");
        let document = shell.editor.document();
        let canvas = document.pages[0].info.canvas.clone();
        assert_eq!((canvas.width, canvas.height), (4.0, 4.0));
        let cell = shell.editor.current_cell().expect("the page's cell");
        assert_eq!(cell.elements.len(), 1);
    }

    /// A file that is neither a Fireworks PNG nor a raster is
    /// refused with a message rather than a panic.
    #[test]
    fn open_raster_refuses_a_non_image() {
        let mut shell = gui_shell(Editor::untitled());
        let error = shell.open_raster_document(b"not an image at all", "x.bin");
        assert!(error.is_err(), "the decode is refused");
    }
}
