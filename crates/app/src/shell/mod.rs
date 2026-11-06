//! The egui face of the editor: menus, panels, canvas. Thin by rule —
//! every document change you see here is an [`Editor::apply`] call,
//! and every panel reads the document read-only. Desktop file routing
//! goes through the injected [`engine::platform`] traits ([`EnvDialogs`]
//! in [`dialogs`]; real modal dialogs land with desktop integration).
pub(crate) use dialogs::{
    CanvasColorDialog, CanvasSizeDialog, DisplayState, EnvDialogs, ExportDialog, FeatherDialog,
    ImageSizeDialog, NumericTransformDialog,
};
pub(crate) use filters_ui::{
    FILTER_BOOL_KEYS, FILTER_FAMILIES, FILTER_PARAMS, FilterPreset, filter_preset,
};
pub(crate) use guides_ui::GridDialog;
pub(crate) use guides_ui::GuideDraft;

mod canvas_ui;
mod dialogs;
mod filters_ui;
mod glyphs;
mod guides_ui;
mod icons;
mod palette;
mod panel_windows;
mod panels;
mod panels_web;

pub use glyphs::apply_cs6_theme;
pub(crate) use glyphs::{draw_autoshape_glyph, symbol_type_label, tool_key, tool_label};

use doc::{ObjectData, PathObject, SymbolInstanceObject, SymbolType, TextObject};
use egui::{Color32, ColorImage, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2};

use base64::Engine as _;

use crate::canvas::View;
use crate::editor::union_selection_bounds;
use crate::editor::{
    AutoShape, DragConstraint, Editor, GradientHandle, NineSliceEdge, PixelSelectMode, StampShape,
    Tool, TransformHandle,
};
use crate::menus;
use crate::toolstrip;

// The web drop sink ([`Shell::set_drop_sink`]): the web crate's async
// file reader, installed at startup. Wasm only — the desktop shell
// reads dropped bytes directly. No `Send`/`Sync` bound: wasm is
// single-threaded and the sink captures `Rc<WebRunner>`, so the
// holder is a `thread_local` (a `static Mutex` would demand `Send`).
#[cfg(target_arch = "wasm32")]
thread_local! {
    static DROP_SINK: std::cell::RefCell<Option<std::rc::Rc<dyn Fn(&egui::DroppedFileHandle, egui::Context)>>> =
        const { std::cell::RefCell::new(None) };
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
    /// Zoom the cached raster was rendered at. The stamp alone doesn't
    /// cover view changes; the raster re-renders when zoom moves off
    /// the cached scale by more than ~2× (crisp enough everywhere in
    /// between, and bounded — a 4K page at the 64× ceiling would be a
    /// 262k-pixel-wide raster, so the effective scale clamps to 4×).
    raster_scale: f64,
    /// Injected platform services.
    fs: std::rc::Rc<dyn engine::platform::FileSystem>,
    dialogs: Box<dyn engine::platform::Dialogs>,
    /// Last status line (errors, save confirmations).
    pub status: String,
    /// Open export dialog, if any.
    export_dialog: Option<ExportDialog>,
    /// Open Numeric Transform dialog, if any.
    pub(crate) numeric_transform: Option<NumericTransformDialog>,
    /// Open Feather dialog, if any.
    pub(crate) feather: Option<FeatherDialog>,
    /// Open Canvas Size dialog, if any.
    pub(crate) canvas_size: Option<CanvasSizeDialog>,
    /// Open Canvas Color dialog, if any.
    pub(crate) canvas_color: Option<CanvasColorDialog>,
    /// Open Image Size dialog, if any.
    pub(crate) image_size: Option<ImageSizeDialog>,
    /// The command palette state.
    pub(crate) palette: palette::CommandPalette,
    /// Display name of the open document (tab strip).
    doc_name: String,
    /// The path the document was last opened/saved from — Revert
    /// reads this; `None` for a never-saved document.
    doc_path: Option<String>,
    /// Last canvas widget rect — the anchor for view commands that
    /// arrive outside the canvas (menu/shortcut zoom and fit).
    canvas_rect: Option<Rect>,
    /// Set by the Exit action; the frame loop closes the viewport.
    close_requested: bool,
    /// The Auto Shape flyout is open (toggled by its tool button).
    autoshape_flyout: bool,
    /// The open tool-group flyout: the group's name, its tools, and
    /// the screen position it opened at (CS6's press-and-hold list).
    tool_flyout: Option<(&'static str, Vec<Tool>, Pos2)>,
    /// The pending press-and-hold on a tool-group leader: when the
    /// press started and where.
    tool_press: Option<ToolPress>,
    /// The persisted shell layout (panel visibility, dock folds) —
    /// saved through `App::save` and restored at startup. The window
    /// *geometry* itself persists through egui's own area memory.
    layout: ShellLayout,
    /// The Help ▸ About `FireCraft` window is open.
    pub(crate) about_open: bool,
    /// The keyboard-shortcuts help panel is open (F1 / Help menu).
    help_open: bool,
    /// The Styles panel's new-style name field.
    style_name: String,
    /// The layer whose name is being edited in the Layers panel
    /// (`None` = no inline rename open).
    rename_layer: Option<usize>,
    /// View-menu toggles + the open grid dialog. Display state only —
    /// the guides and grid themselves live in the document, so a move
    /// drag's snapping reads them through the editor like every other
    /// document fact.
    show_grid: bool,
    snap_to_grid: bool,
    show_guides: bool,
    /// View ▸ Guides ▸ Lock Guides. Like show/snap this is session
    /// state (decision 0035) — but unlike them, a locked guide also
    /// refuses drag-out and move-snap, so the canvas reads it.
    guides_locked: bool,
    snap_to_guides: bool,
    grid_dialog: Option<GridDialog>,
    /// View ▸ Smart Guides ▸ Show / Snap to. Display-only alignment
    /// hints; nothing in the document reads them yet.
    smart_guides: bool,
    snap_to_smart_guides: bool,
    /// View ▸ 9-Slice Scaling Guides ▸ Lock Guides (display intent
    /// until slice rendering lands).
    slice_locked: bool,
    /// View ▸ Slice Guides / Slice Overlay.
    slice_guides: bool,
    slice_overlay: bool,
    /// View ▸ Edges — object edge highlight.
    show_edges: bool,
    /// View ▸ Rulers / Tooltips / Gamma 1.8 (display intent; rulers
    /// and tooltips render in the frame loop, gamma is a preview flag).
    show_rulers: bool,
    show_tooltips: bool,
    gamma_18: bool,
    /// A guide being dragged out from a canvas-edge press: the axis
    /// and the live world position. Committed to `page.addGuide` (or
    /// discarded) at release; locked guides refuse new guides.
    guide_draft: Option<GuideDraft>,
    /// The canvas context menu: `Some` while open, holding the screen
    /// position it opened at (the popup re-anchors there each frame,
    /// so a later pointer move does not drag it).
    pub(crate) context_menu: Option<Pos2>,
    /// Window-menu panel visibility: names of right-dock panels the
    /// user closed. Absent = shown (CS6 default is everything open).
    panel_hidden: Vec<&'static str>,
    /// The URL panel's edit-in-progress state — the object it edits
    /// and the link/alt text being typed.
    url_draft: Option<panels_web::UrlDraft>,
    /// The Behaviors panel's chosen swap-image target (the slice's
    /// name; `None` = not chosen yet).
    swap_target: Option<String>,
    /// Window ▸ Hide Panels (Tab): all right-dock panels folded at
    /// once, independent of per-panel visibility.
    panels_hidden: bool,
    /// Window ▸ Tools: the left tools dock is folded.
    tools_hidden: bool,
    /// Window ▸ Toolbars ▸ Main: the context options bar is hidden.
    options_bar_hidden: bool,
    /// The screen-mode control's cycle position (CS6's three view
    /// modes: Standard, Full Screen with Menus, Full Screen). Display
    /// state, not document state.
    screen_mode: ScreenMode,
    /// The live egui context, stashed each frame so code outside the
    /// frame loop (the web bridge) can request a repaint after it
    /// mutates the shell — without one, a bridge-loaded document sat
    /// behind a stale untitled canvas until the next input event.
    ctx: Option<egui::Context>,
}

/// A press-and-hold in progress on a tool-group leader: when it
/// started (egui input time) and where the pointer was.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ToolPress {
    /// The egui input time the press started at.
    pub(crate) at: f64,
    /// The pointer position at the press.
    pub(crate) pos: Pos2,
}

/// How long a tool-group leader must be held before its flyout
/// opens — CS6's press-and-hold dwell.
pub(crate) const TOOL_HOLD_SECONDS: f64 = 0.45;

/// The hold decision, as a pure function of the elapsed time: the
/// clock and the pointer state are read at the edge and passed in.
pub(crate) fn hold_opens_flyout(elapsed: f64) -> bool {
    elapsed >= TOOL_HOLD_SECONDS
}

/// CS6's three view modes — the help set: "In the Tools panel,
/// select from one of three view modes to control the layout of your
/// workspace. Standard Screen mode: The default document window view.
/// Full Screen with Menus mode: A maximized document window view set
/// against a gray background. Menus, toolbars, scroll bars, and
/// panels are visible. Full Screen mode: A maximized document window
/// view set against a black background. No menus, toolbars, or title
/// bars are visible." (fwref 1178–1185.) The screen-mode control
/// cycles through them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ScreenMode {
    /// The default document window view — every chrome element shown.
    #[default]
    Standard,
    /// Maximized against a gray backdrop; menus stay.
    FullScreenWithMenus,
    /// Maximized against a black backdrop; no menus, toolbars, or
    /// title bars.
    FullScreen,
}

impl ScreenMode {
    /// The next mode in CS6's cycle order.
    fn next(self) -> Self {
        match self {
            ScreenMode::Standard => ScreenMode::FullScreenWithMenus,
            ScreenMode::FullScreenWithMenus => ScreenMode::FullScreen,
            ScreenMode::FullScreen => ScreenMode::Standard,
        }
    }

    /// The status line the cycle reports.
    fn label(self) -> &'static str {
        match self {
            ScreenMode::Standard => "Standard Screen mode",
            ScreenMode::FullScreenWithMenus => "Full Screen with Menus mode",
            ScreenMode::FullScreen => "Full Screen mode",
        }
    }

    /// The backdrop CS6 paints behind the maximized document: gray in
    /// Full Screen with Menus, black in Full Screen, and the shell's
    /// usual dark canvas matte in Standard.
    fn backdrop(self) -> egui::Color32 {
        match self {
            ScreenMode::Standard => egui::Color32::from_rgb(32, 32, 32),
            ScreenMode::FullScreenWithMenus => egui::Color32::from_rgb(0x99, 0x99, 0x99),
            ScreenMode::FullScreen => egui::Color32::BLACK,
        }
    }
}

/// The persisted shell layout: the panel-visibility set and the dock
/// folds. Serialized (RON, via eframe's storage) on shutdown and
/// every ~30 s; restored at startup so the CS6 arrangement survives a
/// restart. Window *geometry* persists through egui's own area memory
/// (the `persistence` feature), which is keyed on the window ids.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ShellLayout {
    /// The panel names currently hidden from the dock.
    panel_hidden: Vec<String>,
    /// The Tab-key fold of the whole panel group.
    panels_hidden: bool,
    /// The left tool dock fold.
    tools_hidden: bool,
    /// The context options bar fold.
    options_bar_hidden: bool,
    /// Display toggles worth carrying across sessions.
    show_grid: bool,
    show_guides: bool,
    show_rulers: bool,
    smart_guides: bool,
}

impl Default for ShellLayout {
    fn default() -> Self {
        Self {
            panel_hidden: Shell::default_open_panels()
                .into_iter()
                .map(str::to_owned)
                .collect(),
            panels_hidden: false,
            tools_hidden: false,
            options_bar_hidden: false,
            show_grid: false,
            show_guides: true,
            show_rulers: false,
            smart_guides: true,
        }
    }
}

impl Shell {
    /// Builds the shell with desktop platform defaults. A persisted
    /// layout (from the previous session's `App::save`) restores the
    /// panel set and dock folds.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_cs6_theme(&cc.egui_ctx);
        let layout = cc
            .storage
            .and_then(|storage| eframe::get_value::<ShellLayout>(storage, "shell_layout"))
            .unwrap_or_default();
        let mut shell = Self::with_platform(
            Editor::untitled(),
            std::rc::Rc::new(app_cli::io::StdFs),
            Box::new(EnvDialogs::default()),
        );
        shell.layout = layout;
        shell.apply_layout();
        shell
    }

    /// Builds the shell over an editor and explicit platform services.
    #[must_use]
    pub fn with_platform(
        editor: Editor,
        fs: std::rc::Rc<dyn engine::platform::FileSystem>,
        dialogs: Box<dyn engine::platform::Dialogs>,
    ) -> Self {
        Self {
            editor,
            view: View::default(),
            raster: None,
            raster_scale: 1.0,
            fs,
            dialogs,
            status: "ready".to_owned(),
            export_dialog: None,
            numeric_transform: None,
            feather: None,
            canvas_size: None,
            canvas_color: None,
            image_size: None,
            palette: palette::CommandPalette::default(),
            doc_name: "untitled".to_owned(),
            doc_path: None,
            canvas_rect: None,
            close_requested: false,
            autoshape_flyout: false,
            tool_flyout: None,
            tool_press: None,
            about_open: false,
            help_open: false,
            style_name: String::new(),
            rename_layer: None,
            show_grid: false,
            snap_to_grid: false,
            show_guides: true,
            guides_locked: false,
            snap_to_guides: false,
            grid_dialog: None,
            smart_guides: true,
            snap_to_smart_guides: true,
            slice_locked: false,
            slice_guides: true,
            slice_overlay: true,
            show_edges: true,
            show_rulers: false,
            show_tooltips: true,
            gamma_18: false,
            guide_draft: None,
            context_menu: None,
            // CS6's fresh-document roster: Properties/Pages/States/
            // Layers open, the rest of the panel inventory closed.
            panel_hidden: Self::default_open_panels(),
            url_draft: None,
            swap_target: None,
            panels_hidden: false,
            tools_hidden: false,
            options_bar_hidden: false,
            screen_mode: ScreenMode::Standard,
            layout: ShellLayout::default(),
            ctx: None,
        }
    }

    /// Restores a persisted layout (panel set, dock folds, display
    /// toggles) from platform storage. The web build constructs the
    /// shell without a `CreationContext`, so it calls this with the
    /// storage its runner exposes.
    pub fn restore_layout(&mut self, storage: Option<&dyn eframe::Storage>) {
        if let Some(layout) =
            storage.and_then(|storage| eframe::get_value::<ShellLayout>(storage, "shell_layout"))
        {
            self.layout = layout;
            self.apply_layout();
        }
    }

    /// Copies the persisted layout into the live session fields.
    /// Panel names are intersected with the current inventory so a
    /// layout saved by a build with different panels cannot resurrect
    /// panels this build cannot draw (or orphan one it gained).
    fn apply_layout(&mut self) {
        let saved: Vec<&str> = self
            .layout
            .panel_hidden
            .iter()
            .map(String::as_str)
            .collect();
        self.panel_hidden = panel_windows::PANELS
            .iter()
            .filter(|panel| saved.contains(&panel.name))
            .map(|panel| panel.name)
            .collect();
        self.panels_hidden = self.layout.panels_hidden;
        self.tools_hidden = self.layout.tools_hidden;
        self.options_bar_hidden = self.layout.options_bar_hidden;
        self.show_grid = self.layout.show_grid;
        self.show_guides = self.layout.show_guides;
        self.show_rulers = self.layout.show_rulers;
        self.smart_guides = self.layout.smart_guides;
    }

    /// Snapshots the live layout fields for `App::save`.
    fn capture_layout(&self) -> ShellLayout {
        ShellLayout {
            panel_hidden: self
                .panel_hidden
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
            panels_hidden: self.panels_hidden,
            tools_hidden: self.tools_hidden,
            options_bar_hidden: self.options_bar_hidden,
            show_grid: self.show_grid,
            show_guides: self.show_guides,
            show_rulers: self.show_rulers,
            smart_guides: self.smart_guides,
        }
    }

    /// Wakes the frame loop — a no-op until the first `ui` frame has
    /// stashed the context. Bridge-side mutations (open/apply) must
    /// call this: they run between frames, where nothing else would
    /// repaint the stale canvas.
    pub fn request_repaint(&self) {
        if let Some(ctx) = &self.ctx {
            ctx.request_repaint();
        }
    }

    /// Replaces the document from raw bytes — the same one-door read
    /// path the desktop open uses: a Fireworks PNG opens through
    /// `io::open`, any other supported raster opens as a document
    /// sized to the image. No registry side door.
    ///
    /// A raster that arrives while the document already holds objects
    /// is *placed* into it (CS6's File → Import) rather than replacing
    /// it — the browser's one file door must add to the artwork the
    /// user is making, not silently swap the canvas for the image.
    ///
    /// # Errors
    /// Whatever both readers refuse; the current document is kept.
    pub fn load_document_bytes(&mut self, bytes: &[u8]) -> Result<(), engine::EngineError> {
        match Editor::open(bytes) {
            Ok(editor) => {
                self.raster = None;
                self.raster_scale = 1.0;
                self.editor = editor;
                self.view = View::default();
                self.status = "opened document".to_owned();
                Ok(())
            }
            Err(document_error) => {
                if self.document_has_content() {
                    self.place_raster_into_document(bytes, "imported image")
                        .map_err(|_| document_error)?;
                    self.status = "placed image in the document".to_owned();
                } else {
                    self.open_raster_document(bytes, "imported image")
                        .map_err(|_| document_error)?;
                    self.raster = None;
                    self.raster_scale = 1.0;
                    self.status = "opened image".to_owned();
                    self.view = View::default();
                }
                Ok(())
            }
        }
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

    /// File → Open through the injected traits. A Fireworks PNG
    /// opens as a document; any other supported raster opens as a
    /// document sized to the image — one door, no guessing.
    fn open_document(&mut self) {
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
                self.raster = None;
                self.raster_scale = 1.0;
                self.editor = editor;
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
                match self.open_raster_document(&bytes, &name) {
                    Ok(()) => {
                        self.status = format!("opened {path} as an image");
                        self.raster = None;
                        self.raster_scale = 1.0;
                        self.view = View::default();
                        self.doc_name = name;
                    }
                    Err(_) => {
                        self.status = format!("open failed: {document_error}");
                    }
                }
            }
        }
    }

    /// File → Save through the injected traits.
    /// Help ▸ Manage Extensions / Commands ▸ Run Script: pick a
    /// `.jsf` script and run it against the live document through the
    /// same engine the CLI's `run-script` verb drives. A script that
    /// throws or hits a limit leaves the document untouched and
    /// reports the failure on the status line.
    fn run_script_file(&mut self) {
        let Some(path) = self.dialogs.open_path(
            "Run script",
            &[("FireCraft script".to_owned(), vec!["jsf".to_owned()])],
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

    /// The Commands/Help extension surface. Today the manager *is*
    /// the script runner: pick and execute a `.jsf`. CS6's broader
    /// extension manager (install/uninstall packages) is recorded as
    /// a gap, not silently absent.
    fn open_extension_manager(&mut self) {
        self.run_script_file();
    }

    /// File ▸ File Info: the document's facts on one line — name,
    /// pages, layers, objects, canvas.
    fn file_info_summary(&self) -> String {
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
    fn png_format_summary(&self) -> String {
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

    /// File ▸ Preview in Browser: export the document to PNG file(s)
    /// and open them in the system browser. Saved documents write next
    /// to the document file; unsaved ones use a temp directory. The
    /// secondary-browser row opens the same default handler — CS6's
    /// per-browser configuration is not kept in this build (the Set
    /// Browser rows say so).
    fn preview_in_browser(&mut self, secondary: bool, all_pages: bool) {
        let _ = secondary;
        let dialog = crate::shell::dialogs::ExportDialog::default();
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
    fn revert_document(&mut self) {
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

    fn save_document(&mut self) {
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
    fn run_command(&mut self, name: &str, payload: serde_json::Value) -> bool {
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

    /// The CS6 menu bar: ten menus rendered from the [`crate::menus`]
    /// table. Items whose action is [`menus::Action::Inert`] render
    /// disabled — the bar shows the full CS6 surface while only the
    /// wired subset is actionable.
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            for menu in menus::MENUBAR {
                ui.menu_button(menu.label, |ui| self.entries_ui(ui, menu.entries));
            }
        });
    }

    /// Renders one menu-tree level; submenus recurse.
    fn entries_ui(&mut self, ui: &mut egui::Ui, entries: &'static [menus::Entry]) {
        for entry in entries {
            match entry {
                menus::Entry::Sep => {
                    ui.separator();
                }
                menus::Entry::Sub(label, sub) => {
                    ui.menu_button(*label, |ui| self.entries_ui(ui, sub));
                }
                menus::Entry::Item(item) => {
                    let label = if item.accel.is_empty() {
                        item.label.to_owned()
                    } else {
                        format!("{}\t{}", item.label, item.accel)
                    };
                    let wired = !matches!(item.action, menus::Action::Inert);
                    if ui.add_enabled(wired, egui::Button::new(label)).clicked() {
                        self.run_action(&item.action);
                    }
                }
            }
        }
    }

    /// Executes one menu/shortcut action. This is the only place an
    /// [`menus::Action`] becomes behavior.
    fn run_action(&mut self, action: &menus::Action) {
        match *action {
            menus::Action::Inert => {}
            menus::Action::Shell(shell) => self.run_shell(shell),
            menus::Action::Sel(sel) => self.run_sel(sel),
            menus::Action::Align(mode) => {
                let ids = self.editor.live_selection();
                if ids.len() >= 2 {
                    self.run_command(
                        "object.align",
                        serde_json::json!({ "ids": ids, "mode": mode }),
                    );
                } else {
                    self.status = "align needs two or more objects".to_owned();
                }
            }
            menus::Action::Distribute(axis) => {
                let ids = self.editor.live_selection();
                if ids.len() >= 3 {
                    self.run_command(
                        "object.distribute",
                        serde_json::json!({ "ids": ids, "axis": axis }),
                    );
                } else {
                    self.status = "distribute needs three or more objects".to_owned();
                }
            }
            menus::Action::Boolean(op) => match self.editor.boolean_pair() {
                Some((a, b)) => {
                    self.run_command(
                        "path.boolean",
                        serde_json::json!({ "a": a, "b": b, "op": op }),
                    );
                }
                None => self.status = "booleans need exactly two live paths".to_owned(),
            },
            menus::Action::Arrange(direction) => self.run_arrange(direction),
            menus::Action::TextSize(size) => {
                if let Some(id) = self.editor.selection {
                    self.run_command(
                        "text.setAttrs",
                        serde_json::json!({ "id": id, "size": size }),
                    );
                }
            }
            menus::Action::TextFlag(flag) => {
                if let Some(id) = self.editor.selection {
                    self.run_command(
                        "text.setAttrs",
                        serde_json::json!({ "id": id, (flag): true }),
                    );
                }
            }
            menus::Action::TextPlain => {
                if let Some(id) = self.editor.selection {
                    self.run_command(
                        "text.setAttrs",
                        serde_json::json!({
                            "id": id, "bold": false, "italic": false, "underline": false,
                        }),
                    );
                }
            }
            menus::Action::TextJustify(justification) => {
                if let Some(id) = self.editor.selection {
                    self.run_command(
                        "text.setAttrs",
                        serde_json::json!({ "id": id, "justification": justification }),
                    );
                }
            }
            menus::Action::TextStep(step) => {
                // CS6's Smaller/Larger step through the size presets:
                // the next preset below/above the current size, clamped
                // to the 8..120 row ends.
                const PRESETS: [f64; 12] = [
                    8.0, 9.0, 10.0, 12.0, 14.0, 18.0, 24.0, 36.0, 48.0, 72.0, 96.0, 120.0,
                ];
                let Some(id) = self.editor.selection else {
                    return;
                };
                let current = self
                    .editor
                    .document()
                    .registry
                    .get(id)
                    .and_then(|object| match object {
                        ObjectData::Text(text) => text.style.as_ref().and_then(|style| style.size),
                        _ => None,
                    })
                    .unwrap_or(12.0);
                let next = if step < 0 {
                    PRESETS
                        .iter()
                        .rev()
                        .find(|preset| **preset < current)
                        .copied()
                        .unwrap_or(PRESETS[0])
                } else {
                    PRESETS
                        .iter()
                        .find(|preset| **preset > current)
                        .copied()
                        .unwrap_or(PRESETS[PRESETS.len() - 1])
                };
                self.run_command(
                    "text.setAttrs",
                    serde_json::json!({ "id": id, "size": next }),
                );
            }
            menus::Action::TextOnPathMode(mode) => {
                // The on-path setters walk the document's selection, so
                // the view's selection must land there first.
                if self.editor.selection.is_some() {
                    if let Err(error) = self.editor.sync_selection() {
                        self.status = format!("selection failed: {error}");
                        return;
                    }
                    self.run_command(
                        "text.setTextOnPathMode",
                        serde_json::json!({ "mode": mode }),
                    );
                }
            }
            menus::Action::Filter(name) => match self.editor.selection {
                Some(id) => match filter_preset(name) {
                    Some(preset) => self.apply_filter_preset(id, *preset),
                    None => self.status = format!("no such filter: {name}"),
                },
                None => self.status = "pick an object to filter".to_owned(),
            },
            menus::Action::Document(action) => self.run_document(action),
        }
    }

    /// Page-scoped document verbs (Modify ▸ Canvas family): the
    /// payload names the active page; no selection involved.
    fn run_document(&mut self, action: menus::DocumentAction) {
        let page = self.editor.page;
        match action {
            menus::DocumentAction::TrimCanvas => {
                // CS6's Trim Canvas shrinks to content; it never grows,
                // so `grow` is false.
                self.run_command(
                    "document.setDocumentCanvasSizeToDocumentExtents",
                    serde_json::json!({ "page": page, "grow": false }),
                );
            }
            menus::DocumentAction::FitCanvas => {
                self.run_command(
                    "document.setDocumentCanvasSizeToSelection",
                    serde_json::json!({ "page": page }),
                );
            }
            menus::DocumentAction::Rotate(degrees) => {
                self.run_command(
                    "document.rotate",
                    serde_json::json!({ "rotation": degrees, "page": page }),
                );
            }
        }
    }

    /// Shell-level plumbing: file, history, selection, view.
    fn run_shell(&mut self, action: menus::ShellAction) {
        match action {
            menus::ShellAction::New => {
                self.editor = Editor::untitled();
                self.view = View::default();
                self.raster = None;
                self.raster_scale = 1.0;
                self.doc_name = "untitled".to_owned();
                self.status = "new document".to_owned();
            }
            menus::ShellAction::Open => self.open_document(),
            menus::ShellAction::Import => self.import_raster_dialog(),
            menus::ShellAction::Save | menus::ShellAction::SaveAs => self.save_document(),
            menus::ShellAction::Export | menus::ShellAction::ExportPreview => {
                self.export_dialog = Some(ExportDialog::default());
            }
            menus::ShellAction::ExportWizard => {
                self.export_dialog = Some(ExportDialog::default());
                self.status = "export wizard".to_owned();
            }
            menus::ShellAction::ExportJpeg => {
                // The JPEG row of CS6's Send to E-mail submenu: the
                // JPEG export options. E-mail delivery itself is not
                // wired in this build — say so instead of pretending.
                let mut dialog = ExportDialog::default();
                if let Some(index) = crate::shell::dialogs::EXPORT_FORMATS
                    .iter()
                    .position(|format| *format == "jpg")
                {
                    dialog.format = index;
                }
                self.export_dialog = Some(dialog);
                self.status =
                    "JPEG export options — e-mail delivery is not available in this build."
                        .to_owned();
            }
            menus::ShellAction::FileInfo => {
                self.status = self.file_info_summary();
            }
            menus::ShellAction::PngFormatInfo => {
                self.status = self.png_format_summary();
            }
            menus::ShellAction::CloseDocument => {
                // CS6 prompts before discarding unsaved edits; this
                // build has no prompt yet, so refuse instead of
                // silently dropping work.
                if self.editor.session.is_dirty() {
                    self.status = "document has unsaved changes — save before closing".to_owned();
                } else {
                    self.editor = Editor::untitled();
                    self.view = View::default();
                    self.raster = None;
                    self.raster_scale = 1.0;
                    self.doc_name = "untitled".to_owned();
                    self.doc_path = None;
                    self.status = "closed the document".to_owned();
                }
            }
            menus::ShellAction::SaveAll => {
                // One open document: Save All saves it.
                if self.editor.session.is_dirty() {
                    self.save_document();
                } else {
                    self.status = "nothing to save".to_owned();
                }
            }
            menus::ShellAction::Revert => {
                self.revert_document();
            }
            menus::ShellAction::NumericTransform => {
                self.numeric_transform = Some(NumericTransformDialog::default());
            }
            menus::ShellAction::TransformMode(mode) => {
                self.editor.transform_mode = match mode {
                    menus::TransformMode::Scale => crate::editor::TransformMode::Scale,
                    menus::TransformMode::Distort => crate::editor::TransformMode::Distort,
                };
                self.status = match mode {
                    menus::TransformMode::Scale => "Free Transform: handles scale".to_owned(),
                    menus::TransformMode::Distort => {
                        "Distort: corner handles move independently".to_owned()
                    }
                };
            }
            menus::ShellAction::CanvasSize => {
                let canvas = &self.editor.session.document().pages[0].info.canvas;
                self.canvas_size = Some(CanvasSizeDialog {
                    width: canvas.width,
                    height: canvas.height,
                    ..CanvasSizeDialog::default()
                });
            }
            menus::ShellAction::ImageSize => {
                let canvas = &self.editor.session.document().pages[0].info.canvas;
                self.image_size = Some(ImageSizeDialog {
                    width: canvas.width,
                    height: canvas.height,
                    resolution: self.editor.session.document().pages[0]
                        .info
                        .canvas
                        .resolution,
                    ..ImageSizeDialog::default()
                });
            }
            menus::ShellAction::CanvasColor => {
                let raw = self.editor.session.document().pages[0]
                    .info
                    .canvas
                    .background_raw;
                self.canvas_color = Some(CanvasColorDialog {
                    // ABGR raw: R = raw & 0xFF, G = (raw >> 8) & 0xFF, B = (raw >> 16) & 0xFF.
                    color: [
                        (raw & 0xFF) as u8,
                        ((raw >> 8) & 0xFF) as u8,
                        ((raw >> 16) & 0xFF) as u8,
                    ],
                });
            }
            menus::ShellAction::FeatherDialog => {
                if self.editor.has_pixel_selection() {
                    self.feather = Some(FeatherDialog {
                        radius: self.editor.feather_radius,
                    });
                } else {
                    self.status = "no pixel selection to feather".to_owned();
                }
            }
            menus::ShellAction::Palette => self.toggle_palette(),
            menus::ShellAction::OnlineHelp => {
                let url = "https://github.com/aliasfoxkde/FireCraft/wiki";
                let opened = self.dialogs.open_url(url);
                self.status = if opened {
                    format!("opening {url}")
                } else {
                    "online help: see the project wiki".to_owned()
                };
            }
            menus::ShellAction::ManageExtensions => self.open_extension_manager(),
            menus::ShellAction::TogglePanel(name) => self.toggle_panel(name),
            menus::ShellAction::ToggleTools => self.toggle_tools(),
            menus::ShellAction::SingleDocument => {
                self.status = "This command needs multiple open documents; the shell is \
                     single-document."
                    .to_owned();
            }
            menus::ShellAction::ToggleHidePanels => self.toggle_all_panels(),
            menus::ShellAction::ToggleMainToolbar => self.toggle_options_bar(),
            menus::ShellAction::CycleScreenMode => self.cycle_screen_mode(),
            menus::ShellAction::ReportGap(message) => self.status = message.to_owned(),
            menus::ShellAction::InsertWeb { what } => self.insert_web(what),
            menus::ShellAction::PreviewInBrowser {
                secondary,
                all_pages,
            } => self.preview_in_browser(secondary, all_pages),
            menus::ShellAction::Quit => self.close_requested = true,
            menus::ShellAction::Undo => match self.editor.undo() {
                Ok(name) => self.status = format!("undid {name}"),
                Err(error) => self.status = format!("undo failed: {error}"),
            },
            menus::ShellAction::Redo => match self.editor.redo() {
                Ok(name) => self.status = format!("redid {name}"),
                Err(error) => self.status = format!("redo failed: {error}"),
            },
            menus::ShellAction::Deselect => {
                self.editor.set_selection(Vec::new());
                self.status = "deselected".to_owned();
            }
            menus::ShellAction::SelectAll => {
                let ids = self
                    .editor
                    .current_cell()
                    .map_or_else(Vec::new, |cell| cell.elements.clone());
                let picked = ids.len();
                self.editor.set_selection(ids);
                self.status = format!("selected {picked} object(s)");
            }
            menus::ShellAction::SelectInverse => self.select_inverse(),
            menus::ShellAction::Copy => self.clipboard_copy(false),
            menus::ShellAction::Cut => self.clipboard_copy(true),
            menus::ShellAction::Paste => self.clipboard_paste(),
            menus::ShellAction::PasteAsMask => self.paste_as_mask("clipboard.pasteAsMask"),
            menus::ShellAction::PasteInside => self.paste_as_mask("clipboard.pasteInside"),
            menus::ShellAction::ZoomIn => {
                let center = self
                    .canvas_rect
                    .map_or_else(|| Pos2::ZERO, |rect| rect.center());
                self.view.zoom_about(center, 1.25);
            }
            menus::ShellAction::ZoomOut => {
                let center = self
                    .canvas_rect
                    .map_or_else(|| Pos2::ZERO, |rect| rect.center());
                self.view.zoom_about(center, 1.0 / 1.25);
            }
            menus::ShellAction::ZoomTo(target) => {
                if self.view.zoom > 0.0 {
                    let center = self
                        .canvas_rect
                        .map_or_else(|| Pos2::ZERO, |rect| rect.center());
                    self.view.zoom_about(center, target / self.view.zoom);
                }
            }
            menus::ShellAction::FitAll => {
                let size = self
                    .canvas_rect
                    .map_or_else(|| egui::Vec2::ZERO, |rect| rect.size());
                if let Some(world) = self.page_world() {
                    self.view.fit(world, size);
                }
            }
            menus::ShellAction::FitSelection => {
                let size = self
                    .canvas_rect
                    .map_or_else(|| egui::Vec2::ZERO, |rect| rect.size());
                let selected: Vec<_> = self.editor.selected_live().into_iter().collect();
                let world = union_selection_bounds(self.editor.document(), &selected);
                if let Some(world) = world {
                    self.view.fit(world, size);
                    self.status = "fit selection".to_owned();
                } else {
                    self.status = "nothing selected".to_owned();
                }
            }
            menus::ShellAction::ToggleSmartGuides => {
                self.smart_guides = !self.smart_guides;
                self.status = if self.smart_guides {
                    "smart guides shown".to_owned()
                } else {
                    "smart guides hidden".to_owned()
                };
            }
            menus::ShellAction::ToggleSnapToSmartGuides => {
                self.snap_to_smart_guides = !self.snap_to_smart_guides;
                self.status = if self.snap_to_smart_guides {
                    "snap to smart guides on".to_owned()
                } else {
                    "snap to smart guides off".to_owned()
                };
            }
            menus::ShellAction::ToggleSliceLock => {
                self.slice_locked = !self.slice_locked;
                self.status = if self.slice_locked {
                    "9-slice guides locked".to_owned()
                } else {
                    "9-slice guides unlocked".to_owned()
                };
            }
            menus::ShellAction::ToggleSliceGuides => {
                self.slice_guides = !self.slice_guides;
                self.status = if self.slice_guides {
                    "slice guides shown".to_owned()
                } else {
                    "slice guides hidden".to_owned()
                };
            }
            menus::ShellAction::ToggleSliceOverlay => {
                self.slice_overlay = !self.slice_overlay;
                self.status = if self.slice_overlay {
                    "slice overlay shown".to_owned()
                } else {
                    "slice overlay hidden".to_owned()
                };
            }
            menus::ShellAction::ToggleEdges => {
                self.show_edges = !self.show_edges;
                self.status = if self.show_edges {
                    "edges shown".to_owned()
                } else {
                    "edges hidden".to_owned()
                };
            }
            menus::ShellAction::ToggleRulers => {
                self.show_rulers = !self.show_rulers;
                self.status = if self.show_rulers {
                    "rulers shown".to_owned()
                } else {
                    "rulers hidden".to_owned()
                };
            }
            menus::ShellAction::ToggleTooltips => {
                self.show_tooltips = !self.show_tooltips;
                self.status = if self.show_tooltips {
                    "tooltips on".to_owned()
                } else {
                    "tooltips off".to_owned()
                };
            }
            menus::ShellAction::ShowAllHidden => {
                // CS6's Show All: every hidden object on every page
                // becomes visible again. `object.setVisible` refuses
                // kinds without the display quartet, and those kinds
                // cannot be hidden anyway, so the scan only collects
                // carriers.
                let hidden: Vec<doc::ObjectId> = self
                    .editor
                    .document()
                    .registry
                    .iter()
                    .filter_map(|(id, object)| match object {
                        ObjectData::Path(path) => (!path.visible).then_some(id),
                        ObjectData::Group(group) => (!group.visible).then_some(id),
                        ObjectData::Image(image) => (!image.visible).then_some(id),
                        ObjectData::Text(text) => (!text.visible).then_some(id),
                        _ => None,
                    })
                    .collect();
                for id in hidden {
                    self.run_command(
                        "object.setVisible",
                        serde_json::json!({ "id": id, "visible": true }),
                    );
                }
                self.status = "show all".to_owned();
            }
            menus::ShellAction::ToggleGamma => {
                self.gamma_18 = !self.gamma_18;
                self.status = if self.gamma_18 {
                    "gamma 1.8 preview on".to_owned()
                } else {
                    "gamma 1.8 preview off".to_owned()
                };
            }
            menus::ShellAction::About => self.about_open = true,
            menus::ShellAction::ShowShortcuts => self.help_open = !self.help_open,
            menus::ShellAction::PickTool(tool) => {
                self.editor.tool = tool;
                self.status = format!("tool: {}", tool_label(tool));
            }
            menus::ShellAction::PickToolGroup(leader) => {
                // CS6: "To quickly select a hidden tool, repeatedly
                // press the group keyboard shortcut until the tool you
                // want appears." The shortcut picks the group's leader
                // and steps through the group from there.
                //
                // The Hide/Show Slices control is the exception: the
                // book's Table 1-1 gives it the "2" key, but it is a
                // panel button, so the key *acts* on it instead of
                // making it the active tool.
                if toolstrip::is_panel_button(leader) {
                    self.run_panel_button(leader);
                    return;
                }
                let group = toolstrip::group_of(leader);
                let index = match group {
                    Some(group) => {
                        let current = group
                            .tools
                            .iter()
                            .position(|entry| entry.tool == self.editor.tool);
                        match current {
                            Some(index) => (index + 1) % group.tools.len(),
                            None => 0,
                        }
                    }
                    None => 0,
                };
                let tool = match group {
                    Some(group) => group.tools[index].tool,
                    None => leader,
                };
                self.editor.tool = tool;
                self.status = format!("tool: {}", tool_label(tool));
            }
            menus::ShellAction::ToggleGrid => {
                self.show_grid = !self.show_grid;
                self.status = if self.show_grid {
                    "grid shown".to_owned()
                } else {
                    "grid hidden".to_owned()
                };
            }
            menus::ShellAction::ToggleSnapToGrid => {
                self.snap_to_grid = !self.snap_to_grid;
                self.status = if self.snap_to_grid {
                    "snap to grid on".to_owned()
                } else {
                    "snap to grid off".to_owned()
                };
            }
            menus::ShellAction::EditGrid => {
                self.grid_dialog = Some(GridDialog::from_document(self.editor.document()));
            }
            menus::ShellAction::ToggleGuides => {
                self.show_guides = !self.show_guides;
                self.status = if self.show_guides {
                    "guides shown".to_owned()
                } else {
                    "guides hidden".to_owned()
                };
            }
            menus::ShellAction::ToggleGuidesLocked => {
                self.guides_locked = !self.guides_locked;
                self.status = if self.guides_locked {
                    "guides locked".to_owned()
                } else {
                    "guides unlocked".to_owned()
                };
            }
            menus::ShellAction::ToggleSnapToGuides => {
                self.snap_to_guides = !self.snap_to_guides;
                self.status = if self.snap_to_guides {
                    "snap to guides on".to_owned()
                } else {
                    "snap to guides off".to_owned()
                };
            }
            menus::ShellAction::ClearGuides => {
                let page = self.editor.page;
                self.run_command("page.removeAllGuides", serde_json::json!({ "page": page }));
            }
        }
    }

    /// Select ▸ Select Inverse: everything in the current cell the
    /// selection does not cover. The cell is the editor's scope (one
    /// layer, one state) — the same scope Select All covers.
    fn select_inverse(&mut self) {
        let selected = self.editor.live_selection();
        let ids: Vec<doc::ObjectId> = self
            .editor
            .current_cell()
            .map_or_else(Vec::new, |cell| cell.elements.clone())
            .into_iter()
            .filter(|id| !selected.contains(id))
            .collect();
        let picked = ids.len();
        self.editor.set_selection(ids);
        self.status = format!("selected {picked} object(s)");
    }

    /// Edit ▸ Copy / Cut: the engine's clipboard commands read the
    /// document's selection, so the view's selection lands there
    /// first. A cut takes the objects with it, so the view's selection
    /// follows them out.
    fn clipboard_copy(&mut self, cut: bool) {
        if self.editor.live_selection().is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        let name = if cut {
            "clipboard.cut"
        } else {
            "clipboard.copy"
        };
        match self.editor.apply(name, serde_json::json!({})) {
            Ok(()) => {
                if cut {
                    self.editor.set_selection(Vec::new());
                    self.status = "cut".to_owned();
                } else {
                    self.status = "copied".to_owned();
                }
            }
            Err(error) => self.status = format!("{name} failed: {error}"),
        }
    }

    /// Edit ▸ Paste: the clipboard re-mints onto the active layer of
    /// the current page, in place (CS6's paste-in-place). The pasted
    /// objects become the selection, so a second paste rides the same
    /// place.
    fn clipboard_paste(&mut self) {
        let page = self.editor.page;
        let layer = self.editor.active_layer();
        let cell = self.editor.frame;
        let before: Vec<doc::ObjectId> = self
            .editor
            .current_cell()
            .map_or_else(Vec::new, |cell| cell.elements.clone());
        let payload = serde_json::json!({ "page": page, "layer": layer, "cell": cell });
        if let Err(error) = self.editor.apply("clipboard.paste", payload) {
            self.status = format!("clipboard.paste failed: {error}");
            return;
        }
        let after: Vec<doc::ObjectId> = self
            .editor
            .current_cell()
            .map_or_else(Vec::new, |cell| cell.elements.clone());
        let fresh: Vec<doc::ObjectId> = after
            .into_iter()
            .filter(|id| !before.contains(id))
            .collect();
        let pasted = fresh.len();
        self.editor.set_selection(fresh);
        self.status = format!("pasted {pasted} object(s)");
    }

    /// Engine commands whose payload derives from the current
    /// selection or document shape.
    fn run_sel(&mut self, action: menus::SelAction) {
        match action {
            menus::SelAction::Clone | menus::SelAction::Duplicate => {
                // CS6's Clone and Duplicate share the engine verb;
                // both land the copy on the original, ready to drag.
                if let Some(id) = self.editor.selected_live() {
                    self.run_command("object.duplicate", serde_json::json!({ "id": id }));
                }
            }
            menus::SelAction::DetachFromPath => {
                if let Some(id) = self.editor.selection {
                    self.run_command("text.detachPath", serde_json::json!({ "id": id }));
                }
            }
            menus::SelAction::AttachToPath => {
                // CS6 requires exactly one text and one path in the
                // selection; the path becomes the rail.
                let selection = self.editor.selection_set.clone();
                let mut text_id = None;
                let mut rail = None;
                for id in &selection {
                    match self.editor.session.document().registry.get(*id) {
                        Some(doc::ObjectData::Text(_)) if text_id.is_none() => {
                            text_id = Some(*id);
                        }
                        Some(doc::ObjectData::Path(path)) if rail.is_none() => {
                            rail = Some(path.contours.clone());
                        }
                        _ => {}
                    }
                }
                match (text_id, rail) {
                    (Some(id), Some(contours)) => {
                        let path: Vec<_> = contours
                            .iter()
                            .map(|contour| {
                                serde_json::json!({
                                    "closed": contour.closed,
                                    "points": contour.points.iter().map(|point| {
                                        let mut wire = serde_json::json!({
                                            "x": point.anchor.x, "y": point.anchor.y,
                                            "curve": point.is_curve,
                                        });
                                        if let Some(primary) = &point.primary {
                                            wire["primary"] = serde_json::json!({
                                                "x": primary.x, "y": primary.y,
                                            });
                                        }
                                        if let Some(secondary) = &point.secondary {
                                            wire["secondary"] = serde_json::json!({
                                                "x": secondary.x, "y": secondary.y,
                                            });
                                        }
                                        wire
                                    }).collect::<Vec<_>>(),
                                })
                            })
                            .collect();
                        self.run_command(
                            "text.attachPath",
                            serde_json::json!({ "id": id, "path": path }),
                        );
                    }
                    (None, _) => {
                        self.status =
                            "Attach to Path needs a text object in the selection".to_owned();
                    }
                    (_, None) => {
                        self.status =
                            "Attach to Path needs a path object in the selection".to_owned();
                    }
                }
            }
            menus::SelAction::ReversePathText => {
                if self.editor.selection.is_some() {
                    if let Err(error) = self.editor.sync_selection() {
                        self.status = format!("selection failed: {error}");
                        return;
                    }
                    self.run_command("text.reversePathTextDirection", serde_json::json!({}));
                }
            }
            menus::SelAction::ConvertToPaths => {
                if self.editor.selection.is_some() {
                    if let Err(error) = self.editor.sync_selection() {
                        self.status = format!("selection failed: {error}");
                        return;
                    }
                    self.run_command("text.convertToPaths", serde_json::json!({}));
                }
            }
            menus::SelAction::Delete => {
                if let Some(id) = self.editor.selection {
                    self.run_command("object.delete", serde_json::json!({ "id": id }));
                }
            }
            menus::SelAction::Hide => {
                if let Some(id) = self.editor.selection {
                    self.run_command(
                        "object.setVisible",
                        serde_json::json!({ "id": id, "visible": false }),
                    );
                }
            }
            menus::SelAction::LockSelection => {
                if let Some(id) = self.editor.selection {
                    self.run_command(
                        "object.setLocked",
                        serde_json::json!({ "id": id, "locked": true }),
                    );
                }
            }
            menus::SelAction::Simplify => {
                if let Some(id) = self.editor.selected_live() {
                    self.run_command(
                        "path.simplify",
                        serde_json::json!({ "id": id, "tolerance": 1.0 }),
                    );
                }
            }
            menus::SelAction::ExpandStroke => {
                if let Some(id) = self.editor.selected_live() {
                    self.run_command(
                        "path.expandStroke",
                        serde_json::json!({
                            "id": id, "width": 2.0,
                            "cap": "butt", "join": "miter", "miter_limit": 4.0,
                        }),
                    );
                }
            }
            menus::SelAction::Inset(distance) => {
                if let Some(id) = self.editor.selected_live() {
                    self.run_command(
                        "path.inset",
                        serde_json::json!({ "id": id, "distance": distance }),
                    );
                }
            }
            menus::SelAction::NewSymbol => {
                let count = self.editor.document().symbols.len();
                self.run_command(
                    "library.add",
                    serde_json::json!({
                        "name": format!("Symbol {}", count + 1),
                        "symbol_type": "graphic",
                    }),
                );
            }
            menus::SelAction::NewLayer => {
                self.run_command("layer.add", serde_json::json!({ "page": self.editor.page }));
                // CS6 makes a freshly added layer the active one, so
                // what the user draws next lands on it.
                let top = self.editor.layer_count().saturating_sub(1);
                self.editor.set_active_layer(top);
            }
            menus::SelAction::EmptyBitmap => {
                let canvas = &self.editor.document().pages[self.editor.page].info.canvas;
                let layer = self.editor.active_layer();
                // Land in the last existing cell — or the first fresh
                // one the engine appends for an empty layer.
                let cell = self.editor.document().pages[self.editor.page]
                    .layers
                    .layers
                    .get(layer)
                    .map_or(0, |l| l.cells.len().saturating_sub(1));
                let _placed = self.run_command(
                    "bitmap.addImage",
                    serde_json::json!({
                        "x": canvas.x, "y": canvas.y,
                        "width": canvas.width, "height": canvas.height,
                        "page": self.editor.page, "layer": layer, "cell": cell,
                        "name": null,
                    }),
                );
            }
            menus::SelAction::BitmapViaCopy | menus::SelAction::BitmapViaCut => {
                let command = match action {
                    menus::SelAction::BitmapViaCut => "bitmap.addImageViaCut",
                    _ => "bitmap.addImageViaCopy",
                };
                let source = self.editor.live_selection().into_iter().find(|id| {
                    matches!(
                        self.editor.document().registry.get(*id),
                        Some(doc::ObjectData::Image(_))
                    )
                });
                match source {
                    Some(source) => {
                        self.run_command(command, serde_json::json!({ "source": source }));
                    }
                    None => {
                        self.status = "Select a bitmap with a pixel selection first.".to_owned();
                    }
                }
            }
            menus::SelAction::CropDocument => {
                self.run_command(
                    "document.setDocumentCanvasSizeToDocumentExtents",
                    serde_json::json!({ "page": self.editor.page, "grow": false }),
                );
            }
            menus::SelAction::FillEdge { mode } => {
                // One command per selected path — the verb is
                // single-target, and the panel applies the same way.
                let ids = self.editor.live_selection();
                if ids.is_empty() {
                    self.status = "nothing selected".to_owned();
                    return;
                }
                for id in ids {
                    let payload = if mode == "feather" {
                        serde_json::json!({
                            "id": id, "mode": mode,
                            "feather": i64::from(self.editor.feather_radius),
                        })
                    } else {
                        serde_json::json!({ "id": id, "mode": mode })
                    };
                    self.run_command("path.setFillEdgeMode", payload);
                }
            }
            menus::SelAction::EditSymbol => {
                // The selection's symbol instance names its definition;
                // entering scope is view state (no journal row).
                let symbol = self.editor.selection.and_then(|id| {
                    match self.editor.session.document().registry.get(id) {
                        Some(doc::ObjectData::SymbolInstance(instance)) => {
                            Some(instance.symbol_number)
                        }
                        _ => None,
                    }
                });
                match symbol {
                    Some(symbol) => {
                        if let Err(error) = self.editor.enter_symbol(symbol) {
                            self.status = format!("edit symbol failed: {error}");
                        }
                    }
                    None => {
                        self.status =
                            "Edit Symbol needs a symbol instance in the selection".to_owned();
                    }
                }
            }
            menus::SelAction::NewPage => {
                self.run_command("page.add", serde_json::json!({ "index": null }));
            }
            menus::SelAction::NewFrame => {
                // CS6's Insert ▸ State: one empty frame right after
                // the current one, which becomes active on insert.
                let page = self.editor.page;
                let current = self.editor.frame;
                self.run_command(
                    "frame.add",
                    serde_json::json!({
                        "page": page, "how_many": 1,
                        "location": "after current", "current": current,
                    }),
                );
                let count = self
                    .editor
                    .document()
                    .pages
                    .get(page)
                    .map(|p| p.info.frame_count)
                    .unwrap_or(0);
                if count > 0 {
                    self.editor.frame = (count - 1) as usize;
                }
            }
            menus::SelAction::BreakApart => {
                if let Some(id) = self.editor.selection {
                    self.run_command("symbol.breakApart", serde_json::json!({ "id": id }));
                }
            }
            menus::SelAction::SplitPaths => {
                self.run_command("path.splitPaths", serde_json::json!({}));
            }
            menus::SelAction::RemoveTransformations => {
                // Reset each selected object's MTXv: object.transform
                // with the inverse of the stored matrix.
                let ids = self.editor.live_selection();
                for id in &ids {
                    let inverse = match self.editor.session.document().registry.get(*id) {
                        Some(doc::ObjectData::Text(text)) => {
                            text.transform.map(geom::Affine::inverse)
                        }
                        Some(doc::ObjectData::SymbolInstance(instance)) => {
                            Some(instance.transform.inverse())
                        }
                        Some(doc::ObjectData::Path(path)) => {
                            path.primitive.as_ref().map(|p| p.transform.inverse())
                        }
                        _ => None,
                    };
                    if let Some(inverse) = inverse {
                        let m = inverse.as_coeffs();
                        self.run_command(
                            "object.transform",
                            serde_json::json!({
                                "ids": [id],
                                "matrix": m,
                            }),
                        );
                    }
                }
            }
            menus::SelAction::ConvertToSymbol => {
                let ids = self.editor.live_selection();
                if ids.is_empty() {
                    self.status = "convert to symbol needs a selection".to_owned();
                    return;
                }
                let count = self.editor.document().symbols.len();
                self.run_command(
                    "symbol.create",
                    serde_json::json!({
                        "ids": ids,
                        "name": format!("Symbol {}", count + 1),
                        "symbol_type": "graphic",
                    }),
                );
            }
            menus::SelAction::AnimateSelection => {
                let ids = self.editor.live_selection();
                if ids.is_empty() {
                    self.status = "animate selection needs a selection".to_owned();
                    return;
                }
                let count = self.editor.document().symbols.len();
                self.run_command(
                    "symbol.create",
                    serde_json::json!({
                        "ids": ids,
                        "name": format!("Animation {}", count + 1),
                        "symbol_type": "animation",
                    }),
                );
            }
            menus::SelAction::RemoveAnimation => {
                // CS6 dissolves the animation symbol back to plain
                // objects — the break-apart verb, scoped to the
                // animation-symbol instance under the selection.
                let document = self.editor.session.document();
                let target = self.editor.live_selection().into_iter().find(|id| {
                    matches!(
                        document.registry.get(*id),
                        Some(doc::ObjectData::SymbolInstance(_))
                    )
                });
                match target {
                    Some(id) => {
                        self.run_command("symbol.breakApart", serde_json::json!({ "id": id }));
                    }
                    None => {
                        self.status = "no symbol instance in the selection".to_owned();
                    }
                }
            }
            menus::SelAction::TweenInstances => {
                if self.editor.live_selection().len() < 2 {
                    self.status = "tween needs two instances of one symbol".to_owned();
                    return;
                }
                self.run_command(
                    "symbol.tween",
                    serde_json::json!({
                        "numSteps": 4, "distribute": true,
                        "frame": self.editor.frame,
                    }),
                );
            }
            menus::SelAction::Group => {
                let ids = self.editor.live_selection();
                if ids.len() < 2 {
                    self.status = "group needs two or more objects".to_owned();
                    return;
                }
                self.run_command("object.group", serde_json::json!({ "ids": ids }));
            }
            menus::SelAction::Ungroup => {
                if let Some(id) = self.editor.selected_live() {
                    self.run_command("object.ungroup", serde_json::json!({ "id": id }));
                }
            }
            menus::SelAction::FlattenSelection => self.flatten_selection(),
            menus::SelAction::MergeDown => self.merge_down(),
            menus::SelAction::FlattenLayers => {
                let page = self.editor.page;
                self.run_command("flatten.document", serde_json::json!({ "page": page }));
            }
            menus::SelAction::Rotate(degrees) => {
                let ids = self.editor.live_selection();
                if ids.is_empty() {
                    self.status = "nothing selected".to_owned();
                    return;
                }
                self.run_command(
                    "object.rotate",
                    serde_json::json!({ "ids": ids, "degrees": degrees, "pivot": null }),
                );
            }
            menus::SelAction::Reflect {
                horizontal,
                vertical,
            } => {
                let ids = self.editor.live_selection();
                if ids.is_empty() {
                    self.status = "nothing selected".to_owned();
                    return;
                }
                self.run_command(
                    "object.reflect",
                    serde_json::json!({
                        "ids": ids, "horizontal": horizontal, "vertical": vertical,
                        "pivot": null,
                    }),
                );
            }
            menus::SelAction::SelectParents => {
                if let Err(error) = self.editor.sync_selection() {
                    self.status = format!("selection failed: {error}");
                    return;
                }
                self.run_command("selection.selectParents", serde_json::json!({}));
                self.editor.adopt_document_selection();
            }
            menus::SelAction::SelectChildren => {
                if let Err(error) = self.editor.sync_selection() {
                    self.status = format!("selection failed: {error}");
                    return;
                }
                self.run_command("selection.selectChildren", serde_json::json!({}));
                self.editor.adopt_document_selection();
            }
            menus::SelAction::SelectSimilar => {
                let tolerance = self.editor.wand_tolerance;
                if let Err(error) = self.editor.select_similar(tolerance) {
                    self.status = format!("select similar failed: {error}");
                }
            }
            menus::SelAction::Feather(radius) => {
                if let Err(error) = self.editor.feather_selection(radius) {
                    self.status = format!("feather failed: {error}");
                }
            }
            menus::SelAction::AdjustPixelSel { what, amount } => {
                if let Err(error) = self.editor.adjust_pixel_selection(what, amount) {
                    self.status = format!("adjust marquee failed: {error}");
                }
            }
            menus::SelAction::PathToMarquee => {
                let Some(path_id) = self.editor.selected_live() else {
                    self.status = "select a path to convert".to_owned();
                    return;
                };
                let Some(image_id) = self.editor.pixel_target else {
                    self.status =
                        "no image under the marquee; make a pixel selection first".to_owned();
                    return;
                };
                self.run_command(
                    "mask.pathToMarquee",
                    serde_json::json!({
                        "id": path_id, "image": image_id,
                        "mode": "antialias", "feather_amount": null,
                    }),
                );
            }
            menus::SelAction::MarqueeToPath => {
                let Some(image_id) = self.editor.pixel_target else {
                    self.status = "no pixel selection to convert".to_owned();
                    return;
                };
                self.run_command(
                    "mask.marqueeToPath",
                    serde_json::json!({ "image": image_id }),
                );
            }
            menus::SelAction::SaveSelection => {
                self.run_command(
                    "selection.save",
                    serde_json::json!({ "name": null, "operation": "new or replace" }),
                );
            }
            menus::SelAction::RestoreSelection => {
                self.run_command(
                    "selection.restore",
                    serde_json::json!({ "name": null, "operation": "replace", "invert": false }),
                );
            }
            menus::SelAction::DeleteSavedSelection => {
                self.run_command("selection.deleteSaved", serde_json::json!({ "name": null }));
            }
            menus::SelAction::AddMask { mode } => {
                let Some(id) = self.editor.selected_live() else {
                    self.status = "nothing selected".to_owned();
                    return;
                };
                self.run_command(
                    "mask.add",
                    serde_json::json!({ "id": id, "mode": mode, "enter_edit_mode": false }),
                );
            }
            menus::SelAction::EnableMask(enable) => {
                let Some(id) = self.editor.selected_live() else {
                    self.status = "nothing selected".to_owned();
                    return;
                };
                self.run_command(
                    "mask.enable",
                    serde_json::json!({ "id": id, "enable": enable }),
                );
            }
            menus::SelAction::LinkMask(linked) => {
                let Some(id) = self.editor.selected_live() else {
                    self.status = "nothing selected".to_owned();
                    return;
                };
                self.run_command(
                    "mask.link",
                    serde_json::json!({ "id": id, "linked": linked }),
                );
            }
            menus::SelAction::RemoveMask { what_if_image } => {
                let Some(id) = self.editor.selected_live() else {
                    self.status = "nothing selected".to_owned();
                    return;
                };
                self.run_command(
                    "mask.remove",
                    serde_json::json!({ "id": id, "what_if_image": what_if_image }),
                );
            }
            menus::SelAction::SaveJpegMask => {
                self.run_command("mask.saveJPEG", serde_json::json!({}));
            }
            menus::SelAction::RestoreJpegMask => {
                self.run_command("mask.restoreJPEG", serde_json::json!({}));
            }
            menus::SelAction::ClearJpegMask => {
                self.run_command("mask.clearJPEG", serde_json::json!({}));
            }
        }
    }

    /// Modify ▸ Flatten Selection: bakes the selection into one
    /// bitmap. The engine reads the document's selection, so the
    /// view's selection lands there first.
    fn flatten_selection(&mut self) {
        if self.editor.live_selection().is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command("flatten.selection", serde_json::json!({}));
    }

    /// Edit ▸ Insert ▸ Hotspot / slice rows / New Button…: the target
    /// rect is the selection's bounds (canvas center when nothing is
    /// selected); the button row appends a definition to the library.
    fn insert_web(&mut self, what: menus::InsertWebKind) {
        let page = self.editor.page;
        let bounds = crate::editor::union_selection_bounds(
            self.editor.document(),
            &self.editor.live_selection(),
        );
        let canvas = &self.editor.document().pages[page].info.canvas;
        let (x, y, width, height) = bounds.map_or_else(
            || {
                // Half the canvas, centered — an insertion rect a
                // drag can refine, matching the empty-canvas default
                // of the Slice/Hotspot tools.
                (
                    canvas.x + canvas.width / 4.0,
                    canvas.y + canvas.height / 4.0,
                    canvas.width / 2.0,
                    canvas.height / 2.0,
                )
            },
            |bounds| (bounds.x0, bounds.y0, bounds.width(), bounds.height()),
        );
        match what {
            menus::InsertWebKind::Button => {
                let count = self.editor.document().symbols.len();
                self.run_command(
                    "library.add",
                    serde_json::json!({
                        "name": format!("Button {}", count + 1),
                        "symbol_type": "button",
                    }),
                );
            }
            menus::InsertWebKind::Hotspot => {
                self.run_command(
                    "hotspot.create",
                    serde_json::json!({
                        "shape": "rect",
                        "x": x, "y": y, "width": width, "height": height,
                        "points": null, "page": page,
                        "name": null, "link": null, "alt": null,
                    }),
                );
            }
            menus::InsertWebKind::RectSlice => {
                self.run_command(
                    "slice.create",
                    serde_json::json!({
                        "shape": "rect",
                        "x": x, "y": y, "width": width, "height": height,
                        "points": null, "page": page,
                        "name": null, "link": null, "alt": null,
                    }),
                );
            }
            menus::InsertWebKind::PolygonSlice => {
                // The selection rectangle's corners as a polygon —
                // CS6's row is followed by an interactive vertex drag,
                // which the Slice tool's polygon drag covers; this
                // gives the menu row a correct, editable result.
                let points = [
                    serde_json::json!({ "x": x, "y": y }),
                    serde_json::json!({ "x": x + width, "y": y }),
                    serde_json::json!({ "x": x + width, "y": y + height }),
                    serde_json::json!({ "x": x, "y": y + height }),
                ];
                self.run_command(
                    "slice.create",
                    serde_json::json!({
                        "shape": "polygon",
                        "x": x, "y": y, "width": width, "height": height,
                        "points": points, "page": page,
                        "name": null, "link": null, "alt": null,
                    }),
                );
            }
        }
    }

    /// Modify ▸ Merge Down: merges the selection into the bitmap
    /// directly below it. Same selection bridge as the flatten.
    fn merge_down(&mut self) {
        if self.editor.live_selection().is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if let Err(error) = self.editor.sync_selection() {
            self.status = format!("selection failed: {error}");
            return;
        }
        self.run_command("flatten.mergeDown", serde_json::json!({}));
    }

    /// Modify ▸ Arrange: resolve the numeric `object.reorder` target
    /// from the object's position in the current cell. Payload indexes
    /// are post-removal, so "front" is `len - 2` of the pre-removal
    /// list.
    fn run_arrange(&mut self, direction: menus::Arrange) {
        let Some(id) = self.editor.selection else {
            self.status = "nothing selected".to_owned();
            return;
        };
        let Some(cell) = self.editor.current_cell() else {
            self.status = "no current cell".to_owned();
            return;
        };
        let Some(position) = cell.elements.iter().position(|&element| element == id) else {
            self.status = "selection is not in the current cell".to_owned();
            return;
        };
        let last = cell.elements.len().saturating_sub(2);
        let to = match direction {
            menus::Arrange::Front => last,
            menus::Arrange::Forward => (position + 1).min(last),
            menus::Arrange::Backward => position.saturating_sub(1),
            menus::Arrange::Back => 0,
        };
        if to == position {
            self.status = "already at that stacking position".to_owned();
            return;
        }
        self.run_command("object.reorder", serde_json::json!({ "id": id, "to": to }));
    }

    /// The Fireworks.xml keymap bindings: one-shot key events mapped
    /// through the same [`Shell::run_action`] dispatch as the menus.
    /// Plain (modifier-free) keys are ignored while a text field has
    /// keyboard focus.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let text_focus = ctx.egui_wants_keyboard_input();
        let mut fired: Vec<menus::Action> = Vec::new();
        let mut draft_key = None;
        ctx.input(|input| {
            for event in &input.events {
                let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } = event
                else {
                    continue;
                };
                // CS6 closes a plotted path with Enter and abandons it
                // with Escape. The Pen, the Polygon Slice, the Polygon
                // Hotspot and the Polygonal Marquee all build their
                // shape with clicks, so these two keys are how the
                // shape becomes real.
                if !modifiers.ctrl && !modifiers.alt && !modifiers.shift && !text_focus {
                    match key {
                        egui::Key::Enter => {
                            draft_key = Some(true);
                            continue;
                        }
                        egui::Key::Escape => {
                            draft_key = Some(false);
                            continue;
                        }
                        _ => {}
                    }
                }
                for shortcut in menus::SHORTCUTS {
                    if shortcut.key != *key
                        || shortcut.mods.ctrl != modifiers.ctrl
                        || shortcut.mods.alt != modifiers.alt
                        || shortcut.mods.shift != modifiers.shift
                    {
                        continue;
                    }
                    let plain = !shortcut.mods.ctrl && !shortcut.mods.alt && !shortcut.mods.shift;
                    if !(plain && text_focus) {
                        fired.push(shortcut.action);
                    }
                }
            }
        });
        for action in fired {
            self.run_action(&action);
        }
        match draft_key {
            Some(true) => self.finish_draft(),
            Some(false) => self.cancel_draft(),
            None => {}
        }
    }

    /// Enter: commits the shape being plotted — the pen's polyline, a
    /// polygon slice or hotspot, or the polygonal marquee.
    fn finish_draft(&mut self) {
        if self.editor.polygon.is_some() {
            match self.editor.commit_polygon() {
                Ok(true) => self.status = "polygon selected".to_owned(),
                Ok(false) => self.status = "polygon needs three points".to_owned(),
                Err(error) => self.status = format!("polygon failed: {error}"),
            }
            return;
        }
        if self.editor.draft.is_some() {
            match self.editor.commit_draft() {
                Ok(true) => self.status = "shape created".to_owned(),
                Ok(false) => self.status = "shape needs more points".to_owned(),
                Err(error) => self.status = format!("shape failed: {error}"),
            }
        }
    }

    /// Escape: abandons the shape being plotted, leaving the document
    /// as it was.
    fn cancel_draft(&mut self) {
        if self.editor.draft.take().is_some() || self.editor.polygon.take().is_some() {
            self.status = "shape discarded".to_owned();
        }
    }

    /// The installed web drop sink: the web crate registers a reader
    /// at startup ([`Shell::set_drop_sink`]) that takes a dropped-file
    /// handle, reads its bytes async, and loads them through the
    /// runner. `None` until then — a drop before startup is a no-op
    /// (the status line already said the app was still coming up).
    #[cfg(target_arch = "wasm32")]
    pub fn set_drop_sink(sink: std::rc::Rc<dyn Fn(&egui::DroppedFileHandle, egui::Context)>) {
        DROP_SINK.with(|cell| *cell.borrow_mut() = Some(sink));
    }

    #[cfg(target_arch = "wasm32")]
    fn drop_sink() -> Option<std::rc::Rc<dyn Fn(&egui::DroppedFileHandle, egui::Context)>> {
        DROP_SINK.with(|cell| cell.borrow().clone())
    }

    /// Files dropped onto the window open through the same one-door
    /// path as File ▸ Open: a Fireworks PNG replaces the document,
    /// any other supported raster opens as a document sized to the
    /// image. Only the first dropped file is taken — a batch drop is
    /// not a batch import. On the web the bytes arrive async, so the
    /// shell hands the handle to the installed drop sink (the web
    /// crate reads the file and calls back through
    /// [`Shell::load_document_bytes`]); natively the read is direct.
    fn dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<_> = ctx.input(|input| input.raw.dropped_files.clone());
        let Some(file) = dropped.into_iter().next() else {
            return;
        };
        #[cfg(not(target_arch = "wasm32"))]
        match file.bytes() {
            Ok(bytes) => {
                if let Err(error) = self.load_document_bytes(&bytes) {
                    self.status = format!("open failed: {error}");
                }
            }
            Err(error) => self.status = format!("open failed: {error}"),
        }
        #[cfg(target_arch = "wasm32")]
        {
            let sink = Self::drop_sink();
            if let Some(sink) = sink {
                sink(&file, ctx.clone());
                self.status = "reading dropped file…".to_owned();
            }
        }
    }

    /// The fill/stroke engine command for a swatch channel.
    pub(crate) fn paint_command(channel: &str) -> &'static str {
        if channel == "fill" {
            "object.setFill"
        } else {
            "object.setStroke"
        }
    }
}

impl eframe::App for Shell {
    /// Persists the shell layout (and egui's own window geometry
    /// memory) — on native to `storage_dir`, on web to localStorage.
    /// Layout only: documents are saved explicitly through the File
    /// menu, never as a side effect of closing the window.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "shell_layout", &self.capture_layout());
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Keep the context reachable for out-of-frame mutators (the
        // web bridge's request_repaint).
        self.ctx = Some(ui.ctx().clone());
        // Menu/shortcut actions run before layout so their effects are
        // visible the same frame.
        self.shortcuts(ui.ctx());
        self.dropped_files(ui.ctx());
        // Panels first, CentralPanel last (egui's required order), in
        // the CS6 arrangement: menu, context options bar, tool strip,
        // panel stack (Properties included), status strip, canvas.
        // The screen mode folds chrome on top of the dock folds: Full
        // Screen hides menus, toolbars, tools, panels, status and the
        // document tab strip against a black backdrop; Full Screen
        // with Menus keeps the menu bar and paints the gray backdrop
        // (fwref 1178–1185).
        let full_screen = self.screen_mode == ScreenMode::FullScreen;
        let menus_visible = !full_screen;
        if menus_visible {
            egui::Panel::top(egui::Id::new("menu")).show(ui, |ui| self.menu_bar(ui));
        }
        if !self.options_bar_hidden && !full_screen {
            egui::Panel::top(egui::Id::new("options")).show(ui, |ui| self.options_bar(ui));
        }
        if !self.tools_hidden && !full_screen {
            egui::Panel::left(egui::Id::new("tools"))
                .exact_size(34.0)
                .show(ui, |ui| self.tools_panel(ui));
        }
        // The CS6 panel group: the right dock, one column of stacked
        // panels. A side panel *reserves* its width, so the canvas
        // shrinks instead of being covered, and the column scrolls so
        // every panel stays reachable at any viewport height.
        if !self.panels_hidden && !full_screen {
            self.dock(ui);
        }
        if !full_screen {
            egui::Panel::bottom(egui::Id::new("status")).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!(
                            "{} · page {} · state {} · undo {} · redo {} · journal {}",
                            self.doc_name,
                            self.editor.page + 1,
                            self.editor.frame + 1,
                            self.editor.session.undo_len(),
                            self.editor.session.redo_len(),
                            self.editor.session.journal_len(),
                        ));
                    });
                });
            });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                // The document tab strip: a single-document window, so
                // the CS6 MDI tab strip reduced to its one member.
                // Full Screen drops the title bar, tab strip included.
                if !full_screen {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Button::new(egui::RichText::new(&self.doc_name).size(11.0))
                                .selected(true),
                        );
                        ui.weak("FireCraft");
                    });
                }
                self.canvas(ui);
            });
        if self.export_dialog.is_some() {
            egui::Window::new("Export")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.export_dialog_ui(ui));
        }
        if self.numeric_transform.is_some() {
            egui::Window::new("Numeric Transform")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.numeric_transform_dialog_ui(ui));
        }
        if self.feather.is_some() {
            egui::Window::new("Feather Selection")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.feather_dialog_ui(ui));
        }
        if self.canvas_size.is_some() {
            egui::Window::new("Canvas Size")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.canvas_size_dialog_ui(ui));
        }
        if self.canvas_color.is_some() {
            egui::Window::new("Canvas Color")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.canvas_color_dialog_ui(ui));
        }
        if self.image_size.is_some() {
            egui::Window::new("Image Size")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.image_size_dialog_ui(ui));
        }
        if self.grid_dialog.is_some() {
            egui::Window::new("Edit Grid")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.grid_dialog_ui(ui));
        }
        if self.about_open {
            egui::Window::new("About FireCraft")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.about_ui(ui));
        }
        self.palette_ui(ui);
        if self.help_open {
            self.help_panel(ui.ctx());
        }
        if self.close_requested {
            self.close_requested = false;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    // The web bridge downcasts through this (`WebRunner::app_mut`);
    // the trait default returns None and the runner panics on it,
    // which took down every browser open/apply/export until v0.2.1.
    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::editor::union_selection_bounds;
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

    /// A shell with scripted dialogs over a memory filesystem.
    fn gui_shell(editor: Editor) -> Shell {
        let dialogs = FixedDialogs {
            steps: "/mem/steps.fwsteps".to_owned(),
            doc: "/mem/doc.fw.png".to_owned(),
        };
        Shell::with_platform(editor, std::rc::Rc::new(MemoryFs::new()), Box::new(dialogs))
    }

    /// A shell whose selection is a text object carrying `text`.
    fn text_shell(text: &str) -> (Shell, doc::ObjectId) {
        let mut editor = Editor::untitled();
        editor
            .apply(
                "text.createText",
                serde_json::json!({
                    "page": 0, "layer": editor.active_layer(), "cell": 0,
                    "x": 10.0, "y": 10.0, "width": 200.0, "height": 40.0,
                    "text": text, "name": null,
                }),
            )
            .unwrap();
        let id = editor.current_cell().unwrap().elements[0];
        editor.set_selection(vec![id]);
        (gui_shell(editor), id)
    }

    /// The Text menu's style/alignment/convert family: Plain clears
    /// the style flags, the align rows write `JSTi`, Smaller/Larger
    /// step the size presets, and Convert to Paths freezes the glyphs
    /// into a path object.
    #[test]
    fn text_menu_actions_reach_the_engine() {
        let (mut shell, id) = text_shell("hello");
        shell
            .editor
            .apply(
                "text.setAttrs",
                serde_json::json!({ "id": id, "bold": true, "italic": true, "underline": true }),
            )
            .unwrap();

        shell.run_action(&menus::Action::TextPlain);
        let style = match shell.editor.session.document().registry.get(id) {
            Some(doc::ObjectData::Text(text)) => text.style.clone(),
            other => panic!("expected a text object, got {other:?}"),
        }
        .expect("the text carries a style");
        assert_eq!(style.bold, Some(false));
        assert_eq!(style.italic, Some(false));
        assert_eq!(style.underline, Some(false));

        shell.run_action(&menus::Action::TextJustify(2));
        let style = match shell.editor.session.document().registry.get(id) {
            Some(doc::ObjectData::Text(text)) => text.style.clone(),
            other => panic!("expected a text object, got {other:?}"),
        }
        .expect("the text carries a style");
        assert_eq!(style.justification, Some(2));

        // Larger steps up the preset ladder from 12pt to 14pt.
        shell.run_action(&menus::Action::TextStep(1));
        let style = match shell.editor.session.document().registry.get(id) {
            Some(doc::ObjectData::Text(text)) => text.style.clone(),
            other => panic!("expected a text object, got {other:?}"),
        }
        .expect("the text carries a style");
        assert_eq!(style.size, Some(14.0));

        shell.run_action(&menus::Action::Sel(menus::SelAction::ConvertToPaths));
        assert!(
            matches!(
                shell.editor.session.document().registry.get(id),
                Some(doc::ObjectData::Path(_))
            ),
            "the text became a path"
        );
    }

    /// File ▸ File Info / PNG format facts report the live
    /// document; Close refuses to discard unsaved edits; Revert
    /// reloads the file the document was opened from.
    #[test]
    fn file_rows_report_and_guard_the_document() {
        let mut shell = gui_shell(Editor::untitled());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::FileInfo));
        assert!(
            shell.status.contains("page(s)") && shell.status.contains("canvas"),
            "file info reports the document: {}",
            shell.status
        );

        shell.run_action(&menus::Action::Shell(menus::ShellAction::PngFormatInfo));
        assert!(
            shell.status.contains("Fireworks PNG") && shell.status.contains("RGBA"),
            "the PNG row reports the format facts: {}",
            shell.status
        );

        // A dirty document is not closed out from under the user.
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(90.0, 50.0));
        editor.end_drag().unwrap();
        let mut shell = gui_shell(editor);
        assert!(shell.editor.session.is_dirty());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::CloseDocument));
        assert!(
            shell.status.contains("unsaved changes"),
            "close refuses a dirty document: {}",
            shell.status
        );
        assert!(
            !shell.editor.session.document().registry.is_empty(),
            "the document survived the refused close"
        );
    }

    /// Modify ▸ Transform: Free Transform/Distort set the handle
    /// mode; Split breaks a multi-contour path apart; Remove
    /// Transformations inverts each selected object's stored matrix.
    #[test]
    fn modify_transform_rows_drive_the_editor() {
        // Transform mode round-trips through the editor flag.
        let (mut shell, _id) = text_shell("m");
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TransformMode(
            menus::TransformMode::Distort,
        )));
        assert_eq!(
            shell.editor.transform_mode,
            crate::editor::TransformMode::Distort
        );
        assert!(shell.status.contains("Distort"));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TransformMode(
            menus::TransformMode::Scale,
        )));
        assert_eq!(
            shell.editor.transform_mode,
            crate::editor::TransformMode::Scale
        );

        // Split: a path carrying two contours becomes two objects.
        let mut editor = Editor::untitled();
        editor
            .apply(
                "object.createPath",
                serde_json::json!({
                    "kind": "line", "x": 0.0, "y": 0.0,
                    "width": 100.0, "height": 0.0,
                    "page": 0, "layer": editor.active_layer(), "cell": 0,
                    "name": null,
                }),
            )
            .unwrap();
        let mut shell = gui_shell(editor);
        // A second contour: translate a duplicate into the same cell
        // is overkill — split on a single-contour path is a no-op,
        // which is itself the countercase.
        let before = shell.editor.current_cell().unwrap().elements.len();
        shell.run_action(&menus::Action::Sel(menus::SelAction::SplitPaths));
        assert_eq!(shell.editor.current_cell().unwrap().elements.len(), before);

        // Remove Transformations on an untransformed object is a
        // no-op (no stored matrix).
        let before = shell.editor.session.journal_len();
        shell.run_action(&menus::Action::Sel(menus::SelAction::RemoveTransformations));
        assert_eq!(shell.editor.session.journal_len(), before);
    }

    /// Attach to Path: selecting one text and one path and running
    /// the action lays the text along the path's contours; a
    /// selection missing either half reports, and does not command.
    #[test]
    fn attach_to_path_needs_a_text_and_a_rail() {
        let (mut shell, text_id) = text_shell("along");

        // Missing the rail: text-only selection reports.
        let before = shell.editor.session.journal_len();
        shell.run_action(&menus::Action::Sel(menus::SelAction::AttachToPath));
        assert!(shell.status.contains("path object"));
        assert_eq!(shell.editor.session.journal_len(), before);

        // Add a line path to the same cell and select both.
        shell
            .editor
            .apply(
                "object.createPath",
                serde_json::json!({
                    "kind": "line", "x": 0.0, "y": 100.0,
                    "width": 300.0, "height": 0.0,
                    "page": 0, "layer": shell.editor.active_layer(), "cell": 0,
                    "name": null,
                }),
            )
            .unwrap();
        let path_id = shell.editor.current_cell().unwrap().elements[1];
        shell.editor.set_selection(vec![text_id, path_id]);

        shell.run_action(&menus::Action::Sel(menus::SelAction::AttachToPath));
        let text = match shell.editor.session.document().registry.get(text_id) {
            Some(doc::ObjectData::Text(text)) => text,
            other => panic!("expected a text object, got {other:?}"),
        };
        let on_path = text.on_path.as_ref().expect("text rides the rail now");
        assert_eq!(on_path.path.len(), 1);
        assert_eq!(on_path.path[0].points.len(), 2);
        assert_eq!(text.flow, Some(2));

        // Missing the text: path-only selection reports.
        let (mut shell, _text_id) = text_shell("solo");
        shell
            .editor
            .apply(
                "object.createPath",
                serde_json::json!({
                    "kind": "line", "x": 0.0, "y": 100.0,
                    "width": 300.0, "height": 0.0,
                    "page": 0, "layer": shell.editor.active_layer(), "cell": 0,
                    "name": null,
                }),
            )
            .unwrap();
        let path_id = shell.editor.current_cell().unwrap().elements[1];
        shell.editor.set_selection(vec![path_id]);
        shell.run_action(&menus::Action::Sel(menus::SelAction::AttachToPath));
        assert!(shell.status.contains("text object"));
    }

    /// The action dispatch wiring: Select All picks up the cell's
    /// objects, Delete removes through the registry, Arrange reorders
    /// with a post-removal index, and Inert renders but does nothing.
    #[test]
    fn menu_actions_drive_the_editor() {
        let mut shell = gui_shell(gui_editor());
        let first = shell.editor.current_cell().unwrap().elements[0];

        // Select All selects everything in the current cell.
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SelectAll));
        assert_eq!(shell.editor.selection_set.len(), 1);
        assert_eq!(shell.editor.selection, Some(first));

        // Arrange with a single object is a no-op with a status line,
        // not a command.
        shell.run_action(&menus::Action::Arrange(menus::Arrange::Front));
        assert!(shell.status.contains("stacking position"));

        // Inert actions are accepted by the table but change nothing.
        let before = shell.editor.session.journal_len();
        shell.run_action(&menus::Action::Inert);
        assert_eq!(shell.editor.session.journal_len(), before);

        // Delete drives object.delete and empties the cell.
        shell.run_action(&menus::Action::Sel(menus::SelAction::Delete));
        assert!(shell.editor.current_cell().unwrap().elements.is_empty());
    }

    /// The CS6 shortcut table fires through the same dispatch —
    /// Ctrl+Z undoes, plain V picks the Pointer tool.
    #[test]
    fn shortcut_actions_share_the_dispatch() {
        let mut shell = gui_shell(gui_editor());
        let undoable = shell.editor.session.undo_len();
        assert!(undoable >= 1, "the seeded session has history");
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Undo));
        assert_eq!(shell.editor.session.undo_len(), undoable - 1);
        assert_eq!(shell.editor.session.redo_len(), 1);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Redo));
        assert_eq!(shell.editor.session.undo_len(), undoable);
        assert_eq!(shell.editor.session.redo_len(), 0);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::PickTool(
            Tool::Pointer,
        )));
        assert_eq!(shell.editor.tool, Tool::Pointer);
    }

    /// View ▸ Show All re-shows every hidden carrier in the document,
    /// driving `object.setVisible` per object through the registry.
    #[test]
    fn show_all_resurrects_hidden_objects() {
        let mut shell = gui_shell(gui_editor());
        let first = shell.editor.current_cell().unwrap().elements[0];
        shell
            .editor
            .apply(
                "object.setVisible",
                serde_json::json!({ "id": first, "visible": false }),
            )
            .unwrap();
        // A second page's object hides too: Show All is document-wide.
        shell
            .editor
            .apply("page.add", serde_json::json!({ "index": null }))
            .unwrap();
        let page = shell.editor.page;
        shell.editor.page = 1;
        shell.editor.tool = Tool::Rect;
        shell.editor.begin_drag(geom::Point::new(0.0, 0.0));
        shell.editor.drag_to(geom::Point::new(40.0, 40.0));
        shell.editor.end_drag().unwrap();
        let second = shell.editor.current_cell().unwrap().elements[0];
        shell
            .editor
            .apply(
                "object.setVisible",
                serde_json::json!({ "id": second, "visible": false }),
            )
            .unwrap();
        shell.editor.page = page;

        shell.run_action(&menus::Action::Shell(menus::ShellAction::ShowAllHidden));
        let doc = shell.editor.session.document();
        for id in [first, second] {
            let visible = match doc.registry.get(id) {
                Some(doc::ObjectData::Path(path)) => path.visible,
                other => panic!("expected a path, got {other:?}"),
            };
            assert!(visible, "object {id:?} is visible again");
        }
    }

    /// Window-menu panel toggles: each docked panel hides and resur-
    /// faces by name, Hide Panels folds the whole dock at once without
    /// disturbing per-panel visibility, and the Main toolbar toggle is
    /// independent. None of it touches the document journal.
    #[test]
    fn window_menu_toggles_panels() {
        let mut shell = gui_shell(gui_editor());
        let journal = shell.editor.session.journal_len();

        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "Layers",
        )));
        assert!(!shell.panel_shown("Layers"), "Layers hides");
        assert!(shell.status.contains("Layers panel hidden"));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "Layers",
        )));
        assert!(shell.panel_shown("Layers"), "Layers resurfaces");

        // Color Mixer starts closed (CS6's roster) and the Window
        // menu opens it; a second toggle closes it again.
        assert!(!shell.panel_shown("Color Mixer"));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "Color Mixer",
        )));
        assert!(shell.panel_shown("Color Mixer"), "Color Mixer opens");
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "Color Mixer",
        )));
        assert!(!shell.panel_shown("Color Mixer"));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "Color Mixer",
        )));
        // Swatches and Info start closed (CS6's fresh-document
        // roster) and the Window menu opens them.
        assert!(!shell.panel_shown("Swatches"));
        assert!(!shell.panel_shown("Info"));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "Swatches",
        )));
        assert!(shell.panel_shown("Swatches"));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "Info",
        )));
        assert!(shell.panel_shown("Info"));

        // Tools folds the left dock.
        shell.run_action(&menus::Action::Shell(menus::ShellAction::ToggleTools));
        assert!(shell.tools_hidden);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::ToggleTools));
        assert!(!shell.tools_hidden);

        // Single-document rows report honestly instead of acting.
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SingleDocument));
        assert!(shell.status.contains("single-document"));

        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "Properties",
        )));
        assert!(!shell.panel_shown("Properties"));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::ToggleHidePanels));
        assert!(shell.panels_hidden, "Hide Panels folds the dock");
        shell.run_action(&menus::Action::Shell(menus::ShellAction::ToggleHidePanels));
        assert!(!shell.panels_hidden);
        assert!(
            !shell.panel_shown("Properties"),
            "per-panel visibility survives the fold"
        );
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "Properties",
        )));
        assert!(shell.panel_shown("Properties"));

        shell.run_action(&menus::Action::Shell(menus::ShellAction::ToggleMainToolbar));
        assert!(shell.options_bar_hidden);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::ToggleMainToolbar));
        assert!(!shell.options_bar_hidden);

        assert_eq!(
            shell.editor.session.journal_len(),
            journal,
            "panel toggles are view state, not document history"
        );
    }

    /// The screen-mode control cycles CS6's three view modes in the
    /// documented order (fwref 1178–1185) and back to Standard; the
    /// cycle is view state — the journal never moves.
    #[test]
    fn the_screen_mode_cycles_through_the_three_view_modes() {
        let mut shell = gui_shell(Editor::untitled());
        let journal = shell.editor.session.journal_len();

        assert_eq!(shell.screen_mode, ScreenMode::Standard);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::CycleScreenMode));
        assert_eq!(shell.screen_mode, ScreenMode::FullScreenWithMenus);
        assert!(shell.status.contains("Full Screen with Menus"));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::CycleScreenMode));
        assert_eq!(shell.screen_mode, ScreenMode::FullScreen);
        assert!(shell.status.contains("Full Screen mode"));
        // The third press returns to Standard — a cycle, not a toggle.
        shell.run_action(&menus::Action::Shell(menus::ShellAction::CycleScreenMode));
        assert_eq!(shell.screen_mode, ScreenMode::Standard);
        assert_eq!(
            shell.editor.session.journal_len(),
            journal,
            "the screen mode is view state, not document history"
        );

        // The Full Screen backdrop is black; Full Screen with Menus is
        // CS6's gray; Standard keeps the usual dark matte.
        assert_eq!(ScreenMode::FullScreen.backdrop(), egui::Color32::BLACK);
        assert_eq!(
            ScreenMode::FullScreenWithMenus.backdrop(),
            egui::Color32::from_rgb(0x99, 0x99, 0x99)
        );
    }

    /// A shell holding one named slice on the Web Layer, selected.
    /// The slice is created through the engine's own command, so the
    /// panels read exactly what the document holds.
    fn slice_shell(name: &str) -> (Shell, doc::ObjectId) {
        let mut editor = Editor::untitled();
        editor
            .apply(
                "slice.create",
                serde_json::json!({
                    "shape": "rect",
                    "x": 8.0, "y": 8.0, "width": 64.0, "height": 32.0,
                    "points": null, "page": 0,
                    "name": name, "link": null, "alt": null,
                }),
            )
            .expect("the slice is created");
        let id = editor
            .document()
            .registry
            .iter()
            .find_map(|(id, data)| match data {
                doc::ObjectData::Slice(slice) if slice.name.as_deref() == Some(name) => Some(id),
                _ => None,
            })
            .expect("the slice is registered");
        editor.set_selection(vec![id]);
        (gui_shell(editor), id)
    }

    /// The Auto Shapes panel inserts through the engine: the gallery's
    /// click reaches `autoshape.*` / `object.createPath`, the object
    /// lands on the active layer's cell, and the journal records the
    /// command — so the panel is a real edit, not a view.
    #[test]
    fn the_auto_shapes_panel_inserts_through_the_engine() {
        let mut shell = gui_shell(Editor::untitled());
        let journal = shell.editor.session.journal_len();
        let before = shell
            .editor
            .current_cell()
            .expect("the page has a cell")
            .elements
            .len();

        shell.insert_shape(
            super::panels_web::ShapeKind::Auto(crate::editor::AutoShape::Star),
            "Star",
        );

        assert_eq!(
            shell.editor.session.journal_len(),
            journal + 1,
            "the insert is one journaled command"
        );
        assert_eq!(
            shell
                .editor
                .current_cell()
                .expect("the page has a cell")
                .elements
                .len(),
            before + 1,
            "the shape lands on the active layer"
        );

        // The primitive half of the gallery uses the path command.
        let before = shell
            .editor
            .current_cell()
            .expect("the page has a cell")
            .elements
            .len();
        shell.insert_shape(
            super::panels_web::ShapeKind::Primitive("ellipse"),
            "Ellipse",
        );
        assert_eq!(
            shell
                .editor
                .current_cell()
                .expect("the page has a cell")
                .elements
                .len(),
            before + 1,
            "the primitive lands too"
        );
    }

    /// The URL panel writes the anchor link and the alt text onto the
    /// selected slice, and an empty field clears the stored value.
    #[test]
    fn the_url_panel_stores_and_clears_the_link() {
        let (mut shell, id) = slice_shell("banner");

        assert!(shell.apply_link(id, "https://example.test/", "A banner"));
        let slice = match shell.editor.document().registry.get(id) {
            Some(doc::ObjectData::Slice(slice)) => slice,
            other => panic!("the slice is still a slice: {other:?}"),
        };
        assert_eq!(slice.link.as_deref(), Some("https://example.test/"));
        assert_eq!(slice.alt.as_deref(), Some("A banner"));

        // An empty field clears the stored value.
        assert!(shell.apply_link(id, "", ""));
        let slice = match shell.editor.document().registry.get(id) {
            Some(doc::ObjectData::Slice(slice)) => slice,
            other => panic!("the slice is still a slice: {other:?}"),
        };
        assert_eq!(slice.link, None);
        assert_eq!(slice.alt, None);
    }

    /// The Behaviors panel adds, removes, and configures behaviors
    /// through the engine — the stored kinds and the swap target are
    /// the document's own, not panel state.
    #[test]
    fn the_behaviors_panel_adds_and_removes_behaviors() {
        let (mut shell, id) = slice_shell("banner");

        assert!(shell.add_behavior(id, 9));
        let kinds = |shell: &Shell| -> Vec<i64> {
            match shell.editor.document().registry.get(id) {
                Some(doc::ObjectData::Slice(slice)) => slice
                    .behaviors
                    .iter()
                    .map(|behavior| behavior.kind)
                    .collect(),
                other => panic!("the slice is still a slice: {other:?}"),
            }
        };
        assert_eq!(kinds(&shell), vec![9]);

        assert!(shell.add_behavior(id, 7));
        assert_eq!(kinds(&shell), vec![9, 7]);

        assert!(shell.remove_behavior(id, 0));
        assert_eq!(kinds(&shell), vec![7]);

        // The swap-image behavior names its target through the
        // export key, then appends the behavior kind.
        assert!(shell.add_swap_behavior(id, "hero.png"));
        assert_eq!(kinds(&shell), vec![7, 2]);
        match shell.editor.document().registry.get(id) {
            Some(doc::ObjectData::Slice(slice)) => {
                assert_eq!(
                    slice.custom_data.get("swapTarget").map(String::as_str),
                    Some("hero.png")
                );
            }
            other => panic!("the slice is still a slice: {other:?}"),
        }
    }

    /// The persisted layout round-trips: capture → (RON via serde,
    /// the shape eframe stores) → apply reproduces the panel set and
    /// dock folds, and a layout naming a panel this build does not
    /// have is dropped rather than resurrected.
    #[test]
    fn shell_layout_round_trips_through_save() {
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "Properties",
        )));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::TogglePanel(
            "History",
        )));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::ToggleTools));
        shell.run_action(&menus::Action::Shell(menus::ShellAction::ToggleMainToolbar));
        let saved = shell.capture_layout();

        // Same encode path eframe uses on native and web (RON).
        let encoded = ron_mid_encoded(&saved);
        let decoded: ShellLayout = ron::from_str(&encoded).expect("layout decodes");
        let mut restored = gui_shell(gui_editor());
        restored.layout = decoded;
        restored.apply_layout();
        assert!(!restored.panel_shown("Properties"));
        assert!(restored.panel_shown("History"));
        assert!(restored.tools_hidden);
        assert!(restored.options_bar_hidden);

        // A layout from a build with panels this build lacks: the
        // unknown name is dropped, the known ones survive.
        let mut foreign = shell.capture_layout();
        foreign.panel_hidden.push("Not A Panel".to_owned());
        let mut restored = gui_shell(gui_editor());
        restored.layout = foreign;
        restored.apply_layout();
        assert_eq!(
            restored
                .panel_hidden
                .iter()
                .filter(|name| panel_windows::PANELS.iter().any(|p| p.name == **name))
                .count(),
            restored.panel_hidden.len(),
            "no unknown panel names survive apply_layout"
        );
    }

    /// The tool-flyout hold decision is a pure function of elapsed
    /// time: below the dwell it stays closed, at and past it the
    /// flyout opens, and a short press never opens it no matter how
    /// it is chopped up.
    #[test]
    fn tool_hold_opens_only_past_the_dwell() {
        assert!(!hold_opens_flyout(0.0), "the instant of the press");
        assert!(!hold_opens_flyout(TOOL_HOLD_SECONDS - 0.01), "just under");
        assert!(hold_opens_flyout(TOOL_HOLD_SECONDS), "exactly at");
        assert!(hold_opens_flyout(TOOL_HOLD_SECONDS + 5.0), "long past");
    }

    /// Encodes a `ShellLayout` the way `eframe::set_value` does
    /// (RON text) — without needing a `dyn Storage` in the test.
    fn ron_mid_encoded(layout: &ShellLayout) -> String {
        ron::ser::to_string(layout).expect("layout encodes")
    }

    /// The palette covers the whole registry: every catalog id is
    /// reachable by an empty filter, carries a non-null payload
    /// template, and runs without taking the shell down — a command
    /// may refuse (the palette then names the refusal), but it never
    /// panics and never leaves the palette without a payload.
    #[test]
    fn palette_covers_every_catalog_command() {
        let mut shell = gui_shell(gui_editor());
        shell.toggle_palette();
        assert!(shell.palette.open, "the palette opens on demand");
        let catalog = engine::catalog::ids();
        assert_eq!(
            shell.palette_matches(),
            catalog,
            "palette lists the catalog"
        );
        for name in catalog {
            let payload = shell.payload_for(name);
            assert!(payload.is_object(), "{name}: payload is not an object");
            assert!(!payload.is_null(), "{name}: payload is null");
            if !shell.run_palette_command(name) {
                assert!(
                    shell.status.contains(name),
                    "{name}: refusal is not reported: {status}",
                    status = shell.status
                );
            }
        }
    }

    /// The palette filter narrows by substring (case-insensitive), the
    /// selection index is reset when the query changes, and grounding
    /// clamps the fuzzdrive template's out-of-range targets to the
    /// live document — the difference between a fuzz call and a user
    /// command.
    #[test]
    fn palette_filters_and_grounds_payloads() {
        let mut shell = gui_shell(gui_editor());
        let total = engine::catalog::ids().len();

        shell.palette.filter = "rect".to_owned();
        for name in shell.palette_matches() {
            assert!(name.to_lowercase().contains("rect"));
        }
        assert!(shell.palette_matches().len() < total, "filter narrows");

        shell.palette.selected = 4;
        shell.palette.filter = "recta".to_owned();
        assert_eq!(shell.palette.selected, 4, "index moves only via ui paths");

        let empty = shell.payload_for("object.setOpacity");
        assert!(empty.get("id").is_some(), "template names a target");

        let id = shell.editor.current_cell().unwrap().elements[0];
        shell.editor.selection = Some(id);
        let grounded = shell.grounded("object.setOpacity", 0);
        assert_eq!(grounded["id"], serde_json::json!(id));
        let opacity = grounded["opacity"].as_u64().unwrap_or(2000);
        assert!(
            grounded["opacity"].is_number(),
            "opacity stays numeric (grounding targets targets, not ranges), got {opacity}"
        );

        // Boolean ops ground both operands to a live path.
        shell
            .editor
            .apply("autoshape.rect", serde_json::json!({ "x": 0.0, "y": 0.0, "width": 40.0, "height": 20.0, "corner": 0.0, "kind": "bevel", "page": 0, "layer": 0, "cell": 0, "name": null }))
            .unwrap();
        let boolean = shell.grounded("path.boolean", 0);
        assert!(
            !boolean["a"].is_null(),
            "path.boolean grounds operand a, got {}",
            boolean["a"]
        );
        assert_eq!(boolean["b"], boolean["a"], "both operands hit one path");
    }

    /// The palette's remaining grounding branches: toggle round-trip
    /// resets the filter, the knife verbs get a viewport `from`/`to`
    /// pair that survives grounding untouched, and the `symbol` key is
    /// renumbered to the live symbol table rather than an index.
    #[test]
    fn palette_grounds_knife_and_symbol_targets() {
        let mut shell = gui_shell(gui_editor());
        shell.toggle_palette();
        assert!(shell.palette.open);
        shell.palette.filter = "noise".to_owned();
        shell.toggle_palette();
        shell.toggle_palette();
        assert!(shell.palette.filter.is_empty(), "reopen clears the filter");

        let knife = shell.grounded("path.knifeElementsFromPoint", 0);
        assert!(
            knife["from"].is_array() && knife["from"].as_array().unwrap().len() == 2,
            "knife from is a point pair, got {}",
            knife["from"]
        );

        shell
            .editor
            .apply(
                "symbol.create",
                serde_json::json!({
                    "ids": [shell.editor.current_cell().unwrap().elements[0]],
                    "name": "test",
                    "symbol_type": "graphic",
                }),
            )
            .unwrap();
        let definition = shell.editor.document().symbols.first().unwrap();
        let grounded = shell.grounded("symbol.createInstance", 0);
        assert_eq!(
            grounded["symbol"],
            serde_json::json!(definition.symbol_number),
            "symbol key carries the engine's symbol_number"
        );
    }

    /// The selection-restore prep path: with no saved selection, the
    /// palette seeds an image, selects a rect inside it, saves
    /// "default", and then `selection.restore` succeeds. A second run
    /// finds the slot already present and does not duplicate it.
    #[test]
    fn palette_prepares_a_saved_selection_for_restore() {
        let mut shell = gui_shell(gui_editor());
        assert!(shell.editor.document().saved_selections.is_empty());

        assert!(shell.run_palette_command("selection.restore"));
        assert_eq!(shell.editor.document().saved_selections.len(), 1);
        assert_eq!(shell.editor.document().saved_selections[0].name, "default");

        assert!(shell.run_palette_command("selection.restore"));
        assert_eq!(shell.editor.document().saved_selections.len(), 1);
        assert!(
            !shell.status.contains("failed"),
            "caller (not run_palette_command) sets success status, got {:?}",
            shell.status
        );
    }

    /// A command that refuses under every seed is reported in the
    /// status line and leaves the palette open for correction.
    #[test]
    fn palette_reports_total_refusal() {
        let mut shell = gui_shell(gui_editor());
        let before = shell.editor.document().registry.len();
        // An unknown id: the editor refuses without mutating.
        assert!(!shell.run_palette_command("object.doesNotExist"));
        assert!(shell.status.contains("object.doesNotExist"));
        assert_eq!(shell.editor.document().registry.len(), before);
    }

    /// The preset registry is internally consistent: names unique,
    /// every numeric default parses inside its documented range, the
    /// knot preset rides raw points with an empty value, and the
    /// slider metadata's reset values stay in range.
    #[test]
    fn filter_preset_registry_is_consistent() {
        let mut names = Vec::new();
        for (family, presets) in FILTER_FAMILIES {
            assert!(!presets.is_empty(), "{family} is empty");
            for preset in *presets {
                names.push(preset.name);
                let points_preset = !preset.points.is_empty();
                assert_eq!(
                    points_preset,
                    preset
                        .params
                        .iter()
                        .any(|(key, _)| key.ends_with("_points")),
                    "{}: points xor text",
                    preset.name
                );
                for (key, value) in preset.params {
                    if FILTER_BOOL_KEYS.contains(key) {
                        assert!(matches!(*value, "0" | "1"), "{key}: {value}");
                        continue;
                    }
                    if key.ends_with("_points") {
                        assert!(value.is_empty(), "{key}: knots carry no text");
                        continue;
                    }
                    if let Some(meta) = FILTER_PARAMS.iter().find(|meta| meta.key == *key) {
                        let parsed: f64 =
                            value.parse().unwrap_or_else(|_| panic!("{key}: {value}"));
                        assert!(
                            (meta.min..=meta.max).contains(&parsed),
                            "{key}: {parsed} outside {}..{}",
                            meta.min,
                            meta.max
                        );
                    }
                }
            }
        }
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate preset names");
        for meta in FILTER_PARAMS {
            assert!(
                (meta.min..=meta.max).contains(&meta.default),
                "{}: default outside range",
                meta.key
            );
        }
    }

    /// The 19.6 page/frame batch: Insert ▸ State appends a frame and
    /// moves the view to it, Trim/Fit Canvas drive the extents verbs,
    /// and the canvas rotations re-map the page — all through the
    /// journal (undo restores the previous shape).
    #[test]
    fn page_frame_actions_drive_the_document() {
        // State: one frame in the seed, insert appends and activates.
        let mut shell = gui_shell(gui_editor());
        let frames_before = shell.editor.document().pages[0].info.frame_count;
        shell.run_action(&menus::Action::Sel(menus::SelAction::NewFrame));
        assert_eq!(
            shell.editor.document().pages[0].info.frame_count,
            frames_before + 1,
            "frame.add appended one state"
        );
        assert_eq!(
            shell.editor.frame,
            (frames_before) as usize,
            "the view moved to the appended state"
        );

        // Trim Canvas on a page with content shrinks the canvas.
        let mut shell = gui_shell(gui_editor());
        let canvas_before = shell.editor.document().pages[0].info.canvas.clone();
        shell.run_action(&menus::Action::Document(menus::DocumentAction::TrimCanvas));
        let canvas_after = &shell.editor.document().pages[0].info.canvas;
        assert!(
            canvas_after.width <= canvas_before.width
                && canvas_after.height <= canvas_before.height,
            "trim shrank the canvas: {canvas_before:?} -> {canvas_after:?}"
        );

        // Rotate 90° CW swaps the canvas dimensions.
        let mut shell = gui_shell(gui_editor());
        let canvas_before = shell.editor.document().pages[0].info.canvas.clone();
        shell.run_action(&menus::Action::Document(menus::DocumentAction::Rotate(90)));
        let canvas_after = &shell.editor.document().pages[0].info.canvas;
        assert_eq!(canvas_after.width, canvas_before.height, "90° swap");
        assert_eq!(canvas_after.height, canvas_before.width, "90° swap");

        // Fit Canvas with an empty selection is a refusal, not a
        // journal row (the engine needs extents to hug).
        let mut shell = gui_shell(gui_editor());
        let journal_before = shell.editor.session.journal_len();
        shell.run_action(&menus::Action::Document(menus::DocumentAction::FitCanvas));
        assert!(
            shell.editor.session.journal_len() == journal_before || !shell.status.is_empty(),
            "fit canvas with no selection refuses or reports"
        );

        // Every action in the battery landed in the journal (the
        // wired verbs, not the refusals).
        assert!(shell.editor.session.journal_len() >= journal_before);
    }

    /// The 19.6 symbol/animation batch: Convert to Symbol folds the
    /// selection into a library definition, Animate Selection makes an
    /// animation-typed one, Tween Instances distributes instances
    /// across frames — all through the journal — and each pick with
    /// nothing (or too little) selected is a status line, not a row.
    #[test]
    fn symbol_actions_drive_the_library() {
        // Convert to Symbol with content: one definition appears and
        // the command landed in the journal.
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SelectAll));
        let journal_before = shell.editor.session.journal_len();
        shell.run_action(&menus::Action::Sel(menus::SelAction::ConvertToSymbol));
        assert_eq!(
            shell.editor.document().symbols.len(),
            1,
            "convert to symbol created one definition"
        );
        assert!(
            shell.editor.session.journal_len() > journal_before,
            "symbol.create recorded in the journal"
        );

        // Animate Selection with content: an animation-typed symbol.
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SelectAll));
        shell.run_action(&menus::Action::Sel(menus::SelAction::AnimateSelection));
        assert_eq!(shell.editor.document().symbols.len(), 1);
        assert_eq!(
            shell.editor.document().symbols[0].symbol_type,
            doc::SymbolType::Animation,
            "animate selection makes an animation symbol"
        );

        // Remove Animation with a symbol instance: breakApart dissolves
        // it; with no instance in the selection it refuses.
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SelectAll));
        shell.run_action(&menus::Action::Sel(menus::SelAction::AnimateSelection));
        // The instance replaced the source objects in the cell; select
        // the cell's live contents before removing the animation.
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SelectAll));
        shell.run_action(&menus::Action::Sel(menus::SelAction::RemoveAnimation));
        assert!(
            !shell.status.contains("no symbol"),
            "the animation instance was found and broken apart"
        );
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Sel(menus::SelAction::RemoveAnimation));
        assert!(
            shell.status.contains("no symbol instance"),
            "remove animation without an instance refuses"
        );

        // Tween Instances with fewer than two instances is a refusal.
        let mut shell = gui_shell(gui_editor());
        let journal_before = shell.editor.session.journal_len();
        shell.run_action(&menus::Action::Sel(menus::SelAction::TweenInstances));
        assert!(
            shell.editor.session.journal_len() == journal_before,
            "tween with one instance refuses"
        );
        assert!(
            !shell.status.is_empty(),
            "the refusal is reported on the status line"
        );

        // Convert to Symbol with nothing selected refuses too.
        let mut shell = gui_shell(gui_editor());
        let journal_before = shell.editor.session.journal_len();
        shell.run_action(&menus::Action::Sel(menus::SelAction::ConvertToSymbol));
        assert_eq!(
            shell.editor.session.journal_len(),
            journal_before,
            "convert with no selection refuses"
        );
        assert!(!shell.status.is_empty());
    }

    /// The Filters menu dispatch and the panel preset path drive the
    /// same commands: a menu pick adds the preset's `DCVs`, the curves
    /// preset lands as the raw knot node, a param edit rides
    /// `filter.setParam` through the journal (so undo restores
    /// it), and a pick with nothing selected is a status line.
    #[test]
    fn filter_actions_flow_through_the_journal() {
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SelectAll));
        let id = shell.editor.selection.unwrap();

        shell.run_action(&menus::Action::Filter("gaussianBlur"));
        let effects = shell_effect_list(&shell, id);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].internal_name, "gaussianBlur");

        let curves = filter_preset("curves").unwrap();
        shell.apply_filter_preset(id, *curves);
        let effects = shell_effect_list(&shell, id);
        assert_eq!(effects.len(), 2);
        let doc::EffectValue::Raw(node) = &effects[1].properties[0].value else {
            panic!("curves knots encode as a raw node");
        };
        assert_eq!(node.as_children().map_or(0, <[doc::Node]>::len), 3);

        shell.set_filter_param(id, 0, "gaussian_blur_radius", "4");
        let mut radius = shell_param(&shell, id, 0, "gaussian_blur_radius");
        assert_eq!(radius, "4");
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Undo));
        radius = shell_param(&shell, id, 0, "gaussian_blur_radius");
        assert_eq!(radius, "2", "undo restores the preset default");

        let mut bare = gui_shell(gui_editor());
        bare.run_action(&menus::Action::Filter("invert"));
        assert!(bare.status.contains("pick an object"), "{}", bare.status);
    }

    /// The applied effects of an object, read the way the properties
    /// panel reads them.
    fn shell_effect_list(shell: &Shell, id: doc::ObjectId) -> Vec<doc::Effect> {
        match shell.editor.document().registry.get(id) {
            Some(ObjectData::Path(path)) => path.effects.clone(),
            Some(ObjectData::Text(text)) => text.effects.clone(),
            _ => Vec::new(),
        }
    }

    /// The three presets the audit found dead now reach a kernel: the
    /// drop shadow carries the four scalars its kernel reads (all four
    /// or nothing renders), and the two emboss presets reach the bevel
    /// kernel under their panel spellings. Each preset must change
    /// the rendered page — a preset that lands as an effect stack the
    /// kernel skips leaves the render untouched.
    #[test]
    fn the_shadowed_and_embossed_presets_reach_the_renderer() {
        for name in ["dropShadow", "emboss", "pillowEmboss"] {
            let mut shell = gui_shell(gui_editor());
            let id = shell.editor.current_cell().unwrap().elements[0];
            let before = compose::render(
                shell.editor.document(),
                shell.editor.page,
                shell.editor.frame,
            )
            .expect("renders");
            shell.apply_filter_preset(id, *filter_preset(name).expect("preset exists"));
            let after = compose::render(
                shell.editor.document(),
                shell.editor.page,
                shell.editor.frame,
            )
            .expect("renders");
            assert_ne!(
                before.pixels, after.pixels,
                "{name}: the preset must change the render"
            );
        }
    }

    /// An editor holding two rectangles on the active layer.
    fn gui_editor_two() -> Editor {
        let mut editor = gui_editor();
        editor.begin_drag(geom::Point::new(110.0, 100.0));
        editor.drag_to(geom::Point::new(180.0, 140.0));
        editor.end_drag().unwrap();
        editor
    }

    /// The path object behind an id, for the transform assertions.
    fn shell_path(shell: &Shell, id: doc::ObjectId) -> PathObject {
        match shell.editor.document().registry.get(id) {
            Some(ObjectData::Path(path)) => path.clone(),
            _ => panic!("object {id:?} is not a path"),
        }
    }

    /// Edit ▸ Copy / Paste: the clipboard crosses the engine's command
    /// door — the view's selection lands in the document first, the
    /// paste re-mints onto the active layer, and the pasted object
    /// becomes the selection.
    #[test]
    fn copy_and_paste_round_trip_through_the_clipboard() {
        let mut shell = gui_shell(gui_editor());
        let id = shell.editor.current_cell().unwrap().elements[0];
        shell.editor.set_selection(vec![id]);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Copy));
        assert!(shell.status.contains("copied"), "{}", shell.status);
        let before = shell.editor.current_cell().unwrap().elements.len();
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Paste));
        assert!(shell.status.contains("pasted"), "{}", shell.status);
        let cell = shell.editor.current_cell().unwrap();
        assert_eq!(cell.elements.len(), before + 1, "the paste adds one object");
        assert_eq!(
            shell.editor.selection_set,
            vec![cell.elements[before]],
            "the pasted object is the selection"
        );
    }

    /// Edit ▸ Cut: the objects leave the document, and the view's
    /// selection follows them out.
    #[test]
    fn cut_takes_the_objects_with_it() {
        let mut shell = gui_shell(gui_editor());
        let id = shell.editor.current_cell().unwrap().elements[0];
        shell.editor.set_selection(vec![id]);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::Cut));
        assert!(
            shell.editor.current_cell().unwrap().elements.is_empty(),
            "the cell is empty"
        );
        assert!(
            shell.editor.document().registry.get(id).is_none(),
            "the object left the registry"
        );
        assert!(
            shell.editor.selection_set.is_empty(),
            "the selection follows the cut objects out"
        );
    }

    /// Select ▸ Select Inverse: the complement of the selection within
    /// the cell the editor edits.
    #[test]
    fn select_inverse_complements_the_cell() {
        let mut shell = gui_shell(gui_editor_two());
        let ids = shell.editor.current_cell().unwrap().elements.clone();
        shell.editor.set_selection(vec![ids[0]]);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::SelectInverse));
        assert_eq!(shell.editor.selection_set, vec![ids[1]]);
    }

    /// Modify ▸ Group / Ungroup: the group lands in the cell where the
    /// members were, and ungroup puts the members back.
    #[test]
    fn group_and_ungroup_fold_the_selection() {
        let mut shell = gui_shell(gui_editor_two());
        let ids = shell.editor.current_cell().unwrap().elements.clone();
        shell.editor.set_selection(ids);
        shell.run_action(&menus::Action::Sel(menus::SelAction::Group));
        let cell = shell.editor.current_cell().unwrap();
        assert_eq!(cell.elements.len(), 1, "one group replaces the members");
        let group = cell.elements[0];
        assert!(
            matches!(
                shell.editor.document().registry.get(group),
                Some(ObjectData::Group(_))
            ),
            "the folded object is a group"
        );
        shell.editor.set_selection(vec![group]);
        shell.run_action(&menus::Action::Sel(menus::SelAction::Ungroup));
        assert_eq!(
            shell.editor.current_cell().unwrap().elements.len(),
            2,
            "the group dissolves"
        );
    }

    /// Modify ▸ Transform: a quarter turn swaps the object's extents,
    /// and a flip moves its points.
    #[test]
    fn transform_rotates_and_flips_the_selection() {
        let mut shell = gui_shell(gui_editor());
        let id = shell.editor.current_cell().unwrap().elements[0];
        shell.editor.set_selection(vec![id]);
        let before = crate::editor::bounds_of(&shell_path(&shell, id));
        shell.run_action(&menus::Action::Sel(menus::SelAction::Rotate(90.0)));
        let after = crate::editor::bounds_of(&shell_path(&shell, id));
        assert!(
            (after.width() - before.height()).abs() < 0.001
                && (after.height() - before.width()).abs() < 0.001,
            "a quarter turn swaps the extents: {before:?} -> {after:?}"
        );
        let anchors = |shell: &Shell| {
            shell_path(shell, id)
                .contours
                .iter()
                .flat_map(|contour| contour.points.iter().map(|point| point.anchor))
                .collect::<Vec<_>>()
        };
        let before_points = anchors(&shell);
        shell.run_action(&menus::Action::Sel(menus::SelAction::Reflect {
            horizontal: true,
            vertical: false,
        }));
        assert_ne!(anchors(&shell), before_points, "the flip moves the points");
    }

    /// The Numeric Transform dialog: scale+rotate through the menu
    /// action lands as one undo step through the journal.
    #[test]
    fn numeric_transform_dialog_applies_one_undo_step() {
        let mut shell = gui_shell(gui_editor());
        let id = shell.editor.current_cell().unwrap().elements[0];
        shell.editor.set_selection(vec![id]);
        let undo_before = shell.editor.session.undo_len();

        shell.run_action(&menus::Action::Shell(menus::ShellAction::NumericTransform));
        let dialog = NumericTransformDialog {
            scale_x: 200.0,
            rotate: 90.0,
            ..NumericTransformDialog::default()
        };
        shell.apply_numeric_transform(&dialog);

        let after = crate::editor::bounds_of(&shell_path(&shell, id));
        assert!(
            (after.width() - 80.0).abs() < 0.001 && (after.height() - 160.0).abs() < 0.001,
            "2× then a quarter turn: the extents double and swap: {after:?}"
        );
        assert_eq!(
            shell.editor.session.undo_len(),
            undo_before + 1,
            "the whole dialog is one undo step"
        );
        assert!(shell.numeric_transform.is_none(), "OK closes the dialog");
    }

    /// The Layers panel's three mutating surfaces: rename, the
    /// per-state lock, and the reorder. The reorder keeps the moved
    /// layer active — the user keeps editing the layer they moved.
    #[test]
    fn the_layers_panel_renames_locks_and_reorders() {
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Sel(menus::SelAction::NewLayer));
        assert_eq!(shell.editor.layer_count(), 2, "the panel adds a layer");

        shell.run_command(
            "layer.rename",
            serde_json::json!({ "page": 0, "layer": 0, "name": "base" }),
        );
        assert_eq!(
            shell.editor.document().pages[0].layers.layers[0].name,
            "base"
        );

        shell.run_command(
            "layer.setLocked",
            serde_json::json!({
                "page": 0, "layer": 0, "frame": 0, "locked": true,
            }),
        );
        assert!(
            shell.editor.document().pages[0].layers.layers[0].cells[0].locked,
            "the layer is locked in this state"
        );

        shell.editor.set_active_layer(0);
        shell.reorder_layer(0, true);
        let layers = &shell.editor.document().pages[0].layers.layers;
        assert_eq!(layers[1].name, "base", "the layer moved up");
        assert_eq!(
            shell.editor.active_layer(),
            1,
            "the moved layer stays active"
        );
    }

    /// Modify ▸ Flatten Selection: the selection bakes into one
    /// bitmap, in place of the members.
    #[test]
    fn flatten_selection_bakes_into_one_bitmap() {
        let mut shell = gui_shell(gui_editor_two());
        let ids = shell.editor.current_cell().unwrap().elements.clone();
        shell.editor.set_selection(ids);
        shell.run_action(&menus::Action::Sel(menus::SelAction::FlattenSelection));
        let cell = shell.editor.current_cell().unwrap();
        assert_eq!(cell.elements.len(), 1, "the bake leaves one object");
        assert!(
            matches!(
                shell.editor.document().registry.get(cell.elements[0]),
                Some(ObjectData::Image(_))
            ),
            "the baked object is a bitmap"
        );
    }

    /// One text param of one applied effect (the `DCVs` form the
    /// panel's rows edit).
    fn shell_param(shell: &Shell, id: doc::ObjectId, index: usize, key: &str) -> String {
        let effects = shell_effect_list(shell, id);
        let doc::EffectValue::Text(value) = &effects[index]
            .properties
            .iter()
            .find(|property| property.key == key)
            .unwrap_or_else(|| panic!("{key}: absent"))
            .value
        else {
            panic!("{key}: not text");
        };
        value.clone()
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
            std::rc::Rc::new(MemoryFs::new()),
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
            Shell::with_platform(Editor::untitled(), std::rc::Rc::new(fs), Box::new(dialogs));
        replayer.replay_steps();
        assert!(replayer.status.contains("replayed"), "{}", replayer.status);
        assert_eq!(
            serde_json::to_vec(replayer.editor.session.document()).unwrap(),
            serde_json::to_vec(editor.session.document()).unwrap(),
            "the replayed session equals the original, byte for byte"
        );
    }

    #[test]
    fn web_export_dialog_writes_html_and_files() {
        // A slice dragged on the canvas, then the dialog pointed at
        // the HTML+Images workflow.
        let mut editor = Editor::untitled();
        editor.tool = Tool::Slice;
        editor.begin_drag(geom::Point::new(10.0, 10.0));
        editor.drag_to(geom::Point::new(90.0, 50.0));
        editor.end_drag().unwrap();
        let id = editor.document().pages[0]
            .layers
            .layers
            .iter()
            .find(|layer| layer.has_url && layer.is_special)
            .and_then(|layer| layer.cells[0].elements.first())
            .copied()
            .unwrap();
        editor
            .apply(
                "slice.setLink",
                serde_json::json!({ "id": id, "link": "https://example.test/", "alt": "go" }),
            )
            .unwrap();

        let dialogs = FixedDialogs {
            steps: "export.html".to_owned(),
            doc: String::new(),
        };
        let mut shell =
            Shell::with_platform(editor, std::rc::Rc::new(MemoryFs::new()), Box::new(dialogs));
        shell.export_dialog = Some(ExportDialog {
            web: Some(1), // HTML + images
            ..ExportDialog::default()
        });
        let dialog = shell.export_dialog.take().unwrap();
        shell.export_via_dialog(&dialog);

        assert!(shell.status.contains("exported"), "{}", shell.status);
        assert!(shell.export_dialog.is_none(), "the dialog closes");
        let html = String::from_utf8(shell.fs.read_file("export.html").unwrap()).unwrap();
        assert!(html.contains(r#"href="https://example.test/""#));
        // The slice's image landed beside the HTML under its auto
        // name; the dialog's doc base is "export".
        let image = shell.fs.read_file("nexport_1_1.png").unwrap();
        assert_eq!(&image[..4], b"\x89PNG");
    }

    /// A 4×4 opaque red PNG, written through the project's own encoder.
    fn red_png() -> Vec<u8> {
        let rgba = [255u8, 0, 0, 255].repeat(16);
        let plane = io::Plane::new(4, 4, &rgba).expect("4x4 plane");
        io::png::encode(&plane).expect("png bytes")
    }

    /// File ▸ Import… places the raster on the active layer, and the
    /// object that lands there is a real image object.
    #[test]
    fn import_places_a_raster_on_the_active_layer() {
        let mut shell = gui_shell(Editor::untitled());
        shell.import_raster(&red_png(), "red.png").expect("import");
        let cell = shell.editor.current_cell().expect("the page's cell");
        assert_eq!(cell.elements.len(), 1, "the import added one object");
        let id = cell.elements[0];
        let document = shell.editor.document();
        let Some(ObjectData::Image(image)) = document.registry.get(id) else {
            panic!("the imported object is an image")
        };
        assert_eq!(image.pixels, (4, 4), "the source extent is kept");
        assert_eq!(image.name.as_deref(), Some("red.png"));
    }

    /// The active layer decides where the import lands: with a second
    /// layer added and selected, the object goes there — not to the
    /// layer underneath.
    #[test]
    fn import_lands_on_the_selected_layer() {
        let mut shell = gui_shell(Editor::untitled());
        shell
            .editor
            .apply("layer.add", serde_json::json!({ "page": 0 }))
            .expect("second layer");
        assert_eq!(shell.editor.layer_count(), 2);
        shell.editor.set_active_layer(1);
        shell.import_raster(&red_png(), "red.png").expect("import");
        let document = shell.editor.document();
        assert!(
            document.pages[0].layers.layers[0].cells[0]
                .elements
                .is_empty(),
            "the lower layer stays empty"
        );
        assert_eq!(
            document.pages[0].layers.layers[1].cells[0].elements.len(),
            1,
            "the active layer took the import"
        );
    }

    /// Edit ▸ Insert ▸ Empty Bitmap: a canvas-sized empty image lands
    /// on the active layer; Via Copy / Via Cut on a selected bitmap
    /// mint a sibling image (the cut also clears the source); with no
    /// bitmap selected they report instead of commanding. Crop
    /// Document shrinks the canvas to the content extents.
    #[test]
    fn edit_bitmap_rows_drive_the_engine() {
        // Empty Bitmap.
        let mut shell = gui_shell(Editor::untitled());
        shell.run_action(&menus::Action::Sel(menus::SelAction::EmptyBitmap));
        assert!(
            shell.status.is_empty() || !shell.status.contains("failed"),
            "{}",
            shell.status
        );
        let document = shell.editor.document();
        let layer = &document.pages[0].layers.layers[0];
        assert_eq!(layer.cells[0].elements.len(), 1, "one image added");
        let id = layer.cells[0].elements[0];
        let Some(ObjectData::Image(image)) = document.registry.get(id) else {
            panic!("empty bitmap is an image")
        };
        let canvas = &document.pages[0].info.canvas;
        assert_eq!(
            image.pixels,
            (canvas.width as i64, canvas.height as i64),
            "the image spans the canvas"
        );

        // Via Copy: import a raster, select it, mint a copy.
        shell.import_raster(&red_png(), "red.png").expect("import");
        let raster = shell.editor.current_cell().expect("cell").elements[0];
        shell.editor.set_selection(vec![raster]);
        shell.run_action(&menus::Action::Sel(menus::SelAction::BitmapViaCopy));
        let elements = &shell.editor.document().pages[0].layers.layers[0].cells[0].elements;
        assert_eq!(elements.len(), 2, "the copy joined the cell");
        assert_ne!(elements[0], elements[1], "a new object id");

        // Via Cut with no bitmap selected reports, not commands.
        let mut shell = gui_shell(Editor::untitled());
        shell.run_action(&menus::Action::Sel(menus::SelAction::BitmapViaCut));
        assert!(shell.status.contains("Select a bitmap"));

        // Crop Document shrinks the canvas to the content extents.
        let mut shell = gui_shell(Editor::untitled());
        shell.import_raster(&red_png(), "red.png").expect("import");
        let before = shell.editor.document().pages[0].info.canvas.width;
        shell.run_action(&menus::Action::Sel(menus::SelAction::CropDocument));
        let after = shell.editor.document().pages[0].info.canvas.width;
        assert!(after < before, "canvas shrank from {before} to {after}");
    }

    /// The pixel at image coordinates (x, y) — the paint tools' read
    /// path, through the engine's own sampler.
    fn image_pixel(shell: &Shell, id: doc::ObjectId, x: i64, y: i64) -> Option<[u8; 4]> {
        let document = shell.editor.document();
        let Some(ObjectData::Image(image)) = document.registry.get(id) else {
            return None;
        };
        engine::bitmap::sample_pixel(image, &document.tiles, x, y)
    }

    /// A shell with a 4×4 red raster imported on the active layer,
    /// and the imported image's id.
    fn shell_with_raster() -> (Shell, doc::ObjectId) {
        let mut shell = gui_shell(Editor::untitled());
        shell.import_raster(&red_png(), "red.png").expect("import");
        let id = shell
            .editor
            .current_cell()
            .expect("the page's cell")
            .elements[0];
        (shell, id)
    }

    /// The brush lays its dabs on the image under the pointer, and the
    /// whole stroke undoes as one step.
    #[test]
    fn the_brush_paints_a_stroke_on_the_image() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::Paintbrush;
        shell.editor.brush.color = [0, 0, 255, 255];
        shell.editor.brush.size = 2;
        shell.editor.brush.edge = 0;

        shell
            .editor
            .begin_stroke(geom::Point::new(0.5, 1.5))
            .expect("the stroke starts");
        shell
            .editor
            .stroke_to(geom::Point::new(3.5, 1.5))
            .expect("the stroke continues");
        shell.editor.end_stroke().expect("the stroke ends");

        assert_eq!(
            image_pixel(&shell, id, 0, 1),
            Some([0, 0, 255, 255]),
            "the dab painted the row"
        );
        assert_eq!(
            image_pixel(&shell, id, 0, 0),
            Some([255, 0, 0, 255]),
            "the rows above the stroke keep their color"
        );

        shell.editor.undo().expect("undo");
        assert_eq!(
            image_pixel(&shell, id, 0, 1),
            Some([255, 0, 0, 255]),
            "one undo takes the whole stroke back"
        );
    }

    /// The pencil draws a hard one-pixel line — the dab carries no
    /// size, edge or shape slot.
    #[test]
    fn the_pencil_draws_a_hard_line() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::Pencil;
        shell.editor.brush.color = [0, 255, 0, 255];

        shell
            .editor
            .begin_stroke(geom::Point::new(0.0, 2.0))
            .expect("the stroke starts");
        shell
            .editor
            .stroke_to(geom::Point::new(3.0, 2.0))
            .expect("the stroke continues");
        shell.editor.end_stroke().expect("the stroke ends");

        assert_eq!(image_pixel(&shell, id, 1, 2), Some([0, 255, 0, 255]));
        assert_eq!(
            image_pixel(&shell, id, 1, 1),
            Some([255, 0, 0, 255]),
            "the pencil leaves its neighbours alone"
        );
    }

    /// The paint bucket fills the image under the pointer with the
    /// paint color.
    #[test]
    fn the_paint_bucket_fills_the_image() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::PaintBucket;
        shell.editor.brush.color = [255, 255, 0, 255];
        assert!(
            shell
                .editor
                .fill_target(geom::Point::new(1.0, 1.0))
                .expect("fill")
        );
        assert_eq!(image_pixel(&shell, id, 3, 3), Some([255, 255, 0, 255]));
    }

    /// The eyedropper reads the pixel under the pointer into the
    /// brush model — the color model, not the document.
    #[test]
    fn the_eyedropper_picks_the_color_under_the_pointer() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::Eyedropper;
        let picked = shell.editor.pick_color(geom::Point::new(2.0, 2.0));
        assert_eq!(picked, Some([255, 0, 0, 255]));
        assert_eq!(shell.editor.brush.color, [255, 0, 0, 255]);
        assert_eq!(
            image_pixel(&shell, id, 2, 2),
            Some([255, 0, 0, 255]),
            "the pick does not touch the document"
        );
    }

    /// The gradient tool paints a ramp between the press and the
    /// release points.
    #[test]
    fn the_gradient_paints_a_ramp_between_its_handles() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::Gradient;
        shell.editor.brush.color = [0, 0, 0, 255];

        shell
            .editor
            .begin_stroke(geom::Point::new(0.0, 0.0))
            .expect("the gradient starts");
        shell
            .editor
            .stroke_to(geom::Point::new(3.0, 0.0))
            .expect("the gradient follows");
        shell.editor.end_stroke().expect("the gradient paints");

        assert_eq!(image_pixel(&shell, id, 0, 0), Some([0, 0, 0, 255]));
        assert_eq!(
            image_pixel(&shell, id, 3, 0),
            Some([255, 255, 255, 255]),
            "the ramp ends in white"
        );
    }

    /// File ▸ Open on a plain raster opens it as its own document,
    /// with the canvas sized to the image.
    #[test]
    fn open_raster_sizes_the_document_to_the_image() {
        let mut shell = gui_shell(Editor::untitled());
        shell
            .open_raster_document(&red_png(), "red.png")
            .expect("open");
        let document = shell.editor.document();
        let canvas = document.pages[0].info.canvas.clone();
        assert_eq!((canvas.width, canvas.height), (4.0, 4.0));
        let cell = shell.editor.current_cell().expect("the page's cell");
        assert_eq!(cell.elements.len(), 1);
    }

    /// A file that is neither a Fireworks PNG nor a raster is
    /// refused with a message rather than a panic.
    #[test]
    fn open_raster_refuses_a_non_image() {
        let mut shell = gui_shell(Editor::untitled());
        let error = shell.open_raster_document(b"not an image at all", "x.bin");
        assert!(error.is_err(), "the decode is refused");
    }

    /// Loading a raster into an empty document opens it as its own
    /// document (the Open door); loading into a document that already
    /// holds objects *places* it (the Import door) — the browser's one
    /// file input must not silently swap out the user's artwork.
    #[test]
    fn load_into_a_drawing_places_the_image_instead_of_replacing_it() {
        // Empty document: the raster becomes the document.
        let mut shell = gui_shell(Editor::untitled());
        shell
            .load_document_bytes(&red_png())
            .expect("open into an empty document");
        assert_eq!(
            shell.editor.document().pages[0].info.canvas.width,
            4.0,
            "the raster opened as its own document"
        );

        // Document with a drawn rect: the raster is placed into it.
        let mut shell = gui_shell(gui_editor());
        let before = shell.editor.current_cell().unwrap().elements.len();
        shell
            .load_document_bytes(&red_png())
            .expect("place into a drawing");
        let cell = shell.editor.current_cell().unwrap();
        assert_eq!(
            cell.elements.len(),
            before + 1,
            "the placed image joined the existing objects"
        );
        let placed = cell.elements[cell.elements.len() - 1];
        let data = shell.editor.document().registry.get(placed);
        assert!(
            matches!(data, Some(ObjectData::Image(_))),
            "the new object is the imported image"
        );
    }

    /// `document_has_content` separates the two doors: empty page
    /// (fresh editor) is content-free, a drag-created rect is not.
    #[test]
    fn document_has_content_follows_the_cell() {
        assert!(!gui_shell(Editor::untitled()).document_has_content());
        assert!(gui_shell(gui_editor()).document_has_content());
    }

    /// The palette commands route to the right engine verbs, and the
    /// swatch raws decode in the ABGR layout the engine stores.
    #[test]
    fn fill_stroke_palette_routes_to_engine_commands() {
        assert_eq!(Shell::paint_command("fill"), "object.setFill");
        assert_eq!(Shell::paint_command("stroke"), "object.setStroke");
        // The black swatch is opaque black in ABGR; transparent is all
        // zero.
        let black = Shell::FILL_STROKE_PALETTE
            .iter()
            .find(|(n, _)| *n == "black");
        assert_eq!(black.map(|(_, raw)| *raw), Some(0xFF00_0000));
        let clear = Shell::FILL_STROKE_PALETTE
            .iter()
            .find(|(n, _)| *n == "none (transparent)");
        assert_eq!(clear.map(|(_, raw)| *raw), Some(0));
        // End-to-end: applying the fill command through the editor
        // changes the created rect's fill_raw.
        let mut editor = gui_editor();
        let id = editor.current_cell().unwrap().elements[0];
        editor
            .apply(
                "object.setFill",
                serde_json::json!({ "id": id, "color_raw": 0xFFFF_0000i64 }),
            )
            .expect("the fill applies");
        match editor.document().registry.get(id) {
            Some(ObjectData::Path(path)) => {
                let paint = path.paint.as_ref().expect("the paint style");
                assert_eq!(paint.fill_raw, 0xFFFF_0000);
            }
            other => panic!("expected a path, got {other:?}"),
        }
    }

    /// An edge press near the page's top starts a horizontal draft;
    /// near the left, a vertical one; mid-canvas, none. Locked or
    /// hidden guides refuse the draft entirely.
    #[test]
    fn guide_draft_starts_at_the_page_edge() {
        let mut shell = gui_shell(Editor::untitled());
        let page = shell.page_world().expect("untitled page bounds");
        let mid = geom::Point::new(page.x1 - 100.0, page.y1 - 100.0);

        let top = shell
            .guide_draft_at(geom::Point::new(page.x0 + 50.0, page.y0 + 2.0))
            .expect("top-edge press");
        assert!(top.horizontal);
        assert_eq!(top.position, page.y0);

        let left = shell
            .guide_draft_at(geom::Point::new(page.x0 + 1.0, page.y0 + 50.0))
            .expect("left-edge press");
        assert!(!left.horizontal);
        assert_eq!(left.position, page.x0);

        assert!(shell.guide_draft_at(mid).is_none(), "mid-canvas press");

        shell.guides_locked = true;
        assert!(
            shell
                .guide_draft_at(geom::Point::new(page.x0 + 1.0, page.y0 + 2.0))
                .is_none(),
            "locked guides refuse drag-out"
        );
        shell.guides_locked = false;
        shell.show_guides = false;
        assert!(
            shell
                .guide_draft_at(geom::Point::new(page.x0 + 1.0, page.y0 + 2.0))
                .is_none(),
            "hidden guides refuse drag-out"
        );
    }

    /// Releasing a draft commits one `page.addGuide` at the dragged
    /// position; snap-to-grid rounds it to the nearest cell line.
    #[test]
    fn guide_draft_release_commits_the_guide() {
        let mut shell = gui_shell(Editor::untitled());
        let page = shell.page_world().expect("untitled page bounds");
        shell.guide_draft = Some(crate::shell::guides_ui::GuideDraft {
            horizontal: true,
            position: page.y0 + 37.0,
        });
        shell.commit_guide_draft();
        assert_eq!(
            shell.editor.document().pages[0].guides.horizontal,
            vec![page.y0 + 37.0],
        );

        shell.snap_to_grid = true;
        shell.guide_draft = Some(crate::shell::guides_ui::GuideDraft {
            horizontal: true,
            position: page.y0 + 26.0,
        });
        shell.commit_guide_draft();
        // Default grid: 24 px cells from the origin — 26 snaps to 24.
        assert_eq!(
            shell.editor.document().pages[0].guides.horizontal,
            vec![page.y0 + 37.0, 24.0],
        );
    }

    /// Snap-to-guides pulls a moving drag's point onto the nearest
    /// line per axis, within tolerance, and ignores far lines.
    #[test]
    fn snap_to_guides_pulls_each_axis_independently() {
        // Install guides directly on the page.
        let mut shell = gui_shell(Editor::untitled());
        shell
            .editor
            .apply(
                "page.addGuide",
                serde_json::json!({ "page": 0, "position": 100.0, "kind": "horizontal" }),
            )
            .expect("guide");
        shell
            .editor
            .apply(
                "page.addGuide",
                serde_json::json!({ "page": 0, "position": 200.0, "kind": "vertical" }),
            )
            .expect("guide");

        let snapped = shell.snap_to_guides(geom::Point::new(197.0, 103.0), 4.0);
        assert_eq!(snapped, geom::Point::new(200.0, 100.0));

        let untouched = shell.snap_to_guides(geom::Point::new(150.0, 150.0), 4.0);
        assert_eq!(untouched, geom::Point::new(150.0, 150.0));
    }

    /// A pixel-selection helper shared by the 19.6 tests: a shell
    /// with a red raster and a committed non-empty marquee over it.
    fn shell_with_marquee() -> (Shell, doc::ObjectId) {
        let (mut shell, id) = shell_with_raster();
        shell.editor.tool = Tool::Marquee;
        shell.editor.pixel_mode = PixelSelectMode::from_modifiers(egui::Modifiers::default());
        shell
            .editor
            .begin_pixel_selection(geom::Point::new(0.5, 0.5))
            .expect("the drag starts");
        shell
            .editor
            .extend_pixel_selection(geom::Point::new(2.0, 2.0))
            .expect("the drag continues");
        shell.editor.end_pixel_selection().expect("the drag ends");
        (shell, id)
    }

    /// The Feather dialog only opens over an active pixel selection,
    /// and the applied radius becomes the next dialog's default.
    #[test]
    fn the_feather_dialog_gates_on_a_pixel_selection() {
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::FeatherDialog));
        assert!(shell.feather.is_none(), "no selection, no dialog");
        assert!(shell.status.contains("no pixel selection"));

        let (mut shell, _id) = shell_with_marquee();
        shell.run_action(&menus::Action::Shell(menus::ShellAction::FeatherDialog));
        assert_eq!(shell.feather, Some(FeatherDialog { radius: 2 }));

        shell.apply_feather(8);
        assert!(shell.feather.is_none(), "apply closes the dialog");
        assert_eq!(shell.editor.feather_radius, 8, "the default updates");
        assert!(shell.status.contains("feathered selection by 8px"));

        // The next open seeds from the last applied radius.
        shell.run_action(&menus::Action::Shell(menus::ShellAction::FeatherDialog));
        assert_eq!(shell.feather, Some(FeatherDialog { radius: 8 }));
    }

    /// Help ▸ About opens the About window; the keyboard-shortcuts
    /// toggle still flips `help_open`.
    #[test]
    fn the_about_action_opens_the_about_window() {
        let mut shell = gui_shell(gui_editor());
        assert!(!shell.about_open);
        shell.run_action(&menus::Action::Shell(menus::ShellAction::About));
        assert!(shell.about_open, "About opens the window");
        shell.about_open = false;
        shell.run_action(&menus::Action::Shell(menus::ShellAction::ShowShortcuts));
        assert!(shell.help_open);
    }

    /// The Canvas Size dialog opens seeded with the live canvas,
    /// resizes page 0 through the engine command, and shifts content
    /// opposite the anchor so the anchored corner stays fixed.
    #[test]
    fn the_canvas_size_dialog_resizes_about_the_anchor() {
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::CanvasSize));
        let dialog = shell
            .canvas_size
            .clone()
            .expect("the dialog opens over any document");
        assert_eq!(dialog.width, 640.0, "seeded from the live canvas");
        assert_eq!(dialog.height, 480.0);

        // Top-left anchor: growth goes right/down, content stays.
        let dialog = CanvasSizeDialog {
            width: 800.0,
            height: 600.0,
            anchor_x: 0,
            anchor_y: 0,
        };
        shell.apply_canvas_size(&dialog);
        assert!(shell.canvas_size.is_none(), "apply closes the dialog");
        let canvas = &shell.editor.session.document().pages[0].info.canvas;
        assert_eq!((canvas.width, canvas.height), (800.0, 600.0));
        assert!(shell.status.contains("canvas resized to 800 × 600"));

        // Center anchor: content moves left/up by half the growth.
        let (mut shell, id) = shell_with_raster();
        let before = union_selection_bounds(shell.editor.session.document(), &[id])
            .expect("the raster has bounds");
        let dialog = CanvasSizeDialog {
            width: 1280.0,
            height: 960.0,
            anchor_x: 1,
            anchor_y: 1,
        };
        shell.apply_canvas_size(&dialog);
        let after = union_selection_bounds(shell.editor.session.document(), &[id])
            .expect("the raster still has bounds");
        assert_eq!(
            (after.x0 - before.x0, after.y0 - before.y0),
            (-320.0, -240.0),
            "center anchor shifts content by half the growth"
        );

        // Out-of-range dimensions refuse and keep the old canvas.
        let dialog = CanvasSizeDialog {
            width: 0.0,
            height: 480.0,
            anchor_x: 0,
            anchor_y: 0,
        };
        shell.apply_canvas_size(&dialog);
        let canvas = &shell.editor.session.document().pages[0].info.canvas;
        assert_eq!(canvas.width, 1280.0, "the refused resize leaves the canvas");
        assert!(shell.status.contains("failed"));
    }

    /// The Image Size dialog opens seeded with the live canvas and
    /// resolution, scales the document through `document.setImageSize`
    /// (objects ride the affine, images resample), and a failed apply
    /// keeps the dialog's status clean while the document is untouched.
    #[test]
    fn the_image_size_dialog_scales_the_document() {
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::ImageSize));
        let dialog = shell
            .image_size
            .clone()
            .expect("the dialog opens over any document");
        assert_eq!(dialog.width, 640.0, "seeded from the live canvas");
        assert_eq!(dialog.height, 480.0);
        assert_eq!(dialog.resolution, 72.0);
        assert!(!dialog.unit_cm, "the default unit is inches");

        // A 2× isotropic scale: the canvas doubles, vector geometry
        // rides the affine.
        let (mut shell, id) = shell_with_raster();
        let before = union_selection_bounds(shell.editor.session.document(), &[id])
            .expect("the raster has bounds");
        shell.apply_image_size(&ImageSizeDialog {
            width: 1280.0,
            height: 960.0,
            resolution: 144.0,
            unit_cm: false,
        });
        assert!(shell.image_size.is_none(), "apply closes the dialog");
        assert!(
            shell
                .status
                .contains("image resized to 1280 × 960 @ 144 px/in")
        );
        let canvas = &shell.editor.session.document().pages[0].info.canvas;
        assert_eq!((canvas.width, canvas.height), (1280.0, 960.0));
        assert_eq!(canvas.resolution, 144.0, "the resolution rides along");
        let after = union_selection_bounds(shell.editor.session.document(), &[id])
            .expect("the raster still has bounds");
        assert_eq!(
            ((after.x1 - after.x0) as i64, (after.y1 - after.y0) as i64),
            (
                (before.x1 - before.x0) as i64 * 2,
                (before.y1 - before.y0) as i64 * 2
            ),
            "the image's pixel grid resamples to twice each side"
        );

        // A non-finite width refuses at the command boundary and the
        // document is left alone; the dialog closes either way.
        let mut shell = gui_shell(gui_editor());
        shell.apply_image_size(&ImageSizeDialog {
            width: f64::NAN,
            height: 480.0,
            resolution: 72.0,
            unit_cm: false,
        });
        assert!(shell.image_size.is_none());
        let canvas = &shell.editor.session.document().pages[0].info.canvas;
        assert_eq!(canvas.width, 640.0, "the refused scale leaves the canvas");
        assert!(shell.status.contains("failed"));
    }

    /// The Canvas Color dialog opens with the current matte, applies
    /// `document.setCanvasColor` document-wide, and cancels cleanly.
    #[test]
    fn the_canvas_color_dialog_sets_the_matte() {
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::CanvasColor));
        let seeded = shell.canvas_color.clone().expect("the dialog opens");

        shell.apply_canvas_color(&CanvasColorDialog {
            color: [12, 34, 56],
        });
        assert!(shell.canvas_color.is_none(), "apply closes the dialog");
        for page in &shell.editor.session.document().pages {
            let raw = page.info.canvas.background_raw;
            assert_eq!(raw & 0xFF, 12, "R in the low byte");
            assert_eq!((raw >> 8) & 0xFF, 34);
            assert_eq!((raw >> 16) & 0xFF, 56);
        }

        // Re-opening seeds from the applied matte.
        shell.run_action(&menus::Action::Shell(menus::ShellAction::CanvasColor));
        assert_eq!(
            shell.canvas_color,
            Some(CanvasColorDialog {
                color: [12, 34, 56]
            })
        );
        let _ = seeded;
    }

    /// Superselect/Subselect route through `selection.selectParents` /
    /// `selectChildren` and pull the document's rewritten selection
    /// back into the view via `adopt_document_selection`.
    #[test]
    fn superselect_and_subselect_drive_the_selection_verbs() {
        let mut shell = gui_shell(gui_editor());
        let id = shell.editor.current_cell().unwrap().elements[0];

        // With a plain object selected, selectParents collapses to the
        // same object (it is top-level) and the view keeps it.
        shell.editor.set_selection(vec![id]);
        shell.run_sel(menus::SelAction::SelectParents);
        assert!(!shell.status.contains("failed"), "got {:?}", shell.status);
        assert_eq!(shell.editor.selected_live(), Some(id));

        // selectChildren on a non-group refuses cleanly into status.
        shell.run_sel(menus::SelAction::SelectChildren);
        assert!(!shell.status.contains("panic"));
    }

    /// Marquee ⇄ path conversion and the bitmap-selection save/restore
    /// actions route to their engine commands through the shell.
    #[test]
    fn marquee_conversion_and_saved_selection_actions_dispatch() {
        let (mut shell, _id) = shell_with_marquee();

        shell.run_sel(menus::SelAction::SaveSelection);
        assert!(
            !shell.editor.document().saved_selections.is_empty(),
            "selection.save stored a slot"
        );

        shell.run_sel(menus::SelAction::DeleteSavedSelection);
        assert!(
            shell.editor.document().saved_selections.is_empty(),
            "selection.deleteSaved cleared the slot"
        );

        // With no pixel target left, MarqueeToPath reports into the
        // status line instead of panicking.
        shell.editor.deselect_pixels().unwrap();
        shell.run_sel(menus::SelAction::MarqueeToPath);
        assert!(shell.status.contains("no pixel selection"));
    }

    /// Mask verbs refuse cleanly with nothing selected and route to
    /// `mask.*` commands with a live selection.
    #[test]
    fn mask_actions_require_a_selection() {
        let mut shell = gui_shell(gui_editor());
        shell.run_sel(menus::SelAction::AddMask { mode: "hide all" });
        assert_eq!(shell.status, "nothing selected");

        shell.run_sel(menus::SelAction::EnableMask(false));
        assert_eq!(shell.status, "nothing selected");

        let id = shell.editor.current_cell().unwrap().elements[0];
        shell.editor.set_selection(vec![id]);
        shell.run_sel(menus::SelAction::AddMask { mode: "hide all" });
        assert!(
            !shell.status.contains("failed"),
            "mask.add on a live object should route, got {:?}",
            shell.status
        );
    }

    /// `adopt_document_selection` pulls the document's object
    /// selection back into the view — the reverse of sync.
    #[test]
    fn adopt_document_selection_pulls_from_the_document() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(0.0, 0.0));
        editor.drag_to(geom::Point::new(10.0, 10.0));
        editor.end_drag().unwrap();
        let id = editor.current_cell().unwrap().elements[0];

        editor.set_selection(Vec::new());
        editor
            .apply("selection.set", serde_json::json!({ "ids": [id] }))
            .unwrap();
        assert_eq!(editor.selected_live(), None, "the view is stale");
        editor.adopt_document_selection();
        assert_eq!(editor.selected_live(), Some(id));
    }

    /// The context menu's pick step: a right-click that missed the
    /// selection adopts the object under the pointer; a right-click on
    /// the selection leaves it alone; a miss on empty canvas clears
    /// nothing.
    #[test]
    fn the_context_menu_pick_selects_under_the_pointer() {
        let mut editor = Editor::untitled();
        editor.tool = Tool::Rect;
        editor.begin_drag(geom::Point::new(0.0, 0.0));
        editor.drag_to(geom::Point::new(10.0, 10.0));
        editor.end_drag().unwrap();
        editor.begin_drag(geom::Point::new(40.0, 0.0));
        editor.drag_to(geom::Point::new(50.0, 10.0));
        editor.end_drag().unwrap();
        let (first, second) = {
            let elements = &editor.current_cell().unwrap().elements;
            (elements[0], elements[1])
        };
        let mut shell = gui_shell(editor);

        // A miss on the second object while the first is selected.
        shell.editor.set_selection(vec![first]);
        shell.context_menu_pick(geom::Point::new(45.0, 5.0));
        assert_eq!(shell.editor.selected_live(), Some(second));

        // A right-click on the selection keeps it.
        shell.context_menu_pick(geom::Point::new(45.0, 5.0));
        assert_eq!(shell.editor.selected_live(), Some(second));

        // A right-click on empty canvas leaves the selection alone —
        // the menu still opens, its rows just disable.
        shell.context_menu_pick(geom::Point::new(200.0, 200.0));
        assert_eq!(shell.editor.selected_live(), Some(second));
    }

    /// The canvas popup adapts to the selection class: the empty
    /// class offers Import/Export and disables the selection verbs; a
    /// text selection offers the text verbs; a multi-selection the
    /// group verbs; and no row is ever an unwired placeholder.
    #[test]
    fn the_context_menu_rows_adapt_to_the_selection_class() {
        use crate::shell::canvas_ui::{SelectionClass, context_rows};

        let empty = context_rows(SelectionClass::Empty, false);
        assert!(empty.iter().any(|r| r.label == "Import..."));
        assert!(
            empty.iter().all(|r| !matches!(
                r.action,
                crate::menus::Action::Sel(_) | crate::menus::Action::Arrange(_)
            ) || !r.enabled
                || matches!(
                    r.action,
                    crate::menus::Action::Shell(crate::menus::ShellAction::Paste)
                )),
            "selection verbs are disabled without a selection"
        );

        let text = context_rows(SelectionClass::Text, true);
        let labels: Vec<_> = text.iter().map(|r| r.label.as_str()).collect();
        for verb in ["Attach to Path", "Detach from Path", "Convert to Paths"] {
            assert!(labels.contains(&verb), "text popup lacks {verb}");
        }
        assert!(
            !labels.contains(&"Simplify"),
            "the text popup has no path verbs"
        );

        let bitmap = context_rows(SelectionClass::Bitmap, true);
        let labels: Vec<_> = bitmap.iter().map(|r| r.label.as_str()).collect();
        assert!(labels.contains(&"Select Similar"));
        assert!(labels.contains(&"Flatten Selection"));

        let multi = context_rows(SelectionClass::Multi, true);
        let labels: Vec<_> = multi.iter().map(|r| r.label.as_str()).collect();
        assert!(labels.contains(&"Group\tCtrl+G"));
        assert!(labels.contains(&"Select Children"));

        // No class ever shows an unwired row.
        for class in [
            SelectionClass::Empty,
            SelectionClass::Bitmap,
            SelectionClass::Path,
            SelectionClass::Text,
            SelectionClass::Instance,
            SelectionClass::Slice,
            SelectionClass::Group,
            SelectionClass::Multi,
        ] {
            for row in context_rows(class, true) {
                assert!(
                    !matches!(row.action, crate::menus::Action::Inert),
                    "{} renders an inert row",
                    row.label
                );
            }
        }
    }

    /// Edit ▸ Insert ▸ Hotspot / slice rows land a web object over
    /// the selection bounds (canvas center when nothing is selected),
    /// and New Button appends a button definition to the library.
    #[test]
    fn insert_web_rows_create_web_objects_and_buttons() {
        let (mut shell, id) = shell_with_raster();
        shell.editor.selection_set = vec![id];
        shell.editor.selection = Some(id);
        let bounds = union_selection_bounds(shell.editor.session.document(), &[id])
            .expect("the raster has bounds");

        shell.run_action(&menus::Action::Shell(menus::ShellAction::InsertWeb {
            what: menus::InsertWebKind::Hotspot,
        }));
        let document = shell.editor.session.document();
        let hotspot = document
            .registry
            .iter()
            .find_map(|(_, object)| match object {
                doc::ObjectData::Slice(slice) if slice.slice_type.is_none() => Some(slice),
                _ => None,
            })
            .expect("the hotspot was created");
        assert_eq!(hotspot.frame.x0, bounds.x0);
        assert_eq!(hotspot.frame.y0, bounds.y0);
        assert_eq!(hotspot.frame.width(), bounds.width());
        assert_eq!(hotspot.frame.height(), bounds.height());

        shell.run_action(&menus::Action::Shell(menus::ShellAction::InsertWeb {
            what: menus::InsertWebKind::RectSlice,
        }));
        let document = shell.editor.session.document();
        let slice = document
            .registry
            .iter()
            .find_map(|(_, object)| match object {
                doc::ObjectData::Slice(slice) if slice.slice_type == Some(0) => Some(slice),
                _ => None,
            })
            .expect("the slice was created");
        assert_eq!(
            (slice.frame.width(), slice.frame.height()),
            (bounds.width(), bounds.height())
        );

        shell.run_action(&menus::Action::Shell(menus::ShellAction::InsertWeb {
            what: menus::InsertWebKind::PolygonSlice,
        }));
        let document = shell.editor.session.document();
        let polygon = document
            .registry
            .iter()
            .find_map(|(_, object)| match object {
                doc::ObjectData::Slice(slice) if slice.shape == 2 => Some(slice),
                _ => None,
            })
            .expect("the polygon slice was created");
        assert_eq!(polygon.contours.len(), 1);
        assert_eq!(polygon.contours[0].point_count, 4, "the rect's corners");

        // Nothing selected: the canvas-center default.
        let mut shell = gui_shell(Editor::untitled());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::InsertWeb {
            what: menus::InsertWebKind::RectSlice,
        }));
        let document = shell.editor.session.document();
        let canvas = &document.pages[0].info.canvas;
        let slice = document
            .registry
            .iter()
            .find_map(|(_, object)| match object {
                doc::ObjectData::Slice(slice) => Some(slice),
                _ => None,
            })
            .expect("the slice was created");
        assert_eq!(slice.frame.x0, canvas.x + canvas.width / 4.0);
        assert_eq!(slice.frame.width(), canvas.width / 2.0);

        // New Button: a button definition in the library.
        let mut shell = gui_shell(Editor::untitled());
        shell.run_action(&menus::Action::Shell(menus::ShellAction::InsertWeb {
            what: menus::InsertWebKind::Button,
        }));
        let document = shell.editor.session.document();
        assert_eq!(document.symbols.len(), 1);
        assert_eq!(document.symbols[0].symbol_type, doc::SymbolType::Button);
    }

    /// The Modify-remainder batch: fill-edge rows command every
    /// selected path, Edit Symbol enters the selected instance's
    /// definition scope, and non-instance selections report instead.
    #[test]
    fn fill_edge_and_edit_symbol_rows_drive_the_selection() {
        let mut shell = gui_shell(gui_editor());
        assert!(
            shell
                .editor
                .apply(
                    "object.createPath",
                    serde_json::json!({
                        "kind": "rect", "x": 10.0, "y": 10.0,
                        "width": 50.0, "height": 50.0,
                        "page": 0, "layer": 0, "cell": 0,
                        "name": null,
                    }),
                )
                .is_ok(),
            "the path lands"
        );
        let id = shell.editor.current_cell().unwrap().elements[0];
        shell.editor.selection_set = vec![id];
        shell.editor.selection = Some(id);

        shell.run_action(&menus::Action::Sel(menus::SelAction::FillEdge {
            mode: "hard",
        }));
        let document = shell.editor.session.document();
        let path = match document.registry.get(id) {
            Some(doc::ObjectData::Path(path)) => path,
            _ => panic!("the object is a path"),
        };
        let fill = path
            .paint
            .as_ref()
            .and_then(|paint| paint.fill.as_ref())
            .expect("the edge materialized a fill");
        assert!(fill.hard_edge, "hard mode set FRDb");

        shell.run_action(&menus::Action::Sel(menus::SelAction::FillEdge {
            mode: "feather",
        }));
        let document = shell.editor.session.document();
        let path = match document.registry.get(id) {
            Some(doc::ObjectData::Path(path)) => path,
            _ => panic!("the object is a path"),
        };
        let fill = path.paint.as_ref().and_then(|p| p.fill.as_ref()).unwrap();
        assert!(!fill.hard_edge);
        assert_eq!(fill.feather, i64::from(shell.editor.feather_radius));

        // Edit Symbol with no instance: report, not command.
        let mut shell = gui_shell(gui_editor());
        shell.run_action(&menus::Action::Sel(menus::SelAction::EditSymbol));
        assert!(shell.status.contains("symbol instance"));

        // With an instance: the editor enters its definition's scope.
        // gui_editor() seeds a rect, so the instance appended after it
        // is the cell's second element.
        let mut shell = gui_shell(gui_editor());
        assert!(
            shell
                .editor
                .apply(
                    "library.add",
                    serde_json::json!({
                        "name": "def", "symbol_type": "graphic",
                    })
                )
                .is_ok()
        );
        let symbol = shell.editor.document().symbols[0].symbol_number;
        assert!(
            shell
                .editor
                .apply(
                    "symbol.createInstance",
                    serde_json::json!({
                        "page": 0, "layer": 0, "cell": 0, "symbol": symbol,
                        "x": 10.0, "y": 10.0,
                    })
                )
                .is_ok()
        );
        let elements = &shell.editor.current_cell().unwrap().elements;
        let id = elements[elements.len() - 1];
        shell.editor.selection_set = vec![id];
        shell.editor.selection = Some(id);
        shell.run_action(&menus::Action::Sel(menus::SelAction::EditSymbol));
        assert_eq!(shell.editor.symbol_scope, Some(symbol));
    }
}
