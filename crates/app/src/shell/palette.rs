//! The command palette: every registered engine command, searchable,
//! executable — the 100%-registry-coverage door CS6 never had.

use super::*;

/// The command palette: Ctrl+P opens; typing filters `catalog::ids()`
/// by substring; Enter runs the top hit through `Editor::apply` (the
/// same registry the menus and the journal use). Payload templates
/// come from the engine's fuzzdrive table — the same table whose
/// coverage test (`the_call_table_covers_the_catalog`) guarantees one
/// arm per registered command. Templates are built lazily, for the
/// visible page of hits only.
#[derive(Default)]
pub(crate) struct CommandPalette {
    pub(super) open: bool,
    pub(super) filter: String,
    pub(super) selected: usize,
    /// Recently run commands, most recent first (persisted with the
    /// layout). Names not in the catalog are dropped on restore.
    pub(super) recents: Vec<String>,
}

/// Fuzzy score of `name` against `query`: a subsequence match scored
/// by how tightly the query characters cluster — a hit at position
/// `p` earns `SCORE.saturating_sub(p * PENALTY)`, so earlier is
/// better and a deep spread-out match decays toward the floor.
/// Returns `None` when `query` is not a subsequence of `name`. Both
/// sides lowercase before calling.
pub(crate) fn fuzzy_score(name: &str, query: &str) -> Option<i64> {
    const SCORE: i64 = 100;
    const PENALTY: i64 = 10;
    let name = name.to_lowercase();
    let mut score = 0i64;
    let mut cursor = 0usize;
    for ch in query.chars() {
        let found = name[cursor..].find(ch)?;
        let position = cursor + found;
        score += SCORE.saturating_sub((position as i64) * PENALTY);
        cursor = position + 1;
    }
    Some(score)
}

impl Shell {
    /// Toggles the palette (the Ctrl+P shortcut).
    pub(crate) fn toggle_palette(&mut self) {
        self.palette.open = !self.palette.open;
        if self.palette.open {
            self.palette.filter.clear();
            self.palette.selected = 0;
        }
    }

    /// The command ids matching the current filter, best-scored
    /// first. An empty filter shows the recents, then the catalog in
    /// order. Ties and unscored names keep catalog order (stable
    /// sort).
    pub(super) fn palette_matches(&self) -> Vec<&'static str> {
        let query = self.palette.filter.to_lowercase();
        let catalog = engine::catalog::ids();
        if query.is_empty() {
            // Recents first (deduplicated, catalog-resident only),
            // then the remaining catalog in order.
            let mut hits: Vec<&'static str> = self
                .palette
                .recents
                .iter()
                .filter_map(|recent| catalog.iter().copied().find(|name| name == recent))
                .collect();
            for name in &catalog {
                if !hits.contains(name) {
                    hits.push(name);
                }
            }
            return hits;
        }
        let mut scored: Vec<(i64, usize, &'static str)> = catalog
            .into_iter()
            .enumerate()
            .filter_map(|(index, name)| fuzzy_score(name, &query).map(|score| (score, index, name)))
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        scored.into_iter().map(|(_, _, name)| name).collect()
    }
}

impl CommandPalette {
    /// Records a successful run at the front of the recents list
    /// (deduplicated, capped at 10).
    pub(super) fn record_recent(&mut self, name: &str) {
        self.recents.retain(|recent| recent != name);
        self.recents.insert(0, name.to_owned());
        self.recents.truncate(10);
    }
}

impl Shell {
    /// The payload template for one command, drawn from the fuzzdrive
    /// table against the live document.
    pub(super) fn payload_for(&self, name: &str) -> serde_json::Value {
        self.payload_for_seed(name, 0)
    }

    /// The payload template for one command under one seed.
    pub(super) fn payload_for_seed(&self, name: &str, seed: u64) -> serde_json::Value {
        let mut rng = engine::fuzzdrive::FuzzRng::new(seed);
        engine::fuzzdrive::payload_template(name, &mut rng, self.editor.document())
    }

    /// Grounds a template to the live editor state. The fuzzdrive
    /// table aims targets out of range on purpose (a fuzz driver wants
    /// refused calls); a user does not. Target keys are clamped to the
    /// active page/layer/frame and the selection, so the run is what
    /// the user asked for, not a lottery.
    pub(super) fn grounded(&self, name: &str, seed: u64) -> serde_json::Value {
        let mut payload = self.payload_for_seed(name, seed);
        let Some(map) = payload.as_object_mut() else {
            return payload;
        };
        // Keys that name two path operands (path.boolean's `a`/`b`).
        let object_pair: Option<&[&str]> = match name {
            "path.boolean" => Some(&["a", "b"]),
            _ => None,
        };
        if let Some(keys) = object_pair
            && let Some((id, _)) = self
                .editor
                .document()
                .registry
                .iter()
                .find(|(_, object)| matches!(object, doc::ObjectData::Path(_)))
        {
            let id = serde_json::json!(id);
            for key in keys {
                if let Some(slot) = map.get_mut(*key) {
                    *slot = id.clone();
                }
            }
        }
        // The knife verbs carry `from`/`to` as `[x, y]` pairs and
        // symbol.setScript's `source` is a script string — none of
        // those are the index/identity keys the passes below rewrite.
        let knife = name == "path.knifeElementsFromPoint" || name == "path.knifeElementsFromPoints";
        if knife {
            let origin = serde_json::json!([320.0, 240.0]);
            if let Some(slot) = map.get_mut("from") {
                *slot = origin;
            } else {
                map.insert("from".to_owned(), origin);
            }
        }
        for (key, value) in map.iter_mut() {
            if knife && (key == "from" || key == "to") {
                continue;
            }
            if name == "symbol.setScript" && key == "source" {
                continue;
            }
            // Saved-selection names are user state, not geometry: the
            // restore/delete verbs name an existing slot. When the
            // document has none, save first — the run order inside the
            // catalog loop makes that work out (save precedes
            // restore/delete alphabetically is not guaranteed, so the
            // seeds retry handles it).
            if name.starts_with("selection.") && key == "name" {
                continue;
            }
            match key.as_str() {
                "page" => *value = serde_json::json!(self.editor.page),
                "layer" => {
                    *value = serde_json::json!(self.editor.active_layer());
                }
                "cell" | "frame" | "index" | "from" | "before" | "to" => {
                    // Index keys ride frame/layer/child bounds with
                    // slack on purpose; clamp to the live shape so the
                    // command lands inside it.
                    let bound: usize = match key.as_str() {
                        "index" if name == "frame.duplicate" || name == "page.reorder" => {
                            self.editor.document().info.frame_count.max(1) as usize
                        }
                        "index" | "to"
                            if name.starts_with("object.")
                                || name.starts_with("symbol.")
                                || name.starts_with("path.") =>
                        {
                            self.editor
                                .current_cell()
                                .map_or(1, |c| c.elements.len().max(1))
                        }
                        "from" | "before" if name.starts_with("frame.") => {
                            self.editor.document().info.frame_count.max(1) as usize
                        }
                        "from" | "before" | "to" => self
                            .editor
                            .document()
                            .pages
                            .get(self.editor.page)
                            .map_or(1, |p| p.layers.layers.len().max(1)),
                        _ => self.editor.document().info.frame_count.max(1) as usize,
                    };
                    let raw = value.as_u64().unwrap_or(0) as usize;
                    *value = serde_json::json!(raw.min(bound.saturating_sub(1)));
                }
                "symbol" => {
                    // The engine matches `symbol_number`, not position.
                    if let Some(definition) = self.editor.document().symbols.first() {
                        *value = serde_json::json!(definition.symbol_number);
                    }
                }
                "id" | "source" | "target" | "attachTo" | "host" | "instance" | "container" => {
                    if let Some(id) = self.editor.selection {
                        *value = serde_json::json!(id);
                    } else if let Some((id, _)) = self.editor.document().registry.iter().next() {
                        *value = serde_json::json!(id);
                    }
                }
                "ids" if !self.editor.selection_set.is_empty() => {
                    *value = serde_json::json!(self.editor.selection_set.clone());
                }
                _ => {}
            }
        }
        payload
    }

    /// Runs one command from the palette.
    ///
    /// The fuzzdrive template aims a stale id at 15% of its calls, so a
    /// single attempt would fail a sixth of the time for no reason the
    /// user can see. A refused call leaves the document untouched (the
    /// engine's refusal contract), which makes a retry safe: the
    /// palette walks the seeds until one lands, and reports the first
    /// error only when every seed is refused.
    pub(super) fn run_palette_command(&mut self, name: &str) -> bool {
        // restore/deleteSaved need a saved slot to name; seed the
        // pixel selection and save "default" first when the document
        // has none. The direct selection write mirrors what
        // bitmap.selectRect does — the app owns the pixel-selection
        // door, and a palette run of restore/deleteSaved is that door.
        if name == "selection.restore" || name == "selection.deleteSaved" {
            let has_default = self
                .editor
                .document()
                .saved_selections
                .iter()
                .any(|saved| saved.name == "default");
            if !has_default {
                // The engine door for a saved selection is a pixel
                // selection plus selection.save. Place an image and
                // select a rect inside it — both through the registry,
                // so this preparatory step journals like any other.
                let before: Vec<_> = self
                    .editor
                    .current_cell()
                    .map(|cell| cell.elements.clone())
                    .unwrap_or_default();
                let placed = self.editor.apply(
                    "bitmap.addImage",
                    serde_json::json!({
                        "x": 0.0, "y": 0.0, "width": 16.0, "height": 16.0,
                        "page": self.editor.page,
                        "layer": self.editor.active_layer(),
                        "cell": self.editor.frame,
                        "name": null,
                    }),
                );
                if placed.is_ok()
                    && let Some(id) = self
                        .editor
                        .current_cell()
                        .and_then(|cell| cell.elements.iter().find(|id| !before.contains(id)))
                        .copied()
                {
                    let _sel = self.editor.apply(
                        "bitmap.selectRect",
                        serde_json::json!({
                            "id": id, "x": 0.0, "y": 0.0,
                            "width": 8.0, "height": 8.0, "mode": "replace",
                        }),
                    );
                }
                let _ = self.editor.apply(
                    "selection.save",
                    serde_json::json!({ "name": "default", "operation": "new or replace" }),
                );
            }
        }
        let mut first_error = None;
        for seed in 0..8 {
            let payload = self.grounded(name, seed);
            match self.editor.apply(name, payload) {
                Ok(()) => return true,
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error.to_string());
                    }
                }
            }
        }
        self.status = format!("{name} failed: {}", first_error.unwrap_or_default());
        false
    }

    /// The palette window: filter field, hit list, run/cancel. Arrow
    /// keys move the selection; Enter runs it.
    pub(super) fn palette_ui(&mut self, ui: &mut egui::Ui) {
        if !self.palette.open {
            return;
        }
        let mut close = false;
        let mut run = false;
        let mut run_index = self.palette.selected;
        egui::Window::new("Command Palette")
            .anchor(egui::Align2::CENTER_TOP, [0.0, 40.0])
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("run:");
                    let field = egui::TextEdit::singleline(&mut self.palette.filter)
                        .id(egui::Id::new("palette_filter"))
                        .desired_width(320.0)
                        .hint_text("type to filter commands…");
                    let response = ui.add(field);
                    response.request_focus();
                    if response.changed() || response.clicked() {
                        self.palette.selected = 0;
                    }
                });
                let names = self.palette_matches();
                if names.is_empty() {
                    ui.weak("(no matching command)");
                    return;
                }
                if self.palette.filter.is_empty() && !self.palette.recents.is_empty() {
                    ui.weak("recent:");
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    self.palette.selected = (self.palette.selected + 1).min(names.len() - 1);
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.palette.selected = self.palette.selected.saturating_sub(1);
                }
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    run = true;
                    run_index = self.palette.selected;
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (index, name) in names.iter().enumerate().take(200) {
                            let label = ui.selectable_label(index == self.palette.selected, *name);
                            if label.clicked() {
                                run = true;
                                run_index = index;
                            }
                            if index == self.palette.selected {
                                label.on_hover_text(self.payload_for(name).to_string());
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    if ui.button("Run").clicked() {
                        run = true;
                        run_index = self.palette.selected;
                    }
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                    ui.weak(format!(
                        "{} of {}",
                        names.len(),
                        engine::catalog::ids().len()
                    ));
                });
            });
        if run {
            let names = self.palette_matches();
            if let Some(name) = names.get(run_index) {
                let name = *name;
                if self.run_palette_command(name) {
                    self.palette.record_recent(name);
                    self.status = format!("palette: {name}");
                }
            }
            close = true;
        }
        if close {
            self.palette.open = false;
        }
    }
}
