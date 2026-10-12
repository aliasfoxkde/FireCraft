//! Symbol + library commands: the document library (`document.symbols`)
//! and the instance lifecycle around it.
//!
//! Library mutations are list-capturing — a [`Delta::Library`] holds the
//! whole pre-command `Vec<SymbolDefinition>`, because a definition
//! carries its own layer lists and cell references, and every edit a
//! command performs inside one (attach / detach / reorder) is exactly
//! inverted by restoring that list. Page-side mutations (moving a
//! selected object into a definition, placing or breaking an instance)
//! capture their containers like every other structural command.
//!
//! Definitions are addressed by `symbol_number` (`MSNi`) — the same key
//! instances reference — never by library position, so payloads stay
//! stable across library edits.

use doc::{
    Cell, Document, Layer, LayerList, NineSliceGuides, ObjectData, ObjectId, SymbolDefinition,
    SymbolInstanceObject, SymbolType,
};

use crate::access;
use crate::catalog::object::{Owner, attach_cell, detach, ensure_detachable, find_owner};
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Group-nesting ceiling for the break-apart duplicate walk, matching
/// compose's symbol depth ceiling in spirit: a deeper chain is a
/// corrupted document, refused rather than walked.
const MAX_NESTING: usize = 32;

/// Registers the symbol + library commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(CreateSymbol)?;
    registry.register(CreateInstance)?;
    registry.register(BreakApart)?;
    registry.register(SetSymbolType)?;
    registry.register(SetSymbol9Slice)?;
    registry.register(EditAttach)?;
    registry.register(EditDetach)?;
    registry.register(EditReorder)?;
    registry.register(LibraryAdd)?;
    registry.register(LibraryRemove)?;
    registry.register(LibraryRename)
}

/// The settable symbol types. `Other(raw)` definitions stay readable
/// but are not a creation choice — nothing observed justifies one.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum SymbolTypeChoice {
    /// Plain reusable artwork (`MSYi` omitted on the wire).
    Graphic,
    /// Multi-state button (`MSYi` 1).
    Button,
    /// Animation source (`MSYi` 2).
    Animation,
}

impl From<SymbolTypeChoice> for SymbolType {
    fn from(choice: SymbolTypeChoice) -> Self {
        match choice {
            SymbolTypeChoice::Graphic => SymbolType::Graphic,
            SymbolTypeChoice::Button => SymbolType::Button,
            SymbolTypeChoice::Animation => SymbolType::Animation,
        }
    }
}

/// The next free `MSNi`: one above the highest in the library (0 for an
/// empty one). Re-executing a replayed command over the same restored
/// state therefore allocates the same number.
fn next_symbol_number(document: &Document) -> i64 {
    document
        .symbols
        .iter()
        .map(|definition| definition.symbol_number)
        .max()
        .map_or(0, |highest| highest + 1)
}

/// Resolves `symbol` (`MSNi`) to its library position, or names the
/// dangling reference.
fn definition_position(
    document: &Document,
    command: &'static str,
    symbol: i64,
) -> Result<usize, EngineError> {
    document
        .symbols
        .iter()
        .position(|definition| definition.symbol_number == symbol)
        .ok_or_else(|| EngineError::MissingTarget {
            name: command.to_owned(),
            detail: format!(
                "no symbol numbered {symbol} in the library ({} definitions)",
                document.symbols.len()
            ),
        })
}

/// How many live instances reference `symbol`.
fn instance_count(document: &Document, symbol: i64) -> usize {
    document
        .registry
        .iter()
        .filter(|(_, object)| {
            matches!(
                object,
                ObjectData::SymbolInstance(instance) if instance.symbol_number == symbol
            )
        })
        .count()
}

/// Mutable access to one cell of one definition layer, bounds-checked.
fn definition_cell_mut<'a>(
    document: &'a mut Document,
    command: &'static str,
    symbol: i64,
    layer: usize,
    cell: usize,
) -> Result<&'a mut Cell, EngineError> {
    let position = definition_position(document, command, symbol)?;
    let definition = &mut document.symbols[position];
    let layer_count = definition.layers.layers.len();
    let layer_data =
        definition
            .layers
            .layers
            .get_mut(layer)
            .ok_or_else(|| EngineError::MissingTarget {
                name: command.to_owned(),
                detail: format!("layer {layer} out of range ({layer_count} symbol layers)"),
            })?;
    let cell_count = layer_data.cells.len();
    layer_data
        .cells
        .get_mut(cell)
        .ok_or_else(|| EngineError::MissingTarget {
            name: command.to_owned(),
            detail: format!("cell {cell} out of range ({cell_count} symbol cells)"),
        })
}

/// Locates `id` inside `definition`'s cells: `(layer, cell, position)`.
fn definition_owner(
    document: &Document,
    symbol: i64,
    id: ObjectId,
) -> Option<(usize, usize, usize)> {
    let definition = document
        .symbols
        .iter()
        .find(|definition| definition.symbol_number == symbol)?;
    for (layer, layer_data) in definition.layers.layers.iter().enumerate() {
        for (cell, cell_data) in layer_data.cells.iter().enumerate() {
            if let Some(position) = cell_data.elements.iter().position(|&element| element == id) {
                return Some((layer, cell, position));
            }
        }
    }
    None
}

/// Removes `id` from whichever definition cell holds it (the caller
/// captured the library beforehand). A no-op when the id is not placed
/// in this definition.
fn definition_unguarded_detach(document: &mut Document, symbol: i64, id: ObjectId) {
    if let Some((layer, cell, position)) = definition_owner(document, symbol, id) {
        if let Some(definition) = document
            .symbols
            .iter_mut()
            .find(|definition| definition.symbol_number == symbol)
        {
            definition.layers.layers[layer].cells[cell]
                .elements
                .remove(position);
        }
    }
}

/// Inserts `id` into a group's children at `position`, capturing the
/// group — the definition-side counterpart of [`attach_cell`].
fn attach_group(
    document: &mut Document,
    command: &'static str,
    parent: ObjectId,
    position: usize,
    id: ObjectId,
) -> Result<Delta, EngineError> {
    let before = match document.registry.get_mut(parent) {
        Some(ObjectData::Group(group)) => {
            if position > group.children.len() {
                return Err(EngineError::InvalidState {
                    name: command.to_owned(),
                    detail: format!(
                        "insert index {position} beyond {} children",
                        group.children.len()
                    ),
                });
            }
            let before = group.clone();
            group.children.insert(position, id);
            before
        }
        _ => {
            return Err(EngineError::MissingTarget {
                name: command.to_owned(),
                detail: format!(
                    "parent group {} (generation {}) is not live",
                    parent.index(),
                    parent.generation()
                ),
            });
        }
    };
    Ok(Delta::Object {
        id: parent,
        inverse: ObjectInverse::Mutated {
            object: Box::new(ObjectData::Group(before)),
        },
    })
}

/// Deep-copies the subtree rooted at `id` into fresh registry slots:
/// groups recurse with duplicated children, every other kind clones
/// as-is. The copy shares nothing mutable with the original.
fn duplicate_subtree(
    document: &mut Document,
    command: &'static str,
    id: ObjectId,
    depth: usize,
) -> Result<ObjectId, EngineError> {
    if depth > MAX_NESTING {
        return Err(EngineError::InvalidState {
            name: command.to_owned(),
            detail: format!("object nesting deeper than {MAX_NESTING} levels"),
        });
    }
    let object = access::object(document, command, id)?.clone();
    let object = match object {
        ObjectData::Group(mut group) => {
            let children = group.children.clone();
            let mut copied = Vec::with_capacity(children.len());
            for child in children {
                copied.push(duplicate_subtree(document, command, child, depth + 1)?);
            }
            group.children = copied;
            ObjectData::Group(group)
        }
        other => other,
    };
    Ok(document.registry.insert(object))
}

/// Read-only walk proving [`duplicate_subtree`] can complete: every
/// node live, no chain deeper than the ceiling. Run before any copy
/// registers so a refusal leaves the registry untouched (a recursive
/// definition — an instance of a symbol inside that same symbol —
/// would otherwise mint a deep chain of copies before failing).
fn ensure_duplicable(
    document: &Document,
    command: &'static str,
    id: ObjectId,
    depth: usize,
) -> Result<(), EngineError> {
    if depth > MAX_NESTING {
        return Err(EngineError::InvalidState {
            name: command.to_owned(),
            detail: format!("object nesting deeper than {MAX_NESTING} levels"),
        });
    }
    if let ObjectData::Group(group) = access::object(document, command, id)? {
        let children = group.children.clone();
        for child in children {
            ensure_duplicable(document, command, child, depth + 1)?;
        }
    }
    Ok(())
}

/// `symbol.create` — converts the selection into a new library
/// definition. The selected objects move into the definition's first
/// cell (in payload order), an instance replaces the first one at its
/// old stacking position, and the library gains the definition.
struct CreateSymbol;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct CreateSymbolPayload {
    /// The selected objects, bottom-first.
    ids: Vec<ObjectId>,
    /// Library name for the new definition.
    name: String,
    /// What kind of symbol to create.
    symbol_type: SymbolTypeChoice,
}

impl TypedCommand for CreateSymbol {
    type Payload = CreateSymbolPayload;

    fn name(&self) -> &'static str {
        "symbol.create"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.create";
        if payload.ids.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the selection is empty".to_owned(),
            });
        }
        if payload.ids.len()
            > payload
                .ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
        {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the selection names an object twice".to_owned(),
            });
        }
        // Pre-flight every member: all live, all detaching cleanly, all
        // containers known — so a refusal leaves every container as it
        // was.
        for id in &payload.ids {
            access::object(document, NAME, *id)?;
            ensure_detachable(document, NAME, *id)?;
        }
        let anchor = find_owner(document, payload.ids[0]);

        let mut containers = Vec::new();
        for id in &payload.ids {
            containers.push(detach(document, NAME, *id)?);
        }

        let symbol_number = next_symbol_number(document);
        let library_before = document.symbols.clone();
        document.symbols.push(SymbolDefinition {
            name: payload.name.clone(),
            symbol_number,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList {
                layers: vec![Layer {
                    name: String::new(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: payload.ids.clone(),
                    }],
                    ..Layer::default()
                }],
            },
            background_raw: 0,
            symbol_type: SymbolType::from(payload.symbol_type),
            nine_slice: None,
        });

        let instance = document
            .registry
            .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
                symbol_number,
                transform: geom::Affine::IDENTITY,
                locked: false,
                name: None,
                animation: None,
            }));
        let placed = match anchor {
            Some(Owner::Cell {
                page,
                layer,
                cell,
                position,
            }) => {
                // The anchor position is pre-detach; selected ids that
                // sat before it in the same cell shifted it left, so
                // clamp to what the shrunken cell can hold — this arm
                // must not fail after the library already grew.
                let len = access::cell_mut(document, NAME, page, layer, cell)?
                    .elements
                    .len();
                attach_cell(
                    document,
                    NAME,
                    page,
                    layer,
                    cell,
                    instance,
                    Some(position.min(len)),
                )
            }
            Some(Owner::Group {
                id: parent,
                position,
            }) => {
                let len = match document.registry.get(parent) {
                    Some(ObjectData::Group(group)) => group.children.len(),
                    _ => 0,
                };
                attach_group(document, NAME, parent, position.min(len), instance)
            }
            None => {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "anchor object {} (generation {}) lost its container mid-command",
                        payload.ids[0].index(),
                        payload.ids[0].generation()
                    ),
                });
            }
        }?;

        let mut deltas = containers;
        deltas.push(Delta::Library {
            before: library_before,
        });
        deltas.push(placed);
        deltas.push(Delta::Object {
            id: instance,
            inverse: ObjectInverse::Created,
        });
        Ok(Delta::Group(deltas))
    }
}

/// `symbol.createInstance` — places a new instance of a library
/// definition into one cell. The transform is a pure translation; the
/// shell scales and rotates instances with the object tools afterwards.
struct CreateInstance;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct CreateInstancePayload {
    /// The definition to instantiate, by `MSNi` number.
    symbol: i64,
    /// Target page index.
    page: usize,
    /// Target layer index, bottom-first.
    layer: usize,
    /// Target cell index (state number).
    cell: usize,
    /// Position in the element list; absent appends.
    index: Option<usize>,
    /// Placement translation, world coordinates.
    x: f64,
    /// Placement translation, world coordinates.
    y: f64,
    /// Optional instance name.
    name: Option<String>,
}

impl TypedCommand for CreateInstance {
    type Payload = CreateInstancePayload;

    fn name(&self) -> &'static str {
        "symbol.createInstance"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.createInstance";
        definition_position(document, NAME, payload.symbol)?;
        if !payload.x.is_finite() || !payload.y.is_finite() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "placement must be finite".to_owned(),
            });
        }
        // Validate the destination before anything registers.
        let len = access::cell_mut(document, NAME, payload.page, payload.layer, payload.cell)?
            .elements
            .len();
        let at = payload.index.unwrap_or(len);
        if at > len {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("insert index {at} beyond {len} elements"),
            });
        }

        let instance = document
            .registry
            .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
                symbol_number: payload.symbol,
                transform: geom::Affine::translate((payload.x, payload.y)),
                locked: false,
                name: payload.name.clone(),
                animation: None,
            }));
        let placed = attach_cell(
            document,
            NAME,
            payload.page,
            payload.layer,
            payload.cell,
            instance,
            Some(at),
        )?;
        Ok(Delta::Group(vec![
            placed,
            Delta::Object {
                id: instance,
                inverse: ObjectInverse::Created,
            },
        ]))
    }
}

/// `symbol.breakApart` — converts one instance back into raw objects:
/// the definition's content is duplicated into fresh registry slots and
/// placed where the instance stood. The definition and any other
/// instances stay untouched. Multi-state definitions refuse — their
/// states would need to distribute across page frames, which has no
/// single correct answer yet.
struct BreakApart;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct BreakApartPayload {
    /// The instance to break apart.
    id: ObjectId,
}

impl TypedCommand for BreakApart {
    type Payload = BreakApartPayload;

    fn name(&self) -> &'static str {
        "symbol.breakApart"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.breakApart";
        let instance = match access::object(document, NAME, payload.id)? {
            ObjectData::SymbolInstance(instance) => instance.clone(),
            _ => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a symbol instance", payload.id.index()),
                });
            }
        };
        let position = definition_position(document, NAME, instance.symbol_number)?;
        let (content, frame_count) = {
            let definition = &document.symbols[position];
            (
                definition
                    .layers
                    .layers
                    .first()
                    .and_then(|layer| layer.cells.first().map(|cell| cell.elements.clone())),
                definition.frame_count,
            )
        };
        if frame_count != 1 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "symbol {} has {frame_count} states; break apart is defined \
                     for single-state symbols",
                    instance.symbol_number
                ),
            });
        }
        let content = content.unwrap_or_default();

        // Pre-flight the placement: an unplaced instance must refuse
        // before any copy registers (failed calls mutate nothing).
        ensure_detachable(document, NAME, payload.id)?;
        // Pre-flight the content walk: a definition whose own instance
        // sits inside it recurses — refuse before any copy registers.
        for element in &content {
            ensure_duplicable(document, NAME, *element, 0)?;
        }

        // Duplicate first: every copy registers before anything else
        // moves, so undo's LIFO retraction order lines up.
        let mut copies = Vec::with_capacity(content.len());
        for element in &content {
            copies.push(duplicate_subtree(document, NAME, *element, 0)?);
        }

        // The anchor is the instance's pre-detach stacking position —
        // capture it before the detach removes the instance from its
        // container.
        let anchor = find_owner(document, payload.id);
        let removed = detach(document, NAME, payload.id)?;
        let placed = match anchor {
            Some(Owner::Cell {
                page,
                layer,
                cell,
                position,
            }) => {
                let mut deltas = Vec::with_capacity(copies.len());
                for (offset, copy) in copies.iter().enumerate() {
                    deltas.push(attach_cell(
                        document,
                        NAME,
                        page,
                        layer,
                        cell,
                        *copy,
                        Some(position + offset),
                    )?);
                }
                deltas
            }
            Some(Owner::Group {
                id: parent,
                position,
            }) => {
                let mut deltas = Vec::with_capacity(copies.len());
                for (offset, copy) in copies.iter().enumerate() {
                    deltas.push(attach_group(
                        document,
                        NAME,
                        parent,
                        position + offset,
                        *copy,
                    )?);
                }
                deltas
            }
            None => {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "instance {} (generation {}) lost its container mid-command",
                        payload.id.index(),
                        payload.id.generation()
                    ),
                });
            }
        };

        let mut deltas = Vec::with_capacity(copies.len() + placed.len() + 2);
        for copy in &copies {
            deltas.push(Delta::Object {
                id: *copy,
                inverse: ObjectInverse::Created,
            });
        }
        deltas.push(removed);
        deltas.extend(placed);
        Ok(Delta::Group(deltas))
    }
}

/// `symbol.setType` — reclassifies a definition (graphic / button /
/// animation). Rendering and instance spans read this per frame, so the
/// change takes effect without touching instances.
struct SetSymbolType;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetSymbolTypePayload {
    /// The definition, by `MSNi` number.
    symbol: i64,
    /// The new type.
    symbol_type: SymbolTypeChoice,
}

impl TypedCommand for SetSymbolType {
    type Payload = SetSymbolTypePayload;

    fn name(&self) -> &'static str {
        "symbol.setType"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setType";
        let position = definition_position(document, NAME, payload.symbol)?;
        let before = document.symbols.clone();
        document.symbols[position].symbol_type = SymbolType::from(payload.symbol_type);
        Ok(Delta::Library { before })
    }
}

/// `symbol.set9Slice` — installs or clears `NGSv` guides on a
/// definition. Controls must be finite; bounds are unvalidated beyond
/// that (they are data, and the renderer re-grids from whatever they
/// say).
struct SetSymbol9Slice;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetSymbol9SlicePayload {
    /// The definition, by `MSNi` number.
    symbol: i64,
    /// `Some` installs the guides, `None` clears them.
    guides: Option<NineSliceGuides>,
}

impl TypedCommand for SetSymbol9Slice {
    type Payload = SetSymbol9SlicePayload;

    fn name(&self) -> &'static str {
        "symbol.set9Slice"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.set9Slice";
        if let Some(guides) = &payload.guides {
            let finite = guides.controls.iter().all(|value| value.is_finite())
                && [
                    guides.shape_bounds.x0,
                    guides.shape_bounds.y0,
                    guides.shape_bounds.x1,
                    guides.shape_bounds.y1,
                ]
                .iter()
                .all(|value| value.is_finite())
                && [
                    guides.limits.x0,
                    guides.limits.y0,
                    guides.limits.x1,
                    guides.limits.y1,
                ]
                .iter()
                .all(|value| value.is_finite());
            if !finite {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: "nine-slice guides must be finite".to_owned(),
                });
            }
        }
        let position = definition_position(document, NAME, payload.symbol)?;
        let before = document.symbols.clone();
        document.symbols[position].nine_slice = payload.guides.clone();
        Ok(Delta::Library { before })
    }
}

/// `symbol.editAttach` — moves an object into a definition cell. The
/// one-owner invariant holds: a page-placed object detaches first (its
/// container captures the change), a definition-placed object is lifted
/// out (covered by the library capture).
struct EditAttach;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct EditAttachPayload {
    /// Target definition, by `MSNi` number.
    symbol: i64,
    /// The object to attach (it may already live anywhere).
    id: ObjectId,
    /// Target definition layer index.
    layer: usize,
    /// Target definition cell index (state number).
    cell: usize,
    /// Position in the element list; absent appends.
    index: Option<usize>,
}

impl TypedCommand for EditAttach {
    type Payload = EditAttachPayload;

    fn name(&self) -> &'static str {
        "symbol.editAttach"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.editAttach";
        definition_position(document, NAME, payload.symbol)?;
        access::object(document, NAME, payload.id)?;
        // Destination bounds first, so a bad index refuses before any
        // container or library state moves. The bound is the cell's
        // length *after* the lift-out: when the object already sits in
        // this very cell, the lift shrinks it by one, and validating
        // against the pre-lift length would let `insert` run past the
        // end later.
        let (len, already_here) = {
            let target =
                definition_cell_mut(document, NAME, payload.symbol, payload.layer, payload.cell)?;
            let already_here = target.elements.contains(&payload.id);
            (target.elements.len(), already_here)
        };
        let bound = len - usize::from(already_here);
        let at = payload.index.unwrap_or(bound);
        if at > bound {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("insert index {at} beyond {bound} elements"),
            });
        }

        let library_before = document.symbols.clone();
        let mut deltas: Vec<Delta> = Vec::new();
        // A page- or group-placed object detaches first (its container
        // captures the move); a definition-only object needs no page
        // detach. Either way the lift-out below keeps exactly one
        // placement — inside the definition.
        if find_owner(document, payload.id).is_some() {
            deltas.push(detach(document, NAME, payload.id)?);
        }
        definition_unguarded_detach(document, payload.symbol, payload.id);
        let target =
            definition_cell_mut(document, NAME, payload.symbol, payload.layer, payload.cell)?;
        target.elements.insert(at, payload.id);
        deltas.push(Delta::Library {
            before: library_before,
        });
        Ok(Delta::Group(deltas))
    }
}

/// `symbol.editDetach` — lifts an object out of a definition cell. The
/// object stays in the registry, exactly like `object.delete`'s
/// container phase leaves things when only detaching.
struct EditDetach;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct EditDetachPayload {
    /// Target definition, by `MSNi` number.
    symbol: i64,
    /// The object to lift out.
    id: ObjectId,
}

impl TypedCommand for EditDetach {
    type Payload = EditDetachPayload;

    fn name(&self) -> &'static str {
        "symbol.editDetach"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.editDetach";
        definition_position(document, NAME, payload.symbol)?;
        access::object(document, NAME, payload.id)?;
        if definition_owner(document, payload.symbol, payload.id).is_none() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "object {} is not placed in symbol {}",
                    payload.id.index(),
                    payload.symbol
                ),
            });
        }
        let before = document.symbols.clone();
        definition_unguarded_detach(document, payload.symbol, payload.id);
        Ok(Delta::Library { before })
    }
}

/// `symbol.editReorder` — moves an object within its definition cell
/// (z-order inside the symbol).
struct EditReorder;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct EditReorderPayload {
    /// Target definition, by `MSNi` number.
    symbol: i64,
    /// The object to reorder.
    id: ObjectId,
    /// Target index in the cell list (after removal).
    to: usize,
}

impl TypedCommand for EditReorder {
    type Payload = EditReorderPayload;

    fn name(&self) -> &'static str {
        "symbol.editReorder"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.editReorder";
        definition_position(document, NAME, payload.symbol)?;
        access::object(document, NAME, payload.id)?;
        let Some((layer, cell, position)) = definition_owner(document, payload.symbol, payload.id)
        else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "object {} is not placed in symbol {}",
                    payload.id.index(),
                    payload.symbol
                ),
            });
        };
        let len = definition_cell_mut(document, NAME, payload.symbol, layer, cell)?
            .elements
            .len();
        if payload.to >= len {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("index {} beyond {len} elements", payload.to),
            });
        }
        let before = document.symbols.clone();
        let target = definition_cell_mut(document, NAME, payload.symbol, layer, cell)?;
        let moved = target.elements.remove(position);
        target.elements.insert(payload.to, moved);
        Ok(Delta::Library { before })
    }
}

/// `library.add` — appends an empty single-cell definition.
struct LibraryAdd;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct LibraryAddPayload {
    /// Library name for the definition.
    name: String,
    /// What kind of symbol it is.
    symbol_type: SymbolTypeChoice,
}

impl TypedCommand for LibraryAdd {
    type Payload = LibraryAddPayload;

    fn name(&self) -> &'static str {
        "library.add"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        let before = document.symbols.clone();
        document.symbols.push(SymbolDefinition {
            name: payload.name.clone(),
            symbol_number: next_symbol_number(document),
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList {
                layers: vec![Layer {
                    name: String::new(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: Vec::new(),
                    }],
                    ..Layer::default()
                }],
            },
            background_raw: 0,
            symbol_type: SymbolType::from(payload.symbol_type),
            nine_slice: None,
        });
        Ok(Delta::Library { before })
    }
}

/// `library.remove` — deletes a definition. Refused while any instance
/// still references it: a dangling `MSNi` is a document defect under
/// the render contract, so the command refuses rather than minting one.
struct LibraryRemove;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct LibraryRemovePayload {
    /// The definition, by `MSNi` number.
    symbol: i64,
}

impl TypedCommand for LibraryRemove {
    type Payload = LibraryRemovePayload;

    fn name(&self) -> &'static str {
        "library.remove"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "library.remove";
        let position = definition_position(document, NAME, payload.symbol)?;
        let live = instance_count(document, payload.symbol);
        if live > 0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "symbol {} still has {live} instance(s); break or delete them first",
                    payload.symbol
                ),
            });
        }
        let before = document.symbols.clone();
        document.symbols.remove(position);
        Ok(Delta::Library { before })
    }
}

/// `library.rename` — sets a definition's `UNMs` name.
struct LibraryRename;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct LibraryRenamePayload {
    /// The definition, by `MSNi` number.
    symbol: i64,
    /// The new name.
    name: String,
}

impl TypedCommand for LibraryRename {
    type Payload = LibraryRenamePayload;

    fn name(&self) -> &'static str {
        "library.rename"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "library.rename";
        let position = definition_position(document, NAME, payload.symbol)?;
        let before = document.symbols.clone();
        document.symbols[position].name = payload.name.clone();
        Ok(Delta::Library { before })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::super::testdoc;
    use super::*;
    use serde_json::json;

    fn registry() -> CommandRegistry {
        let mut registry = CommandRegistry::new();
        install(&mut registry).unwrap();
        registry
    }

    fn cell_elements(document: &Document) -> &[ObjectId] {
        &document.pages[0].layers.layers[0].cells[0].elements
    }

    fn instance_of(document: &Document) -> (ObjectId, i64) {
        for (id, object) in document.registry.iter() {
            if let ObjectData::SymbolInstance(instance) = object {
                return (id, instance.symbol_number);
            }
        }
        panic!("no instance in the document");
    }

    #[test]
    fn create_moves_selection_into_a_definition_and_back() {
        let (mut document, ids) = testdoc::sample_document();
        let original = serde_json::to_vec(&document).unwrap();
        let payload = json!({
            "ids": [ids[0], ids[1]],
            "name": "badge",
            "symbol_type": "graphic",
        });
        let registry = registry();
        let delta = registry
            .execute(&mut document, "symbol.create", payload)
            .unwrap();

        let (instance, number) = instance_of(&document);
        assert_eq!(number, 0, "first definition numbers from zero");
        assert_eq!(
            cell_elements(&document),
            &[instance],
            "the instance replaces the selection at its position"
        );
        let definition = &document.symbols[0];
        assert_eq!(definition.name, "badge");
        assert_eq!(definition.symbol_type, SymbolType::Graphic);
        assert_eq!(
            definition.layers.layers[0].cells[0].elements,
            vec![ids[0], ids[1]]
        );

        delta.apply_inverse(&mut document).unwrap();
        assert!(document.symbols.is_empty());
        assert!(document.registry.get(instance).is_none());
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            original,
            "undo restores the exact pre-create bytes"
        );
    }

    #[test]
    fn create_from_a_fully_selected_cell_clamps_the_anchor() {
        // The whole anchor cell is selected: after the detaches the
        // anchor position (1) exceeds the emptied cell, and the command
        // must clamp instead of failing after the library grew.
        let (mut document, ids) = testdoc::sample_document();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        let delta = registry
            .execute(
                &mut document,
                "symbol.create",
                json!({
                    "ids": [ids[1], ids[0]],
                    "name": "clamped",
                    "symbol_type": "graphic",
                }),
            )
            .unwrap();
        let (instance, _) = instance_of(&document);
        assert_eq!(cell_elements(&document), &[instance]);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
    }

    #[test]
    fn edit_attach_into_its_own_cell_validates_after_the_lift() {
        let (mut document, ids) = testdoc::sample_document();
        document.pages[0].layers.layers[0].cells[0].elements.clear();
        document.pages[1].layers.layers[0].cells[0].elements.clear();
        document.symbols.push(SymbolDefinition {
            name: "target".to_owned(),
            symbol_number: 0,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList {
                layers: vec![Layer {
                    name: String::new(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: vec![ids[0], ids[1]],
                    }],
                    ..Layer::default()
                }],
            },
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
        });
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();

        // ids[0] sits in the target cell: the lift shrinks it to one
        // element, so index 2 is out of bounds and must refuse before
        // any state moves.
        let error = registry
            .execute(
                &mut document,
                "symbol.editAttach",
                json!({ "symbol": 0, "id": ids[0], "layer": 0, "cell": 0, "index": 2 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);

        // Index 1 (the post-lift top) is a same-cell reorder.
        let delta = registry
            .execute(
                &mut document,
                "symbol.editAttach",
                json!({ "symbol": 0, "id": ids[0], "layer": 0, "cell": 0, "index": 1 }),
            )
            .unwrap();
        assert_eq!(
            document.symbols[0].layers.layers[0].cells[0].elements,
            vec![ids[1], ids[0]]
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
    }

    #[test]
    fn create_numbers_from_the_libraries_high() {
        let (mut document, ids) = testdoc::sample_document();
        document.symbols.push(SymbolDefinition {
            name: "seeded".to_owned(),
            symbol_number: 4,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList::default(),
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
        });
        let registry = registry();
        registry
            .execute(
                &mut document,
                "symbol.create",
                json!({ "ids": [ids[0]], "name": "next", "symbol_type": "button" }),
            )
            .unwrap();
        assert_eq!(document.symbols[1].symbol_number, 5);
        assert_eq!(document.symbols[1].symbol_type, SymbolType::Button);
    }

    #[test]
    fn create_refuses_empty_duplicate_or_detached_selections_without_mutation() {
        let (mut document, ids) = testdoc::sample_document();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        for payload in [
            json!({ "ids": [], "name": "x", "symbol_type": "graphic" }),
            json!({ "ids": [ids[0], ids[0]], "name": "x", "symbol_type": "graphic" }),
        ] {
            let error = registry
                .execute(&mut document, "symbol.create", payload)
                .unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }));
            assert_eq!(
                serde_json::to_vec(&document).unwrap(),
                original,
                "a refused create must not mutate"
            );
        }
        // A stale id in the selection dies in the pre-flight — after
        // the empty/duplicate checks, before anything detaches.
        let stale = doc::ObjectId::from_parts(999, 0);
        let error = registry
            .execute(
                &mut document,
                "symbol.create",
                json!({ "ids": [ids[0], stale], "name": "x", "symbol_type": "graphic" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
    }

    #[test]
    fn create_instance_places_and_undo_removes() {
        let (mut document, _) = testdoc::sample_document();
        document.symbols.push(SymbolDefinition {
            name: "dot".to_owned(),
            symbol_number: 7,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList::default(),
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
        });
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        let delta = registry
            .execute(
                &mut document,
                "symbol.createInstance",
                json!({
                    "symbol": 7, "page": 0, "layer": 1, "cell": 1,
                    "index": 0, "x": 12.0, "y": -3.0, "name": "dot 1",
                }),
            )
            .unwrap();
        let (instance, number) = instance_of(&document);
        assert_eq!(number, 7);
        assert_eq!(
            document.pages[0].layers.layers[1].cells[1].elements,
            vec![instance]
        );
        match document.registry.get(instance).unwrap() {
            ObjectData::SymbolInstance(instance) => {
                assert_eq!(instance.transform, geom::Affine::translate((12.0, -3.0)));
                assert_eq!(instance.name.as_deref(), Some("dot 1"));
            }
            other => panic!("wrong object kind: {other:?}"),
        }
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.registry.get(instance).is_none());
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
    }

    #[test]
    fn create_instance_refuses_dangling_numbers_and_bad_cells() {
        let (mut document, _) = testdoc::sample_document();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        for payload in [
            json!({ "symbol": 9, "page": 0, "layer": 0, "cell": 0, "x": 0.0, "y": 0.0 }),
            json!({ "symbol": 0, "page": 9, "layer": 0, "cell": 0, "x": 0.0, "y": 0.0 }),
            json!({ "symbol": 0, "page": 0, "layer": 0, "cell": 0, "index": 9, "x": 0.0, "y": 0.0 }),
            json!({ "symbol": 0, "page": 0, "layer": 0, "cell": 0, "x": " NaN", "y": 0.0 }),
        ] {
            let error = registry
                .execute(&mut document, "symbol.createInstance", payload)
                .unwrap_err();
            assert!(
                matches!(error, EngineError::MissingTarget { .. })
                    || matches!(error, EngineError::InvalidState { .. })
                    || matches!(error, EngineError::BadPayload { .. })
            );
            assert_eq!(serde_json::to_vec(&document).unwrap(), original);
        }
    }

    #[test]
    fn break_apart_copies_definition_content_where_the_instance_stood() {
        let (mut document, ids) = testdoc::sample_document();
        // A definition holding one path, an instance of it in cell 0.
        let content = ids[0];
        document.symbols.push(SymbolDefinition {
            name: "solo".to_owned(),
            symbol_number: 2,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList {
                layers: vec![Layer {
                    name: String::new(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: vec![content],
                    }],
                    ..Layer::default()
                }],
            },
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
        });
        let instance = document
            .registry
            .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
                symbol_number: 2,
                transform: geom::Affine::IDENTITY,
                locked: false,
                name: None,
                animation: None,
            }));
        document.pages[0].layers.layers[0].cells[0].elements = vec![instance, ids[1]];
        let original = serde_json::to_vec(&document).unwrap();

        let registry = registry();
        let delta = registry
            .execute(
                &mut document,
                "symbol.breakApart",
                json!({ "id": instance }),
            )
            .unwrap();
        // The copy sits where the instance stood; the definition keeps
        // its own content.
        let placed = cell_elements(&document).to_vec();
        assert_eq!(placed.len(), 2, "the copy takes the instance's slot");
        assert!(placed.contains(&ids[1]));
        assert_ne!(placed[0], content, "the copy is a fresh slot");
        match document.registry.get(placed[0]).unwrap() {
            ObjectData::Path(path) => match document.registry.get(content).unwrap() {
                ObjectData::Path(original_path) => {
                    assert_eq!(path.contours, original_path.contours);
                }
                other => panic!("wrong object kind: {other:?}"),
            },
            other => panic!("wrong object kind: {other:?}"),
        }
        assert_eq!(
            document.symbols[0].layers.layers[0].cells[0].elements,
            vec![content]
        );

        delta.apply_inverse(&mut document).unwrap();
        assert!(document.registry.get(placed[0]).is_none());
        assert_eq!(cell_elements(&document), &[instance, ids[1]]);
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
    }

    #[test]
    fn break_apart_refuses_non_instances_and_multi_state_definitions() {
        let (mut document, ids) = testdoc::sample_document();
        document.symbols.push(SymbolDefinition {
            name: "multi".to_owned(),
            symbol_number: 1,
            origin: geom::Point::ZERO,
            frame_count: 2,
            layers: LayerList::default(),
            background_raw: 0,
            symbol_type: SymbolType::Button,
            nine_slice: None,
        });
        let instance = document
            .registry
            .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
                symbol_number: 1,
                transform: geom::Affine::IDENTITY,
                locked: false,
                name: None,
                animation: None,
            }));
        document.pages[0].layers.layers[0].cells[0]
            .elements
            .push(instance);
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();

        let error = registry
            .execute(&mut document, "symbol.breakApart", json!({ "id": ids[0] }))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        let error = registry
            .execute(
                &mut document,
                "symbol.breakApart",
                json!({ "id": instance }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
    }

    #[test]
    fn set_type_and_nine_slice_capture_and_undo() {
        let (mut document, _) = testdoc::sample_document();
        document.symbols.push(SymbolDefinition {
            name: "slicee".to_owned(),
            symbol_number: 0,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList::default(),
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
        });
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();

        let delta = registry
            .execute(
                &mut document,
                "symbol.setType",
                json!({ "symbol": 0, "symbol_type": "animation" }),
            )
            .unwrap();
        assert_eq!(document.symbols[0].symbol_type, SymbolType::Animation);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.symbols[0].symbol_type, SymbolType::Graphic);
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);

        let guides = doc::NineSliceGuides {
            locked: true,
            color_raw: 0x00_80_FF,
            controls: [3.0, 5.0, 2.0, 7.0],
            shape_bounds: geom::Rect::new(0.0, 0.0, 40.0, 20.0),
            limits: geom::Rect::new(0.0, 0.0, 40.0, 20.0),
        };
        let delta = registry
            .execute(
                &mut document,
                "symbol.set9Slice",
                json!({ "symbol": 0, "guides": guides }),
            )
            .unwrap();
        assert_eq!(document.symbols[0].nine_slice.as_ref(), Some(&guides));
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.symbols[0].nine_slice.is_none());
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);

        // Clearing an absent guide set is a valid no-change command.
        registry
            .execute(
                &mut document,
                "symbol.set9Slice",
                json!({ "symbol": 0, "guides": null }),
            )
            .unwrap();
        // Non-finite controls refuse.
        let error = registry
            .execute(
                &mut document,
                "symbol.set9Slice",
                json!({
                    "symbol": 0,
                    "guides": {
                        "locked": false, "color_raw": 0,
                        "controls": [1.0, 2.0, 3.0, 4.0],
                        "shape_bounds": { "x0": 0.0, "y0": 0.0, "x1": 4.0, "y1": 4.0 },
                        "limits": { "x0": 0.0, "y0": 0.0, "x1": 4.0, "y1": "NaN" },
                    },
                }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
    }

    #[test]
    fn edit_attach_moves_an_object_into_the_definition() {
        let (mut document, ids) = testdoc::sample_document();
        document.symbols.push(SymbolDefinition {
            name: "target".to_owned(),
            symbol_number: 0,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList {
                layers: vec![Layer {
                    name: String::new(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: Vec::new(),
                    }],
                    ..Layer::default()
                }],
            },
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
        });
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();

        let delta = registry
            .execute(
                &mut document,
                "symbol.editAttach",
                json!({ "symbol": 0, "id": ids[0], "layer": 0, "cell": 0 }),
            )
            .unwrap();
        assert_eq!(
            document.symbols[0].layers.layers[0].cells[0].elements,
            vec![ids[0]]
        );
        assert_eq!(cell_elements(&document), &[ids[1]], "left the page cell");
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(cell_elements(&document), vec![ids[0], ids[1]]);
        assert!(
            document.symbols[0].layers.layers[0].cells[0]
                .elements
                .is_empty()
        );
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
    }

    #[test]
    fn edit_attach_never_makes_a_double_reference() {
        let (mut document, ids) = testdoc::sample_document();
        document.symbols.push(SymbolDefinition {
            name: "target".to_owned(),
            symbol_number: 0,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList {
                layers: vec![Layer {
                    name: String::new(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: vec![ids[1]],
                    }],
                    ..Layer::default()
                }],
            },
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
        });
        // ids[1] is inside the definition AND (from the fixture) in the
        // page cell — a corrupted start. editAttach of ids[1] lifts it
        // out of the page (definition side keeps the single placement).
        let registry = registry();
        registry
            .execute(
                &mut document,
                "symbol.editAttach",
                json!({ "symbol": 0, "id": ids[1], "layer": 0, "cell": 0, "index": 0 }),
            )
            .unwrap();
        assert_eq!(
            document.symbols[0].layers.layers[0].cells[0].elements,
            vec![ids[1]]
        );
        // And attaching ids[0] twice ends with one placement.
        registry
            .execute(
                &mut document,
                "symbol.editAttach",
                json!({ "symbol": 0, "id": ids[0], "layer": 0, "cell": 0 }),
            )
            .unwrap();
        let once = document.symbols[0].layers.layers[0].cells[0]
            .elements
            .iter()
            .filter(|&&element| element == ids[0])
            .count();
        assert_eq!(once, 1);
        assert_eq!(cell_elements(&document), &[]);
    }

    #[test]
    fn edit_detach_and_reorder_round_trip() {
        let (mut document, ids) = testdoc::sample_document();
        document.symbols.push(SymbolDefinition {
            name: "target".to_owned(),
            symbol_number: 3,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList {
                layers: vec![Layer {
                    name: String::new(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: vec![ids[0], ids[1]],
                    }],
                    ..Layer::default()
                }],
            },
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
        });
        // Both ids start page-placed too (fixture); lift them out so the
        // definition is their only owner, then exercise detach/reorder.
        document.pages[0].layers.layers[0].cells[0].elements.clear();
        document.pages[1].layers.layers[0].cells[0].elements.clear();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();

        let delta = registry
            .execute(
                &mut document,
                "symbol.editReorder",
                json!({ "symbol": 3, "id": ids[0], "to": 1 }),
            )
            .unwrap();
        assert_eq!(
            document.symbols[0].layers.layers[0].cells[0].elements,
            vec![ids[1], ids[0]]
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);

        let delta = registry
            .execute(
                &mut document,
                "symbol.editDetach",
                json!({ "symbol": 3, "id": ids[0] }),
            )
            .unwrap();
        assert_eq!(
            document.symbols[0].layers.layers[0].cells[0].elements,
            vec![ids[1]]
        );
        assert!(
            document.registry.get(ids[0]).is_some(),
            "detach keeps the object"
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);

        // Detaching something the definition does not hold refuses
        // (ids[1] still sits in page 1's first cell — live, but not a
        // member of this definition).
        let error = registry
            .execute(
                &mut document,
                "symbol.editDetach",
                json!({ "symbol": 3, "id": ids[2] }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
    }

    #[test]
    fn library_add_remove_rename_round_trip() {
        let (mut document, _) = testdoc::sample_document();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();

        let delta = registry
            .execute(
                &mut document,
                "library.add",
                json!({ "name": "fresh", "symbol_type": "button" }),
            )
            .unwrap();
        assert_eq!(document.symbols.len(), 1);
        assert_eq!(document.symbols[0].symbol_number, 0);
        assert_eq!(document.symbols[0].symbol_type, SymbolType::Button);
        assert_eq!(document.symbols[0].layers.layers[0].cells.len(), 1);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);

        // Rename and remove on a missing definition refuse.
        let error = registry
            .execute(
                &mut document,
                "library.rename",
                json!({ "symbol": 0, "name": "x" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));

        registry
            .execute(
                &mut document,
                "library.add",
                json!({ "name": "keep", "symbol_type": "graphic" }),
            )
            .unwrap();
        let delta = registry
            .execute(
                &mut document,
                "library.rename",
                json!({ "symbol": 0, "name": "kept" }),
            )
            .unwrap();
        assert_eq!(document.symbols[0].name, "kept");
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.symbols[0].name, "keep");

        // Removing while instances exist refuses — and mutates nothing.
        // (Baseline is taken after the setup insert: the refusal must
        // leave the with-instance state exactly as it stands.)
        let instance = document
            .registry
            .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
                symbol_number: 0,
                transform: geom::Affine::IDENTITY,
                locked: false,
                name: None,
                animation: None,
            }));
        document.pages[0].layers.layers[0].cells[0]
            .elements
            .push(instance);
        let baseline = serde_json::to_vec(&document).unwrap();
        let error = registry
            .execute(&mut document, "library.remove", json!({ "symbol": 0 }))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), baseline);
        document.pages[0].layers.layers[0].cells[0].elements.pop();
        document.registry.remove(instance).unwrap();

        // A live definition with no instances removes cleanly; the
        // byte-exact proof lives in the remove test below.
        let error = registry
            .execute(&mut document, "library.remove", json!({ "symbol": 5 }))
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
    }

    #[test]
    fn library_remove_and_undo_is_byte_exact() {
        let (mut document, _) = testdoc::sample_document();
        let registry = registry();
        registry
            .execute(
                &mut document,
                "library.add",
                json!({ "name": "gone", "symbol_type": "graphic" }),
            )
            .unwrap();
        let baseline = serde_json::to_vec(&document).unwrap();
        let delta = registry
            .execute(&mut document, "library.remove", json!({ "symbol": 0 }))
            .unwrap();
        assert!(document.symbols.is_empty());
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.symbols.len(), 1);
        assert_eq!(document.symbols[0].name, "gone");
        assert_eq!(serde_json::to_vec(&document).unwrap(), baseline);
    }

    #[test]
    fn unknown_symbol_numbers_are_missing_targets() {
        let (mut document, ids) = testdoc::sample_document();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        for (name, payload) in [
            (
                "symbol.setType",
                json!({ "symbol": 5, "symbol_type": "graphic" }),
            ),
            ("symbol.set9Slice", json!({ "symbol": 5, "guides": null })),
            (
                "symbol.editAttach",
                json!({ "symbol": 5, "id": ids[0], "layer": 0, "cell": 0 }),
            ),
            ("symbol.editDetach", json!({ "symbol": 5, "id": ids[0] })),
            (
                "symbol.editReorder",
                json!({ "symbol": 5, "id": ids[0], "to": 0 }),
            ),
            ("library.remove", json!({ "symbol": 5 })),
            ("library.rename", json!({ "symbol": 5, "name": "x" })),
        ] {
            let error = registry.execute(&mut document, name, payload).unwrap_err();
            assert!(matches!(error, EngineError::MissingTarget { .. }), "{name}");
            assert_eq!(serde_json::to_vec(&document).unwrap(), original, "{name}");
        }
    }

    #[test]
    fn payload_unknown_fields_are_rejected() {
        let (mut document, _) = testdoc::sample_document();
        let registry = registry();
        let error = registry
            .execute(
                &mut document,
                "library.rename",
                json!({ "symbol": 0, "name": "x", "extra": 1 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }));
    }
}
