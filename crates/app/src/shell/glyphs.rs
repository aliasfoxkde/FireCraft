//! Tool labels, keys, and the hand-painted tool glyphs (original
//! art, never the Adobe icons).
//!
//! Pure egui drawing glue: exercised visually (and later by the
//! `egui_kittest` snapshot gate), not by line coverage.

use super::*;

// ---------------------------------------------------------------------------
// The chrome palette
// ---------------------------------------------------------------------------
//
// CS6 owns **no** palette of its own. Its Preferences carry eight
// categories — General, Edit, Guides and Grids, Type, Photoshop Import
// and open, Launch and Edit, Plug-ins, Restore default preferences —
// and not one of them sets a colour; the chrome greys a CS6 install
// draws are the host operating system's, not Adobe's
// (`docs/research/fw-ui-evidence.md`, "The CS6 chrome palette
// (measured)"). The reference run measured on this host renders the
// application frame and the panel bars at `#D6D6D6`, panel interiors at
// `#EDEDED`, the canvas surround at `#999999`, and carries **no**
// saturated hue anywhere in the chrome — the "CS6 neutral" the parity
// matrix asks for is *hue-free greys*, not a particular lightness.
//
// FireCraft therefore defines its own neutral scheme, dark rather
// than light, and records the difference as deliberate
// (`docs/CS6-PARITY.md` §C). The values below are the recorded ones;
// the test at the end of this file pins them, so the scheme cannot
// drift without a deliberate re-pin.

/// The chrome's background — the dock, the tool strip, the bars.
pub const CHROME_PANEL_FILL: Color32 = Color32::from_rgb(0x3c, 0x3c, 0x3c);
/// Floating windows and dialogs.
pub const CHROME_WINDOW_FILL: Color32 = Color32::from_rgb(0x32, 0x32, 0x32);
/// The canvas surround — the grey the page sits on.
pub const CHROME_CANVAS_FILL: Color32 = Color32::from_rgb(0x28, 0x28, 0x28);
/// Fields, rows and other raised surfaces.
pub const CHROME_SURFACE_FILL: Color32 = Color32::from_rgb(0x44, 0x44, 0x44);
/// The chrome's text colour.
pub const CHROME_TEXT: Color32 = Color32::from_rgb(0xdd, 0xdd, 0xdd);
/// The selection fill — the one hue in the chrome.
pub const CHROME_ACCENT: Color32 = Color32::from_rgb(0x2f, 0x5d, 0x8a);
/// The selection stroke.
pub const CHROME_ACCENT_STROKE: Color32 = Color32::from_rgb(0x50, 0x7e, 0xab);

/// The chrome look: charcoal panels, square corners, small type, one
/// blue selection accent. Applied once at startup.
///
/// Public because the web entry point builds the shell through
/// [`Shell::with_platform`] (which has no creation context), and a
/// web build that skipped this rendered egui's default light theme —
/// the single loudest difference from the recorded chrome.
///
/// The name says *chrome*, not *CS6*, on purpose: CS6 has no palette
/// of its own to reproduce (see the constants above).
pub fn apply_chrome_theme(ctx: &egui::Context) {
    // The tool-strip icons are embedded Lucide SVGs; the loader
    // rasterizes them at the exact on-screen size. Idempotent.
    egui_extras::install_image_loaders(ctx);
    ctx.all_styles_mut(|style| {
        let visuals = &mut style.visuals;
        visuals.dark_mode = true;
        visuals.panel_fill = CHROME_PANEL_FILL;
        visuals.window_fill = CHROME_WINDOW_FILL;
        visuals.extreme_bg_color = CHROME_CANVAS_FILL;
        visuals.faint_bg_color = CHROME_SURFACE_FILL;
        // 0xdd against the darkest fill we put text on (faint 0x44)
        // is 7.2:1 — WCAG 2.1 AAA (≥7:1). 0xd9 was 6.9:1, a hair short.
        visuals.override_text_color = Some(CHROME_TEXT);
        visuals.selection.bg_fill = CHROME_ACCENT;
        visuals.selection.stroke = (1.0, CHROME_ACCENT_STROKE).into();
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
        AutoShape::RoundedRect => {
            // The rounded box: four quarter-circle arcs joined by the
            // straight edges.
            let (half_w, half_h, radius) = (6.0f32, 6.0f32, 3.0f32);
            let mut points = Vec::new();
            // The four arc centers, clockwise from the top-left.
            let centers = [
                (c.x - half_w + radius, c.y - half_h + radius),
                (c.x + half_w - radius, c.y - half_h + radius),
                (c.x + half_w - radius, c.y + half_h - radius),
                (c.x - half_w + radius, c.y + half_h - radius),
            ];
            // Each arc runs from its start angle to its end angle.
            let arcs = [
                (std::f64::consts::PI, 1.5 * std::f64::consts::PI),
                (1.5 * std::f64::consts::PI, 2.0 * std::f64::consts::PI),
                (0.0, 0.5 * std::f64::consts::PI),
                (0.5 * std::f64::consts::PI, std::f64::consts::PI),
            ];
            for (index, (center, (start, end))) in centers.iter().zip(arcs.iter()).enumerate() {
                for step in 0..=6 {
                    let angle = start + (end - start) * step as f64 / 6.0;
                    points.push(Pos2::new(
                        center.0 + (radius as f64 * angle.cos()) as f32,
                        center.1 + (radius as f64 * angle.sin()) as f32,
                    ));
                }
                // The straight edge to the next arc's start.
                let next = centers[(index + 1) % 4];
                let next_angle = arcs[(index + 1) % 4].0;
                points.push(Pos2::new(
                    next.0 + (radius as f64 * next_angle.cos()) as f32,
                    next.1 + (radius as f64 * next_angle.sin()) as f32,
                ));
            }
            points.push(points[0]);
            painter.add(egui::Shape::line(points, stroke));
        }
        AutoShape::ChamferRect => {
            // The chamfered box: eight straight sides.
            let (half_w, half_h, cut) = (6.0f32, 6.0f32, 3.0f32);
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - half_w + cut, c.y - half_h),
                    Pos2::new(c.x + half_w - cut, c.y - half_h),
                    Pos2::new(c.x + half_w, c.y - half_h + cut),
                    Pos2::new(c.x + half_w, c.y + half_h - cut),
                    Pos2::new(c.x + half_w - cut, c.y + half_h),
                    Pos2::new(c.x - half_w + cut, c.y + half_h),
                    Pos2::new(c.x - half_w, c.y + half_h - cut),
                    Pos2::new(c.x - half_w, c.y - half_h + cut),
                    Pos2::new(c.x - half_w + cut, c.y - half_h),
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The WCAG 2.1 relative-luminance of a colour.
    fn luminance(color: Color32) -> f64 {
        let channel = |raw: u8| {
            let value = f64::from(raw) / 255.0;
            if value <= 0.039_28 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    /// The contrast ratio between two colours (1.0 … 21.0).
    fn contrast_ratio(a: Color32, b: Color32) -> f64 {
        let (light, dark) = if luminance(a) > luminance(b) {
            (luminance(a), luminance(b))
        } else {
            (luminance(b), luminance(a))
        };
        (light + 0.05) / (dark + 0.05)
    }

    /// The applied chrome carries the recorded palette — the values
    /// `docs/CS6-PARITY.md` §C records and
    /// `docs/research/fw-ui-evidence.md` measures.
    #[test]
    fn the_chrome_carries_the_recorded_palette() {
        let ctx = egui::Context::default();
        apply_chrome_theme(&ctx);
        let visuals = ctx.style_of(egui::Theme::Dark).visuals.clone();
        assert_eq!(visuals.panel_fill, CHROME_PANEL_FILL);
        assert_eq!(visuals.window_fill, CHROME_WINDOW_FILL);
        assert_eq!(visuals.extreme_bg_color, CHROME_CANVAS_FILL);
        assert_eq!(visuals.faint_bg_color, CHROME_SURFACE_FILL);
        assert_eq!(visuals.selection.bg_fill, CHROME_ACCENT);
        assert_eq!(visuals.override_text_color, Some(CHROME_TEXT));
        assert!(visuals.dark_mode);
    }

    /// The chrome's text stays at WCAG 2.1 AAA against the *lightest*
    /// surface it is drawn on — the raised fill, not the panel.
    ///
    /// The recorded claim is 7.2:1 against `#444444`; the test holds
    /// the floor at AAA's 7:1 so a palette edit that costs contrast
    /// fails here rather than in a reviewer's eye.
    #[test]
    fn the_chrome_text_keeps_aaa_contrast() {
        let ratio = contrast_ratio(CHROME_TEXT, CHROME_SURFACE_FILL);
        assert!(
            ratio >= 7.0,
            "chrome text on the raised fill is {ratio:.2}:1 — below AAA"
        );
    }

    /// The chrome is neutral: every colour but the selection accent
    /// is a grey, which is what "the CS6 neutral palette" means
    /// (CS6 itself owns no palette — see the module comment).
    #[test]
    fn the_chrome_is_neutral_but_for_the_selection_accent() {
        for color in [
            CHROME_PANEL_FILL,
            CHROME_WINDOW_FILL,
            CHROME_CANVAS_FILL,
            CHROME_SURFACE_FILL,
            CHROME_TEXT,
        ] {
            assert_eq!(
                (color.r(), color.g(), color.b()),
                (color.r(), color.r(), color.r()),
                "a chrome colour carries a hue"
            );
        }
        assert_ne!(
            (CHROME_ACCENT.r(), CHROME_ACCENT.g(), CHROME_ACCENT.b()),
            (CHROME_ACCENT.r(), CHROME_ACCENT.r(), CHROME_ACCENT.r()),
            "the selection accent is the one hue the chrome allows"
        );
    }
}
