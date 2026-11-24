//! `firecraft` — the desktop shell entry point. The wasm face of this
//! crate is the library (consumed by `firecraft-web`); the binary is
//! native-only.
//!
//! Native splash screen: Fireworks CS6 style — appears before the main
//! app window. The splash shows the CS6-style logo with fade-in animation
//! while eframe initializes. After initialization, the splash closes and
//! the main app appears.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    // Show native OS splash before app loads
    show_native_splash();
    // Run the main application
    run_native_app()
}

#[cfg(target_arch = "wasm32")]
fn main() {}

/// Show a native OS splash screen before the main application starts.
/// Mimics Fireworks CS6 splash — centered window with CS6 logo,
/// fade-in animation, and progress bar. User can click to dismiss early.
#[cfg(not(target_arch = "wasm32"))]
fn show_native_splash() {
    use egui::*;
    use std::time::Instant;

    let ctx = Context::default();

    // Load splash logo from the crate's source directory.
    // Path: crates/app/src/logo.png
    let logo_bytes = include_bytes!("logo.png");
    let img = image::load_from_memory(logo_bytes).expect("valid splash logo");
    let size = [img.width() as usize, img.height() as usize];
    let rgba = img.to_rgba8();
    let color_img = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    let tex = ctx.load_texture("splash_logo", color_img, TextureOptions::LINEAR);

    // Minimum splash duration (prevents flash on fast machines)
    let start = Instant::now();
    let min_duration = std::time::Duration::from_millis(800);

    // Use a reasonable center for the splash (hardcoded since we don't have
    // a frame yet — 1920x1080 is standard)
    let splash_pos = Pos2::new(740.0, 385.0); // Centered on 1920x1080
    let splash_size = [420.0, 260.0];

    let mut can_dismiss = false;

    // Run splash loop until user dismisses or minimum time elapses
    loop {
        let t = ctx.input(|i| i.time);
        let elapsed = start.elapsed();

        // Fade-in/fade-out alpha
        let alpha = if t < 0.35 {
            (t / 0.35) as f32
        } else if t > 1.9 {
            ((2.4 - t) / 0.5).clamp(0.0, 1.0) as f32
        } else {
            1.0
        };

        // Render splash window
        egui::Window::new("Splash")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .default_size(Vec2::from(splash_size))
            .fixed_pos(splash_pos)
            .show(&ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter();

                // Dark charcoal background
                let bg = Color32::from_rgba_unmultiplied(26, 26, 30, (215.0 * alpha) as u8);
                painter.rect_filled(rect, 0.0, bg);

                // Accent border
                let accent = Color32::from_rgb(0x3a, 0x72, 0xa8);
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.5, accent.gamma_multiply(alpha)),
                    StrokeKind::Inside,
                );

                // Pulsing logo
                let pulse = 1.0 + 0.04 * (t as f32 * 5.0).sin();
                let logo_sz = Vec2::splat(136.0 * pulse);
                let logo_r =
                    Rect::from_center_size(Pos2::new(rect.center().x, rect.top() + 92.0), logo_sz);
                painter.image(
                    tex.id(),
                    logo_r,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::from_white_alpha((255.0 * alpha) as u8),
                );

                // "FireCraft Studio" text
                painter.text(
                    Pos2::new(rect.center().x, rect.bottom() - 62.0),
                    Align2::CENTER_CENTER,
                    "FireCraft Studio",
                    FontId::proportional(20.0),
                    Color32::from_white_alpha((245.0 * alpha) as u8),
                );

                // Progress bar
                let prog = (t / 2.2).clamp(0.0, 1.0) as f32;
                let bar_bg = Rect::from_min_size(
                    Pos2::new(rect.left() + 48.0, rect.bottom() - 32.0),
                    Vec2::new(rect.width() - 96.0, 5.0),
                );
                painter.rect_filled(bar_bg, 0.0, Color32::from_white_alpha((45.0 * alpha) as u8));
                let bar_fg = Rect::from_min_size(
                    bar_bg.min,
                    Vec2::new(bar_bg.width() * prog, bar_bg.height()),
                );
                painter.rect_filled(bar_fg, 0.0, accent.gamma_multiply(alpha));

                // Check for click on splash area
                let resp = ui.interact(rect, ui.id(), Sense::click());
                if resp.clicked() {
                    can_dismiss = true;
                }
            });

        ctx.request_repaint();

        // Exit if user dismissed or minimum time elapsed
        if can_dismiss || elapsed >= min_duration {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

/// Run the main eframe application.
#[cfg(not(target_arch = "wasm32"))]
fn run_native_app() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("FireCraft"),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "FireCraft",
        options,
        Box::new(|_cc| Ok(Box::new(app::shell::Shell::new(_cc)))),
    )
}
