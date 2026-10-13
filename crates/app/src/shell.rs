//! The egui face of the editor: menus, panels, canvas. Thin by rule —
//! every document change you see here is an [`Editor::apply`] call,
//! and every panel reads the document read-only. Desktop file routing
//! goes through the injected [`engine::platform`] traits (`EnvDialogs`
//! below; real modal dialogs land with desktop integration).

use doc::{ObjectData, PathObject, SymbolInstanceObject, SymbolType, TextObject};
use egui::{Color32, ColorImage, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2};

use crate::canvas::View;
use crate::editor::{AutoShape, Editor, NineSliceEdge, Tool};
use crate::menus;

/// The CS6 chrome look: near-black charcoal panels, square corners,
/// small type. Applied once at startup; `eww`-era Fireworks was dark
/// graphite against a mid-gray canvas.
fn apply_cs6_theme(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        let visuals = &mut style.visuals;
        visuals.dark_mode = true;
        visuals.panel_fill = Color32::from_rgb(0x3c, 0x3c, 0x3c);
        visuals.window_fill = Color32::from_rgb(0x32, 0x32, 0x32);
        visuals.extreme_bg_color = Color32::from_rgb(0x28, 0x28, 0x28);
        visuals.faint_bg_color = Color32::from_rgb(0x44, 0x44, 0x44);
        visuals.override_text_color = Some(Color32::from_rgb(0xd9, 0xd9, 0xd9));
        visuals.selection.bg_fill = Color32::from_rgb(0x2f, 0x5d, 0x8a);
        visuals.selection.stroke = (1.0, Color32::from_rgb(0x50, 0x7e, 0xab)).into();
        visuals.window_corner_radius = egui::CornerRadius::ZERO;
        visuals.menu_corner_radius = egui::CornerRadius::ZERO;
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = egui::CornerRadius::ZERO;
        }
        style.spacing.menu_margin = egui::Margin::same(2);
        style.spacing.button_padding = egui::Vec2::new(6.0, 2.0);
        style.spacing.item_spacing = egui::Vec2::new(6.0, 3.0);
    });
}

/// Display name for a tool (the CS6 tool names).
fn tool_label(tool: Tool) -> &'static str {
    match tool {
        Tool::Pointer => "Pointer",
        Tool::Subselect => "Subselection",
        Tool::Rect => "Rectangle",
        Tool::Ellipse => "Ellipse",
        Tool::Line => "Line",
        Tool::Text => "Text",
        Tool::Zoom => "Zoom",
        Tool::Hand => "Hand",
        Tool::Slice => "Rectangle Slice",
        Tool::Hotspot => "Web Hotspot",
        Tool::AutoShape(_) => "Auto Shape",
    }
}

/// The CS6 one-key select for a tool ("" when unbound here).
fn tool_key(tool: Tool) -> &'static str {
    match tool {
        Tool::Pointer => "V",
        Tool::Rect => "R",
        Tool::Ellipse => "E",
        Tool::Line => "L",
        Tool::Text => "T",
        Tool::Zoom => "Z",
        Tool::Hand => "H",
        _ => "",
    }
}

/// Original glyph art for a tool, drawn from `Painter` primitives.
/// Silhouettes are FireCraft's own — no Adobe icon bytes, no traces.
fn draw_tool_glyph(painter: &egui::Painter, rect: Rect, tool: Tool, color: Color32) {
    let c = rect.center();
    let stroke = (1.4, color);
    match tool {
        Tool::Pointer => {
            // filled head plus tail
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 4.0, c.y - 6.0),
                    Pos2::new(c.x + 2.5, c.y + 0.5),
                    Pos2::new(c.x - 1.5, c.y + 1.0),
                ],
                color,
                stroke,
            ));
            painter.line_segment(
                [
                    Pos2::new(c.x - 1.0, c.y + 2.0),
                    Pos2::new(c.x + 1.0, c.y + 6.0),
                ],
                stroke,
            );
        }
        Tool::Subselect => {
            // the same arrow, outline only
            let points = [
                Pos2::new(c.x - 4.0, c.y - 6.0),
                Pos2::new(c.x + 2.5, c.y + 0.5),
                Pos2::new(c.x - 1.5, c.y + 1.0),
                Pos2::new(c.x + 1.0, c.y + 6.0),
                Pos2::new(c.x - 1.5, c.y + 1.5),
                Pos2::new(c.x - 4.0, c.y + 3.5),
                Pos2::new(c.x - 4.0, c.y - 6.0),
            ];
            painter.add(egui::Shape::line(points.to_vec(), stroke));
        }
        Tool::Rect => {
            painter.rect_stroke(
                Rect::from_center_size(c, egui::Vec2::new(11.0, 9.0)),
                0.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Tool::Ellipse => {
            painter.circle_stroke(c, 5.5, stroke);
        }
        Tool::Line => {
            painter.line_segment(
                [
                    Pos2::new(c.x - 5.0, c.y + 5.0),
                    Pos2::new(c.x + 5.0, c.y - 5.0),
                ],
                stroke,
            );
        }
        Tool::Text => {
            painter.text(
                c,
                egui::Align2::CENTER_CENTER,
                "T",
                egui::FontId::proportional(13.0),
                color,
            );
        }
        Tool::Zoom => {
            painter.circle_stroke(Pos2::new(c.x - 1.5, c.y - 1.5), 4.0, stroke);
            painter.line_segment(
                [
                    Pos2::new(c.x + 1.5, c.y + 1.5),
                    Pos2::new(c.x + 6.0, c.y + 6.0),
                ],
                (2.0, color),
            );
        }
        Tool::Hand => {
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(c.x - 4.5, c.y - 1.5), egui::Vec2::new(9.0, 7.5)),
                3.0,
                color,
            );
            for (offset, height) in [(-3.0, 3.5), (0.0, 4.5), (3.0, 3.0)] {
                painter.line_segment(
                    [
                        Pos2::new(c.x + offset, c.y - 1.5),
                        Pos2::new(c.x + offset, c.y - 1.5 - height),
                    ],
                    stroke,
                );
            }
        }
        Tool::Slice => {
            painter.rect_stroke(
                Rect::from_center_size(c, egui::Vec2::splat(9.0)),
                0.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - 6.5, c.y - 3.0),
                    Pos2::new(c.x - 2.5, c.y - 3.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - 3.0, c.y - 6.5),
                    Pos2::new(c.x - 3.0, c.y - 2.5),
                ],
                stroke,
            );
        }
        Tool::Hotspot => {
            painter.circle_filled(c, 5.5, Color32::from_rgb(0x4f, 0x7c, 0xc0));
            painter.circle_stroke(c, 5.5, (1.0, Color32::from_rgb(0x9c, 0xb8, 0xe0)));
        }
        Tool::AutoShape(shape) => draw_autoshape_glyph(painter, rect, shape, color),
    }
}

/// Preview glyphs for the Auto Shape flyout: each of the ten
/// generators' silhouette as a one-stroke vector sketch (original
/// art — see `docs/ATTRIBUTION.md`).
fn draw_autoshape_glyph(painter: &egui::Painter, rect: Rect, shape: AutoShape, color: Color32) {
    let c = rect.center();
    let stroke = (1.4, color);
    let loop_points = |radii: &[f64]| -> Vec<Pos2> {
        let mut points = Vec::with_capacity(radii.len() + 1);
        for (index, radius) in radii.iter().enumerate() {
            let angle = -std::f64::consts::FRAC_PI_2
                + index as f64 * std::f64::consts::TAU / radii.len() as f64;
            points.push(Pos2::new(
                c.x + (radius * angle.cos()) as f32,
                c.y + (radius * angle.sin()) as f32,
            ));
        }
        points.push(points[0]);
        points
    };
    match shape {
        AutoShape::Star => {
            painter.add(egui::Shape::line(
                loop_points(&[6.5, 3.0, 6.5, 3.0, 6.5, 3.0, 6.5, 3.0, 6.5, 3.0]),
                stroke,
            ));
        }
        AutoShape::Polygon => {
            painter.add(egui::Shape::line(
                loop_points(&[6.5, 6.5, 6.5, 6.5, 6.5, 6.5]),
                stroke,
            ));
        }
        AutoShape::Spiral => {
            let mut points = Vec::new();
            for step in 0..48 {
                let t = step as f64 / 47.0;
                let angle = t * 3.0 * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
                let radius = 1.0 + t * 5.5;
                points.push(Pos2::new(
                    c.x + (radius * angle.cos()) as f32,
                    c.y + (radius * angle.sin()) as f32,
                ));
            }
            painter.add(egui::Shape::line(points, stroke));
        }
        AutoShape::Pie => {
            let center = Pos2::new(c.x - 1.5, c.y + 1.5);
            let mut points = vec![center];
            for step in 0..9 {
                let angle =
                    std::f64::consts::FRAC_PI_2 - step as f64 * std::f64::consts::TAU * 0.75 / 8.0;
                points.push(Pos2::new(
                    center.x + (6.0 * angle.cos()) as f32,
                    center.y + (6.0 * angle.sin()) as f32,
                ));
            }
            points.push(center);
            painter.add(egui::Shape::line(points, stroke));
        }
        AutoShape::Doughnut => {
            painter.circle_stroke(c, 6.0, stroke);
            painter.circle_stroke(c, 2.5, stroke);
        }
        AutoShape::LShape => {
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - 5.0, c.y - 6.0),
                    Pos2::new(c.x - 5.0, c.y + 6.0),
                    Pos2::new(c.x + 6.0, c.y + 6.0),
                ],
                stroke,
            ));
        }
        AutoShape::Arrow => {
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - 6.0, c.y - 2.5),
                    Pos2::new(c.x + 0.5, c.y - 2.5),
                    Pos2::new(c.x + 0.5, c.y - 5.5),
                    Pos2::new(c.x + 6.0, c.y),
                    Pos2::new(c.x + 0.5, c.y + 5.5),
                    Pos2::new(c.x + 0.5, c.y + 2.5),
                    Pos2::new(c.x - 6.0, c.y + 2.5),
                    Pos2::new(c.x - 6.0, c.y - 2.5),
                ],
                stroke,
            ));
        }
        AutoShape::ArrowLine => {
            painter.line_segment(
                [
                    Pos2::new(c.x - 6.0, c.y + 4.0),
                    Pos2::new(c.x + 4.5, c.y - 3.5),
                ],
                stroke,
            );
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x + 6.0, c.y - 6.0),
                    Pos2::new(c.x + 4.5, c.y - 3.5),
                    Pos2::new(c.x + 1.0, c.y - 4.0),
                ],
                stroke,
            ));
        }
        AutoShape::Measure => {
            painter.line_segment(
                [Pos2::new(c.x - 6.0, c.y), Pos2::new(c.x + 6.0, c.y)],
                stroke,
            );
            for x in [-6.0, 6.0] {
                painter.line_segment(
                    [Pos2::new(c.x + x, c.y - 4.0), Pos2::new(c.x + x, c.y + 4.0)],
                    stroke,
                );
            }
        }
        AutoShape::ConnectorLine => {
            painter.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - 6.0, c.y - 5.0),
                    Pos2::new(c.x + 1.5, c.y - 5.0),
                    Pos2::new(c.x + 1.5, c.y + 5.0),
                    Pos2::new(c.x + 6.0, c.y + 5.0),
                ],
                stroke,
            ));
        }
    }
}

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

/// One Web Layer panel row: id, display name, is-foreground-slice,
/// link, alt text, and the raw `TSLi` behind the foreground toggle.
type WebRow = (
    doc::ObjectId,
    String,
    bool,
    Option<String>,
    Option<String>,
    i64,
);

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

/// The web export workflows the dialog offers (the M11 set), in menu
/// order — the same names the CLI's `--workflow` flag takes.
const WEB_WORKFLOWS: &[&str] = &[
    "images only",
    "HTML + images",
    "CSS + images",
    "CSS sprites",
    "CSS properties",
    "pages to files",
    "states to files",
    "layers to files",
    "states to GIF",
];

/// The [`webexport::Workflow`] behind a dialog index — the two lists
/// move together.
fn web_workflow(index: usize) -> Option<webexport::Workflow> {
    if index >= WEB_WORKFLOWS.len() {
        return None;
    }
    Some(match index {
        0 => webexport::Workflow::ImagesOnly,
        1 => webexport::Workflow::HtmlAndImages,
        2 => webexport::Workflow::CssAndImages,
        3 => webexport::Workflow::CssSprites,
        4 => webexport::Workflow::CssProperties,
        5 => webexport::Workflow::PagesToFiles,
        6 => webexport::Workflow::StatesToFiles,
        7 => webexport::Workflow::LayersToFiles,
        _ => webexport::Workflow::StatesToGif,
    })
}

/// Export-dialog state (File → Export…): format, per-format controls,
/// and the cached 2-Up preview. The controls mirror the `EXPv` fields
/// the model maps; Export stamps them onto a cloned document and runs
/// the same codec path the CLI does. A web workflow replaces the
/// raster format entirely — the dialog then drives
/// [`app_cli::ops::export_web`], the same entry point the CLI verb
/// shares.
struct ExportDialog {
    /// Index into [`EXPORT_FORMATS`].
    format: usize,
    /// Selected web workflow, or `None` for the raster formats.
    web: Option<usize>,
    /// Emit the 1px shim row/column (web table workflows).
    web_shim: bool,
    /// Give areas without slices their own image (`include_undefined`).
    web_undefined: bool,
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
            web: None,
            web_shim: true,
            web_undefined: true,
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
    /// Display name of the open document (tab strip).
    doc_name: String,
    /// Last canvas widget rect — the anchor for view commands that
    /// arrive outside the canvas (menu/shortcut zoom and fit).
    canvas_rect: Option<Rect>,
    /// Set by the Exit action; the frame loop closes the viewport.
    close_requested: bool,
    /// The Auto Shape flyout is open (toggled by its tool button).
    autoshape_flyout: bool,
}

impl Shell {
    /// Builds the shell with desktop platform defaults.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_cs6_theme(&cc.egui_ctx);
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
            doc_name: "untitled".to_owned(),
            canvas_rect: None,
            close_requested: false,
            autoshape_flyout: false,
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
                self.doc_name = std::path::Path::new(&path)
                    .file_name()
                    .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
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

    /// The Web Layer panel: the page's slices and hotspots with link
    /// editing (`slice.setLink`) and the foreground/background toggle
    /// (`slice.setType`). Snapshots the web objects first so the edit
    /// fields never borrow the document while commands run.
    fn web_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Web Layer");
        let page = self.editor.page;
        let frame = self.editor.frame;
        let rows: Vec<WebRow> = {
            let document = self.editor.document();
            let Some(layer) = document.pages.get(page).and_then(|page| {
                page.layers
                    .layers
                    .iter()
                    .rev()
                    .find(|layer| layer.has_url && layer.is_special)
            }) else {
                return;
            };
            let Some(cell) = layer.cells.get(frame) else {
                return;
            };
            cell.elements
                .iter()
                .filter_map(|id| match document.registry.get(*id) {
                    Some(ObjectData::Slice(slice)) => Some((
                        *id,
                        slice
                            .name
                            .clone()
                            .unwrap_or_else(|| format!("slice {}", id.index())),
                        slice.slice_type.is_some(),
                        slice.link.clone(),
                        slice.alt.clone(),
                        slice.slice_type.unwrap_or(0),
                    )),
                    _ => None,
                })
                .collect()
        };
        if rows.is_empty() {
            ui.label("no slices on this state");
            return;
        }
        for (id, name, is_slice, link, alt, slice_type) in rows {
            ui.horizontal(|ui| {
                ui.strong(&name);
                ui.label(if is_slice { "slice" } else { "hotspot" });
            });
            let mut edited = link.unwrap_or_default();
            let response = ui.add(
                egui::TextEdit::singleline(&mut edited)
                    .hint_text("link URL")
                    .desired_width(f32::INFINITY),
            );
            if response.changed() {
                // An emptied field clears the link (null, not "");
                // re-assert the existing alt so a link edit never
                // clears it.
                let link = (!edited.is_empty()).then_some(edited);
                self.run_command(
                    "slice.setLink",
                    serde_json::json!({ "id": id, "link": link, "alt": alt }),
                );
            }
            if is_slice {
                ui.horizontal(|ui| {
                    ui.label("type");
                    if ui.selectable_label(slice_type == 0, "foreground").clicked() {
                        self.run_command(
                            "slice.setType",
                            serde_json::json!({ "id": id, "slice_type": 0 }),
                        );
                    }
                    if ui.selectable_label(slice_type == 2, "background").clicked() {
                        self.run_command(
                            "slice.setType",
                            serde_json::json!({ "id": id, "slice_type": 2 }),
                        );
                    }
                });
            }
            ui.separator();
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
                egui::ComboBox::from_label("Web workflow")
                    .selected_text(match dialog.web {
                        Some(index) => WEB_WORKFLOWS[index],
                        None => "—",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut dialog.web, None, "—");
                        for (i, name) in WEB_WORKFLOWS.iter().enumerate() {
                            ui.selectable_value(&mut dialog.web, Some(i), *name);
                        }
                    });
                if dialog.web.is_some() {
                    ui.checkbox(&mut dialog.web_shim, "Shim edges");
                    ui.checkbox(&mut dialog.web_undefined, "Include areas without slices");
                }
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
        // document move. Web workflows write many files — no preview.
        let stamp = self.preview_stamp();
        let regenerate = dialog.web.is_none()
            && match &dialog.preview {
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
    /// settings, write through the injected file system. A selected
    /// web workflow routes to [`Self::export_web_via_dialog`] instead.
    fn export_via_dialog(&mut self, dialog: &ExportDialog) {
        if let Some(index) = dialog.web {
            self.export_web_via_dialog(dialog, index);
            return;
        }
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

    /// Web export through the save dialog: the save path names the
    /// primary text artifact (`export.html`, the sprite/properties
    /// CSS); every image file lands beside it under its export name.
    /// The pipeline is [`app_cli::ops::export_web`] — the CLI verb's
    /// exact path, so the dialog cannot drift from the CLI.
    fn export_web_via_dialog(&mut self, dialog: &ExportDialog, index: usize) {
        let Some(workflow) = web_workflow(index) else {
            return;
        };
        let options = webexport::WebExportOptions {
            page: self.editor.page,
            state: self.editor.frame,
            doc_name: "export".to_owned(),
            title: "export".to_owned(),
            shim: dialog.web_shim,
            include_undefined: dialog.web_undefined,
            ..webexport::WebExportOptions::default()
        };
        let export = match app_cli::ops::export_web(self.editor.document(), &options, workflow) {
            Ok(export) => export,
            Err(error) => {
                self.status = format!("export failed: {error}");
                return;
            }
        };
        let (suggested, filter) = match workflow {
            webexport::Workflow::CssSprites => ("sprites.css", "css"),
            webexport::Workflow::CssProperties => ("properties.css", "css"),
            webexport::Workflow::StatesToGif => ("export.gif", "gif"),
            _ => ("export.html", "html"),
        };
        let Some(path) = self.dialogs.save_path(
            "Export",
            suggested,
            &[(filter.to_owned(), vec![filter.to_owned()])],
        ) else {
            return;
        };
        let dir = std::path::Path::new(&path)
            .parent()
            .map_or_else(std::path::PathBuf::new, std::path::Path::to_path_buf);
        let mut count = 0usize;
        for file in &export.files {
            let target = dir.join(&file.name);
            match self.fs.write_file(&target.to_string_lossy(), &file.bytes) {
                Ok(()) => count += 1,
                Err(error) => {
                    self.status = format!("export failed: {error}");
                    return;
                }
            }
        }
        if let Some(text) = &export.html {
            if let Err(error) = self.fs.write_file(&path, text.as_bytes()) {
                self.status = format!("export failed: {error}");
                return;
            }
            count += 1;
        }
        self.status = format!("exported {count} files to {}", dir.to_string_lossy());
        self.export_dialog = None;
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
                Some((a, b)) => self.run_command(
                    "path.boolean",
                    serde_json::json!({ "a": a, "b": b, "op": op }),
                ),
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
        }
    }

    /// Shell-level plumbing: file, history, selection, view.
    fn run_shell(&mut self, action: menus::ShellAction) {
        match action {
            menus::ShellAction::New => {
                self.editor = Editor::untitled();
                self.view = View::default();
                self.raster = None;
                self.doc_name = "untitled".to_owned();
                self.status = "new document".to_owned();
            }
            menus::ShellAction::Open => self.open_document(),
            menus::ShellAction::Save | menus::ShellAction::SaveAs => self.save_document(),
            menus::ShellAction::Export | menus::ShellAction::ExportPreview => {
                self.export_dialog = Some(ExportDialog::default());
            }
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
                self.editor.selection = None;
                self.editor.selection_set.clear();
                self.status = "deselected".to_owned();
            }
            menus::ShellAction::SelectAll => {
                let ids = self
                    .editor
                    .current_cell()
                    .map_or_else(Vec::new, |cell| cell.elements.clone());
                self.editor.selection = ids.last().copied();
                let picked = ids.len();
                self.editor.selection_set = ids;
                self.status = format!("selected {picked} object(s)");
            }
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
            menus::ShellAction::About => {
                self.status = "FireCraft — a clean-room reimplementation of \
                    Adobe Fireworks CS6 (Apache-2.0)"
                    .to_owned();
            }
            menus::ShellAction::PickTool(tool) => {
                self.editor.tool = tool;
                self.status = format!("tool: {}", tool_label(tool));
            }
        }
    }

    /// Engine commands whose payload derives from the current
    /// selection or document shape.
    fn run_sel(&mut self, action: menus::SelAction) {
        match action {
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
            }
            menus::SelAction::NewPage => {
                self.run_command("page.add", serde_json::json!({ "index": null }));
            }
            menus::SelAction::BreakApart => {
                if let Some(id) = self.editor.selection {
                    self.run_command("symbol.breakApart", serde_json::json!({ "id": id }));
                }
            }
        }
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
    }

    /// The right panel stack: the CS6 panel roster from the Window
    /// menu, as collapsible sections (the fixed-layout stand-in for
    /// CS6's floating docks).
    fn right_panels(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::CollapsingHeader::new("Pages")
                .default_open(true)
                .show(ui, |ui| self.pages_section(ui));
            egui::CollapsingHeader::new("States")
                .default_open(true)
                .show(ui, |ui| self.states_section(ui));
            egui::CollapsingHeader::new("Layers")
                .default_open(true)
                .show(ui, |ui| self.layers_section(ui));
            egui::CollapsingHeader::new("Document Library")
                .default_open(false)
                .show(ui, |ui| self.library_panel(ui));
            egui::CollapsingHeader::new("History")
                .default_open(false)
                .show(ui, |ui| self.history_section(ui));
            egui::CollapsingHeader::new("Web Layer")
                .default_open(false)
                .show(ui, |ui| self.web_panel(ui));
        });
    }

    /// Pages section: one selectable row per page.
    fn pages_section(&mut self, ui: &mut egui::Ui) {
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
    fn states_section(&mut self, ui: &mut egui::Ui) {
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
    fn layers_section(&mut self, ui: &mut egui::Ui) {
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
                    format!("Layer {}", index + 1)
                } else {
                    layer.name.clone()
                };
                (label, layer.display)
            })
            .collect();
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
    }

    /// History section: journal rows plus the steps save/replay pair.
    fn history_section(&mut self, ui: &mut egui::Ui) {
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

    /// World-space page bounds — the Fit All target.
    fn page_world(&self) -> Option<geom::Rect> {
        self.editor
            .document()
            .pages
            .get(self.editor.page)
            .map(|page| {
                geom::Rect::new(
                    page.info.canvas.x,
                    page.info.canvas.y,
                    page.info.canvas.x + page.info.canvas.width,
                    page.info.canvas.y + page.info.canvas.height,
                )
            })
    }

    /// The CS6 tool strip: vertical, grouped, glyph buttons painted
    /// with `Painter` primitives (original art, never the Adobe icons).
    fn tools_panel(&mut self, ui: &mut egui::Ui) {
        let groups: &[&[Tool]] = &[
            &[Tool::Pointer, Tool::Subselect],
            &[Tool::Rect, Tool::Ellipse, Tool::Line, Tool::Text],
            &[Tool::Slice, Tool::Hotspot],
            &[Tool::Zoom, Tool::Hand],
        ];
        ui.add_space(4.0);
        for (index, group) in groups.iter().enumerate() {
            if index > 0 {
                ui.separator();
            }
            for tool in *group {
                self.tool_button(ui, *tool);
            }
        }
        ui.separator();
        self.autoshape_button(ui);
    }

    /// One painted tool button; clicking picks the tool.
    fn tool_button(&mut self, ui: &mut egui::Ui, tool: Tool) {
        let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(26.0), Sense::click());
        let active = self.editor.tool == tool;
        if response.clicked() {
            self.editor.tool = tool;
            self.status = format!("tool: {}", tool_label(tool));
        }
        // CS6's tool-button double-clicks: Zoom resets to 100%, Hand
        // fits the page in the window.
        if response.double_clicked() {
            match tool {
                Tool::Zoom => self.run_shell(menus::ShellAction::ZoomTo(1.0)),
                Tool::Hand => self.run_shell(menus::ShellAction::FitAll),
                _ => {}
            }
        }
        let painter = ui.painter();
        if active {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x2f, 0x5d, 0x8a));
        } else if response.hovered() {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
        }
        let ink = if active {
            Color32::from_rgb(0xe8, 0xe8, 0xe8)
        } else {
            Color32::from_rgb(0xc4, 0xc4, 0xc4)
        };
        draw_tool_glyph(painter, rect, tool, ink);
        let key = tool_key(tool);
        if key.is_empty() {
            response.on_hover_text(tool_label(tool));
        } else {
            response.on_hover_text(format!("{} ({key})", tool_label(tool)));
        }
    }

    /// The Auto Shape flyout: the star button toggles a popup listing
    /// every shape with its preview glyph; picking one selects the
    /// tool and closes the flyout.
    fn autoshape_button(&mut self, ui: &mut egui::Ui) {
        let shapes = AutoShape::ALL;
        let current = match self.editor.tool {
            Tool::AutoShape(shape) => Some(shape),
            _ => None,
        };
        let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(26.0), Sense::click());
        let mut open = self.autoshape_flyout;
        if response.clicked() {
            open = !open;
            if open && current.is_none() {
                self.editor.tool = Tool::AutoShape(shapes[0]);
            }
        }
        let painter = ui.painter();
        let active = current.is_some();
        if active {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x2f, 0x5d, 0x8a));
        } else if response.hovered() {
            painter.rect_filled(rect, 2.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
        }
        let ink = if active {
            Color32::from_rgb(0xe8, 0xe8, 0xe8)
        } else {
            Color32::from_rgb(0xc4, 0xc4, 0xc4)
        };
        let shown = current.unwrap_or(shapes[0]);
        draw_tool_glyph(painter, rect, Tool::AutoShape(shown), ink);
        let tip = match current {
            Some(shape) => format!("Auto Shape — {}", shape.label()),
            None => "Auto Shape".to_owned(),
        };
        let response = response.on_hover_text(tip);

        let mut picked: Option<AutoShape> = None;
        egui::Popup::from_response(&response)
            .open_bool(&mut open)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                for shape in shapes {
                    let (row, row_response) =
                        ui.allocate_exact_size(egui::Vec2::new(150.0, 20.0), Sense::click());
                    let selected = current == Some(shape);
                    let rp = ui.painter();
                    if selected {
                        rp.rect_filled(row, 2.0, Color32::from_rgb(0x2f, 0x5d, 0x8a));
                    } else if row_response.hovered() {
                        rp.rect_filled(row, 2.0, Color32::from_rgb(0x4a, 0x4a, 0x4a));
                    }
                    let ink = if selected || row_response.hovered() {
                        Color32::from_rgb(0xe8, 0xe8, 0xe8)
                    } else {
                        Color32::from_rgb(0xd9, 0xd9, 0xd9)
                    };
                    draw_autoshape_glyph(
                        rp,
                        Rect::from_min_size(
                            Pos2::new(row.left() + 4.0, row.center().y - 8.0),
                            egui::Vec2::splat(16.0),
                        ),
                        shape,
                        ink,
                    );
                    rp.text(
                        Pos2::new(row.left() + 26.0, row.center().y),
                        egui::Align2::LEFT_CENTER,
                        shape.label(),
                        egui::FontId::proportional(12.0),
                        ink,
                    );
                    if row_response.clicked() {
                        picked = Some(shape);
                    }
                }
            });
        self.autoshape_flyout = open;
        if let Some(shape) = picked {
            self.editor.tool = Tool::AutoShape(shape);
            self.autoshape_flyout = false;
            self.status = format!("tool: Auto Shape — {}", shape.label());
        }
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
        // Remembered for view commands that arrive outside the canvas
        // (menu zoom/fit, keyboard shortcuts) — the anchor is the
        // canvas center of the last laid-out frame.
        self.canvas_rect = Some(rect);

        // Per-tool cursor icons (CS6 semantics); space held anywhere
        // switches to the temporary Hand (the CS6 space-drag pan).
        let space_held = ui.input(|input| input.key_down(egui::Key::Space));
        let cursor = if space_held {
            egui::CursorIcon::Grab
        } else {
            match self.editor.tool {
                Tool::Pointer | Tool::Subselect => egui::CursorIcon::Default,
                Tool::Rect | Tool::Ellipse | Tool::Line | Tool::Slice | Tool::Hotspot => {
                    egui::CursorIcon::Crosshair
                }
                Tool::Text => egui::CursorIcon::Text,
                Tool::Zoom => egui::CursorIcon::ZoomIn,
                Tool::Hand => egui::CursorIcon::Grab,
                Tool::AutoShape(_) => egui::CursorIcon::Crosshair,
            }
        };
        let response = response.on_hover_cursor(cursor);

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
        if let Some(drag) = &self.editor.drag {
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
                    None => {
                        // Space-drag pan: no tool drag begins while the
                        // temporary Hand is active.
                        if !space_held {
                            match self.editor.tool {
                                Tool::Pointer | Tool::Subselect => {
                                    // A press selects its target first,
                                    // then moves the selection live —
                                    // or (Alt) clones it, CS6 pointer
                                    // semantics. Empty canvas starts a
                                    // (no-op) rubber band.
                                    if let Some(hit) = self.editor.hit_at(world) {
                                        let ids = if self.editor.selection_set.contains(&hit) {
                                            self.editor.live_selection()
                                        } else {
                                            self.editor.selection = Some(hit);
                                            self.editor.selection_set = vec![hit];
                                            vec![hit]
                                        };
                                        let outcome = if ui.input(|i| i.modifiers.alt) {
                                            self.editor
                                                .begin_clone_drag(world, ids)
                                                .map(|_created| ())
                                        } else {
                                            self.editor.begin_move_drag(world, ids)
                                        };
                                        if let Err(error) = outcome {
                                            self.status = format!("drag failed: {error}");
                                        }
                                    } else {
                                        self.editor.begin_drag(world);
                                    }
                                }
                                _ => self.editor.begin_drag(world),
                            }
                        }
                    }
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
                    || space_held
                    || response.dragged_by(egui::PointerButton::Middle)
                {
                    // Hand tool, space-drag pan, or middle-button pan.
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
        // Double-click enters a symbol instance's definition (CS6's
        // pointer double-click), like the inspector's Edit Definition.
        if response.double_clicked()
            && matches!(self.editor.tool, Tool::Pointer)
            && let Some(pos) = response.interact_pointer_pos()
        {
            let world = self.view.to_world(pos - rect.left_top().to_vec2());
            let entered = self.editor.hit_at(world).and_then(|id| {
                self.editor
                    .document()
                    .registry
                    .get(id)
                    .map(|data| (id, data))
            });
            if let Some((_id, ObjectData::SymbolInstance(instance))) = entered {
                let number = instance.symbol_number;
                match self.editor.enter_symbol(number) {
                    Ok(()) => self.status = format!("editing symbol {number}"),
                    Err(error) => self.status = format!("enter symbol failed: {error}"),
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
        // Menu/shortcut actions run before layout so their effects are
        // visible the same frame.
        self.shortcuts(ui.ctx());
        // Panels first, CentralPanel last (egui's required order), in
        // the CS6 arrangement: menu, tool strip, panel stack,
        // bottom-docked Properties, status strip, document canvas.
        egui::Panel::top(egui::Id::new("menu")).show(ui, |ui| self.menu_bar(ui));
        egui::Panel::left(egui::Id::new("tools"))
            .exact_size(34.0)
            .show(ui, |ui| self.tools_panel(ui));
        egui::Panel::right(egui::Id::new("panels"))
            .default_size(230.0)
            .size_range(160.0..=420.0)
            .resizable(true)
            .show(ui, |ui| self.right_panels(ui));
        egui::Panel::bottom(egui::Id::new("properties"))
            .default_size(210.0)
            .size_range(120.0..=360.0)
            .resizable(true)
            .show(ui, |ui| self.properties_panel(ui));
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
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                // The document tab strip: a single-document window, so
                // the CS6 MDI tab strip reduced to its one member.
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Button::new(egui::RichText::new(&self.doc_name).size(11.0))
                            .selected(true),
                    );
                    ui.weak("FireCraft");
                });
                self.canvas(ui);
            });
        if self.export_dialog.is_some() {
            egui::Window::new("Export")
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| self.export_dialog_ui(ui));
        }
        if self.close_requested {
            self.close_requested = false;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
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

    /// A shell with scripted dialogs over a memory filesystem.
    fn gui_shell(editor: Editor) -> Shell {
        let dialogs = FixedDialogs {
            steps: "/mem/steps.fwsteps".to_owned(),
            doc: "/mem/doc.fw.png".to_owned(),
        };
        Shell::with_platform(editor, Box::new(MemoryFs::new()), Box::new(dialogs))
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
        let mut shell = Shell::with_platform(editor, Box::new(MemoryFs::new()), Box::new(dialogs));
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
}
