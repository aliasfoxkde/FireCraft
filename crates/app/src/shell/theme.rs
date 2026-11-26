//! CS6-faithful Charcoal Dark & Classic Light theme engine.
//!
//! All widgets, panels, menus, and windows use `CornerRadius::ZERO`, crisp
//! 1 px borders, dark widget fills (fixing egui's default light widget fills),
//! and tight desktop typography.

use egui::{Color32, CornerRadius, Stroke, Visuals};

use super::glyphs::{
    CHROME_ACCENT, CHROME_ACCENT_STROKE, CHROME_CANVAS_FILL, CHROME_PANEL_FILL,
    CHROME_SURFACE_FILL, CHROME_TEXT, CHROME_WINDOW_FILL, system_is_light,
};

/// Selectable workspace theme mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum ThemeMode {
    /// Crisp Fireworks CS6 Charcoal Dark theme (default).
    #[default]
    Dark,
    /// Measured Fireworks CS6 Classic Light chrome (`#D6D6D6` / `#EDEDED` / `#999999`).
    Light,
    /// Follow the host operating system's light/dark preference.
    System,
}

impl ThemeMode {
    /// All selectable modes in menu order.
    pub const ALL: [Self; 3] = [Self::Dark, Self::Light, Self::System];

    /// Human-readable label shown in the header theme switcher and Preferences.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark (CS6 Charcoal)",
            Self::Light => "Light (CS6 Classic)",
            Self::System => "System Theme",
        }
    }
}

/// Complete colour palette for one workspace theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub surface: Color32,
    pub well: Color32,
    pub stroke: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub accent_stroke: Color32,
    pub accent_text: Color32,
    pub canvas_pad: Color32,
    pub selection: Color32,
    pub banner: Color32,
    pub banner_text: Color32,
    pub welcome: Color32,
}

impl Palette {
    /// Whether this palette is dark (relative luminance < 128).
    #[must_use]
    pub fn is_dark(&self) -> bool {
        let c = self.bg;
        let luma =
            0.2126 * f32::from(c.r()) + 0.7152 * f32::from(c.g()) + 0.0722 * f32::from(c.b());
        luma < 128.0
    }
}

/// Charcoal Dark palette — keeps every pinned `CHROME_*` constant exact so the
/// WCAG AAA contrast gate (`glyphs::tests`) remains locked.
pub const DARK: Palette = Palette {
    bg: CHROME_CANVAS_FILL,
    panel: CHROME_PANEL_FILL,
    panel_alt: CHROME_WINDOW_FILL,
    surface: CHROME_SURFACE_FILL,
    well: Color32::from_rgb(0x22, 0x22, 0x22),
    stroke: Color32::from_rgb(0x55, 0x55, 0x55),
    text: CHROME_TEXT,
    text_dim: Color32::from_rgb(0xaa, 0xaa, 0xaa),
    accent: CHROME_ACCENT,
    accent_stroke: CHROME_ACCENT_STROKE,
    accent_text: Color32::WHITE,
    canvas_pad: CHROME_CANVAS_FILL,
    selection: Color32::from_rgb(0x38, 0x98, 0xff),
    banner: Color32::from_rgb(56, 50, 22),
    banner_text: Color32::from_rgb(248, 202, 24),
    welcome: Color32::from_rgb(0x32, 0x32, 0x32),
};

/// Measured Fireworks CS6 Classic Light chrome (`#D6D6D6` / `#EDEDED` / `#999999`).
pub const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(0x99, 0x99, 0x99),
    panel: Color32::from_rgb(0xd6, 0xd6, 0xd6),
    panel_alt: Color32::from_rgb(0xc4, 0xc4, 0xc4),
    surface: Color32::from_rgb(0xe2, 0xe2, 0xe2),
    well: Color32::from_rgb(0xed, 0xed, 0xed),
    stroke: Color32::from_rgb(0x9a, 0x9a, 0x9a),
    text: Color32::from_rgb(0x18, 0x18, 0x1a),
    text_dim: Color32::from_rgb(0x50, 0x50, 0x56),
    accent: Color32::from_rgb(0x2a, 0x6f, 0xaf),
    accent_stroke: Color32::from_rgb(0x18, 0x78, 0xe6),
    accent_text: Color32::WHITE,
    canvas_pad: Color32::from_rgb(0x99, 0x99, 0x99),
    selection: Color32::from_rgb(0x18, 0x78, 0xe6),
    banner: Color32::from_rgb(58, 52, 22),
    banner_text: Color32::from_rgb(245, 196, 0),
    welcome: Color32::from_rgb(0xed, 0xed, 0xed),
};

/// Resolve a [`ThemeMode`] to a concrete [`Palette`].
#[must_use]
pub fn resolve(mode: ThemeMode) -> Palette {
    match mode {
        ThemeMode::Dark => DARK,
        ThemeMode::Light => LIGHT,
        ThemeMode::System => {
            if system_is_light() {
                LIGHT
            } else {
                DARK
            }
        }
    }
}

/// Apply crisp, square-cornered Fireworks CS6 desktop chrome to `ctx`.
pub fn apply_cs6_style(ctx: &egui::Context, p: &Palette, dark: bool) {
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.dark_mode = dark;
    v.override_text_color = Some(p.text);
    v.panel_fill = p.panel;
    v.window_fill = p.panel_alt;
    v.extreme_bg_color = p.bg;
    v.faint_bg_color = p.surface;
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;

    let stroke = Stroke::new(1.0, p.stroke);
    let accent_stroke = Stroke::new(1.0, p.accent_stroke);
    let text_stroke = Stroke::new(1.0, p.text);

    v.widgets.noninteractive.bg_fill = p.panel_alt;
    v.widgets.noninteractive.weak_bg_fill = p.panel;
    v.widgets.noninteractive.bg_stroke = stroke;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text_dim);
    v.widgets.noninteractive.corner_radius = CornerRadius::ZERO;

    v.widgets.inactive.bg_fill = p.panel_alt;
    v.widgets.inactive.weak_bg_fill = p.panel_alt;
    v.widgets.inactive.bg_stroke = stroke;
    v.widgets.inactive.fg_stroke = text_stroke;
    v.widgets.inactive.corner_radius = CornerRadius::ZERO;

    v.widgets.hovered.bg_fill = p.surface;
    v.widgets.hovered.weak_bg_fill = p.surface;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.hovered.fg_stroke = text_stroke;
    v.widgets.hovered.corner_radius = CornerRadius::ZERO;
    v.widgets.hovered.expansion = 0.0;

    v.widgets.active.bg_fill = p.accent;
    v.widgets.active.weak_bg_fill = p.accent;
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.selection);
    v.widgets.active.fg_stroke = Stroke::new(1.0, p.accent_text);
    v.widgets.active.corner_radius = CornerRadius::ZERO;
    v.widgets.active.expansion = 0.0;

    v.widgets.open.bg_fill = p.panel_alt;
    v.widgets.open.weak_bg_fill = p.panel;
    v.widgets.open.bg_stroke = stroke;
    v.widgets.open.fg_stroke = text_stroke;
    v.widgets.open.corner_radius = CornerRadius::ZERO;
    v.widgets.open.expansion = 0.0;

    v.selection.bg_fill = p.accent;
    v.selection.stroke = accent_stroke;
    v.hyperlink_color = p.selection;
    v.window_stroke = Stroke::new(1.0, p.accent.gamma_multiply(0.85));
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(125),
    };
    v.popup_shadow = egui::epaint::Shadow {
        offset: [2, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(105),
    };

    ctx.all_styles_mut(|style| {
        style.visuals = v.clone();
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(12.5));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(12.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(10.5));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(15.5));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, egui::FontId::monospace(11.5));
        style.spacing.menu_margin = egui::Margin::same(2);
        style.spacing.button_padding = egui::Vec2::new(7.0, 2.5);
        style.spacing.item_spacing = egui::Vec2::new(6.0, 3.5);
        style.spacing.window_margin = egui::Margin::same(10);
        style.spacing.interact_size.y = 20.0;
        style.spacing.combo_width = 115.0;
        style.spacing.slider_width = 115.0;
    });
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn relative_luminance(c: Color32) -> f64 {
        let ch = |v: u8| {
            let s = f64::from(v) / 255.0;
            if s <= 0.039_28 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
    }

    fn contrast_ratio(a: Color32, b: Color32) -> f64 {
        let (la, lb) = (relative_luminance(a), relative_luminance(b));
        let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn dark_and_light_palettes_meet_wcag_aa() {
        for (name, p) in [("dark", DARK), ("light", LIGHT)] {
            for (surface, color) in [
                ("panel", p.panel),
                ("panel_alt", p.panel_alt),
                ("well", p.well),
                ("bg", p.bg),
            ] {
                let ratio = contrast_ratio(p.text, color);
                assert!(
                    ratio >= 4.5,
                    "{name}: text on {surface} is {ratio:.2}:1 (< 4.5:1)"
                );
            }
            let accent = contrast_ratio(p.accent_text, p.accent);
            assert!(
                accent >= 4.5,
                "{name}: accent label contrast is {accent:.2}:1 (< 4.5:1)"
            );
        }
    }

    #[test]
    fn apply_cs6_style_sets_dark_widget_fills() {
        let ctx = egui::Context::default();
        apply_cs6_style(&ctx, &DARK, true);
        let visuals = ctx.style_of(egui::Theme::Dark).visuals.clone();
        assert_eq!(visuals.widgets.inactive.bg_fill, DARK.panel_alt);
        assert_eq!(visuals.widgets.hovered.bg_fill, DARK.surface);
        assert_eq!(visuals.widgets.active.bg_fill, DARK.accent);
    }
}
