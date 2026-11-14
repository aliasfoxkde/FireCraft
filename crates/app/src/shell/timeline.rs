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

/// The animation clock: whether the animation is playing, and where it
/// is (issue #116).
///
/// The clock is a *core over data*: it never reads the wall clock, the
/// environment or the document. The shell hands it the elapsed time
/// and the per-state delays, and it answers with the state to show —
/// so the timing is testable without sleeping and without a running
/// UI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Playback {
    /// Whether the clock is running.
    playing: bool,
    /// Whether the clock wraps around at the last state.
    looping: bool,
    /// The state the clock is showing.
    current: usize,
    /// Hundredths of a second left on the current state.
    remaining_cs: i64,
    /// The page the clock was started on.
    page: usize,
    /// The state count the clock was started with.
    states: usize,
    /// The journal length the clock was started with.
    journal_len: usize,
}

/// What the clock decided this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    /// The clock keeps running; the state does not change.
    Idle,
    /// Show this state.
    Advance(usize),
    /// The clock stopped.
    Stop,
}

impl Playback {
    /// Starts the clock on `state`.
    ///
    /// `durations` is the animation's per-state delay list, already
    /// resolved to the format default where the document records
    /// none. The page and the journal length are recorded as the
    /// clock's *context*: a page switch or a document edit is one of
    /// the conditions that stops playback, and the clock has to know
    /// what the session looked like when it started.
    pub(super) fn start(
        &mut self,
        state: usize,
        durations: &[i64],
        page: usize,
        journal_len: usize,
    ) {
        self.playing = true;
        self.current = state.min(durations.len().saturating_sub(1));
        self.remaining_cs = durations
            .get(self.current)
            .copied()
            .unwrap_or(doc::DEFAULT_FRAME_DURATION);
        self.page = page;
        self.states = durations.len();
        self.journal_len = journal_len;
    }

    /// Stops the clock. The state stays where it is.
    pub(super) fn stop(&mut self) {
        self.playing = false;
    }

    /// Whether the clock is running.
    pub(super) fn playing(&self) -> bool {
        self.playing
    }

    /// Whether the clock wraps around at the last state.
    pub(super) fn looping(&self) -> bool {
        self.looping
    }

    /// Sets whether the clock wraps around at the last state.
    pub(super) fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    /// The state the clock is showing.
    pub(super) fn current(&self) -> usize {
        self.current
    }

    /// Whether the session still looks the way it did when the clock
    /// started: the same page, the same state count, the same journal
    /// length.
    ///
    /// A change in any of them is one of the conditions that stops
    /// playback — a page switch, a state added or removed, or an edit
    /// to the document. The check is a pure function of the two
    /// sides, so it can be tested without a running shell.
    pub(super) fn context_holds(&self, page: usize, states: usize, journal_len: usize) -> bool {
        page == self.page && states == self.states && journal_len == self.journal_len
    }
}

/// Advances the clock by `elapsed_cs` hundredths of a second.
///
/// `durations` is the animation's per-state delay list. The elapsed
/// time and the delays are the clock's *only* inputs — the decision
/// reads nothing else, so the timing is testable without sleeping.
///
/// The clock stops at the last state unless it was told to loop. A
/// frame longer than the delay it crosses carries the surplus into the
/// next state rather than losing it, so a slow frame does not make the
/// animation drift late.
pub(super) fn tick(clock: &mut Playback, elapsed_cs: i64, durations: &[i64]) -> Step {
    if !clock.playing() {
        return Step::Idle;
    }
    if durations.is_empty() {
        clock.stop();
        return Step::Stop;
    }
    // A frame's elapsed time is bounded: a backgrounded window or a
    // debugger breakpoint must not run the animation through the
    // whole timeline at once.
    let elapsed = elapsed_cs.clamp(0, 60_000);
    clock.remaining_cs -= elapsed;
    let mut advanced = false;
    while clock.remaining_cs <= 0 {
        let next = clock.current() + 1;
        if next >= durations.len() {
            if !clock.looping() {
                clock.stop();
                return Step::Stop;
            }
            clock.current = 0;
        } else {
            clock.current = next;
        }
        // A zero delay still advances the state, but it cannot leave
        // the clock with nothing left to count down.
        let delay = durations
            .get(clock.current())
            .copied()
            .unwrap_or(doc::DEFAULT_FRAME_DURATION)
            .max(1);
        clock.remaining_cs += delay;
        advanced = true;
    }
    if advanced {
        Step::Advance(clock.current())
    } else {
        Step::Idle
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
                            // A hand on the strip stops the clock: the
                            // user is choosing the state, so the
                            // animation is no longer driving it.
                            self.playback.stop();
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

        // The playback control: CS6's Play Animation. The clock holds
        // the animation's own timing, so the panel only starts and
        // stops it.
        ui.horizontal(|ui| {
            if self.playback.playing() {
                if ui
                    .button("Stop")
                    .on_hover_text("stop the animation where it is")
                    .clicked()
                {
                    self.playback.stop();
                }
            } else if ui
                .button("Play")
                .on_hover_text("CS6's Play Animation: the states advance on their own delays")
                .clicked()
            {
                self.start_playback(page, current, &strip);
                // egui paints on demand: the frame that starts the
                // clock has to ask for the frame that advances it, or
                // the animation waits for the next input event.
                ui.ctx().request_repaint();
            }
            let mut looping = self.playback.looping();
            if ui
                .checkbox(&mut looping, "Loop")
                .on_hover_text("play the states again from the first one")
                .changed()
            {
                self.playback.set_looping(looping);
            }
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

        // The state's place in the strip: CS6's reorderFrame, as a
        // move one place left and a move one place right.
        ui.horizontal(|ui| {
            if ui
                .button("Move left")
                .on_hover_text("move this state one place left in the strip")
                .clicked()
            {
                self.move_state(page, current, -1);
            }
            if ui
                .button("Move right")
                .on_hover_text("move this state one place right in the strip")
                .clicked()
            {
                self.move_state(page, current, 1);
            }
        });

        // CS6's selection fans: the selection goes onto the page's
        // other states.
        ui.horizontal(|ui| {
            if ui
                .button("Duplicate to next")
                .on_hover_text(
                    "CS6's duplicateSelectionToFrames: the selection is copied onto the next state",
                )
                .clicked()
            {
                self.duplicate_selection_to_frames(page, current, "next");
            }
            if ui
                .button("Duplicate to all")
                .on_hover_text("the selection is copied onto every state of the page")
                .clicked()
            {
                self.duplicate_selection_to_frames(page, current, "all");
            }
            if ui
                .button("Distribute selection")
                .on_hover_text(
                    "CS6's distributeSelectionToFrames: the selected items move one per state",
                )
                .clicked()
            {
                self.distribute_selection_to_frames(page, current);
            }
        });
        ui.horizontal(|ui| {
            let mut first = self.fan_first;
            let mut last = self.fan_last;
            let first_field = ui.add(
                egui::DragValue::new(&mut first)
                    .speed(1.0)
                    .range(-1..=64)
                    .prefix("from "),
            );
            let last_field = ui.add(
                egui::DragValue::new(&mut last)
                    .speed(1.0)
                    .range(-1..=64)
                    .prefix("to "),
            );
            if first_field.changed() {
                self.fan_first = first;
            }
            if last_field.changed() {
                self.fan_last = last;
            }
            if ui
                .button("Duplicate to range")
                .on_hover_text("CS6's duplicateSelectionToFrameRange: the selection is copied across the range")
                .clicked()
            {
                let (first, last) = (self.fan_first, self.fan_last);
                self.duplicate_selection_to_frame_range(page, current, first, last);
            }
        });

        // The active layer's cell in the current state: visible and
        // locked. The flags live on the cell, so they belong to the
        // layer, not to the page.
        let layer = self.editor.active_layer();
        let flags = self
            .editor
            .document()
            .pages
            .get(page)
            .and_then(|page| page.layers.layers.get(layer))
            .and_then(|layer| layer.cells.get(current))
            .map(|cell| (cell.visible, cell.locked));
        if let Some((visible, locked)) = flags {
            ui.horizontal(|ui| {
                let mut visible = visible;
                if ui
                    .checkbox(&mut visible, "visible")
                    .on_hover_text("show the active layer's cell in this state")
                    .changed()
                {
                    self.set_cell_visible(page, layer, current, visible);
                }
                let mut locked = locked;
                if ui
                    .checkbox(&mut locked, "locked")
                    .on_hover_text("lock the active layer's cell in this state")
                    .changed()
                {
                    self.set_cell_locked(page, layer, current, locked);
                }
            });
        }

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

    /// Starts the animation clock on the current state.
    ///
    /// The clock records the page, the state count and the journal
    /// length as it starts: those are the session facts a page switch,
    /// an edit or a state-count change would contradict, and the
    /// clock stops when one of them no longer holds.
    fn start_playback(&mut self, page: usize, current: usize, strip: &TimelineStrip) {
        let durations: Vec<i64> = strip.frames.iter().map(|row| row.duration).collect();
        let journal_len = self.editor.session.journal_len();
        self.playback.start(current, &durations, page, journal_len);
    }

    /// Advances the animation clock by one frame's elapsed time.
    ///
    /// The clock stops on the conditions CS6 stops on: a document
    /// edit, a page switch, a change in the state count, and the end
    /// of the last state. The first three are facts about the
    /// session rather than about the clock, so they are checked
    /// here; the last one is the clock's own decision.
    pub(super) fn playback_tick(&mut self, elapsed_cs: i64) {
        if !self.playback.playing() {
            return;
        }
        let page = self.editor.page;
        let Some(strip) = TimelineStrip::read(self.editor.document(), page) else {
            self.playback.stop();
            return;
        };
        // The session facts the clock started with: the same page, the
        // same state count, the same journal length. One of them
        // failing is a page switch, a state added or removed, or an
        // edit to the document.
        let journal_len = self.editor.session.journal_len();
        if !self
            .playback
            .context_holds(page, strip.frames.len(), journal_len)
        {
            self.playback.stop();
            return;
        }
        let durations: Vec<i64> = strip.frames.iter().map(|row| row.duration).collect();
        match tick(&mut self.playback, elapsed_cs, &durations) {
            Step::Idle => {}
            Step::Advance(index) => {
                self.editor.frame = index;
                self.raster = None;
            }
            Step::Stop => self.playback.stop(),
        }
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

    /// CS6's reorderFrame: the state at `current` moves one place
    /// left (`step` -1) or right (`step` 1) in the strip.
    ///
    /// The engine's `before` is an index in the strip's own
    /// numbering, and it shifts the list by one when the moved state
    /// came first — so a move to the right names the state *after*
    /// the target, and a move to the left names the target itself.
    fn move_state(&mut self, page: usize, current: usize, step: i32) {
        let total = self
            .editor
            .document()
            .pages
            .get(page)
            .map_or(1, |page| page.info.frame_count.max(1) as usize);
        let before = if step < 0 {
            if current == 0 {
                return;
            }
            current - 1
        } else {
            if current + 1 >= total {
                return;
            }
            current + 2
        };
        self.run_command(
            "frame.reorder",
            serde_json::json!({
                "page": page, "from": current, "before": before, "make_copy": false,
            }),
        );
    }

    /// CS6's duplicateSelectionToFrames: the selection is copied onto
    /// the states the fan names — "next", "previous", "end" or
    /// "all".
    fn duplicate_selection_to_frames(&mut self, page: usize, current: usize, which: &str) {
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command(
            "selection.duplicateToFrames",
            serde_json::json!({ "page": page, "frame": current, "which": which }),
        );
    }

    /// CS6's duplicateSelectionToFrameRange: the selection is copied
    /// across an inclusive range of states.
    fn duplicate_selection_to_frame_range(
        &mut self,
        page: usize,
        current: usize,
        first: i64,
        last: i64,
    ) {
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command(
            "selection.duplicateToFrameRange",
            serde_json::json!({
                "page": page, "frame": current, "first": first, "last": last,
            }),
        );
    }

    /// CS6's distributeSelectionToFrames: the selected items move
    /// one per state, starting at the current one.
    fn distribute_selection_to_frames(&mut self, page: usize, current: usize) {
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command(
            "selection.distributeToFrames",
            serde_json::json!({ "page": page, "frame": current }),
        );
    }

    /// The active layer's cell in one state: shown or hidden.
    fn set_cell_visible(&mut self, page: usize, layer: usize, cell: usize, visible: bool) {
        self.run_command(
            "cell.setVisible",
            serde_json::json!({
                "page": page, "layer": layer, "cell": cell, "visible": visible,
            }),
        );
    }

    /// The active layer's cell in one state: locked or unlocked.
    fn set_cell_locked(&mut self, page: usize, layer: usize, cell: usize, locked: bool) {
        self.run_command(
            "cell.setLocked",
            serde_json::json!({
                "page": page, "layer": layer, "cell": cell, "locked": locked,
            }),
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

    /// The cell flags of one layer's one state.
    fn flags(shell: &Shell, layer: usize, state: usize) -> (bool, bool) {
        let cell = &shell.editor.document().pages[0].layers.layers[layer].cells[state];
        (cell.visible, cell.locked)
    }

    #[test]
    fn moving_a_state_left_swaps_it_with_the_one_before() {
        let mut shell = shell_with_states(3);
        let layer = shell.editor.active_layer();
        put_rect(&mut shell, layer, 0, 10.0);
        put_rect(&mut shell, layer, 2, 40.0);
        let moved = elements(&shell, layer, 2)[0];
        shell.move_state(0, 2, -1);
        // The state that sat third now sits second, and the empty
        // state takes its place.
        assert_eq!(elements(&shell, layer, 1), vec![moved]);
        assert!(elements(&shell, layer, 2).is_empty());
    }

    #[test]
    fn moving_a_state_right_swaps_it_with_the_one_after() {
        let mut shell = shell_with_states(3);
        let layer = shell.editor.active_layer();
        put_rect(&mut shell, layer, 0, 10.0);
        put_rect(&mut shell, layer, 1, 40.0);
        let moved = elements(&shell, layer, 1)[0];
        shell.move_state(0, 1, 1);
        assert_eq!(elements(&shell, layer, 2), vec![moved]);
        assert!(elements(&shell, layer, 1).is_empty());
    }

    #[test]
    fn the_first_and_last_states_do_not_move_off_the_strip() {
        let mut shell = shell_with_states(2);
        let layer = shell.editor.active_layer();
        put_rect(&mut shell, layer, 0, 10.0);
        let first = elements(&shell, layer, 0)[0];
        shell.move_state(0, 0, -1);
        shell.move_state(0, 1, 1);
        assert_eq!(elements(&shell, layer, 0), vec![first]);
    }

    #[test]
    fn a_cell_hides_and_locks_through_the_commands() {
        let mut shell = shell_with_states(2);
        let layer = shell.editor.active_layer();
        assert_eq!(flags(&shell, layer, 0), (true, false));
        shell.set_cell_visible(0, layer, 0, false);
        shell.set_cell_locked(0, layer, 0, true);
        assert_eq!(flags(&shell, layer, 0), (false, true));
        // The other state keeps its own flags.
        assert_eq!(flags(&shell, layer, 1), (true, false));
    }

    #[test]
    fn the_selection_is_copied_onto_the_next_state() {
        let mut shell = shell_with_states(2);
        let layer = shell.editor.active_layer();
        put_rect(&mut shell, layer, 0, 10.0);
        let id = elements(&shell, layer, 0)[0];
        shell.editor.set_selection(vec![id]);
        shell.duplicate_selection_to_frames(0, 0, "next");
        // The copy is a new object on the next state.
        assert_eq!(elements(&shell, layer, 0).len(), 1);
        assert_eq!(elements(&shell, layer, 1).len(), 1);
        assert_ne!(elements(&shell, layer, 1)[0], id);
    }

    #[test]
    fn the_selection_distributes_one_item_per_state() {
        let mut shell = shell_with_states(1);
        let layer = shell.editor.active_layer();
        put_rect(&mut shell, layer, 0, 10.0);
        put_rect(&mut shell, layer, 0, 40.0);
        let ids = elements(&shell, layer, 0);
        shell.editor.set_selection(ids.clone());
        shell.distribute_selection_to_frames(0, 0);
        // Two items, one per state — the page grew to hold them.
        assert_eq!(state_count(&shell), 2);
        assert_eq!(elements(&shell, layer, 0), vec![ids[0]]);
        assert_eq!(elements(&shell, layer, 1), vec![ids[1]]);
    }

    #[test]
    fn the_selection_is_copied_across_a_range_of_states() {
        let mut shell = shell_with_states(3);
        let layer = shell.editor.active_layer();
        put_rect(&mut shell, layer, 0, 10.0);
        let id = elements(&shell, layer, 0)[0];
        shell.editor.set_selection(vec![id]);
        shell.duplicate_selection_to_frame_range(0, 0, 1, 2);
        // The range skipped the source state and covered the other two.
        assert_eq!(elements(&shell, layer, 0).len(), 1);
        assert_eq!(elements(&shell, layer, 1).len(), 1);
        assert_eq!(elements(&shell, layer, 2).len(), 1);
    }
    /// The delays the animation example uses: three states at 10, 25
    /// and 10 hundredths of a second.
    const DELAYS: [i64; 3] = [10, 25, 10];

    /// A clock started on the first of `durations`.
    fn started_clock(durations: &[i64]) -> Playback {
        let mut clock = Playback::default();
        clock.start(0, durations, 0, 7);
        clock
    }

    #[test]
    fn the_clock_waits_for_the_state_delay() {
        let mut clock = started_clock(&DELAYS);
        assert_eq!(tick(&mut clock, 9, &DELAYS), Step::Idle);
        assert_eq!(clock.current(), 0, "the state holds until the delay is up");
        assert_eq!(tick(&mut clock, 1, &DELAYS), Step::Advance(1));
    }

    #[test]
    fn the_clock_advances_on_the_recorded_delays() {
        let mut clock = started_clock(&DELAYS);
        assert_eq!(tick(&mut clock, 10, &DELAYS), Step::Advance(1));
        assert_eq!(tick(&mut clock, 25, &DELAYS), Step::Advance(2));
    }

    #[test]
    fn the_clock_stops_at_the_last_state() {
        let mut clock = started_clock(&DELAYS);
        assert_eq!(tick(&mut clock, 10, &DELAYS), Step::Advance(1));
        assert_eq!(tick(&mut clock, 25, &DELAYS), Step::Advance(2));
        assert_eq!(tick(&mut clock, 10, &DELAYS), Step::Stop);
        assert!(!clock.playing(), "the clock stopped at the last state");
        assert_eq!(clock.current(), 2, "the last state stays on screen");
    }

    #[test]
    fn the_clock_loops_when_asked() {
        let mut clock = started_clock(&DELAYS);
        clock.set_looping(true);
        assert_eq!(tick(&mut clock, 10, &DELAYS), Step::Advance(1));
        assert_eq!(tick(&mut clock, 25, &DELAYS), Step::Advance(2));
        assert_eq!(tick(&mut clock, 10, &DELAYS), Step::Advance(0));
        assert!(clock.playing(), "the loop keeps the clock running");
    }

    #[test]
    fn a_long_frame_carries_its_surplus_into_the_next_state() {
        let mut clock = started_clock(&DELAYS);
        // One frame of 30 cs crosses the first delay and spends 20 cs
        // of the second.
        assert_eq!(tick(&mut clock, 30, &DELAYS), Step::Advance(1));
        // The remaining 5 cs of the second delay are still owed, so
        // the animation is neither early nor late.
        assert_eq!(tick(&mut clock, 4, &DELAYS), Step::Idle);
        assert_eq!(tick(&mut clock, 1, &DELAYS), Step::Advance(2));
    }

    #[test]
    fn a_zero_delay_advances_without_stalling() {
        let mut clock = started_clock(&[0, 10]);
        assert_eq!(tick(&mut clock, 1, &[0, 10]), Step::Advance(1));
        assert!(clock.playing(), "the clock did not stall on the zero delay");
    }

    #[test]
    fn a_stopped_clock_does_not_move() {
        let mut clock = started_clock(&DELAYS);
        clock.stop();
        assert_eq!(tick(&mut clock, 100, &DELAYS), Step::Idle);
        assert_eq!(clock.current(), 0);
    }

    #[test]
    fn the_clock_keeps_the_session_it_started_with() {
        let clock = started_clock(&DELAYS);
        assert!(clock.context_holds(0, 3, 7), "nothing changed");
        assert!(!clock.context_holds(1, 3, 7), "the page changed");
        assert!(!clock.context_holds(0, 4, 7), "a state was added");
        assert!(!clock.context_holds(0, 3, 8), "the document was edited");
    }

    /// A shell over a page whose three states carry the example's
    /// delays.
    fn shell_with_delays() -> Shell {
        let mut shell = shell_with_states(3);
        for (frame, duration) in DELAYS.iter().enumerate() {
            assert!(
                shell.run_command(
                    "frame.setDuration",
                    serde_json::json!({ "page": 0, "frame": frame, "duration": duration }),
                ),
                "the delay must be recorded",
            );
        }
        shell
    }

    /// Starts the clock on the shell's first state.
    fn start(shell: &mut Shell) {
        let strip = TimelineStrip::read(shell.editor.document(), 0).expect("the page is there");
        shell.start_playback(0, 0, &strip);
        assert!(shell.playback.playing(), "the clock started");
    }

    #[test]
    fn playback_moves_the_view_to_the_next_state() {
        let mut shell = shell_with_delays();
        start(&mut shell);
        shell.playback_tick(10);
        assert_eq!(shell.editor.frame, 1, "the view followed the playhead");
        shell.playback_tick(25);
        assert_eq!(shell.editor.frame, 2);
        // The last state ends the playback.
        shell.playback_tick(10);
        assert!(!shell.playback.playing());
        assert_eq!(shell.editor.frame, 2);
    }

    #[test]
    fn playback_stops_when_the_document_is_edited() {
        let mut shell = shell_with_delays();
        start(&mut shell);
        assert!(shell.run_command(
            "frame.setDuration",
            serde_json::json!({ "page": 0, "frame": 0, "duration": 11 }),
        ));
        shell.playback_tick(10);
        assert!(!shell.playback.playing(), "an edit stops the playback");
    }

    #[test]
    fn playback_stops_when_the_state_count_changes() {
        let mut shell = shell_with_delays();
        start(&mut shell);
        assert!(shell.run_command(
            "frame.add",
            serde_json::json!({ "page": 0, "how_many": 1, "location": "end" }),
        ));
        shell.playback_tick(10);
        assert!(!shell.playback.playing(), "a new state stops the playback");
    }

    #[test]
    fn playback_stops_when_the_page_changes() {
        let mut shell = shell_with_delays();
        start(&mut shell);
        assert!(shell.run_command("page.add", serde_json::json!({ "index": null })));
        shell.editor.page = 1;
        shell.playback_tick(10);
        assert!(
            !shell.playback.playing(),
            "a page switch stops the playback"
        );
    }
}
