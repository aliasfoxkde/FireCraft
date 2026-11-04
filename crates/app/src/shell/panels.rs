//! The right-hand panel stack: Properties, Pages, States, Layers,
//! History, Styles, Library, Fill & Stroke, plus the help panel and
//! the shared display/filter row builders.
//!
//! Pure egui drawing glue: exercised visually (and later by the
//! `egui_kittest` snapshot gate), not by line coverage.

use super::*;
use compose::abgr_pack;

impl Shell {
    /// Edit ▸ Paste as Mask / Paste Inside: the clipboard becomes the
    /// selection's mask. Both commands read the document's selection,
    /// so it is synced first.
    pub(super) fn paste_as_mask(&mut self, name: &str) {
        if self.editor.live_selection().is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command(name, serde_json::json!({}));
    }

    /// Help ▸ About `FireCraft`: version, license, and project links.
    /// No reference to the reference application beyond the factual
    /// clean-room statement in the docs — the product speaks for
    /// itself.
    pub(super) fn about_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(4.0);
            ui.heading("FireCraft");
            ui.weak(concat!("v", env!("CARGO_PKG_VERSION")));
            ui.add_space(6.0);
            ui.label("A clean-room, open-source design and\nimage editor in pure Rust.");
            ui.add_space(6.0);
            if ui.link("Project home").clicked() {
                let url = "https://github.com/aliasfoxkde/FireCraft";
                let opened = self.dialogs.open_url(url);
                self.status = if opened {
                    format!("opening {url}")
                } else {
                    format!("see {url}")
                };
            }
            if ui.link("Online Help (wiki)").clicked() {
                let url = "https://github.com/aliasfoxkde/FireCraft/wiki";
                let opened = self.dialogs.open_url(url);
                self.status = if opened {
                    format!("opening {url}")
                } else {
                    format!("see {url}")
                };
            }
            ui.add_space(6.0);
            ui.weak("Dual-licensed under Apache-2.0 OR MIT.");
            ui.add_space(4.0);
            if ui.button("Close").clicked() {
                self.about_open = false;
            }
        });
    }

    /// The keyboard-shortcuts help panel: every binding in the
    /// SHORTCUTS table, grouped by the table's section structure.
    pub(super) fn help_panel(&mut self, ctx: &egui::Context) {
        let mut open = self.help_open;
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_width(340.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(
                                    "Plain keys do not fire while a text field has focus.",
                                )
                                .italics(),
                            )
                            .wrap(),
                        );
                        ui.separator();
                        // (title, row count) — mirrors the SHORTCUTS table's
                        // section comments; the sums are asserted in tests.
                        let groups: &[(&str, usize)] = &[
                            ("File", 7),
                            ("Edit", 6),
                            ("Clipboard", 3),
                            ("Modify ▸ Group", 3),
                            ("View", 3),
                            ("Modify ▸ Arrange", 4),
                            ("Modify ▸ Combine Paths", 1),
                            ("Text styles", 3),
                            ("Tools (one-key selects)", 14),
                            ("Pixel-selection tools", 4),
                            ("Help", 1),
                        ];
                        let mut row = 0usize;
                        for (title, count) in groups {
                            ui.heading(*title);
                            egui::Grid::new((*title).to_string())
                                .num_columns(2)
                                .spacing([12.0, 2.0])
                                .show(ui, |ui| {
                                    for s in &menus::SHORTCUTS[row..row + count] {
                                        ui.strong(s.label());
                                        ui.label(menus::action_name(s.action));
                                        ui.end_row();
                                    }
                                });
                            ui.separator();
                            row += count;
                        }
                    });
            });
        self.help_open = open;
    }

    /// Window-menu panel visibility (absent from `panel_hidden` =
    /// shown; CS6 opens everything by default).
    pub(crate) fn panel_shown(&self, name: &str) -> bool {
        !self.panel_hidden.contains(&name)
    }

    /// Toggle one Window-menu panel by its menu label; a panel not in
    /// the right dock (Tools, Optimize, Color Mixer, ...) is reported,
    /// not silently accepted.
    pub(crate) fn toggle_panel(&mut self, name: &'static str) {
        let known = super::panel_windows::PANELS
            .iter()
            .any(|panel| panel.name == name);
        if known {
            if let Some(pos) = self.panel_hidden.iter().position(|n| *n == name) {
                self.panel_hidden.remove(pos);
                self.status = format!("{name} panel shown.");
            } else {
                self.panel_hidden.push(name);
                self.status = format!("{name} panel hidden.");
            }
        } else if name == "Optimize" {
            self.status = format!("{name}: panel not yet implemented.");
        } else {
            self.status = format!("{name}: no docked panel in this shell.");
        }
    }

    /// Window ▸ Tools: fold/unfold the left tools dock.
    pub(crate) fn toggle_tools(&mut self) {
        self.tools_hidden = !self.tools_hidden;
        self.status = if self.tools_hidden {
            "Tools panel hidden."
        } else {
            "Tools panel shown."
        }
        .to_owned();
    }

    /// Window ▸ Hide Panels (the CS6 Tab key): fold/unfold the whole
    /// right dock at once, leaving per-panel visibility untouched.
    pub(crate) fn toggle_all_panels(&mut self) {
        self.panels_hidden = !self.panels_hidden;
        self.status = if self.panels_hidden {
            "Panels hidden — press Tab or Window ▸ Hide Panels to show."
        } else {
            "Panels shown."
        }
        .to_owned();
    }

    /// Window ▸ Toolbars ▸ Main: the context options bar.
    pub(crate) fn toggle_options_bar(&mut self) {
        self.options_bar_hidden = !self.options_bar_hidden;
        self.status = if self.options_bar_hidden {
            "Main toolbar hidden."
        } else {
            "Main toolbar shown."
        }
        .to_owned();
    }

    /// Pages section: one selectable row per page.
    pub(super) fn pages_section(&mut self, ui: &mut egui::Ui) {
        let pages = self.editor.document().pages.len();
        for index in 0..pages {
            let name = self.editor.document().pages[index]
                .name
                .clone()
                .unwrap_or_else(|| format!("Page {}", index + 1));
            if ui
                .selectable_label(self.editor.page == index, &name)
                .clicked()
            {
                self.editor.page = index;
                self.raster = None;
            }
        }
    }

    /// States section: one row per state of the tallest layer.
    pub(super) fn states_section(&mut self, ui: &mut egui::Ui) {
        let states = self
            .editor
            .document()
            .pages
            .get(self.editor.page)
            .map_or(1, |page| {
                page.layers
                    .layers
                    .iter()
                    .map(|layer| layer.cells.len())
                    .max()
                    .unwrap_or(1)
            });
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

    /// Layers section: name + display toggle per layer, bottom-first.
    /// The active layer's row is highlighted and can be changed by
    /// clicking it — that row is where every placement command lands
    /// its object.
    pub(super) fn layers_section(&mut self, ui: &mut egui::Ui) {
        // Snapshot what the panel shows so closures never borrow the
        // document while the editor is mutable.
        let Some(page) = self.editor.document().pages.get(self.editor.page) else {
            return;
        };
        let frame = self.editor.frame;
        let layers: Vec<(String, bool, bool)> = page
            .layers
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| {
                let label = if layer.name.is_empty() {
                    format!("Layer {}", index + 1)
                } else {
                    layer.name.clone()
                };
                // The lock is per state (CS6 locks a layer's cell in
                // one frame), so the row reads the current frame.
                let locked = layer.cells.get(frame).is_some_and(|cell| cell.locked);
                (label, layer.display, locked)
            })
            .collect();
        let count = layers.len();
        let active = self.editor.active_layer();
        for (layer_index, (label, display, locked)) in layers.iter().enumerate() {
            let mut rename = label.clone();
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
                let lock = if *locked { "🔒" } else { "🔓" };
                if ui
                    .button(lock)
                    .on_hover_text("Lock this layer in the current state")
                    .clicked()
                {
                    let _ = self.editor.apply(
                        "layer.setLocked",
                        serde_json::json!({
                            "page": self.editor.page,
                            "layer": layer_index,
                            "frame": frame,
                            "locked": !locked,
                        }),
                    );
                }
                if self.rename_layer == Some(layer_index) {
                    let field = ui
                        .add(egui::TextEdit::singleline(&mut rename).desired_width(72.0))
                        .on_hover_text("rename (Enter applies)");
                    let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
                    if (field.lost_focus() || enter) && rename != *label {
                        self.run_command(
                            "layer.rename",
                            serde_json::json!({
                                "page": self.editor.page,
                                "layer": layer_index,
                                "name": rename,
                            }),
                        );
                    }
                    if field.lost_focus() || enter {
                        self.rename_layer = None;
                    }
                } else if ui
                    .selectable_label(active == layer_index, label)
                    .on_hover_text("Make this the active layer")
                    .clicked()
                {
                    self.editor.set_active_layer(layer_index);
                }
                if ui
                    .small_button("✎")
                    .on_hover_text("Rename this layer")
                    .clicked()
                {
                    self.rename_layer = Some(layer_index);
                }
                if ui
                    .small_button("↑")
                    .on_hover_text("Move this layer up")
                    .clicked()
                {
                    self.reorder_layer(layer_index, true);
                }
                if ui
                    .small_button("↓")
                    .on_hover_text("Move this layer down")
                    .clicked()
                {
                    self.reorder_layer(layer_index, false);
                }
            });
        }
        ui.horizontal(|ui| {
            if ui
                .button("+")
                .on_hover_text("Add a layer above the active one")
                .clicked()
            {
                self.run_command(
                    "layer.add",
                    serde_json::json!({ "page": self.editor.page, "index": null }),
                );
                let top = self.editor.layer_count().saturating_sub(1);
                self.editor.set_active_layer(top);
            }
            if ui
                .button("−")
                .on_hover_text("Delete the active layer")
                .clicked()
                && count > 1
            {
                self.run_command(
                    "layer.delete",
                    serde_json::json!({ "page": self.editor.page, "layer": active }),
                );
                self.editor.set_active_layer(active.saturating_sub(1));
            }
        });
    }

    /// Layers-panel reorder: one slot up (toward the top of the
    /// stack) or down. `before` is the pre-removal index the moved
    /// layer lands in front of, so moving up one slot is `index + 2`.
    pub(super) fn reorder_layer(&mut self, index: usize, up: bool) {
        let count = self.editor.layer_count();
        if count == 0 {
            return;
        }
        let before = if up {
            (index + 2).min(count)
        } else {
            index.saturating_sub(1)
        };
        if before == index || before == index + 1 {
            return; // already at the end of the stack
        }
        let payload = serde_json::json!({
            "page": self.editor.page,
            "from": index,
            "before": before,
        });
        match self.editor.apply("layer.reorder", payload) {
            Ok(()) => {
                // The moved layer keeps the active role; its index
                // shifts by one slot in the direction of the move.
                let moved = if up { index + 1 } else { index - 1 };
                self.editor.set_active_layer(moved);
            }
            Err(error) => self.status = format!("layer.reorder failed: {error}"),
        }
    }

    /// History section: journal rows plus the steps save/replay pair.
    pub(super) fn history_section(&mut self, ui: &mut egui::Ui) {
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
    /// The Styles panel: the document style library (`MKBv`'s `ZSLv`).
    /// A row click applies the bundle onto the selection
    /// (`style.apply` — the CS6 panel's apply), the name field defines
    /// a new (or replaces a same-named) style from the selection's
    /// appearance, and Import merges a CS6 `.stl` library through
    /// [`Editor::import_styles`].
    pub(super) fn styles_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Styles");
        // (index, label) — snapshotted so closures never borrow the
        // document while the editor is mutable (the panel rule).
        let rows: Vec<(usize, String)> = self
            .editor
            .document()
            .styles
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let label = if entry.kind == 0x1f {
                    format!("{} · text", entry.name)
                } else {
                    entry.name.clone()
                };
                (index, label)
            })
            .collect();
        let targets: Vec<doc::ObjectId> = self.editor.selection_set.clone();
        for (index, label) in &rows {
            if ui
                .selectable_label(false, label)
                .on_hover_text("apply onto the selection")
                .clicked()
            {
                self.run_command(
                    "style.apply",
                    serde_json::json!({ "index": index, "ids": targets }),
                );
            }
        }
        if rows.is_empty() {
            ui.label("(empty — define one from a selection or import a library)");
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("name");
            ui.add(egui::TextEdit::singleline(&mut self.style_name).desired_width(96.0));
            if ui
                .button("New From Selection")
                .on_hover_text("redefines a style of this name, or defines a new one")
                .clicked()
                && let Some(source) = targets.last().copied()
                && !self.style_name.is_empty()
            {
                self.run_command(
                    "style.redefineFromSelection",
                    serde_json::json!({ "name": self.style_name, "source": source }),
                );
            }
        });
        if ui.button("Import Style Library...").clicked() {
            self.import_styles_dialog();
        }
    }

    /// Styles panel ▸ Import: reads a CS6 style library (`.stl`)
    /// through the injected traits; entries join the document library,
    /// their preserved texture subtrees join the editor bag.
    pub(super) fn import_styles_dialog(&mut self) {
        let Some(path) = self.dialogs.open_path(
            "Import style library",
            &[("Style library".to_owned(), vec!["stl".to_owned()])],
        ) else {
            return;
        };
        let bytes = match self.fs.read_file(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("import failed: {error}");
                return;
            }
        };
        match self.editor.import_styles(&bytes) {
            Ok(count) => self.status = format!("imported {count} styles from {path}"),
            Err(error) => self.status = format!("import failed: {error}"),
        }
    }

    pub(super) fn library_panel(&mut self, ui: &mut egui::Ui) {
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

    /// The fill/stroke palette: ABGR raws per the node dictionary's
    /// layout, shared by the swatch panel and its tests.
    pub(crate) const FILL_STROKE_PALETTE: [(&str, i64); 12] = [
        ("black", 0xFF00_0000),
        ("white", 0xFFFF_FFFF),
        ("red", 0xFF00_00FF),
        ("orange", 0xFF00_A5FF),
        ("yellow", 0xFF00_FFFF),
        ("green", 0xFF00_8000),
        ("cyan", 0xFFFF_FF00),
        ("blue", 0xFFFF_0000),
        ("purple", 0xFF80_0080),
        ("gray", 0xFF80_8080),
        ("dark gray", 0xFF40_4040),
        ("none (transparent)", 0x0000_0000),
    ];

    /// Fill/stroke swatches for the selection — the CS6 Properties
    /// essentials. The engine's `object.setFill`/`object.setStroke` were
    /// registered with no UI bound to them, leaving every created shape
    /// stuck opaque black.
    pub(super) fn fill_stroke_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Fill & Stroke");
        let ids = self.editor.live_selection();
        if ids.is_empty() {
            ui.label("(select a vector object to edit its colors)");
            return;
        }
        self.gradient_editor_section(ui, &ids);
        for (channel, label) in [("fill", "Fill"), ("stroke", "Stroke")] {
            ui.horizontal(|ui| {
                ui.label(label);
                // The current color of the first selected path, as a
                // full color picker next to the quick swatches.
                let current =
                    ids.iter()
                        .find_map(|id| match self.editor.document().registry.get(*id) {
                            Some(ObjectData::Path(path)) => path.paint.as_ref().map(|paint| {
                                let raw = if channel == "fill" {
                                    paint.fill_raw
                                } else {
                                    paint.stroke_raw
                                } as u32;
                                Color32::from_rgba_unmultiplied(
                                    (raw & 0xFF) as u8,
                                    ((raw >> 8) & 0xFF) as u8,
                                    ((raw >> 16) & 0xFF) as u8,
                                    ((raw >> 24) & 0xFF) as u8,
                                )
                            }),
                            _ => None,
                        });
                if let Some(mut color) = current
                    && ui.color_edit_button_srgba(&mut color).changed()
                {
                    let raw = abgr_pack(color.r(), color.g(), color.b(), color.a());
                    for id in &ids {
                        self.run_command(
                            Self::paint_command(channel),
                            serde_json::json!({ "id": id, "color_raw": raw }),
                        );
                    }
                }
                for (name, color_raw) in Self::FILL_STROKE_PALETTE {
                    let raw = color_raw as u32;
                    let swatch = Color32::from_rgba_unmultiplied(
                        (raw & 0xFF) as u8,
                        ((raw >> 8) & 0xFF) as u8,
                        ((raw >> 16) & 0xFF) as u8,
                        ((raw >> 24) & 0xFF) as u8,
                    );
                    let size = egui::Vec2::splat(16.0);
                    let (response, painter) = ui.allocate_painter(size, egui::Sense::click());
                    let square = response.rect;
                    painter.rect_filled(square, 2.0, swatch);
                    painter.rect_stroke(
                        square,
                        2.0,
                        (1.0, Color32::from_rgb(0x77, 0x77, 0x77)),
                        egui::StrokeKind::Outside,
                    );
                    if response.clicked() {
                        for id in &ids {
                            self.run_command(
                                Self::paint_command(channel),
                                serde_json::json!({ "id": id, "color_raw": color_raw }),
                            );
                        }
                        self.status = format!("{channel} → {name}");
                    }
                }
            });
        }
    }

    /// Window ▸ Swatches: the same ABGR palette the Properties
    /// essentials use, applied to the selection's fill channel — the
    /// CS6 panel's core gesture (click a swatch, paint the selection).
    pub(super) fn swatches_panel(&mut self, ui: &mut egui::Ui) {
        let ids = self.editor.live_selection();
        if ids.is_empty() {
            ui.label("(select a vector object to apply a swatch)");
            return;
        }
        egui::Grid::new("swatches")
            .num_columns(6usize)
            .show(ui, |ui| {
                for (index, (name, color_raw)) in Self::FILL_STROKE_PALETTE.iter().enumerate() {
                    let raw = *color_raw as u32;
                    let swatch = Color32::from_rgba_unmultiplied(
                        (raw & 0xFF) as u8,
                        ((raw >> 8) & 0xFF) as u8,
                        ((raw >> 16) & 0xFF) as u8,
                        ((raw >> 24) & 0xFF) as u8,
                    );
                    let size = egui::Vec2::splat(18.0);
                    let (response, painter) = ui.allocate_painter(size, egui::Sense::click());
                    painter.rect_filled(response.rect, 2.0, swatch);
                    painter.rect_stroke(
                        response.rect,
                        2.0,
                        (1.0, Color32::from_rgb(0x77, 0x77, 0x77)),
                        egui::StrokeKind::Outside,
                    );
                    if response.clicked() {
                        for id in &ids {
                            self.run_command(
                                Self::paint_command("fill"),
                                serde_json::json!({ "id": id, "color_raw": raw }),
                            );
                        }
                        self.status = format!("fill → {name}");
                    }
                    if index % 6 == 5 {
                        ui.end_row();
                    }
                }
            });
    }

    /// Window ▸ Info: document and selection facts (canvas size,
    /// selection count and bounding box) — read-only.
    pub(super) fn info_panel(&mut self, ui: &mut egui::Ui) {
        let document = self.editor.document();
        let canvas = &document.pages[0].info.canvas;
        ui.label(format!("canvas: {} × {}", canvas.width, canvas.height));
        let ids = self.editor.live_selection();
        ui.label(format!("selected: {} object(s)", ids.len()));
        if let Some(bounds) = crate::editor::union_selection_bounds(document, &ids) {
            ui.label(format!(
                "bounds: {:.1}, {:.1} → {:.1}, {:.1} ({} × {})",
                bounds.x0,
                bounds.y0,
                bounds.x1,
                bounds.y1,
                bounds.width(),
                bounds.height()
            ));
        }
    }

    /// Right panel: the align/distribute group, then properties of the
    /// selection.
    pub(super) fn properties_panel(&mut self, ui: &mut egui::Ui) {
        self.fill_stroke_panel(ui);
        ui.separator();
        if matches!(
            self.editor.tool,
            Tool::Paintbrush
                | Tool::Pencil
                | Tool::Eraser
                | Tool::PaintBucket
                | Tool::Gradient
                | Tool::Eyedropper
                | Tool::Marquee
                | Tool::EllipticalMarquee
                | Tool::Lasso
                | Tool::PolygonalMarquee
                | Tool::MagicWand
        ) {
            self.brush_panel(ui);
            ui.separator();
        }
        if matches!(
            self.editor.tool,
            Tool::Marquee
                | Tool::EllipticalMarquee
                | Tool::Lasso
                | Tool::PolygonalMarquee
                | Tool::MagicWand
        ) {
            self.selection_panel(ui);
            ui.separator();
        }
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

    /// The paint tools' slots: the Edit Brush dialog's size, edge,
    /// shape, aspect, angle, pressure and strength, plus the paint
    /// color. The canvas paints with exactly these — the engine reads
    /// them from every stroke's payload.
    pub(super) fn brush_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Brush");
        let brush = self.editor.brush;
        let mut size = f32::from(u16::try_from(brush.size.min(1024)).unwrap_or(u16::MAX));
        if ui
            .add(egui::Slider::new(&mut size, 1.0..=1024.0).text("size"))
            .changed()
        {
            self.editor.brush.size = size.round().clamp(1.0, 1024.0) as u32;
        }
        let mut edge = f32::from(brush.edge);
        if ui
            .add(egui::Slider::new(&mut edge, 0.0..=100.0).text("edge"))
            .changed()
        {
            self.editor.brush.edge = edge.round().clamp(0.0, 100.0) as u8;
        }
        let mut aspect = f32::from(brush.aspect);
        if ui
            .add(egui::Slider::new(&mut aspect, 1.0..=100.0).text("aspect"))
            .changed()
        {
            self.editor.brush.aspect = aspect.round().clamp(1.0, 100.0) as u8;
        }
        let mut angle = f32::from(brush.angle);
        if ui
            .add(egui::Slider::new(&mut angle, -180.0..=180.0).text("angle"))
            .changed()
        {
            self.editor.brush.angle = angle.round().clamp(-180.0, 180.0) as i16;
        }
        let mut pressure = f32::from(brush.pressure);
        if ui
            .add(egui::Slider::new(&mut pressure, 1.0..=100.0).text("pressure"))
            .changed()
        {
            self.editor.brush.pressure = pressure.round().clamp(1.0, 100.0) as u8;
        }
        let mut strength = f32::from(brush.strength);
        if ui
            .add(egui::Slider::new(&mut strength, 1.0..=100.0).text("strength"))
            .changed()
        {
            self.editor.brush.strength = strength.round().clamp(1.0, 100.0) as u8;
        }
        ui.horizontal(|ui| {
            ui.label("shape");
            for (shape, label) in [(StampShape::Round, "round"), (StampShape::Square, "square")] {
                if ui.selectable_label(brush.shape == shape, label).clicked() {
                    self.editor.brush.shape = shape;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("color");
            let mut color = egui::Color32::from_rgba_unmultiplied(
                brush.color[0],
                brush.color[1],
                brush.color[2],
                brush.color[3],
            );
            if ui.color_edit_button_srgba(&mut color).changed() {
                self.editor.brush.color = [color.r(), color.g(), color.b(), color.a()];
            }
        });
        if matches!(self.editor.tool, Tool::MagicWand) {
            let mut tolerance = f32::from(self.editor.wand_tolerance);
            if ui
                .add(egui::Slider::new(&mut tolerance, 0.0..=255.0).text("tolerance"))
                .changed()
            {
                self.editor.wand_tolerance = tolerance.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    /// The pixel-selection tools' panel: the mask's presence and
    /// statistics, the mode words, and the feather / adjust / move /
    /// erase actions on it.
    pub(super) fn selection_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Select");
        let active = self.editor.has_pixel_selection();
        if active {
            let (pixels, coverage) =
                self.editor
                    .document()
                    .selection
                    .as_ref()
                    .map_or((0, 0), |mask| {
                        (
                            mask.alpha.iter().filter(|&&a| a != 0).count(),
                            u64::from(mask.alpha.iter().map(|&a| u16::from(a)).sum::<u16>()) * 100
                                / (255 * u64::try_from(mask.alpha.len().max(1)).unwrap_or(1)),
                        )
                    });
            ui.label(format!("{pixels} pixels selected ({coverage}% coverage)"));
        } else {
            ui.label("no pixel selection");
        }
        ui.horizontal(|ui| {
            ui.label("mode");
            for (mode, label) in [
                (PixelSelectMode::Replace, "new"),
                (PixelSelectMode::Add, "add"),
                (PixelSelectMode::Subtract, "subtract"),
                (PixelSelectMode::Intersect, "intersect"),
            ] {
                if ui
                    .selectable_label(self.editor.pixel_mode == mode, label)
                    .clicked()
                {
                    self.editor.pixel_mode = mode;
                }
            }
        });
        ui.separator();
        let mut feather = f32::from(self.editor.feather_radius);
        if ui
            .add(egui::Slider::new(&mut feather, 1.0..=64.0).text("feather"))
            .changed()
        {
            self.editor.feather_radius = feather.round().clamp(1.0, 64.0) as u8;
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(active, egui::Button::new("feather").small())
                .clicked()
            {
                let radius = self.editor.feather_radius;
                if let Err(error) = self.editor.feather_selection(radius) {
                    self.status = format!("feather failed: {error}");
                }
            }
            if ui
                .add_enabled(active, egui::Button::new("deselect").small())
                .clicked()
                && let Err(error) = self.editor.deselect_pixels()
            {
                self.status = format!("deselect failed: {error}");
            }
        });
        ui.horizontal(|ui| {
            for (what, label) in [
                ("expand", "grow"),
                ("contract", "shrink"),
                ("smooth", "smooth"),
            ] {
                if ui
                    .add_enabled(active, egui::Button::new(label).small())
                    .clicked()
                    && let Err(error) = self.editor.adjust_pixel_selection(what, 1)
                {
                    self.status = format!("{label} failed: {error}");
                }
            }
        });
        if matches!(self.editor.tool, Tool::MagicWand) && ui.button("select similar").clicked() {
            let tolerance = self.editor.wand_tolerance;
            if let Err(error) = self.editor.select_similar(tolerance) {
                self.status = format!("similar failed: {error}");
            }
        }
        ui.separator();
        ui.horizontal(|ui| {
            for (dx, dy, label) in [(-1, 0, "◀"), (0, -1, "▲"), (0, 1, "▼"), (1, 0, "▶")] {
                if ui
                    .add_enabled(active, egui::Button::new(label).small())
                    .clicked()
                {
                    if ui.input(|i| i.modifiers.shift) {
                        // Shift moves the pixels; the plain arrows
                        // move the selection frame.
                        let target_move = (dx, dy);
                        if let Err(error) = self
                            .editor
                            .move_selected_pixels(target_move.0, target_move.1)
                        {
                            self.status = format!("move failed: {error}");
                        }
                    } else if let Err(error) = self.editor.move_selection_mask(dx, dy) {
                        self.status = format!("move failed: {error}");
                    }
                }
            }
        });
        if ui
            .add_enabled(active, egui::Button::new("erase pixels").small())
            .clicked()
            && let Err(error) = self.editor.erase_selected()
        {
            self.status = format!("erase failed: {error}");
        }
    }

    /// Symbol-instance properties: the referenced definition, the
    /// button-state inspector (per-state content), the 9-slice
    /// toggle, and the actions on the instance or its definition.
    pub(super) fn symbol_instance_properties(
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
    pub(super) fn path_properties(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        path: &PathObject,
    ) {
        ui.label(format!("object {} (gen {})", id.index(), id.generation()));
        self.geometry_fields(ui, id);
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
    pub(super) fn text_properties(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        text: &TextObject,
    ) {
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
        self.geometry_fields(ui, id);
    }

    /// CS6's Properties X/Y/W/H strip: the selection's union box,
    /// applied on Enter through `object.setBounds`.
    pub(super) fn geometry_fields(&mut self, ui: &mut egui::Ui, id: doc::ObjectId) {
        let document = self.editor.session.document();
        let Some(bounds) = crate::editor::union_selection_bounds(document, &[id]) else {
            return;
        };
        let mut x = bounds.x0 as f32;
        let mut y = bounds.y0 as f32;
        let mut w = bounds.width() as f32;
        let mut h = bounds.height() as f32;
        ui.horizontal(|ui| {
            let x_field = ui.add(
                egui::DragValue::new(&mut x)
                    .speed(1.0)
                    .prefix("X "),
            );
            let y_field = ui.add(
                egui::DragValue::new(&mut y)
                    .speed(1.0)
                    .prefix("Y "),
            );
            let w_field = ui.add(
                egui::DragValue::new(&mut w)
                    .speed(1.0)
                    .prefix("W "),
            );
            let h_field = ui.add(
                egui::DragValue::new(&mut h)
                    .speed(1.0)
                    .prefix("H "),
            );
            let applied = [x_field, y_field, w_field, h_field]
                .into_iter()
                .any(|field| field.lost_focus() && field.changed());
            if applied && w > 0.0 && h > 0.0 {
                let _ = self.editor.apply(
                    "object.setBounds",
                    serde_json::json!({
                        "ids": [id], "left": x, "top": y, "width": w, "height": h,
                    }),
                );
            }
        });
    }

    /// The display quartet plus the object name — shared by every
    /// property-bearing kind.
    pub(super) fn display_controls(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        state: DisplayState,
    ) {
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

    /// The live effect list: one collapsible row per effect (remove +
    /// per-parameter edit rows), plus the family-grouped quick-add
    /// menu over the shared preset registry. Live preview needs
    /// nothing here — every command below invalidates the compose
    /// stamp and the canvas re-renders through the live stack.
    pub(super) fn filter_panel(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        effects: &[doc::Effect],
    ) {
        ui.heading("Filters");
        if effects.is_empty() {
            ui.label("(none)");
        }
        for (index, effect) in effects.iter().enumerate() {
            egui::CollapsingHeader::new(format!("{}. {}", index + 1, effect.internal_name))
                .id_salt(("filter", id, index))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.small_button("remove").clicked() {
                            self.run_command(
                                "filter.remove",
                                serde_json::json!({ "id": id, "index": index }),
                            );
                        }
                    });
                    self.filter_param_rows(ui, id, index, effect);
                });
        }
        self.filter_add_menu(ui, id);
    }

    /// The family-grouped quick-add menu — the same table the Filters
    /// menu submenu casts as `Action::Filter` entries.
    pub(super) fn filter_add_menu(&mut self, ui: &mut egui::Ui, id: doc::ObjectId) {
        ui.menu_button("add filter", |ui| {
            for (family, presets) in FILTER_FAMILIES {
                ui.menu_button(*family, |ui| {
                    for preset in *presets {
                        if ui.selectable_label(false, preset.label).clicked() {
                            self.apply_filter_preset(id, *preset);
                        }
                    }
                });
            }
        });
    }

    /// One quick-add: `filter.add` with the preset's `DCVs` defaults
    /// (curves ride the raw knot list — `value` stays empty there).
    pub(super) fn apply_filter_preset(&mut self, id: doc::ObjectId, preset: FilterPreset) {
        let params: Vec<_> = preset
            .params
            .iter()
            .map(|(key, value)| {
                let mut entry = serde_json::json!({ "key": key, "value": value });
                if !preset.points.is_empty() {
                    entry["points"] = serde_json::json!(preset.points);
                }
                entry
            })
            .collect();
        self.run_command(
            "filter.add",
            serde_json::json!({ "id": id, "name": preset.name, "params": params }),
        );
        self.status = format!("filter: {}", preset.label);
    }

    /// One parameter edit on an applied effect.
    pub(super) fn set_filter_param(
        &mut self,
        id: doc::ObjectId,
        index: usize,
        key: &str,
        value: &str,
    ) {
        self.run_command(
            "filter.setParam",
            serde_json::json!({ "id": id, "index": index, "key": key, "value": value }),
        );
    }

    /// The per-effect parameter rows: documented numeric keys get
    /// range sliders with a reset, boolean keys checkboxes, knot lists
    /// a read-only count with a reset-to-linear, everything else a
    /// text row (colors keep their `#rrggbb` text shape — the same
    /// form the file carries). Each edit is one `filter.setParam`
    /// through the journal, so undo undoes it exactly.
    pub(super) fn filter_param_rows(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        index: usize,
        effect: &doc::Effect,
    ) {
        for property in &effect.properties {
            let key = property.key.as_str();
            if key == "EffectMoaID" {
                continue;
            }
            if key.ends_with("_points") {
                // Curve knots live as a raw `GPLv` node (or legacy
                // text); the rows show the count and a reset to the
                // identity line — full curve editing stays in the
                // engine's `points` JSON until a curve canvas exists.
                let knots = match &property.value {
                    doc::EffectValue::Raw(node) => node.as_children().map_or(0, <[doc::Node]>::len),
                    doc::EffectValue::Text(text) => text.split(';').count(),
                };
                ui.horizontal(|ui| {
                    ui.label(format!("{key}: {knots} knots"));
                    if ui.small_button("reset curve").clicked() {
                        let _ = self.editor.apply(
                            "filter.setParam",
                            serde_json::json!({
                                "id": id, "index": index, "key": key,
                                "value": "",
                                "points": [[0.0, 0.0], [255.0, 255.0]],
                            }),
                        );
                    }
                });
                continue;
            }
            let doc::EffectValue::Text(text) = &property.value else {
                // Raw nodes render read-only — a misread is not a value.
                ui.label(format!("{key}: (raw node)"));
                continue;
            };
            if FILTER_BOOL_KEYS.contains(&key) {
                let mut on = !matches!(text.trim(), "0" | "false");
                if ui.checkbox(&mut on, key).changed() {
                    self.set_filter_param(id, index, key, if on { "1" } else { "0" });
                }
                continue;
            }
            let Some(meta) = FILTER_PARAMS.iter().find(|meta| meta.key == key) else {
                // Text rows (colors, blend codes, file-borne keys):
                // commit on enter/blur so a keystroke is not a journal
                // entry.
                let mut edit = text.clone();
                let field = egui::TextEdit::singleline(&mut edit).desired_width(140.0);
                let response = ui.add(field);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.set_filter_param(id, index, key, &edit);
                } else {
                    ui.label(key);
                }
                continue;
            };
            let Ok(mut value) = text.trim().parse::<f64>() else {
                ui.label(format!("{key}: {text}"));
                continue;
            };
            ui.horizontal(|ui| {
                let slider =
                    ui.add(egui::Slider::new(&mut value, meta.min..=meta.max).text(meta.label));
                if slider.changed() {
                    self.set_filter_param(id, index, key, &format!("{value}"));
                }
                if ui.small_button("reset").clicked() {
                    self.set_filter_param(id, index, key, &format!("{}", meta.default));
                }
            });
        }
    }

    /// The gradient section of the fill editor: toggles the first
    /// selected path between solid and a linear ramp, and edits the
    /// ramp's stops (position + color). The on-canvas start/end
    /// handles live in the canvas overlay (`fill_handles_frame`).
    pub(super) fn gradient_editor_section(&mut self, ui: &mut egui::Ui, ids: &[doc::ObjectId]) {
        ui.horizontal(|ui| {
            ui.label("Fill type:");
            let Some(first) = ids.first() else {
                return;
            };
            let is_gradient = match self.editor.document().registry.get(*first) {
                Some(ObjectData::Path(path)) => path
                    .paint
                    .as_ref()
                    .and_then(|paint| paint.fill.as_ref())
                    .is_some_and(|fill| !matches!(fill.shape, doc::FillShape::Solid)),
                _ => false,
            };
            let mut linear = is_gradient;
            if ui.checkbox(&mut linear, "linear gradient").changed() {
                if linear {
                    // Seed the ramp: the fill color to white.
                    let color_raw = self.first_fill_raw(*first).unwrap_or(0xFF00_0000);
                    let id = *first;
                    self.run_command(
                        "path.setFill",
                        serde_json::json!({
                            "id": id, "shape": "linear",
                            "stops": [
                                { "position": 0.0, "color_raw": color_raw },
                                { "position": 1.0, "color_raw": 0xFFFF_FFFF_i64 },
                            ],
                            "alpha": [],
                        }),
                    );
                    // Give the new ramp a default vector across the
                    // object's bounds.
                    if let Some(bounds) =
                        crate::editor::union_selection_bounds(self.editor.document(), ids)
                    {
                        self.run_command(
                            "path.setFillVector",
                            serde_json::json!({
                                "id": id,
                                "start": { "x": bounds.x0, "y": bounds.center().y },
                                "end": { "x": bounds.x1, "y": bounds.center().y },
                            }),
                        );
                    }
                    self.status = "linear gradient fill".to_owned();
                } else {
                    // Collapse back to solid at the ramp's first color.
                    let color_raw = self.first_fill_raw(*first).unwrap_or(0xFF00_0000);
                    let id = *first;
                    self.run_command(
                        "path.setFill",
                        serde_json::json!({ "id": id, "shape": "solid", "stops": [], "alpha": [] }),
                    );
                    self.run_command(
                        "path.setFillColor",
                        serde_json::json!({ "id": id, "color_raw": color_raw }),
                    );
                    self.status = "solid fill".to_owned();
                }
            }
        });

        // Stop editing: one row per RGB stop of the first gradient.
        let Some(first) = ids.first().copied() else {
            return;
        };
        let Some(stops) = self.gradient_stops(first) else {
            return;
        };
        for (index, stop) in stops.iter().enumerate() {
            ui.horizontal(|ui| {
                let raw = stop.color_raw as u32;
                let mut color = Color32::from_rgba_unmultiplied(
                    (raw & 0xFF) as u8,
                    ((raw >> 8) & 0xFF) as u8,
                    ((raw >> 16) & 0xFF) as u8,
                    ((raw >> 24) & 0xFF) as u8,
                );
                if ui.color_edit_button_srgba(&mut color).changed() {
                    let new_raw = abgr_pack(color.r(), color.g(), color.b(), color.a());
                    self.set_gradient_stop(first, index, stop.position, new_raw);
                }
                let mut position = stop.position;
                let slider = ui.add(egui::Slider::new(&mut position, 0.0..=1.0));
                if slider.changed() {
                    self.set_gradient_stop(first, index, position, stop.color_raw);
                }
            });
        }
    }

    /// The fill color (`FCLi`) of a path, if it carries a paint.
    fn first_fill_raw(&self, id: doc::ObjectId) -> Option<i64> {
        match self.editor.document().registry.get(id) {
            Some(ObjectData::Path(path)) => path.paint.as_ref().map(|paint| paint.fill_raw),
            _ => None,
        }
    }

    /// The RGB ramp of the first selected path's gradient fill, if any.
    fn gradient_stops(&self, id: doc::ObjectId) -> Option<Vec<doc::GradientStop>> {
        match self.editor.document().registry.get(id) {
            Some(ObjectData::Path(path)) => path
                .paint
                .as_ref()
                .and_then(|paint| paint.fill.as_ref())
                .and_then(|fill| fill.gradient.as_ref())
                .map(|gradient| gradient.rgb.clone()),
            _ => None,
        }
    }

    /// Rewrites the whole RGB ramp with one stop changed —
    /// `path.setFill` carries no per-stop mutation verb, so the panel
    /// round-trips the ramp (one undo step per edit, same as a drag).
    fn set_gradient_stop(
        &mut self,
        id: doc::ObjectId,
        index: usize,
        position: f64,
        color_raw: i64,
    ) {
        let Some(mut stops) = self.gradient_stops(id) else {
            return;
        };
        if let Some(stop) = stops.get_mut(index) {
            stop.position = position;
            stop.color_raw = color_raw;
        }
        let payload: Vec<_> = stops
            .iter()
            .map(|stop| serde_json::json!({ "position": stop.position, "color_raw": stop.color_raw }))
            .collect();
        self.run_command(
            "path.setFill",
            serde_json::json!({ "id": id, "shape": "linear", "stops": payload, "alpha": [] }),
        );
    }
}
