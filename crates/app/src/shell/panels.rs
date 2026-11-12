//! The right-hand panel stack: Properties, Pages, States, Layers,
//! History, Styles, Library, Fill & Stroke, plus the help panel and
//! the shared display/filter row builders.
//!
//! Pure egui drawing glue: exercised visually (and later by the
//! `egui_kittest` snapshot gate), not by line coverage.

use super::*;
use compose::abgr_pack;
use doc::{GroupObject, ImageObject, SliceObject, SymbolAnimation};

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
            ui.weak("Licensed under Apache-2.0.");
            ui.add_space(4.0);
            if ui.button("Close").clicked() {
                self.about_open = false;
            }
        });
    }

    /// Edit ▸ Preferences (21.4): the persisted user settings. The
    /// UI-scale drag writes `ui_scale` live (applied to the context
    /// each frame); the value rides the shell layout, so it survives
    /// restarts like the panel set does.
    pub(super) fn preferences_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.heading("General");
        ui.horizontal(|ui| {
            ui.label("UI scale:");
            let mut scale = self.ui_scale.unwrap_or(1.0);
            let changed = ui
                .add(
                    egui::DragValue::new(&mut scale)
                        .speed(0.01)
                        .range(0.5..=3.0)
                        .suffix("×"),
                )
                .changed();
            if changed {
                self.ui_scale = Some(scale);
            }
            if ui.button("Reset").clicked() {
                self.ui_scale = None;
            }
        });
        ui.weak("Scales the whole interface (HiDPI displays). Persists across sessions.");
        ui.add_space(6.0);
        if ui.button("Close").clicked() {
            self.preferences_open = false;
        }
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
                        // Every binding carries the section it is filed
                        // under; a new heading starts wherever that
                        // section changes.
                        let mut row = 0usize;
                        while row < menus::SHORTCUTS.len() {
                            let title = menus::SHORTCUTS[row].group;
                            let start = row;
                            while row < menus::SHORTCUTS.len()
                                && menus::SHORTCUTS[row].group == title
                            {
                                row += 1;
                            }
                            let section = &menus::SHORTCUTS[start..row];
                            ui.heading(title);
                            egui::Grid::new(title.to_string())
                                .num_columns(2)
                                .spacing([12.0, 2.0])
                                .show(ui, |ui| {
                                    for s in section {
                                        ui.strong(s.label());
                                        ui.label(menus::action_name(s.action));
                                        ui.end_row();
                                    }
                                });
                            ui.separator();
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
    /// the right dock (Tools, Color Mixer, ...) is reported, not
    /// silently accepted.
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

    /// The Tools panel's screen-mode control: cycle Standard Screen /
    /// Full Screen with Menus / Full Screen (fwref 1178–1185). The
    /// layout in `ui()` reads the mode each frame: Full Screen folds
    /// every chrome element, Full Screen with Menus keeps the menu bar
    /// and paints the gray backdrop, Standard restores everything.
    pub(crate) fn cycle_screen_mode(&mut self) {
        self.screen_mode = self.screen_mode.next();
        self.status = self.screen_mode.label().to_owned();
    }

    /// Pages section: one selectable row per page, with a rename and
    /// a delete button beside it.
    pub(super) fn pages_section(&mut self, ui: &mut egui::Ui) {
        let pages = self.editor.document().pages.len();
        for index in 0..pages {
            let name = self.editor.document().pages[index]
                .name
                .clone()
                .unwrap_or_else(|| format!("Page {}", index + 1));
            let mut rename = name.clone();
            ui.horizontal(|ui| {
                if self.rename_page == Some(index) {
                    let field = ui
                        .add(egui::TextEdit::singleline(&mut rename).desired_width(96.0))
                        .on_hover_text("rename (Enter applies)");
                    let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
                    if (field.lost_focus() || enter) && rename != name {
                        self.run_command(
                            "page.setName",
                            serde_json::json!({ "page": index, "name": rename }),
                        );
                    }
                    if field.lost_focus() || enter {
                        self.rename_page = None;
                    }
                } else if ui
                    .selectable_label(self.editor.page == index, &name)
                    .on_hover_text("Make this the current page")
                    .clicked()
                {
                    self.editor.page = index;
                    self.raster = None;
                }
                if ui
                    .small_button("✎")
                    .on_hover_text("Rename this page")
                    .clicked()
                {
                    self.rename_page = Some(index);
                }
                if ui
                    .small_button("×")
                    .on_hover_text("Delete this page")
                    .clicked()
                {
                    self.delete_page(index);
                }
            });
        }
        self.guides_section(ui);
    }

    /// Delete a page through the engine's `page.delete` — the last
    /// page cannot go (the engine refuses it), and the current page
    /// index follows the shorter list.
    pub(super) fn delete_page(&mut self, index: usize) {
        if !self.run_command("page.delete", serde_json::json!({ "page": index })) {
            return;
        }
        let count = self.editor.document().pages.len();
        if self.editor.page >= count {
            self.editor.page = count.saturating_sub(1);
        }
        self.raster = None;
    }

    /// The page's guides: each one can be moved or removed.
    ///
    /// The guide list lives in the document (`page.guides`), so the
    /// rows read it there and write through the engine's
    /// `page.changeGuide` and `page.removeGuide`.
    pub(super) fn guides_section(&mut self, ui: &mut egui::Ui) {
        let page = self.editor.page;
        let Some(guides) = self
            .editor
            .document()
            .pages
            .get(page)
            .map(|page| page.guides.clone())
        else {
            return;
        };
        ui.label("Guides");
        for (kind, positions) in [
            ("horizontal", guides.horizontal.clone()),
            ("vertical", guides.vertical.clone()),
        ] {
            for position in positions {
                let mut value = position;
                ui.horizontal(|ui| {
                    let field = ui.add(
                        egui::DragValue::new(&mut value)
                            .speed(1.0)
                            .prefix(format!("{kind} ")),
                    );
                    if field.changed() {
                        self.change_guide(page, kind, position, value);
                    }
                    if ui
                        .small_button("×")
                        .on_hover_text("Remove this guide")
                        .clicked()
                    {
                        self.remove_guide(page, kind, position);
                    }
                });
            }
        }
    }

    /// Move one guide to a new position.
    pub(super) fn change_guide(&mut self, page: usize, kind: &str, current: f64, new: f64) {
        self.run_command(
            "page.changeGuide",
            serde_json::json!({
                "page": page, "current": current, "new": new, "kind": kind,
            }),
        );
    }

    /// Remove one guide.
    pub(super) fn remove_guide(&mut self, page: usize, kind: &str, position: f64) {
        self.run_command(
            "page.removeGuide",
            serde_json::json!({
                "page": page, "position": position, "kind": kind,
            }),
        );
    }

    /// The selection moves onto another layer of the current page, in
    /// the current state.
    ///
    /// The engine's `object.moveToCell` moves one object at a time,
    /// so the panel moves the selection one by one and skips the
    /// objects that already live there.
    pub(super) fn move_selection_to_layer(&mut self, layer: usize) {
        let ids = self.editor.live_selection();
        if ids.is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        let page = self.editor.page;
        let cell = self.editor.frame;
        let already: Vec<doc::ObjectId> = self
            .editor
            .document()
            .pages
            .get(page)
            .and_then(|page| page.layers.layers.get(layer))
            .and_then(|layer| layer.cells.get(cell))
            .map(|cell| cell.elements.clone())
            .unwrap_or_default();
        let mut moved = 0usize;
        for id in ids {
            if already.contains(&id) {
                continue;
            }
            if self.run_command(
                "object.moveToCell",
                serde_json::json!({
                    "id": id, "page": page, "layer": layer, "cell": cell, "index": null,
                }),
            ) {
                moved += 1;
            }
        }
        self.status = format!("moved {moved} object(s) to layer {layer}");
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
                    .small_button("all")
                    .on_hover_text("Select this layer's objects")
                    .clicked()
                {
                    self.select_layer_objects(layer_index);
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
                if ui
                    .small_button("⇔")
                    .on_hover_text("Move the selection onto this layer")
                    .clicked()
                {
                    self.move_selection_to_layer(layer_index);
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

    /// History section: the undo steps, newest first (CS6's History
    /// panel), each row clickable — clicking a row rewinds the
    /// document to the state that step recorded. Plus the steps
    /// save/replay pair.
    pub(super) fn history_section(&mut self, ui: &mut egui::Ui) {
        // Snapshot the rows so closures never borrow the session while
        // the editor is mutable (same rule as the layer list above).
        let mut entries = self.editor.session.history_entries();
        entries.reverse();
        egui::ScrollArea::vertical()
            .max_height(160.0)
            .show(ui, |ui| {
                for entry in &entries {
                    let label = format!("{}. {}", entry.depth, entry.label);
                    let response = ui.selectable_label(false, label);
                    if response.clicked() {
                        self.rewind_history(entry.depth);
                    }
                    response.on_hover_text(format!("Rewind to step {}", entry.depth));
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

    /// Rewinds the session's history to `depth`: the document returns
    /// to the state that step recorded and the steps above it are
    /// dropped.
    pub(super) fn rewind_history(&mut self, depth: usize) {
        match self.editor.session.rewind_to(depth) {
            Ok(0) => self.status = "already at that step".to_owned(),
            Ok(undone) => self.status = format!("rewound {undone} step(s)"),
            Err(error) => self.status = format!("history.rewind failed: {error}"),
        }
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
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(false, label)
                    .on_hover_text("apply onto the selection")
                    .clicked()
                {
                    self.run_command(
                        "style.apply",
                        serde_json::json!({ "index": index, "ids": targets.clone() }),
                    );
                }
                if ui
                    .small_button("redefine")
                    .on_hover_text("redefine this style from the selection's appearance")
                    .clicked()
                    && let Some(source) = targets.last().copied()
                {
                    self.redefine_style(*index, source);
                }
            });
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
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("text scale");
            ui.add(
                egui::DragValue::new(&mut self.style_scale)
                    .speed(0.05)
                    .range(0.1..=10.0),
            );
            if ui
                .button("Scale Styles")
                .on_hover_text("multiplies every style's text size by the factor")
                .clicked()
            {
                let scale = self.style_scale;
                self.scale_styles(scale);
            }
        });
    }

    /// The Styles panel's per-row redefine: the named object's
    /// appearance becomes the style at `index` — CS6's
    /// `redefineStyleByIndex`.
    pub(super) fn redefine_style(&mut self, index: usize, source: doc::ObjectId) {
        self.run_command(
            "style.redefineByIndex",
            serde_json::json!({ "index": index, "source": source }),
        );
    }

    /// The Styles panel's scale: every style's text size by the
    /// factor — CS6's `scaleStyles`.
    pub(super) fn scale_styles(&mut self, scale: f64) {
        self.run_command("style.scaleStyles", serde_json::json!({ "scale": scale }));
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
        self.draft_panel(ui);
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
                | Tool::Blur
                | Tool::Sharpen
                | Tool::Smudge
                | Tool::Dodge
                | Tool::Burn
                | Tool::RubberStamp
                | Tool::RedEyeRemoval
                | Tool::ReplaceColor
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
        let ids = self.editor.live_selection();
        if ids.len() >= 2 {
            self.multi_selection_properties(ui, &ids);
            return;
        }
        let Some(id) = self.editor.selection else {
            self.document_properties(ui);
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
            Some(ObjectData::Group(group)) => {
                self.group_properties(ui, id, &group);
                group.effects
            }
            Some(ObjectData::Image(image)) => {
                self.image_properties(ui, id, &image);
                image.effects
            }
            Some(ObjectData::Slice(slice)) => {
                // A slice carries no `EFLv` spine, so the effect list
                // below has nothing to read.
                self.slice_properties(ui, id, &slice);
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

    /// The in-progress shape's panel. CS6's Pen, Polygon Slice,
    /// Polygon Hotspot and Polygonal Marquee all build their shape
    /// with clicks, and the shape is only real once it is finished —
    /// so the panel carries the finish button and the point count.
    pub(super) fn draft_panel(&mut self, ui: &mut egui::Ui) {
        let polygon_points = self
            .editor
            .polygon
            .as_ref()
            .map_or(0, |draft| draft.points.len());
        let path_points = self
            .editor
            .draft
            .as_ref()
            .map_or(0, |draft| draft.points.len());
        if polygon_points == 0 && path_points == 0 {
            return;
        }
        ui.heading("Shape in progress");
        if polygon_points > 0 {
            ui.label(format!("marquee: {polygon_points} points"));
        }
        if path_points > 0 {
            ui.label(format!("path: {path_points} points"));
        }
        ui.horizontal(|ui| {
            if ui.button("finish shape").clicked() {
                if polygon_points > 0 {
                    match self.editor.commit_polygon() {
                        Ok(true) => self.status = "polygon selected".to_owned(),
                        Ok(false) => self.status = "polygon needs three points".to_owned(),
                        Err(error) => self.status = format!("polygon failed: {error}"),
                    }
                } else {
                    match self.editor.commit_draft() {
                        Ok(true) => self.status = "shape created".to_owned(),
                        Ok(false) => self.status = "shape needs more points".to_owned(),
                        Err(error) => self.status = format!("shape failed: {error}"),
                    }
                }
            }
            if ui.button("cancel").clicked() {
                self.editor.draft = None;
                self.editor.polygon = None;
                self.status = "shape discarded".to_owned();
            }
        });
        ui.separator();
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
                definition.auto_slice,
                definition.include_down_state,
                definition.show_down_on_load,
                definition.include_over_while_down,
                definition
                    .layers
                    .layers
                    .iter()
                    .map(|layer| {
                        layer
                            .cells
                            .iter()
                            .map(|cell| cell.elements.clone())
                            .collect::<Vec<Vec<doc::ObjectId>>>()
                    })
                    .collect::<Vec<Vec<Vec<doc::ObjectId>>>>(),
            )
        });
        let Some((
            name,
            kind,
            kind_raw,
            state_parts,
            sliced,
            auto_slice,
            include_down_state,
            show_down_on_load,
            include_over_while_down,
            cells,
        )) = facts
        else {
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

        // The button's behavior: the four state flags and the export
        // options that ride the button's auto-slice.
        self.symbol_button_fields(
            ui,
            number,
            kind_raw,
            auto_slice,
            include_down_state,
            show_down_on_load,
            include_over_while_down,
        );

        // The animation span — CS6's Animation panel, for an
        // instance that already carries one or names an animated
        // definition.
        if matches!(kind_raw, SymbolType::Animation) || instance.animation.is_some() {
            self.symbol_animation_fields(ui, instance);
        }

        // The instance's own parameters and custom data.
        self.symbol_data_fields(ui, id, instance);

        // The definition's content: detach an object from the
        // symbol, or move it within the symbol's z-order.
        self.symbol_content_fields(ui, number, &cells);

        // "Duplicate for alias": the selected instances point at
        // fresh copies of the definitions they name.
        if ui
            .button("Duplicate for alias")
            .on_hover_text(
                "CS6's duplicateSymbolForAlias: the selected instances point at copies of \
                 the definitions they name",
            )
            .clicked()
        {
            self.duplicate_symbol_for_alias();
        }
    }

    /// The button's behavior: the four state flags (`ATSb`, `IDSb`,
    /// `DVLb`, `IODb`) and the export options that ride the button's
    /// linked slices (`A2Ts`, `ALTs`, `OBNs`).
    ///
    /// The flags live on the *definition*, so the write names the
    /// symbol number; the options live on the definition's linked
    /// slices, so the write names the symbol number too and the
    /// engine applies the payload to every linked slice.
    pub(super) fn symbol_button_fields(
        &mut self,
        ui: &mut egui::Ui,
        number: i64,
        kind: SymbolType,
        auto_slice: Option<bool>,
        include_down_state: Option<bool>,
        show_down_on_load: Option<bool>,
        include_over_while_down: Option<bool>,
    ) {
        let is_button = matches!(kind, SymbolType::Button);
        if !is_button
            && auto_slice.is_none()
            && include_down_state.is_none()
            && show_down_on_load.is_none()
            && include_over_while_down.is_none()
        {
            return;
        }
        ui.heading("Button");
        let mut flag = auto_slice.unwrap_or(false);
        if ui.checkbox(&mut flag, "auto-slice").changed() {
            self.set_button_auto_slice(number, flag);
        }
        let mut flag = include_down_state.unwrap_or(false);
        if ui.checkbox(&mut flag, "include down state").changed() {
            self.set_button_include_down_state(number, flag);
        }
        let mut flag = show_down_on_load.unwrap_or(false);
        if ui.checkbox(&mut flag, "show down on load").changed() {
            self.set_button_show_down_on_load(number, flag);
        }
        let mut flag = include_over_while_down.unwrap_or(false);
        if ui.checkbox(&mut flag, "include over while down").changed() {
            self.set_button_include_over_while_down(number, flag);
        }
        // The export options: the first linked slice is the read
        // side; the write reaches every slice linked to the symbol.
        let linked = self.linked_slice_facts(number);
        let mut link = linked.link;
        let mut alt = linked.alt;
        let mut slice_name = linked.slice_name;
        ui.horizontal(|ui| {
            let link_field = ui.add(egui::TextEdit::singleline(&mut link).hint_text("link URL"));
            let alt_field = ui.add(egui::TextEdit::singleline(&mut alt).hint_text("alt text"));
            let name_field =
                ui.add(egui::TextEdit::singleline(&mut slice_name).hint_text("slice name"));
            if link_field.lost_focus() && link_field.changed() {
                self.set_button_options(number, Some(link.clone()), None, None);
            }
            if alt_field.lost_focus() && alt_field.changed() {
                self.set_button_options(number, None, Some(alt.clone()), None);
            }
            if name_field.lost_focus() && name_field.changed() {
                self.set_button_options(number, None, None, Some(slice_name.clone()));
            }
        });
        if !linked.found {
            ui.label("no slice is linked to this symbol yet");
        }
        // The button's text: CS6's replacement verbs — one across the
        // library's button definitions, one across the definitions the
        // selected instances reference.
        let mut button_text = String::new();
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut button_text).hint_text("button text"));
            if ui.button("in library").clicked() {
                self.replace_button_text(&button_text);
            }
            if ui.button("in instances").clicked() {
                self.replace_button_text_in_instances(&button_text);
            }
        });
    }

    /// Replace the button text of every button definition in the
    /// library.
    pub(super) fn replace_button_text(&mut self, text: &str) {
        self.run_command(
            "text.replaceButtonTextStrings",
            serde_json::json!({ "text": text, "uniform_attrs": false }),
        );
    }

    /// Replace the button text of the definitions the selected
    /// instances reference. The engine reads the document's selection,
    /// so the view's selection is synced first.
    pub(super) fn replace_button_text_in_instances(&mut self, text: &str) {
        if self.editor.live_selection().is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command(
            "text.replaceButtonTextStringsInInstances",
            serde_json::json!({ "text": text, "uniform_attrs": false }),
        );
    }

    /// The first slice linked to `number`, read for the button's
    /// export options.
    fn linked_slice_facts(&self, number: i64) -> LinkedSliceFacts {
        let mut facts = LinkedSliceFacts::default();
        for (_, data) in self.editor.document().registry.iter() {
            let ObjectData::Slice(slice) = data else {
                continue;
            };
            if slice.symbol_number != Some(number) {
                continue;
            }
            facts.found = true;
            if facts.link.is_empty() {
                facts.link = slice.link.clone().unwrap_or_default();
            }
            if facts.alt.is_empty() {
                facts.alt = slice.alt.clone().unwrap_or_default();
            }
            if facts.slice_name.is_empty() {
                facts.slice_name = slice.name.clone().unwrap_or_default();
            }
        }
        facts
    }

    /// Write the button's auto-slice flag (`ATSb`).
    pub(super) fn set_button_auto_slice(&mut self, number: i64, value: bool) {
        self.run_command(
            "symbol.setButtonAutoSlice",
            serde_json::json!({ "symbol": number, "value": value }),
        );
    }

    /// Write the button's include-down-state flag (`IDSb`).
    pub(super) fn set_button_include_down_state(&mut self, number: i64, value: bool) {
        self.run_command(
            "symbol.setButtonIncludeDownState",
            serde_json::json!({ "symbol": number, "value": value }),
        );
    }

    /// Write the button's show-down-on-load flag (`DVLb`).
    pub(super) fn set_button_show_down_on_load(&mut self, number: i64, value: bool) {
        self.run_command(
            "symbol.setButtonShowDownOnLoad",
            serde_json::json!({ "symbol": number, "value": value }),
        );
    }

    /// Write the button's include-over-while-down flag (`IODb`).
    pub(super) fn set_button_include_over_while_down(&mut self, number: i64, value: bool) {
        self.run_command(
            "symbol.setButtonIncludeOverWhileDownState",
            serde_json::json!({ "symbol": number, "value": value }),
        );
    }

    /// Write one of the button's export options. The engine applies
    /// every present field to every slice linked to the symbol and
    /// leaves the absent ones untouched, so the panel sends one field
    /// at a time.
    pub(super) fn set_button_options(
        &mut self,
        number: i64,
        link: Option<String>,
        alt: Option<String>,
        slice_name: Option<String>,
    ) {
        self.run_command(
            "symbol.setButtonOptions",
            serde_json::json!({
                "symbol": number,
                "link": link,
                "alt": alt,
                "slice_name": slice_name,
            }),
        );
    }

    /// The animation span: the `ANNi` family's six writes. CS6's
    /// `setAnimInstance*` verbs address the *selected instances*, so
    /// the panel syncs the view's selection into the document first
    /// (decision 0104's bridge).
    pub(super) fn symbol_animation_fields(
        &mut self,
        ui: &mut egui::Ui,
        instance: &SymbolInstanceObject,
    ) {
        let animation = instance.animation.clone().unwrap_or(SymbolAnimation {
            frames: 1,
            x_shift: 0.0,
            y_shift: 0.0,
            scaling: 100.0,
            rotation: None,
            start_opacity: None,
            end_opacity: None,
        });
        let mut frames = animation.frames;
        let mut loop_count = animation.frames;
        let mut shift_x = animation.x_shift;
        let mut shift_y = animation.y_shift;
        let mut rotation = animation.rotation.unwrap_or(0) as f64;
        let mut scale = animation.scaling;
        // The model stores opacity per mille; the wire speaks 0–100.
        let mut start = animation.start_opacity.unwrap_or(1000) as f64 / 10.0;
        let mut end = animation.end_opacity.unwrap_or(1000) as f64 / 10.0;
        ui.heading("Animation");
        ui.horizontal(|ui| {
            let frame_field = ui.add(
                egui::DragValue::new(&mut frames)
                    .speed(1.0)
                    .range(1..=1000)
                    .prefix("frames "),
            );
            let loop_field = ui.add(
                egui::DragValue::new(&mut loop_count)
                    .speed(1.0)
                    .range(1..=1000)
                    .prefix("loop count "),
            );
            if frame_field.changed() {
                self.set_anim_num_frames(frames);
            }
            if loop_field.changed() {
                self.set_anim_loop_count(loop_count);
            }
        });
        ui.horizontal(|ui| {
            let x_field = ui.add(
                egui::DragValue::new(&mut shift_x)
                    .speed(1.0)
                    .prefix("offset x "),
            );
            let y_field = ui.add(
                egui::DragValue::new(&mut shift_y)
                    .speed(1.0)
                    .prefix("offset y "),
            );
            if x_field.changed() || y_field.changed() {
                self.set_anim_offset(shift_x, shift_y);
            }
        });
        ui.horizontal(|ui| {
            let rotation_field = ui.add(
                egui::DragValue::new(&mut rotation)
                    .speed(1.0)
                    .prefix("rotation "),
            );
            let scale_field = ui.add(
                egui::DragValue::new(&mut scale)
                    .speed(1.0)
                    .range(0.01..=1000.0)
                    .prefix("scale % "),
            );
            if rotation_field.changed() {
                self.set_anim_rotation(rotation);
            }
            if scale_field.changed() {
                self.set_anim_scale(scale);
            }
        });
        ui.horizontal(|ui| {
            let start_field = ui.add(
                egui::DragValue::new(&mut start)
                    .speed(1.0)
                    .range(0.0..=100.0)
                    .prefix("start opacity "),
            );
            let end_field = ui.add(
                egui::DragValue::new(&mut end)
                    .speed(1.0)
                    .range(0.0..=100.0)
                    .prefix("end opacity "),
            );
            if start_field.changed() || end_field.changed() {
                self.set_anim_opacity(start, end);
            }
        });
    }

    /// The animation and effect verbs address the document's
    /// selection, so the view's selection is synced into the
    /// document first.
    fn sync_document_selection(&mut self) -> bool {
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return false;
        }
        true
    }

    /// Write the animation's frame count (`ANNi`).
    pub(super) fn set_anim_num_frames(&mut self, frames: i64) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "symbol.setAnimInstanceNumFrames",
            serde_json::json!({ "frames": frames }),
        );
    }

    /// Write the animation's loop count (CS6's deprecated spelling of
    /// the frame count).
    pub(super) fn set_anim_loop_count(&mut self, loop_count: i64) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "symbol.setAnimInstanceLoopCount",
            serde_json::json!({ "loop_count": loop_count }),
        );
    }

    /// Write the animation's travel across the span (`ANXf`/`ANYf`).
    pub(super) fn set_anim_offset(&mut self, x: f64, y: f64) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "symbol.setAnimInstanceOffsetDist",
            serde_json::json!({ "offset": { "x": x, "y": y } }),
        );
    }

    /// Write the animation's rotation across the span (`ANRi`).
    pub(super) fn set_anim_rotation(&mut self, rotation: f64) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "symbol.setAnimInstanceRotationAmount",
            serde_json::json!({ "rotation": rotation }),
        );
    }

    /// Write the animation's scaling across the span (`ASCf`).
    pub(super) fn set_anim_scale(&mut self, scale: f64) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "symbol.setAnimInstanceScaleAmount",
            serde_json::json!({ "scale": scale }),
        );
    }

    /// Write the animation's opacity ramp (`ASOi`/`AEOi`).
    pub(super) fn set_anim_opacity(&mut self, start: f64, end: f64) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "symbol.setAnimInstanceStartEndOpacity",
            serde_json::json!({ "start": start, "end": end }),
        );
    }

    /// The instance's parameters (`currentValues`) and its custom data
    /// (`DTAv`).
    pub(super) fn symbol_data_fields(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        instance: &SymbolInstanceObject,
    ) {
        ui.heading("Parameters");
        let mut name = String::new();
        let mut kind = "text".to_owned();
        let mut value = String::new();
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut name)
                    .desired_width(96.0)
                    .hint_text("name"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut kind)
                    .desired_width(64.0)
                    .hint_text("type"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut value)
                    .desired_width(96.0)
                    .hint_text("value"),
            );
        });
        if ui.button("Set parameter").clicked() {
            self.set_symbol_param(id, &name, &kind, &value);
        }
        ui.heading("Custom data");
        let mut key = String::new();
        let mut text = String::new();
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut key)
                    .desired_width(96.0)
                    .hint_text("key"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut text)
                    .desired_width(96.0)
                    .hint_text("value"),
            );
        });
        if ui.button("Set custom data").clicked() {
            self.set_symbol_custom_data(id, &key, &text);
        }
        for (existing, stored) in &instance.custom_data {
            ui.horizontal(|ui| {
                ui.label(format!("{existing}: {stored}"));
                if ui.small_button("×").clicked() {
                    self.remove_symbol_custom_data(id, existing);
                }
            });
        }
    }

    /// Write one parameter of the instance (`currentValues`).
    pub(super) fn set_symbol_param(
        &mut self,
        id: doc::ObjectId,
        name: &str,
        kind: &str,
        value: &str,
    ) {
        self.run_command(
            "symbol.setParam",
            serde_json::json!({
                "id": id, "name": name, "kind": kind, "value": value,
            }),
        );
    }

    /// Write one custom-data entry of the instance (`DTAv`).
    pub(super) fn set_symbol_custom_data(&mut self, id: doc::ObjectId, key: &str, value: &str) {
        self.run_command(
            "symbol.setCustomData",
            serde_json::json!({ "id": id, "key": key, "value": value }),
        );
    }

    /// Remove one custom-data entry of the instance.
    pub(super) fn remove_symbol_custom_data(&mut self, id: doc::ObjectId, key: &str) {
        self.run_command(
            "symbol.setCustomData",
            serde_json::json!({ "id": id, "key": key, "value": null }),
        );
    }

    /// The definition's own content: every object in the symbol's
    /// cells, with the two edits CS6 offers on them — lift the object
    /// out of the symbol, or move it within the symbol's z-order.
    pub(super) fn symbol_content_fields(
        &mut self,
        ui: &mut egui::Ui,
        number: i64,
        cells: &[Vec<Vec<doc::ObjectId>>],
    ) {
        if cells.is_empty() {
            return;
        }
        ui.heading("Definition content");
        for (layer_index, layer) in cells.iter().enumerate() {
            for (cell_index, cell) in layer.iter().enumerate() {
                if cell.is_empty() {
                    continue;
                }
                ui.label(format!("layer {layer_index}, state {cell_index}"));
                for (position, element) in cell.iter().enumerate() {
                    let top = cell.len().saturating_sub(1);
                    ui.horizontal(|ui| {
                        ui.label(format!("object {}", element.index()));
                        if position < top
                            && ui
                                .small_button("▲")
                                .on_hover_text("Move the object up in the symbol")
                                .clicked()
                        {
                            self.reorder_symbol_object(number, *element, position + 1);
                        }
                        if position > 0
                            && ui
                                .small_button("▼")
                                .on_hover_text("Move the object down in the symbol")
                                .clicked()
                        {
                            self.reorder_symbol_object(number, *element, position - 1);
                        }
                        if ui
                            .small_button("×")
                            .on_hover_text("Lift the object out of the symbol")
                            .clicked()
                        {
                            self.detach_symbol_object(number, *element);
                        }
                    });
                }
            }
        }
    }

    /// Move an object within its definition's cell. The engine's
    /// target index is the position *after* the object leaves the
    /// list, so a move up is `position + 1` and a move down is
    /// `position - 1`.
    pub(super) fn reorder_symbol_object(&mut self, number: i64, id: doc::ObjectId, to: usize) {
        self.run_command(
            "symbol.editReorder",
            serde_json::json!({ "symbol": number, "id": id, "to": to }),
        );
    }

    /// Lift an object out of its definition's cell.
    pub(super) fn detach_symbol_object(&mut self, number: i64, id: doc::ObjectId) {
        self.run_command(
            "symbol.editDetach",
            serde_json::json!({ "symbol": number, "id": id }),
        );
    }

    /// The selected instances point at fresh copies of the definitions
    /// they name. The engine reads the document's selection, so the
    /// view's selection is synced first.
    pub(super) fn duplicate_symbol_for_alias(&mut self) {
        if self.editor.live_selection().is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command("symbol.duplicateSymbolForAlias", serde_json::json!({}));
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
        self.text_layout_fields(ui, id, text);
        self.text_render_fields(ui, id, text);
        self.text_markup_fields(ui, id, size, &font);
        self.geometry_fields(ui, id);
    }

    /// The markup verbs: CS6's character and font markup, which
    /// restyle the *runs* rather than the base style.
    ///
    /// The panel's bold/italic/underline checkboxes write the base
    /// style (`text.setAttrs`); these are the sibling verbs that
    /// reach the runs, which is what CS6's Format ▸ Text menu does
    /// to a mixed-format text item.
    pub(super) fn text_markup_fields(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        size: f64,
        face: &str,
    ) {
        ui.heading("Markup");
        ui.horizontal(|ui| {
            if ui.button("bold").clicked() {
                self.apply_character_markup(id, "b");
            }
            if ui.button("italic").clicked() {
                self.apply_character_markup(id, "i");
            }
            if ui.button("underline").clicked() {
                self.apply_character_markup(id, "u");
            }
            if ui
                .button("plain")
                .on_hover_text("CS6's fwplain: no character markup, base included")
                .clicked()
            {
                self.apply_character_markup(id, "fwplain");
            }
        });
        ui.horizontal(|ui| {
            if ui.button("clear bold").clicked() {
                self.remove_character_markup(id, "b");
            }
            if ui.button("clear italic").clicked() {
                self.remove_character_markup(id, "i");
            }
            if ui.button("clear underline").clicked() {
                self.remove_character_markup(id, "u");
            }
        });
        ui.horizontal(|ui| {
            if ui.button("apply size").clicked() {
                self.apply_font_markup(id, "size", &format!("{size}pt"));
            }
            if ui.button("apply face").clicked() {
                self.apply_font_markup(id, "face", face);
            }
        });
        ui.horizontal(|ui| {
            if ui.button("clear size").clicked() {
                self.remove_font_markup(id, "size");
            }
            if ui.button("clear color").clicked() {
                self.remove_font_markup(id, "color");
            }
            if ui.button("clear face").clicked() {
                self.remove_font_markup(id, "face");
            }
        });
    }

    /// Apply one character markup tag to the text's runs.
    pub(super) fn apply_character_markup(&mut self, id: doc::ObjectId, tag: &str) {
        self.run_command(
            "text.applyCharacterMarkup",
            serde_json::json!({ "id": id, "tag": tag }),
        );
    }

    /// Reapply the default for one character markup tag.
    pub(super) fn remove_character_markup(&mut self, id: doc::ObjectId, tag: &str) {
        self.run_command(
            "text.removeCharacterMarkup",
            serde_json::json!({ "id": id, "tag": tag }),
        );
    }

    /// Apply one font markup attribute to the text's runs. The size
    /// takes the dom's `"NNNpt"` form; the face takes a family name.
    pub(super) fn apply_font_markup(&mut self, id: doc::ObjectId, attribute: &str, value: &str) {
        self.run_command(
            "text.applyFontMarkup",
            serde_json::json!({
                "id": id, "font_attribute": attribute, "value": value,
            }),
        );
    }

    /// Reapply the default for one font markup attribute.
    pub(super) fn remove_font_markup(&mut self, id: doc::ObjectId, attribute: &str) {
        self.run_command(
            "text.removeFontMarkup",
            serde_json::json!({ "id": id, "font_attribute": attribute }),
        );
    }

    /// The text object's layout: the paragraph's indent and spacing,
    /// the on-path offset, and the frame that fits the text.
    ///
    /// The paragraph values live on the text style (`PINf`, `PSBf`,
    /// `PSAf`), so the panel reads them from the style and writes
    /// them through the three paragraph setters.
    pub(super) fn text_layout_fields(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        text: &TextObject,
    ) {
        let mut indent = text
            .style
            .as_ref()
            .and_then(|style| style.paragraph_indent)
            .unwrap_or(0.0);
        let mut before = text
            .style
            .as_ref()
            .and_then(|style| style.space_before)
            .unwrap_or(0.0);
        let mut after = text
            .style
            .as_ref()
            .and_then(|style| style.space_after)
            .unwrap_or(0.0);
        ui.horizontal(|ui| {
            let indent_field = ui.add(
                egui::DragValue::new(&mut indent)
                    .speed(1.0)
                    .range(-1000.0..=1000.0)
                    .prefix("indent "),
            );
            let before_field = ui.add(
                egui::DragValue::new(&mut before)
                    .speed(1.0)
                    .range(-1000.0..=1000.0)
                    .prefix("before "),
            );
            let after_field = ui.add(
                egui::DragValue::new(&mut after)
                    .speed(1.0)
                    .range(-1000.0..=1000.0)
                    .prefix("after "),
            );
            if indent_field.changed() {
                self.set_paragraph_indent(id, indent);
            }
            if before_field.changed() {
                self.set_paragraph_spacing_before(id, before);
            }
            if after_field.changed() {
                self.set_paragraph_spacing_after(id, after);
            }
        });
        // The on-path offset: where the text starts along its guide
        // path (`CHOf`).
        if let Some(on_path) = text.on_path.as_ref() {
            let mut offset = on_path.offset;
            if ui
                .add(
                    egui::DragValue::new(&mut offset)
                        .speed(1.0)
                        .range(-10_000.0..=10_000.0)
                        .prefix("offset "),
                )
                .on_hover_text("where the text starts along its path")
                .changed()
            {
                self.set_on_path_offset(offset);
            }
        }
        if ui
            .button("Fit box to text")
            .on_hover_text("CS6's setTextRectangleAuto: the frame shrinks to the text it holds")
            .clicked()
        {
            self.fit_text_box();
        }
    }

    /// The paragraph's indent, in pixels (`PINf`).
    pub(super) fn set_paragraph_indent(&mut self, id: doc::ObjectId, indent: f64) {
        let _ = self.editor.apply(
            "text.setTextParaIndent",
            serde_json::json!({ "id": id, "para_indent": indent }),
        );
    }

    /// The space above the paragraph, in pixels (`PSBf`).
    pub(super) fn set_paragraph_spacing_before(&mut self, id: doc::ObjectId, before: f64) {
        let _ = self.editor.apply(
            "text.setTextParaSpacingBefore",
            serde_json::json!({ "id": id, "para_space_before": before }),
        );
    }

    /// The space below the paragraph, in pixels (`PSAf`).
    pub(super) fn set_paragraph_spacing_after(&mut self, id: doc::ObjectId, after: f64) {
        let _ = self.editor.apply(
            "text.setTextParaSpacingAfter",
            serde_json::json!({ "id": id, "para_space_after": after }),
        );
    }

    /// Where the text starts along its guide path (`CHOf`).
    pub(super) fn set_on_path_offset(&mut self, offset: f64) {
        // The setter walks the document's selection, so the view's
        // selection lands there first.
        if self.sync_document_selection() {
            let _ = self.editor.apply(
                "text.setTextOnPathOffset",
                serde_json::json!({ "offset": offset }),
            );
        }
    }

    /// CS6's setTextRectangleAuto: the frame shrinks to the text it
    /// holds.
    pub(super) fn fit_text_box(&mut self) {
        // The setter reads the document's selection, so the view's
        // selection lands there first.
        if self.sync_document_selection() {
            let _ = self
                .editor
                .apply("text.setTextRectangleAuto", serde_json::json!({}));
        }
    }

    /// The text commands that read the document's selection need the
    /// view's selection to land there first — the same bridge the
    /// Select menu's verbs use (decision 0104).
    pub(super) fn sync_document_selection(&mut self) -> bool {
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return false;
        }
        true
    }

    /// The text object's rendering: the custom anti-alias dials, the
    /// run list, and the frame the text is laid out in.
    ///
    /// The three dials are MX 2004's custom anti-alias mode (`TA1i`
    /// sharpness, `TA2i` strength, `TOSi` oversampling); the runs
    /// are the structured string's segments, one line per run; the
    /// frame is the box the text flows inside — a different box from
    /// the object's own bounds.
    pub(super) fn text_render_fields(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        text: &TextObject,
    ) {
        // The custom anti-alias dials.
        let mut sharpness = text.anti_alias_sharpness.unwrap_or(0);
        let mut strength = text.anti_alias_strength.unwrap_or(0);
        ui.horizontal(|ui| {
            let sharp = ui.add(
                egui::DragValue::new(&mut sharpness)
                    .speed(1.0)
                    .range(0..=255)
                    .prefix("sharpness "),
            );
            let strong = ui.add(
                egui::DragValue::new(&mut strength)
                    .speed(1.0)
                    .range(0..=255)
                    .prefix("strength "),
            );
            if sharp.changed() {
                self.set_custom_anti_alias_sharpness(id, sharpness);
            }
            if strong.changed() {
                self.set_custom_anti_alias_strength(id, strength);
            }
        });
        let mut over_sample = text.anti_alias_oversample.unwrap_or(4);
        egui::ComboBox::from_label("oversample")
            .selected_text(over_sample.to_string())
            .show_ui(ui, |ui| {
                for value in [4i64, 8, 16] {
                    if ui
                        .selectable_label(over_sample == value, value.to_string())
                        .clicked()
                    {
                        self.set_custom_anti_alias_over_sample(id, value);
                    }
                }
            });
        // The runs: one line per run. The base style rides along
        // unchanged — the command writes the run list alone.
        let mut runs = text
            .runs
            .iter()
            .map(|run| run.text.clone())
            .collect::<Vec<String>>()
            .join("\n");
        let field = ui.add(
            egui::TextEdit::multiline(&mut runs)
                .hint_text("runs, one per line (click away to apply)"),
        );
        if field.changed() && field.lost_focus() {
            self.set_text_runs(id, &runs);
        }
        // The frame the text flows inside.
        let mut width = text.frame.width() as f32;
        let mut height = text.frame.height() as f32;
        ui.horizontal(|ui| {
            let wide = ui.add(
                egui::DragValue::new(&mut width)
                    .speed(1.0)
                    .prefix("text W "),
            );
            let tall = ui.add(
                egui::DragValue::new(&mut height)
                    .speed(1.0)
                    .prefix("text H "),
            );
            if wide.changed() || tall.changed() {
                self.set_text_rectangle(id, width as f64, height as f64);
            }
        });
        if ui
            .button("Fit box at the corner")
            .on_hover_text(
                "CS6's setTextRectangleAutoFromPoint: the fitted box anchors at the frame's top-left corner",
            )
            .clicked()
        {
            self.fit_text_box_at(id);
        }
    }

    /// The custom anti-alias sharpness (`TA1i`), 0..=255.
    pub(super) fn set_custom_anti_alias_sharpness(&mut self, id: doc::ObjectId, sharpness: i64) {
        self.run_command(
            "text.setTextCustomAntiAliasSharpness",
            serde_json::json!({ "id": id, "sharpness": sharpness }),
        );
    }

    /// The custom anti-alias strength (`TA2i`), 0..=255.
    pub(super) fn set_custom_anti_alias_strength(&mut self, id: doc::ObjectId, strength: i64) {
        self.run_command(
            "text.setTextCustomAntiAliasStrength",
            serde_json::json!({ "id": id, "strength": strength }),
        );
    }

    /// The custom anti-alias oversampling (`TOSi`): 4, 8 or 16.
    pub(super) fn set_custom_anti_alias_over_sample(
        &mut self,
        id: doc::ObjectId,
        over_sample: i64,
    ) {
        self.run_command(
            "text.setTextCustomAntiAliasOverSample",
            serde_json::json!({ "id": id, "over_sample": over_sample }),
        );
    }

    /// The structured string's runs: one line per run, written in
    /// order. The base style stays as it is.
    pub(super) fn set_text_runs(&mut self, id: doc::ObjectId, text: &str) {
        let runs: Vec<serde_json::Value> = if text.is_empty() {
            Vec::new()
        } else {
            text.split('\n')
                .map(|characters| serde_json::json!({ "characters": characters }))
                .collect()
        };
        self.run_command(
            "text.setTextRuns",
            serde_json::json!({ "id": id, "runs": runs }),
        );
    }

    /// The frame the text flows inside. The command reads the
    /// document's selection, so the view's selection lands there
    /// first.
    pub(super) fn set_text_rectangle(&mut self, id: doc::ObjectId, width: f64, height: f64) {
        if !self.sync_document_selection() {
            return;
        }
        let Some(frame) = self.text_frame(id) else {
            return;
        };
        self.run_command(
            "text.setTextRectangle",
            serde_json::json!({
                "rect": {
                    "left": frame.x0, "top": frame.y0,
                    "right": frame.x0 + width, "bottom": frame.y0 + height,
                }
            }),
        );
    }

    /// CS6's setTextRectangleAutoFromPoint: the fitted box anchors at
    /// the frame's top-left corner.
    pub(super) fn fit_text_box_at(&mut self, id: doc::ObjectId) {
        if !self.sync_document_selection() {
            return;
        }
        let Some(frame) = self.text_frame(id) else {
            return;
        };
        self.run_command(
            "text.setTextRectangleAutoFromPoint",
            serde_json::json!({ "anchor": { "x": frame.x0, "y": frame.y0 } }),
        );
    }

    /// The frame of one text object.
    fn text_frame(&self, id: doc::ObjectId) -> Option<geom::Rect> {
        match self.editor.document().registry.get(id) {
            Some(ObjectData::Text(text)) => Some(text.frame),
            _ => None,
        }
    }

    /// Document properties (22.4): the field set the inspector shows
    /// when nothing is selected — CS6's document tab. The canvas
    /// geometry, the resolution with its unit, and the canvas
    /// background color, each writing through its document.* command.
    pub(super) fn document_properties(&mut self, ui: &mut egui::Ui) {
        ui.label("Document");
        let document = self.editor.session.document();
        let canvas = &document.info.canvas;
        let mut width = canvas.width as f32;
        let mut height = canvas.height as f32;
        let mut resolution = canvas.resolution as f32;
        let mut unit_cm = canvas.resolution_unit == 1;
        let mut background_raw = canvas.background_raw;
        let mut applied = false;
        ui.horizontal(|ui| {
            let w = ui.add(egui::DragValue::new(&mut width).speed(1.0).prefix("W "));
            let h = ui.add(egui::DragValue::new(&mut height).speed(1.0).prefix("H "));
            applied |= (w.lost_focus() && w.changed()) || (h.lost_focus() && h.changed());
        });
        if applied && width > 0.0 && height > 0.0 {
            self.run_command(
                "document.setCanvasRect",
                serde_json::json!({
                    "page": self.editor.frame,
                    "x": canvas.x, "y": canvas.y,
                    "width": width, "height": height,
                }),
            );
        }
        ui.horizontal(|ui| {
            let res = ui
                .add(
                    egui::DragValue::new(&mut resolution)
                        .speed(1.0)
                        .suffix(" px"),
                )
                .on_hover_text("document resolution");
            if ui.checkbox(&mut unit_cm, "per cm").changed() {
                self.run_command(
                    "document.setResolution",
                    serde_json::json!({
                        "resolution": resolution,
                        "unit": if unit_cm { "cm" } else { "inch" },
                    }),
                );
            }
            if res.lost_focus() && res.changed() && resolution > 0.0 {
                self.run_command(
                    "document.setResolution",
                    serde_json::json!({
                        "resolution": resolution,
                        "unit": if unit_cm { "cm" } else { "inch" },
                    }),
                );
            }
        });
        ui.horizontal(|ui| {
            ui.label("canvas color");
            for (name, color_raw) in Self::FILL_STROKE_PALETTE {
                let raw = color_raw as u32;
                let swatch = Color32::from_rgba_unmultiplied(
                    (raw & 0xFF) as u8,
                    ((raw >> 8) & 0xFF) as u8,
                    ((raw >> 16) & 0xFF) as u8,
                    ((raw >> 24) & 0xFF) as u8,
                );
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("  ").background_color(swatch),
                    ))
                    .on_hover_text(name)
                    .clicked()
                {
                    self.run_command(
                        "document.setCanvasColor",
                        serde_json::json!({ "color_raw": color_raw }),
                    );
                }
            }
            let _ = &mut background_raw;
        });
    }

    /// Multi-selection properties (22.4): a count and union-bounds
    /// readout, plus the shared display quartet — each edit fans out
    /// to every selected object. Renames stay a per-object affordance
    /// (CS6 renames the active element only).
    pub(super) fn multi_selection_properties(&mut self, ui: &mut egui::Ui, ids: &[doc::ObjectId]) {
        ui.label(format!("{} objects selected", ids.len()));
        let document = self.editor.session.document();
        if let Some(bounds) = crate::editor::union_selection_bounds(document, ids) {
            ui.label(format!(
                "bounds {:.1}, {:.1} — {:.1} x {:.1}",
                bounds.x0,
                bounds.y0,
                bounds.width(),
                bounds.height(),
            ));
        }
        ui.separator();
        // The display quartet, fanned out per id: opacity shows the
        // selection minimum, blend the minimum index, visible the AND,
        // locked the OR — so the controls read as "mixed" until every
        // member agrees, and any edit rewrites all members.
        let members: Vec<(doc::ObjectId, i64, i64, bool, bool)> = ids
            .iter()
            .filter_map(|id| {
                let fields = match self.editor.document().registry.get(*id) {
                    Some(ObjectData::Path(path)) => {
                        (path.opacity, path.blend, path.visible, path.locked)
                    }
                    Some(ObjectData::Text(text)) => {
                        (text.opacity, text.blend, text.visible, text.locked)
                    }
                    Some(ObjectData::Group(group)) => {
                        (group.opacity, group.blend, group.visible, group.locked)
                    }
                    Some(ObjectData::Image(image)) => {
                        (image.opacity, image.blend, image.visible, image.locked)
                    }
                    _ => return None,
                };
                Some((*id, fields.0, fields.1, fields.2, fields.3))
            })
            .collect();
        let opacity = members.iter().map(|&(_, o, ..)| o).min().unwrap_or(1000);
        let blend = members.iter().map(|&(_, _, b, ..)| b).min().unwrap_or(0);
        let visible = members.iter().all(|&(_, _, _, v, _)| v);
        let locked = members.iter().any(|&(_, _, _, _, l)| l);
        let mut opacity = opacity;
        if ui
            .add(egui::Slider::new(&mut opacity, 0..=1000).text("opacity"))
            .changed()
        {
            for (id, ..) in &members {
                self.run_command(
                    "object.setOpacity",
                    serde_json::json!({ "id": id, "opacity": opacity }),
                );
            }
        }
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
                        for (id, ..) in &members {
                            self.run_command(
                                "object.setBlend",
                                serde_json::json!({ "id": id, "blend": index }),
                            );
                        }
                    }
                }
            });
        ui.horizontal(|ui| {
            if ui.checkbox(&mut { visible }, "visible").changed() {
                for (id, ..) in &members {
                    self.run_command(
                        "object.setVisible",
                        serde_json::json!({ "id": id, "visible": !visible }),
                    );
                }
            }
            if ui.checkbox(&mut { locked }, "locked").changed() {
                for (id, ..) in &members {
                    self.run_command(
                        "object.setLocked",
                        serde_json::json!({ "id": id, "locked": !locked }),
                    );
                }
            }
        });
    }

    /// Group properties: the shared display controls, the group's
    /// geometry, and the objects it holds.
    ///
    /// CS6's Properties panel shows the group's own attributes; the
    /// membership list lives here because the Layers panel shows the
    /// page's layer stack, not a group's contents.
    pub(super) fn group_properties(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        group: &GroupObject,
    ) {
        ui.label(format!("object {} (gen {})", id.index(), id.generation()));
        self.geometry_fields(ui, id);
        self.display_controls(
            ui,
            id,
            DisplayState {
                opacity: group.opacity,
                blend: group.blend,
                visible: group.visible,
                locked: group.locked,
                name: group.name.clone(),
            },
        );
        ui.heading("Contents");
        if group.children.is_empty() {
            ui.label("(empty group)");
        }
        // Snapshot the child labels: the registry is borrowed
        // read-only while the panel draws.
        let children: Vec<(doc::ObjectId, String)> = group
            .children
            .iter()
            .map(|child| {
                let label = self
                    .editor
                    .document()
                    .registry
                    .get(*child)
                    .map(object_label)
                    .unwrap_or_else(|| format!("object {} (stale)", child.index()));
                (*child, label)
            })
            .collect();
        for (child, label) in children {
            // CS6: clicking a member row selects that child inside the
            // group — enter-group by selection.
            if ui.selectable_label(false, &label).clicked() {
                self.editor.set_selection(vec![child]);
            }
        }
    }

    /// Bitmap properties: the pixel geometry as a readout, then the
    /// shared display controls.
    ///
    /// The X/Y/W/H strip is read-only here and the CS6 fill/stroke
    /// editors are absent: `object.setBounds` refuses images (a
    /// transform that rewrites geometry needs the resample pass —
    /// decision 0019) and `IMGv` carries no fill/stroke nodes in this
    /// build. Both are recorded as open work in `docs/CS6-PARITY.md`
    /// rather than faked.
    pub(super) fn image_properties(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        image: &ImageObject,
    ) {
        ui.label(format!("object {} (gen {})", id.index(), id.generation()));
        ui.label(format!(
            "bitmap {}x{} px, {} tile(s) of {} px",
            image.pixels.0,
            image.pixels.1,
            image.tiles.len(),
            image.tile_size
        ));
        ui.label(format!(
            "at ({:.1}, {:.1}), offset ({:.1}, {:.1})",
            image.location.x, image.location.y, image.offset.x, image.offset.y
        ));
        self.display_controls(
            ui,
            id,
            DisplayState {
                opacity: image.opacity,
                blend: image.blend,
                visible: image.visible,
                locked: image.locked,
                name: image.name.clone(),
            },
        );
    }

    /// Slice properties: the name, the slice type, the frame
    /// geometry, and the anchor link and alt text.
    ///
    /// The link fields are the shared editor the Window ▸ URL panel
    /// renders, so the two surfaces cannot show different values.
    pub(super) fn slice_properties(
        &mut self,
        ui: &mut egui::Ui,
        id: doc::ObjectId,
        slice: &SliceObject,
    ) {
        ui.label(format!("object {} (gen {})", id.index(), id.generation()));
        // The name (`object.rename` covers slices).
        let mut name = slice.name.clone().unwrap_or_default();
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
        // The type: foreground (`0`), background (`2`), or hotspot
        // (no `TSLi` node).
        let current = match slice.slice_type {
            Some(0) => "foreground",
            Some(2) => "background",
            _ => "hotspot",
        };
        egui::ComboBox::from_label("type")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for (choice, label) in [
                    ("foreground", "foreground"),
                    ("background", "background"),
                    ("hotspot", "hotspot"),
                ] {
                    if ui.selectable_label(current == choice, label).clicked() {
                        let value = match choice {
                            "foreground" => serde_json::json!(0),
                            "background" => serde_json::json!(2),
                            _ => serde_json::Value::Null,
                        };
                        let _ = self.editor.apply(
                            "slice.setType",
                            serde_json::json!({ "id": id, "slice_type": value }),
                        );
                    }
                }
            });
        let shape = match slice.shape {
            0 => "rectangle".to_owned(),
            1 => "ellipse".to_owned(),
            2 => "polygon".to_owned(),
            other => format!("unknown ({other})"),
        };
        ui.label(format!("shape: {shape}"));
        self.geometry_fields(ui, id);
        self.slice_link_editor(ui, id, slice.link.as_ref(), slice.alt.as_ref(), false);
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
            let x_field = ui.add(egui::DragValue::new(&mut x).speed(1.0).prefix("X "));
            let y_field = ui.add(egui::DragValue::new(&mut y).speed(1.0).prefix("Y "));
            let w_field = ui.add(egui::DragValue::new(&mut w).speed(1.0).prefix("W "));
            let h_field = ui.add(egui::DragValue::new(&mut h).speed(1.0).prefix("H "));
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
                    // The effect's own name: CS6's `setEffectName`
                    // renames it under its category.
                    let mut new_name = effect.internal_name.clone();
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut new_name).desired_width(96.0));
                        if ui.small_button("rename").clicked() {
                            self.rename_filter_effect(
                                &effect.category,
                                &effect.internal_name,
                                &new_name,
                            );
                        }
                    });
                    self.filter_param_rows(ui, id, index, effect);
                });
        }
        self.filter_add_menu(ui, id);
        self.filter_bake_fields(ui);
    }

    /// Rename the effect under its category — CS6's
    /// `dom.setEffectName(category, oldName, newName)`. The verb
    /// walks the *selection*, so the view's selection is synced into
    /// the document first.
    pub(super) fn rename_filter_effect(&mut self, category: &str, old_name: &str, new_name: &str) {
        if new_name.is_empty() {
            self.status = "the new name is empty".to_owned();
            return;
        }
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "filter.setEffectName",
            serde_json::json!({
                "category": category, "old_name": old_name, "new_name": new_name,
            }),
        );
    }

    /// The bake section: CS6's `dom.filterSelection*` verbs, which
    /// apply a filter to the selection and bake it into the pixels.
    ///
    /// The bake verbs read the document's selection, so the panel
    /// syncs the view's selection into the document first.
    pub(super) fn filter_bake_fields(&mut self, ui: &mut egui::Ui) {
        ui.heading("Bake");
        let mut category = String::new();
        let mut name = String::new();
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut category).hint_text("category"));
            ui.add(egui::TextEdit::singleline(&mut name).hint_text("filter name"));
        });
        ui.horizontal(|ui| {
            if ui.button("bake").clicked() {
                self.bake_filter(&category, &name);
            }
            if ui
                .button("bake by catalog name")
                .on_hover_text(
                    "CS6's filterSelectionByName: the filter's GUID comes from the catalog",
                )
                .clicked()
            {
                self.bake_filter_by_name(&category, &name);
            }
        });
        let mut guid = String::new();
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut guid).hint_text("EffectMoaID"));
            if ui.button("bake by id").clicked() {
                self.bake_filter_by_id(&guid);
            }
        });
        // The convolution kernel: raw integer coefficients, row
        // major, exactly as the dom takes them.
        let mut width = 3i64;
        let mut height = 3i64;
        let mut values = "0,0,0,0,1,0,0,0,0".to_owned();
        let mut affects_alpha = false;
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut width)
                    .speed(1.0)
                    .range(1..=7)
                    .prefix("w "),
            );
            ui.add(
                egui::DragValue::new(&mut height)
                    .speed(1.0)
                    .range(1..=7)
                    .prefix("h "),
            );
            ui.add(
                egui::TextEdit::singleline(&mut values)
                    .desired_width(160.0)
                    .hint_text("kernel values"),
            );
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut affects_alpha, "affects alpha");
            if ui.button("convolve").clicked() {
                match parse_kernel(&values, width, height) {
                    Ok(kernel) => self.convolve_selection(width, height, kernel, affects_alpha),
                    Err(detail) => self.status = format!("kernel refused: {detail}"),
                }
            }
        });
        // The motion blur: the dom's three kinds.
        let mut blur_type = "linear".to_owned();
        let mut angle = 0.0f64;
        let mut distance = 10u32;
        let mut samples = 8u32;
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("blur")
                .selected_text(blur_type.clone())
                .show_ui(ui, |ui| {
                    for choice in ["linear", "radial", "zoom"] {
                        if ui.selectable_label(blur_type == choice, choice).clicked() {
                            blur_type = choice.to_owned();
                        }
                    }
                });
            ui.add(egui::DragValue::new(&mut angle).speed(1.0).prefix("angle "));
            ui.add(
                egui::DragValue::new(&mut distance)
                    .speed(1.0)
                    .range(0..=400)
                    .prefix("distance "),
            );
            ui.add(
                egui::DragValue::new(&mut samples)
                    .speed(1.0)
                    .range(0..=100)
                    .prefix("samples "),
            );
        });
        if ui.button("motion blur").clicked() {
            self.motion_blur_selection(&blur_type, angle, distance, samples);
        }
    }

    /// Bake the named filter into the selection.
    pub(super) fn bake_filter(&mut self, category: &str, name: &str) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "filter.selection",
            serde_json::json!({ "category": category, "name": name, "params": [] }),
        );
    }

    /// Bake the named filter into the selection, taking the filter's
    /// GUID from the catalog.
    pub(super) fn bake_filter_by_name(&mut self, category: &str, name: &str) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "filter.selectionByName",
            serde_json::json!({ "category": category, "name": name, "params": [] }),
        );
    }

    /// Bake the filter with the given `EffectMoaID` into the
    /// selection.
    pub(super) fn bake_filter_by_id(&mut self, guid: &str) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "filter.selectionByID",
            serde_json::json!({ "id": guid, "params": [] }),
        );
    }

    /// Bake an arbitrary integer kernel into the selection.
    pub(super) fn convolve_selection(
        &mut self,
        width: i64,
        height: i64,
        values: Vec<i64>,
        affects_alpha: bool,
    ) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "filter.convolveSelection",
            serde_json::json!({
                "kernel_width": width,
                "kernel_height": height,
                "kernel_values": values,
                "affects_alpha": affects_alpha,
            }),
        );
    }

    /// Bake a motion blur into the selection.
    pub(super) fn motion_blur_selection(
        &mut self,
        blur_type: &str,
        angle: f64,
        distance: u32,
        samples: u32,
    ) {
        if !self.sync_document_selection() {
            return;
        }
        self.run_command(
            "filter.motionBlurSelection",
            serde_json::json!({
                "blur_type": blur_type,
                "angle": angle,
                "distance": distance,
                "samples": samples,
            }),
        );
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
        self.last_filter = Some(preset.name);
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

/// A display label for one object: its stored name when it has one,
/// then its kind. The kind is always shown so an unnamed object is
/// still identifiable in a list.
fn object_label(object: &ObjectData) -> String {
    let (kind, name) = match object {
        ObjectData::Path(path) => ("path", path.name.clone()),
        ObjectData::Text(text) => ("text", text.name.clone()),
        ObjectData::Group(group) => ("group", group.name.clone()),
        ObjectData::Image(image) => ("bitmap", image.name.clone()),
        ObjectData::Slice(slice) => ("slice", slice.name.clone()),
        ObjectData::SymbolInstance(instance) => ("instance", instance.name.clone()),
    };
    match name {
        Some(name) if !name.is_empty() => format!("{name} ({kind})"),
        _ => format!("(unnamed {kind})"),
    }
}

/// The button's export options, read from the first slice linked to
/// the symbol. `found` records whether any slice is linked at all —
/// the engine refuses `symbol.setButtonOptions` when none is.
#[derive(Default)]
struct LinkedSliceFacts {
    found: bool,
    link: String,
    alt: String,
    slice_name: String,
}

/// The convolution kernel's coefficients: the dom takes them raw,
/// row major, and their count must match the kernel's sides.
fn parse_kernel(text: &str, width: i64, height: i64) -> Result<Vec<i64>, String> {
    let cells = width * height;
    if cells <= 0 {
        return Err("the kernel has no cells".to_owned());
    }
    let mut values = Vec::new();
    for part in text.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        match trimmed.parse::<i64>() {
            Ok(value) => values.push(value),
            Err(_) => return Err(format!("{trimmed:?} is not an integer")),
        }
    }
    if values.len() as i64 != cells {
        return Err(format!(
            "{} values for a {width}x{height} kernel",
            values.len()
        ));
    }
    Ok(values)
}
