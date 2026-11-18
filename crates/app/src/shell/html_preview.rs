//! The live HTML preview: the document's real web export, rendered
//! inside the app.
//!
//! The preview is the export, not a mock-up: it runs the same
//! [`app_cli::ops::export_web`] path the CLI verb and the Export
//! dialog run, parses the markup the exporter wrote, and draws the
//! exported image files at the positions that markup carries. The
//! markup is shown verbatim under the rendering, so what the panel
//! draws and what the exporter writes can be compared line by line.
//!
//! The panel's cache lives in a thread-local rather than on the
//! shell: the preview is a view of the document, not document state,
//! and it is rebuilt whenever the session's change stamp moves.

use std::cell::RefCell;

use super::*;
use crate::shell::panel_windows::DOCK_CONTENT_WIDTH;

/// The export workflow the preview renders: CS6's "Export HTML" —
/// the shim table plus the image files it references.
const WORKFLOW: webexport::Workflow = webexport::Workflow::HtmlAndImages;

/// The base name the export stamps onto its auto-named slices.
const EXPORT_NAME: &str = "preview";

/// One `<img>` the exported markup carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreviewImage {
    /// The `src` attribute — the exported file name.
    pub(crate) src: String,
    /// The `width` attribute, in pixels.
    pub(crate) width: u32,
    /// The `height` attribute, in pixels.
    pub(crate) height: u32,
}

/// One `<td>` of the exported table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreviewCell {
    /// The image the cell carries.
    pub(crate) image: Option<PreviewImage>,
    /// The cell's `colspan`.
    pub(crate) col_span: usize,
    /// The cell's `rowspan`.
    pub(crate) row_span: usize,
}

/// A cell of the exported table, placed on the page.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlacedCell {
    /// The cell's origin, in export pixels.
    pub(crate) x: f32,
    /// The cell's origin, in export pixels.
    pub(crate) y: f32,
    /// The image the cell carries.
    pub(crate) image: Option<PreviewImage>,
}

/// The exported table, placed.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct PreviewLayout {
    /// The cells, in row-major order.
    pub(crate) cells: Vec<PlacedCell>,
    /// The table's width, in export pixels.
    pub(crate) width: f32,
    /// The table's height, in export pixels.
    pub(crate) height: f32,
}

/// One exported file, decoded for drawing.
struct PreviewFile {
    /// The exported file name.
    name: String,
    /// The decoded width, in pixels.
    width: u32,
    /// The decoded height, in pixels.
    height: u32,
    /// The straight-alpha RGBA pixels.
    pixels: Vec<u8>,
}

/// The cached export the panel draws.
struct PreviewCache {
    /// The exported markup, verbatim.
    html: String,
    /// The decoded image files, in export order.
    files: Vec<PreviewFile>,
    /// The placed table.
    layout: PreviewLayout,
    /// The export error, when the export failed.
    error: Option<String>,
}

/// The identity of the document a cached export was made from: the
/// session's id, plus the session's change stamp.
///
/// The session id is the document's identity — two sessions are two
/// documents even when their change stamps match, because a freshly
/// opened file and a freshly created one both carry an empty history.
type PreviewKey = (u64, usize, usize, usize);

/// The preview panel's state: the cached export and its textures.
struct PreviewState {
    /// The document the cache was made from.
    stamp: Option<PreviewKey>,
    /// The cached export.
    cache: Option<PreviewCache>,
    /// The textures of the cached files, parallel to `cache.files`.
    textures: Vec<TextureHandle>,
}

impl PreviewState {
    /// The empty state — nothing exported yet.
    fn new() -> Self {
        Self {
            stamp: None,
            cache: None,
            textures: Vec::new(),
        }
    }
}

thread_local! {
    /// The preview's per-thread state. The shell is single-threaded
    /// (the wasm build shares one thread), and the preview is a view
    /// of the document rather than document state, so it does not
    /// belong on the shell.
    static STATE: RefCell<PreviewState> = RefCell::new(PreviewState::new());
}

impl Shell {
    /// Window ▸ HTML Preview: the document's live web export, as the
    /// exporter writes it.
    ///
    /// The export runs through [`app_cli::ops::export_web`] — the
    /// CLI verb's and the Export dialog's exact path — so the panel
    /// cannot drift from what the export produces.
    pub(super) fn html_preview_panel(&mut self, ui: &mut egui::Ui) {
        let options = webexport::WebExportOptions {
            page: self.editor.page,
            state: self.editor.frame,
            doc_name: EXPORT_NAME.to_owned(),
            title: EXPORT_NAME.to_owned(),
            ..webexport::WebExportOptions::default()
        };
        let stamp = self.stamp();
        let key = (self.editor.session.id(), stamp.0, stamp.1, stamp.2);
        let stale = STATE.with(|state| state.borrow().stamp != Some(key));
        if stale {
            let cache = build_cache(self.editor.document(), &options);
            STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.stamp = Some(key);
                state.cache = Some(cache);
                state.textures.clear();
            });
        }
        ui.weak(format!(
            "HTML + images · page {} · state {}",
            self.editor.page + 1,
            self.editor.frame + 1
        ));
        ui.separator();
        STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            // The cache leaves the state for the draw: the textures
            // and the cache are separate borrows of the same state.
            let Some(cache) = state.cache.take() else {
                return;
            };
            if let Some(error) = &cache.error {
                ui.colored_label(
                    Color32::from_rgb(220, 120, 120),
                    format!("export failed: {error}"),
                );
                state.cache = Some(cache);
                return;
            }
            ensure_textures(&mut state.textures, &cache.files, ui.ctx());
            draw_page(ui, &cache.layout, &cache.files, &state.textures);
            ui.separator();
            ui.label(format!("{} image file(s)", cache.files.len()));
            let mut html = cache.html;
            ui.collapsing("HTML source", |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut html)
                        .desired_width(DOCK_CONTENT_WIDTH)
                        .desired_rows(10)
                        .interactive(false),
                );
            });
            state.cache = Some(PreviewCache {
                html,
                files: cache.files,
                layout: cache.layout,
                error: None,
            });
        });
    }
}

/// Runs the export and decodes its image files.
fn build_cache(document: &doc::Document, options: &webexport::WebExportOptions) -> PreviewCache {
    let export = match app_cli::ops::export_web(document, options, WORKFLOW) {
        Ok(export) => export,
        Err(error) => {
            return PreviewCache {
                html: String::new(),
                files: Vec::new(),
                layout: PreviewLayout::default(),
                error: Some(error.to_string()),
            };
        }
    };
    let html = export.html.clone().unwrap_or_default();
    let layout = layout(&parse_table(&html));
    let files = export
        .files
        .iter()
        .filter_map(|file| {
            let decoded = io::raster::decode(&file.bytes).ok()?;
            Some(PreviewFile {
                name: file.name.clone(),
                width: decoded.width,
                height: decoded.height,
                pixels: decoded.rgba,
            })
        })
        .collect();
    PreviewCache {
        html,
        files,
        layout,
        error: None,
    }
}

/// Creates the textures for the cached files, once per export.
fn ensure_textures(textures: &mut Vec<TextureHandle>, files: &[PreviewFile], ctx: &egui::Context) {
    if textures.len() == files.len() {
        return;
    }
    *textures = files
        .iter()
        .map(|file| {
            let image = ColorImage::from_rgba_unmultiplied(
                [file.width as usize, file.height as usize],
                &file.pixels,
            );
            ctx.load_texture(&file.name, image, TextureOptions::LINEAR)
        })
        .collect();
}

/// Draws the exported page: the images at the positions the markup
/// carries, scaled into the panel.
fn draw_page(
    ui: &mut egui::Ui,
    layout: &PreviewLayout,
    files: &[PreviewFile],
    textures: &[TextureHandle],
) {
    if layout.cells.is_empty() {
        ui.weak("No slices on this page — the export has no images.");
        return;
    }
    let scale = if layout.width > 0.0 {
        (DOCK_CONTENT_WIDTH / layout.width).min(1.0)
    } else {
        1.0
    };
    let size = egui::vec2(
        (layout.width * scale).max(1.0),
        (layout.height * scale).max(1.0),
    );
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
    for cell in &layout.cells {
        let Some(image) = &cell.image else { continue };
        let Some(index) = files.iter().position(|file| file.name == image.src) else {
            continue;
        };
        let Some(texture) = textures.get(index) else {
            continue;
        };
        let cell_rect = Rect::from_min_size(
            rect.left_top() + egui::vec2(cell.x * scale, cell.y * scale),
            egui::vec2(image.width as f32 * scale, image.height as f32 * scale),
        );
        ui.painter().image(
            texture.id(),
            cell_rect,
            Rect::from_min_size(Pos2::ZERO, Vec2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

/// The table rows of the exported markup.
pub(crate) fn parse_table(html: &str) -> Vec<Vec<PreviewCell>> {
    let mut rows = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<tr") {
        let after = &rest[start..];
        let Some(end) = after.find("</tr>") else {
            break;
        };
        rows.push(parse_row(&after[..end]));
        rest = &after[end + "</tr>".len()..];
    }
    rows
}

/// The cells of one table row.
fn parse_row(row: &str) -> Vec<PreviewCell> {
    let mut cells = Vec::new();
    let mut rest = row;
    while let Some(start) = rest.find("<td") {
        let after = &rest[start..];
        let Some(end) = after.find("</td>") else {
            break;
        };
        cells.push(parse_cell(&after[..end]));
        rest = &after[end + "</td>".len()..];
    }
    cells
}

/// One table cell: its spans and the image it carries.
fn parse_cell(cell: &str) -> PreviewCell {
    let tag = opening_tag(cell, "td").unwrap_or_default();
    let image = opening_tag(cell, "img").and_then(|image| {
        Some(PreviewImage {
            src: attribute(image, "src")?,
            width: attribute(image, "width")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            height: attribute(image, "height")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        })
    });
    PreviewCell {
        image,
        col_span: span(tag, "colspan"),
        row_span: span(tag, "rowspan"),
    }
}

/// The first `<name …>` tag's text.
fn opening_tag<'a>(html: &'a str, name: &str) -> Option<&'a str> {
    let start = html.find(&format!("<{name}"))?;
    let end = html[start..].find('>')? + start;
    Some(&html[start..=end])
}

/// One attribute's value out of a tag's text.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_owned())
}

/// The span an attribute carries (one when absent or unparsable).
fn span(tag: &str, name: &str) -> usize {
    attribute(tag, name)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1)
}

/// Places the exported table's cells on a grid, honoring `rowspan`
/// and `colspan`, and sizes the columns and rows from the images the
/// markup carries.
pub(crate) fn layout(rows: &[Vec<PreviewCell>]) -> PreviewLayout {
    // Occupancy: the grid slots a spanning cell already took.
    let mut occupied: Vec<Vec<bool>> = Vec::new();
    let mut placed: Vec<(usize, usize, usize)> = Vec::new();
    let mut columns = 0usize;
    for (row_index, row) in rows.iter().enumerate() {
        let mut col = 0usize;
        for (cell_index, cell) in row.iter().enumerate() {
            while occupied
                .get(row_index)
                .is_some_and(|slots| slots.get(col).copied().unwrap_or(false))
            {
                col += 1;
            }
            for spanned_row in row_index..row_index + cell.row_span {
                if occupied.len() <= spanned_row {
                    occupied.resize(spanned_row + 1, Vec::new());
                }
                let slots = &mut occupied[spanned_row];
                if slots.len() < col + cell.col_span {
                    slots.resize(col + cell.col_span, false);
                }
                for slot in slots.iter_mut().skip(col).take(cell.col_span) {
                    *slot = true;
                }
            }
            placed.push((row_index, col, cell_index));
            col += cell.col_span;
        }
        columns = columns.max(col);
    }

    let mut col_widths = vec![0.0f32; columns];
    let mut row_heights = vec![0.0f32; rows.len()];
    for (row_index, col, cell_index) in &placed {
        let cell = &rows[*row_index][*cell_index];
        let Some(image) = &cell.image else { continue };
        if cell.col_span == 1 && *col < col_widths.len() {
            col_widths[*col] = col_widths[*col].max(image.width as f32);
        }
        if cell.row_span == 1 && *row_index < row_heights.len() {
            row_heights[*row_index] = row_heights[*row_index].max(image.height as f32);
        }
    }
    // A spanning cell's image has to fit the columns and rows it
    // covers; the shortfall is shared out over them.
    for (row_index, col, cell_index) in &placed {
        let cell = &rows[*row_index][*cell_index];
        let Some(image) = &cell.image else { continue };
        if cell.col_span > 1 {
            let end = (*col + cell.col_span).min(col_widths.len());
            if end > *col {
                let spanned: f32 = col_widths[*col..end].iter().sum();
                let deficit = image.width as f32 - spanned;
                if deficit > 0.0 {
                    let share = deficit / (end - *col) as f32;
                    for width in col_widths.iter_mut().skip(*col).take(end - *col) {
                        *width += share;
                    }
                }
            }
        }
        if cell.row_span > 1 {
            let end = (*row_index + cell.row_span).min(row_heights.len());
            if end > *row_index {
                let spanned: f32 = row_heights[*row_index..end].iter().sum();
                let deficit = image.height as f32 - spanned;
                if deficit > 0.0 {
                    let share = deficit / (end - *row_index) as f32;
                    for height in row_heights
                        .iter_mut()
                        .skip(*row_index)
                        .take(end - *row_index)
                    {
                        *height += share;
                    }
                }
            }
        }
    }

    let mut col_x = vec![0.0f32; columns];
    let mut x = 0.0f32;
    for (index, width) in col_widths.iter().enumerate() {
        col_x[index] = x;
        x += width;
    }
    let mut row_y = vec![0.0f32; rows.len()];
    let mut y = 0.0f32;
    for (index, height) in row_heights.iter().enumerate() {
        row_y[index] = y;
        y += height;
    }
    let cells = placed
        .iter()
        .map(|(row_index, col, cell_index)| PlacedCell {
            x: col_x.get(*col).copied().unwrap_or(0.0),
            y: row_y.get(*row_index).copied().unwrap_or(0.0),
            image: rows[*row_index][*cell_index].image.clone(),
        })
        .collect();
    PreviewLayout {
        cells,
        width: x,
        height: y,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The markup the exporter writes for one slice: a table with the
    /// slice's image in a cell, plus the shim row.
    const ONE_SLICE: &str = concat!(
        "<table border=\"0\" cellpadding=\"0\" cellspacing=\"0\" width=\"80\">\n",
        "  <tr>\n",
        "   <td><img src=\"shim.gif\" width=\"80\" height=\"1\" alt=\"\"></td>\n",
        "   <td><img src=\"shim.gif\" width=\"1\" height=\"1\" alt=\"\"></td>\n  </tr>\n",
        "  <tr>\n",
        "   <td>\n",
        "    <img name=\"plate\" src=\"plate.png\" width=\"80\" height=\"40\" alt=\"\">\n",
        "   </td>\n",
        "   <td><img src=\"shim.gif\" width=\"1\" height=\"40\" alt=\"\"></td>\n",
        "  </tr>\n",
        "</table>\n",
    );

    /// The parser reads the exporter's table: two rows, the images
    /// with their sizes, and the spans the markup carries.
    #[test]
    fn the_parser_reads_the_exporters_table() {
        let rows = parse_table(ONE_SLICE);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].len(), 2);
        assert_eq!(rows[1].len(), 2);
        let image = rows[1][0]
            .image
            .clone()
            .expect("the slice cell carries an image");
        assert_eq!(image.src, "plate.png");
        assert_eq!(image.width, 80);
        assert_eq!(image.height, 40);
        assert_eq!(rows[1][0].col_span, 1);
        assert_eq!(rows[1][0].row_span, 1);
    }

    /// A spanning cell covers the columns and rows it names, and the
    /// next cell starts after it.
    #[test]
    fn a_spanning_cell_covers_its_columns_and_rows() {
        let rows = vec![vec![PreviewCell {
            image: Some(PreviewImage {
                src: "wide.png".to_owned(),
                width: 60,
                height: 20,
            }),
            col_span: 2,
            row_span: 2,
        }]];
        let layout = layout(&rows);
        assert_eq!(layout.cells.len(), 1);
        assert_eq!(layout.width, 60.0);
        assert_eq!(layout.height, 20.0);
    }

    /// The columns are as wide as the images they carry, and each
    /// cell sits at its column's origin.
    #[test]
    fn the_columns_are_sized_by_their_images() {
        let rows = vec![vec![
            PreviewCell {
                image: Some(PreviewImage {
                    src: "a.png".to_owned(),
                    width: 30,
                    height: 10,
                }),
                col_span: 1,
                row_span: 1,
            },
            PreviewCell {
                image: Some(PreviewImage {
                    src: "b.png".to_owned(),
                    width: 50,
                    height: 20,
                }),
                col_span: 1,
                row_span: 1,
            },
        ]];
        let layout = layout(&rows);
        assert_eq!(layout.width, 80.0);
        assert_eq!(layout.height, 20.0);
        assert_eq!(layout.cells[0].x, 0.0);
        assert_eq!(layout.cells[1].x, 30.0);
    }

    /// A shell over one document whose slice carries `name`.
    ///
    /// The slice's name reaches the exported markup, so the name is
    /// the marker a test reads the preview's cache back with.
    fn preview_shell(name: &str) -> Shell {
        let mut editor = crate::editor::Editor::untitled();
        editor
            .apply(
                "slice.create",
                serde_json::json!({
                    "shape": "rect",
                    "x": 0.0, "y": 0.0,
                    "width": 32.0, "height": 16.0,
                    "points": null, "page": 0,
                    "name": name, "link": null, "alt": null,
                }),
            )
            .expect("slice.create applies");
        super::super::tests::gui_shell(editor)
    }

    /// Runs the preview panel headlessly, as the dock draws it.
    fn draw_preview(shell: &mut Shell) -> egui::FullOutput {
        let context = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(800.0, 600.0),
            )),
            ..egui::RawInput::default()
        };
        let mut output = context.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                shell.html_preview_panel(ui);
            });
        });
        // The panel draws the exported page's images, so the frame
        // carries texture deltas. Nothing applies them here — the test
        // has no GPU — and egui refuses to drop an unapplied delta, so
        // the frame is cleared the same way the shell's own headless
        // draws clear it.
        output.textures_delta.clear();
        output
    }

    /// The markup the preview is currently showing.
    fn cached_html() -> String {
        STATE.with(|state| {
            state
                .borrow()
                .cache
                .as_ref()
                .map(|cache| cache.html.clone())
                .unwrap_or_default()
        })
    }

    /// The preview shows the document it is showing, not the one it
    /// showed before.
    ///
    /// Two freshly opened documents carry the same change stamp — an
    /// empty history, an empty journal, no undo — so the stamp alone
    /// cannot tell them apart. The preview's cache is keyed on the
    /// session's identity as well, or the second document's preview
    /// is the first document's page.
    #[test]
    fn the_preview_follows_the_document() {
        let mut first = preview_shell("alpha");
        draw_preview(&mut first);
        let first_html = cached_html();
        assert!(
            first_html.contains("alpha"),
            "the first document's preview carries its slice:\n{first_html}"
        );

        let mut second = preview_shell("beta");
        draw_preview(&mut second);
        let second_html = cached_html();
        assert!(
            second_html.contains("beta"),
            "the second document's preview carries its own slice:\n{second_html}"
        );
        assert!(
            !second_html.contains("alpha"),
            "the second document's preview is not the first document's:\n{second_html}"
        );
    }

    /// The preview draws the exported page.
    ///
    /// The preview is the only place a user sees the web export before
    /// publishing it, so its drawing path is covered: the panel runs
    /// headless against a document with a slice, and draws.
    #[test]
    fn the_preview_draws_the_exported_page() {
        let mut shell = preview_shell("plate");
        let output = draw_preview(&mut shell);
        assert!(
            !output.shapes.is_empty(),
            "the preview draws the exported page"
        );
    }

    /// The preview is the real export: the markup the panel shows is
    /// the markup the exporter writes, image for image.
    #[test]
    fn the_preview_runs_the_real_export_path() {
        let mut editor = crate::editor::Editor::untitled();
        editor.tool = crate::editor::Tool::Slice;
        editor.begin_drag(geom::Point::new(0.0, 0.0));
        editor.drag_to(geom::Point::new(32.0, 16.0));
        editor.end_drag().expect("slice.create applies");
        let options = webexport::WebExportOptions::default();
        let cache = build_cache(editor.document(), &options);
        assert!(cache.error.is_none(), "{:?}", cache.error);
        assert!(cache.html.contains("<table"), "the export writes a table");
        assert!(
            cache.files.iter().any(|file| file.name.ends_with(".png")),
            "the export writes the slice's image"
        );
        // The parsed layout places the exported images.
        assert!(!cache.layout.cells.is_empty());
        assert!(cache.layout.width > 0.0);
    }

    /// The preview's own widgets are asserted through egui's harness.
    ///
    /// The harness draws the panel into the accessibility tree a
    /// screen reader reads, so the panel's labels are queried as
    /// widgets rather than compared as pixels. The image-snapshot
    /// half of this coverage needs a renderer (egui_kittest's `wgpu`
    /// feature) and a verified Vulkan adapter, which the CI image
    /// does not carry yet — that half is the ledger's queued item 21.
    #[test]
    fn the_preview_draws_its_widgets_through_the_harness() {
        use egui_kittest::kittest::Queryable as _;

        let mut shell = preview_shell("plate");
        let mut harness = egui_kittest::Harness::new_ui(move |ui| {
            shell.html_preview_panel(ui);
        });
        harness.run();

        let source = harness.query_by_label_contains("HTML source");
        assert!(
            source.is_some(),
            "the preview offers the exported markup:\n{:#?}",
            harness.root()
        );
        let files = harness.query_by_label_contains("image file(s)");
        assert!(
            files.is_some(),
            "the preview reports the exported files:\n{:#?}",
            harness.root()
        );
    }

    /// The Polygon Hotspot tool commits through the append state
    /// machine (decision 0029's Design C): point 1 takes
    /// `hotspot.appendPoint`'s create branch, each later point
    /// appends, and the final anchor closes the outline — one undo
    /// step named for the verb. The anchors reach the exported image
    /// map, so the geometry the replay writes is the geometry a
    /// browser would hit.
    #[test]
    fn the_polygon_hotspot_tool_commits_through_append_point() {
        let mut editor = crate::editor::Editor::untitled();
        // A slice covering the canvas gives the hotspot a host piece
        // — the map hangs off the piece under the hotspot's center.
        editor.tool = crate::editor::Tool::Slice;
        editor.begin_drag(geom::Point::new(0.0, 0.0));
        editor.drag_to(geom::Point::new(320.0, 240.0));
        editor.end_drag().expect("slice.create applies");

        let selection_before = editor.document().object_selection.clone();
        editor.tool = crate::editor::Tool::PolygonHotspot;
        let plotted = [(10.0, 10.0), (60.0, 10.0), (60.0, 40.0)];
        for (x, y) in plotted {
            editor
                .add_draft_point(geom::Point::new(x, y))
                .expect("the point plots");
        }
        editor.commit_draft().expect("the append replay applies");

        let hotspots: Vec<_> = editor
            .document()
            .registry
            .iter()
            .filter_map(|(id, data)| match data {
                doc::ObjectData::Slice(slice) if slice.slice_type.is_none() => {
                    Some((id, slice.clone()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(hotspots.len(), 1, "one polygon hotspot was created");
        let hotspot = &hotspots[0].1;
        assert_eq!(hotspot.shape, 2, "the area is a polygon");
        let contour = hotspot.contours.first().expect("the outline");
        assert!(contour.closed, "the final anchor closes the outline");
        let anchors: Vec<(f64, f64)> = contour
            .points
            .iter()
            .map(|point| (point.anchor.x, point.anchor.y))
            .collect();
        assert_eq!(
            anchors,
            plotted.to_vec(),
            "every plotted anchor landed in order"
        );
        assert_eq!(
            editor.document().object_selection,
            selection_before,
            "the replay restores the selection it started with"
        );

        // The exported image map carries those anchors.
        let options = webexport::WebExportOptions::default();
        let cache = build_cache(editor.document(), &options);
        assert!(cache.error.is_none(), "{:?}", cache.error);
        assert!(cache.html.contains("<map "), "the export writes a map");
        assert!(
            cache
                .html
                .contains("shape=\"poly\" coords=\"10,10,60,10,60,40\""),
            "the map carries the replayed anchors"
        );

        // One undo step, named for the verb the tool replays.
        assert_eq!(
            editor.session.undo().expect("an undo step"),
            "hotspot.appendPoint",
            "the tool dispatched the append verb"
        );
        assert!(
            !editor
                .document()
                .registry
                .iter()
                .any(|(_, data)| matches!(data, doc::ObjectData::Slice(slice) if slice.slice_type.is_none())),
            "undo takes the replayed hotspot away"
        );
    }

    /// The Polygon Slice tool takes the same replay through
    /// `slice.appendPoint`, and the frame grows to carry every
    /// anchor.
    #[test]
    fn the_polygon_slice_tool_commits_through_append_point() {
        let mut editor = crate::editor::Editor::untitled();
        editor.tool = crate::editor::Tool::PolygonSlice;
        let plotted = [(20.0, 30.0), (80.0, 30.0), (80.0, 70.0)];
        for (x, y) in plotted {
            editor
                .add_draft_point(geom::Point::new(x, y))
                .expect("the point plots");
        }
        editor.commit_draft().expect("the append replay applies");

        let slices: Vec<_> = editor
            .document()
            .registry
            .iter()
            .filter_map(|(id, data)| match data {
                doc::ObjectData::Slice(slice) if slice.slice_type.is_some() => {
                    Some((id, slice.clone()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(slices.len(), 1, "one polygon slice was created");
        let slice = &slices[0].1;
        assert_eq!(slice.shape, 2, "the slice is a polygon");
        let contour = slice.contours.first().expect("the outline");
        assert!(contour.closed, "the final anchor closes the outline");
        let anchors: Vec<(f64, f64)> = contour
            .points
            .iter()
            .map(|point| (point.anchor.x, point.anchor.y))
            .collect();
        assert_eq!(
            anchors,
            plotted.to_vec(),
            "every plotted anchor landed in order"
        );
        assert_eq!(
            (
                slice.frame.x0,
                slice.frame.y0,
                slice.frame.x1,
                slice.frame.y1
            ),
            (20.0, 30.0, 80.0, 70.0),
            "the frame carries the anchors"
        );
        assert_eq!(
            editor.session.undo().expect("an undo step"),
            "slice.appendPoint",
            "the tool dispatched the append verb"
        );
        assert!(
            !editor
                .document()
                .registry
                .iter()
                .any(|(_, data)| matches!(data, doc::ObjectData::Slice(_))),
            "undo takes the replayed slice away"
        );
    }
}
