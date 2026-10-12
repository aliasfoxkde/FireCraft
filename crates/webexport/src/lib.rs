//! Web export workflows (M11): pure functions `(&Document, options) ->
//! {text, images}`.
//!
//! No engine mutation, no I/O side effects, deterministic bytes — the
//! shell, the CLI and scripting all call the same entry points, and
//! golden tests pin the output. The HTML structure follows the CS6
//! `Configuration/HTML Code/Dreamweaver/SLICES.XTT` template as a
//! behavioral spec (clean-room readable): the shim-table layout — a
//! row/column scan of slice bounds, `rowspan`/`colspan` merged cells,
//! the 1px shim edges — with `<img name="...">` cells, `usemap`
//! emission for hotspots and the `MM_swapImage` / `MM_nbGroup`
//! behavior hooks. The MM_* runtime blocks are clean-room functional
//! equivalents, not Adobe's text.
//!
//! Naming decisions (recorded for decision 0017's sibling note):
//! - A slice's per-slice `EXPv` override chooses the image codec;
//!   slices without one export PNG so the exported pixels are exactly
//!   the composer's (the pixel-identity floor holds by construction).
//! - Auto-naming (`naming`) applies the template's documented scheme
//!   at export time and is never stored on the slice.
//! - A hotspot anchors to the cell containing its center; its
//!   `<area>` coordinates are relative to that host image's origin.
//!   A hotspot spanning several regions therefore emits one area, not
//!   a per-region clip.

pub mod html;
pub mod images;
pub mod layout;
pub mod naming;

use doc::{Document, Layer, ObjectData, SliceObject};

/// Which export workflow to run. More arrive with M11 task 4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workflow {
    /// One image file per slice region, no HTML.
    ImagesOnly,
    /// The classic shim table plus the same image files.
    HtmlAndImages,
}

/// Options for one export run. File-system-free: names, not paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebExportOptions {
    /// Page to export.
    pub page: usize,
    /// State whose pixels feed the region rasters.
    pub state: usize,
    /// The export dialog's base name — the fallback stem for
    /// auto-named slices (`n{doc}_{row}_{col}`).
    pub doc_name: String,
    /// The exported HTML's `<title>` (and file stem for hosts that
    /// write one file).
    pub title: String,
    /// Directory prefix written into `src` attributes (`""` = the
    /// images sit beside the HTML).
    pub images_dir: String,
    /// Emit the 1px shim row/column (`doShimEdges`).
    pub shim: bool,
    /// Give cells no slice covers their own image (`doSkipUndefined`
    /// inverted — CS6's "Include areas without slices" checkbox).
    pub include_undefined: bool,
    /// `width="n%"` on the table instead of pixel width.
    pub table_pct_width: Option<u16>,
}

impl Default for WebExportOptions {
    fn default() -> Self {
        Self {
            page: 0,
            state: 0,
            doc_name: "doc".to_owned(),
            title: "doc".to_owned(),
            images_dir: String::new(),
            shim: true,
            include_undefined: true,
            table_pct_width: None,
        }
    }
}

/// One produced file: a name relative to the export directory and its
/// bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedFile {
    /// File name (e.g. `doc_r1_c1.png`, `shim.gif`).
    pub name: String,
    /// File content.
    pub bytes: Vec<u8>,
}

/// The result of one export run: image files plus, for HTML
/// workflows, the markup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebExport {
    /// Image (and shim) files, in emission order.
    pub files: Vec<ExportedFile>,
    /// The HTML document, when the workflow writes one.
    pub html: Option<String>,
}

/// Why an export cannot run.
#[derive(Clone, Debug, PartialEq)]
pub enum WebExportError {
    /// Page index outside `document.pages`.
    PageOutOfBounds {
        /// The offending index.
        page: usize,
    },
    /// State index outside the page's frame count.
    StateOutOfBounds {
        /// The offending index.
        state: usize,
        /// States the page carries.
        states: i64,
    },
    /// The page canvas rounds to fewer than one pixel.
    CanvasEmpty,
    /// The canvas exceeds the rasterizer's `u16` extent.
    CanvasTooLarge {
        /// Canvas width in document units.
        width: f64,
        /// Canvas height in document units.
        height: f64,
    },
    /// The composer refused the document (dangling ids, missing
    /// tiles).
    Compose(String),
    /// An image codec refused a region.
    Encode(String),
}

impl std::fmt::Display for WebExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebExportError::PageOutOfBounds { page } => {
                write!(f, "page {page} is outside the document")
            }
            WebExportError::StateOutOfBounds { state, states } => {
                write!(f, "state {state} is outside the page's {states} states")
            }
            WebExportError::CanvasEmpty => write!(f, "canvas rounds to fewer than one pixel"),
            WebExportError::CanvasTooLarge { width, height } => {
                write!(f, "canvas {width}x{height} exceeds the u16 extent")
            }
            WebExportError::Compose(reason) => write!(f, "compose failed: {reason}"),
            WebExportError::Encode(reason) => write!(f, "image encode failed: {reason}"),
        }
    }
}

impl std::error::Error for WebExportError {}

/// A web object resolved for export. Element order is paint order —
/// later elements sit on top.
#[derive(Clone, Debug)]
pub(crate) struct WebObject<'a> {
    pub slice: &'a SliceObject,
}

/// Collects the page's web objects in paint order: the Web Layer's
/// cells across all states, deduplicated with first-seen order
/// preserved (a slice's cell assignment does not change its z-order).
pub(crate) fn web_objects<'a>(
    document: &'a Document,
    options: &WebExportOptions,
) -> Vec<WebObject<'a>> {
    let Some(page) = document.pages.get(options.page) else {
        return Vec::new();
    };
    let Some(layer_index) = page
        .layers
        .layers
        .iter()
        .rposition(|layer: &Layer| layer.has_url && layer.is_special)
    else {
        return Vec::new();
    };
    let layer = &page.layers.layers[layer_index];
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for cell in &layer.cells {
        for id in &cell.elements {
            if seen.insert(*id) {
                if let Some(ObjectData::Slice(slice)) = document.registry.get(*id) {
                    out.push(WebObject { slice });
                }
            }
        }
    }
    out
}

/// Runs one export workflow: render, lay out, rasterize, (write HTML).
///
/// # Errors
/// [`WebExportError`] for out-of-range page/state, canvases outside
/// the rasterizer's extent, and compose/codec failures.
pub fn export(
    document: &Document,
    options: &WebExportOptions,
    workflow: Workflow,
) -> Result<WebExport, WebExportError> {
    let Some(page) = document.pages.get(options.page) else {
        return Err(WebExportError::PageOutOfBounds { page: options.page });
    };
    let states = page.info.frame_count.max(1);
    if options.state >= usize::try_from(states).unwrap_or(usize::MAX) {
        return Err(WebExportError::StateOutOfBounds {
            state: options.state,
            states,
        });
    }

    let objects = web_objects(document, options);
    let rendered = compose::render(document, options.page, options.state)
        .map_err(|error| WebExportError::Compose(error.to_string()))?;
    if rendered.width == 0 || rendered.height == 0 {
        return Err(WebExportError::CanvasEmpty);
    }
    let canvas = images::straight_rgba(&rendered);

    let grid = layout::scan(&objects, rendered.width, rendered.height);
    let (mut files, pieces) = images::rasterize(options, &objects, &grid, &canvas, rendered.width)?;

    let html = match workflow {
        Workflow::ImagesOnly => None,
        Workflow::HtmlAndImages => {
            // The shim rides last so image order is stable across
            // workflows.
            if options.shim {
                files.push(ExportedFile {
                    name: "shim.gif".to_owned(),
                    bytes: images::shim()?,
                });
            }
            Some(html::write_html(
                document, options, &objects, &grid, &pieces,
            ))
        }
    };
    Ok(WebExport { files, html })
}
