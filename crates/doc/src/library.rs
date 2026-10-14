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

use crate::model::{Document, PaintStyle, SymbolDefinition, TileRef};
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
    /// A member references a tile id the source bank does not carry
    /// (a corrupt source document).
    MissingTile { tile: u32 },
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
            ExtractError::MissingTile { tile } => {
                write!(f, "a member references missing tile {tile}")
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
    // The referenced tiles join the library's bank under their source
    // ids — the file's `TIDi` values match the source bank, so re-import
    // finds its rasters. A referenced id with no bank entry is a corrupt
    // source; refuse whole rather than land a dangling reference.
    for tile in referenced_tiles(document, &order) {
        let Some(bytes) = document.tiles.get(tile) else {
            return Err(ExtractError::MissingTile { tile });
        };
        target.tiles.insert(tile, bytes.clone());
    }
    Ok(target)
}

/// The tile ids the member closure references: image grids plus the
/// pattern fills of path paints. Group children are already members —
/// `order` is the depth-first closure — so the walk stays flat.
fn referenced_tiles(document: &Document, order: &[ObjectId]) -> BTreeSet<u32> {
    fn paint_tiles(paint: Option<&PaintStyle>, out: &mut BTreeSet<u32>) {
        let Some(pattern) = paint
            .and_then(|paint| paint.fill.as_ref())
            .and_then(|fill| fill.pattern.as_ref())
        else {
            return;
        };
        for tile in &pattern.tiles {
            if let TileRef::Tile(id) = tile {
                out.insert(*id);
            }
        }
    }
    let mut out = BTreeSet::new();
    for id in order {
        match document.registry.get(*id) {
            Some(ObjectData::Image(image)) => {
                for tile in &image.tiles {
                    if let TileRef::Tile(id) = tile {
                        out.insert(*id);
                    }
                }
            }
            Some(ObjectData::Path(path)) => paint_tiles(path.paint.as_ref(), &mut out),
            _ => {}
        }
    }
    out
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::model::{
        Cell, Fill, FillShape, ImageObject, Layer, LayerList, PathObject, SymbolDefinition,
        SymbolType,
    };
    use geom::Point;

    /// A bank tile whose first pixel carries `marker`.
    fn marker_tile(marker: u8) -> raster::Tile {
        let mut plane = [0u8; raster::TILE_BYTES];
        plane[..4].copy_from_slice(&[marker, marker, marker, 0xFF]);
        raster::Tile::from_rgba8(plane)
    }

    /// An image member referencing tile 7.
    fn image_member() -> ObjectData {
        ObjectData::Image(ImageObject {
            location: Point::ZERO,
            offset: Point::ZERO,
            pixels: (128, 128),
            tile_size: 128,
            tiles: vec![TileRef::Tile(7)],
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        })
    }

    /// A path member whose pattern fill references tile 3 plus a
    /// solid-matte reference (which must not pull a bank entry).
    fn path_member() -> ObjectData {
        ObjectData::Path(PathObject {
            contours: Vec::new(),
            primitive: None,
            even_odd: false,
            paint: Some(PaintStyle {
                stroke_raw: 0,
                fill_raw: 0,
                fill: Some(Fill {
                    shape: FillShape::Pattern,
                    shape_extended: true,
                    texture_blend: 0,
                    feather: 0,
                    stamping: None,
                    hard_edge: false,
                    gradient: None,
                    dither: None,
                    pattern: Some(crate::model::PatternTile {
                        name: Some("p".to_owned()),
                        pixels: (128, 128),
                        tile_size: 128,
                        tiles: vec![TileRef::Tile(3), TileRef::Matte(0xFF00_00FF)],
                    }),
                }),
            }),
            fill_start: None,
            fill_end: None,
            fill_third: None,
            brush_placement: None,
            fill_on_top: None,
            locked: false,
            visible: true,
            opacity: 1000,
            blend: 0,
            name: None,
            effects: Vec::new(),
        })
    }

    /// One definition named "sym" carrying `elements`.
    fn definition(elements: Vec<ObjectId>) -> SymbolDefinition {
        SymbolDefinition {
            name: "sym".to_owned(),
            symbol_number: 0,
            origin: Point::ZERO,
            frame_count: 1,
            layers: LayerList {
                layers: vec![Layer {
                    name: String::new(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements,
                    }],
                    ..Layer::default()
                }],
            },
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
            auto_slice: None,
            include_down_state: None,
            show_down_on_load: None,
            include_over_while_down: None,
            op_code: None,
        }
    }

    #[test]
    fn extract_carries_referenced_tiles() {
        let mut document = Document::default();
        let image_id = document.registry.insert(image_member());
        let path_id = document.registry.insert(path_member());
        document.tiles.insert(3, marker_tile(3));
        document.tiles.insert(7, marker_tile(7));
        document.symbols.push(definition(vec![image_id, path_id]));

        let library = extract(&document, &["sym"]).expect("extracts");
        let ids: Vec<u32> = library.tiles.ids().collect();
        assert_eq!(ids, vec![3, 7], "referenced tiles only, bank order");
        assert_eq!(library.tiles.get(3).expect("tile 3").pixels()[0], 3);
        assert_eq!(library.tiles.get(7).expect("tile 7").pixels()[0], 7);
    }

    #[test]
    fn extract_refuses_a_dangling_tile_reference() {
        let mut document = Document::default();
        let image_id = document.registry.insert(image_member());
        document.symbols.push(definition(vec![image_id]));

        let error = extract(&document, &["sym"]).expect_err("refuses");
        assert_eq!(error, ExtractError::MissingTile { tile: 7 });
    }
}
