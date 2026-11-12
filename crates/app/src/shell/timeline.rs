//! The States panel: the page's animation strip (CS6 phase 20.3).
//!
//! CS6's animation workflow is a strip of *states*: one per frame of
//! the animation, each with a delay, and the animator moves between
//! them. `FireCraft` keeps CS6's naming — the panel is "States", the
//! commands are `frame.*` — and adds the two things the workflow
//! needs to be usable as a daily driver: a per-state delay that
//! reaches the GIF exporter, and the onion-skinning toggle that ghosts
//! the neighbouring states behind the current one.
//!
//! The model is the authority (decision 0081): the delay lives in the
//! page's `frame_durations` record, the onion counts in the page's
//! `onion_before` / `onion_after`. The panel reads them and writes
//! them through the engine's commands, so the compositor and the
//! panel can never disagree about what the page shows.

use super::*;

/// One state of the page, as the strip shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FrameRow {
    /// The state's index on the page.
    pub index: usize,
    /// The state's delay, in hundredths of a second.
    pub duration: i64,
}

/// The strip as the panel sees it: the page's states, their delays,
/// and the page's onion-skinning counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TimelineStrip {
    /// One row per state, in order.
    pub frames: Vec<FrameRow>,
    /// How many states before the current one are ghosted.
    pub onion_before: i64,
    /// How many states after the current one are ghosted.
    pub onion_after: i64,
}

impl TimelineStrip {
    /// Reads the strip off the document. `None` when the page is out
    /// of range.
    ///
    /// The state count is the page's own `FRCi` — the count every
    /// frame command accepts — and never the tallest layer's cell
    /// list: the two can differ in a corrupt document, and the
    /// commands are the authority on how many states exist.
    pub(super) fn read(document: &doc::Document, page: usize) -> Option<Self> {
        let page = document.pages.get(page)?;
        let count = page.info.frame_count.max(1) as usize;
        let frames = (0..count)
            .map(|index| FrameRow {
                index,
                duration: page
                    .info
                    .frame_durations
                    .get(index)
                    .copied()
                    .unwrap_or(doc::DEFAULT_FRAME_DURATION),
            })
            .collect();
        Some(Self {
            frames,
            onion_before: page.info.onion_before,
            onion_after: page.info.onion_after,
        })
    }

    /// The delay of the state at `index`, in hundredths of a second.
    pub(super) fn duration_at(&self, index: usize) -> i64 {
        self.frames
            .iter()
            .find(|row| row.index == index)
            .map_or(doc::DEFAULT_FRAME_DURATION, |row| row.duration)
    }

    /// Whether the page ghosts any neighbour at all.
    pub(super) fn onionning(&self) -> bool {
        self.onion_before > 0 || self.onion_after > 0
    }
}

impl Shell {
    /// The States panel: the page's animation strip.
    ///
    /// One row per state, the active one marked; the delay of the
    /// active state below it; the onion-skinning counts under that.
    /// Every edit goes through an engine command, so the model, the
    /// journal, and the undo stack stay in step.
    pub(super) fn states_section(&mut self, ui: &mut egui::Ui) {
        let Some(strip) = TimelineStrip::read(self.editor.document(), self.editor.page) else {
            return;
        };
        let page = self.editor.page;
        // A page switch can leave the view on a state the new page
        // does not have; the strip reads the last one it does.
        let current = self.editor.frame.min(strip.frames.len().saturating_sub(1));

        // The strip itself: one button per state.
        egui::ScrollArea::horizontal()
            .id_salt("states_strip")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for row in &strip.frames {
                        let response =
                            ui.selectable_label(current == row.index, format!("{}", row.index + 1));
                        if response.clicked() {
                            self.editor.frame = row.index;
                            self.raster = None;
                        }
                        response.on_hover_text(format!(
                            "State {} — delay {} cs",
                            row.index + 1,
                            row.duration
                        ));
                    }
                });
            });

        // The delay of the active state. Applied when the field loses
        // the focus, so a drag is one journal entry, not one per frame.
        ui.horizontal(|ui| {
            ui.label("Delay:");
            let mut duration = strip.duration_at(current);
            let response = ui
                .add(
                    egui::DragValue::new(&mut duration)
                        .speed(1.0)
                        .range(0..=65_535)
                        .suffix(" cs"),
                )
                .on_hover_text("how long this state is shown (hundredths of a second)");
            if response.lost_focus() && response.changed() {
                self.run_command(
                    "frame.setDuration",
                    serde_json::json!({
                        "page": page, "frame": current, "duration": duration,
                    }),
                );
            }
        });

        // The state's own commands: CS6's Add State, Duplicate State
        // and Delete State, then Distribute to States — the step that
        // turns a tweened run of instances into an animation.
        ui.horizontal(|ui| {
            if ui
                .button("Add")
                .on_hover_text("CS6's Add State: an empty state after the current one")
                .clicked()
            {
                self.add_state(page, current);
            }
            if ui
                .button("Duplicate")
                .on_hover_text("CS6's Duplicate State: a copy of the current state after it")
                .clicked()
            {
                self.duplicate_state(page, current);
            }
            if ui
                .button("Delete")
                .on_hover_text("CS6's Delete State: the current state goes, the last one stays")
                .clicked()
            {
                self.delete_state(page, current);
            }
            if ui
                .button("Distribute")
                .on_hover_text("CS6's Distribute to States: the layer's items move one per state")
                .clicked()
            {
                self.distribute_to_states(page, current);
            }
        });

        // The onion-skinning counts: how many states ghost behind the
        // current one on each side. Zero on both sides is off.
        ui.horizontal(|ui| {
            let mut before = strip.onion_before.max(0);
            let mut after = strip.onion_after.max(0);
            let mut on = strip.onionning();
            let mut applied: Option<(i64, i64)> = None;
            if ui
                .checkbox(&mut on, "Onion skin")
                .on_hover_text("ghost the neighbouring states behind the current one")
                .changed()
            {
                // Switching on keeps the counts the user chose, but
                // ghosts at least one state on each side.
                applied = Some(if on {
                    (before.max(1), after.max(1))
                } else {
                    (0, 0)
                });
            }
            let before_field = ui.add(
                egui::DragValue::new(&mut before)
                    .speed(1.0)
                    .range(0..=64)
                    .prefix("before "),
            );
            let after_field = ui.add(
                egui::DragValue::new(&mut after)
                    .speed(1.0)
                    .range(0..=64)
                    .prefix("after "),
            );
            if before_field.lost_focus() && before_field.changed() {
                applied = Some((before, after));
            }
            if after_field.lost_focus() && after_field.changed() {
                applied = Some((before, after));
            }
            if let Some((before, after)) = applied {
                self.run_command(
                    "page.setOnionSkinning",
                    serde_json::json!({
                        "page": page, "before": before, "after": after,
                    }),
                );
            }
        });
    }

    /// CS6's Add State: an empty state right after the current one,
    /// which becomes the active state.
    fn add_state(&mut self, page: usize, current: usize) {
        let at = current + 1;
        if self.run_command(
            "frame.add",
            serde_json::json!({
                "page": page, "how_many": 1,
                "location": "after current", "current": current,
            }),
        ) {
            self.editor.frame = at;
            self.raster = None;
        }
    }

    /// CS6's Duplicate State: a copy of the current state lands right
    /// after it and becomes the active state.
    fn duplicate_state(&mut self, page: usize, current: usize) {
        let at = current + 1;
        if self.run_command(
            "frame.duplicate",
            serde_json::json!({
                "page": page, "index": current, "how_many": 1,
                "location": "after current", "current": current,
            }),
        ) {
            self.editor.frame = at;
            self.raster = None;
        }
    }

    /// CS6's Delete State: the current state goes and the one after it
    /// takes its place. The page's last state stays (the engine
    /// refuses to empty a page).
    fn delete_state(&mut self, page: usize, current: usize) {
        if self.run_command(
            "frame.delete",
            serde_json::json!({ "page": page, "index": current, "how_many": 1 }),
        ) {
            let count = self
                .editor
                .document()
                .pages
                .get(page)
                .map_or(1, |p| p.info.frame_count.max(1) as usize);
            self.editor.frame = current.min(count.saturating_sub(1));
            self.raster = None;
        }
    }

    /// CS6's Distribute to States: the active layer's items in the
    /// current state move one per state — the step that turns a
    /// tweened run of instances into an animation.
    fn distribute_to_states(&mut self, page: usize, current: usize) {
        let layer = self.editor.active_layer();
        self.run_command(
            "frame.distributeLayer",
            serde_json::json!({ "page": page, "layer": layer, "cell": current }),
        );
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use doc::{Canvas, Document, LayerList, Page, PageInfo};

    /// A page with `count` states and no recorded delays.
    fn document_with(count: i64) -> Document {
        let mut document = Document::default();
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: count,
                frame_count_duplicate: count,
                request_id: 0,
                canvas: Canvas::default(),
                onion_before: 0,
                onion_after: 0,
                frame_durations: Vec::new(),
            },
            name: None,
            is_master: false,
            is_linked: false,
            page_id: None,
            last_export_dir: None,
            last_export_file: None,
            layers: LayerList::default(),
            grid: None,
            guides: doc::Guides::default(),
            export: None,
        });
        document
    }

    #[test]
    fn the_strip_counts_the_page_states() {
        let document = document_with(3);
        let strip = TimelineStrip::read(&document, 0).expect("the page is there");
        assert_eq!(strip.frames.len(), 3);
        assert_eq!(strip.frames[0].index, 0);
        assert_eq!(strip.frames[2].index, 2);
    }

    #[test]
    fn a_page_without_states_still_shows_one() {
        let document = document_with(0);
        let strip = TimelineStrip::read(&document, 0).expect("the page is there");
        assert_eq!(strip.frames.len(), 1);
    }

    #[test]
    fn an_unrecorded_delay_reads_as_the_format_default() {
        let document = document_with(2);
        let strip = TimelineStrip::read(&document, 0).expect("the page is there");
        assert_eq!(strip.duration_at(0), doc::DEFAULT_FRAME_DURATION);
        assert_eq!(strip.duration_at(1), doc::DEFAULT_FRAME_DURATION);
    }

    #[test]
    fn a_recorded_delay_is_read_positionally() {
        let mut document = document_with(3);
        document.pages[0].info.frame_durations = vec![10, 25];
        let strip = TimelineStrip::read(&document, 0).expect("the page is there");
        assert_eq!(strip.duration_at(0), 10);
        assert_eq!(strip.duration_at(1), 25);
        // A record shorter than the state list: the missing state
        // takes the default.
        assert_eq!(strip.duration_at(2), doc::DEFAULT_FRAME_DURATION);
    }

    #[test]
    fn the_onion_counts_come_off_the_page() {
        let mut document = document_with(2);
        document.pages[0].info.onion_before = 1;
        document.pages[0].info.onion_after = 2;
        let strip = TimelineStrip::read(&document, 0).expect("the page is there");
        assert_eq!(strip.onion_before, 1);
        assert_eq!(strip.onion_after, 2);
        assert!(strip.onionning());
    }

    #[test]
    fn a_page_without_onion_skinning_is_off() {
        let document = document_with(2);
        let strip = TimelineStrip::read(&document, 0).expect("the page is there");
        assert!(!strip.onionning());
    }

    #[test]
    fn a_missing_page_has_no_strip() {
        let document = document_with(1);
        assert!(TimelineStrip::read(&document, 9).is_none());
    }

    /// A shell over a fresh document whose page 0 carries `states`
    /// states.
    fn shell_with_states(states: i64) -> Shell {
        let mut shell = super::super::tests::gui_shell(crate::editor::Editor::untitled());
        for _ in 1..states {
            assert!(
                shell.run_command(
                    "frame.add",
                    serde_json::json!({ "page": 0, "how_many": 1, "location": "end" }),
                ),
                "adding a state must succeed",
            );
        }
        shell
    }

    /// The page's state count, as the model holds it.
    fn state_count(shell: &Shell) -> i64 {
        shell.editor.document().pages[0].info.frame_count
    }

    /// The elements of one layer's one state.
    fn elements(shell: &Shell, layer: usize, state: usize) -> Vec<doc::ObjectId> {
        shell.editor.document().pages[0].layers.layers[layer].cells[state]
            .elements
            .clone()
    }

    /// Puts one rectangle into the given layer's given state.
    fn put_rect(shell: &mut Shell, layer: usize, state: usize, x: f64) {
        shell
            .editor
            .apply(
                "object.createPath",
                serde_json::json!({
                    "kind": "rect", "x": x, "y": 0.0,
                    "width": 20.0, "height": 20.0,
                    "page": 0, "layer": layer, "cell": state, "name": null,
                }),
            )
            .expect("the rectangle must land");
    }

    #[test]
    fn add_state_puts_an_empty_state_after_the_current_one() {
        let mut shell = shell_with_states(2);
        let layer = shell.editor.active_layer();
        shell.editor.frame = 0;
        shell.add_state(0, 0);
        assert_eq!(state_count(&shell), 3);
        // The new state becomes the active one.
        assert_eq!(shell.editor.frame, 1);
        // And it is empty: Add State is not Duplicate State.
        assert!(elements(&shell, layer, 1).is_empty());
    }

    #[test]
    fn duplicate_state_copies_the_current_state_after_it() {
        let mut shell = shell_with_states(1);
        let layer = shell.editor.active_layer();
        put_rect(&mut shell, layer, 0, 10.0);
        shell.duplicate_state(0, 0);
        assert_eq!(state_count(&shell), 2);
        assert_eq!(shell.editor.frame, 1);
        // The copy carries the original's content, in its own slot.
        let source = elements(&shell, layer, 0);
        let copy = elements(&shell, layer, 1);
        assert_eq!(source.len(), 1);
        assert_eq!(copy.len(), 1);
        assert_ne!(source[0], copy[0]);
    }

    #[test]
    fn delete_state_drops_the_current_state_and_the_next_one_takes_its_place() {
        let mut shell = shell_with_states(3);
        shell.editor.frame = 1;
        shell.delete_state(0, 1);
        assert_eq!(state_count(&shell), 2);
        assert_eq!(shell.editor.frame, 1);
    }

    #[test]
    fn the_last_state_stays() {
        // The engine refuses to empty a page, so the refusal is
        // reported on the status line and the view does not move.
        let mut shell = shell_with_states(1);
        shell.editor.frame = 0;
        shell.delete_state(0, 0);
        assert_eq!(state_count(&shell), 1);
        assert!(shell.status.contains("frame.delete failed"));
        assert_eq!(shell.editor.frame, 0);
    }

    #[test]
    fn distribute_to_states_moves_the_items_one_per_state() {
        let mut shell = shell_with_states(1);
        let layer = shell.editor.active_layer();
        put_rect(&mut shell, layer, 0, 10.0);
        put_rect(&mut shell, layer, 0, 40.0);
        shell.distribute_to_states(0, 0);
        // Two items, one per state — the page grew to hold them.
        assert_eq!(state_count(&shell), 2);
        assert_eq!(elements(&shell, layer, 0).len(), 1);
        assert_eq!(elements(&shell, layer, 1).len(), 1);
    }

    #[test]
    fn a_single_item_distributes_to_nothing() {
        // CS6's documented no-op: one item cannot be spread.
        let mut shell = shell_with_states(1);
        let layer = shell.editor.active_layer();
        put_rect(&mut shell, layer, 0, 10.0);
        shell.distribute_to_states(0, 0);
        assert_eq!(state_count(&shell), 1);
        assert_eq!(elements(&shell, layer, 0).len(), 1);
    }
}
