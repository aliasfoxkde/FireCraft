//! Document lifecycle operations: open, save, revert, import, export,
//! tabs management, script/steps replay, and file info.
//!
//! All methods are `impl Shell` — Rust permits multiple impl blocks
//! across modules. The methods access Shell fields directly.

use super::*;
use crate::editor::Editor;

impl Shell {
    /// Parks the active document as a new tab and swaps in `tab`'s
    /// state. This is the whole switch mechanic: the active fields
    /// (`editor`, `view`, `raster`, …) are canonical, so a switch is
    /// a mutual exchange with the parked slot. The transient
    /// `Editor::untitled()` placeholders exist only between the two
    /// `replace`s and are both overwritten before the call returns.
    pub(super) fn activate_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        let active = DocTab {
            editor: std::mem::replace(&mut self.editor, Editor::untitled()),
            view: self.view,
            raster: self.raster.take(),
            raster_scale: self.raster_scale,
            doc_name: std::mem::take(&mut self.doc_name),
            doc_path: self.doc_path.take(),
        };
        let parked = std::mem::replace(
            &mut self.tabs[index],
            DocTab {
                editor: active.editor,
                view: active.view,
                raster: active.raster,
                raster_scale: active.raster_scale,
                doc_name: active.doc_name,
                doc_path: active.doc_path,
            },
        );
        self.editor = parked.editor;
        self.view = parked.view;
        self.raster = parked.raster;
        self.raster_scale = parked.raster_scale;
        self.doc_name = parked.doc_name;
        self.doc_path = parked.doc_path;
        self.status = format!("switched to {}", self.doc_name);
    }

    /// File ▸ File Info: the document's facts on one line — name,
    /// pages, layers, objects, canvas.
    pub(super) fn file_info_summary(&self) -> String {
        let document = self.editor.session.document();
        let pages = document.pages.len();
        let layers: usize = document
            .pages
            .iter()
            .map(|page| page.layers.layers.len())
            .sum();
        let objects = document.registry.len();
        let canvas = &document.pages[self.editor.page].info.canvas;
        format!(
            "{} — {pages} page(s), {layers} layer(s), {objects} object(s), canvas {}×{}",
            self.doc_name, canvas.width, canvas.height
        )
    }

    /// File ▸ Send to E-mail ▸ Fireworks PNG (format)…: the PNG
    /// format facts of the live document — the canvas the format
    /// stores, its color depth, and the object count it carries.
    pub(super) fn png_format_summary(&self) -> String {
        let document = self.editor.session.document();
        let canvas = &document.pages[self.editor.page].info.canvas;
        format!(
            "Fireworks PNG — {}×{} px, 32-bit RGBA, {} object(s); e-mail delivery is not \
             available in this build",
            canvas.width,
            canvas.height,
            document.registry.len()
        )
    }

    /// File ▸ Open through the injected traits. A Fireworks PNG
    /// opens as a document; any other supported raster opens as a
    /// document sized to the image — one door, no guessing.
    pub(super) fn open_document(&mut self) {
        let Some(path) = self.dialogs.open_path(
            "Open document",
            &[
                ("FireCraft document".to_owned(), vec!["png".to_owned()]),
                (
                    "Images".to_owned(),
                    vec![
                        "png".to_owned(),
                        "jpg".to_owned(),
                        "jpeg".to_owned(),
                        "gif".to_owned(),
                        "bmp".to_owned(),
                        "tif".to_owned(),
                        "tiff".to_owned(),
                        "webp".to_owned(),
                    ],
                ),
            ],
        ) else {
            return;
        };
        let bytes = match self.fs.read_file(&path).map_err(|e| e.to_string()) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("open failed: {error}");
                return;
            }
        };
        match Editor::open(&bytes) {
            Ok(editor) => {
                self.status = format!("opened {path}");
                // Park the previous document rather than dropping it
                // (20.5 multi-document tabs).
                let previous = DocTab {
                    editor: std::mem::replace(&mut self.editor, editor),
                    view: self.view,
                    raster: self.raster.take(),
                    raster_scale: self.raster_scale,
                    doc_name: std::mem::take(&mut self.doc_name),
                    doc_path: self.doc_path.take(),
                };
                self.tabs.push(previous);
                self.raster = None;
                self.raster_scale = 1.0;
                self.view = View::default();
                self.doc_path = Some(path.clone());
                self.doc_name = std::path::Path::new(&path)
                    .file_name()
                    .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
            }
            Err(document_error) => {
                // Not a Fireworks PNG — try a plain raster. A raster
                // opens as its own document, sized to the image, which
                // is what CS6 does with a JPEG.
                let name = std::path::Path::new(&path)
                    .file_name()
                    .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
                // `open_raster_document` overwrites the active
                // editor, so park the previous document first and
                // restore it if the decode fails.
                let previous = DocTab {
                    editor: std::mem::replace(&mut self.editor, Editor::untitled()),
                    view: self.view,
                    raster: self.raster.take(),
                    raster_scale: self.raster_scale,
                    doc_name: std::mem::take(&mut self.doc_name),
                    doc_path: self.doc_path.take(),
                };
                match self.open_raster_document(&bytes, &name) {
                    Ok(()) => {
                        self.tabs.push(previous);
                        self.status = format!("opened {path} as an image");
                        self.raster = None;
                        self.raster_scale = 1.0;
                        self.view = View::default();
                        self.doc_name = name;
                        self.doc_path = Some(path.clone());
                    }
                    Err(_) => {
                        self.editor = previous.editor;
                        self.view = previous.view;
                        self.raster = previous.raster;
                        self.raster_scale = previous.raster_scale;
                        self.doc_name = previous.doc_name;
                        self.doc_path = previous.doc_path;
                        self.status = format!("open failed: {document_error}");
                    }
                }
            }
        }
    }

    /// File ▸ Save through the injected traits.
    /// Help ▸ Manage Extensions / Commands ▸ Run Script: pick a
    /// `.jsf` script or a `.fwsteps` journal and run it against the
    /// live document — the script through the same engine the CLI's
    /// `run-script` verb drives, the steps through the same replay
    /// path the History panel's Replay uses (21.6: one door, both
    /// automation formats). A script that throws or hits a limit, or
    /// a journal that refuses, leaves the document untouched and
    /// reports the failure on the status line.
    pub(super) fn run_script_file(&mut self) {
        let Some(path) = self.dialogs.open_path(
            "Run script",
            &[
                ("FireCraft script".to_owned(), vec!["jsf".to_owned()]),
                ("FireCraft steps".to_owned(), vec!["fwsteps".to_owned()]),
            ],
        ) else {
            return;
        };
        let source = match self.fs.read_file(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) => {
                self.status = format!("script read failed: {error}");
                return;
            }
        };
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
        if path.ends_with(".fwsteps") {
            match self.editor.replay_steps(&source) {
                Ok(()) => {
                    let rows = self.editor.session.journal_len();
                    self.status = format!("ran steps {name} ({rows} journal row(s))");
                    self.raster = None;
                    self.raster_scale = 1.0;
                }
                Err(error) => self.status = format!("steps {name} failed: {error}"),
            }
            return;
        }
        #[cfg(feature = "scripting")]
        {
            let mut engine = match scripting::ScriptEngine::new(scripting::Limits::default()) {
                Ok(engine) => engine,
                Err(error) => {
                    self.status = format!("script engine: {error}");
                    return;
                }
            };
            // The run needs an `Rc`; the shell keeps its own — cloning is
            // all it takes, no swap, and the shell's handle survives the
            // script machinery's clones.
            let fs_for_run = std::rc::Rc::clone(&self.fs);
            let report = scripting::run_with_file_system(fs_for_run, || {
                scripting::run_in_session(&mut self.editor.session, &mut engine, &name, &source)
            });
            if report.completed() {
                self.status = format!("ran {name} ({} host call(s))", report.host_calls);
                self.raster = None;
                self.raster_scale = 1.0;
            } else {
                self.status = format!(
                    "script {name} failed: {}",
                    report.message.as_deref().unwrap_or("unknown error")
                );
            }
        }
    }

    /// The Commands/Help extension surface. Today the manager *is*
    /// the script runner: pick and execute a `.jsf`. CS6's broader
    /// extension manager (install/uninstall packages) is recorded as
    /// a gap, not silently absent.
    pub(super) fn open_extension_manager(&mut self) {
        self.run_script_file();
    }

    /// File ▸ Preview in Browser: export the document to PNG file(s)
    /// and open them in the system browser. Saved documents write next
    /// to the document file; unsaved ones use a temp directory. The
    /// secondary-browser row opens the same default handler — CS6's
    /// per-browser configuration is not kept in this build (the Set
    /// Browser rows say so).
    pub(super) fn preview_in_browser(&mut self, secondary: bool, all_pages: bool) {
        let _ = secondary;
        let dialog = ExportDialog::default();
        let document = self.editor.session.document().clone();
        let page_count = if all_pages { document.pages.len() } else { 1 };
        let base = self.doc_path.as_ref().map_or_else(
            || std::env::temp_dir().join("firecraft-preview"),
            |path| {
                let mut stem = std::path::PathBuf::from(path);
                stem.set_extension("");
                stem
            },
        );
        let mut opened = 0usize;
        let mut failed = false;
        for page in 0..page_count {
            let mut export_document = document.clone();
            if let Some(target) = export_document.pages.get_mut(page) {
                target.export = Some(Self::dialog_options(&dialog));
            }
            let name = if page_count == 1 {
                format!("{}.png", base.display())
            } else {
                format!("{}-page{}.png", base.display(), page + 1)
            };
            let flags = Self::dialog_flags(&dialog);
            match app_cli::ops::export_raster(&export_document, &name, &flags) {
                Ok(bytes) => {
                    if std::fs::write(&name, &bytes).is_ok()
                        && self.dialogs.open_url(&format!("file://{name}"))
                    {
                        opened += 1;
                    } else {
                        failed = true;
                    }
                }
                Err(error) => {
                    self.status = format!("preview export failed: {error}");
                    return;
                }
            }
        }
        self.status = if failed {
            format!("exported {opened} preview(s); the browser could not be opened")
        } else {
            format!("exported and opened {opened} preview(s) in the browser")
        };
    }

    /// File ▸ Revert: reload the document from the file it was last
    /// saved to, discarding the in-memory edits.
    pub(super) fn revert_document(&mut self) {
        let Some(path) = self.doc_path.clone() else {
            self.status = "revert needs a document saved to a file".to_owned();
            return;
        };
        // CS6 prompts before discarding unsaved edits; refuse rather
        // than silently dropping work.
        if self.editor.session.is_dirty() {
            self.status = "document has unsaved changes — save before reverting".to_owned();
            return;
        }
        let bytes = match self.fs.read_file(&path).map_err(|error| error.to_string()) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("revert failed: {error}");
                return;
            }
        };
        match Editor::open(&bytes) {
            Ok(editor) => {
                self.editor = editor;
                self.view = View::default();
                self.raster = None;
                self.raster_scale = 1.0;
                self.status = format!("reverted to {path}");
            }
            Err(error) => self.status = format!("revert failed: {error}"),
        }
    }

    pub(super) fn save_document(&mut self) {
        let Some(path) = self.dialogs.save_path(
            "Save document",
            "untitled.fw.png",
            &[("FireCraft document".to_owned(), vec!["png".to_owned()])],
        ) else {
            return;
        };
        match self.editor.save() {
            Ok(bytes) => match self.fs.write_file(&path, &bytes) {
                Ok(()) => {
                    self.editor.session.mark_clean();
                    #[cfg(not(target_arch = "wasm32"))]
                    self.clear_recovery();
                    self.doc_path = Some(path.clone());
                    self.doc_name = std::path::Path::new(&path)
                        .file_name()
                        .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
                    self.status = format!("saved {path} ({} bytes)", bytes.len());
                }
                Err(error) => self.status = format!("save failed: {error}"),
            },
            Err(error) => self.status = format!("save failed: {error}"),
        }
    }

    /// Applies through the editor and reports failures on the status
    /// line — the one idiom every menu/panel action shares. Returns
    /// whether the command was accepted.
    pub(super) fn run_command(&mut self, name: &str, payload: serde_json::Value) -> bool {
        match self.editor.apply(name, payload) {
            Ok(()) => true,
            Err(error) => {
                self.status = format!("{name} failed: {error}");
                false
            }
        }
    }

    /// History → Save Steps…: the session journal written as a
    /// `.fwsteps` command file (JSON lines) through the injected
    /// traits. The file replays through [`Session::reopen`] — the same
    /// path the CLI's `apply` verb drives — so a saved command is not
    /// an export but the session's own restart record.
    pub(super) fn save_steps(&mut self) {
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
    pub(super) fn replay_steps(&mut self) {
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
        let doc_filters = [("FireCraft document".to_owned(), vec!["png".to_owned()])];
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
                self.raster_scale = 1.0;
                self.status = format!("replayed {steps_path} onto {doc_path} ({rows} rows)");
            }
            Err(error) => self.status = format!("replay failed: {error}"),
        }
    }
}
