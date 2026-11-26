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

#[cfg(not(target_arch = "wasm32"))]
struct FireCraftApp {
    editor: app::shell::Shell,
}

#[cfg(not(target_arch = "wasm32"))]
impl FireCraftApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut editor = app::shell::Shell::new(cc);
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|a| a == "--no-welcome") {
            editor.show_welcome = false;
        }
        if args.iter().any(|a| a == "--blank") {
            editor.show_welcome = false;
            editor.run_shell_new_blank();
        }
        Self { editor }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl eframe::App for FireCraftApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let bg = self.editor.active_palette().bg;
        [
            f32::from(bg.r()) / 255.0,
            f32::from(bg.g()) / 255.0,
            f32::from(bg.b()) / 255.0,
            1.0,
        ]
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.editor.save(storage);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.editor.ui(ui, frame);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("logo.png"))
        .ok()?
        .to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

/// Run the main eframe application.
#[cfg(not(target_arch = "wasm32"))]
fn run_native_app() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([960.0, 640.0])
        .with_decorations(true)
        .with_title("FireCraft Studio");
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport,
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "FireCraft Studio",
        options,
        Box::new(|cc| Ok(Box::new(FireCraftApp::new(cc)))),
    )
}
