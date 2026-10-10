//! Inverse deltas: the undo currency of the engine.
//!
//! A [`Delta`] is captured *before* each mutation and holds the prior
//! state of exactly what the command touched — never a reverse
//! command. Undo replays the delta; redo re-applies the original
//! payload, so a delta only ever has to answer "what was here before?".

use doc::{Cell, Document, DocumentInfo, Layer, LayerList, ObjectData, ObjectId, Page, PageInfo};

use crate::error::EngineError;

/// What happened to one object during a command, and therefore what
/// undo must do to it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "how", rename_all = "snake_case")]
pub enum ObjectInverse {
    /// The object existed and was edited in place: undo writes this
    /// pre-command data back over the live slot.
    Mutated {
        /// The object as it was before the command.
        object: Box<ObjectData>,
    },
    /// The command removed the object: undo re-creates it at its exact
    /// slot and generation, reviving every id issued before the
    /// removal.
    Removed {
        /// The object as it was before the command.
        object: Box<ObjectData>,
    },
    /// The command inserted the object: undo removes it again.
    Created,
}

/// The inverse of one applied command.
///
/// Serialize shapes are stable: the journal (and anything that replays
/// one) may persist deltas.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Delta {
    /// The command changed nothing (a validated no-op).
    Noop,
    /// Restore one object.
    Object {
        /// The object's registry id.
        id: ObjectId,
        /// What the command did to it.
        inverse: ObjectInverse,
    },
    /// Restore one page slot: `Some` puts the previous page back at
    /// `index`, `None` removes the page the command inserted there.
    Page {
        /// Page index in document order.
        index: usize,
        /// The page as it was before the command, or `None` when the
        /// command inserted it.
        before: Option<Box<Page>>,
    },
    /// Restore a page's whole layer list.
    Layers {
        /// Page index.
        page: usize,
        /// The list as it was before the command.
        layers: LayerList,
    },
    /// Restore one layer.
    Layer {
        /// Page index.
        page: usize,
        /// Layer index, bottom-first.
        layer: usize,
        /// The layer as it was before the command.
        before: Box<Layer>,
    },
    /// Restore one cell (one state's element list of one layer).
    Cell {
        /// Page index.
        page: usize,
        /// Layer index.
        layer: usize,
        /// Cell index (state number).
        cell: usize,
        /// The cell as it was before the command.
        before: Box<Cell>,
    },
    /// Restore the document-level scalars and canvas.
    Info {
        /// The info as it was before the command.
        before: Box<DocumentInfo>,
    },
    /// Restore one page's scalars and canvas.
    PageInfo {
        /// Page index.
        page: usize,
        /// The info as it was before the command.
        before: Box<PageInfo>,
    },
    /// Several inverses, applied last-captured-first on undo. Commands
    /// push captures in the order they mutate, so a plain reverse walk
    /// un-mutates in exact reverse order.
    Group(Vec<Delta>),
}

impl Delta {
    /// Applies the inverse to `document`, restoring what the captured
    /// command changed. Fails with [`EngineError::DeltaMismatch`] when
    /// the delta no longer lines up with the document — that is a
    /// lineage bug (undo across a diverged base), never a user error,
    /// and it leaves the document untouched up to that point.
    pub fn apply_inverse(&self, document: &mut Document) -> Result<(), EngineError> {
        match self {
            Delta::Noop => {}
            Delta::Object { id, inverse } => match inverse {
                ObjectInverse::Mutated { object } => {
                    let slot = document.registry.get_mut(*id).ok_or_else(|| {
                        EngineError::DeltaMismatch {
                            detail: format!(
                                "object {} (generation {}) is not live to mutate back",
                                id.index(),
                                id.generation()
                            ),
                        }
                    })?;
                    *slot = (**object).clone();
                }
                ObjectInverse::Removed { object } => document
                    .registry
                    .restore(*id, (**object).clone())
                    .ok_or_else(|| EngineError::DeltaMismatch {
                        detail: format!(
                            "slot {} (generation {}) is not restorable",
                            id.index(),
                            id.generation()
                        ),
                    })?,
                ObjectInverse::Created => {
                    document
                        .registry
                        .remove(*id)
                        .ok_or_else(|| EngineError::DeltaMismatch {
                            detail: format!(
                                "cannot remove object {} (generation {}): not live",
                                id.index(),
                                id.generation()
                            ),
                        })?;
                }
            },
            Delta::Page { index, before } => match before {
                Some(page) => {
                    if *index > document.pages.len() {
                        return Err(EngineError::DeltaMismatch {
                            detail: format!("page slot {index} out of range"),
                        });
                    }
                    document.pages.insert(*index, (**page).clone());
                }
                None => {
                    if *index >= document.pages.len() {
                        return Err(EngineError::DeltaMismatch {
                            detail: format!("page {index} out of range"),
                        });
                    }
                    document.pages.remove(*index);
                }
            },
            Delta::Layers { page, layers } => {
                page_mut(document, *page)?.layers = layers.clone();
            }
            Delta::Layer {
                page,
                layer,
                before,
            } => {
                let (page_index, layer_index) = (*page, *layer);
                let target = page_mut(document, page_index)?;
                let slot = target.layers.layers.get_mut(layer_index).ok_or_else(|| {
                    EngineError::DeltaMismatch {
                        detail: format!("layer {layer_index} of page {page_index} out of range"),
                    }
                })?;
                *slot = (**before).clone();
            }
            Delta::Cell {
                page,
                layer,
                cell,
                before,
            } => {
                let (page_index, layer_index, cell_index) = (*page, *layer, *cell);
                let layer = page_mut(document, page_index)?;
                let target = layer.layers.layers.get_mut(layer_index).ok_or_else(|| {
                    EngineError::DeltaMismatch {
                        detail: format!("layer {layer_index} of page {page_index} out of range"),
                    }
                })?;
                let slot =
                    target
                        .cells
                        .get_mut(cell_index)
                        .ok_or_else(|| EngineError::DeltaMismatch {
                            detail: format!(
                                "cell {cell_index} of layer {layer_index} out of range"
                            ),
                        })?;
                *slot = (**before).clone();
            }
            Delta::Info { before } => document.info = (**before).clone(),
            Delta::PageInfo { page, before } => {
                page_mut(document, *page)?.info = (**before).clone();
            }
            Delta::Group(deltas) => {
                for delta in deltas.iter().rev() {
                    delta.apply_inverse(document)?;
                }
            }
        }
        Ok(())
    }
}

fn page_mut(document: &mut Document, page: usize) -> Result<&mut Page, EngineError> {
    document
        .pages
        .get_mut(page)
        .ok_or_else(|| EngineError::DeltaMismatch {
            detail: format!("page {page} out of range"),
        })
}
