//! Target lookups shared by catalog commands: resolve payload indices
//! and ids to model slots, or name what was missing.

use doc::{Cell, Document, Layer, ObjectData, ObjectId, Page};

use crate::error::EngineError;

pub(crate) fn page<'a>(
    document: &'a Document,
    command: &'static str,
    index: usize,
) -> Result<&'a Page, EngineError> {
    let len = document.pages.len();
    document
        .pages
        .get(index)
        .ok_or_else(|| EngineError::MissingTarget {
            name: command.to_owned(),
            detail: format!("page {index} out of range ({len} pages)"),
        })
}

pub(crate) fn page_mut<'a>(
    document: &'a mut Document,
    command: &'static str,
    index: usize,
) -> Result<&'a mut Page, EngineError> {
    let len = document.pages.len();
    document
        .pages
        .get_mut(index)
        .ok_or_else(|| EngineError::MissingTarget {
            name: command.to_owned(),
            detail: format!("page {index} out of range ({len} pages)"),
        })
}

pub(crate) fn layer_mut<'a>(
    document: &'a mut Document,
    command: &'static str,
    page: usize,
    layer: usize,
) -> Result<&'a mut Layer, EngineError> {
    let target = page_mut(document, command, page)?;
    let count = target.layers.layers.len();
    target
        .layers
        .layers
        .get_mut(layer)
        .ok_or_else(|| EngineError::MissingTarget {
            name: command.to_owned(),
            detail: format!("layer {layer} out of range ({count} layers on page {page})"),
        })
}

pub(crate) fn cell_mut<'a>(
    document: &'a mut Document,
    command: &'static str,
    page: usize,
    layer: usize,
    cell: usize,
) -> Result<&'a mut Cell, EngineError> {
    let target = layer_mut(document, command, page, layer)?;
    let count = target.cells.len();
    target
        .cells
        .get_mut(cell)
        .ok_or_else(|| EngineError::MissingTarget {
            name: command.to_owned(),
            detail: format!("cell {cell} out of range ({count} cells on layer {layer})"),
        })
}

pub(crate) fn object<'a>(
    document: &'a Document,
    command: &'static str,
    id: ObjectId,
) -> Result<&'a ObjectData, EngineError> {
    document
        .registry
        .get(id)
        .ok_or_else(|| EngineError::MissingTarget {
            name: command.to_owned(),
            detail: format!(
                "object {} (generation {}) is not live",
                id.index(),
                id.generation()
            ),
        })
}
