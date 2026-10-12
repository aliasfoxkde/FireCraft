//! The egui face of the editor: menus, panels, canvas. Thin by rule —
//! every document change you see here is an [`Editor::apply`] call,
//! and every panel reads the document read-only. Desktop file routing
//! goes through the injected [`engine::platform`] traits (`EnvDialogs`
//! below; real modal dialogs land with desktop integration).

use doc::{ObjectData, PathObject, SymbolInstanceObject, SymbolType, TextObject};
use egui::{Color32, ColorImage, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2};

use crate::canvas::View;
use crate::editor::{AutoShape, Editor, NineSliceEdge, Tool};

/// UI label for a symbol type (`MSYi`).
fn symbol_type_label(kind: &SymbolType) -> String {
    match kind {
        SymbolType::Graphic => "graphic".to_owned(),
        SymbolType::Button => "button".to_owned(),
        SymbolType::Animation => "animation".to_owned(),
        SymbolType::Other(raw) => format!("other ({raw})"),
    }
}

/// The quick-add list of the filter panel: internal name plus the
/// `DCVs` defaults that make each kernel do something visible. All of
/// these compose interprets by name (`dropShadow` is the deliberate
/// pass-through control, exactly as in `filtered.fw.png`).
const FILTER_PRESETS: &[(&str, &[(&str, &str)])] = &[
    ("invert", &[]),
    ("gaussianBlur", &[("gaussian_blur_radius", "2")]),
    ("blur", &[]),
    ("blurMore", &[]),
    ("findEdges", &[]),
    (
        "brightnessContrast",
        &[("brightness_amount", "10"), ("contrast_amount", "10")],
    ),
    (
        "hueSaturation",
        &[
            ("hue_amount", "15"),
            ("saturation_amount", "10"),
            ("lightness_amount", "0"),
        ],
    ),
    (
        "unsharpMask",
        &[("unsharp_mask_radius", "2"), ("unsharp_mask_amount", "50")],
    ),
    ("addNoise", &[("add_noise_amount", "10")]),
    ("dropShadow", &[]),
];

/// The display quartet plus name, captured read-only from any
/// property-bearing kind.
struct DisplayState {
    opacity: i64,
    blend: i64,
    visible: bool,
    locked: bool,
    name: Option<String>,
}

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

/// The export formats the dialog offers, in menu order.
const EXPORT_FORMATS: &[&str] = &["png", "jpg", "bmp", "tiff", "wbmp", "svg", "psd", "gif"];

/// Export-dialog state (File → Export…): format, per-format controls,
/// and the cached 2-Up preview. The controls mirror the `EXPv` fields
/// the model maps; Export stamps them onto a cloned document and runs
/// the same codec path the CLI does.
struct ExportDialog {
    /// Index into [`EXPORT_FORMATS`].
    format: usize,
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
}

impl Shell {
    /// Builds the shell with desktop platform defaults.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::with_platform(
            Editor::untitled(),
            Box::new(app_cli::io::StdFs),
            Box::new(EnvDialogs::default()),
        )
    }

    /// Builds the shell over an editor and explicit platform services.
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

    /// File → Open through the injected traits.
    fn open_document(&mut self) {
        let Some(path) = self.dialogs.open_path(
            "Open document",
            &[("Fireworks PNG".to_owned(), vec!["png".to_owned()])],
        ) else {
            return;
        };
        match self
            .fs
            .read_file(&path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| Editor::open(&bytes).map_err(|e| e.to_string()))
        {
            Ok(editor) => {
                self.status = format!("opened {path}");
                self.raster = None;
                self.editor = editor;
                self.view = View::default();
            }
            Err(error) => {
                self.status = format!("open failed: {error}");
            }
        }
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
        // document move.
        let stamp = self.preview_stamp();
        let regenerate = match &dialog.preview {
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
    /// settings, write through the injected file system.
    fn export_via_dialog(&mut self, dialog: &ExportDialog) {
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

    /// The menu bar: File/Edit/Tools/Path/Commands.
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open…").clicked() {
                    self.open_document();
                }
                if ui.button("Save").clicked() {
                    self.save_document();
                }
                if ui.button("Export…").clicked() {
                    self.export_dialog = Some(ExportDialog::default());
                }
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui
                    .add_enabled(
                        self.editor.session.undo_len() > 0,
                        egui::Button::new("Undo"),
                    )
                    .clicked()
                {
                    match self.editor.undo() {
                        Ok(name) => self.status = format!("undid {name}"),
                        Err(error) => self.status = format!("undo failed: {error}"),
                    }
                }
                if ui
                    .add_enabled(
                        self.editor.session.redo_len() > 0,
                        egui::Button::new("Redo"),
                    )
                    .clicked()
                {
                    match self.editor.redo() {
                        Ok(name) => self.status = format!("redid {name}"),
                        Err(error) => self.status = format!("redo failed: {error}"),
                    }
                }
            });
            ui.menu_button("Tools", |ui| {
                for (tool, label) in [
                    (Tool::Pointer, "Pointer (V)"),
                    (Tool::Subselect, "Subselect"),
                    (Tool::Rect, "Rectangle (R)"),
                    (Tool::Ellipse, "Ellipse (E)"),
                    (Tool::Line, "Line (L)"),
                    (Tool::Text, "Text (T)"),
                    (Tool::Zoom, "Zoom (Z)"),
                    (Tool::Hand, "Hand (H)"),
                ] {
                    if ui
                        .selectable_label(self.editor.tool == tool, label)
                        .clicked()
                    {
                        self.editor.tool = tool;
                    }
                }
                // The Auto Shape group: same drag-to-create gesture,
                // one pinned-default payload per shape.
                ui.menu_button("Auto Shape", |ui| {
                    for shape in AutoShape::ALL {
                        let tool = Tool::AutoShape(shape);
                        if ui
                            .selectable_label(self.editor.tool == tool, shape.label())
                            .clicked()
                        {
                            self.editor.tool = tool;
                        }
                    }
                });
            });
            // Path ops on the current selection. Booleans need exactly
            // two live paths; single-object ops need one.
            ui.menu_button("Path", |ui| {
                let pair = self.editor.boolean_pair();
                for (op, label) in [
                    ("union", "Union"),
                    ("intersect", "Intersect"),
                    ("punch", "Punch"),
                    ("crop", "Crop"),
                ] {
                    if ui
                        .add_enabled(pair.is_some(), egui::Button::new(label))
                        .clicked()
                    {
                        if let Some((a, b)) = pair {
                            self.run_command(
                                "path.boolean",
                                serde_json::json!({ "a": a, "b": b, "op": op }),
                            );
                        }
                    }
                }
                ui.separator();
                let id = self.editor.selected_live();
                if ui
                    .add_enabled(id.is_some(), egui::Button::new("Simplify (1px)"))
                    .clicked()
                {
                    self.run_command(
                        "path.simplify",
                        serde_json::json!({ "id": id, "tolerance": 1.0 }),
                    );
                }
                if ui
                    .add_enabled(id.is_some(), egui::Button::new("Expand Stroke (2px)"))
                    .clicked()
                {
                    self.run_command(
                        "path.expandStroke",
                        serde_json::json!({
                            "id": id, "width": 2.0,
                            "cap": "butt", "join": "miter", "miter_limit": 4.0,
                        }),
                    );
                }
                if ui
                    .add_enabled(id.is_some(), egui::Button::new("Inset 2px"))
                    .clicked()
                {
                    self.run_command(
                        "path.inset",
                        serde_json::json!({ "id": id, "distance": 2.0 }),
                    );
                }
                if ui
                    .add_enabled(id.is_some(), egui::Button::new("Outset 2px"))
                    .clicked()
                {
                    self.run_command(
                        "path.inset",
                        serde_json::json!({ "id": id, "distance": -2.0 }),
                    );
                }
            });
            // Registry-driven commands: every catalog id, enabled only
            // when this shell can build a sensible payload for it.
            ui.menu_button("Commands", |ui| {
                for name in self.editor.session.command_names() {
                    let enabled = name == "page.add"
                        || (name == "object.delete" && self.editor.selection.is_some());
                    let payload = if name == "page.add" {
                        serde_json::json!({ "index": null })
                    } else if name == "object.delete" {
                        serde_json::json!({ "id": self.editor.selection })
                    } else {
                        serde_json::Value::Null
                    };
                    if ui.add_enabled(enabled, egui::Button::new(name)).clicked() {
                        if let Err(error) = self.editor.apply(name, payload) {
                            self.status = format!("{name} failed: {error}");
                        }
                    }
                }
            });
        });
    }

    /// Left panel: pages, layers, states.
    fn document_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Pages");
        let pages = self.editor.document().pages.len();
        for index in 0..pages {
            let name = self.editor.document().pages[index]
                .name
                .clone()
                .unwrap_or_else(|| format!("Page {index}"));
            if ui
                .selectable_label(self.editor.page == index, &name)
                .clicked()
            {
                self.editor.page = index;
                self.raster = None;
            }
        }

        ui.separator();
        ui.heading("Layers");
        // Snapshot what the panel shows so closures never borrow the
        // document while the editor is mutable.
        let Some(page) = self.editor.document().pages.get(self.editor.page) else {
            return;
        };
        let layers: Vec<(String, bool)> = page
            .layers
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| {
                let label = if layer.name.is_empty() {
                    format!("Layer {index}")
                } else {
                    layer.name.clone()
                };
                (label, layer.display)
            })
            .collect();
        let states = page
            .layers
            .layers
            .iter()
            .map(|layer| layer.cells.len())
            .max()
            .unwrap_or(1);
        for (layer_index, (label, display)) in layers.iter().enumerate() {
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
                ui.label(label);
            });
        }

        ui.separator();
        ui.heading("States");
        for frame in 0..states {
            if ui
                .selectable_label(self.editor.frame == frame, format!("State {}", frame + 1))
                .clicked()
            {
                self.editor.frame = frame;
                self.raster = None;
            }
        }

        ui.separator();
        self.library_panel(ui);

        ui.separator();
        ui.heading("History");
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

    /// The live effect list: one row per effect with a remove button,
    /// plus a quick-add combo over the kernels compose interprets.
    fn filter_panel(&mut self, ui: &mut egui::Ui, id: doc::ObjectId, effects: &[doc::Effect]) {
        ui.heading("Filters");
        if effects.is_empty() {
            ui.label("(none)");
        }
        for (index, effect) in effects.iter().enumerate() {
            ui.horizontal(|ui| {
                let params = if effect.properties.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", effect.properties.len())
                };
                ui.label(format!("{}. {}{params}", index + 1, effect.internal_name));
                if ui.small_button("remove").clicked() {
                    let _ = self.editor.apply(
                        "filter.remove",
                        serde_json::json!({ "id": id, "index": index }),
                    );
                }
            });
        }
        egui::ComboBox::from_label("add filter")
            .selected_text("add…")
            .show_ui(ui, |ui| {
                for (name, params) in FILTER_PRESETS {
                    if ui.selectable_label(false, *name).clicked() {
                        let _ = self.editor.apply(
                            "filter.add",
                            serde_json::json!({
                                "id": id,
                                "name": name,
                                "params": params
                                    .iter()
                                    .map(|(key, value)| serde_json::json!({
                                        "key": key, "value": value,
                                    }))
                                    .collect::<Vec<_>>(),
                            }),
                        );
                    }
                }
            });
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

        // Rubber band for a creation drag.
        if let Some(drag) = self.editor.drag {
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
                    None => self.editor.begin_drag(world),
                }
            }
        }
        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.left_top().to_vec2();
                let world = self.view.to_world(local);
                if self.editor.guide_drag.is_some() {
                    self.editor.drag_guide_to(world);
                } else if self.editor.drag.is_some() {
                    self.editor.drag_to(world);
                } else if self.editor.tool == Tool::Hand
                    || response.dragged_by(egui::PointerButton::Middle)
                {
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
        // Panels first, CentralPanel last (egui's required order).
        egui::Panel::top(egui::Id::new("menu")).show(ui, |ui| self.menu_bar(ui));
        egui::Panel::left(egui::Id::new("document")).show(ui, |ui| self.document_panel(ui));
        egui::Panel::right(egui::Id::new("properties")).show(ui, |ui| self.properties_panel(ui));
        egui::Panel::bottom(egui::Id::new("status")).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "page {} · state {} · undo {} · redo {} · journal {}",
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
            .show(ui, |ui| self.canvas(ui));
        if self.export_dialog.is_some() {
            egui::Window::new("Export")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.export_dialog_ui(ui));
        }
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
}
