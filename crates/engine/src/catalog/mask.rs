//! Element (object) masks + the pixel/JPEG selection-mask rows (M7
//! gap close, decision 0020).
//!
//! Element masks are mask-structured groups: a `GRPv` with `GRTi` 1
//! (grayscale) or 2 (shape) whose `ELMv` is `[mask, content]` — the
//! dictionary's own encoding, already round-tripped by the model. The
//! commands here resolve their target through that structure: the
//! group itself, or the object sitting in its content slot. Pixel
//! masks (`bitmap.movePixelMaskBy`, `bitmap.setPixelMask`) stay
//! selection planes (`Document.selection`), and the "Selective JPEG
//! mask" is the second plane slot (`Document.jpeg_mask`) — editing
//! state both, never file content. Minting commands (add / remove /
//! setMode / converters) push one `Created` inverse per minted slot
//! in mint order and one `Bank` inverse per minted tile id: the LIFO
//! retraction rule the frame family documented.

use doc::{
    Contour, Document, GroupObject, ImageObject, ObjectData, ObjectId, PathObject, PathPoint,
    SelectionMask, TileRef,
};
use geom::{Point, Vec2};

use super::object::{Owner, attach_cell, content_bounds, detach, find_owner, translate_object};
use crate::access;
use crate::bitmap::{check_editable, gather, scatter, touched_tiles};
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::mask::{SelectMode, feather as feather_mask, fold, stamp_polygon};
use crate::registry::CommandRegistry;

/// `GRTi` — regular group. A disabled mask keeps its structure under
/// this type (decision 0020's flagged design point, corpus-gated).
pub(crate) const REGULAR_GROUP: i64 = 0;
/// `GRTi` — grayscale (bitmap) mask.
pub(crate) const GRAYSCALE_MASK: i64 = 1;
/// `GRTi` — shape (vector) mask.
pub(crate) const SHAPE_MASK: i64 = 2;

/// Registers the mask commands (the pixel-mask plane rows ride with
/// the family; `bitmap.select*` remains the select family's home).
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(AddMask)?;
    registry.register(EnableMask)?;
    registry.register(LinkMask)?;
    registry.register(MoveMaskBy)?;
    registry.register(MoveMaskContentsBy)?;
    registry.register(RemoveMask)?;
    registry.register(SetMaskMode)?;
    registry.register(SetMaskShowAttrs)?;
    registry.register(PathToMarquee)?;
    registry.register(MarqueeToPath)?;
    registry.register(SaveJpegMask)?;
    registry.register(RestoreJpegMask)?;
    registry.register(ClearJpegMask)?;
    registry.register(MovePixelMaskBy)?;
    registry.register(SetPixelMask)
}

// --- shared helpers -----------------------------------------------------

/// A resolved mask group: its registry id and a clone of its data.
pub(crate) struct MaskGroup {
    pub(crate) id: ObjectId,
    pub(crate) group: GroupObject,
}

/// The `GRTi` a mask child's kind dictates: images rasterize as
/// grayscale masks, paths as shape masks; anything else cannot be a
/// mask in this model (CS6 offers bitmap and vector masks only).
fn group_type_of_mask(mask: &ObjectData) -> Option<i64> {
    match mask {
        ObjectData::Image(_) => Some(GRAYSCALE_MASK),
        ObjectData::Path(_) => Some(SHAPE_MASK),
        _ => None,
    }
}

/// Resolves the enabled mask group that owns `id`: the group itself
/// when `id` is a live `GRTi` 1/2 group, else the group holding `id`
/// in its content slot (`ELMv[1]`). Errors when no enabled mask group
/// matches — CS6 throws the same way.
pub(crate) fn owning_mask_group(
    document: &Document,
    name: &'static str,
    id: ObjectId,
) -> Result<MaskGroup, EngineError> {
    owning_mask_group_deep(document, name, id, false)
}

/// As `owning_mask_group`, but optionally also accepting a disabled
/// mask (a `GRTi` 0 group whose first child is mask-kind — the
/// structure `mask.enable` restores). Ambiguous with a plain
/// two-object group, which is why only `mask.enable`/`mask.link`/
/// `mask.setShowAttrs` ask for it.
fn owning_mask_group_deep(
    document: &Document,
    name: &'static str,
    id: ObjectId,
    accept_disabled: bool,
) -> Result<MaskGroup, EngineError> {
    let is_mask_group = |group_type: i64| {
        group_type == GRAYSCALE_MASK
            || group_type == SHAPE_MASK
            || (accept_disabled && group_type == REGULAR_GROUP)
    };
    if let ObjectData::Group(group) = access::object(document, name, id)? {
        if is_mask_group(group.group_type) {
            return Ok(MaskGroup {
                id,
                group: group.clone(),
            });
        }
    }
    if let Some(Owner::Group {
        id: parent,
        position,
    }) = find_owner(document, id)
    {
        if position == 1 {
            if let Some(ObjectData::Group(group)) = document.registry.get(parent) {
                if is_mask_group(group.group_type) {
                    return Ok(MaskGroup {
                        id: parent,
                        group: group.clone(),
                    });
                }
            }
        }
    }
    Err(EngineError::InvalidState {
        name: name.to_owned(),
        detail: format!(
            "object {} (generation {}) carries no element mask",
            id.index(),
            id.generation()
        ),
    })
}

/// Union content bounds of one object (identity transform).
pub(crate) fn bounds_of(
    document: &Document,
    name: &'static str,
    id: ObjectId,
) -> Result<geom::Rect, EngineError> {
    let mut out = None;
    let mut seen = std::collections::HashSet::new();
    content_bounds(document, id, geom::Affine::IDENTITY, 0, &mut seen, &mut out);
    out.ok_or_else(|| EngineError::InvalidState {
        name: name.to_owned(),
        detail: format!(
            "object {} (generation {}) carries no bounds to mask",
            id.index(),
            id.generation()
        ),
    })
}

/// A uniform-grayscale mask image over `rect`: every tile the same
/// gray via the format's solid-matte `TMCi` refs, so a flat mask
/// carries no tile payloads at all.
fn uniform_mask_image(rect: geom::Rect, level: u8) -> ImageObject {
    let width = (rect.width().ceil() as i64).max(1);
    let height = (rect.height().ceil() as i64).max(1);
    let side = raster::TILE_SIDE as i64;
    let argb = i64::from(0xFF00_0000u32 | (u32::from(level) * 0x01_01_01));
    let grid = (width as usize).div_ceil(raster::TILE_SIDE)
        * (height as usize).div_ceil(raster::TILE_SIDE);
    ImageObject {
        location: Point::new(rect.x0, rect.y0),
        offset: Point::ZERO,
        pixels: (width, height),
        tile_size: side,
        tiles: vec![TileRef::Matte(argb); grid],
        name: None,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        effects: Vec::new(),
    }
}

/// A mask image carrying `plane`'s coverage as one gray byte per
/// pixel, stored in fresh tiles at `location`. Tile ids allocate
/// sequentially from `next`; every id minted lands in `minted` so the
/// caller can push a `Delta::Bank` inverse.
pub(crate) fn plane_mask_image(
    document: &mut Document,
    plane: &SelectionMask,
    location: Point,
    next: &mut u32,
    minted: &mut Vec<u32>,
) -> ImageObject {
    let side = raster::TILE_SIDE;
    let width = plane.width as usize;
    let height = plane.height as usize;
    let cols = width.div_ceil(side).max(1);
    let rows = height.div_ceil(side).max(1);
    let mut tiles = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            let mut buffer = [0u8; raster::TILE_BYTES];
            for ty in 0..side {
                let py = row * side + ty;
                if py >= height {
                    break;
                }
                for tx in 0..side {
                    let px = col * side + tx;
                    if px >= width {
                        break;
                    }
                    let gray = plane.alpha[py * width + px];
                    let base = (ty * side + tx) * 4;
                    buffer[base] = gray;
                    buffer[base + 1] = gray;
                    buffer[base + 2] = gray;
                    buffer[base + 3] = 255;
                }
            }
            let tile_id = *next;
            *next += 1;
            minted.push(tile_id);
            document
                .tiles
                .insert(tile_id, raster::Tile::from_rgba8(buffer));
            tiles.push(TileRef::Tile(tile_id));
        }
    }
    ImageObject {
        location,
        offset: Point::ZERO,
        pixels: (i64::from(plane.width), i64::from(plane.height)),
        tile_size: raster::TILE_SIDE as i64,
        tiles,
        name: None,
        opacity: 1000,
        blend: 0,
        visible: true,
        locked: false,
        effects: Vec::new(),
    }
}

/// The mask child's coverage as a gray plane spanning `content`'s
/// pixels: image masks gather their pixels (red channel, overlaying
/// the content exactly), path masks rasterize through [`stamp_path`].
fn mask_gray_plane(
    document: &Document,
    name: &'static str,
    mask_id: ObjectId,
    content: &ImageObject,
) -> Result<SelectionMask, EngineError> {
    let (width, height) = check_editable(content, name)?;
    let dims = (
        u32::try_from(width).unwrap_or(0),
        u32::try_from(height).unwrap_or(0),
    );
    match access::object(document, name, mask_id)? {
        ObjectData::Image(mask) => {
            let (mask_w, mask_h) = check_editable(mask, name)?;
            if (mask_w, mask_h) != (width, height) {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: format!(
                        "mask image {mask_w}x{mask_h} does not cover the {width}x{height} content"
                    ),
                });
            }
            if mask.location != content.location {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: "mask image and content do not overlay (locations differ)".to_owned(),
                });
            }
            let flat = gather(mask, &document.tiles, name)?;
            Ok(SelectionMask {
                width: dims.0,
                height: dims.1,
                alpha: flat.pixels.chunks_exact(4).map(|pixel| pixel[0]).collect(),
            })
        }
        ObjectData::Path(path) => {
            let mut plane = SelectionMask::empty(dims.0, dims.1);
            stamp_path(
                &mut plane,
                path,
                Vec2::new(-content.location.x, -content.location.y),
                name,
            )?;
            Ok(plane)
        }
        other => Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("mask kind {other:?} carries no rasterizable coverage"),
        }),
    }
}

/// Stamps a path object's contours into `mask`, shifting every point
/// by `offset` (callers pass the negated image origin to move from
/// document space into image pixel space). Cubic spans flatten
/// exactly like the renderer's (`vector::flatten_cubic`), lines stay
/// straight. Open contours close chord-wise (a mask from an open path
/// is degenerate in CS6 too).
pub(crate) fn stamp_path(
    mask: &mut SelectionMask,
    path: &PathObject,
    offset: Vec2,
    name: &'static str,
) -> Result<(), EngineError> {
    for contour in &path.contours {
        let mut vertices: Vec<(f64, f64)> = Vec::new();
        for seg in vector::contour_segments(contour) {
            match seg {
                vector::Seg::Line(line) => {
                    if vertices.is_empty() {
                        vertices.push((line.p0.x + offset.x, line.p0.y + offset.y));
                    }
                    vertices.push((line.p1.x + offset.x, line.p1.y + offset.y));
                }
                vector::Seg::Cubic(cubic) => {
                    let polyline = vector::flatten_cubic(cubic).map_err(|error| {
                        EngineError::InvalidState {
                            name: name.to_owned(),
                            detail: format!("mask path will not flatten: {error}"),
                        }
                    })?;
                    for (_, point) in polyline {
                        vertices.push((point.x + offset.x, point.y + offset.y));
                    }
                }
            }
        }
        if vertices.len() >= 3 {
            stamp_polygon(mask, &vertices);
        }
    }
    Ok(())
}

/// Traces `plane`'s ≥128 coverage boundary into closed linear
/// contours: one directed unit edge along every filled cell's empty
/// faces (E/S/W/N), linked into loops — straight, then right-turn,
/// then any remainder at saddle points — with collinear runs merged.
/// Loop starts come in sorted lattice order, so the output is
/// deterministic; lattice points shift by `origin` into document
/// space.
fn plane_to_path(plane: &SelectionMask, origin: Point) -> Vec<Contour> {
    let width = plane.width as usize;
    let height = plane.height as usize;
    let filled = |x: i64, y: i64| {
        x >= 0
            && y >= 0
            && (x as usize) < width
            && (y as usize) < height
            && plane.alpha[y as usize * width + x as usize] >= 128
    };
    // Directed boundary edges, from-cell scan order.
    let mut edges: std::collections::BTreeMap<(i64, i64), Vec<(i64, i64)>> =
        std::collections::BTreeMap::new();
    for y in 0..height {
        for x in 0..width {
            if !filled(x as i64, y as i64) {
                continue;
            }
            let (xi, yi) = (x as i64, y as i64);
            let mut push = |from: (i64, i64), to: (i64, i64)| {
                edges.entry(from).or_default().push(to);
            };
            if !filled(xi, yi - 1) {
                push((xi, yi), (xi + 1, yi));
            }
            if !filled(xi + 1, yi) {
                push((xi + 1, yi), (xi + 1, yi + 1));
            }
            if !filled(xi, yi + 1) {
                push((xi + 1, yi + 1), (xi, yi + 1));
            }
            if !filled(xi - 1, yi) {
                push((xi, yi + 1), (xi, yi));
            }
        }
    }
    let direction = |from: (i64, i64), to: (i64, i64)| (to.0 - from.0, to.1 - from.1);
    // Right turn in y-down space: E→S→W→N→E.
    let right_of = |dir: (i64, i64)| match dir {
        (1, 0) => (0, 1),
        (0, 1) => (-1, 0),
        (-1, 0) => (0, -1),
        _ => (1, 0),
    };
    let mut contours = Vec::new();
    let starts: Vec<(i64, i64)> = edges.keys().copied().collect();
    for start in starts {
        while edges
            .get(&start)
            .is_some_and(|outgoing| !outgoing.is_empty())
        {
            let mut loop_points: Vec<(i64, i64)> = vec![start];
            let mut current = start;
            let mut dir: Option<(i64, i64)> = None;
            while let Some(outgoing) = edges.get_mut(&current).filter(|outs| !outs.is_empty()) {
                let pick = match dir {
                    None => 0,
                    Some(dir) => outgoing
                        .iter()
                        .position(|&to| direction(current, to) == dir)
                        .or_else(|| {
                            outgoing
                                .iter()
                                .position(|&to| direction(current, to) == right_of(dir))
                        })
                        .unwrap_or(0),
                };
                let to = outgoing.remove(pick);
                dir = Some(direction(current, to));
                current = to;
                if to == start {
                    break;
                }
                loop_points.push(to);
            }
            // Merge collinear runs; corners anchor the path points.
            let count = loop_points.len();
            if count < 4 {
                continue;
            }
            let mut corners: Vec<(i64, i64)> = Vec::new();
            for (index, point) in loop_points.iter().enumerate() {
                let point = *point;
                let prev = loop_points[(index + count - 1) % count];
                let next = loop_points[(index + 1) % count];
                if direction(prev, point) == direction(point, next) {
                    continue;
                }
                corners.push(point);
            }
            if corners.len() < 3 {
                continue;
            }
            contours.push(Contour {
                closed: true,
                point_count: corners.len() as i64,
                points: corners
                    .into_iter()
                    .map(|(x, y)| PathPoint {
                        anchor: Point::new(x as f64 + origin.x, y as f64 + origin.y),
                        primary: None,
                        secondary: None,
                        is_curve: false,
                    })
                    .collect(),
            });
        }
    }
    contours
}

/// Parses `mask.add`'s CS6 mode strings.
fn add_mode_of(name: &'static str, mode: &str) -> Result<AddMode, EngineError> {
    match mode {
        "reveal all" => Ok(AddMode::RevealAll),
        "hide all" => Ok(AddMode::HideAll),
        "reveal selection" => Ok(AddMode::RevealSelection),
        "hide selection" => Ok(AddMode::HideSelection),
        other => Err(EngineError::BadPayload {
            name: name.to_owned(),
            detail: format!(
                "mode {other:?} is not one of \"reveal all\"/\"hide all\"/\"reveal selection\"/\"hide selection\""
            ),
        }),
    }
}

/// The `mask.add` mask source, after selection degradation.
#[derive(Clone, Copy, PartialEq)]
enum AddMode {
    RevealAll,
    HideAll,
    RevealSelection,
    HideSelection,
}

impl AddMode {
    /// True when the mask starts hidden (`hide *` modes).
    fn is_hide(self) -> bool {
        matches!(self, Self::HideAll | Self::HideSelection)
    }

    /// The all-mode this mode degrades to without a pixel selection.
    fn to_all(self) -> Self {
        if self.is_hide() {
            Self::HideAll
        } else {
            Self::RevealAll
        }
    }
}

/// Parses `mask.remove`'s CS6 `whatIfElementIsAnImage` strings.
fn remove_mode_of(name: &'static str, mode: Option<&String>) -> Result<RemoveMode, EngineError> {
    match mode.map(String::as_str) {
        None | Some("discard") => Ok(RemoveMode::Discard),
        Some("apply") => Ok(RemoveMode::Apply),
        Some("ask") => Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "\"ask\" opens a dialog; the engine has no dialog surface".to_owned(),
        }),
        Some(other) => Err(EngineError::BadPayload {
            name: name.to_owned(),
            detail: format!("what_if_image {other:?} is not one of \"apply\"/\"discard\"/\"ask\""),
        }),
    }
}

/// The `mask.remove` image-content policy.
#[derive(Clone, Copy, PartialEq)]
enum RemoveMode {
    Apply,
    Discard,
}

/// Attaches `id` back where `owner` was captured (the `AddMask` /
/// `RemoveMask` splice step).
pub(crate) fn attach_at(
    document: &mut Document,
    name: &'static str,
    owner: Owner,
    id: ObjectId,
) -> Result<Delta, EngineError> {
    match owner {
        Owner::Cell {
            page,
            layer,
            cell,
            position,
        } => attach_cell(document, name, page, layer, cell, id, Some(position)),
        Owner::Group {
            id: parent,
            position,
        } => {
            let before = match document.registry.get(parent) {
                Some(ObjectData::Group(group)) => group.clone(),
                _ => {
                    return Err(EngineError::MissingTarget {
                        name: name.to_owned(),
                        detail: format!(
                            "parent group {} (generation {}) is not live",
                            parent.index(),
                            parent.generation()
                        ),
                    });
                }
            };
            if let Some(ObjectData::Group(group)) = document.registry.get_mut(parent) {
                if position > group.children.len() {
                    return Err(EngineError::DeltaMismatch {
                        detail: format!(
                            "parent group {} (generation {}) lost child slot {position}",
                            parent.index(),
                            parent.generation()
                        ),
                    });
                }
                group.children.insert(position, id);
            }
            Ok(Delta::Object {
                id: parent,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(ObjectData::Group(before)),
                },
            })
        }
    }
}

// --- mask.add -----------------------------------------------------------

/// `mask.add` — wraps the target in a fresh grayscale mask group
/// (addElementMask). Exactly one target; an existing element mask is
/// replaced by building fresh. "reveal/hide selection" degrade to the
/// all-modes without a pixel selection (documented CS6 behavior).
struct AddMask;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddMaskPayload {
    /// The object to mask (or the mask group to replace).
    id: ObjectId,
    /// "reveal all" / "hide all" / "reveal selection" / "hide selection".
    mode: String,
    /// Signature parity with CS6's `bEnterMaskEditMode`: accepted and
    /// ignored — the engine has no paint-mode surface.
    #[serde(default)]
    enter_edit_mode: Option<bool>,
}

impl TypedCommand for AddMask {
    type Payload = AddMaskPayload;

    fn name(&self) -> &'static str {
        "mask.add"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.add";
        let mut mode = add_mode_of(NAME, &payload.mode)?;
        // A mask group names its content child as the true target.
        let (target_id, previous) =
            match access::object(document, NAME, payload.id)? {
                ObjectData::Group(group)
                    if group.group_type == GRAYSCALE_MASK || group.group_type == SHAPE_MASK =>
                {
                    let content = group.children.get(1).copied().ok_or_else(|| {
                        EngineError::InvalidState {
                            name: NAME.to_owned(),
                            detail: "the mask group carries no content child".to_owned(),
                        }
                    })?;
                    (
                        content,
                        Some(MaskGroup {
                            id: payload.id,
                            group: group.clone(),
                        }),
                    )
                }
                _ => (
                    payload.id,
                    owning_mask_group(document, NAME, payload.id).ok(),
                ),
            };
        // "reveal/hide selection" without a pixel selection degrade to
        // the all-modes (documented CS6 behavior).
        let mut selection_plane = None;
        if matches!(mode, AddMode::RevealSelection | AddMode::HideSelection) {
            match document.selection.clone() {
                Some(plane) => selection_plane = Some(plane),
                None => mode = mode.to_all(),
            }
        }
        let content_rect = bounds_of(document, NAME, target_id)?;

        // Detach whichever object the new group replaces in its
        // container: the old mask group (replace) or the content.
        let (owner, container_delta) = match &previous {
            Some(resolved) => {
                let owner = find_owner(document, resolved.id).ok_or_else(|| {
                    EngineError::MissingTarget {
                        name: NAME.to_owned(),
                        detail: format!(
                            "mask group {} (generation {}) is referenced by no cell or group",
                            resolved.id.index(),
                            resolved.id.generation()
                        ),
                    }
                })?;
                (owner, detach(document, NAME, resolved.id)?)
            }
            None => {
                let owner =
                    find_owner(document, target_id).ok_or_else(|| EngineError::MissingTarget {
                        name: NAME.to_owned(),
                        detail: format!(
                            "object {} (generation {}) is referenced by no cell or group",
                            target_id.index(),
                            target_id.generation()
                        ),
                    })?;
                (owner, detach(document, NAME, target_id)?)
            }
        };

        // Mint the mask image, then the group shell — mint order is
        // the Created-inverse order (LIFO retraction).
        let mut next_tile = document.tiles.ids().max().unwrap_or(0) + 1;
        let mut minted = Vec::new();
        let mask_image = match mode {
            AddMode::RevealAll => uniform_mask_image(content_rect, 255),
            AddMode::HideAll => uniform_mask_image(content_rect, 0),
            AddMode::RevealSelection | AddMode::HideSelection => {
                let Some(mut plane) = selection_plane else {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: "selection vanished between validation and mint".to_owned(),
                    });
                };
                if mode.is_hide() {
                    for slot in plane.alpha.iter_mut() {
                        *slot = 255 - *slot;
                    }
                }
                plane_mask_image(
                    document,
                    &plane,
                    Point::new(content_rect.x0, content_rect.y0),
                    &mut next_tile,
                    &mut minted,
                )
            }
        };
        let mask_id = document.registry.insert(ObjectData::Image(mask_image));
        let group_id = document.registry.insert(ObjectData::Group(GroupObject {
            group_type: GRAYSCALE_MASK,
            name: None,
            children: vec![mask_id, target_id],
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            mask_hidden: false,
            mask_linked: None,
            effects: Vec::new(),
        }));

        let mut deltas = vec![container_delta, attach_at(document, NAME, owner, group_id)?];
        deltas.push(Delta::Object {
            id: mask_id,
            inverse: ObjectInverse::Created,
        });
        deltas.push(Delta::Object {
            id: group_id,
            inverse: ObjectInverse::Created,
        });
        if !minted.is_empty() {
            deltas.push(Delta::Bank { added: minted });
        }
        if let Some(old) = previous {
            let removed_mask = old.group.children.first().copied();
            deltas.push(Delta::Object {
                id: old.id,
                inverse: ObjectInverse::Removed {
                    object: Box::new(ObjectData::Group(old.group)),
                },
            });
            document.registry.remove(old.id);
            if let Some(old_mask_id) = removed_mask {
                if let Some(old_mask) = document.registry.remove(old_mask_id) {
                    deltas.push(Delta::Object {
                        id: old_mask_id,
                        inverse: ObjectInverse::Removed {
                            object: Box::new(old_mask),
                        },
                    });
                }
            }
        }
        Ok(Delta::Group(deltas))
    }
}

// --- mask.enable --------------------------------------------------------

/// `mask.enable` — toggles the owning element mask between enabled
/// (`GRTi` 1/2) and disabled (`GRTi` 0 with the mask-first structure
/// preserved; decision 0020's flagged design point). Re-enabling
/// re-derives the type from the mask child's kind.
struct EnableMask;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct EnableMaskPayload {
    /// The masked object, or its mask group.
    id: ObjectId,
    /// Enable (true) or disable (false).
    enable: bool,
}

impl TypedCommand for EnableMask {
    type Payload = EnableMaskPayload;

    fn name(&self) -> &'static str {
        "mask.enable"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.enable";
        let resolved = owning_mask_group_deep(document, NAME, payload.id, payload.enable)?;
        let mut group = resolved.group.clone();
        if payload.enable {
            if group.group_type != REGULAR_GROUP {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "object {} (generation {}) already carries an enabled mask",
                        payload.id.index(),
                        payload.id.generation()
                    ),
                });
            }
            let derived = group
                .children
                .first()
                .and_then(|&mask_id| document.registry.get(mask_id))
                .and_then(group_type_of_mask);
            let Some(group_type) = derived else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: "the disabled group's first child is no mask kind".to_owned(),
                });
            };
            group.group_type = group_type;
        } else {
            group.group_type = REGULAR_GROUP;
        }
        if let Some(ObjectData::Group(slot)) = document.registry.get_mut(resolved.id) {
            *slot = group.clone();
        }
        Ok(Delta::Object {
            id: resolved.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(ObjectData::Group(resolved.group)),
            },
        })
    }
}

// --- mask.link ----------------------------------------------------------

/// `mask.link` — sets the mask group's `LNKb` (linkElementMask):
/// linked is the node's absence; unlinked writes `LNKb false`.
struct LinkMask;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct LinkMaskPayload {
    /// The masked object, or its mask group.
    id: ObjectId,
    /// Link the mask to its content (true) or unlink (false).
    linked: bool,
}

impl TypedCommand for LinkMask {
    type Payload = LinkMaskPayload;

    fn name(&self) -> &'static str {
        "mask.link"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.link";
        let resolved = owning_mask_group_deep(document, NAME, payload.id, true)?;
        let mut group = resolved.group.clone();
        group.mask_linked = (!payload.linked).then_some(false);
        if let Some(ObjectData::Group(slot)) = document.registry.get_mut(resolved.id) {
            *slot = group.clone();
        }
        Ok(Delta::Object {
            id: resolved.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(ObjectData::Group(resolved.group)),
            },
        })
    }
}

// --- mask.moveBy / mask.moveContentsBy ----------------------------------

/// `mask.moveBy` — translates the MASK of every masked target
/// (moveElementMaskBy). Elements without masks are skipped; when no
/// target carries a mask the command refuses (CS6 throws).
struct MoveMaskBy;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct MoveMaskByPayload {
    /// Objects whose masks move.
    ids: Vec<ObjectId>,
    /// Pixel delta.
    dx: f64,
    dy: f64,
}

impl TypedCommand for MoveMaskBy {
    type Payload = MoveMaskByPayload;

    fn name(&self) -> &'static str {
        "mask.moveBy"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.moveBy";
        let mut mask_ids = Vec::new();
        for id in &payload.ids {
            if let Ok(resolved) = owning_mask_group(document, NAME, *id) {
                if let Some(&mask_id) = resolved.group.children.first() {
                    if !mask_ids.contains(&mask_id) {
                        mask_ids.push(mask_id);
                    }
                }
            }
        }
        if mask_ids.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "no target carries an element mask".to_owned(),
            });
        }
        let mut moved = std::collections::HashSet::new();
        let mut inverses = Vec::new();
        for mask_id in mask_ids {
            translate_object(
                document,
                NAME,
                mask_id,
                Vec2::new(payload.dx, payload.dy),
                &mut moved,
                &mut inverses,
            )?;
        }
        Ok(Delta::Group(inverses))
    }
}

/// `mask.moveContentsBy` — translates the CONTENT of every masked
/// target, leaving masks in place (moveMaskGroupContentsBy): the
/// group's content child for a mask group, the element behind an
/// existing mask for a masked object. Nothing matched is a no-op.
struct MoveMaskContentsBy;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct MoveMaskContentsByPayload {
    /// Mask groups or masked objects.
    ids: Vec<ObjectId>,
    /// Pixel delta.
    dx: f64,
    dy: f64,
}

impl TypedCommand for MoveMaskContentsBy {
    type Payload = MoveMaskContentsByPayload;

    fn name(&self) -> &'static str {
        "mask.moveContentsBy"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.moveContentsBy";
        let mut content_ids = Vec::new();
        for id in &payload.ids {
            let content = match access::object(document, NAME, *id) {
                Ok(ObjectData::Group(group))
                    if group.group_type == GRAYSCALE_MASK || group.group_type == SHAPE_MASK =>
                {
                    group.children.get(1).copied()
                }
                Ok(_) => owning_mask_group(document, NAME, *id)
                    .ok()
                    .and_then(|resolved| resolved.group.children.get(1).copied()),
                Err(_) => None,
            };
            if let Some(content) = content {
                if !content_ids.contains(&content) {
                    content_ids.push(content);
                }
            }
        }
        if content_ids.is_empty() {
            return Ok(Delta::Noop);
        }
        let mut moved = std::collections::HashSet::new();
        let mut inverses = Vec::new();
        for content in content_ids {
            translate_object(
                document,
                NAME,
                content,
                Vec2::new(payload.dx, payload.dy),
                &mut moved,
                &mut inverses,
            )?;
        }
        Ok(Delta::Group(inverses))
    }
}

// --- mask.remove --------------------------------------------------------

/// `mask.remove` — dissolves the owning mask group, restoring the
/// content to the group's own position (removeElementMask). "apply"
/// first multiplies an image content's alpha by the mask's coverage;
/// "discard" drops the mask; "ask" is a dialog and refuses.
struct RemoveMask;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RemoveMaskPayload {
    /// The masked object, or its mask group.
    id: ObjectId,
    /// "apply" / "discard" / "ask" (CS6's `whatIfElementIsAnImage`;
    /// omitted = "discard").
    #[serde(default)]
    what_if_image: Option<String>,
}

impl TypedCommand for RemoveMask {
    type Payload = RemoveMaskPayload;

    fn name(&self) -> &'static str {
        "mask.remove"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.remove";
        let mode = remove_mode_of(NAME, payload.what_if_image.as_ref())?;
        let resolved = owning_mask_group(document, NAME, payload.id)?;
        let Some(&mask_id) = resolved.group.children.first() else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the mask group carries no mask child".to_owned(),
            });
        };
        let Some(&content_id) = resolved.group.children.get(1) else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the mask group carries no content child".to_owned(),
            });
        };

        // "apply": bake the mask into the content's alpha first.
        let mut deltas = Vec::new();
        if matches!(mode, RemoveMode::Apply) {
            let content = match access::object(document, NAME, content_id)? {
                ObjectData::Image(image) => image.clone(),
                _ => {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: "\"apply\" bakes into image content only; use \"discard\" here"
                            .to_owned(),
                    });
                }
            };
            let coverage = mask_gray_plane(document, NAME, mask_id, &content)?;
            deltas.push(apply_coverage_to_image(
                document, NAME, content_id, &content, coverage,
            )?);
        }

        // Splice: the content takes the group's place; the mask and
        // the group leave the registry (undo revives both).
        let owner =
            find_owner(document, resolved.id).ok_or_else(|| EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!(
                    "mask group {} (generation {}) is referenced by no cell or group",
                    resolved.id.index(),
                    resolved.id.generation()
                ),
            })?;
        deltas.push(detach(document, NAME, resolved.id)?);
        deltas.push(attach_at(document, NAME, owner, content_id)?);
        for removed in [mask_id, resolved.id] {
            let Some(object) = document.registry.remove(removed) else {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "object {} (generation {}) vanished between capture and removal",
                        removed.index(),
                        removed.generation()
                    ),
                });
            };
            deltas.push(Delta::Object {
                id: removed,
                inverse: ObjectInverse::Removed {
                    object: Box::new(object),
                },
            });
        }
        Ok(Delta::Group(deltas))
    }
}

/// Multiplies every pixel's alpha by `coverage`'s gray /255 (the
/// "apply" bake), capturing and rewriting the touched tiles.
fn apply_coverage_to_image(
    document: &mut Document,
    name: &'static str,
    id: ObjectId,
    image: &ImageObject,
    coverage: SelectionMask,
) -> Result<Delta, EngineError> {
    let (width, height) = check_editable(image, name)?;
    if u64::from(coverage.width) != width || u64::from(coverage.height) != height {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!(
                "mask plane spans {}x{} but the image is {width}x{height}",
                coverage.width, coverage.height
            ),
        });
    }
    let tile_ids = touched_tiles(image, &coverage);
    let before_tiles: Vec<(u32, raster::Tile)> = tile_ids
        .iter()
        .filter_map(|tile_id| {
            document
                .tiles
                .get(*tile_id)
                .map(|tile| (*tile_id, tile.clone()))
        })
        .collect();
    let mut flat = gather(image, &document.tiles, name)?;
    for (pixel, &gray) in flat.pixels.chunks_exact_mut(4).zip(&coverage.alpha) {
        pixel[3] = ((u16::from(pixel[3]) * u16::from(gray) + 127) / 255) as u8;
    }
    scatter(&flat, image, &mut document.tiles, &tile_ids, name)?;
    Ok(Delta::Tiles {
        id,
        before: before_tiles,
    })
}

// --- mask.setMode -------------------------------------------------------

/// `mask.setMode` — converts the mask between raster and vector
/// (setElementMaskMode). "mask to image" rasterizes the path mask
/// (binary coverage, no anti-aliasing); "mask to path" thresholds the
/// image mask's gray at 128 and traces its boundary. The mask child
/// is replaced and `GRTi` follows the kind.
struct SetMaskMode;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetMaskModePayload {
    /// The masked object, or its mask group.
    id: ObjectId,
    /// "mask to image" / "mask to path".
    mode: String,
}

/// The `mask.setMode` conversion direction.
enum Direction {
    ToImage,
    ToPath,
}

impl TypedCommand for SetMaskMode {
    type Payload = SetMaskModePayload;

    fn name(&self) -> &'static str {
        "mask.setMode"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.setMode";
        let direction = match payload.mode.as_str() {
            "mask to image" => Direction::ToImage,
            "mask to path" => Direction::ToPath,
            other => {
                return Err(EngineError::BadPayload {
                    name: NAME.to_owned(),
                    detail: format!(
                        "mode {other:?} is not one of \"mask to image\"/\"mask to path\""
                    ),
                });
            }
        };
        let resolved = owning_mask_group(document, NAME, payload.id)?;
        let Some(&mask_id) = resolved.group.children.first() else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the mask group carries no mask child".to_owned(),
            });
        };
        let Some(&content_id) = resolved.group.children.get(1) else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the mask group carries no content child".to_owned(),
            });
        };

        // Build the replacement mask before minting anything.
        let mut next_tile = document.tiles.ids().max().unwrap_or(0) + 1;
        let mut minted = Vec::new();
        let replacement = match direction {
            Direction::ToImage => {
                let path = match access::object(document, NAME, mask_id)? {
                    ObjectData::Path(path) => path.clone(),
                    _ => {
                        return Err(EngineError::InvalidState {
                            name: NAME.to_owned(),
                            detail: "\"mask to image\" needs a path mask".to_owned(),
                        });
                    }
                };
                // Rasterize over the content's bounds — what the mask
                // applies to — at the document's unit scale.
                let bounds = bounds_of(document, NAME, content_id)?;
                let mut plane = SelectionMask::empty(
                    u32::try_from(bounds.width().ceil() as i64)
                        .unwrap_or(1)
                        .max(1),
                    u32::try_from(bounds.height().ceil() as i64)
                        .unwrap_or(1)
                        .max(1),
                );
                stamp_path(&mut plane, &path, Vec2::new(-bounds.x0, -bounds.y0), NAME)?;
                ObjectData::Image(plane_mask_image(
                    document,
                    &plane,
                    Point::new(bounds.x0, bounds.y0),
                    &mut next_tile,
                    &mut minted,
                ))
            }
            Direction::ToPath => {
                let image = match access::object(document, NAME, mask_id)? {
                    ObjectData::Image(image) => image.clone(),
                    _ => {
                        return Err(EngineError::InvalidState {
                            name: NAME.to_owned(),
                            detail: "\"mask to path\" needs an image mask".to_owned(),
                        });
                    }
                };
                let (width, height) = check_editable(&image, NAME)?;
                let flat = gather(&image, &document.tiles, NAME)?;
                let mut plane = SelectionMask::empty(
                    u32::try_from(width).unwrap_or(0),
                    u32::try_from(height).unwrap_or(0),
                );
                for (slot, pixel) in plane.alpha.iter_mut().zip(flat.pixels.chunks_exact(4)) {
                    *slot = pixel[0];
                }
                let contours = plane_to_path(&plane, image.location);
                if contours.is_empty() {
                    return Err(EngineError::InvalidState {
                        name: NAME.to_owned(),
                        detail: "the mask's coverage has no boundary to trace".to_owned(),
                    });
                }
                ObjectData::Path(PathObject::neutral(contours))
            }
        };

        let before = resolved.group;
        let new_mask_id = document.registry.insert(replacement);
        let new_type = match document.registry.get(new_mask_id) {
            Some(ObjectData::Image(_)) => GRAYSCALE_MASK,
            _ => SHAPE_MASK,
        };
        if let Some(ObjectData::Group(slot)) = document.registry.get_mut(resolved.id) {
            slot.children[0] = new_mask_id;
            slot.group_type = new_type;
        }
        let removed = document.registry.remove(mask_id);
        let mut deltas = vec![Delta::Object {
            id: resolved.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(ObjectData::Group(before)),
            },
        }];
        deltas.push(Delta::Object {
            id: new_mask_id,
            inverse: ObjectInverse::Created,
        });
        if !minted.is_empty() {
            deltas.push(Delta::Bank { added: minted });
        }
        if let Some(object) = removed {
            deltas.push(Delta::Object {
                id: mask_id,
                inverse: ObjectInverse::Removed {
                    object: Box::new(object),
                },
            });
        }
        Ok(Delta::Group(deltas))
    }
}

// --- mask.setShowAttrs --------------------------------------------------

/// `mask.setShowAttrs` — toggles the mask's own fill/stroke
/// visibility (setElementMaskShowAttrs): `MNAb` present = hidden.
struct SetMaskShowAttrs;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetMaskShowAttrsPayload {
    /// The masked object, or its mask group.
    id: ObjectId,
    /// Show (true) or hide (false) the mask's fill and stroke.
    show: bool,
}

impl TypedCommand for SetMaskShowAttrs {
    type Payload = SetMaskShowAttrsPayload;

    fn name(&self) -> &'static str {
        "mask.setShowAttrs"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.setShowAttrs";
        let resolved = owning_mask_group_deep(document, NAME, payload.id, true)?;
        let mut group = resolved.group.clone();
        group.mask_hidden = !payload.show;
        if let Some(ObjectData::Group(slot)) = document.registry.get_mut(resolved.id) {
            *slot = group.clone();
        }
        Ok(Delta::Object {
            id: resolved.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(ObjectData::Group(resolved.group)),
            },
        })
    }
}

// --- mask.pathToMarquee -------------------------------------------------

/// `mask.pathToMarquee` — rasterizes a path into the pixel selection
/// over a target image (convertPathToMarquee). "hard edge" and
/// "antialias" stamp binary coverage (no AA sampling — recorded);
/// "feather" runs the plane through the feather pass.
struct PathToMarquee;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathToMarqueePayload {
    /// The path object.
    id: ObjectId,
    /// The image whose pixel space the marquee spans.
    image: ObjectId,
    /// "hard edge" / "antialias" / "feather".
    mode: String,
    /// Feather radius in pixels (mode "feather" only).
    #[serde(default)]
    feather_amount: Option<f64>,
}

impl TypedCommand for PathToMarquee {
    type Payload = PathToMarqueePayload;

    fn name(&self) -> &'static str {
        "mask.pathToMarquee"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.pathToMarquee";
        let feathering = match payload.mode.as_str() {
            "hard edge" | "antialias" => false,
            "feather" => true,
            other => {
                return Err(EngineError::BadPayload {
                    name: NAME.to_owned(),
                    detail: format!(
                        "mode {other:?} is not one of \"hard edge\"/\"antialias\"/\"feather\""
                    ),
                });
            }
        };
        let path = match access::object(document, NAME, payload.id)? {
            ObjectData::Path(path) => path.clone(),
            _ => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a path", payload.id.index()),
                });
            }
        };
        let image = match access::object(document, NAME, payload.image)? {
            ObjectData::Image(image) => image.clone(),
            _ => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not an image", payload.image.index()),
                });
            }
        };
        let (width, height) = check_editable(&image, NAME)?;
        let radius = if feathering {
            let amount = payload.feather_amount.unwrap_or(1.0);
            if !amount.is_finite() || amount <= 0.0 {
                return Err(EngineError::BadPayload {
                    name: NAME.to_owned(),
                    detail: format!("feather_amount {amount} is not a positive radius"),
                });
            }
            // The feather pass caps its kernel at 64.
            Some((amount.min(64.0) as u32).max(1))
        } else {
            None
        };
        let before = document.selection.clone();
        let mut plane = SelectionMask::empty(
            u32::try_from(width).unwrap_or(0),
            u32::try_from(height).unwrap_or(0),
        );
        stamp_path(
            &mut plane,
            &path,
            Vec2::new(-image.location.x, -image.location.y),
            NAME,
        )?;
        if let Some(radius) = radius {
            feather_mask(&mut plane, radius);
        }
        document.selection = Some(plane);
        Ok(Delta::Selection {
            before: before.map(Box::new),
        })
    }
}

// --- mask.marqueeToPath -------------------------------------------------

/// `mask.marqueeToPath` — traces the active selection into a path of
/// linear contours, minted into the source image's cell directly
/// above it (convertMarqueeToPath). Boundary tracing is lossy by
/// nature; structural equivalence, not byte equality, is the gate.
struct MarqueeToPath;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct MarqueeToPathPayload {
    /// The image the selection spans.
    image: ObjectId,
}

impl TypedCommand for MarqueeToPath {
    type Payload = MarqueeToPathPayload;

    fn name(&self) -> &'static str {
        "mask.marqueeToPath"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.marqueeToPath";
        let plane = document
            .selection
            .clone()
            .ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "no active selection to trace".to_owned(),
            })?;
        let image = match access::object(document, NAME, payload.image)? {
            ObjectData::Image(image) => image.clone(),
            _ => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not an image", payload.image.index()),
                });
            }
        };
        let (width, height) = check_editable(&image, NAME)?;
        if u64::from(plane.width) != width || u64::from(plane.height) != height {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "selection spans {}x{} but the image is {width}x{height}",
                    plane.width, plane.height
                ),
            });
        }
        let owner =
            find_owner(document, payload.image).ok_or_else(|| EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!(
                    "image {} (generation {}) is referenced by no cell or group",
                    payload.image.index(),
                    payload.image.generation()
                ),
            })?;
        let Owner::Cell {
            page,
            layer,
            cell,
            position,
        } = owner
        else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the image sits inside a group; there is no cell position for the path"
                    .to_owned(),
            });
        };
        let contours = plane_to_path(&plane, image.location);
        if contours.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "the selection has no boundary to trace".to_owned(),
            });
        }
        let path_id = document
            .registry
            .insert(ObjectData::Path(PathObject::neutral(contours)));
        let attach_delta = attach_cell(
            document,
            NAME,
            page,
            layer,
            cell,
            path_id,
            Some(position + 1),
        )?;
        Ok(Delta::Group(vec![
            attach_delta,
            Delta::Object {
                id: path_id,
                inverse: ObjectInverse::Created,
            },
        ]))
    }
}

// --- mask.saveJPEG / mask.restoreJPEG / mask.clearJPEG ------------------

/// `mask.saveJPEG` — stashes the active selection as the "Selective
/// JPEG mask" (saveJPEGMask). Editing state only: the format has no
/// node for it.
struct SaveJpegMask;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
struct SaveJpegMaskPayload {}

impl TypedCommand for SaveJpegMask {
    type Payload = SaveJpegMaskPayload;

    fn name(&self) -> &'static str {
        "mask.saveJPEG"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.saveJPEG";
        let selection = document
            .selection
            .clone()
            .ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "no active selection to store".to_owned(),
            })?;
        let before = document.jpeg_mask.replace(selection);
        Ok(Delta::JpegMask {
            before: before.map(Box::new),
        })
    }
}

/// `mask.restoreJPEG` — makes the stashed JPEG mask the active
/// selection again (restoreJPEGMask).
struct RestoreJpegMask;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
struct RestoreJpegMaskPayload {}

impl TypedCommand for RestoreJpegMask {
    type Payload = RestoreJpegMaskPayload;

    fn name(&self) -> &'static str {
        "mask.restoreJPEG"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "mask.restoreJPEG";
        let stash = document
            .jpeg_mask
            .clone()
            .ok_or_else(|| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "no stored JPEG mask to restore".to_owned(),
            })?;
        let before = document.selection.replace(stash);
        Ok(Delta::Selection {
            before: before.map(Box::new),
        })
    }
}

/// `mask.clearJPEG` — discards the stash (clearJPEGMask); clearing
/// nothing is a no-op.
struct ClearJpegMask;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
struct ClearJpegMaskPayload {}

impl TypedCommand for ClearJpegMask {
    type Payload = ClearJpegMaskPayload;

    fn name(&self) -> &'static str {
        "mask.clearJPEG"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        match document.jpeg_mask.take() {
            Some(stash) => Ok(Delta::JpegMask {
                before: Some(Box::new(stash)),
            }),
            None => Ok(Delta::Noop),
        }
    }
}

// --- bitmap.movePixelMaskBy / bitmap.setPixelMask -----------------------

/// `bitmap.movePixelMaskBy` — translates the pixel selection without
/// moving the pixels inside it (movePixelMaskBy); coverage leaving
/// the image clips away.
struct MovePixelMaskBy;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct MovePixelMaskByPayload {
    /// The image the selection spans.
    id: ObjectId,
    /// Pixel delta.
    dx: i64,
    dy: i64,
}

impl TypedCommand for MovePixelMaskBy {
    type Payload = MovePixelMaskByPayload;

    fn name(&self) -> &'static str {
        "bitmap.movePixelMaskBy"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.movePixelMaskBy";
        let image = match access::object(document, NAME, payload.id)? {
            ObjectData::Image(image) => image.clone(),
            _ => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not an image", payload.id.index()),
                });
            }
        };
        let (width, height) = check_editable(&image, NAME)?;
        let plane = selection_over(document, NAME, width, height)?;
        let before = document.selection.clone();
        let (w, h) = (i64::from(plane.width), i64::from(plane.height));
        let mut shifted = SelectionMask::empty(plane.width, plane.height);
        for y in 0..h {
            for x in 0..w {
                let (sx, sy) = (x - payload.dx, y - payload.dy);
                if sx < 0 || sy < 0 || sx >= w || sy >= h {
                    continue;
                }
                shifted.alpha[(y * w + x) as usize] = plane.alpha[(sy * w + sx) as usize];
            }
        }
        document.selection = Some(shifted);
        Ok(Delta::Selection {
            before: before.map(Box::new),
        })
    }
}

/// The active selection, required to span exactly `width × height`.
fn selection_over(
    document: &Document,
    name: &'static str,
    width: u64,
    height: u64,
) -> Result<SelectionMask, EngineError> {
    let plane = document
        .selection
        .as_ref()
        .ok_or_else(|| EngineError::InvalidState {
            name: name.to_owned(),
            detail: "no active pixel selection".to_owned(),
        })?;
    if u64::from(plane.width) != width || u64::from(plane.height) != height {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!(
                "selection spans {}x{} but the image is {width}x{height}",
                plane.width, plane.height
            ),
        });
    }
    Ok(plane.clone())
}

/// A caller-supplied selection plane (the "Mask data type" shape).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PlanePayload {
    /// Plane width.
    width: u32,
    /// Plane height.
    height: u32,
    /// `width * height` coverage bytes.
    alpha: Vec<u8>,
}

/// `bitmap.setPixelMask` — replaces or folds the pixel selection from
/// a caller-supplied plane (setPixelMask, the deprecated alias of
/// setSelectionMask); a null mask clears.
struct SetPixelMask;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetPixelMaskPayload {
    /// The image the selection spans.
    id: ObjectId,
    /// The plane to apply; null clears the selection.
    #[serde(default)]
    mask: Option<PlanePayload>,
    /// "replace" / "add" / "subtract" / "intersect" (default replace).
    #[serde(default)]
    mode: Option<String>,
}

impl TypedCommand for SetPixelMask {
    type Payload = SetPixelMaskPayload;

    fn name(&self) -> &'static str {
        "bitmap.setPixelMask"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.setPixelMask";
        let image = match access::object(document, NAME, payload.id)? {
            ObjectData::Image(image) => image.clone(),
            _ => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not an image", payload.id.index()),
                });
            }
        };
        let (width, height) = check_editable(&image, NAME)?;
        let before = document.selection.clone();
        let Some(plane) = &payload.mask else {
            document.selection = None;
            return Ok(Delta::Selection {
                before: before.map(Box::new),
            });
        };
        let expected =
            usize::try_from(u64::from(plane.width).saturating_mul(u64::from(plane.height)))
                .unwrap_or(usize::MAX);
        if plane.alpha.len() != expected {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!(
                    "mask carries {} coverage bytes for a {}x{} plane",
                    plane.alpha.len(),
                    plane.width,
                    plane.height
                ),
            });
        }
        if u64::from(plane.width) != width || u64::from(plane.height) != height {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "mask spans {}x{} but the image is {width}x{height}",
                    plane.width, plane.height
                ),
            });
        }
        let Some(mode) = SelectMode::parse(payload.mode.as_deref()) else {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!(
                    "mode {:?} is not one of replace/add/subtract/intersect",
                    payload.mode.clone().unwrap_or_default()
                ),
            });
        };
        let fresh = SelectionMask {
            width: plane.width,
            height: plane.height,
            alpha: plane.alpha.clone(),
        };
        document.selection = Some(fold(document.selection.as_ref(), fresh, mode));
        Ok(Delta::Selection {
            before: before.map(Box::new),
        })
    }
}

// --- tests --------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::command::Command;
    use doc::{Cell, Layer, LayerList, Page, PageInfo};

    const NAME: &str = "mask.test";

    /// One page, one layer, one cell holding an 8×8 red image at the
    /// origin (single 128-tile slot, tile id 0).
    fn image_document() -> (Document, ObjectId) {
        let mut document = Document::default();
        let image = ImageObject {
            location: Point::new(0.0, 0.0),
            offset: Point::ZERO,
            pixels: (8, 8),
            tile_size: raster::TILE_SIDE as i64,
            tiles: vec![TileRef::Tile(0)],
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        };
        document.tiles.insert(
            0,
            raster::Tile::from_rgba8({
                let mut pixels = [0u8; raster::TILE_BYTES];
                for pixel in pixels.chunks_exact_mut(4) {
                    pixel.copy_from_slice(&[255, 0, 0, 255]);
                }
                pixels
            }),
        );
        let image_id = document.registry.insert(ObjectData::Image(image));
        let mut layer = Layer {
            name: "Layer 0".to_owned(),
            display: true,
            ..Layer::default()
        };
        layer.cells.push(Cell {
            locked: false,
            visible: true,
            elements: vec![image_id],
        });
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: 1,
                frame_count_duplicate: 1,
                request_id: 0,
                canvas: Default::default(),
                onion_before: 0,
                onion_after: 0,
                frame_durations: Vec::new(),
            },
            name: Some("Page 0".to_owned()),
            is_master: false,
            is_linked: false,
            page_id: None,
            last_export_dir: None,
            last_export_file: None,
            layers: LayerList {
                layers: vec![layer],
            },
            grid: None,
            guides: Default::default(),
            export: None,
        });
        (document, image_id)
    }

    /// A square path object covering `x0,y0..x0+s,y0+s`.
    fn square_path(x0: f64, y0: f64, s: f64) -> ObjectData {
        let point = |x: f64, y: f64| PathPoint {
            anchor: Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        };
        ObjectData::Path(PathObject::neutral(vec![Contour {
            closed: true,
            point_count: 4,
            points: vec![
                point(x0, y0),
                point(x0 + s, y0),
                point(x0 + s, y0 + s),
                point(x0, y0 + s),
            ],
        }]))
    }

    /// An image-mask group over the 8×8 image: left half gray 255,
    /// right half 0, real tiles at the image origin.
    fn half_plane_group(document: &mut Document, image_id: ObjectId) -> (ObjectId, ObjectId) {
        let mut plane = SelectionMask::empty(8, 8);
        for y in 0..8usize {
            for x in 0..4usize {
                plane.alpha[y * 8 + x] = 255;
            }
        }
        let mut next = document.tiles.ids().max().unwrap_or(0) + 1;
        let mut minted = Vec::new();
        let mask_image = plane_mask_image(
            document,
            &plane,
            Point::new(0.0, 0.0),
            &mut next,
            &mut minted,
        );
        let mask_id = document.registry.insert(ObjectData::Image(mask_image));
        let group_id = document.registry.insert(ObjectData::Group(GroupObject {
            group_type: GRAYSCALE_MASK,
            name: None,
            children: vec![mask_id, image_id],
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            mask_hidden: false,
            mask_linked: None,
            effects: Vec::new(),
        }));
        (mask_id, group_id)
    }

    /// The id the first cell's element slot `position` points at.
    fn cell_element(document: &Document, position: usize) -> Option<ObjectId> {
        document.pages[0].layers.layers[0].cells[0]
            .elements
            .get(position)
            .copied()
    }

    #[test]
    fn add_reveal_all_structures_and_undo_restores() {
        let (mut document, image_id) = image_document();
        let before = document.clone();
        let delta = AddMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "mode": "reveal all" }),
            )
            .unwrap();
        // The cell now references a GRTi 1 group with [mask, image].
        let group_id = cell_element(&document, 0).unwrap();
        let ObjectData::Group(group) = document.registry.get(group_id).unwrap() else {
            panic!("cell should reference the mask group");
        };
        assert_eq!(group.group_type, GRAYSCALE_MASK);
        assert_eq!(group.children[1], image_id);
        let ObjectData::Image(mask) = document.registry.get(group.children[0]).unwrap() else {
            panic!("mask child should be an image");
        };
        // Uniform matte mask: no tile payloads, 8x8 coverage.
        assert!(
            mask.tiles
                .iter()
                .all(|tile| matches!(tile, TileRef::Matte(_)))
        );
        assert_eq!(mask.pixels, (8, 8));

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document, before);
    }

    #[test]
    fn add_hide_selection_inverts_plane() {
        let (mut document, image_id) = image_document();
        // Select the left half (columns 0..4).
        let mut plane = SelectionMask::empty(8, 8);
        for y in 0..8usize {
            for x in 0..4usize {
                plane.alpha[y * 8 + x] = 255;
            }
        }
        document.selection = Some(plane);
        AddMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "mode": "hide selection" }),
            )
            .unwrap();
        let group_id = cell_element(&document, 0).unwrap();
        let ObjectData::Group(group) = document.registry.get(group_id).unwrap() else {
            panic!("cell should reference the mask group");
        };
        let ObjectData::Image(mask) = document.registry.get(group.children[0]).unwrap() else {
            panic!("mask child should be an image");
        };
        // The minted mask inverts the plane: the selected half hides
        // (gray 0), the unselected half reveals (gray 255).
        let flat = gather(mask, &document.tiles, NAME).unwrap();
        assert_eq!(flat.pixels[0], 0); // pixel (0,0) was selected
        assert_eq!(flat.pixels[16], 255); // pixel (4,0) was not
    }

    #[test]
    fn add_reveal_selection_degrades_without_selection() {
        let (mut document, image_id) = image_document();
        AddMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "mode": "reveal selection" }),
            )
            .unwrap();
        let group_id = cell_element(&document, 0).unwrap();
        let ObjectData::Group(group) = document.registry.get(group_id).unwrap() else {
            panic!("cell should reference the mask group");
        };
        let ObjectData::Image(mask) = document.registry.get(group.children[0]).unwrap() else {
            panic!("mask child should be an image");
        };
        // Degraded to "reveal all": uniform opaque matte, no payloads.
        assert!(
            mask.tiles
                .iter()
                .all(|tile| matches!(tile, TileRef::Matte(_)))
        );
    }

    #[test]
    fn enable_disable_round_trips_group_type() {
        let (mut document, image_id) = image_document();
        AddMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "mode": "reveal all" }),
            )
            .unwrap();
        let group_id = cell_element(&document, 0).unwrap();

        // Disable via the content id: GRTi 1 → 0.
        EnableMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "enable": false }),
            )
            .unwrap();
        let ObjectData::Group(group) = document.registry.get(group_id).unwrap() else {
            panic!("group vanished");
        };
        assert_eq!(group.group_type, REGULAR_GROUP);

        // Re-enable via the group id: re-derives 1 from the image child.
        EnableMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": group_id, "enable": true }),
            )
            .unwrap();
        let ObjectData::Group(group) = document.registry.get(group_id).unwrap() else {
            panic!("group vanished");
        };
        assert_eq!(group.group_type, GRAYSCALE_MASK);
    }

    #[test]
    fn link_and_show_attrs_round_trip() {
        let (mut document, image_id) = image_document();
        AddMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "mode": "reveal all" }),
            )
            .unwrap();
        let group_id = cell_element(&document, 0).unwrap();

        LinkMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "linked": false }),
            )
            .unwrap();
        let ObjectData::Group(group) = document.registry.get(group_id).unwrap() else {
            panic!("group vanished");
        };
        assert_eq!(group.mask_linked, Some(false));

        LinkMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "linked": true }),
            )
            .unwrap();
        let ObjectData::Group(group) = document.registry.get(group_id).unwrap() else {
            panic!("group vanished");
        };
        assert_eq!(group.mask_linked, None);

        SetMaskShowAttrs
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "show": false }),
            )
            .unwrap();
        let ObjectData::Group(group) = document.registry.get(group_id).unwrap() else {
            panic!("group vanished");
        };
        assert!(group.mask_hidden);
    }

    #[test]
    fn move_by_moves_mask_contents_move_content() {
        let (mut document, image_id) = image_document();
        AddMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "mode": "reveal all" }),
            )
            .unwrap();
        let group_id = cell_element(&document, 0).unwrap();
        let mask_id = match document.registry.get(group_id).unwrap() {
            ObjectData::Group(group) => group.children[0],
            _ => panic!("group vanished"),
        };
        let mask_before = document.registry.get(mask_id).unwrap().clone();

        MoveMaskContentsBy
            .apply_json(
                &mut document,
                serde_json::json!({ "ids": [image_id], "dx": 3, "dy": -2 }),
            )
            .unwrap();
        assert_eq!(document.registry.get(mask_id).unwrap(), &mask_before);
        let ObjectData::Image(image) = document.registry.get(image_id).unwrap() else {
            panic!("content vanished");
        };
        assert_eq!(image.location, Point::new(3.0, -2.0));

        MoveMaskBy
            .apply_json(
                &mut document,
                serde_json::json!({ "ids": [image_id], "dx": 1, "dy": 1 }),
            )
            .unwrap();
        let moved = document.registry.get(mask_id).unwrap().clone();
        assert_ne!(moved, mask_before);
        // Content untouched by mask.moveBy.
        let ObjectData::Image(image) = document.registry.get(image_id).unwrap() else {
            panic!("content vanished");
        };
        assert_eq!(image.location, Point::new(3.0, -2.0));
    }

    #[test]
    fn remove_discard_restores_content_and_undoes() {
        let (mut document, image_id) = image_document();
        AddMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "mode": "reveal all" }),
            )
            .unwrap();
        let before_remove = document.clone();
        let delta = RemoveMask
            .apply_json(&mut document, serde_json::json!({ "id": image_id }))
            .unwrap();
        // The image is back at slot 0.
        assert_eq!(cell_element(&document, 0), Some(image_id));
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document, before_remove);
    }

    #[test]
    fn remove_apply_bakes_alpha_and_captures_tiles() {
        let (mut document, image_id) = image_document();
        // Shape-mask group over the image: path covering the left half.
        let path_id = document.registry.insert(square_path(0.0, 0.0, 4.0));
        let group_id = document.registry.insert(ObjectData::Group(GroupObject {
            group_type: SHAPE_MASK,
            name: None,
            children: vec![path_id, image_id],
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            mask_hidden: false,
            mask_linked: None,
            effects: Vec::new(),
        }));
        document.pages[0].layers.layers[0].cells[0].elements[0] = group_id;

        RemoveMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "what_if_image": "apply" }),
            )
            .unwrap();
        let ObjectData::Image(image) = document.registry.get(image_id).unwrap() else {
            panic!("content vanished");
        };
        let flat = gather(image, &document.tiles, NAME).unwrap();
        // Left half keeps alpha 255; right half bakes to 0.
        assert_eq!(flat.pixels[3], 255); // pixel (0,0) alpha
        assert_eq!(flat.pixels[19], 0); // pixel (4,0) alpha
        // The path and the group left the registry.
        assert!(document.registry.get(path_id).is_none());
        assert!(document.registry.get(group_id).is_none());
    }

    #[test]
    fn set_mode_image_to_path_traces_boundary() {
        let (mut document, image_id) = image_document();
        let (mask_id, group_id) = half_plane_group(&mut document, image_id);
        document.pages[0].layers.layers[0].cells[0].elements[0] = group_id;

        SetMaskMode
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "mode": "mask to path" }),
            )
            .unwrap();
        let ObjectData::Group(group) = document.registry.get(group_id).unwrap() else {
            panic!("group vanished");
        };
        assert_eq!(group.group_type, SHAPE_MASK);
        assert_ne!(group.children[0], mask_id);
        let ObjectData::Path(path) = document.registry.get(group.children[0]).unwrap() else {
            panic!("replacement mask should be a path");
        };
        // One rectangle contour at the 128-threshold boundary.
        assert_eq!(path.contours.len(), 1);
        assert_eq!(path.contours[0].point_count, 4);
        assert!(document.registry.get(mask_id).is_none());
    }

    #[test]
    fn path_to_marquee_and_marquee_to_path_round_trip_structure() {
        let (mut document, image_id) = image_document();
        let path_id = document.registry.insert(square_path(0.0, 0.0, 4.0));
        PathToMarquee
            .apply_json(
                &mut document,
                serde_json::json!({
                    "id": path_id,
                    "image": image_id,
                    "mode": "hard edge"
                }),
            )
            .unwrap();
        let plane = document.selection.clone().unwrap();
        assert_eq!(plane.alpha[0], 255);
        assert_eq!(plane.alpha[4], 0); // pixel (4,0), outside the path

        let delta = MarqueeToPath
            .apply_json(&mut document, serde_json::json!({ "image": image_id }))
            .unwrap();
        // The path landed in the cell above the image.
        let path_slot = cell_element(&document, 1).unwrap();
        let ObjectData::Path(traced) = document.registry.get(path_slot).unwrap() else {
            panic!("cell slot 1 should be the traced path");
        };
        assert_eq!(traced.contours.len(), 1);
        assert_eq!(traced.contours[0].point_count, 4);
        assert_eq!(traced.contours[0].points[0].anchor, Point::new(0.0, 0.0));
        delta.apply_inverse(&mut document).unwrap();
        assert!(cell_element(&document, 1).is_none());
        assert!(document.registry.get(path_slot).is_none());
    }

    #[test]
    fn feather_mode_softens_the_marquee() {
        let (mut document, image_id) = image_document();
        let path_id = document.registry.insert(square_path(2.0, 2.0, 4.0));
        PathToMarquee
            .apply_json(
                &mut document,
                serde_json::json!({
                    "id": path_id,
                    "image": image_id,
                    "mode": "feather",
                    "feather_amount": 1
                }),
            )
            .unwrap();
        let plane = document.selection.clone().unwrap();
        // Feathered coverage: the densest coverage sits at the square's
        // center, the square's own corner pixels are partial, and the
        // plane's far corner is softer still.
        let center = plane.alpha[4 * 8 + 4];
        let corner = plane.alpha[2 * 8 + 2];
        let far = plane.alpha[63];
        assert!(
            center > corner && corner > far,
            "{center} > {corner} > {far}"
        );
        assert!(
            center < 255 && far > 0,
            "feather spreads but never walls off: {center}, {far}"
        );
    }

    #[test]
    fn jpeg_mask_stash_round_trips() {
        let (mut document, image_id) = image_document();
        let mut plane = SelectionMask::empty(8, 8);
        plane.alpha[0] = 255;
        document.selection = Some(plane);

        SaveJpegMask
            .apply_json(&mut document, serde_json::json!({}))
            .unwrap();
        assert!(document.jpeg_mask.is_some());

        // Clearing the live selection does not touch the stash.
        SetPixelMask
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "mask": null }),
            )
            .unwrap();
        assert!(document.selection.is_none());
        assert!(document.jpeg_mask.is_some());

        RestoreJpegMask
            .apply_json(&mut document, serde_json::json!({}))
            .unwrap();
        assert_eq!(document.selection.as_ref().unwrap().alpha[0], 255);

        let delta = ClearJpegMask
            .apply_json(&mut document, serde_json::json!({}))
            .unwrap();
        assert!(document.jpeg_mask.is_none());
        delta.apply_inverse(&mut document).unwrap();
        assert!(document.jpeg_mask.is_some());

        // Clearing an absent stash is a no-op.
        let (mut fresh, _) = image_document();
        assert!(matches!(
            ClearJpegMask
                .apply_json(&mut fresh, serde_json::json!({}))
                .unwrap(),
            Delta::Noop
        ));
    }

    #[test]
    fn set_pixel_mask_modes_and_clear() {
        let (mut document, image_id) = image_document();
        let full = vec![255u8; 64];
        SetPixelMask
            .apply_json(
                &mut document,
                serde_json::json!({
                    "id": image_id,
                    "mask": { "width": 8, "height": 8, "alpha": full },
                    "mode": "replace"
                }),
            )
            .unwrap();
        assert_eq!(document.selection.as_ref().unwrap().alpha[63], 255);

        let mut column = vec![0u8; 64];
        for y in 0..8 {
            column[y * 8] = 255;
        }
        SetPixelMask
            .apply_json(
                &mut document,
                serde_json::json!({
                    "id": image_id,
                    "mask": { "width": 8, "height": 8, "alpha": column },
                    "mode": "subtract"
                }),
            )
            .unwrap();
        assert_eq!(document.selection.as_ref().unwrap().alpha[0], 0);
        assert_eq!(document.selection.as_ref().unwrap().alpha[63], 255);

        // Wrong-length plane is refused.
        assert!(
            SetPixelMask
                .apply_json(
                    &mut document,
                    serde_json::json!({
                        "id": image_id,
                        "mask": { "width": 4, "height": 4, "alpha": [0] }
                    }),
                )
                .is_err()
        );
    }

    #[test]
    fn move_pixel_mask_by_shifts_with_clipping() {
        let (mut document, image_id) = image_document();
        let mut plane = SelectionMask::empty(8, 8);
        for y in 0..8 {
            plane.alpha[y * 8] = 255; // column 0
        }
        document.selection = Some(plane);
        MovePixelMaskBy
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "dx": 2, "dy": 0 }),
            )
            .unwrap();
        let shifted = document.selection.as_ref().unwrap();
        assert_eq!(shifted.alpha[2], 255); // column 0 → column 2
        assert_eq!(shifted.alpha[0], 0);
        // Move far enough that everything clips away.
        MovePixelMaskBy
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "dx": 100, "dy": 0 }),
            )
            .unwrap();
        assert!(
            document
                .selection
                .as_ref()
                .unwrap()
                .alpha
                .iter()
                .all(|&value| value == 0)
        );
    }

    #[test]
    fn plane_to_path_traces_rectangle_with_merged_runs() {
        let mut plane = SelectionMask::empty(4, 3);
        for y in 0..3usize {
            for x in 0..2usize {
                plane.alpha[y * 4 + x] = 255;
            }
        }
        let contours = plane_to_path(&plane, Point::new(10.0, 20.0));
        assert_eq!(contours.len(), 1);
        assert_eq!(contours[0].point_count, 4);
        let anchors: Vec<(f64, f64)> = contours[0]
            .points
            .iter()
            .map(|point| (point.anchor.x, point.anchor.y))
            .collect();
        assert_eq!(
            anchors,
            vec![(10.0, 20.0), (12.0, 20.0), (12.0, 23.0), (10.0, 23.0)]
        );
    }

    #[test]
    fn no_mask_refusals_match_cs6_throws() {
        let (mut document, image_id) = image_document();
        // moveBy with no masks anywhere: refusal.
        assert!(
            MoveMaskBy
                .apply_json(
                    &mut document,
                    serde_json::json!({ "ids": [image_id], "dx": 1, "dy": 1 }),
                )
                .is_err()
        );
        // remove with no mask: refusal.
        assert!(
            RemoveMask
                .apply_json(&mut document, serde_json::json!({ "id": image_id }))
                .is_err()
        );
        // enable false with no mask: refusal.
        assert!(
            EnableMask
                .apply_json(
                    &mut document,
                    serde_json::json!({ "id": image_id, "enable": false }),
                )
                .is_err()
        );
        // "ask" is a dialog: refusal.
        assert!(
            RemoveMask
                .apply_json(
                    &mut document,
                    serde_json::json!({ "id": image_id, "what_if_image": "ask" }),
                )
                .is_err()
        );
    }

    #[test]
    fn path_to_marquee_refuses_bad_mode_and_non_path_and_bad_feather() {
        let (mut document, image_id) = image_document();
        let path = document.registry.insert(square_path(0.0, 0.0, 4.0));
        // Bad mode.
        let error = PathToMarquee
            .apply_json(
                &mut document,
                serde_json::json!({ "id": path, "image": image_id, "mode": "sparkle" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }), "{error}");
        // Non-path target.
        let error = PathToMarquee
            .apply_json(
                &mut document,
                serde_json::json!({ "id": image_id, "image": image_id, "mode": "hard edge" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        // Non-image source.
        let error = PathToMarquee
            .apply_json(
                &mut document,
                serde_json::json!({ "id": path, "image": path, "mode": "hard edge" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        // Feather with a non-positive radius.
        let error = PathToMarquee
            .apply_json(
                &mut document,
                serde_json::json!({ "id": path, "image": image_id, "mode": "feather",
                                    "feather_amount": 0.0 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }), "{error}");
        // Feather with a good radius applies (and caps above 64).
        PathToMarquee
            .apply_json(
                &mut document,
                serde_json::json!({ "id": path, "image": image_id, "mode": "feather",
                                    "feather_amount": 500.0 }),
            )
            .unwrap();
        assert!(document.selection.is_some());
    }

    #[test]
    fn mask_kind_without_coverage_is_invalid_state() {
        // A group as the mask source: no rasterizable coverage.
        let (mut document, image_id) = image_document();
        let (group_id, _mask_id) = half_plane_group(&mut document, image_id);
        // Reach the coverage path through marqueeToPath's mask read:
        // put the group in as the selection source via a path-shaped
        // probe — the simplest door is PathToMarquee with the group id.
        let error = PathToMarquee
            .apply_json(
                &mut document,
                serde_json::json!({ "id": group_id, "image": image_id, "mode": "hard edge" }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
    }

    #[test]
    fn path_to_marquee_with_a_cubic_contour_stamps_the_flattened_curve() {
        let (mut document, image_id) = image_document();
        let mut path = match square_path(0.0, 0.0, 4.0) {
            ObjectData::Path(inner) => inner,
            // The test module already allows panic; this is a fixture
            // invariant, not a production path.
            _ => panic!("square_path is a path"),
        };
        // Make one segment a cubic: primaries off the corners.
        if let Some(first) = path.contours[0].points.first_mut() {
            first.is_curve = true;
            first.primary = Some(Point::new(2.0, -2.0));
            first.secondary = Some(Point::new(4.0, 2.0));
        }
        let path_id = document.registry.insert(ObjectData::Path(path));
        PathToMarquee
            .apply_json(
                &mut document,
                serde_json::json!({ "id": path_id, "image": image_id, "mode": "hard edge" }),
            )
            .unwrap();
        let mask = document.selection.as_ref().unwrap();
        assert!(mask.alpha.iter().any(|a| *a > 0), "the curve stamped");
    }
}
