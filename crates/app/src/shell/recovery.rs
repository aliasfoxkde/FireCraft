//! Crash-recovery sidecar management (desktop only).
//!
//! Periodic journal snapshots to a temp file so a crash doesn't lose
//! the user's unsaved work. The desktop shell checks for a leftover
//! sidecar at startup and offers to replay it.

use super::*;

impl Shell {
    /// The default recovery sidecar path: `FIRECRAFT_RECOVERY` when
    /// set, else the platform temp dir. Called once, at construction.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn default_recovery_path() -> std::path::PathBuf {
        if let Ok(path) = std::env::var("FIRECRAFT_RECOVERY") {
            return std::path::PathBuf::from(path);
        }
        std::env::temp_dir().join("firecraft-autorecover.fwsteps")
    }

    /// The recovery sidecar path as a `&str` for the injected
    /// [`engine::platform::FileSystem`].
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn recovery_path_str(&self) -> String {
        self.recovery_path.to_string_lossy().into_owned()
    }

    /// Periodic journal snapshot to the recovery sidecar (desktop;
    /// the web build persists the whole document through the browser
    /// instead). Debounced by the same change stamp the web persist
    /// uses: a write happens only when the session actually moved.
    /// An empty journal clears the sidecar — an untitled document's
    /// first steps must not resurrect a discarded recovery file, and
    /// a cleared journal must not outlive its session.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn recovery_tick(&mut self) {
        let stamp = self.stamp();
        if self.persisted == Some(stamp) {
            return;
        }
        let path = self.recovery_path_str();
        let outcome = if self.editor.session.journal_len() == 0 {
            if self.fs.exists(&path) {
                self.fs.remove_file(&path)
            } else {
                Ok(())
            }
        } else {
            self.fs
                .write_file(&path, self.editor.steps_text().as_bytes())
        };
        if outcome.is_ok() {
            self.persisted = Some(stamp);
        }
    }

    /// Removes the recovery sidecar after a clean save or File ▸ New:
    /// the document now has a durable home, so a crash-recovery
    /// replay would only resurrect stale steps.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn clear_recovery(&mut self) {
        let path = self.recovery_path_str();
        if self.fs.exists(&path) {
            let _ = self.fs.remove_file(&path);
        }
        self.persisted = None;
    }

    /// Checks for a leftover recovery journal and, when found, arms
    /// the restore-offer window. Called from `Shell::new` only — a
    /// mid-session appearance would mean this very session crashed,
    /// which it cannot have.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn check_recovery(&mut self) {
        let path = self.recovery_path_str();
        if self.fs.exists(&path) {
            self.recover_open = true;
        }
    }

    /// Replays the recovery journal onto the fresh untitled session
    /// (the crash-recovery path of `Editor::replay_steps`: history is
    /// reconstructed, so undo works across the crash boundary).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn recover_steps(&mut self) -> Result<usize, String> {
        let path = self.recovery_path_str();
        let text = self
            .fs
            .read_file(&path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| String::from_utf8(bytes).map_err(|error| error.to_string()))?;
        self.editor
            .replay_steps(&text)
            .map_err(|error| error.to_string())?;
        let rows = self.editor.session.journal_len();
        self.clear_recovery();
        Ok(rows)
    }

    /// The recovery-offer window: what was found, Recover (replay
    /// onto this session) or Discard (delete the sidecar).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn recovery_ui(&mut self, ui: &mut egui::Ui) {
        if !self.recover_open {
            return;
        }
        egui::Window::new("Recover unsaved work?")
            .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                ui.label(format!(
                    "A journal snapshot from a previous session exists:\n{}",
                    self.recovery_path.display()
                ));
                ui.label("Recover replays its steps onto this session; undo history comes back with them.");
                ui.horizontal(|ui| {
                    if ui.button("Recover").clicked() {
                        match self.recover_steps() {
                            Ok(rows) => {
                                self.status = format!("recovered {rows} step(s) from the snapshot");
                                self.view = View::default();
                                self.raster = None;
                                self.raster_scale = 1.0;
                            }
                            Err(error) => self.status = format!("recovery failed: {error}"),
                        }
                        self.recover_open = false;
                    }
                    if ui.button("Discard").clicked() {
                        self.clear_recovery();
                        self.recover_open = false;
                        self.status = "recovery snapshot discarded".to_owned();
                    }
                });
            });
    }
}
