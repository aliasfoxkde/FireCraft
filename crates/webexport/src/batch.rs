//! Pages/States/Layers To Files: one flattened PNG per page, per
//! state, or per visible layer — the CS6 batch exports. Naming is
//! `{doc}_p{n}`, `{doc}_state{n}`, and the layer's own name
//! respectively, deduplicated with the same `_x` suffix rule as the
//! slice rasterizer.

use crate::naming;
use crate::{ExportedFile, WebExportError, WebExportOptions};

/// Renders one page/state to a straight-alpha PNG under `name`.
fn png_of(
    document: &doc::Document,
    page: usize,
    state: usize,
    name: &str,
) -> Result<ExportedFile, WebExportError> {
    let rendered = compose::render(document, page, state)
        .map_err(|error| WebExportError::Compose(error.to_string()))?;
    if rendered.width == 0 || rendered.height == 0 {
        return Err(WebExportError::CanvasEmpty);
    }
    let rgba = compose::straight_alpha(&rendered.pixels);
    let plane = io::Plane::new(u32::from(rendered.width), u32::from(rendered.height), &rgba)
        .map_err(|error| WebExportError::Encode(error.to_string()))?;
    let bytes =
        io::png::encode(&plane).map_err(|error| WebExportError::Encode(error.to_string()))?;
    Ok(ExportedFile {
        name: name.to_owned(),
        bytes,
    })
}

/// Reserves `stem.extension`; `_x` suffix until free (same rule as
/// the rasterizer's dedup).
fn reserve(used: &mut std::collections::BTreeSet<String>, stem: String, extension: &str) -> String {
    let mut candidate = stem;
    loop {
        let name = naming::with_extension(&candidate, extension);
        if used.insert(name.clone()) {
            return name;
        }
        candidate = format!("{candidate}_x");
    }
}

/// One PNG per page, in page order.
///
/// # Errors
/// [`WebExportError`] from any page's render or encode.
pub(crate) fn pages_to_files(
    document: &doc::Document,
    options: &WebExportOptions,
) -> Result<Vec<ExportedFile>, WebExportError> {
    let base = naming::clean(&options.doc_name);
    let mut used = std::collections::BTreeSet::new();
    let mut files = Vec::new();
    for index in 0..document.pages.len() {
        let name = reserve(&mut used, format!("{base}_p{}", index + 1), "png");
        files.push(png_of(document, index, options.state, &name)?);
    }
    Ok(files)
}

/// One PNG per state of the exported page.
///
/// # Errors
/// [`WebExportError::PageOutOfBounds`] and render/encode failures.
pub(crate) fn states_to_files(
    document: &doc::Document,
    options: &WebExportOptions,
) -> Result<Vec<ExportedFile>, WebExportError> {
    let page = document
        .pages
        .get(options.page)
        .ok_or(WebExportError::PageOutOfBounds { page: options.page })?;
    let states = usize::try_from(page.info.frame_count.max(1)).unwrap_or(usize::MAX);
    let base = naming::clean(&options.doc_name);
    let mut used = std::collections::BTreeSet::new();
    let mut files = Vec::new();
    for state in 0..states {
        let name = reserve(&mut used, format!("{base}_state{}", state + 1), "png");
        files.push(png_of(document, options.page, state, &name)?);
    }
    Ok(files)
}

/// One PNG per visible layer of the exported page, each rendered
/// alone (every other layer hidden; the canvas background stays).
///
/// # Errors
/// [`WebExportError::PageOutOfBounds`] and render/encode failures.
pub(crate) fn layers_to_files(
    document: &doc::Document,
    options: &WebExportOptions,
) -> Result<Vec<ExportedFile>, WebExportError> {
    let page = document
        .pages
        .get(options.page)
        .ok_or(WebExportError::PageOutOfBounds { page: options.page })?;
    let mut used = std::collections::BTreeSet::new();
    let mut files = Vec::new();
    for (index, layer) in page.layers.layers.iter().enumerate() {
        if !layer.display {
            continue;
        }
        let mut alone = document.clone();
        let solo = &mut alone.pages[options.page].layers.layers;
        for other in solo.iter_mut() {
            other.display = false;
        }
        solo[index].display = true;
        let stem = match naming::clean(&layer.name).is_empty() {
            true => format!("layer{}", index + 1),
            false => naming::clean(&layer.name),
        };
        let name = reserve(&mut used, stem, "png");
        files.push(png_of(&alone, options.page, options.state, &name)?);
    }
    Ok(files)
}
