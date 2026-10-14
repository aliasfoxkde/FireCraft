//! Inverse deltas: the undo currency of the engine.
//!
//! A [`Delta`] is captured *before* each mutation and holds the prior
//! state of exactly what the command touched — never a reverse
//! command. Undo replays the delta; redo re-applies the original
//! payload, so a delta only ever has to answer "what was here before?".

use doc::{
    Cell, Document, DocumentInfo, Layer, LayerList, ObjectData, ObjectId, Page, PageInfo,
    StyleEntry, SymbolDefinition,
};

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
    /// Restore bitmap tiles of one image (M7 pixel edits): every
    /// touched `TIDi` with its pre-command copy-on-write tile. Commands
    /// only touch tiles already present in the bank (absent payload is
    /// a refusal), so undo is always a re-insert.
    Tiles {
        /// The image object the tiles belong to (lineage bookkeeping).
        id: ObjectId,
        /// `(tile id, pre-command tile)` for every tile the command
        /// touched.
        before: Vec<(u32, raster::Tile)>,
    },
    /// Restore the active selection (or its absence).
    Selection {
        /// The mask as it was before the command, `None` when nothing
        /// was selected.
        before: Option<Box<doc::SelectionMask>>,
    },
    /// Restore one page's scalars and canvas.
    PageInfo {
        /// Page index.
        page: usize,
        /// The info as it was before the command.
        before: Box<PageInfo>,
    },
    /// Restore one page's guides (`GDSv`).
    Guides {
        /// Page index.
        page: usize,
        /// The guides as they were before the command.
        before: doc::Guides,
    },
    /// Restore one page's grid override (`GRDv`; `None` means the page
    /// inherits the document grid).
    PageGrid {
        /// Page index.
        page: usize,
        /// The override as it was before the command.
        before: Option<Box<doc::Grid>>,
    },
    /// Restore the whole document symbol library. Library mutations are
    /// list-capturing: a definition carries its layer lists and the
    /// cell references inside them, so the pre-command `Vec` is the
    /// exact inverse of any add / remove / edit the command performed.
    Library {
        /// The library as it was before the command.
        before: Vec<SymbolDefinition>,
    },
    /// Restore the document-level style library (`MKBv`'s `ZSLv`).
    /// List-capturing like [`Delta::Library`]: a style entry is a
    /// self-contained property bundle, so the pre-command `Vec` is the
    /// exact inverse of any create / replace / scale the command
    /// performed.
    Styles {
        /// The style list as it was before the command.
        before: Vec<StyleEntry>,
    },
    /// Remove the bank slots the command created (the style-library
    /// tile join): only fresh ids are inverse-expressible, and the
    /// command only ever inserts into absent slots (a colliding id is
    /// a refusal or a content-equal replay no-op), so undo is exactly
    /// "drop these slots". Last-captured-first undo keeps this exact
    /// even when a later command re-touched the same ids — that
    /// command's own capture restores them first.
    Bank {
        /// The ids the command inserted into the (previously absent)
        /// bank.
        added: Vec<u32>,
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
                    document.registry.retract_insert(*id).ok_or_else(|| {
                        EngineError::DeltaMismatch {
                            detail: format!(
                                "cannot retract object {} (generation {}): not live",
                                id.index(),
                                id.generation()
                            ),
                        }
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
            Delta::Tiles { before, .. } => {
                for (tile_id, tile) in before {
                    document.tiles.insert(*tile_id, tile.clone());
                }
            }
            Delta::Selection { before } => {
                document.selection = before.as_deref().cloned();
            }
            Delta::PageInfo { page, before } => {
                page_mut(document, *page)?.info = (**before).clone();
            }
            Delta::Guides { page, before } => {
                page_mut(document, *page)?.guides = before.clone();
            }
            Delta::PageGrid { page, before } => {
                page_mut(document, *page)?.grid = before.as_deref().cloned();
            }
            Delta::Library { before } => document.symbols = before.clone(),
            Delta::Styles { before } => document.styles = before.clone(),
            Delta::Bank { added } => {
                for tile_id in added {
                    document.tiles.remove(*tile_id);
                }
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
