//! The egui face of the editor: menus, panels, canvas. Thin by rule —
//! every document change you see here is an [`Editor::apply`] call,
//! and every panel reads the document read-only. Desktop file routing
//! goes through the injected [`engine::platform`] traits (`EnvDialogs`
//! below; real modal dialogs land with desktop integration).

use doc::ObjectData;
use egui::{Color32, ColorImage, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2};

use crate::canvas::View;
use crate::editor::{AutoShape, Editor, Tool};

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
        let Some(ObjectData::Path(path)) = self.editor.document().registry.get(id) else {
            ui.label("selection is not a path");
            return;
        };
        let path = path.clone();

        ui.label(format!("object {} (gen {})", id.index(), id.generation()));
        // Opacity, per mille.
        let mut opacity = path.opacity;
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
        let blend = (path.blend as usize).min(blend_names.len() - 1);
        egui::ComboBox::from_label("blend")
            .selected_text(blend_names[blend])
            .show_ui(ui, |ui| {
                for (index, name) in blend_names.iter().enumerate() {
                    if ui.selectable_label(index == blend, *name).clicked() {
                        let _ = self.editor.apply(
                            "object.setBlend",
                            serde_json::json!({ "id": id, "blend": index }),
                        );
                    }
                }
            });
        // Visibility / lock.
        ui.horizontal(|ui| {
            if ui.checkbox(&mut path.visible.clone(), "visible").changed() {
                let _ = self.editor.apply(
                    "object.setVisible",
                    serde_json::json!({ "id": id, "visible": !path.visible }),
                );
            }
            if ui.checkbox(&mut path.locked.clone(), "locked").changed() {
                let _ = self.editor.apply(
                    "object.setLocked",
                    serde_json::json!({ "id": id, "locked": !path.locked }),
                );
            }
        });
        // Name.
        let mut name = path.name.clone().unwrap_or_default();
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
        if response.drag_started() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.left_top().to_vec2();
                self.editor.begin_drag(self.view.to_world(local));
            }
        }
        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.left_top().to_vec2();
                if self.editor.drag.is_some() {
                    self.editor.drag_to(self.view.to_world(local));
                } else if self.editor.tool == Tool::Hand
                    || response.dragged_by(egui::PointerButton::Middle)
                {
                    self.view.pan_by(response.drag_delta());
                }
            }
        }
        if response.drag_stopped() {
            if let Err(error) = self.editor.end_drag() {
                self.status = format!("create failed: {error}");
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
    }
}
