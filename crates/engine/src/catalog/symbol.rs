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

use std::collections::{BTreeMap, BTreeSet};

use doc::{
    Cell, Document, ExportOptions, Layer, LayerList, NineSliceGuides, ObjectData, ObjectId,
    SymbolAnimation, SymbolDefinition, SymbolInstanceObject, SymbolType, TextRun,
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
    registry.register(SetButtonFlag {
        slot: ButtonSlot::AutoSlice,
    })?;
    registry.register(SetButtonFlag {
        slot: ButtonSlot::IncludeDownState,
    })?;
    registry.register(SetButtonFlag {
        slot: ButtonSlot::ShowDownOnLoad,
    })?;
    registry.register(SetButtonFlag {
        slot: ButtonSlot::IncludeOverWhileDown,
    })?;
    registry.register(SetButtonOptions)?;
    registry.register(SetScript)?;
    registry.register(SetParam)?;
    registry.register(SetCustomData)?;
    registry.register(SetAnimInstanceNumFrames)?;
    registry.register(SetAnimInstanceLoopCount)?;
    registry.register(SetAnimInstanceOffsetDist)?;
    registry.register(SetAnimInstanceRotationAmount)?;
    registry.register(SetAnimInstanceScaleAmount)?;
    registry.register(SetAnimInstanceStartEndOpacity)?;
    registry.register(EditAttach)?;
    registry.register(EditDetach)?;
    registry.register(EditReorder)?;
    registry.register(LibraryAdd)?;
    registry.register(LibraryRemove)?;
    registry.register(LibraryRename)?;
    registry.register(LibraryImport)?;
    registry.register(Tween)?;
    registry.register(ReplaceButtonTextStrings)?;
    registry.register(ReplaceButtonTextStringsInInstances)
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
            // CS6 writes the button-behavior defaults on every
            // definition (decision 0036): auto-slice on, the three
            // state boxes off.
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
        });

        let instance = document
            .registry
            .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
                symbol_number,
                transform: geom::Affine::IDENTITY,
                locked: false,
                name: None,
                animation: None,
                custom_data: std::collections::BTreeMap::new(),
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
                custom_data: std::collections::BTreeMap::new(),
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
        // resolve it before anything moves (the pre-flight above
        // already refuses unplaced instances; this is the same check
        // on the same data, kept ahead of the first mutation).
        let anchor = match find_owner(document, payload.id) {
            Some(anchor) => anchor,
            None => {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "instance {} (generation {}) is referenced by no cell or group",
                        payload.id.index(),
                        payload.id.generation()
                    ),
                });
            }
        };
        let removed = detach(document, NAME, payload.id)?;
        // Break apart dissolves the instance: it leaves the registry
        // too, not just its cell. An object referenced by nothing
        // would never reach the wire (the writer emits attached
        // objects only) and would fork the model on re-open.
        let object = match document.registry.remove(payload.id) {
            Some(object) => object,
            None => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "instance {} vanished before its removal",
                        payload.id.index()
                    ),
                });
            }
        };
        let placed = match anchor {
            Owner::Cell {
                page,
                layer,
                cell,
                position,
            } => {
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
            Owner::Group {
                id: parent,
                position,
            } => {
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
        };

        let mut deltas = Vec::with_capacity(copies.len() + placed.len() + 3);
        for copy in &copies {
            deltas.push(Delta::Object {
                id: *copy,
                inverse: ObjectInverse::Created,
            });
        }
        deltas.push(removed);
        deltas.push(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Removed {
                object: Box::new(object),
            },
        });
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

/// The button-behavior booleans (`ATSb`/`IDSb`/`DVLb`/`IODb` on `MSTv`,
/// decision 0036). One command per CS6 verb — `setButtonOptions` is the
/// only compound one, and it mutates slices, not the definition.
struct SetButtonFlag {
    slot: ButtonSlot,
}

#[derive(Clone, Copy, Debug)]
enum ButtonSlot {
    AutoSlice,
    IncludeDownState,
    ShowDownOnLoad,
    IncludeOverWhileDown,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetButtonFlagPayload {
    /// The definition, by `MSNi` number.
    symbol: i64,
    /// The new value.
    value: bool,
}

impl TypedCommand for SetButtonFlag {
    type Payload = SetButtonFlagPayload;

    fn name(&self) -> &'static str {
        match self.slot {
            ButtonSlot::AutoSlice => "symbol.setButtonAutoSlice",
            ButtonSlot::IncludeDownState => "symbol.setButtonIncludeDownState",
            ButtonSlot::ShowDownOnLoad => "symbol.setButtonShowDownOnLoad",
            ButtonSlot::IncludeOverWhileDown => "symbol.setButtonIncludeOverWhileDownState",
        }
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        let position = definition_position(document, self.name(), payload.symbol)?;
        let before = document.symbols.clone();
        let definition = &mut document.symbols[position];
        let slot = match self.slot {
            ButtonSlot::AutoSlice => &mut definition.auto_slice,
            ButtonSlot::IncludeDownState => &mut definition.include_down_state,
            ButtonSlot::ShowDownOnLoad => &mut definition.show_down_on_load,
            ButtonSlot::IncludeOverWhileDown => &mut definition.include_over_while_down,
        };
        *slot = Some(payload.value);
        Ok(Delta::Library { before })
    }
}

/// `symbol.setButtonOptions` — the button's export options (CS6
/// `dom.setButtonOptions`): the URL, alt tag, slice name and export
/// override live on the button's auto-slice, found through the
/// slice→definition `MSNi` link (decision 0036). Every present payload
/// field is applied to every linked slice; absent fields stay
/// untouched. `targetTagString`/`statusMessage` have no wire home
/// (decision 0036, deviation 1) and are not carried.
struct SetButtonOptions;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
struct SetButtonOptionsPayload {
    /// The definition, by `MSNi` number.
    symbol: i64,
    /// The anchor link URL (`A2Ts`).
    link: Option<String>,
    /// The alt tag (`ALTs`).
    alt: Option<String>,
    /// The slice name (`OBNs`); `None` leaves the name (CS6's null
    /// means auto-named).
    slice_name: Option<String>,
    /// The per-slice export override (`EXPv`).
    export: Option<ExportOptions>,
}

impl TypedCommand for SetButtonOptions {
    type Payload = SetButtonOptionsPayload;

    fn name(&self) -> &'static str {
        "symbol.setButtonOptions"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setButtonOptions";
        definition_position(document, NAME, payload.symbol)?;
        // Capture every linked slice's before-state in one pass, then
        // mutate — the delta holds prior state, so the capture may not
        // interleave with the writes.
        let touched: Vec<(ObjectId, ObjectData)> = document
            .registry
            .iter()
            .filter_map(|(id, data)| match data {
                ObjectData::Slice(slice) if slice.symbol_number == Some(payload.symbol) => {
                    Some((id, data.clone()))
                }
                _ => None,
            })
            .collect();
        if touched.is_empty() {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!(
                    "no slice linked to symbol {} (CS6 auto-slices its buttons; \
                     create one with web.addSlice and set its MSNi)",
                    payload.symbol
                ),
            });
        }
        for (id, _) in &touched {
            if let Some(ObjectData::Slice(slice)) = document.registry.get_mut(*id) {
                if let Some(link) = &payload.link {
                    slice.link = Some(link.clone());
                }
                if let Some(alt) = &payload.alt {
                    slice.alt = Some(alt.clone());
                }
                if let Some(name) = &payload.slice_name {
                    slice.name = Some(name.clone());
                }
                if let Some(export) = &payload.export {
                    slice.export = Some(export.clone());
                }
            }
        }
        let deltas = touched
            .into_iter()
            .map(|(id, before)| Delta::Object {
                id,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(before),
                },
            })
            .collect::<Vec<_>>();
        Ok(Delta::Group(deltas))
    }
}

/// `symbol.setScript` — installs or clears a definition's rich-symbol
/// script (`JSVv` on the wire). The source is data here: running it is
/// the scripting host's job (`scripting::rederive_symbol`), which
/// queues its mutations through the same registry.
struct SetScript;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetScriptPayload {
    /// The definition, by `MSNi` number.
    symbol: i64,
    /// `Some` installs the script source, `None` clears it.
    source: Option<String>,
}

impl TypedCommand for SetScript {
    type Payload = SetScriptPayload;

    fn name(&self) -> &'static str {
        "symbol.setScript"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setScript";
        let position = definition_position(document, NAME, payload.symbol)?;
        let before = document.symbols.clone();
        document.symbols[position].op_code = payload.source.clone();
        Ok(Delta::Library { before })
    }
}

/// The `customData["currentValues"]` entry shape the corpus's rich
/// symbols use: `{name, type, value}` triples in array order.
fn set_current_value(
    document: &mut Document,
    command: &'static str,
    id: doc::ObjectId,
    entry: serde_json::Value,
) -> Result<ObjectData, EngineError> {
    let before = access::object(document, command, id)?.clone();
    let Some(ObjectData::SymbolInstance(instance)) = document.registry.get_mut(id) else {
        return Err(EngineError::InvalidState {
            name: command.to_owned(),
            detail: format!("object {} is not a symbol instance", id.index()),
        });
    };
    let current = instance
        .custom_data
        .entry("currentValues".to_owned())
        .or_insert_with(|| "[]".to_owned());
    let mut values: Vec<serde_json::Value> =
        serde_json::from_str(current).map_err(|_| EngineError::InvalidState {
            name: command.to_owned(),
            detail: "currentValues is not a JSON array".to_owned(),
        })?;
    let name = entry
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if let Some(slot) = values
        .iter_mut()
        .find(|slot| slot.get("name").and_then(serde_json::Value::as_str) == Some(name.as_str()))
    {
        *slot = entry;
    } else {
        values.push(entry);
    }
    *current = serde_json::to_string(&values).map_err(|_| EngineError::InvalidState {
        name: command.to_owned(),
        detail: "currentValues does not serialize".to_owned(),
    })?;
    Ok(before)
}

/// `symbol.setParam` — create-or-update one `{name, type, value}`
/// entry in an instance's `currentValues`. The re-derivation that
/// consumes the change runs through the scripting host; the write
/// itself is a plain journaled mutation.
struct SetParam;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetParamPayload {
    /// The instance.
    id: doc::ObjectId,
    /// Parameter name (matched against existing entries).
    name: String,
    /// Parameter value (the corpus keeps these strings).
    value: String,
    /// Parameter type tag (`"color"`, `"font"`, `"text"`, …).
    kind: String,
}

impl TypedCommand for SetParam {
    type Payload = SetParamPayload;

    fn name(&self) -> &'static str {
        "symbol.setParam"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setParam";
        if payload.name.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "parameter name must not be empty".to_owned(),
            });
        }
        let entry = serde_json::json!({
            "name": payload.name,
            "type": payload.kind,
            "value": payload.value,
        });
        let before = set_current_value(document, NAME, payload.id, entry)?;
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// The animation verbs (`symbol.setAnimInstance*`) — one write per
/// field of the `ANNi` family. Each walks the selection's symbol
/// instances, handing the animation carrier to `edit` — materializing
/// the carrier (the Objects-panel defaults: one frame, no shift,
/// 100% scale, no rotation or opacity ramp) when the instance is
/// plain. Non-instance members skip — the verbs address "the selected
/// instances of multi-frame image symbols"; an empty selection
/// refuses, and a selection without instances is a `Noop`. Payload
/// validation happened in `apply`, before any capture.
/// The topmost text item of each of `definition`'s frames — CS6's
/// "Button Text items are defined as the topmost text items on each
/// frame". Z-order is list order (bottom-first), so the scan runs from
/// the cell's tail. Frames are the cells of every layer.
fn button_text_items(
    document: &Document,
    definition: &doc::SymbolDefinition,
) -> Vec<doc::ObjectId> {
    let mut found = Vec::new();
    for layer in &definition.layers.layers {
        for cell in &layer.cells {
            for id in cell.elements.iter().rev() {
                if matches!(document.registry.get(*id), Some(doc::ObjectData::Text(_))) {
                    found.push(*id);
                    break;
                }
            }
        }
    }
    found
}

/// `text.replaceButtonTextStrings` — CS6's
/// `dom.replaceButtonTextStrings(newString, uniformAttrs)`: replaces
/// the button text of every button-symbol definition in the library
/// with one run of `text`. `uniformAttrs` distinguishes per-character
/// attribute handling CS6's multi-run text model implements; the
/// single-run model has one style per object, so both spellings write
/// the same shape (recorded non-equivalence). An empty library or a
/// library without buttons is the no-op.
struct ReplaceButtonTextStrings;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ReplaceButtonTextStringsPayload {
    /// The replacement characters.
    text: String,
    /// CS6's `uniformAttrs` (no observable difference under the
    /// single-run text model).
    #[serde(default)]
    uniform_attrs: bool,
}

/// `text.replaceButtonTextStringsInInstances` — CS6's
/// `dom.replaceButtonTextStringsInInstances(newString, uniformAttrs)`:
/// the same replacement scoped to the definitions the selected
/// instances reference. An empty selection refuses; a selection with
/// no instances is the no-op.
struct ReplaceButtonTextStringsInInstances;

impl TypedCommand for ReplaceButtonTextStrings {
    type Payload = ReplaceButtonTextStringsPayload;

    fn name(&self) -> &'static str {
        "text.replaceButtonTextStrings"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        let numbers: Vec<i64> = document
            .symbols
            .iter()
            .filter(|definition| definition.symbol_type == doc::SymbolType::Button)
            .map(|definition| definition.symbol_number)
            .collect();
        replace_button_text(document, payload, &numbers)
    }
}

impl TypedCommand for ReplaceButtonTextStringsInInstances {
    type Payload = ReplaceButtonTextStringsPayload;

    fn name(&self) -> &'static str {
        "text.replaceButtonTextStringsInInstances"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        if document.object_selection.is_empty() {
            return Err(EngineError::InvalidState {
                name: self.name().to_owned(),
                detail: "nothing is selected".to_owned(),
            });
        }
        let mut numbers: Vec<i64> = Vec::new();
        for id in &document.object_selection {
            if let Some(doc::ObjectData::SymbolInstance(instance)) = document.registry.get(*id) {
                numbers.push(instance.symbol_number);
            }
        }
        replace_button_text(document, payload, &numbers)
    }
}

/// Shared replacement tail: the button text items of `numbers`'
/// definitions (duplicates skipped) get one run of the payload text.
/// Definition structure is untouched — the edits are plain object
/// mutations, so `Delta::Library` stays out of the transaction.
fn replace_button_text(
    document: &mut Document,
    payload: &ReplaceButtonTextStringsPayload,
    numbers: &[i64],
) -> Result<Delta, EngineError> {
    let mut targets: Vec<doc::ObjectId> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for symbol in numbers {
        if !seen.insert(*symbol) {
            continue;
        }
        let Some(position) = document
            .symbols
            .iter()
            .position(|definition| definition.symbol_number == *symbol)
        else {
            continue;
        };
        // Only button definitions carry button text; the
        // instances-scoped caller can hand us any definition number.
        if document.symbols[position].symbol_type != doc::SymbolType::Button {
            continue;
        }
        // Clone the cell contents out before mutating the registry —
        // the definition borrow ends where the edit begins.
        let definition = document.symbols[position].clone();
        targets.extend(button_text_items(document, &definition));
    }
    let mut deltas = Vec::new();
    for id in targets {
        let Some(before) = document.registry.get(id).cloned() else {
            continue;
        };
        let Some(doc::ObjectData::Text(slot)) = document.registry.get_mut(id) else {
            continue;
        };
        slot.runs = if payload.text.is_empty() {
            Vec::new()
        } else {
            vec![TextRun {
                text: payload.text.clone(),
            }]
        };
        deltas.push(Delta::Object {
            id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        });
    }
    if deltas.is_empty() {
        return Ok(Delta::Noop);
    }
    Ok(Delta::Group(deltas))
}

fn edit_animation_instances(
    document: &mut Document,
    name: &'static str,
    edit: impl Fn(&mut SymbolAnimation),
) -> Result<Delta, EngineError> {
    if document.object_selection.is_empty() {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "nothing is selected".to_owned(),
        });
    }
    let mut deltas = Vec::new();
    for id in document.object_selection.clone() {
        // Resolution reads only: a stale id skips (the walker rule),
        // never an error — a refusal after an earlier member mutated
        // would break the refused-calls-mutate-nothing law (fuzz seed
        // 5846161706420016621).
        let Some(before) = document.registry.get(id).cloned() else {
            continue;
        };
        let ObjectData::SymbolInstance(_) = &before else {
            continue;
        };
        let Some(ObjectData::SymbolInstance(instance)) = document.registry.get_mut(id) else {
            continue; // the read above settled existence and kind
        };
        let animation = instance.animation.get_or_insert(SymbolAnimation {
            frames: 1,
            x_shift: 0.0,
            y_shift: 0.0,
            scaling: 100.0,
            rotation: None,
            start_opacity: None,
            end_opacity: None,
        });
        edit(animation);
        deltas.push(Delta::Object {
            id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        });
    }
    if deltas.is_empty() {
        Ok(Delta::Noop)
    } else {
        Ok(Delta::Group(deltas))
    }
}

/// Guards the fields the dom documents as counts: a zero or negative
/// frame/loop count animates nothing.
fn positive_count(value: i64, field: &str, name: &'static str) -> Result<(), EngineError> {
    if value < 1 {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("{field} {value} must be at least 1"),
        });
    }
    Ok(())
}

/// `symbol.setAnimInstanceNumFrames` — CS6's
/// `dom.setAnimInstanceNumFrames(numFrames)` (Fireworks 4): "the
/// number of frames through which the symbol animates" — the `ANNi`
/// anchor.
struct SetAnimInstanceNumFrames;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetAnimInstanceNumFramesPayload {
    /// Frames to generate.
    frames: i64,
}

impl TypedCommand for SetAnimInstanceNumFrames {
    type Payload = SetAnimInstanceNumFramesPayload;

    fn name(&self) -> &'static str {
        "symbol.setAnimInstanceNumFrames"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setAnimInstanceNumFrames";
        positive_count(payload.frames, "frames", NAME)?;
        edit_animation_instances(document, NAME, |animation| {
            animation.frames = payload.frames;
        })
    }
}

/// `symbol.setAnimInstanceLoopCount` — CS6's
/// `dom.setAnimInstanceLoopCount(loopCount)` (Fireworks 3, deprecated
/// in 4 "in favor of `dom.setAnimInstanceNumFrames`"): Adobe's own
/// substitution is the mapping — the count writes `ANNi`.
struct SetAnimInstanceLoopCount;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetAnimInstanceLoopCountPayload {
    /// The loop count; the FW4 format folds it into the frame count.
    loop_count: i64,
}

impl TypedCommand for SetAnimInstanceLoopCount {
    type Payload = SetAnimInstanceLoopCountPayload;

    fn name(&self) -> &'static str {
        "symbol.setAnimInstanceLoopCount"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setAnimInstanceLoopCount";
        positive_count(payload.loop_count, "loop count", NAME)?;
        edit_animation_instances(document, NAME, |animation| {
            animation.frames = payload.loop_count;
        })
    }
}

/// `symbol.setAnimInstanceOffsetDist` — CS6's
/// `dom.setAnimInstanceOffsetDist(offsetDistPt)`: "a point that
/// specifies the distance the animation moves in pixels" — the
/// `ANXf`/`ANYf` pair.
struct SetAnimInstanceOffsetDist;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct OffsetPoint {
    x: f64,
    y: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetAnimInstanceOffsetDistPayload {
    /// Translation across the animation span.
    offset: OffsetPoint,
}

impl TypedCommand for SetAnimInstanceOffsetDist {
    type Payload = SetAnimInstanceOffsetDistPayload;

    fn name(&self) -> &'static str {
        "symbol.setAnimInstanceOffsetDist"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setAnimInstanceOffsetDist";
        if !payload.offset.x.is_finite() || !payload.offset.y.is_finite() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the offset point must be finite".to_owned(),
            });
        }
        edit_animation_instances(document, NAME, |animation| {
            animation.x_shift = payload.offset.x;
            animation.y_shift = payload.offset.y;
        })
    }
}

/// `symbol.setAnimInstanceRotationAmount` — CS6's
/// `dom.setAnimInstanceRotationAmount(rotationAmount)`: clockwise
/// degrees across the span ("720 does two complete clockwise
/// rotations; negative rotates counter-clockwise"). The wire field
/// `ANRi` is integral, so fractional degrees round (recorded
/// substitution).
struct SetAnimInstanceRotationAmount;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetAnimInstanceRotationAmountPayload {
    /// Clockwise degrees across the span; negative is counter.
    rotation: f64,
}

impl TypedCommand for SetAnimInstanceRotationAmount {
    type Payload = SetAnimInstanceRotationAmountPayload;

    fn name(&self) -> &'static str {
        "symbol.setAnimInstanceRotationAmount"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setAnimInstanceRotationAmount";
        if !payload.rotation.is_finite() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "rotation must be finite".to_owned(),
            });
        }
        let rounded = payload.rotation.round() as i64;
        edit_animation_instances(document, NAME, |animation| {
            animation.rotation = Some(rounded);
        })
    }
}

/// `symbol.setAnimInstanceScaleAmount` — CS6's
/// `dom.setAnimInstanceScaleAmount(scaleAmount)`: "a positive
/// floating-point value… pass 100 for no scaling" — `ASCf`.
struct SetAnimInstanceScaleAmount;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetAnimInstanceScaleAmountPayload {
    /// Percent-of-size at the span's end; 100 = no scaling.
    scale: f64,
}

impl TypedCommand for SetAnimInstanceScaleAmount {
    type Payload = SetAnimInstanceScaleAmountPayload;

    fn name(&self) -> &'static str {
        "symbol.setAnimInstanceScaleAmount"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setAnimInstanceScaleAmount";
        if !payload.scale.is_finite() || payload.scale <= 0.0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("scale {} must be finite and positive", payload.scale),
            });
        }
        edit_animation_instances(document, NAME, |animation| {
            animation.scaling = payload.scale;
        })
    }
}

/// `symbol.setAnimInstanceStartEndOpacity` — CS6's
/// `dom.setAnimInstanceStartEndOpacity(startOpacity, endOpacity)`:
/// "float values between 0 and 100" — the `ASOi`/`AEOi` pair (the
/// model stores per-mille like every opacity slot).
struct SetAnimInstanceStartEndOpacity;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetAnimInstanceStartEndOpacityPayload {
    /// Opacity at the span's start, 0–100.
    start: f64,
    /// Opacity at the span's end, 0–100.
    end: f64,
}

impl TypedCommand for SetAnimInstanceStartEndOpacity {
    type Payload = SetAnimInstanceStartEndOpacityPayload;

    fn name(&self) -> &'static str {
        "symbol.setAnimInstanceStartEndOpacity"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setAnimInstanceStartEndOpacity";
        for (field, value) in [("start", payload.start), ("end", payload.end)] {
            if !value.is_finite() || !(0.0..=100.0).contains(&value) {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("{field} opacity {value} must be finite within 0..=100"),
                });
            }
        }
        let start = (payload.start * 10.0).round() as i64;
        let end = (payload.end * 10.0).round() as i64;
        edit_animation_instances(document, NAME, |animation| {
            animation.start_opacity = Some(start);
            animation.end_opacity = Some(end);
        })
    }
}

/// `symbol.setCustomData` — raw per-instance `DTAv` entry write (the
/// scripting surface's `customData` store). `value: None` removes the
/// key.
struct SetCustomData;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetCustomDataPayload {
    /// The instance.
    id: doc::ObjectId,
    /// `DCKs` key.
    key: String,
    /// `DCVs` value; `None` deletes the entry.
    value: Option<String>,
}

impl TypedCommand for SetCustomData {
    type Payload = SetCustomDataPayload;

    fn name(&self) -> &'static str {
        "symbol.setCustomData"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.setCustomData";
        if payload.key.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "custom-data key must not be empty".to_owned(),
            });
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let Some(ObjectData::SymbolInstance(instance)) = document.registry.get_mut(payload.id)
        else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} is not a symbol instance", payload.id.index()),
            });
        };
        match &payload.value {
            Some(value) => {
                instance
                    .custom_data
                    .insert(payload.key.clone(), value.clone());
            }
            None => {
                instance.custom_data.remove(&payload.key);
            }
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
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
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
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

/// Map key for the import remap table — `ObjectId` is `Hash` but not
/// `Ord` (see `doc::library` for the same idiom).
type IdKey = (usize, u32);

/// The map key of an id.
fn id_key(id: ObjectId) -> IdKey {
    (id.index(), id.generation())
}

/// `library.importSymbol` — installs a definition and its member
/// closure from a foreign document (a library file's `MSLv` + unpaged
/// registry). Payload ids are *source* ids: members re-slot into fresh
/// registry slots and every reference (group children, definition
/// cell lists) is rewritten through the remap; the definition is
/// renumbered to the next free `MSNi`. The command refuses unless the
/// closure is complete — every id reachable from the definition
/// present exactly once in the payload — because a partial import
/// would mint dangling ids, the same defect `library.remove` refuses
/// to create. Payload members outside the reachable closure are
/// ignored: a library file's registry may carry members of *other*
/// definitions, and importing one symbol must not drag the rest in as
/// unreferenced registry orphans.
struct LibraryImport;

/// One member of the imported closure: its source-registry id and the
/// object as it lived there.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImportedMember {
    /// The object's id in the source document.
    pub(crate) id: ObjectId,
    /// The object data.
    pub(crate) object: ObjectData,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct LibraryImportPayload {
    /// The definition as it lived in the source document; layer lists
    /// still reference source ids and are remapped on the way in.
    pub(crate) definition: SymbolDefinition,
    /// The member closure in source-registry form.
    pub(crate) members: Vec<ImportedMember>,
}

impl TypedCommand for LibraryImport {
    type Payload = LibraryImportPayload;

    fn name(&self) -> &'static str {
        "library.importSymbol"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "library.importSymbol";
        // Closure check: every member id unique, every referenced id
        // (definition roots, group children inside members) present.
        let mut members: BTreeMap<IdKey, &ObjectData> = BTreeMap::new();
        for member in &payload.members {
            if members.insert(id_key(member.id), &member.object).is_some() {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "member {}:{} appears twice in the payload",
                        member.id.index(),
                        member.id.generation()
                    ),
                });
            }
        }
        let references: Vec<ObjectId> = payload
            .definition
            .layers
            .layers
            .iter()
            .flat_map(|layer| layer.cells.iter())
            .flat_map(|cell| cell.elements.iter())
            .copied()
            .collect();
        // Worklist walk over the whole closure: every id reachable
        // from the definition — through nested group children — must
        // be a payload member, and the walk order is what gets
        // imported (first-touch, so slot allocation is deterministic
        // across replay).
        let mut order: Vec<ObjectId> = Vec::new();
        let mut seen: BTreeSet<IdKey> = BTreeSet::new();
        let mut queue: std::collections::VecDeque<ObjectId> = references.into();
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id_key(id)) {
                continue;
            }
            let Some(object) = members.get(&id_key(id)) else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "the closure is incomplete: {}:{} is referenced but not in the payload",
                        id.index(),
                        id.generation()
                    ),
                });
            };
            order.push(id);
            if let ObjectData::Group(group) = object {
                for child in &group.children {
                    queue.push_back(*child);
                }
            }
        }

        // Insert the closure, then rewrite references through the map.
        // Every id in `order` (and every child it names) is in the map
        // by the walk above, so a missing entry is unreachable — the
        // fallback keeps the lookup total.
        let before = document.symbols.clone();
        let mut map: BTreeMap<IdKey, ObjectId> = BTreeMap::new();
        for id in &order {
            let object = members[&id_key(*id)];
            let fresh = document.registry.insert(object.clone());
            map.insert(id_key(*id), fresh);
        }
        for id in &order {
            let Some(slot) = document.registry.get_mut(map[&id_key(*id)]) else {
                continue;
            };
            if let ObjectData::Group(group) = slot {
                for child in &mut group.children {
                    if let Some(fresh) = map.get(&id_key(*child)) {
                        *child = *fresh;
                    }
                }
            }
        }
        let mut definition = payload.definition.clone();
        definition.symbol_number = next_symbol_number(document);
        for layer in &mut definition.layers.layers {
            for cell in &mut layer.cells {
                cell.elements = cell
                    .elements
                    .iter()
                    .map(|id| map.get(&id_key(*id)).copied().unwrap_or(*id))
                    .collect::<Vec<_>>();
            }
        }
        document.symbols.push(definition);

        // Captures in mutation order: member creations first, the
        // library capture last — undo restores the list (dropping the
        // imported definition), then removes the members.
        let mut deltas: Vec<Delta> = order
            .iter()
            .map(|id| Delta::Object {
                id: map[&id_key(*id)],
                inverse: ObjectInverse::Created,
            })
            .collect();
        deltas.push(Delta::Library { before });
        Ok(Delta::Group(deltas))
    }
}

/// Splits a transform into the translation, rotation and scale the
/// tween interpolates. Skew-free by construction: a skewed source
/// rounds into the nearest rotation + anisotropic scale (the tween
/// panel exposes exactly these three knobs, so the decomposition is
/// the format's own vocabulary).
fn decompose(m: geom::Affine) -> ([f64; 2], f64, [f64; 2]) {
    let [a, b, c, d, e, f] = m.as_coeffs();
    let rotation = b.atan2(a);
    let scale_x = a.hypot(b);
    // Strip the rotation from the second column; what remains is the
    // y scale (the residue is skew, which the tween does not carry).
    let scale_y = -c * rotation.sin() + d * rotation.cos();
    ([e, f], rotation, [scale_x, scale_y])
}

/// Recomposes [`decompose`]'s parts.
fn recompose(translation: [f64; 2], rotation: f64, scale: [f64; 2]) -> geom::Affine {
    let (sin, cos) = rotation.sin_cos();
    geom::Affine::new([
        scale[0] * cos,
        scale[0] * sin,
        -scale[1] * sin,
        scale[1] * cos,
        translation[0],
        translation[1],
    ])
}

/// `symbol.tween` — CS6 `dom.tween(numSteps, bDistribute)`: "Tweens
/// between the two selected instances", generating `numSteps` new
/// instances on the line between them (the Transform panel's
/// location/rotation/scale knobs interpolated; the rotation takes the
/// shortest arc), optionally distributed across the states following
/// the source frame (`bDistribute`), adding frames as
/// `distributeToFrames` does. Both instances must be of one symbol in
/// one cell of one page — the dialog's own precondition. Per-instance
/// animation parameters are not tweened (copies carry none); recorded
/// as the decision's deviation.
struct Tween;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TweenPayload {
    /// How many new instances sit between the two selected ones.
    num_steps: u32,
    /// Distribute the new instances across the following frames.
    distribute: bool,
    /// Source ("current") frame for the distribution.
    frame: usize,
}

impl TypedCommand for Tween {
    type Payload = TweenPayload;

    fn name(&self) -> &'static str {
        "symbol.tween"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "symbol.tween";
        if payload.num_steps == 0 {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: "numSteps must be at least 1".to_owned(),
            });
        }
        let picked: Vec<(ObjectId, SymbolInstanceObject)> = document
            .object_selection
            .iter()
            .copied()
            .filter_map(|id| match document.registry.get(id) {
                Some(ObjectData::SymbolInstance(instance)) => Some((id, instance.clone())),
                _ => None,
            })
            .collect();
        if picked.len() != 2 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "tween needs exactly two selected instances, got {}",
                    picked.len()
                ),
            });
        }
        let selected: Vec<ObjectId> = picked.iter().map(|(id, _)| *id).collect();
        let instances: Vec<SymbolInstanceObject> =
            picked.into_iter().map(|(_, instance)| instance).collect();
        if instances[0].symbol_number != instances[1].symbol_number {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the two instances are not of the same symbol".to_owned(),
            });
        }
        let cell_of = |id: ObjectId| match find_owner(document, id) {
            Some(Owner::Cell {
                page,
                layer,
                cell,
                position,
            }) => Some((page, layer, cell, position)),
            _ => None,
        };
        let (Some((page, layer, cell, position)), Some((other_page, other_layer, other_cell, _))) =
            (cell_of(selected[0]), cell_of(selected[1]))
        else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "both instances must sit in cells (not nested in groups)".to_owned(),
            });
        };
        if (other_page, other_layer, other_cell) != (page, layer, cell) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the two instances must share one cell".to_owned(),
            });
        }
        // Shortest-arc rotation between the endpoints (round is 0 for
        // a half-turn or less, ±1 past it).
        let (from_t, from_r, from_s) = decompose(instances[0].transform);
        let (to_t, mut to_r, to_s) = decompose(instances[1].transform);
        let turn = (to_r - from_r) / std::f64::consts::TAU;
        to_r -= turn.round() * std::f64::consts::TAU;
        let count = document.pages[page].info.frame_count.max(0) as usize;
        if payload.distribute && payload.frame >= count {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!(
                    "frame {} is beyond the page's {} frames",
                    payload.frame, count
                ),
            });
        }
        let needed = if payload.distribute {
            (payload.frame + payload.num_steps as usize + 1).saturating_sub(count)
        } else {
            0
        };
        let before = Box::new(document.pages[page].clone());
        if needed > 0 {
            super::cell::append_frames(document, page, needed);
        }
        let steps = payload.num_steps as f64;
        let mut minted = Vec::new();
        for step in 1..=payload.num_steps {
            let t = step as f64 / (steps + 1.0);
            let lerp = |from: f64, to: f64| from + (to - from) * t;
            let transform = recompose(
                [lerp(from_t[0], to_t[0]), lerp(from_t[1], to_t[1])],
                lerp(from_r, to_r),
                [lerp(from_s[0], to_s[0]), lerp(from_s[1], to_s[1])],
            );
            let instance =
                document
                    .registry
                    .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
                        symbol_number: instances[0].symbol_number,
                        transform,
                        locked: false,
                        name: None,
                        animation: None,
                        custom_data: instances[0].custom_data.clone(),
                    }));
            minted.push(instance);
            if payload.distribute {
                let target = payload.frame + step as usize;
                access::cell_mut(document, NAME, page, layer, target)?
                    .elements
                    .push(instance);
            } else {
                attach_cell(
                    document,
                    NAME,
                    page,
                    layer,
                    cell,
                    instance,
                    Some(position + step as usize),
                )?;
            }
        }
        let mut deltas = vec![Delta::PageBody { page, before }];
        deltas.extend(minted.into_iter().map(|id| Delta::Object {
            id,
            inverse: ObjectInverse::Created,
        }));
        Ok(Delta::Group(deltas))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::super::testdoc;
    use super::*;
    use crate::command::Command;
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
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
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
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
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
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
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
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
        });
        let instance = document
            .registry
            .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
                symbol_number: 2,
                transform: geom::Affine::IDENTITY,
                locked: false,
                name: None,
                animation: None,
                custom_data: std::collections::BTreeMap::new(),
            }));
        document.pages[0].layers.layers[0].cells[0].elements = vec![instance, ids[1]];
        let original = serde_json::to_vec(&document).unwrap();
        let live_before = document.registry.len();

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
        // The instance dissolves entirely: gone from the registry, not
        // just its cell — an object referenced by nothing would never
        // reach the wire and would fork the model on re-open. The one
        // copy added balances the one instance removed.
        assert!(document.registry.get(instance).is_none());
        assert_eq!(document.registry.len(), live_before);
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
        // Undo revives the instance at its exact slot and generation.
        assert!(document.registry.get(instance).is_some());
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
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
        });
        let instance = document
            .registry
            .insert(ObjectData::SymbolInstance(SymbolInstanceObject {
                symbol_number: 1,
                transform: geom::Affine::IDENTITY,
                locked: false,
                name: None,
                animation: None,
                custom_data: std::collections::BTreeMap::new(),
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
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
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
    fn button_flags_capture_and_undo() {
        let (mut document, _) = testdoc::sample_document();
        document.symbols.push(testdoc::definition(0));
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();

        // Each setter flips its slot (the fixture carries the CS6
        // defaults — auto-slice on, the three state boxes off — so the
        // command writes the opposite value); undo restores the
        // default exactly.
        let defaults = [
            ("symbol.setButtonAutoSlice", false),
            ("symbol.setButtonIncludeDownState", true),
            ("symbol.setButtonShowDownOnLoad", true),
            ("symbol.setButtonIncludeOverWhileDownState", true),
        ];
        for (name, value) in defaults {
            let read = |document: &Document| match name {
                "symbol.setButtonAutoSlice" => document.symbols[0].auto_slice,
                "symbol.setButtonIncludeDownState" => document.symbols[0].include_down_state,
                "symbol.setButtonShowDownOnLoad" => document.symbols[0].show_down_on_load,
                _ => document.symbols[0].include_over_while_down,
            };
            let delta = registry
                .execute(&mut document, name, json!({ "symbol": 0, "value": value }))
                .unwrap();
            assert_eq!(read(&document), Some(value), "{name}");
            delta.apply_inverse(&mut document).unwrap();
            assert_eq!(read(&document), Some(!value), "{name} undo");
        }
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);

        // Unknown definitions refuse whole-command.
        let error = registry
            .execute(
                &mut document,
                "symbol.setButtonAutoSlice",
                json!({ "symbol": 7, "value": true }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
    }

    #[test]
    fn set_button_options_updates_linked_slices_and_undoes() {
        let (mut document, _) = testdoc::sample_document();
        document.symbols.push(testdoc::definition(3));
        let owned_a = document
            .registry
            .insert(ObjectData::Slice(doc::SliceObject {
                internal_name: None,
                frame: geom::Rect::new(0.0, 0.0, 8.0, 8.0),
                shape: 0,
                fill_raw: 0,
                name: None,
                slice_type: Some(0),
                link: None,
                alt: None,
                label: None,
                td_text: None,
                symbol_number: Some(3),
                contours: Vec::new(),
                background: None,
                behaviors: Vec::new(),
                custom_data: std::collections::BTreeMap::new(),
                export: None,
            }));
        let owned_b = document
            .registry
            .insert(ObjectData::Slice(doc::SliceObject {
                internal_name: None,
                frame: geom::Rect::new(8.0, 0.0, 16.0, 8.0),
                shape: 0,
                fill_raw: 0,
                name: None,
                slice_type: Some(0),
                link: None,
                alt: None,
                label: None,
                td_text: None,
                symbol_number: Some(3),
                contours: Vec::new(),
                background: None,
                behaviors: Vec::new(),
                custom_data: std::collections::BTreeMap::new(),
                export: None,
            }));
        let unlinked = document
            .registry
            .insert(ObjectData::Slice(doc::SliceObject {
                internal_name: None,
                frame: geom::Rect::new(0.0, 8.0, 8.0, 16.0),
                shape: 0,
                fill_raw: 0,
                name: None,
                slice_type: Some(0),
                link: None,
                alt: None,
                label: None,
                td_text: None,
                symbol_number: None,
                contours: Vec::new(),
                background: None,
                behaviors: Vec::new(),
                custom_data: std::collections::BTreeMap::new(),
                export: None,
            }));
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();

        // Every present field lands on both linked slices; the
        // unlinked slice and absent fields stay untouched.
        let delta = registry
            .execute(
                &mut document,
                "symbol.setButtonOptions",
                json!({
                    "symbol": 3,
                    "link": "https://firecraft.example/",
                    "alt": "fuzz button",
                    "slice_name": "btn_home",
                    "export": { "format": 2 },
                }),
            )
            .unwrap();
        for id in [owned_a, owned_b] {
            let Some(ObjectData::Slice(slice)) = document.registry.get(id) else {
                panic!("slice {id:?} vanished");
            };
            assert_eq!(slice.link.as_deref(), Some("https://firecraft.example/"));
            assert_eq!(slice.alt.as_deref(), Some("fuzz button"));
            assert_eq!(slice.name.as_deref(), Some("btn_home"));
            assert_eq!(slice.export.as_ref().and_then(|e| e.format), Some(2));
        }
        let Some(ObjectData::Slice(slice)) = document.registry.get(unlinked) else {
            panic!("unlinked slice vanished");
        };
        assert!(slice.link.is_none() && slice.alt.is_none());
        assert!(slice.name.is_none() && slice.export.is_none());
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);

        // A definition with no linked slice refuses (CS6's auto-slice
        // invariant means a real button always has one). Definition 4
        // exists to have no slices pointing at it.
        document.symbols.push(testdoc::definition(4));
        let extended = serde_json::to_vec(&document).unwrap();
        let error = registry
            .execute(
                &mut document,
                "symbol.setButtonOptions",
                json!({ "symbol": 4, "link": "https://firecraft.example/" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), extended);
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
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
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
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
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
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
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
                custom_data: std::collections::BTreeMap::new(),
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

    #[test]
    fn script_param_and_custom_data_undo_exactly() {
        let (mut document, _) = testdoc::sample_document();
        document.symbols.push(SymbolDefinition {
            name: "rich".to_owned(),
            symbol_number: 0,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList::default(),
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
        });
        let registry = registry();

        // setScript installs, then clears; each inverts exactly.
        let empty = serde_json::to_vec(&document).unwrap();
        let delta = registry
            .execute(
                &mut document,
                "symbol.setScript",
                json!({ "symbol": 0, "source": "// rich" }),
            )
            .unwrap();
        assert_eq!(document.symbols[0].op_code.as_deref(), Some("// rich"));
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.symbols[0].op_code.is_none());
        assert_eq!(serde_json::to_vec(&document).unwrap(), empty);

        // The data commands need an instance to carry them.
        registry
            .execute(
                &mut document,
                "symbol.createInstance",
                json!({
                    "symbol": 0, "page": 0, "layer": 0, "cell": 0,
                    "index": null, "x": 5.0, "y": 6.0, "name": null,
                }),
            )
            .unwrap();
        let instance = document
            .registry
            .iter()
            .find(|(_, object)| matches!(object, ObjectData::SymbolInstance(_)))
            .map(|(id, _)| id)
            .unwrap();
        let before = serde_json::to_vec(&document).unwrap();

        let delta = registry
            .execute(
                &mut document,
                "symbol.setParam",
                json!({
                    "id": instance, "name": "Color",
                    "value": "#009900", "kind": "color",
                }),
            )
            .unwrap();
        let read_current = |document: &Document| -> Vec<serde_json::Value> {
            match document.registry.get(instance) {
                Some(ObjectData::SymbolInstance(instance)) => serde_json::from_str(
                    instance
                        .custom_data
                        .get("currentValues")
                        .map(String::as_str)
                        .unwrap_or("[]"),
                )
                .unwrap(),
                _ => Vec::new(),
            }
        };
        assert_eq!(read_current(&document).len(), 1);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);

        // Create-or-update: a second entry lands, then the same name
        // replaces in place.
        for name in ["Label", "Color"] {
            registry
                .execute(
                    &mut document,
                    "symbol.setParam",
                    json!({
                        "id": instance, "name": name,
                        "value": format!("v-{name}"), "kind": "text",
                    }),
                )
                .unwrap();
        }
        let current = read_current(&document);
        assert_eq!(current.len(), 2);
        assert_eq!(current[1]["value"], "v-Color");

        // Raw custom data: set, then the inverse drops the key.
        let delta = registry
            .execute(
                &mut document,
                "symbol.setCustomData",
                json!({ "id": instance, "key": "shapeName", "value": "roundRect" }),
            )
            .unwrap();
        let custom = |document: &Document| match document.registry.get(instance) {
            Some(ObjectData::SymbolInstance(instance)) => instance.clone(),
            _ => panic!("instance vanished"),
        };
        assert_eq!(
            custom(&document)
                .custom_data
                .get("shapeName")
                .map(String::as_str),
            Some("roundRect")
        );
        delta.apply_inverse(&mut document).unwrap();
        assert!(!custom(&document).custom_data.contains_key("shapeName"));

        // A non-instance target is a typed refusal that mutates nothing.
        let path = document
            .registry
            .iter()
            .find(|(_, object)| matches!(object, ObjectData::Path(_)))
            .map(|(id, _)| id)
            .unwrap();
        let refused = serde_json::to_vec(&document).unwrap();
        let error = registry
            .execute(
                &mut document,
                "symbol.setParam",
                json!({ "id": path, "name": "x", "value": "y", "kind": "text" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), refused);
    }

    // --- library.importSymbol -------------------------------------------

    use doc::{Contour, GroupObject, PathObject, PathPoint};

    /// One named rectangle path.
    fn path_named(name: &str) -> ObjectData {
        let point = |x: f64, y: f64| PathPoint {
            anchor: geom::Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        };
        ObjectData::Path(PathObject {
            contours: vec![Contour {
                closed: true,
                point_count: 4,
                points: vec![
                    point(0.0, 0.0),
                    point(8.0, 0.0),
                    point(8.0, 8.0),
                    point(0.0, 8.0),
                ],
            }],
            primitive: None,
            even_odd: false,
            paint: None,
            fill_start: None,
            fill_end: None,
            fill_third: None,
            brush_placement: None,
            fill_on_top: None,
            locked: false,
            visible: true,
            opacity: 1000,
            blend: 0,
            name: Some(name.to_owned()),
            effects: Vec::new(),
        })
    }

    fn group_named(name: Option<&str>, children: Vec<ObjectId>) -> ObjectData {
        ObjectData::Group(GroupObject {
            group_type: 0,
            name: name.map(str::to_owned),
            children,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            mask_hidden: false,
            mask_linked: None,
            effects: Vec::new(),
        })
    }

    /// A library file: one graphic definition "btn" whose closure is a
    /// nested group (outer > inner > path), nothing on any page.
    fn library_file() -> Document {
        let mut source = Document::default();
        let leaf = source.registry.insert(path_named("dot"));
        let inner = source
            .registry
            .insert(group_named(Some("inner"), vec![leaf]));
        let outer = source.registry.insert(group_named(None, vec![inner]));
        source.symbols.push(SymbolDefinition {
            name: "btn".to_owned(),
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
                        elements: vec![outer],
                    }],
                    ..Layer::default()
                }],
            },
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
        });
        doc::library::extract(&source, &["btn"]).unwrap()
    }

    /// The import payload as it would cross the file boundary: the
    /// extracted definition plus every registry entry as a member.
    fn import_payload(library: &Document) -> serde_json::Value {
        serde_json::to_value(LibraryImportPayload {
            definition: library.symbols[0].clone(),
            members: library
                .registry
                .iter()
                .map(|(id, object)| ImportedMember {
                    id,
                    object: object.clone(),
                })
                .collect(),
        })
        .unwrap()
    }

    #[test]
    fn import_installs_the_closure_and_renumbers() {
        let mut payload = import_payload(&library_file());
        // An orphan member — referenced by nothing — rides along, the
        // way a multi-definition library file's registry does. The
        // import must not drag it in as an unreferenced slot.
        payload["members"].as_array_mut().unwrap().push(json!({
            "id": { "index": 4242, "generation": 7 },
            "object": {
                "Path": {
                    "contours": [],
                    "primitive": null,
                    "even_odd": false,
                    "paint": null,
                    "fill_start": null,
                    "fill_end": null,
                    "fill_third": null,
                    "brush_placement": null,
                    "fill_on_top": null,
                    "locked": false,
                    "visible": true,
                    "opacity": 1000,
                    "blend": 0,
                    "name": null,
                    "effects": [],
                },
            },
        }));
        let source_ids: Vec<(usize, u32)> = payload["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|member| {
                (
                    member["id"]["index"].as_u64().unwrap() as usize,
                    member["id"]["generation"].as_u64().unwrap() as u32,
                )
            })
            .collect();

        let (mut document, _) = testdoc::sample_document();
        let registry = registry();
        // An existing definition so the import renumbers past it.
        registry
            .execute(
                &mut document,
                "library.add",
                json!({ "name": "existing", "symbol_type": "graphic" }),
            )
            .unwrap();
        let live_before_import = document.registry.iter().count();
        let delta = registry
            .execute(&mut document, "library.importSymbol", payload)
            .unwrap();

        // The imported definition is renumbered to the next free slot.
        assert_eq!(document.symbols.len(), 2);
        assert_eq!(document.symbols[1].name, "btn");
        assert_eq!(document.symbols[1].symbol_number, 1);

        // The closure is live and remapped: outer > inner > "dot",
        // every id fresh (none of the payload's source ids survived).
        let root = document.symbols[1].layers.layers[0].cells[0].elements[0];
        let fresh_ids = {
            let mut ids = vec![(root.index(), root.generation())];
            let mut child = root;
            for _ in 0..2 {
                match document.registry.get(child) {
                    Some(ObjectData::Group(group)) => {
                        child = group.children[0];
                        ids.push((child.index(), child.generation()));
                    }
                    Some(ObjectData::Path(path)) => {
                        assert_eq!(path.name.as_deref(), Some("dot"));
                    }
                    other => panic!("unexpected member shape: {other:?}"),
                }
            }
            ids
        };
        assert_eq!(
            document.registry.iter().count(),
            live_before_import + fresh_ids.len(),
            "the pre-import registry stays put; three members arrive"
        );
        for source in &source_ids {
            assert!(
                !fresh_ids.contains(source),
                "source id {source:?} must not survive the re-slot"
            );
        }

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.symbols.len(), 1);
    }

    #[test]
    fn import_undoes_to_the_pre_import_bytes() {
        let (mut document, _) = testdoc::sample_document();
        let before = serde_json::to_vec(&document).unwrap();
        let live = document.registry.iter().count();

        let delta = registry()
            .execute(
                &mut document,
                "library.importSymbol",
                import_payload(&library_file()),
            )
            .unwrap();
        assert_eq!(document.symbols.len(), 1);
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.symbols.is_empty());
        assert_eq!(
            document.registry.iter().count(),
            live,
            "the member slots are gone again"
        );
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn import_refuses_incomplete_closures_without_mutation() {
        let (mut document, _) = testdoc::sample_document();
        let before = serde_json::to_vec(&document).unwrap();
        let registry = registry();

        // Drop the leaf path: the inner group then references an id
        // outside the payload. (`ObjectData` is externally tagged, so
        // the path member sits under the `"Path"` key.)
        let mut partial = import_payload(&library_file());
        partial["members"]
            .as_array_mut()
            .unwrap()
            .retain(|member| member["object"].get("Path").is_none());
        let error = registry
            .execute(&mut document, "library.importSymbol", partial)
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);

        // A duplicated member id is the same refusal.
        let mut duplicated = import_payload(&library_file());
        let members = duplicated["members"].as_array_mut().unwrap();
        let again = members[0].clone();
        members.push(again);
        let error = registry
            .execute(&mut document, "library.importSymbol", duplicated)
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
        assert!(document.symbols.is_empty());
    }

    /// Two instances of one symbol in page 0's anchor cell: `first` at
    /// the origin, `second` at (30, 60). Returns their ids.
    fn tween_endpoints() -> (Document, ObjectId, ObjectId) {
        let (mut document, ids) = testdoc::sample_document();
        let registry = registry();
        registry
            .execute(
                &mut document,
                "symbol.create",
                json!({
                    "ids": [ids[0], ids[1]],
                    "name": "dot",
                    "symbol_type": "graphic",
                }),
            )
            .unwrap();
        let (first, _) = instance_of(&document);
        registry
            .execute(
                &mut document,
                "symbol.createInstance",
                json!({
                    "symbol": 0, "page": 0, "layer": 0, "cell": 0,
                    "x": 30.0, "y": 60.0,
                }),
            )
            .unwrap();
        let elements = cell_elements(&document).to_vec();
        let second = *elements.last().unwrap();
        if let Some(ObjectData::SymbolInstance(instance)) = document.registry.get_mut(second) {
            instance.transform = geom::Affine::translate((30.0, 60.0));
        }
        document.object_selection = vec![first, second];
        (document, first, second)
    }

    fn transform_coeffs(document: &Document, id: ObjectId) -> [f64; 6] {
        match document.registry.get(id).unwrap() {
            ObjectData::SymbolInstance(instance) => instance.transform.as_coeffs(),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn tween_inserts_lerped_instances_between_the_endpoints() {
        let (mut document, first, second) = tween_endpoints();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        let delta = registry
            .execute(
                &mut document,
                "symbol.tween",
                json!({ "numSteps": 2, "distribute": false, "frame": 0 }),
            )
            .unwrap();

        // The new instances sit between the endpoints: attached after
        // the first endpoint's position, so the order reads first,
        // step 1, step 2, second.
        let elements = cell_elements(&document).to_vec();
        assert_eq!(elements.len(), 4);
        assert_eq!(elements[0], first);
        assert_eq!(elements[3], second);
        let t1 = transform_coeffs(&document, elements[1]);
        let t2 = transform_coeffs(&document, elements[2]);
        assert!(near(t1[4], 10.0) && near(t1[5], 20.0), "{t1:?}");
        assert!(near(t2[4], 20.0) && near(t2[5], 40.0), "{t2:?}");
        for coeffs in [t1, t2] {
            assert!(near(coeffs[0], 1.0) && near(coeffs[3], 1.0), "{coeffs:?}");
            assert!(near(coeffs[1], 0.0) && near(coeffs[2], 0.0), "{coeffs:?}");
        }
        // Copies are fresh instances: unnamed, unlocked, no animation,
        // carrying the source's custom data.
        for id in [elements[1], elements[2]] {
            match document.registry.get(id).unwrap() {
                ObjectData::SymbolInstance(instance) => {
                    assert_eq!(instance.symbol_number, 0);
                    assert_eq!(instance.name, None);
                    assert!(!instance.locked);
                    assert!(instance.animation.is_none());
                }
                other => panic!("wrong object kind: {other:?}"),
            }
        }

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            original,
            "undo removes the minted instances"
        );
    }

    #[test]
    fn tween_rotation_takes_the_shortest_arc() {
        let (mut document, _, second) = tween_endpoints();
        // Second endpoint at 270°: the short way round is -90°, so the
        // midpoint leans -45° — not +135°.
        if let Some(ObjectData::SymbolInstance(instance)) = document.registry.get_mut(second) {
            instance.transform = geom::Affine::translate((30.0, 60.0))
                * geom::Affine::rotate(3.0 * std::f64::consts::FRAC_PI_2);
        }
        let registry = registry();
        registry
            .execute(
                &mut document,
                "symbol.tween",
                json!({ "numSteps": 1, "distribute": false, "frame": 0 }),
            )
            .unwrap();
        let elements = cell_elements(&document).to_vec();
        let mid = transform_coeffs(&document, elements[1]);
        assert!(
            near(mid[1], -std::f64::consts::FRAC_1_SQRT_2),
            "midpoint must lean the short way: {mid:?}"
        );
    }

    #[test]
    fn tween_distribute_grows_the_frames_and_inverts() {
        let (mut document, first, second) = tween_endpoints();
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        let delta = registry
            .execute(
                &mut document,
                "symbol.tween",
                json!({ "numSteps": 2, "distribute": true, "frame": 0 }),
            )
            .unwrap();

        // Frame 0 keeps the endpoints; steps 1..=2 land on frames 1 and
        // 2, and the page grew one frame to hold the last one.
        assert_eq!(document.pages[0].info.frame_count, 3);
        let cells = &document.pages[0].layers.layers[0].cells;
        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].elements, vec![first, second]);
        assert_eq!(cells[1].elements.len(), 1);
        assert_eq!(cells[2].elements.len(), 1);
        let t1 = transform_coeffs(&document, cells[1].elements[0]);
        let t2 = transform_coeffs(&document, cells[2].elements[0]);
        assert!(near(t1[4], 10.0) && near(t1[5], 20.0), "{t1:?}");
        assert!(near(t2[4], 20.0) && near(t2[5], 40.0), "{t2:?}");

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            original,
            "undo restores the pre-tween frames"
        );
    }

    #[test]
    fn tween_refusals_leave_the_document_alone() {
        let (mut document, first, _) = tween_endpoints();
        let registry = registry();
        let original = serde_json::to_vec(&document).unwrap();

        // Zero steps is a payload error.
        let error = registry
            .execute(
                &mut document,
                "symbol.tween",
                json!({ "numSteps": 0, "distribute": false, "frame": 0 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }));

        // Not exactly two selected instances.
        document.object_selection = vec![first];
        let one_selected = serde_json::to_vec(&document).unwrap();
        let error = registry
            .execute(
                &mut document,
                "symbol.tween",
                json!({ "numSteps": 1, "distribute": false, "frame": 0 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), one_selected);
        let _ = original;

        // Mixed symbols refuse: a second definition, one instance each.
        document.symbols.push(SymbolDefinition {
            name: "other".to_owned(),
            symbol_number: 5,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList::default(),
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
        });
        registry
            .execute(
                &mut document,
                "symbol.createInstance",
                json!({
                    "symbol": 5, "page": 0, "layer": 0, "cell": 0,
                    "x": -4.0, "y": -4.0,
                }),
            )
            .unwrap();
        let stranger = *cell_elements(&document).last().unwrap();
        document.object_selection = vec![first, stranger];
        let before = serde_json::to_vec(&document).unwrap();
        let error = registry
            .execute(
                &mut document,
                "symbol.tween",
                json!({ "numSteps": 1, "distribute": false, "frame": 0 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);

        // Different cells refuse too: move the stranger's twin of the
        // first instance into frame 1 and try again.
        registry
            .execute(
                &mut document,
                "symbol.createInstance",
                json!({
                    "symbol": 0, "page": 0, "layer": 0, "cell": 1,
                    "x": 6.0, "y": 6.0,
                }),
            )
            .unwrap();
        let twin = document.pages[0].layers.layers[0].cells[1].elements[0];
        document.object_selection = vec![first, twin];
        let before = serde_json::to_vec(&document).unwrap();
        let error = registry
            .execute(
                &mut document,
                "symbol.tween",
                json!({ "numSteps": 1, "distribute": false, "frame": 0 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    /// Places a plain instance of a scratch definition and selects it.
    fn placed_instance(document: &mut Document) -> ObjectId {
        document.symbols.push(SymbolDefinition {
            name: "dot".to_owned(),
            symbol_number: 7,
            origin: geom::Point::ZERO,
            frame_count: 1,
            layers: LayerList::default(),
            background_raw: 0,
            symbol_type: SymbolType::Graphic,
            nine_slice: None,
            auto_slice: Some(true),
            include_down_state: Some(false),
            show_down_on_load: Some(false),
            include_over_while_down: Some(false),
            op_code: None,
        });
        let registry = registry();
        registry
            .execute(
                document,
                "symbol.createInstance",
                json!({
                    "symbol": 7, "page": 0, "layer": 1, "cell": 1,
                    "index": 0, "x": 0.0, "y": 0.0,
                }),
            )
            .unwrap();
        let (instance, _) = instance_of(document);
        document.object_selection = vec![instance];
        instance
    }

    /// A two-definition library: button (7) and graphic (8), each
    /// holding one text item in its single frame. Returns the text ids.
    fn button_library(document: &mut Document) -> (ObjectId, ObjectId) {
        let make_definition = |number: i64, kind: SymbolType, text_id: ObjectId| {
            let mut layer = Layer {
                name: "content".to_owned(),
                display: true,
                ..Layer::default()
            };
            layer.cells.push(Cell {
                locked: false,
                visible: true,
                elements: vec![text_id],
            });
            let mut layers = LayerList::default();
            layers.layers.push(layer);
            SymbolDefinition {
                name: format!("def {number}"),
                symbol_number: number,
                origin: geom::Point::ZERO,
                frame_count: 1,
                layers,
                background_raw: 0,
                symbol_type: kind,
                nine_slice: None,
                auto_slice: None,
                include_down_state: None,
                show_down_on_load: None,
                include_over_while_down: None,
                op_code: None,
            }
        };
        let button_text = document
            .registry
            .insert(ObjectData::Text(doc::TextObject::new(geom::Rect::new(
                0.0, 0.0, 40.0, 12.0,
            ))));
        let graphic_text = document
            .registry
            .insert(ObjectData::Text(doc::TextObject::new(geom::Rect::new(
                0.0, 0.0, 40.0, 12.0,
            ))));
        let button = make_definition(7, SymbolType::Button, button_text);
        let graphic = make_definition(8, SymbolType::Graphic, graphic_text);
        document.symbols.push(button);
        document.symbols.push(graphic);
        (button_text, graphic_text)
    }

    fn runs_of(document: &Document, id: ObjectId) -> Vec<String> {
        match document.registry.get(id).unwrap() {
            ObjectData::Text(text) => text.runs.iter().map(|r| r.text.clone()).collect(),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn replace_button_text_targets_button_definitions_only() {
        let (mut document, _) = testdoc::sample_document();
        let (button_text, graphic_text) = button_library(&mut document);
        let before = serde_json::to_vec(&document).unwrap();

        let delta = ReplaceButtonTextStrings
            .apply_json(
                &mut document,
                json!({"text": "Buy now", "uniform_attrs": true}),
            )
            .expect("applies");
        assert_eq!(runs_of(&document, button_text), vec!["Buy now"]);
        assert_eq!(runs_of(&document, graphic_text), Vec::<String>::new());
        delta.apply_inverse(&mut document).expect("inverse applies");
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);

        // The no-buttons library is the no-op.
        document.symbols.clear();
        let delta = ReplaceButtonTextStrings
            .apply_json(&mut document, json!({"text": "x"}))
            .expect("applies");
        assert!(matches!(delta, crate::delta::Delta::Noop));
    }

    #[test]
    fn replace_button_text_in_instances_scopes_to_selection() {
        let (mut document, _) = testdoc::sample_document();
        let (button_text, _) = button_library(&mut document);
        // Instance of the graphic definition (8).
        let registry = registry();
        registry
            .execute(
                &mut document,
                "symbol.createInstance",
                json!({
                    "symbol": 8, "page": 0, "layer": 1, "cell": 1,
                    "index": 0, "x": 0.0, "y": 0.0,
                }),
            )
            .unwrap();
        let (graphic_instance, _) = instance_of(&document);
        document.object_selection = vec![graphic_instance];
        let delta = ReplaceButtonTextStringsInInstances
            .apply_json(&mut document, json!({"text": "x"}))
            .expect("applies");
        assert!(matches!(delta, crate::delta::Delta::Noop));

        // Selecting the button's text instead — the definition edit
        // flows through the instance lookup only for instances; a text
        // selection is the no-instance no-op too.
        document.object_selection = vec![button_text];
        let delta = ReplaceButtonTextStringsInInstances
            .apply_json(&mut document, json!({"text": "x"}))
            .expect("applies");
        assert!(matches!(delta, crate::delta::Delta::Noop));

        // An instance of the button definition is the real target.
        registry
            .execute(
                &mut document,
                "symbol.createInstance",
                json!({
                    "symbol": 7, "page": 0, "layer": 1, "cell": 1,
                    "index": 0, "x": 4.0, "y": 0.0,
                }),
            )
            .unwrap();
        let button_instance = document
            .registry
            .iter()
            .find_map(|(id, object)| match object {
                ObjectData::SymbolInstance(instance) if instance.symbol_number == 7 => Some(id),
                _ => None,
            })
            .unwrap();
        document.object_selection = vec![button_instance];
        ReplaceButtonTextStringsInInstances
            .apply_json(&mut document, json!({"text": "Click"}))
            .expect("applies");
        assert_eq!(runs_of(&document, button_text), vec!["Click"]);

        // Empty selection refuses.
        document.object_selection = Vec::new();
        let err = ReplaceButtonTextStringsInInstances
            .apply_json(&mut document, json!({"text": "x"}))
            .expect_err("empty selection is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::InvalidState { .. }
        ));
    }

    fn animation_of(document: &Document, id: ObjectId) -> &SymbolAnimation {
        match document.registry.get(id).unwrap() {
            ObjectData::SymbolInstance(instance) => instance.animation.as_ref().unwrap(),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn anim_params_write_every_field_and_undo() {
        let (mut document, _) = testdoc::sample_document();
        let instance = placed_instance(&mut document);
        // The plain instance carries no animation block until the
        // first anim verb materializes it.
        assert!(matches!(
            document.registry.get(instance).unwrap(),
            ObjectData::SymbolInstance(SymbolInstanceObject {
                animation: None,
                ..
            })
        ));
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        let mut deltas = Vec::new();
        for (name, payload) in [
            ("symbol.setAnimInstanceNumFrames", json!({ "frames": 12 })),
            (
                "symbol.setAnimInstanceOffsetDist",
                json!({ "offset": {"x": 100.0, "y": 25.0} }),
            ),
            (
                "symbol.setAnimInstanceRotationAmount",
                json!({ "rotation": 720.5 }),
            ),
            (
                "symbol.setAnimInstanceScaleAmount",
                json!({ "scale": 200.0 }),
            ),
            (
                "symbol.setAnimInstanceStartEndOpacity",
                json!({ "start": 40.0, "end": 12.34 }),
            ),
            // The FW3 spelling lands on the same ANNi anchor as the
            // FW4 verb it was deprecated in favor of.
            (
                "symbol.setAnimInstanceLoopCount",
                json!({ "loop_count": 6 }),
            ),
        ] {
            deltas.push(registry.execute(&mut document, name, payload).unwrap());
        }
        let animation = animation_of(&document, instance);
        assert_eq!(animation.frames, 6);
        assert_eq!(animation.x_shift, 100.0);
        assert_eq!(animation.y_shift, 25.0);
        // Fractional degrees round onto the integral ANRi field.
        assert_eq!(animation.rotation, Some(721));
        assert_eq!(animation.scaling, 200.0);
        // Opacities are per-mille in the model.
        assert_eq!(animation.start_opacity, Some(400));
        assert_eq!(animation.end_opacity, Some(123));

        for delta in deltas.into_iter().rev() {
            delta.apply_inverse(&mut document).unwrap();
        }
        assert_eq!(serde_json::to_vec(&document).unwrap(), original);
        assert!(matches!(
            document.registry.get(instance).unwrap(),
            ObjectData::SymbolInstance(SymbolInstanceObject {
                animation: None,
                ..
            })
        ));
    }

    #[test]
    fn anim_params_refuse_garbage_and_touch_nothing() {
        let (mut document, _) = testdoc::sample_document();
        let _ = placed_instance(&mut document);
        let original = serde_json::to_vec(&document).unwrap();
        let registry = registry();
        for (name, payload) in [
            ("symbol.setAnimInstanceNumFrames", json!({ "frames": 0 })),
            ("symbol.setAnimInstanceNumFrames", json!({ "frames": -3 })),
            (
                "symbol.setAnimInstanceLoopCount",
                json!({ "loop_count": 0 }),
            ),
            (
                "symbol.setAnimInstanceOffsetDist",
                json!({ "offset": {"x": 1.0, "y": "x"} }),
            ),
            (
                "symbol.setAnimInstanceRotationAmount",
                json!({ "rotation": "spin" }),
            ),
            ("symbol.setAnimInstanceScaleAmount", json!({ "scale": 0.0 })),
            (
                "symbol.setAnimInstanceScaleAmount",
                json!({ "scale": -5.0 }),
            ),
            (
                "symbol.setAnimInstanceStartEndOpacity",
                json!({ "start": 101.0, "end": 0.0 }),
            ),
            (
                "symbol.setAnimInstanceStartEndOpacity",
                json!({ "start": 0.0, "end": -1.0 }),
            ),
            (
                "symbol.setAnimInstanceNumFrames",
                json!({ "loop_count": 5 }),
            ),
        ] {
            let error = registry.execute(&mut document, name, payload).unwrap_err();
            assert!(
                matches!(error, EngineError::InvalidState { .. })
                    || matches!(error, EngineError::BadPayload { .. }),
                "{name}: unexpected error shape {error:?}"
            );
            assert_eq!(
                serde_json::to_vec(&document).unwrap(),
                original,
                "{name} mutated on refusal"
            );
        }
    }

    #[test]
    fn anim_params_skip_non_instances_and_refuse_empty_selection() {
        let (mut document, ids) = testdoc::sample_document();
        let instance = placed_instance(&mut document);
        let registry = registry();
        // A path in the selection rides along; only the instance
        // mutates. A selection with no instances at all is a Noop.
        document.object_selection = vec![ids[0], instance];
        let delta = registry
            .execute(
                &mut document,
                "symbol.setAnimInstanceScaleAmount",
                json!({ "scale": 50.0 }),
            )
            .unwrap();
        assert_eq!(animation_of(&document, instance).scaling, 50.0);
        delta.apply_inverse(&mut document).unwrap();
        assert!(matches!(
            document.registry.get(instance).unwrap(),
            ObjectData::SymbolInstance(SymbolInstanceObject {
                animation: None,
                ..
            })
        ));

        document.object_selection = vec![ids[0]];
        let delta = registry
            .execute(
                &mut document,
                "symbol.setAnimInstanceScaleAmount",
                json!({ "scale": 50.0 }),
            )
            .unwrap();
        assert!(matches!(delta, Delta::Noop));

        document.object_selection.clear();
        let error = registry
            .execute(
                &mut document,
                "symbol.setAnimInstanceScaleAmount",
                json!({ "scale": 50.0 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }
}
