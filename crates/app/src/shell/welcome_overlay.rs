//! Clean-room CS6-style Welcome Screen modal (`Fc` studio launcher).
//!
//! Gives immediate access to Recent/Open, New Document, the 1080p Showcase
//! Demo, Command Palette, Keyboard Shortcuts, and "Start Blank 1080p Canvas".

use super::vector_icons::{self, Icon};
use super::*;
use egui::{Align2, Color32, FontId, Rect, Sense, Stroke, Vec2, pos2};

impl Shell {
    /// Cached logo texture loaded once from the embedded `logo.png`.
    pub(super) fn logo_texture(&self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        let id = egui::Id::new("firecraft_brand_logo_tex");
        if let Some(tex) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
            return Some(tex);
        }
        let bytes = include_bytes!("../logo.png");
        let img = image::load_from_memory(bytes).ok()?;
        let rgba = img.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let color_img = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        let handle = ctx.load_texture("firecraft_brand_logo", color_img, TextureOptions::LINEAR);
        ctx.data_mut(|d| d.insert_temp(id, handle.clone()));
        Some(handle)
    }

    /// Render the CS6-style Welcome Screen modal over the workspace when `show_welcome` is true.
    pub(crate) fn render_welcome_overlay(&mut self, ctx: &egui::Context) {
        if !self.show_welcome {
            return;
        }
        let p = self.active_palette();
        let logo = self.logo_texture(ctx);
        let mut close_welcome = false;
        let mut action_load_showcase = false;
        let mut action_blank_1080p = false;
        let mut action_open = false;
        let mut action_palette = false;
        let mut action_shortcuts = false;
        let mut action_about = false;

        egui::Window::new("Welcome to FireCraft Studio")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .movable(false)
            .fixed_size([624.0, 412.0])
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                egui::Frame::new()
                    .fill(p.welcome)
                    .stroke(Stroke::new(1.0, p.stroke))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 10],
                        blur: 28,
                        spread: 0,
                        color: Color32::from_black_alpha(150),
                    })
                    .inner_margin(egui::Margin::same(4)),
            )
            .show(ctx, |ui| {
                let (panel_rect, _) =
                    ui.allocate_exact_size(Vec2::new(614.0, 362.0), Sense::hover());
                let painter = ui.painter();

                // 1. Dark Gold Header Banner
                let title_bar = Rect::from_min_size(panel_rect.min, Vec2::new(614.0, 74.0));
                painter.rect_filled(title_bar, 0.0, p.banner);
                // Subtle diagonal sheen bands
                for i in 0..8 {
                    let x = title_bar.left() + 60.0 + i as f32 * 76.0;
                    painter.line_segment(
                        [pos2(x, title_bar.top()), pos2(x - 28.0, title_bar.bottom())],
                        Stroke::new(16.0, Color32::from_white_alpha(6)),
                    );
                }

                let mut text_left = title_bar.left() + 20.0;
                if let Some(tex) = &logo {
                    let logo_r = Rect::from_min_size(
                        pos2(title_bar.left() + 16.0, title_bar.top() + 10.0),
                        Vec2::splat(36.0),
                    );
                    painter.image(
                        tex.id(),
                        logo_r,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    text_left = logo_r.right() + 10.0;
                }
                painter.text(
                    pos2(text_left, title_bar.top() + 26.0),
                    Align2::LEFT_CENTER,
                    "Fc",
                    FontId::proportional(32.0),
                    p.banner_text,
                );
                painter.text(
                    pos2(title_bar.left() + 20.0, title_bar.top() + 58.0),
                    Align2::LEFT_CENTER,
                    "FireCraft Studio · Open-Source Vector, Bitmap & Web Graphics Editor",
                    FontId::proportional(12.0),
                    p.banner_text,
                );

                // Close button
                let close_btn = Rect::from_min_size(
                    pos2(panel_rect.right() - 30.0, title_bar.top() + 8.0),
                    Vec2::new(22.0, 20.0),
                );
                let close_resp = ui.interact(
                    close_btn,
                    ui.make_persistent_id("welcome-close"),
                    Sense::click(),
                );
                painter.rect_filled(
                    close_btn,
                    0.0,
                    if close_resp.hovered() {
                        p.accent.gamma_multiply(0.55)
                    } else {
                        Color32::from_black_alpha(90)
                    },
                );
                painter.text(
                    close_btn.center(),
                    Align2::CENTER_CENTER,
                    "×",
                    FontId::proportional(13.0),
                    Color32::WHITE,
                );
                if close_resp.on_hover_text("Close Welcome Screen").clicked() {
                    close_welcome = true;
                }

                let draw_fc_mini_badge = |pt: egui::Pos2| {
                    let b = Rect::from_min_size(pt, Vec2::splat(14.0));
                    painter.rect_filled(b, 0.0, Color32::from_rgb(36, 34, 20));
                    painter.rect_stroke(
                        b,
                        0.0,
                        Stroke::new(1.0, p.banner_text),
                        egui::StrokeKind::Middle,
                    );
                    painter.text(
                        b.center(),
                        Align2::CENTER_CENTER,
                        "Fc",
                        FontId::proportional(8.5),
                        p.banner_text,
                    );
                };

                // 2. Two-Column Body: Open / Templates | Create New + Extend
                let body_top = title_bar.bottom() + 14.0;
                let body_bottom = panel_rect.bottom() - 88.0;
                let mid_x = panel_rect.center().x;
                let col1_x = panel_rect.left() + 24.0;
                let col2_x = mid_x + 18.0;

                painter.line_segment(
                    [pos2(mid_x, body_top - 4.0), pos2(mid_x, body_bottom - 6.0)],
                    Stroke::new(1.0, p.stroke),
                );

                painter.text(
                    pos2(col1_x, body_top),
                    Align2::LEFT_TOP,
                    "Open or Start From Sample",
                    FontId::proportional(13.0),
                    p.text,
                );
                for (idx, (label, is_folder)) in [
                    ("Showcase_1080p_UI_Kit.fw.png", false),
                    ("Web_Hero_Banner.fw.png", false),
                    ("FireCraft_Studio_Demo.fw.png", false),
                    ("Open Document…", true),
                ]
                .iter()
                .enumerate()
                {
                    let r = Rect::from_min_size(
                        pos2(col1_x, body_top + 24.0 + idx as f32 * 24.0),
                        Vec2::new(250.0, 20.0),
                    );
                    let resp = ui.interact(
                        r,
                        ui.make_persistent_id(("welcome-recent", idx)),
                        Sense::click(),
                    );
                    if resp.hovered() {
                        painter.rect_filled(r, 0.0, p.accent.gamma_multiply(0.32));
                    }
                    let ic_r =
                        Rect::from_min_size(pos2(r.left() + 2.0, r.top() + 3.0), Vec2::splat(14.0));
                    if *is_folder {
                        vector_icons::draw(painter, ic_r, Icon::Folder, p.banner_text, 1.1);
                    } else {
                        vector_icons::draw(painter, ic_r, Icon::New, p.text_dim, 1.0);
                    }
                    painter.text(
                        pos2(ic_r.right() + 6.0, r.center().y),
                        Align2::LEFT_CENTER,
                        *label,
                        FontId::proportional(11.5),
                        p.text,
                    );
                    if resp.clicked() {
                        close_welcome = true;
                        if *is_folder {
                            action_open = true;
                        } else {
                            action_load_showcase = true;
                        }
                    }
                }

                painter.text(
                    pos2(col2_x, body_top),
                    Align2::LEFT_TOP,
                    "Create New",
                    FontId::proportional(13.0),
                    p.text,
                );
                for (idx, label) in [
                    "Blank FireCraft Document (1080p)",
                    "From Template (1080p Showcase)",
                ]
                .iter()
                .enumerate()
                {
                    let r = Rect::from_min_size(
                        pos2(col2_x, body_top + 24.0 + idx as f32 * 24.0),
                        Vec2::new(255.0, 20.0),
                    );
                    let resp = ui.interact(
                        r,
                        ui.make_persistent_id(("welcome-create", idx)),
                        Sense::click(),
                    );
                    if resp.hovered() {
                        painter.rect_filled(r, 0.0, p.accent.gamma_multiply(0.32));
                    }
                    draw_fc_mini_badge(pos2(r.left() + 2.0, r.top() + 3.0));
                    painter.text(
                        pos2(r.left() + 22.0, r.center().y),
                        Align2::LEFT_CENTER,
                        *label,
                        FontId::proportional(11.5),
                        p.text,
                    );
                    if resp.clicked() {
                        close_welcome = true;
                        if idx == 0 {
                            action_blank_1080p = true;
                        } else {
                            action_load_showcase = true;
                        }
                    }
                }

                let extend_top = body_top + 82.0;
                painter.text(
                    pos2(col2_x, extend_top),
                    Align2::LEFT_TOP,
                    "Extend & Navigate",
                    FontId::proportional(13.0),
                    p.text,
                );
                for (idx, label) in [
                    "Command Palette (Ctrl+P / Ctrl+K) »",
                    "Keyboard Shortcuts (F1) »",
                ]
                .iter()
                .enumerate()
                {
                    let r = Rect::from_min_size(
                        pos2(col2_x, extend_top + 22.0 + idx as f32 * 22.0),
                        Vec2::new(255.0, 19.0),
                    );
                    let resp = ui.interact(
                        r,
                        ui.make_persistent_id(("welcome-extend", idx)),
                        Sense::click(),
                    );
                    if resp.hovered() {
                        painter.rect_filled(r, 0.0, p.accent.gamma_multiply(0.32));
                    }
                    let ic_r =
                        Rect::from_min_size(pos2(r.left() + 2.0, r.top() + 2.5), Vec2::splat(14.0));
                    vector_icons::draw(painter, ic_r, Icon::Group, p.selection, 1.0);
                    painter.text(
                        pos2(r.left() + 22.0, r.center().y),
                        Align2::LEFT_CENTER,
                        *label,
                        FontId::proportional(11.0),
                        p.text,
                    );
                    if resp.clicked() {
                        close_welcome = true;
                        if idx == 0 {
                            action_palette = true;
                        } else {
                            action_shortcuts = true;
                        }
                    }
                }

                // 3. Lower Resources & Brand Badge Section
                painter.line_segment(
                    [
                        pos2(panel_rect.left() + 8.0, body_bottom),
                        pos2(panel_rect.right() - 8.0, body_bottom),
                    ],
                    Stroke::new(1.0, p.stroke),
                );
                painter.line_segment(
                    [
                        pos2(mid_x, body_bottom + 6.0),
                        pos2(mid_x, panel_rect.bottom() - 6.0),
                    ],
                    Stroke::new(1.0, p.stroke),
                );

                for (idx, label) in [
                    "Getting Started »",
                    "Keyboard Shortcuts »",
                    "About FireCraft »",
                ]
                .iter()
                .enumerate()
                {
                    let r = Rect::from_min_size(
                        pos2(col1_x, body_bottom + 8.0 + idx as f32 * 23.0),
                        Vec2::new(220.0, 19.0),
                    );
                    let resp = ui.interact(
                        r,
                        ui.make_persistent_id(("welcome-links", idx)),
                        Sense::click(),
                    );
                    if resp.hovered() {
                        painter.rect_filled(r, 0.0, p.accent.gamma_multiply(0.32));
                    }
                    draw_fc_mini_badge(pos2(r.left() + 2.0, r.top() + 2.5));
                    painter.text(
                        pos2(r.left() + 22.0, r.center().y),
                        Align2::LEFT_CENTER,
                        *label,
                        FontId::proportional(11.0),
                        p.text,
                    );
                    if resp.clicked() {
                        close_welcome = true;
                        match idx {
                            0 => action_load_showcase = true,
                            1 => action_shortcuts = true,
                            _ => action_about = true,
                        }
                    }
                }

                // Bottom-right Fc Emblem Badge + Flame Mark
                let badge_r =
                    Rect::from_min_size(pos2(col2_x + 4.0, body_bottom + 18.0), Vec2::splat(42.0));
                painter.rect_filled(badge_r, 0.0, Color32::from_rgb(36, 34, 20));
                painter.rect_stroke(
                    badge_r,
                    0.0,
                    Stroke::new(2.0, p.banner_text),
                    egui::StrokeKind::Middle,
                );
                painter.text(
                    badge_r.center(),
                    Align2::CENTER_CENTER,
                    "Fc",
                    FontId::proportional(20.0),
                    p.banner_text,
                );
                if let Some(tex) = &logo {
                    let flame_r = Rect::from_min_size(
                        pos2(badge_r.right() + 12.0, body_bottom + 15.0),
                        Vec2::splat(48.0),
                    );
                    painter.image(
                        tex.id(),
                        flame_r,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut self.dont_show_welcome_again, "Don't show again")
                        .changed()
                        && self.dont_show_welcome_again
                    {
                        close_welcome = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Start Blank 1080p Canvas").clicked() {
                            action_blank_1080p = true;
                            close_welcome = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Load 1080p Showcase Demo")
                                        .color(p.accent_text),
                                )
                                .fill(p.accent),
                            )
                            .clicked()
                        {
                            action_load_showcase = true;
                            close_welcome = true;
                        }
                    });
                });
            });

        if close_welcome || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_welcome = false;
        }
        if action_load_showcase {
            self.load_demo_showcase();
        } else if action_blank_1080p {
            self.new_blank_1080p_document();
        } else if action_open {
            self.run_shell(menus::ShellAction::Open);
        } else if action_palette {
            self.toggle_palette();
        } else if action_shortcuts {
            self.help_open = true;
        } else if action_about {
            self.about_open = true;
        }
    }
}
