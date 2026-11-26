//! `firecraft` — the native desktop shell entry point.
//!
//! Release builds use the Windows GUI subsystem so Explorer/taskbar launches do
//! not flash a console. Debug builds retain the console for startup diagnostics.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    run_native_app()
}

#[cfg(target_arch = "wasm32")]
fn main() {}

/// One eframe app/event loop transitions from the floating splash to the
/// editor. Desktop event loops cannot safely be restarted after closing a
/// separate splash window.
#[cfg(not(target_arch = "wasm32"))]
struct FireCraftApp {
    started: std::time::Instant,
    logo: egui::TextureHandle,
    editor: app::shell::Shell,
    splash_complete: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl FireCraftApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, image::ImageError> {
        let image = image::load_from_memory(include_bytes!("logo.png"))?.to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
        let logo = cc.egui_ctx.load_texture(
            "firecraft-splash-logo",
            color_image,
            egui::TextureOptions::LINEAR,
        );
        let editor = app::shell::Shell::new(cc);
        Ok(Self {
            started: std::time::Instant::now(),
            logo,
            editor,
            splash_complete: false,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl eframe::App for FireCraftApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [30.0 / 255.0, 30.0 / 255.0, 34.0 / 255.0, 1.0]
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.editor.save(storage);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.splash_complete {
            self.editor.ui(ui, frame);
            return;
        }
        let elapsed = self.started.elapsed();
        let duration = std::time::Duration::from_millis(2200);
        if elapsed >= duration {
            self.splash_complete = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([1280.0, 800.0].into()));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::viewport::WindowLevel::Normal,
            ));
            self.editor.ui(ui, frame);
            return;
        }

        let fade = (elapsed.as_secs_f32() / 0.25).clamp(0.0, 1.0);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(30, 30, 34)))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter();
                let border = egui::Color32::from_rgb(58, 114, 168).gamma_multiply(fade);
                painter.rect_stroke(
                    rect.shrink(1.0),
                    5.0,
                    egui::Stroke::new(2.0, border),
                    egui::StrokeKind::Inside,
                );

                let logo_size = egui::Vec2::splat(176.0);
                let logo_rect = egui::Rect::from_center_size(
                    egui::Pos2::new(rect.center().x, rect.center().y - 32.0),
                    logo_size,
                );
                painter.image(
                    self.logo.id(),
                    logo_rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    egui::Color32::from_white_alpha((255.0 * fade) as u8),
                );
                painter.text(
                    egui::Pos2::new(rect.center().x, rect.bottom() - 72.0),
                    egui::Align2::CENTER_CENTER,
                    "FireCraft",
                    egui::FontId::proportional(28.0),
                    egui::Color32::from_white_alpha((245.0 * fade) as u8),
                );
                painter.text(
                    egui::Pos2::new(rect.center().x, rect.bottom() - 42.0),
                    egui::Align2::CENTER_CENTER,
                    "Open Source Raster and Vector Graphics Editor",
                    egui::FontId::proportional(14.0),
                    egui::Color32::from_white_alpha((175.0 * fade) as u8),
                );
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

/// Run the main eframe application.
#[cfg(not(target_arch = "wasm32"))]
fn run_native_app() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 400.0])
            .with_decorations(false)
            .with_transparent(false)
            .with_always_on_top()
            .with_title("FireCraft"),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "FireCraft",
        options,
        Box::new(|cc| Ok(Box::new(FireCraftApp::new(cc)?))),
    )
}
