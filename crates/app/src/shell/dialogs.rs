//! Dialog types and the import/export plumbing on [`Shell`].

use super::*;
use engine::EngineError;

/// The display quartet plus name, captured read-only from any
/// property-bearing kind.
pub(crate) struct DisplayState {
    pub(super) opacity: i64,
    pub(super) blend: i64,
    pub(super) visible: bool,
    pub(super) locked: bool,
    pub(super) name: Option<String>,
}

/// One Web Layer panel row: id, display name, is-foreground-slice,
/// link, alt text, and the raw `TSLi` behind the foreground toggle.
pub(crate) type WebRow = (
    doc::ObjectId,
    String,
    bool,
    Option<String>,
    Option<String>,
    i64,
);

/// The Numeric Transform dialog's fields — CS6's Modify ▸ Transform ▸
/// Numeric Transform: a uniform or per-axis scale, then a rotate and a
/// skew, composed about the selection bounds center and applied as one
/// transaction step (`scale` × `rotate` × `skew`, in that order).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NumericTransformDialog {
    /// Horizontal scale percent (100 = unchanged).
    pub scale_x: f64,
    /// Vertical scale percent.
    pub scale_y: f64,
    /// Uniform-scale toggle: `scale_y` follows `scale_x`.
    pub uniform: bool,
    /// Clockwise rotation in degrees.
    pub rotate: f64,
    /// Horizontal skew angle in degrees.
    pub skew_x: f64,
    /// Vertical skew angle in degrees.
    pub skew_y: f64,
}

impl Default for NumericTransformDialog {
    fn default() -> Self {
        Self {
            scale_x: 100.0,
            scale_y: 100.0,
            uniform: true,
            rotate: 0.0,
            skew_x: 0.0,
            skew_y: 0.0,
        }
    }
}

/// The Feather dialog's field — Select ▸ Feather…: softens the active
/// pixel selection's edge by a radius in pixels (1..=64, the engine's
/// kernel cap).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FeatherDialog {
    /// Feather radius in pixels.
    pub radius: u8,
}

/// The Canvas Size dialog's fields — Modify ▸ Canvas ▸ Canvas Size:
/// absolute width/height in document units plus the CS6 anchor grid
/// (which corner the resize grows from).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CanvasSizeDialog {
    /// New canvas width in document units.
    pub width: f64,
    /// New canvas height in document units.
    pub height: f64,
    /// Anchor column 0..=2 (left/center/right).
    pub anchor_x: usize,
    /// Anchor row 0..=2 (top/middle/bottom).
    pub anchor_y: usize,
}

/// The Canvas Color dialog's field — Modify ▸ Canvas ▸ Canvas Color:
/// the document-wide matte (`document.setCanvasColor`), ABGR raw.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CanvasColorDialog {
    /// Matte color, sRGB bytes (r, g, b).
    pub color: [u8; 3],
}

impl Default for CanvasSizeDialog {
    fn default() -> Self {
        Self {
            width: 640.0,
            height: 480.0,
            anchor_x: 1,
            anchor_y: 1,
        }
    }
}

impl Default for CanvasColorDialog {
    fn default() -> Self {
        Self {
            color: [255, 255, 255],
        }
    }
}

impl Default for FeatherDialog {
    fn default() -> Self {
        Self { radius: 2 }
    }
}

/// The Image Size dialog's fields — Modify ▸ Image Size (CS6
/// command 35226): scales the document into the target rectangle
/// (`document.setImageSize`) and sets the resolution.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ImageSizeDialog {
    /// New width in document units.
    pub width: f64,
    /// New height in document units.
    pub height: f64,
    /// Pixels per unit for the document resolution.
    pub resolution: f64,
    /// Whether the resolution unit is cm (false = inch).
    pub unit_cm: bool,
}

impl Default for ImageSizeDialog {
    fn default() -> Self {
        Self {
            width: 640.0,
            height: 480.0,
            resolution: 72.0,
            unit_cm: false,
        }
    }
}

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

    fn open_url(&self, url: &str) -> bool {
        if url.split("://").next() == Some(url) || url.contains("://") {
            // Best-effort desktop handoff; false when no launcher ran.
            std::process::Command::new("xdg-open")
                .arg(url)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
        } else {
            false
        }
    }
}

/// The export formats the dialog offers, in menu order. The index is
/// the document's `FMTi` export-format id, so new formats append —
/// reordering would re-point every saved document's setting.
pub(crate) const EXPORT_FORMATS: &[&str] = &[
    "png", "jpg", "bmp", "tiff", "wbmp", "svg", "psd", "gif", "webp",
];

/// The web export workflows the dialog offers (the M11 set), in menu
/// order — the same names the CLI's `--workflow` flag takes.
pub(crate) const WEB_WORKFLOWS: &[&str] = &[
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
pub(crate) fn web_workflow(index: usize) -> Option<webexport::Workflow> {
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
pub(crate) struct ExportDialog {
    /// Index into [`EXPORT_FORMATS`].
    pub(super) format: usize,
    /// Selected web workflow, or `None` for the raster formats.
    pub(super) web: Option<usize>,
    /// Emit the 1px shim row/column (web table workflows).
    pub(super) web_shim: bool,
    /// Give areas without slices their own image (`include_undefined`).
    pub(super) web_undefined: bool,
    /// JPEG quality 1..=100.
    pub(super) quality: i32,
    /// Progressive JPEG scans.
    pub(super) progressive: bool,
    /// Matte for alpha-less formats: false = white, true = black.
    pub(super) matte_black: bool,
    /// GIF loop: 0 infinite, 1 never, 2 repeat n times.
    pub(super) gif_loop: usize,
    /// Repeat count when `gif_loop == 2`.
    pub(super) gif_times: i32,
    /// GIF per-frame ink crop (`AACb`).
    pub(super) gif_crop: bool,
    /// GIF diff frames (`AADb`).
    pub(super) gif_diff: bool,
    /// GIF Floyd–Steinberg amount (`PLDi`), percent.
    pub(super) gif_dither: i32,
    /// Cached 2-Up panes and the settings stamp they were made from.
    pub(super) preview: Option<(u64, TextureHandle, String, TextureHandle, String)>,
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

impl Shell {
    /// Whether the current document holds anything — the test that
    /// separates "open this file" from "place this file".
    pub(super) fn document_has_content(&self) -> bool {
        self.editor
            .current_cell()
            .is_some_and(|cell| !cell.elements.is_empty())
    }

    /// Places a raster into the current document as a new image
    /// object (CS6's File → Import): at its natural size, centred on
    /// the page canvas, scaled down to fit when it is larger than the
    /// canvas.
    pub(super) fn place_raster_into_document(
        &mut self,
        bytes: &[u8],
        name: &str,
    ) -> Result<(), String> {
        let decoded = io::raster::decode(bytes).map_err(|error| error.to_string())?;
        let canvas = self
            .editor
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
            .ok_or_else(|| "no page to place into".to_owned())?;
        let image = geom::Rect::new(
            0.0,
            0.0,
            f64::from(decoded.width),
            f64::from(decoded.height),
        );
        // The placement rectangle: the canvas when the image has to be
        // scaled, otherwise a degenerate rectangle at the centred
        // top-left corner (the unscaled placement law).
        let (bounds, maintain_aspect_ratio) =
            if image.width() > canvas.width() || image.height() > canvas.height() {
                (
                    serde_json::json!({
                        "left": canvas.x0,
                        "top": canvas.y0,
                        "right": canvas.x1,
                        "bottom": canvas.y1,
                    }),
                    true,
                )
            } else {
                (
                    serde_json::json!({
                        "left": canvas.x0 + (canvas.width() - image.width()) / 2.0,
                        "top": canvas.y0 + (canvas.height() - image.height()) / 2.0,
                        "right": canvas.x0 + (canvas.width() - image.width()) / 2.0,
                        "bottom": canvas.y0 + (canvas.height() - image.height()) / 2.0,
                    }),
                    true,
                )
            };
        let payload = serde_json::json!({
            "width": decoded.width,
            "height": decoded.height,
            "pixels": base64::engine::general_purpose::STANDARD.encode(&decoded.rgba),
            "bounds": bounds,
            "maintain_aspect_ratio": maintain_aspect_ratio,
            "page": self.editor.page,
            "layer": self.editor.active_layer(),
            "cell": self.editor.frame,
            "name": name,
        });
        self.editor
            .apply("bitmap.importFile", payload)
            .map_err(|error| error.to_string())
    }

    /// File → Import…: place a raster into the current document, on
    /// the active layer.
    pub(super) fn import_raster_dialog(&mut self) {
        let Some(path) = self.dialogs.open_path(
            "Import image",
            &[(
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
            )],
        ) else {
            return;
        };
        let bytes = match self.fs.read_file(&path).map_err(|e| e.to_string()) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("import failed: {error}");
                return;
            }
        };
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        match self.import_raster(&bytes, &name) {
            Ok(placed) => {
                self.status = format!("imported {name} ({placed})");
            }
            Err(error) => {
                self.status = format!("import failed: {error}");
            }
        }
    }

    /// Decodes `bytes` and places the raster on the active layer of
    /// the current page. A source larger than the canvas is scaled to
    /// fit it (aspect kept); a smaller one lands unscaled at the
    /// canvas corner, the way CS6 imports it.
    pub(super) fn import_raster(&mut self, bytes: &[u8], name: &str) -> Result<String, String> {
        let decoded = io::raster::decode(bytes).map_err(|error| error.to_string())?;
        let (page, layer, cell) = (
            self.editor.page,
            self.editor.active_layer(),
            self.editor.frame,
        );
        let canvas = self
            .editor
            .document()
            .pages
            .get(page)
            .map(|page| page.info.canvas.clone())
            .ok_or_else(|| "the document has no such page".to_owned())?;
        let fits =
            f64::from(decoded.width) <= canvas.width && f64::from(decoded.height) <= canvas.height;
        let bounds = if fits {
            // A degenerate rectangle places the file unscaled with
            // its top-left corner at that point.
            serde_json::json!({
                "left": canvas.x,
                "top": canvas.y,
                "right": canvas.x,
                "bottom": canvas.y,
            })
        } else {
            serde_json::json!({
                "left": canvas.x,
                "top": canvas.y,
                "right": canvas.x + canvas.width,
                "bottom": canvas.y + canvas.height,
            })
        };
        let payload = serde_json::json!({
            "width": decoded.width,
            "height": decoded.height,
            "pixels": base64::engine::general_purpose::STANDARD.encode(&decoded.rgba),
            "bounds": bounds,
            "maintain_aspect_ratio": true,
            "page": page,
            "layer": layer,
            "cell": cell,
            "name": name,
        });
        self.editor
            .apply("bitmap.importFile", payload)
            .map_err(|error| error.to_string())?;
        Ok(format!("{}x{}", decoded.width, decoded.height))
    }

    /// Opens a plain raster as its own document: a fresh one-page
    /// document whose canvas matches the image, with the image on the
    /// page's layer.
    pub(super) fn open_raster_document(&mut self, bytes: &[u8], name: &str) -> Result<(), String> {
        let decoded = io::raster::decode(bytes).map_err(|error| error.to_string())?;
        let mut editor = Editor::untitled();
        let _ = editor.apply(
            "page.setCanvasSize",
            serde_json::json!({
                "page": 0,
                "width": decoded.width,
                "height": decoded.height,
            }),
        );
        let payload = serde_json::json!({
            "width": decoded.width,
            "height": decoded.height,
            "pixels": base64::engine::general_purpose::STANDARD.encode(&decoded.rgba),
            "bounds": {
                "left": 0.0,
                "top": 0.0,
                "right": 0.0,
                "bottom": 0.0,
            },
            "maintain_aspect_ratio": true,
            "page": 0,
            "layer": 0,
            "cell": 0,
            "name": name,
        });
        editor
            .apply("bitmap.importFile", payload)
            .map_err(|error| error.to_string())?;
        // The construction applies are not user edits: a just-opened
        // file is clean, or Close would refuse it.
        editor.session.mark_clean();
        self.editor = editor;
        Ok(())
    }

    /// The Web Layer panel: the page's slices and hotspots with link
    /// editing (`slice.setLink`) and the foreground/background toggle
    /// (`slice.setType`). Snapshots the web objects first so the edit
    /// fields never borrow the document while commands run.
    pub(super) fn web_panel(&mut self, ui: &mut egui::Ui) {
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

    /// The export dialog (File → Export…): format + per-format
    /// controls, a 2-Up preview of the chosen settings against a
    /// lossless PNG pane, and Export through the save dialog.
    pub(super) fn export_dialog_ui(&mut self, ui: &mut egui::Ui) {
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

    /// The Numeric Transform dialog: per-field scale/rotate/skew,
    /// applied to the live selection as one composed transaction step.
    pub(super) fn numeric_transform_dialog_ui(&mut self, ui: &mut egui::Ui) {
        let Some(mut dialog) = self.numeric_transform.take() else {
            return;
        };
        egui::Grid::new("numeric_transform")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.add(
                    egui::DragValue::new(&mut dialog.scale_x)
                        .range(0.0..=10_000.0)
                        .suffix("%"),
                );
                ui.label("Scale X");
                if dialog.uniform {
                    dialog.scale_y = dialog.scale_x;
                }
                ui.add_enabled(
                    !dialog.uniform,
                    egui::DragValue::new(&mut dialog.scale_y)
                        .range(0.0..=10_000.0)
                        .suffix("%"),
                );
                ui.label("Scale Y");
                ui.checkbox(&mut dialog.uniform, "Uniform");
                ui.add(
                    egui::DragValue::new(&mut dialog.rotate)
                        .range(-360.0..=360.0)
                        .suffix("°"),
                );
                ui.label("Rotate");
                ui.add(
                    egui::DragValue::new(&mut dialog.skew_x)
                        .range(-89.0..=89.0)
                        .suffix("°"),
                );
                ui.label("Skew X");
                ui.add(
                    egui::DragValue::new(&mut dialog.skew_y)
                        .range(-89.0..=89.0)
                        .suffix("°"),
                );
                ui.label("Skew Y");
            });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("OK").clicked() {
                self.apply_numeric_transform(&dialog);
            } else {
                self.status = "numeric transform cancelled".to_owned();
            }
        });
        self.numeric_transform = Some(dialog);
    }

    /// The Feather dialog: the radius slider over the active pixel
    /// selection. OK applies `bitmap.feather` and closes; Cancel
    /// closes without touching the document.
    pub(super) fn feather_dialog_ui(&mut self, ui: &mut egui::Ui) {
        let Some(dialog) = self.feather.take() else {
            return;
        };
        let mut radius = f32::from(dialog.radius);
        let mut apply = false;
        ui.add(egui::Slider::new(&mut radius, 1.0..=64.0).text("feather radius"));
        let dialog = FeatherDialog {
            radius: radius.round().clamp(1.0, 64.0) as u8,
        };
        ui.separator();
        ui.horizontal(|ui| {
            apply |= ui.button("OK").clicked();
            if ui.button("Cancel").clicked() {
                self.feather = None;
                self.status = "feather cancelled".to_owned();
            }
        });
        if apply {
            self.feather = None;
            self.apply_feather(dialog.radius);
            return;
        }
        self.feather = Some(dialog);
    }

    /// Applies the feather radius to the active pixel selection and
    /// records the radius as the next dialog's default. The dialog
    /// closes either way — a refusal lands on the status line.
    pub(super) fn apply_feather(&mut self, radius: u8) {
        self.feather = None;
        if let Err(error) = self.editor.feather_selection(radius) {
            self.status = format!("feather failed: {error}");
            return;
        }
        self.editor.feather_radius = radius;
        self.status = format!("feathered selection by {radius}px");
    }

    /// The Canvas Size dialog: absolute width/height plus the CS6
    /// 3×3 anchor grid. OK applies `page.setCanvasSize` to page 0
    /// (the shell is single-page); Cancel closes without touching
    /// the document.
    pub(super) fn canvas_size_dialog_ui(&mut self, ui: &mut egui::Ui) {
        let Some(dialog) = self.canvas_size.take() else {
            return;
        };
        let mut dialog = dialog;
        egui::Grid::new("canvas_size")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.add(
                    egui::DragValue::new(&mut dialog.width)
                        .range(1.0..=65_535.0)
                        .suffix(" px"),
                );
                ui.label("Width");
                ui.add(
                    egui::DragValue::new(&mut dialog.height)
                        .range(1.0..=65_535.0)
                        .suffix(" px"),
                );
                ui.label("Height");
            });
        ui.separator();
        ui.label("Anchor");
        ui.horizontal(|ui| {
            for (row, label) in ["top", "middle", "bottom"].iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_min_width(60.0);
                    for (col, _) in ["left", "center", "right"].iter().enumerate() {
                        let selected = dialog.anchor_x == col && dialog.anchor_y == row;
                        if ui.selectable_label(selected, *label).clicked() {
                            dialog.anchor_x = col;
                            dialog.anchor_y = row;
                        }
                    }
                });
            }
        });
        ui.separator();
        let mut apply = false;
        ui.horizontal(|ui| {
            apply |= ui.button("OK").clicked();
            if ui.button("Cancel").clicked() {
                self.canvas_size = None;
                self.status = "canvas size cancelled".to_owned();
            }
        });
        if apply {
            self.canvas_size = None;
            self.apply_canvas_size(&dialog);
            return;
        }
        self.canvas_size = Some(dialog);
    }

    /// Applies the canvas resize anchored per the dialog's grid:
    /// the anchor corner stays fixed while the opposite edges move,
    /// and every page-0 object shifts by the same offset (CS6
    /// `dom.setDocumentCanvasSize` behavior).
    pub(super) fn apply_canvas_size(&mut self, dialog: &CanvasSizeDialog) {
        self.canvas_size = None;
        let document = self.editor.session.document();
        let canvas = &document.pages[0].info.canvas;
        let dx = dialog.width - canvas.width;
        let dy = dialog.height - canvas.height;
        // Fraction of the growth that lands left/above the fixed anchor.
        let (fx, fy) = match (dialog.anchor_x, dialog.anchor_y) {
            (0, 0) => (0.0, 0.0),
            (1, 0) => (0.5, 0.0),
            (2, 0) => (1.0, 0.0),
            (0, 1) => (0.0, 0.5),
            (1, 1) => (0.5, 0.5),
            (2, 1) => (1.0, 0.5),
            (0, 2) => (0.0, 1.0),
            (1, 2) => (0.5, 1.0),
            _ => (1.0, 1.0),
        };
        let shift_x = -dx * fx;
        let shift_y = -dy * fy;
        let ids: Vec<_> = document.pages[0]
            .layers
            .layers
            .iter()
            .flat_map(|layer| &layer.cells)
            .flat_map(|cell| &cell.elements)
            .copied()
            .collect();
        if !self.run_command(
            "page.setCanvasSize",
            serde_json::json!({"page": 0, "width": dialog.width, "height": dialog.height}),
        ) {
            return;
        }
        if !(shift_x == 0.0 && shift_y == 0.0) && !ids.is_empty() {
            self.run_command(
                "object.translate",
                serde_json::json!({"ids": ids, "dx": shift_x, "dy": shift_y}),
            );
        }
        self.status = format!("canvas resized to {} × {}", dialog.width, dialog.height);
    }

    /// The Image Size dialog: target width/height plus the document
    /// resolution. OK applies `document.setImageSize`; Cancel closes
    /// without touching the document.
    pub(super) fn image_size_dialog_ui(&mut self, ui: &mut egui::Ui) {
        let Some(dialog) = self.image_size.take() else {
            return;
        };
        let mut dialog = dialog;
        egui::Grid::new("image_size")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.add(
                    egui::DragValue::new(&mut dialog.width)
                        .range(1.0..=65_535.0)
                        .suffix(" px"),
                );
                ui.label("Width");
                ui.add(
                    egui::DragValue::new(&mut dialog.height)
                        .range(1.0..=65_535.0)
                        .suffix(" px"),
                );
                ui.label("Height");
                ui.add(egui::DragValue::new(&mut dialog.resolution).range(1.0..=10_000.0));
                ui.horizontal(|ui| {
                    if ui.selectable_label(!dialog.unit_cm, "px/in").clicked() {
                        dialog.unit_cm = false;
                    }
                    if ui.selectable_label(dialog.unit_cm, "px/cm").clicked() {
                        dialog.unit_cm = true;
                    }
                });
                ui.label("Resolution");
            });
        ui.separator();
        let mut apply = false;
        ui.horizontal(|ui| {
            apply |= ui.button("OK").clicked();
            if ui.button("Cancel").clicked() {
                self.image_size = None;
                self.status = "image size cancelled".to_owned();
            }
        });
        if apply {
            self.image_size = None;
            self.apply_image_size(&dialog);
            return;
        }
        self.image_size = Some(dialog);
    }

    /// Applies the image-size scale: the target rect is the new
    /// document bounds (left/top-anchored, the corpus `BatchTemplate`
    /// convention) and the resolution rides along.
    pub(super) fn apply_image_size(&mut self, dialog: &ImageSizeDialog) {
        self.image_size = None;
        if self.run_command(
            "document.setImageSize",
            serde_json::json!({
                "rect": {
                    "left": 0.0, "top": 0.0,
                    "right": dialog.width, "bottom": dialog.height,
                },
                "resolution": dialog.resolution,
                "unit": if dialog.unit_cm { "cm" } else { "inch" },
                "page": 0,
            }),
        ) {
            self.status = format!(
                "image resized to {} × {} @ {} px/{}",
                dialog.width,
                dialog.height,
                dialog.resolution,
                if dialog.unit_cm { "cm" } else { "in" }
            );
        }
    }

    /// The Canvas Color dialog: an egui color picker over the
    /// document matte. OK applies `document.setCanvasColor`; Cancel
    /// closes without touching the document.
    pub(super) fn canvas_color_dialog_ui(&mut self, ui: &mut egui::Ui) {
        let Some(dialog) = self.canvas_color.take() else {
            return;
        };
        let mut dialog = dialog;
        let mut color = egui::Color32::from_rgb(dialog.color[0], dialog.color[1], dialog.color[2]);
        ui.horizontal(|ui| {
            ui.label("Canvas color");
            ui.color_edit_button_srgba(&mut color);
        });
        dialog.color = [color.r(), color.g(), color.b()];
        ui.separator();
        let mut apply = false;
        ui.horizontal(|ui| {
            apply |= ui.button("OK").clicked();
            if ui.button("Cancel").clicked() {
                self.canvas_color = None;
                self.status = "canvas color cancelled".to_owned();
            }
        });
        if apply {
            self.canvas_color = None;
            self.apply_canvas_color(&dialog);
            return;
        }
        self.canvas_color = Some(dialog);
    }

    /// Applies the matte color document-wide (one undo step).
    pub(super) fn apply_canvas_color(&mut self, dialog: &CanvasColorDialog) {
        self.canvas_color = None;
        let [r, g, b] = dialog.color;
        // ABGR raw: fully opaque, R in the low byte.
        let raw = 0xFF00_0000 | (u32::from(b) << 16) | (u32::from(g) << 8) | u32::from(r);
        if self.run_command(
            "document.setCanvasColor",
            serde_json::json!({"color_raw": raw as i64}),
        ) {
            self.status = "canvas color set".to_owned();
        }
    }

    /// Composes scale → rotate → skew about the selection bounds
    /// center and applies the three commands inside one "Transform"
    /// transaction — one undo step for the whole dialog.
    pub(super) fn apply_numeric_transform(&mut self, dialog: &NumericTransformDialog) {
        let ids = self.editor.live_selection();
        if ids.is_empty() {
            self.status = "nothing selected".to_owned();
            return;
        }
        if self.editor.session.begin_transaction("Transform").is_err() {
            self.status = "transform failed: a transaction is already open".to_owned();
            return;
        }
        let result = (|| -> Result<(), EngineError> {
            let (sx, sy) = (
                dialog.scale_x / 100.0,
                if dialog.uniform {
                    dialog.scale_x / 100.0
                } else {
                    dialog.scale_y / 100.0
                },
            );
            if (sx - 1.0).abs() > 1e-9 || (sy - 1.0).abs() > 1e-9 {
                self.editor.session.apply_delta(
                    "object.scale",
                    serde_json::json!({ "ids": ids, "sx": sx, "sy": sy, "pivot": null }),
                )?;
            }
            if dialog.rotate.abs() > 1e-9 {
                self.editor.session.apply_delta(
                    "object.rotate",
                    serde_json::json!({ "ids": ids, "degrees": dialog.rotate, "pivot": null }),
                )?;
            }
            if dialog.skew_x.abs() > 1e-9 || dialog.skew_y.abs() > 1e-9 {
                self.editor.session.apply_delta(
                    "object.skew",
                    serde_json::json!({
                        "ids": ids,
                        "skew_x": dialog.skew_x, "skew_y": dialog.skew_y,
                        "pivot": null,
                    }),
                )?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                let _ = self.editor.session.commit_transaction();
                self.numeric_transform = None;
                self.status = "transformed selection".to_owned();
            }
            Err(error) => {
                let _ = self.editor.session.abort_transaction();
                self.numeric_transform = None;
                self.status = format!("transform failed: {error}");
            }
        }
    }
    /// A change stamp over the dialog settings plus the document, so
    /// the preview regenerates exactly when either could have moved.
    pub(super) fn preview_stamp(&self) -> u64 {
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
    pub(super) fn dialog_options(dialog: &ExportDialog) -> doc::ExportOptions {
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
    pub(super) fn dialog_flags(dialog: &ExportDialog) -> app_cli::ops::ExportFlags {
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
    pub(super) fn encode_with_dialog(
        &self,
        dialog: &ExportDialog,
        format: &str,
    ) -> Result<Vec<u8>, String> {
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
    pub(super) fn render_preview(
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
    pub(super) fn export_via_dialog(&mut self, dialog: &ExportDialog) {
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
    pub(super) fn export_web_via_dialog(&mut self, dialog: &ExportDialog, index: usize) {
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
}
