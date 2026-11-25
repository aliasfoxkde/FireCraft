//! CS6 Welcome splash overlay — animated logo fade + static splash card.
extern crate image;

use super::*;

impl Shell {
    /// Renders the CS6 Welcome splash overlay (animated or static).
    pub(crate) fn render_welcome_overlay(&mut self, ui: &mut egui::Ui) {
        let t = ui.input(|i| i.time);
        if t < 2.4 {
            self.render_animated_splash(ui, t);
            return;
        }
        if !self.show_welcome {
            return;
        }
        self.render_static_splash(ui);
    }

    fn render_animated_splash(&mut self, ui: &mut egui::Ui, t: f64) {
        let ctx = ui.ctx().clone();
        let alpha = if t < 0.35 {
            (t / 0.35) as f32
        } else if t > 1.9 {
            ((2.4 - t) / 0.5).clamp(0.0, 1.0) as f32
        } else {
            1.0
        };

        // Load the splash logo texture (cached)
        let tex = ctx.data_mut(|d| {
            d.get_temp::<egui::TextureHandle>(egui::Id::new("firecraft_splash_tex"))
        }).unwrap_or_else(|| {
            let bytes = include_bytes!("../logo.png");
            let img = image::load_from_memory(bytes).expect("valid splash logo");
            let size = [img.width() as usize, img.height() as usize];
            let rgba = img.to_rgba8();
            let color_img = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            let handle = ctx.load_texture("firecraft_splash_tex", color_img, egui::TextureOptions::LINEAR);
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("firecraft_splash_tex"), handle.clone()));
            handle
        });

        egui::Area::new(egui::Id::new("cs6_startup_splash"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(&ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(egui::Vec2::new(420.0, 260.0), egui::Sense::click());
                let painter = ui.painter();

                // Dark charcoal background (CS6 dark)
                let bg = egui::Color32::from_rgba_unmultiplied(26, 26, 30, (215.0 * alpha) as u8);
                painter.rect_filled(rect, 0.0, bg);
                let accent = egui::Color32::from_rgb(0x3a, 0x72, 0xa8);
                painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.5, accent.gamma_multiply(alpha)), egui::StrokeKind::Inside);

                // Pulsing logo
                let pulse = 1.0 + 0.04 * (t as f32 * 5.0).sin();
                let logo_sz = egui::Vec2::splat(136.0 * pulse);
                let logo_r = egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.top() + 92.0),
                    logo_sz,
                );
                painter.image(
                    tex.id(),
                    logo_r,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::from_white_alpha((255.0 * alpha) as u8),
                );

                // "FireCraft Studio" text
                painter.text(
                    egui::pos2(rect.center().x, rect.bottom() - 62.0),
                    egui::Align2::CENTER_CENTER,
                    "FireCraft Studio",
                    egui::FontId::proportional(20.0),
                    egui::Color32::from_white_alpha((245.0 * alpha) as u8),
                );

                // Progress bar
                let prog = (t / 2.2).clamp(0.0, 1.0) as f32;
                let bar_bg = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 48.0, rect.bottom() - 32.0),
                    egui::Vec2::new(rect.width() - 96.0, 5.0),
                );
                painter.rect_filled(bar_bg, 0.0, egui::Color32::from_white_alpha((45.0 * alpha) as u8));
                let bar_fg = egui::Rect::from_min_size(
                    bar_bg.min,
                    egui::Vec2::new(bar_bg.width() * prog, bar_bg.height()),
                );
                let accent = egui::Color32::from_rgb(0x3a, 0x72, 0xa8);
                painter.rect_filled(bar_fg, 0.0, accent.gamma_multiply(alpha));

                if resp.clicked() {
                    ctx.request_repaint();
                }
            });
        ctx.request_repaint();
    }

    fn render_static_splash(&self, _ui: &mut egui::Ui) {
        // Placeholder — will be rendered in ui() after panels
    }
}
