//! Library extract/import support (`doc`, L1): moving symbol
//! definitions between documents.
//!
//! A library file is an ordinary `.fw.png` document whose `MSLv`
//! (`document.symbols`) carries the definitions and whose registry
//! holds their member closure — the members need not sit on any page.
//! [`extract`] builds such a document from named definitions of a
//! source document (ids remapped into the fresh registry); the
//! importing direction is the engine's `library.importSymbol`
//! command, which journals the same transform as ordinary mutations.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Document, SymbolDefinition};
use crate::registry::{ObjectData, ObjectId};

/// Map key for id tables — `ObjectId` is `Hash` but not `Ord`, and the
/// `"index:generation"` string form would be gratuitous allocation.
type IdKey = (usize, u32);

/// The map key of an id.
fn key(id: ObjectId) -> IdKey {
    (id.index(), id.generation())
}

/// Why an extract refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtractError {
    /// No definition carries the name.
    UnknownSymbol(String),
    /// A member id referenced by a definition is not in the registry.
    DanglingMember { symbol: String, id: ObjectId },
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractError::UnknownSymbol(name) => {
                write!(f, "no symbol named {name:?} in this document's library")
            }
            ExtractError::DanglingMember { symbol, id } => {
                write!(
                    f,
                    "symbol {symbol:?} references missing member {}:{}",
                    id.index(),
                    id.generation()
                )
            }
        }
    }
}

impl std::error::Error for ExtractError {}

/// Extracts the named definitions (first match wins, in the order
/// given) and their member closure into a fresh document: the library
/// file shape. Ids are remapped into the new registry; definitions
/// are renumbered `0..n` in output order. Nothing is placed on a page.
///
/// # Errors
/// [`ExtractError`] — an unknown name, or a definition whose member
/// ids are not live in the source registry (corrupt library).
pub fn extract(document: &Document, names: &[&str]) -> Result<Document, ExtractError> {
    // The member closure, source ids in deterministic first-touch
    // order (definition layers bottom-up, groups depth-first).
    let mut order: Vec<ObjectId> = Vec::new();
    let mut seen: BTreeSet<IdKey> = BTreeSet::new();
    let mut definitions: Vec<SymbolDefinition> = Vec::new();
    for name in names {
        let Some(definition) = document
            .symbols
            .iter()
            .find(|definition| &definition.name == name)
        else {
            return Err(ExtractError::UnknownSymbol((*name).to_owned()));
        };
        let roots: Vec<ObjectId> = definition
            .layers
            .layers
            .iter()
            .flat_map(|layer| layer.cells.iter())
            .flat_map(|cell| cell.elements.iter())
            .copied()
            .collect();
        collect_members(document, &roots, &mut order, &mut seen).map_err(|id| {
            ExtractError::DanglingMember {
                symbol: definition.name.clone(),
                id,
            }
        })?;
        let mut copy = definition.clone();
        copy.symbol_number = definitions.len() as i64;
        definitions.push(copy);
    }

    // Re-slot the members, then rewrite every id reference through the
    // map — group children inside the member data, cell element lists
    // inside the definitions.
    let mut target = Document::default();
    let mut map: BTreeMap<IdKey, ObjectId> = BTreeMap::new();
    for id in &order {
        let data = document
            .registry
            .get(*id)
            .ok_or_else(|| ExtractError::DanglingMember {
                symbol: "<closure>".to_owned(),
                id: *id,
            })?;
        let fresh = target.registry.insert(data.clone());
        map.insert(key(*id), fresh);
    }
    for id in &order {
        // The slot was inserted one loop above; the `if let` only
        // keeps the registry's fallible lookup off the panic path.
        if let Some(slot) = target.registry.get_mut(map[&key(*id)]) {
            remap_object(slot, &map);
        }
    }
    for definition in &mut definitions {
        for layer in &mut definition.layers.layers {
            for cell in &mut layer.cells {
                cell.elements = cell
                    .elements
                    .iter()
                    .map(|id| map[&key(*id)])
                    .collect::<Vec<_>>();
            }
        }
    }
    target.symbols = definitions;
    Ok(target)
}

/// Depth-first member collection over a definition's element lists.
/// Group children recurse; every referenced id must be live.
fn collect_members(
    document: &Document,
    ids: &[ObjectId],
    order: &mut Vec<ObjectId>,
    seen: &mut BTreeSet<IdKey>,
) -> Result<(), ObjectId> {
    for id in ids {
        if !seen.insert(key(*id)) {
            continue;
        }
        let data = document.registry.get(*id).ok_or(*id)?;
        order.push(*id);
        if let ObjectData::Group(group) = data {
            collect_members(document, &group.children, order, seen)?;
        }
    }
    Ok(())
}

/// Rewrites the id references inside one member (group children).
fn remap_object(data: &mut ObjectData, map: &BTreeMap<IdKey, ObjectId>) {
    if let ObjectData::Group(group) = data {
        for child in &mut group.children {
            if let Some(fresh) = map.get(&key(*child)) {
                *child = *fresh;
            }
        }
    }
}
