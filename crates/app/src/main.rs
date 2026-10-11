//! `firecraft` — the desktop shell entry point. The wasm face of this
//! crate is the library (consumed by `firecraft-web`); the binary is
//! native-only.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
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

#[cfg(target_arch = "wasm32")]
fn main() {}
