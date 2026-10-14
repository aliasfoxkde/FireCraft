//! Bitmap selection + masked pixel commands (M7): marquee/lasso/wand →
//! `SelectionMask`, feather, and the masked fill/erase/move edits.
//!
//! Selections live on the document (`Document.selection`, image pixel
//! space) so commands see them through the same `&mut Document` every
//! other handler does — surfaces, journal replay, and scripts all
//! reproduce selection state for free. The mask plane always spans the
//! whole target image. Pixel ops capture every touched tile (Arc clone)
//! plus the prior mask, so undo is byte-exact even under feathered
//! coverage.

use doc::{Document, ImageObject, ObjectData, ObjectId, SelectionMask};

use super::object::{Owner, attach_cell, find_owner};
use super::retouch::{BrushOptionsPayload, brush_of, strength_of};
use crate::access;
use crate::bitmap::{assemble_rgba, gather, retile_rgba, scatter, touched_tiles};
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::mask::{
    SelectMode, WandColor, feather as feather_mask, fold, stamp_ellipse, stamp_polygon, stamp_rect,
    stamp_wand,
};
use crate::registry::CommandRegistry;

/// Registers the bitmap commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(SelectRect)?;
    registry.register(SelectEllipse)?;
    registry.register(SelectLasso)?;
    registry.register(SelectPolygon)?;
    registry.register(SelectWand)?;
    registry.register(SelectSimilar)?;
    registry.register(SelectFeather)?;
    registry.register(MoveSelectionMaskBy)?;
    registry.register(AdjustPixelSel)?;
    registry.register(SelectNone)?;
    registry.register(Fill)?;
    registry.register(Erase)?;
    registry.register(BrushPaint)?;
    registry.register(Pencil)?;
    registry.register(MovePixels)?;
    registry.register(AddImage)?;
    registry.register(AddImageViaCopy)?;
    registry.register(AddImageViaCut)
}

/// The image object named by `id`, cloned out of the registry so
/// callers can hold it across `&mut` work.
fn image_of(
    document: &Document,
    name: &'static str,
    id: ObjectId,
) -> Result<doc::ImageObject, EngineError> {
    match access::object(document, name, id)? {
        ObjectData::Image(image) => Ok(image.clone()),
        _ => Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("object {} is not a bitmap image", id.index()),
        }),
    }
}

/// The image's pixel size as `u32`s (validated against the working
/// space by `gather`/`check_editable` before any pixel flows).
fn pixel_size(image: &doc::ImageObject, name: &'static str) -> Result<(u32, u32), EngineError> {
    let (Ok(width), Ok(height)) = (
        u32::try_from(image.pixels.0.max(0)),
        u32::try_from(image.pixels.1.max(0)),
    ) else {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("image pixels {:?} are not addressable", image.pixels),
        });
    };
    if width == 0 || height == 0 {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "image has no pixels to select".to_owned(),
        });
    }
    Ok((width, height))
}

/// Parses + validates a `SelectMode` payload string.
fn mode_of(name: &'static str, mode: Option<&String>) -> Result<SelectMode, EngineError> {
    SelectMode::parse(mode.map(String::as_str)).ok_or_else(|| EngineError::InvalidState {
        name: name.to_owned(),
        detail: format!(
            "mode {:?} is not one of replace/add/subtract/intersect",
            mode.unwrap_or(&String::new())
        ),
    })
}

/// The selection a stamping command leaves behind: the fresh plane
/// folded through `mode` over the active one.
fn stamped_selection(
    document: &mut Document,
    size: (u32, u32),
    mode: SelectMode,
    stamp: impl FnOnce(&mut SelectionMask),
) {
    let mut fresh = SelectionMask::empty(size.0, size.1);
    stamp(&mut fresh);
    let active = document.selection.take();
    let folded = fold(active.as_ref(), fresh, mode);
    // A mask that covers nothing is no selection (a collapsed marquee
    // deselects, as in the original).
    document.selection = if folded.alpha.iter().all(|&a| a == 0) {
        None
    } else {
        Some(folded)
    };
}

/// Capture for a selection-changing command.
fn selection_delta(before: Option<SelectionMask>) -> Delta {
    Delta::Selection {
        before: before.map(Box::new),
    }
}

/// `bitmap.selectRect` — rectangular marquee over the image.
struct SelectRect;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectRectPayload {
    /// Target image.
    id: ObjectId,
    /// Rect in image pixel coordinates.
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    /// How to combine with the active selection.
    #[serde(default)]
    mode: Option<String>,
}

impl TypedCommand for SelectRect {
    type Payload = SelectRectPayload;

    fn name(&self) -> &'static str {
        "bitmap.selectRect"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.selectRect";
        let mode = mode_of(NAME, payload.mode.as_ref())?;
        let size = pixel_size(&image_of(document, NAME, payload.id)?, NAME)?;
        let before = document.selection.clone();
        let (x, y, width, height) = (payload.x, payload.y, payload.width, payload.height);
        stamped_selection(document, size, mode, |mask| {
            stamp_rect(mask, x, y, width, height);
        });
        Ok(selection_delta(before))
    }
}

/// `bitmap.selectEllipse` — oval marquee.
struct SelectEllipse;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectEllipsePayload {
    /// Target image.
    id: ObjectId,
    /// Bounding rect in image pixel coordinates.
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    /// How to combine with the active selection.
    #[serde(default)]
    mode: Option<String>,
}

impl TypedCommand for SelectEllipse {
    type Payload = SelectEllipsePayload;

    fn name(&self) -> &'static str {
        "bitmap.selectEllipse"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.selectEllipse";
        let mode = mode_of(NAME, payload.mode.as_ref())?;
        let size = pixel_size(&image_of(document, NAME, payload.id)?, NAME)?;
        let before = document.selection.clone();
        let (x, y, width, height) = (payload.x, payload.y, payload.width, payload.height);
        stamped_selection(document, size, mode, |mask| {
            stamp_ellipse(mask, x, y, width, height);
        });
        Ok(selection_delta(before))
    }
}

/// `bitmap.selectLasso` — polygon lasso (freehand points).
struct SelectLasso;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectLassoPayload {
    /// Target image.
    id: ObjectId,
    /// Freehand trace vertices in image pixel coordinates (closed).
    points: Vec<[f64; 2]>,
    /// How to combine with the active selection.
    #[serde(default)]
    mode: Option<String>,
}

impl TypedCommand for SelectLasso {
    type Payload = SelectLassoPayload;

    fn name(&self) -> &'static str {
        "bitmap.selectLasso"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.selectLasso";
        let mode = mode_of(NAME, payload.mode.as_ref())?;
        let size = pixel_size(&image_of(document, NAME, payload.id)?, NAME)?;
        let before = document.selection.clone();
        let points = payload.points.clone();
        stamped_selection(document, size, mode, |mask| {
            let vertices: Vec<(f64, f64)> = points.iter().map(|p| (p[0], p[1])).collect();
            stamp_polygon(mask, &vertices);
        });
        Ok(selection_delta(before))
    }
}

/// `bitmap.selectPolygon` — polygon lasso over the image.
struct SelectPolygon;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectPolygonPayload {
    /// Target image.
    id: ObjectId,
    /// Polygon vertices in image pixel coordinates.
    points: Vec<[f64; 2]>,
    /// How to combine with the active selection.
    #[serde(default)]
    mode: Option<String>,
}

impl TypedCommand for SelectPolygon {
    type Payload = SelectPolygonPayload;

    fn name(&self) -> &'static str {
        "bitmap.selectPolygon"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.selectPolygon";
        let mode = mode_of(NAME, payload.mode.as_ref())?;
        let size = pixel_size(&image_of(document, NAME, payload.id)?, NAME)?;
        let before = document.selection.clone();
        let points = payload.points.clone();
        stamped_selection(document, size, mode, |mask| {
            let vertices: Vec<(f64, f64)> = points.iter().map(|p| (p[0], p[1])).collect();
            stamp_polygon(mask, &vertices);
        });
        Ok(selection_delta(before))
    }
}

/// `bitmap.selectWand` — magic wand over the image's pixels.
struct SelectWand;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectWandPayload {
    /// Target image.
    id: ObjectId,
    /// Seed pixel in image coordinates.
    x: u32,
    y: u32,
    /// Per-channel tolerance `0..=255`.
    tolerance: u8,
    /// Flood from the seed (`true`, default) or select every matching
    /// pixel in the image (`false`).
    #[serde(default)]
    contiguous: Option<bool>,
    /// How to combine with the active selection.
    #[serde(default)]
    mode: Option<String>,
}

impl TypedCommand for SelectWand {
    type Payload = SelectWandPayload;

    fn name(&self) -> &'static str {
        "bitmap.selectWand"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.selectWand";
        let mode = mode_of(NAME, payload.mode.as_ref())?;
        let image = image_of(document, NAME, payload.id)?.clone();
        let size = pixel_size(&image, NAME)?;
        if payload.x >= size.0 || payload.y >= size.1 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "seed ({}, {}) is outside the {}x{} image",
                    payload.x, payload.y, size.0, size.1
                ),
            });
        }
        let flat = gather(&image, &document.tiles, NAME)?;
        let seed_index =
            (payload.y as u64 * u64::from(flat.width) + u64::from(payload.x)) as usize * 4;
        let seed: [u8; 4] = flat.pixels[seed_index..seed_index + 4]
            .try_into()
            .map_err(|_| EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "flat buffer shorter than its dimensions".to_owned(),
            })?;
        let contiguous = payload.contiguous.unwrap_or(true);
        let target = WandColor {
            seed,
            tolerance: payload.tolerance,
        };
        let before = document.selection.clone();
        let (x, y) = (payload.x, payload.y);
        let pixels = flat.pixels.clone();
        stamped_selection(document, size, mode, |mask| {
            stamp_wand(mask, &pixels, x, y, target, contiguous);
        });
        Ok(selection_delta(before))
    }
}

/// `bitmap.selectSimilar` — CS6's `dom.selectSimilar()`: every image
/// pixel within tolerance of the active selection's average color.
/// CS6's `edgemode`/`featherAmt` arguments are edge softening the
/// model applies separately (`bitmap.feather`); recorded deviation,
/// decision 0025. The match is global (every matching pixel joins)
/// unless `contiguous` asks for a flood from the first selected pixel.
struct SelectSimilar;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectSimilarPayload {
    /// Target image.
    id: ObjectId,
    /// Per-channel tolerance `0..=255` around the selection's average
    /// color.
    tolerance: u8,
    /// Flood from the average-colored seed region (`true`) or select
    /// every matching pixel in the image (`false`, CS6's behavior).
    #[serde(default)]
    contiguous: Option<bool>,
    /// How to combine with the active selection.
    #[serde(default)]
    mode: Option<String>,
}

impl TypedCommand for SelectSimilar {
    type Payload = SelectSimilarPayload;

    fn name(&self) -> &'static str {
        "bitmap.selectSimilar"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.selectSimilar";
        let mode = mode_of(NAME, payload.mode.as_ref())?;
        let Some(active) = document.selection.clone() else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "selectSimilar needs an active pixel selection".to_owned(),
            });
        };
        let image = image_of(document, NAME, payload.id)?.clone();
        let size = pixel_size(&image, NAME)?;
        if (active.width, active.height) != size {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "selection plane {}x{} does not span the {}x{} image",
                    active.width, active.height, size.0, size.1
                ),
            });
        }
        let flat = gather(&image, &document.tiles, NAME)?;
        // Channel average over the selected pixels (the mask's alpha
        // weights membership; zero-alpha pixels do not count).
        let mut sum = [0u64; 4];
        let mut count = 0u64;
        for (index, alpha) in active.alpha.iter().enumerate() {
            if *alpha == 0 {
                continue;
            }
            let pixel = &flat.pixels[index * 4..index * 4 + 4];
            for channel in 0..4 {
                sum[channel] += u64::from(pixel[channel]);
            }
            count += 1;
        }
        if count == 0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "selectSimilar over an empty selection".to_owned(),
            });
        }
        let seed: [u8; 4] =
            std::array::from_fn(|channel| ((sum[channel] + count / 2) / count).min(255) as u8);
        let target = WandColor {
            seed,
            tolerance: payload.tolerance,
        };
        let contiguous = payload.contiguous.unwrap_or(false);
        let before = document.selection.clone();
        let pixels = flat.pixels.clone();
        stamped_selection(document, size, mode, |mask| {
            stamp_wand(mask, &pixels, 0, 0, target, contiguous);
        });
        Ok(selection_delta(before))
    }
}

/// `bitmap.feather` — soften the active selection's edge.
struct SelectFeather;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectFeatherPayload {
    /// Feather radius in pixels; values above 64 clamp.
    radius: u8,
}

impl TypedCommand for SelectFeather {
    type Payload = SelectFeatherPayload;

    fn name(&self) -> &'static str {
        "bitmap.feather"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.feather";
        let Some(active) = document.selection.clone() else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "no active selection to feather".to_owned(),
            });
        };
        if payload.radius == 0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "radius 0 would be a no-op; use bitmap.selectNone".to_owned(),
            });
        }
        let mut feathered = active.clone();
        feather_mask(&mut feathered, u32::from(payload.radius));
        document.selection = Some(feathered);
        Ok(selection_delta(Some(active)))
    }
}

/// Chamfer distance field (3-4 weights, ×3 an axial step): every
/// pixel's distance to the nearest pixel where `present` holds.
/// Out-of-image neighbors contribute nothing — the image edge is not
/// a selection boundary, so a full plane has no unselected pixels to
/// contract from. Two passes, O(pixels), radius-independent.
fn distance_field(mask: &SelectionMask, present: bool) -> Vec<u64> {
    const INF: u64 = u64::MAX / 4;
    let (w, h) = (mask.width as usize, mask.height as usize);
    let mut dist = vec![INF; w * h];
    for (index, &coverage) in mask.alpha.iter().enumerate() {
        if (coverage != 0) == present {
            dist[index] = 0;
        }
    }
    let relax = |dist: &mut Vec<u64>, x: usize, y: usize, neighbors: &[(isize, isize, u64)]| {
        let index = y * w + x;
        if dist[index] == 0 {
            return;
        }
        let mut best = dist[index];
        for (dx, dy, cost) in neighbors {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                continue;
            }
            let nd = dist[ny as usize * w + nx as usize];
            if nd < INF {
                best = best.min(nd + cost);
            }
        }
        dist[index] = best;
    };
    const FORWARD: [(isize, isize, u64); 4] = [(-1, 0, 3), (0, -1, 3), (-1, -1, 4), (1, -1, 4)];
    const BACKWARD: [(isize, isize, u64); 4] = [(1, 0, 3), (0, 1, 3), (1, 1, 4), (-1, 1, 4)];
    for y in 0..h {
        for x in 0..w {
            relax(&mut dist, x, y, &FORWARD);
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            relax(&mut dist, x, y, &BACKWARD);
        }
    }
    dist
}

/// Chamfer radius for a pixel radius: an axial step is 3 field units.
fn chamfer(radius: u64) -> u64 {
    radius * 3
}

fn expand_mask(mask: &SelectionMask, radius: u64) -> SelectionMask {
    let dist = distance_field(mask, true);
    let mut out = mask.clone();
    for (index, d) in dist.iter().enumerate() {
        out.alpha[index] = if *d <= chamfer(radius) { 255 } else { 0 };
    }
    out
}

fn contract_mask(mask: &SelectionMask, radius: u64) -> SelectionMask {
    let dist = distance_field(mask, false);
    let mut out = mask.clone();
    for (index, d) in dist.iter().enumerate() {
        out.alpha[index] = if mask.alpha[index] != 0 && *d > chamfer(radius) {
            255
        } else {
            0
        };
    }
    out
}

fn border_mask(mask: &SelectionMask, radius: u64) -> SelectionMask {
    let dist = distance_field(mask, false);
    let mut out = mask.clone();
    for (index, d) in dist.iter().enumerate() {
        out.alpha[index] = if mask.alpha[index] != 0 && *d <= chamfer(radius) {
            255
        } else {
            0
        };
    }
    out
}

/// Morphological smooth: opening (contract then expand) removes
/// specks, closing (expand then contract) fills pinholes — the same
/// radius all four passes.
fn smooth_mask(mask: &SelectionMask, radius: u64) -> SelectionMask {
    let opened = expand_mask(&contract_mask(mask, radius), radius);
    contract_mask(&expand_mask(&opened, radius), radius)
}

/// `bitmap.moveSelectionMaskBy` — CS6 `dom.moveSelectionMaskBy(delta)`:
/// "Moves the current pixel mask by the specified amount. If there is
/// no pixel selection, an exception is thrown." Only the coverage
/// plane moves; image pixels are untouched (`bitmap.movePixels` moves
/// the pixels under it).
struct MoveSelectionMaskBy;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct MoveSelectionMaskByPayload {
    /// Coverage-plane shift; positive moves right/down.
    delta: MoveDeltaPayload,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct MoveDeltaPayload {
    x: i64,
    y: i64,
}

impl TypedCommand for MoveSelectionMaskBy {
    type Payload = MoveSelectionMaskByPayload;

    fn name(&self) -> &'static str {
        "bitmap.moveSelectionMaskBy"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.moveSelectionMaskBy";
        let Some(active) = document.selection.clone() else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "there is no pixel selection".to_owned(),
            });
        };
        let (dx, dy) = (payload.delta.x, payload.delta.y);
        if dx == 0 && dy == 0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "a zero delta moves nothing".to_owned(),
            });
        }
        if dx.abs() > 65535 || dy.abs() > 65535 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "delta exceeds the 65535-plane bound".to_owned(),
            });
        }
        let (w, h) = (active.width as usize, active.height as usize);
        let mut moved = SelectionMask::empty(active.width, active.height);
        for y in 0..h {
            let source_y = y as i64 - dy;
            if source_y < 0 || source_y >= h as i64 {
                continue;
            }
            for x in 0..w {
                let source_x = x as i64 - dx;
                if source_x < 0 || source_x >= w as i64 {
                    continue;
                }
                moved.alpha[y * w + x] = active.alpha[source_y as usize * w + source_x as usize];
            }
        }
        let before = document.selection.replace(moved);
        Ok(selection_delta(before))
    }
}

/// `bitmap.adjustPixelSel` — CS6
/// `dom.selectAdjustPixelSel(whatToDo, amount)`: "Expands or reduces
/// the pixel selection by the specified number of pixels, selects a
/// border of pixels, or smooths the edge of the pixel selection."
/// Morphology over the coverage plane with a 3-4 chamfer distance
/// field: expand/contract are threshold moves of the selected/unselected
/// boundary, border keeps the edge band, smooth is an opening then a
/// closing. The binary projection (`alpha != 0`) is what adjusts —
/// feather softness collapses to hard coverage (recorded).
struct AdjustPixelSel;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AdjustPixelSelPayload {
    /// The CS6 `whatToDo` words.
    what: AdjustKind,
    /// Pixels to adjust by; 1..=65535.
    amount: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum AdjustKind {
    Expand,
    Contract,
    Border,
    Smooth,
}

impl TypedCommand for AdjustPixelSel {
    type Payload = AdjustPixelSelPayload;

    fn name(&self) -> &'static str {
        "bitmap.adjustPixelSel"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.adjustPixelSel";
        let Some(active) = document.selection.clone() else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "there is no pixel selection".to_owned(),
            });
        };
        if !(1..=65535).contains(&payload.amount) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("amount {} outside 1..=65535", payload.amount),
            });
        }
        let radius = u64::try_from(payload.amount).unwrap_or(0);
        let adjusted = match payload.what {
            AdjustKind::Expand => expand_mask(&active, radius),
            AdjustKind::Contract => contract_mask(&active, radius),
            AdjustKind::Border => border_mask(&active, radius),
            AdjustKind::Smooth => smooth_mask(&active, radius),
        };
        let before = document.selection.replace(adjusted);
        Ok(selection_delta(before))
    }
}

/// `bitmap.selectNone` — clear the active selection.
struct SelectNone;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
struct SelectNonePayload {}

impl TypedCommand for SelectNone {
    type Payload = SelectNonePayload;

    fn name(&self) -> &'static str {
        "bitmap.selectNone"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        let before = document.selection.take();
        Ok(selection_delta(before))
    }
}

/// Captures the tiles `image`'s active selection touches, borrows the
/// flat working buffer through `edit`, scatters the result back, and
/// returns the combined delta. Shared by fill/erase/move.
pub(crate) fn masked_pixel_edit(
    document: &mut Document,
    name: &'static str,
    id: ObjectId,
    edit: impl FnOnce(&mut [u8], (u32, u32), &SelectionMask) -> Result<(), EngineError>,
) -> Result<Delta, EngineError> {
    let image = image_of(document, name, id)?.clone();
    let size = pixel_size(&image, name)?;
    let mask = match &document.selection {
        Some(mask) if mask.width == size.0 && mask.height == size.1 => mask.clone(),
        Some(mask) => {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!(
                    "selection spans {}x{} but the image is {}x{}",
                    mask.width, mask.height, size.0, size.1
                ),
            });
        }
        // No selection: the whole image is the mask (Fireworks fills
        // everything when nothing is selected).
        None => {
            let mut full = SelectionMask::empty(size.0, size.1);
            for slot in full.alpha.iter_mut() {
                *slot = 255;
            }
            full
        }
    };
    let touched = touched_tiles(&image, &mask);
    if touched.is_empty() {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "the selection covers no loaded tile pixels".to_owned(),
        });
    }
    let before_tiles: Vec<(u32, raster::Tile)> = touched
        .iter()
        .filter_map(|tile_id| {
            document
                .tiles
                .get(*tile_id)
                .map(|tile| (*tile_id, tile.clone()))
        })
        .collect();
    let mut flat = gather(&image, &document.tiles, name)?;
    edit(&mut flat.pixels, size, &mask)?;
    scatter(&flat, &image, &mut document.tiles, &touched, name)?;
    Ok(Delta::Tiles {
        id,
        before: before_tiles,
    })
}

/// Blends one straight pixel toward `target` by coverage/255 (the
/// feathered-fill rule), writing into `pixel`.
pub(crate) fn blend_pixel(pixel: &mut [u8; 4], target: [u8; 4], coverage: u8) {
    if coverage == 255 {
        *pixel = target;
        return;
    }
    if coverage == 0 {
        return;
    }
    for (slot, value) in pixel.iter_mut().zip(target) {
        *slot = ((u16::from(*slot) * (255 - u16::from(coverage))
            + u16::from(value) * u16::from(coverage)
            + 127)
            / 255) as u8;
    }
}

/// `bitmap.fill` — fill the selection (whole image when nothing is
/// selected) with an ARGB color at full strength, feather-scaled by
/// the mask coverage.
struct Fill;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct FillPayload {
    /// Target image.
    id: ObjectId,
    /// Fill color as ARGB raw int (same convention as
    /// `object.setFill`).
    color_raw: i64,
}

impl TypedCommand for Fill {
    type Payload = FillPayload;

    fn name(&self) -> &'static str {
        "bitmap.fill"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.fill";
        // Raw ARGB: the low 32 bits are the color, however the file
        // signed them (−1 = opaque white).
        let argb = payload.color_raw as u32;
        let color: [u8; 4] = [
            ((argb >> 16) & 0xFF) as u8,
            ((argb >> 8) & 0xFF) as u8,
            (argb & 0xFF) as u8,
            ((argb >> 24) & 0xFF) as u8,
        ];
        masked_pixel_edit(document, NAME, payload.id, |pixels, _size, mask| {
            for (index, &coverage) in mask.alpha.iter().enumerate() {
                if coverage == 0 {
                    continue;
                }
                let mut pixel: [u8; 4] =
                    pixels[index * 4..index * 4 + 4].try_into().map_err(|_| {
                        EngineError::InvalidState {
                            name: NAME.to_owned(),
                            detail: "flat buffer shorter than its mask".to_owned(),
                        }
                    })?;
                blend_pixel(&mut pixel, color, coverage);
                pixels[index * 4..index * 4 + 4].copy_from_slice(&pixel);
            }
            Ok::<(), EngineError>(())
        })
    }
}

/// `bitmap.erase` — reduce alpha under the selection (fully erase where
/// coverage is full; feathered coverage scales the alpha down).
struct Erase;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ErasePayload {
    /// Target image.
    id: ObjectId,
}

impl TypedCommand for Erase {
    type Payload = ErasePayload;

    fn name(&self) -> &'static str {
        "bitmap.erase"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.erase";
        masked_pixel_edit(document, NAME, payload.id, |pixels, _size, mask| {
            for (index, &coverage) in mask.alpha.iter().enumerate() {
                if coverage == 0 {
                    continue;
                }
                let alpha = &mut pixels[index * 4 + 3];
                *alpha = ((u16::from(*alpha) * (255 - u16::from(coverage))) / 255) as u8;
            }
            Ok::<(), EngineError>(())
        })
    }
}

/// One paint dab's inputs: the validated stamp plus its strength,
/// pressure, and straight-RGBA color.
struct PaintDab {
    brush: crate::brush::Brush,
    strength: u32,
    pressure: u32,
    color: [u8; 4],
}

/// Shared paint fold: stamp the dab over the image, scaled by
/// strength × pressure × mask coverage.
fn paint_dab(
    pixels: &mut [u8],
    size: (u32, u32),
    mask: &SelectionMask,
    dab: &PaintDab,
    name: &'static str,
) -> Result<(), EngineError> {
    for py in 0..size.1 {
        for px in 0..size.0 {
            let index = py as usize * size.0 as usize + px as usize;
            let eff = dab
                .brush
                .effect(px, py, dab.strength, dab.pressure, mask.alpha[index]);
            if eff == 0 {
                continue;
            }
            let mut pixel: [u8; 4] = pixels[index * 4..index * 4 + 4].try_into().map_err(|_| {
                EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: "flat buffer shorter than its mask".to_owned(),
                }
            })?;
            blend_pixel(&mut pixel, dab.color, eff);
            pixels[index * 4..index * 4 + 4].copy_from_slice(&pixel);
        }
    }
    Ok(())
}

/// Decodes the ARGB raw int the fill/paint payloads carry (the low 32
/// bits are the color, however the file signed them).
fn argb_color(color_raw: i64) -> [u8; 4] {
    let argb = color_raw as u32;
    [
        ((argb >> 16) & 0xFF) as u8,
        ((argb >> 8) & 0xFF) as u8,
        (argb & 0xFF) as u8,
        ((argb >> 24) & 0xFF) as u8,
    ]
}

/// `bitmap.brush` — paint a brush dab: the Edit Brush dialog's slots
/// (Size diameter, Edge softness, Shape round/square, Aspect, Angle)
/// plus the per-dab Sensitivity pressure, over the M7 integer-exact
/// stamp. A stroke is a journal of dabs.
struct BrushPaint;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct BrushPaintPayload {
    /// Target image.
    id: ObjectId,
    /// Brush center in image pixel coordinates.
    x: f64,
    y: f64,
    /// Brush diameter in pixels (the dialog's Size slot), 1..=1024.
    size: u32,
    /// Paint color as ARGB raw int (same convention as
    /// `bitmap.fill`).
    color_raw: i64,
    /// Effect strength percent (1..=100); default 100.
    #[serde(default)]
    strength: Option<u8>,
    /// The dialog's Edge/Shape/Aspect/Angle/Pressure slots, all
    /// defaulted.
    #[serde(default)]
    brush: BrushOptionsPayload,
}

impl TypedCommand for BrushPaint {
    type Payload = BrushPaintPayload;

    fn name(&self) -> &'static str {
        "bitmap.brush"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.brush";
        let options = brush_of(NAME, payload.x, payload.y, payload.size, &payload.brush)?;
        let strength = strength_of(NAME, payload.strength.unwrap_or(100))?;
        let color = argb_color(payload.color_raw);
        let dab = PaintDab {
            brush: options.brush,
            strength,
            pressure: options.pressure,
            color,
        };
        masked_pixel_edit(document, NAME, payload.id, |pixels, size, mask| {
            paint_dab(pixels, size, mask, &dab, NAME)
        })
    }
}

/// `bitmap.pencil` — the hard one-pixel dab: no size/edge/shape slots
/// (the pencil is definitionally a hard single-pixel stamp), only the
/// per-dab pressure.
struct Pencil;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PencilPayload {
    /// Target image.
    id: ObjectId,
    /// The dab point in image pixel coordinates.
    x: f64,
    y: f64,
    /// Pencil color as ARGB raw int.
    color_raw: i64,
    /// Effect strength percent (1..=100); default 100.
    #[serde(default)]
    strength: Option<u8>,
    /// Per-dab sensitivity percent 1..=100; default 100.
    #[serde(default)]
    pressure: Option<u8>,
}

impl TypedCommand for Pencil {
    type Payload = PencilPayload;

    fn name(&self) -> &'static str {
        "bitmap.pencil"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.pencil";
        let options = brush_of(
            NAME,
            payload.x,
            payload.y,
            1,
            &BrushOptionsPayload {
                edge: Some(0),
                ..BrushOptionsPayload::default()
            },
        )?;
        let strength = strength_of(NAME, payload.strength.unwrap_or(100))?;
        let pressure = match payload.pressure {
            Some(0) => {
                return Err(EngineError::BadPayload {
                    name: NAME.to_owned(),
                    detail: "brush pressure 0 would be a no-op".to_owned(),
                });
            }
            Some(value) => u32::from(value),
            None => 100,
        };
        let color = argb_color(payload.color_raw);
        let dab = PaintDab {
            brush: options.brush,
            strength,
            pressure,
            color,
        };
        masked_pixel_edit(document, NAME, payload.id, |pixels, size, mask| {
            paint_dab(pixels, size, mask, &dab, NAME)
        })
    }
}

/// `bitmap.movePixels` — translate the masked block by `(dx, dy)`;
/// sources clear, destinations overwrite, edges clip. Coverage ≥ 128
/// counts as moved (a feathered edge does not split a pixel in half).
struct MovePixels;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct MovePixelsPayload {
    /// Target image.
    id: ObjectId,
    /// Pixel offset.
    dx: i64,
    dy: i64,
}

impl TypedCommand for MovePixels {
    type Payload = MovePixelsPayload;

    fn name(&self) -> &'static str {
        "bitmap.movePixels"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.movePixels";
        let (dx, dy) = (payload.dx, payload.dy);
        masked_pixel_edit(document, NAME, payload.id, |pixels, size, mask| {
            let moved = pixels.to_vec();
            let width = u64::from(size.0);
            let height = u64::from(size.1);
            // Pass 1: clear sources under the mask.
            for (index, &coverage) in mask.alpha.iter().enumerate() {
                if coverage >= 128 {
                    pixels[index * 4..index * 4 + 4].fill(0);
                }
            }
            // Pass 2: write moved pixels (from the pre-move copy) at
            // their offsets, clipping the frame.
            for py in 0..height {
                for px in 0..width {
                    let index = (py * width + px) as usize;
                    if mask.alpha[index] < 128 {
                        continue;
                    }
                    let (Ok(tx), Ok(ty)) = (
                        i64::try_from(px).map(|v| v + dx),
                        i64::try_from(py).map(|v| v + dy),
                    ) else {
                        continue;
                    };
                    if tx < 0 || ty < 0 || tx >= i64::from(size.0) || ty >= i64::from(size.1) {
                        continue;
                    }
                    let target = (ty as u64 * width + tx as u64) as usize * 4;
                    pixels[target..target + 4].copy_from_slice(&moved[index * 4..index * 4 + 4]);
                }
            }
            Ok::<(), EngineError>(())
        })
    }
}

/// `bitmap.addImage` — CS6 `dom.AddNewImage(boundRectangle,
/// bEnterPaintMode)`: a new empty (transparent) image. Bounds larger
/// than the document constrain to it (the payload rectangle is
/// intersected with the page canvas); paint-mode posture is UI state
/// the engine does not carry (decision 0037, deviation 1).
struct AddImage;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddImagePayload {
    /// Bounds origin.
    x: f64,
    y: f64,
    /// Bounds extent.
    width: f64,
    height: f64,
    /// Placement.
    page: usize,
    layer: usize,
    cell: usize,
    /// `OBNs` — object name.
    name: Option<String>,
}

impl TypedCommand for AddImage {
    type Payload = AddImagePayload;

    fn name(&self) -> &'static str {
        "bitmap.addImage"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.addImage";
        if ![payload.x, payload.y, payload.width, payload.height]
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "bounds are not finite".to_owned(),
            });
        }
        if payload.width <= 0.0 || payload.height <= 0.0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "bounds extent {:?} x {:?} has no area",
                    payload.width, payload.height
                ),
            });
        }
        let canvas = &access::page(document, NAME, payload.page)?.info.canvas;
        // CS6: a rectangle larger than the document is constrained to
        // the document size — the intersection with the canvas.
        let x0 = payload.x.max(canvas.x);
        let y0 = payload.y.max(canvas.y);
        let x1 = (payload.x + payload.width).min(canvas.x + canvas.width);
        let y1 = (payload.y + payload.height).min(canvas.y + canvas.height);
        let (left, top) = (x0.round() as i64, y0.round() as i64);
        let (right, bottom) = (x1.round() as i64, y1.round() as i64);
        if right <= left || bottom <= top {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "bounds do not intersect the document canvas".to_owned(),
            });
        }
        // Pre-flight the placement cell before minting anything: a
        // refusal must leave the document untouched, and the attach
        // below runs after tiles and the registry slot exist.
        access::cell(document, NAME, payload.page, payload.layer, payload.cell)?;
        let (width, height) = (right - left, bottom - top);
        let empty = vec![0u8; (width * height * 4) as usize];
        // Fresh zero tiles, not matte slots: a matte-only grid is
        // invisible to gather/scatter, so the bitmap could never be
        // painted (decision 0037).
        let (tiles, minted) = crate::bitmap::retile_rgba(document, &empty, width, height);
        let id = document.registry.insert(ObjectData::Image(ImageObject {
            location: geom::Point {
                x: left as f64,
                y: top as f64,
            },
            offset: geom::Point::ZERO,
            pixels: (width, height),
            tile_size: i64::from(raster::TILE_SIDE as u32),
            tiles,
            name: payload.name.clone(),
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }));
        let placed = attach_cell(
            document,
            NAME,
            payload.page,
            payload.layer,
            payload.cell,
            id,
            None,
        )?;
        Ok(Delta::Group(vec![
            placed,
            Delta::Object {
                id,
                inverse: ObjectInverse::Created,
            },
            Delta::Bank { added: minted },
        ]))
    }
}

/// `bitmap.addImageViaCopy` — CS6 `dom.addNewImageViaCopy()`: the
/// paint-mode selection becomes a new image placed directly above the
/// active bitmap. The selection plane must span the source's pixel
/// grid (the marquee contract of every `bitmap.*` pixel command).
struct AddImageViaCopy;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AddImageViaSourcePayload {
    /// The active bitmap the selection reads from.
    source: ObjectId,
}

impl TypedCommand for AddImageViaCopy {
    type Payload = AddImageViaSourcePayload;

    fn name(&self) -> &'static str {
        "bitmap.addImageViaCopy"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.addImageViaCopy";
        let image = image_of(document, NAME, payload.source)?;
        let mask = spanning_selection(document, &image, NAME)?;
        let mut scaled = assemble_rgba(document, &image, NAME)?;
        scale_by_coverage(&mut scaled, &mask);
        let owner = above_source(document, payload.source, NAME)?;
        let (tiles, minted) = retile_rgba(document, &scaled, image.pixels.0, image.pixels.1);
        // A new bitmap: the source's geometry, default paint state
        // (decision 0037).
        let id = document.registry.insert(ObjectData::Image(ImageObject {
            location: image.location,
            offset: image.offset,
            pixels: image.pixels,
            tile_size: image.tile_size,
            tiles,
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }));
        let placed = attach_cell(
            document,
            NAME,
            owner.0,
            owner.1,
            owner.2,
            id,
            Some(owner.3 + 1),
        )?;
        Ok(Delta::Group(vec![
            placed,
            Delta::Object {
                id,
                inverse: ObjectInverse::Created,
            },
            Delta::Bank { added: minted },
        ]))
    }
}

/// `bitmap.addImageViaCut` — CS6 `dom.addNewImageViaCut()`: the
/// selection also clears from the source bitmap, over fresh tile ids
/// (shared bank slots are never edited in place — decision 0037); the
/// orphaned originals are the recorded bank-GC debt.
struct AddImageViaCut;

impl TypedCommand for AddImageViaCut {
    type Payload = AddImageViaSourcePayload;

    fn name(&self) -> &'static str {
        "bitmap.addImageViaCut"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "bitmap.addImageViaCut";
        let image = image_of(document, NAME, payload.source)?;
        let mask = spanning_selection(document, &image, NAME)?;
        // Capture before mutating (the standing law): the source's
        // registry slot rides the delta whole.
        let before = access::object(document, NAME, payload.source)?.clone();
        let assembled = assemble_rgba(document, &image, NAME)?;
        let mut scaled = assembled.clone();
        scale_by_coverage(&mut scaled, &mask);
        let mut cleared = assembled;
        clear_by_coverage(&mut cleared, &mask);
        let owner = above_source(document, payload.source, NAME)?;
        let (source_tiles, mut minted) =
            retile_rgba(document, &cleared, image.pixels.0, image.pixels.1);
        let (tiles, new_minted) = retile_rgba(document, &scaled, image.pixels.0, image.pixels.1);
        minted.extend(new_minted);
        let id = document.registry.insert(ObjectData::Image(ImageObject {
            location: image.location,
            offset: image.offset,
            pixels: image.pixels,
            tile_size: image.tile_size,
            tiles,
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }));
        let Some(ObjectData::Image(slot)) = document.registry.get_mut(payload.source) else {
            return Err(EngineError::MissingTarget {
                name: NAME.to_owned(),
                detail: format!(
                    "source {} (generation {}) vanished mid-command",
                    payload.source.index(),
                    payload.source.generation()
                ),
            });
        };
        slot.tiles = source_tiles;
        let placed = attach_cell(
            document,
            NAME,
            owner.0,
            owner.1,
            owner.2,
            id,
            Some(owner.3 + 1),
        )?;
        Ok(Delta::Group(vec![
            placed,
            Delta::Object {
                id: payload.source,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(before),
                },
            },
            Delta::Object {
                id,
                inverse: ObjectInverse::Created,
            },
            Delta::Bank { added: minted },
        ]))
    }
}

/// The pixel selection a ViaCopy/ViaCut needs: present, and spanning
/// the source image's grid (the `bitmap.*` marquee contract).
fn spanning_selection(
    document: &Document,
    image: &ImageObject,
    name: &'static str,
) -> Result<SelectionMask, EngineError> {
    let Some(mask) = document.selection.as_ref() else {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "there is no pixel selection".to_owned(),
        });
    };
    if (i64::from(mask.width), i64::from(mask.height)) != image.pixels {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!(
                "the selection plane spans {}x{}, the bitmap is {:?}",
                mask.width, mask.height, image.pixels
            ),
        });
    }
    Ok(mask.clone())
}

/// The cell placement "directly above the active bitmap": the
/// source's `(page, layer, cell, position)`. Group-nested sources
/// refuse — a group has no cell to sit above in.
fn above_source(
    document: &Document,
    source: ObjectId,
    name: &'static str,
) -> Result<(usize, usize, usize, usize), EngineError> {
    match find_owner(document, source) {
        Some(Owner::Cell {
            page,
            layer,
            cell,
            position,
        }) => Ok((page, layer, cell, position)),
        Some(Owner::Group { id, .. }) => Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!(
                "source {} is nested in group {} (generation {}); there is no cell to sit above",
                source.index(),
                id.index(),
                id.generation()
            ),
        }),
        None => Err(EngineError::MissingTarget {
            name: name.to_owned(),
            detail: format!(
                "object {} (generation {}) is referenced by no cell or group",
                source.index(),
                source.generation()
            ),
        }),
    }
}

/// Scales every pixel's channels by the mask coverage — compositing's
/// source-scale in the premultiplied domain (decision 0037).
fn scale_by_coverage(pixels: &mut [u8], mask: &SelectionMask) {
    for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
        let coverage = u32::from(mask.alpha[index]);
        if coverage == 255 {
            continue;
        }
        if coverage == 0 {
            pixel.fill(0);
            continue;
        }
        for channel in pixel.iter_mut() {
            *channel = ((u32::from(*channel) * coverage + 127) / 255) as u8;
        }
    }
}

/// Clears every pixel under any nonzero coverage (the cut is binary
/// at the selection edge, as in the original).
fn clear_by_coverage(pixels: &mut [u8], mask: &SelectionMask) {
    for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
        if mask.alpha[index] > 0 {
            pixel.fill(0);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::command::Command;
    use serde_json::json;

    /// A 4×4 image with one loaded tile: opaque red on even columns,
    /// opaque blue on odd columns, row-major.
    fn checker_document() -> (Document, ObjectId) {
        let mut document = Document::default();
        let image = document
            .registry
            .insert(ObjectData::Image(doc::ImageObject {
                location: geom::Point::ZERO,
                offset: geom::Point::ZERO,
                pixels: (4, 4),
                tile_size: 128,
                tiles: vec![doc::TileRef::Tile(0)],
                name: None,
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                effects: Vec::new(),
            }));
        let mut buffer = [0u8; raster::TILE_BYTES];
        for py in 0usize..4 {
            for px in 0usize..4 {
                let color: [u8; 4] = if px % 2 == 0 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 255, 255]
                };
                buffer[(py * 128 + px) * 4..(py * 128 + px) * 4 + 4].copy_from_slice(&color);
            }
        }
        document.tiles.insert(0, raster::Tile::from_rgba8(buffer));
        (document, image)
    }

    fn flat_pixel(document: &Document, x: u32, y: u32) -> [u8; 4] {
        // Straight-alpha view via the compose-independent tile read.
        let tile = document.tiles.get(0).unwrap().pixels();
        let premult: [u8; 4] = tile[(y as usize * 128 + x as usize) * 4..][..4]
            .try_into()
            .unwrap();
        match premult[3] {
            0 | 255 => premult,
            a => {
                let mut straight = premult;
                for channel in &mut straight[..3] {
                    *channel =
                        ((u16::from(*channel) * 255 + u16::from(a) / 2) / u16::from(a)) as u8;
                }
                straight
            }
        }
    }

    #[test]
    fn rect_then_fill_and_undo_round_trip() {
        let (mut document, image) = checker_document();
        let mut blank = document.tiles.get(0).cloned().unwrap();
        blank
            .pixels_mut()
            .copy_from_slice(&[0u8; raster::TILE_BYTES]);
        document.tiles.insert(0, blank);

        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 1, "y": 1, "width": 2, "height": 2 }),
            )
            .unwrap();
        let mask = document.selection.clone().unwrap();
        let selected: Vec<usize> = mask
            .alpha
            .iter()
            .enumerate()
            .filter(|(_, value)| **value == 255)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(selected, vec![5, 6, 9, 10], "centers of the 2×2 marquee");

        let delta = Fill
            .apply_json(
                &mut document,
                json!({ "id": image, "color_raw": 0xFF_00_FF_00i64 }),
            )
            .unwrap();
        for &index in &selected {
            assert_eq!(
                flat_pixel(&document, (index as u32) % 4, (index as u32) / 4),
                [0, 255, 0, 255]
            );
        }
        assert_eq!(
            flat_pixel(&document, 0, 0),
            [0, 0, 0, 0],
            "outside the marquee stays untouched"
        );

        delta.apply_inverse(&mut document).unwrap();
        for &index in &selected {
            assert_eq!(
                flat_pixel(&document, (index as u32) % 4, (index as u32) / 4),
                [0, 0, 0, 0]
            );
        }
        // Selection survives the pixel undo (it was not part of it).
        assert!(document.selection.is_some());
    }

    #[test]
    fn intersect_mode_keeps_the_overlap() {
        let (mut document, image) = checker_document();
        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 3, "height": 4, "mode": "add" }),
            )
            .unwrap();
        SelectRect.apply_json(
            &mut document,
            json!({ "id": image, "x": 1, "y": 1, "width": 3, "height": 3, "mode": "intersect" }),
        )
        .unwrap();
        let mask = document.selection.clone().unwrap();
        let selected: Vec<usize> = mask
            .alpha
            .iter()
            .enumerate()
            .filter(|(_, value)| **value == 255)
            .map(|(i, _)| i)
            .collect();
        // A 3-wide × 4-tall strip crossed with a 3×3 offset block
        // keeps only the shared columns/rows.
        assert_eq!(selected, vec![5, 6, 9, 10, 13, 14]);
        // On a feathered mask intersect is the product: inside the
        // fresh rect the feathered value survives unchanged, outside
        // it zeroes — not the max an union would give.
        SelectFeather
            .apply_json(&mut document, json!({ "radius": 1 }))
            .unwrap();
        let feathered = document.selection.clone().unwrap();
        SelectRect.apply_json(
            &mut document,
            json!({ "id": image, "x": 1, "y": 1, "width": 3, "height": 3, "mode": "intersect" }),
        )
        .unwrap();
        let twice = document.selection.clone().unwrap();
        for (index, value) in twice.alpha.iter().enumerate() {
            let (px, py) = ((index as u32) % 4, (index as u32) / 4);
            let inside = (1..4).contains(&px) && (1..4).contains(&py);
            if inside {
                assert_eq!(
                    *value, feathered.alpha[index],
                    "pixel {index}: the feathered value rides through"
                );
            } else {
                assert_eq!(*value, 0, "pixel {index}: outside zeroes");
            }
        }
    }

    #[test]
    fn unknown_modes_are_refused_everywhere() {
        let (mut document, image) = checker_document();
        let error = SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0, "mode": "xor" }),
            )
            .unwrap_err();
        assert!(error.to_string().contains("replace/add/subtract/intersect"));
        let error = SelectPolygon
            .apply_json(
                &mut document,
                json!({ "id": image, "points": [[0.0, 0.0]], "mode": "xor" }),
            )
            .unwrap_err();
        assert!(error.to_string().contains("replace/add/subtract/intersect"));
    }

    #[test]
    fn polygon_select_stamps_and_undo_restores() {
        let (mut document, image) = checker_document();
        // Two vertices do not enclose an area; three do.
        SelectPolygon
            .apply_json(
                &mut document,
                json!({ "id": image, "points": [[0.0, 0.0], [4.0, 0.0]] }),
            )
            .unwrap();
        assert!(
            document.selection.is_none(),
            "degenerate polygon selects nothing"
        );

        let delta = SelectPolygon
            .apply_json(
                &mut document,
                json!({ "id": image, "points": [[1.0, 1.0], [3.0, 1.0], [2.0, 3.0]] }),
            )
            .unwrap();
        let mask = document.selection.clone().unwrap();
        assert!(mask.alpha.contains(&255), "the triangle has area");

        delta.apply_inverse(&mut document).unwrap();
        assert!(document.selection.is_none(), "undo takes the polygon back");
    }

    #[test]
    fn brush_paints_a_hard_dab_and_undo_is_byte_exact() {
        let (mut document, image) = checker_document();
        // Blank the checker so the dab reads clean.
        let mut blank = document.tiles.get(0).cloned().unwrap();
        blank
            .pixels_mut()
            .copy_from_slice(&[0u8; raster::TILE_BYTES]);
        document.tiles.insert(0, blank);

        let before = serde_json::to_vec(&document).unwrap();
        let delta = BrushPaint
            .apply_json(
                &mut document,
                json!({
                    "id": image, "x": 2.0, "y": 2.0, "size": 4,
                    "color_raw": 0xFFFF0000u64 as i64,
                    "brush": { "edge": 0 }
                }),
            )
            .unwrap();
        // Center (2,1): distance 1.12 < radius 2 — full paint red
        // (ARGB 0xFF0000FF = r 255 g 0 b 0 a 255).
        assert_eq!(flat_pixel(&document, 2, 1), [255, 0, 0, 255]);
        // Corner (0,0): offset (-1.5,-1.5), distance 2.12 > 2 —
        // untouched.
        assert_eq!(flat_pixel(&document, 0, 0), [0, 0, 0, 0]);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "undo restores the pre-dab bytes exactly"
        );
    }

    #[test]
    fn brush_edge_softens_and_square_reaches_the_corner() {
        let (mut document, image) = checker_document();
        // Center (1.3, 1.3), size 6 (radius 3): pixel (2,2) sits in
        // the falloff band (offset 1.2,1.2 → d 0.57) and pixel (3,3)
        // sits outside the circle (offset 2.2,2.2 → d 1.04) but
        // inside the square's Chebyshev rim (0.73).
        let mut blank = document.tiles.get(0).cloned().unwrap();
        blank
            .pixels_mut()
            .copy_from_slice(&[0u8; raster::TILE_BYTES]);
        document.tiles.insert(0, blank);
        BrushPaint
            .apply_json(
                &mut document,
                json!({
                    "id": image, "x": 1.3, "y": 1.3, "size": 6,
                    "color_raw": 0xFFFF0000u64 as i64,
                    "brush": { "edge": 0 }
                }),
            )
            .unwrap();
        assert_eq!(
            flat_pixel(&document, 2, 2),
            [255, 0, 0, 255],
            "edge 0 replaces"
        );
        assert_eq!(
            flat_pixel(&document, 3, 3),
            [0, 0, 0, 0],
            "the round rim misses"
        );

        let mut blank = document.tiles.get(0).cloned().unwrap();
        blank
            .pixels_mut()
            .copy_from_slice(&[0u8; raster::TILE_BYTES]);
        document.tiles.insert(0, blank);
        BrushPaint
            .apply_json(
                &mut document,
                json!({
                    "id": image, "x": 1.3, "y": 1.3, "size": 6,
                    "color_raw": 0xFFFF0000u64 as i64,
                    "brush": { "edge": 100 }
                }),
            )
            .unwrap();
        assert_ne!(
            flat_pixel(&document, 2, 2),
            [255, 0, 0, 255],
            "edge 100 blends at the same pixel"
        );

        let mut blank = document.tiles.get(0).cloned().unwrap();
        blank
            .pixels_mut()
            .copy_from_slice(&[0u8; raster::TILE_BYTES]);
        document.tiles.insert(0, blank);
        BrushPaint
            .apply_json(
                &mut document,
                json!({
                    "id": image, "x": 1.3, "y": 1.3, "size": 6,
                    "color_raw": 0xFFFF0000u64 as i64,
                    "brush": { "edge": 0, "shape": "square" }
                }),
            )
            .unwrap();
        assert_eq!(
            flat_pixel(&document, 3, 3),
            [255, 0, 0, 255],
            "the square shape reaches the corner"
        );
    }

    #[test]
    fn brush_strength_pressure_and_aspect_scale_the_dab() {
        let (mut document, image) = checker_document();
        let mut blank = document.tiles.get(0).cloned().unwrap();
        blank
            .pixels_mut()
            .copy_from_slice(&[0u8; raster::TILE_BYTES]);
        document.tiles.insert(0, blank);
        // Quarter effect: strength 50 x pressure 50 over full
        // coverage — straight alpha 255 -> 64.
        BrushPaint
            .apply_json(
                &mut document,
                json!({
                    "id": image, "x": 2.0, "y": 2.0, "size": 4,
                    "color_raw": 0xFFFF0000u64 as i64,
                    "strength": 50,
                    "brush": { "edge": 0, "pressure": 50 }
                }),
            )
            .unwrap();
        // Premultiplied read: the tile keeps straight RGBA here, so
        // the probe sees the blended alpha directly.
        assert_eq!(flat_pixel(&document, 2, 1)[3], 64);

        // Aspect 50 turns the same dab into a horizontal streak: the
        // pixel one step up at half radius drops out.
        let mut blank = document.tiles.get(0).cloned().unwrap();
        blank
            .pixels_mut()
            .copy_from_slice(&[0u8; raster::TILE_BYTES]);
        document.tiles.insert(0, blank);
        BrushPaint
            .apply_json(
                &mut document,
                json!({
                    "id": image, "x": 2.0, "y": 2.0, "size": 4,
                    "color_raw": 0xFFFF0000u64 as i64,
                    "brush": { "edge": 0, "aspect": 50 }
                }),
            )
            .unwrap();
        assert_eq!(flat_pixel(&document, 2, 1)[3], 255, "the wide axis holds");
        assert_eq!(flat_pixel(&document, 2, 3)[3], 0, "the squeezed axis drops");
    }

    #[test]
    fn pencil_stamps_one_pixel() {
        let (mut document, image) = checker_document();
        let mut blank = document.tiles.get(0).cloned().unwrap();
        blank
            .pixels_mut()
            .copy_from_slice(&[0u8; raster::TILE_BYTES]);
        document.tiles.insert(0, blank);
        Pencil
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 1.4, "y": 2.6, "color_raw": 0xFF00FF00u64 as i64 }),
            )
            .unwrap();
        // Exactly the pixel containing the point; neighbors untouched.
        assert_eq!(flat_pixel(&document, 1, 2), [0, 255, 0, 255]);
        assert_eq!(flat_pixel(&document, 0, 2), [0, 0, 0, 0]);
        assert_eq!(flat_pixel(&document, 2, 2), [0, 0, 0, 0]);
        assert_eq!(flat_pixel(&document, 1, 1), [0, 0, 0, 0]);
        assert_eq!(flat_pixel(&document, 1, 3), [0, 0, 0, 0]);
    }

    #[test]
    fn paint_refusals_leave_pixels_untouched() {
        let (mut document, image) = checker_document();
        let before = serde_json::to_vec(&document).unwrap();
        for payload in [
            json!({ "id": image, "x": 2.0, "y": 2.0, "size": 0, "color_raw": 0 }),
            json!({ "id": image, "x": 2.0, "y": 2.0, "size": 1025, "color_raw": 0 }),
            json!({ "id": image, "x": 2.0, "y": 2.0, "size": 4, "color_raw": 0,
                    "brush": { "edge": 101 } }),
            json!({ "id": image, "x": 2.0, "y": 2.0, "size": 4, "color_raw": 0,
                    "brush": { "aspect": 0 } }),
            json!({ "id": image, "x": 2.0, "y": 2.0, "size": 4, "color_raw": 0,
                    "brush": { "pressure": 0 } }),
            json!({ "id": image, "x": 2.0, "y": 2.0, "size": 4, "color_raw": 0,
                    "brush": { "shape": "tri" } }),
            json!({ "id": image, "x": 2.0, "y": 2.0, "size": 4, "color_raw": 0,
                    "strength": 0 }),
            json!({ "id": image, "x": 1.0, "y": 2.0, "color_raw": 0, "pressure": 0 }),
        ] {
            let command = if payload.get("size").is_some() {
                "bitmap.brush"
            } else {
                "bitmap.pencil"
            };
            let error = BrushPaint
                .apply_json(&mut document, payload.clone())
                .err()
                .or_else(|| Pencil.apply_json(&mut document, payload.clone()).err());
            let error = error.unwrap_or_else(|| panic!("{command} should refuse {payload}"));
            assert!(
                matches!(
                    error,
                    crate::error::EngineError::BadPayload { .. }
                        | crate::error::EngineError::InvalidState { .. }
                ),
                "{error}"
            );
        }
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "refused dabs change nothing"
        );
    }

    #[test]
    fn select_commands_capture_the_prior_mask() {
        let (mut document, image) = checker_document();
        let first = SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 1, "height": 1 }),
            )
            .unwrap();
        let second = SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 2, "y": 2, "width": 1, "height": 1, "mode": "add" }),
            )
            .unwrap();
        let mask = document.selection.as_ref().unwrap();
        assert_eq!(mask.get(0, 0), Some(255));
        assert_eq!(mask.get(2, 2), Some(255));
        assert_eq!(mask.get(1, 1), Some(0));

        second.apply_inverse(&mut document).unwrap();
        assert_eq!(
            document.selection.as_ref().unwrap().get(2, 2),
            Some(0),
            "undo of the add restores the first marquee"
        );
        first.apply_inverse(&mut document).unwrap();
        assert!(
            document.selection.is_none(),
            "undo of the first marquee restores no selection"
        );
    }

    #[test]
    fn wand_selects_by_color_within_tolerance() {
        let (mut document, image) = checker_document();
        SelectWand
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "tolerance": 0 }),
            )
            .unwrap();
        let mask = document.selection.as_ref().unwrap();
        // The flood walls off at the blue column: only column 0 joins.
        for y in 0..4 {
            assert_eq!(mask.get(0, y), Some(255), "red at (0,{y})");
        }
        for x in 1..4 {
            for y in 0..4 {
                assert_eq!(mask.get(x, y), Some(0), "walled off at ({x},{y})");
            }
        }

        SelectWand
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "tolerance": 0, "contiguous": false, "mode": "add" }),
            )
            .unwrap();
        let mask = document.selection.as_ref().unwrap();
        // The red seed finds the far red column (add keeps column 0).
        for x in [0u32, 2] {
            assert_eq!(mask.get(x, 0), Some(255), "red joins at ({x},0)");
        }
        for x in [1u32, 3] {
            assert_eq!(mask.get(x, 0), Some(0), "blue never matches at ({x},0)");
        }
    }

    #[test]
    fn select_similar_matches_the_selections_average_color() {
        let (mut document, image) = checker_document();
        // Seed: one red pixel (column 0). The average over a solid
        // selection is that color, so tolerance 0 finds every red
        // pixel image-wide (columns 0 and 2) — CS6 selectSimilar.
        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 1, "height": 1 }),
            )
            .unwrap();
        SelectSimilar
            .apply_json(&mut document, json!({ "id": image, "tolerance": 0 }))
            .unwrap();
        let mask = document.selection.as_ref().unwrap();
        for x in [0u32, 2] {
            for y in 0..4 {
                assert_eq!(mask.get(x, y), Some(255), "red joins at ({x},{y})");
            }
        }
        for x in [1u32, 3] {
            for y in 0..4 {
                assert_eq!(mask.get(x, y), Some(0), "blue never joins at ({x},{y})");
            }
        }

        // No active selection: a refusal, never a silent no-op.
        document.selection = None;
        let err = SelectSimilar
            .apply_json(&mut document, json!({ "id": image, "tolerance": 0 }))
            .expect_err("no active selection is refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));
    }

    #[test]
    fn feather_softens_then_fill_blends() {
        let (mut document, image) = checker_document();
        let mut blank = document.tiles.get(0).cloned().unwrap();
        blank
            .pixels_mut()
            .copy_from_slice(&[0u8; raster::TILE_BYTES]);
        document.tiles.insert(0, blank);
        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 4, "height": 4 }),
            )
            .unwrap();
        // Feather radius far larger than the plane: interior stays
        // fully selected.
        SelectFeather
            .apply_json(&mut document, json!({ "radius": 8 }))
            .unwrap();
        let mask = document.selection.as_ref().unwrap();
        assert_eq!(mask.get(1, 1), Some(255));

        Fill.apply_json(
            &mut document,
            json!({ "id": image, "color_raw": 0xFF_FF_FF_FFi64 }),
        )
        .unwrap();
        // Interior: full white. (Edge rows are dimmed by the falloff —
        // the pinned falloff table lives in the mask module tests.)
        assert_eq!(flat_pixel(&document, 1, 1), [255, 255, 255, 255]);
    }

    #[test]
    fn erase_and_move_undo_byte_exact() {
        let (mut document, image) = checker_document();

        let erase_delta = Erase
            .apply_json(&mut document, json!({ "id": image }))
            .unwrap();
        assert!(
            document
                .tiles
                .get(0)
                .unwrap()
                .pixels()
                .iter()
                .all(|&b| b == 0),
            "full-image erase (no selection) zeroes every pixel"
        );
        erase_delta.apply_inverse(&mut document).unwrap();
        assert_eq!(flat_pixel(&document, 0, 0), [255, 0, 0, 255]);
        assert_eq!(flat_pixel(&document, 1, 0), [0, 0, 255, 255]);

        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 1, "height": 1 }),
            )
            .unwrap();
        let move_delta = MovePixels
            .apply_json(&mut document, json!({ "id": image, "dx": 2, "dy": 0 }))
            .unwrap();
        assert_eq!(flat_pixel(&document, 0, 0), [0, 0, 0, 0], "source cleared");
        assert_eq!(
            flat_pixel(&document, 2, 0),
            [255, 0, 0, 255],
            "red moved +2x"
        );
        move_delta.apply_inverse(&mut document).unwrap();
        assert_eq!(flat_pixel(&document, 0, 0), [255, 0, 0, 255]);
        assert_eq!(
            flat_pixel(&document, 2, 0),
            [255, 0, 0, 255],
            "undo restores the overwritten pixel"
        );
    }

    #[test]
    fn refusals_leave_document_and_selection_untouched() {
        let (mut document, image) = checker_document();
        let not_image = document.registry.insert(ObjectData::Path(doc::PathObject {
            contours: Vec::new(),
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
            name: None,
            effects: Vec::new(),
        }));
        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 1, "height": 1 }),
            )
            .unwrap();
        let before = serde_json::to_vec(&document).unwrap();
        for payload in [
            // Wrong kind.
            json!({ "id": not_image, "x": 0, "y": 0, "width": 1, "height": 1 }),
            // Out-of-image seed.
            json!({ "id": image, "x": 9, "y": 0, "tolerance": 1 }),
            // Bad mode.
            json!({ "id": image, "x": 0, "y": 0, "width": 1, "height": 1, "mode": "xor" }),
            // Zero feather.
            json!({ "radius": 0 }),
        ] {
            let error = if payload.get("tolerance").is_some() {
                SelectWand.apply_json(&mut document, payload).unwrap_err()
            } else if payload.get("radius").is_some() {
                SelectFeather
                    .apply_json(&mut document, payload)
                    .unwrap_err()
            } else {
                // Wrong kind, bad mode — both go through the marquee.
                SelectRect.apply_json(&mut document, payload).unwrap_err()
            };
            assert!(matches!(error, EngineError::InvalidState { .. }));
        }
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "refused selection commands change nothing"
        );
    }

    #[test]
    fn selection_size_mismatch_refuses_pixel_edits() {
        let (mut document, image) = checker_document();
        // A mask sized for a different image (5 wide) must not edit.
        document.selection = Some(SelectionMask::empty(5, 4));
        let before = serde_json::to_vec(&document).unwrap();
        let error = Fill
            .apply_json(
                &mut document,
                json!({ "id": image, "color_raw": 0xFF000000u64 as i64 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    /// A sample document (2 pages, 2 layers, 2 cells each) whose page
    /// 0 canvas is 64×48 and whose cell (0, 0, 0) holds two paths plus
    /// a 4×4 opaque-red bitmap at position 2 (one tile, bank slot 0).
    fn placed_image_document() -> (Document, ObjectId) {
        let (mut document, ids) = crate::catalog::testdoc::sample_document();
        document.pages[0].info.canvas.width = 64.0;
        document.pages[0].info.canvas.height = 48.0;
        let mut buffer = [0u8; raster::TILE_BYTES];
        for pixel in buffer.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[255, 0, 0, 255]);
        }
        document.tiles.insert(0, raster::Tile::from_rgba8(buffer));
        let image = document
            .registry
            .insert(ObjectData::Image(doc::ImageObject {
                location: geom::Point::ZERO,
                offset: geom::Point::ZERO,
                pixels: (4, 4),
                tile_size: 128,
                tiles: vec![doc::TileRef::Tile(0)],
                name: None,
                opacity: 1000,
                blend: 0,
                visible: true,
                locked: false,
                effects: Vec::new(),
            }));
        document.pages[0].layers.layers[0].cells[0]
            .elements
            .push(image);
        let _ = ids;
        (document, image)
    }

    /// The first pixel of the tile behind `tiles[0]` of the image at
    /// `id`, premultiplied.
    fn tile_pixel(document: &Document, id: ObjectId) -> [u8; 4] {
        let doc::ObjectData::Image(image) = document.registry.get(id).unwrap() else {
            panic!("not an image")
        };
        let doc::TileRef::Tile(tile_id) = image.tiles[0] else {
            panic!("expected a bank tile")
        };
        document.tiles.get(tile_id).unwrap().pixels()[0..4]
            .try_into()
            .unwrap()
    }

    #[test]
    fn add_image_places_paintable_empty_tiles_and_undoes() {
        let (mut document, _) = placed_image_document();
        let before = serde_json::to_vec(&document).unwrap();
        let delta = AddImage
            .apply_json(
                &mut document,
                json!({
                    "x": 8.0, "y": 4.0, "width": 16.0, "height": 8.0,
                    "page": 0, "layer": 0, "cell": 0,
                    "name": "spot",
                }),
            )
            .unwrap();
        let elements = &document.pages[0].layers.layers[0].cells[0].elements;
        assert_eq!(elements.len(), 4, "appended after paths + the 4×4 bitmap");
        let id = elements[3];
        let doc::ObjectData::Image(image) = document.registry.get(id).unwrap() else {
            panic!("new object is an image")
        };
        assert_eq!(image.pixels, (16, 8));
        assert_eq!((image.location.x, image.location.y), (8.0, 4.0));
        assert_eq!(image.name.as_deref(), Some("spot"));
        let doc::TileRef::Tile(tile_id) = image.tiles[0] else {
            panic!("fresh bank tiles, not mattes")
        };
        assert_eq!(
            &document.tiles.get(tile_id).unwrap().pixels()[0..4],
            &[0, 0, 0, 0],
            "an empty image is transparent but paintable"
        );

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "byte-exact undo"
        );
    }

    #[test]
    fn add_image_constrains_to_canvas_and_refuses_outside() {
        let (mut document, _) = placed_image_document();
        // CS6: larger-than-document bounds constrain to the document.
        AddImage
            .apply_json(
                &mut document,
                json!({
                    "x": 40.0, "y": 40.0, "width": 100.0, "height": 100.0,
                    "page": 0, "layer": 0, "cell": 0, "name": null,
                }),
            )
            .unwrap();
        let elements = &document.pages[0].layers.layers[0].cells[0].elements;
        let doc::ObjectData::Image(image) = document.registry.get(elements[3]).unwrap() else {
            panic!("new object is an image")
        };
        assert_eq!(image.pixels, (24, 8), "clipped to the 64×48 canvas");
        assert_eq!((image.location.x, image.location.y), (40.0, 40.0));

        let before = serde_json::to_vec(&document).unwrap();
        for bounds in [
            json!({ "x": 100.0, "y": 0.0, "width": 10.0, "height": 10.0 }),
            json!({ "x": 0.0, "y": 0.0, "width": 0.0, "height": 8.0 }),
            json!({ "x": 0.0, "y": 0.0, "width": 8.0, "height": -1.0 }),
        ] {
            let mut payload = bounds;
            payload["page"] = json!(0);
            payload["layer"] = json!(0);
            payload["cell"] = json!(0);
            let error = AddImage.apply_json(&mut document, payload).unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }));
        }
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "refused placements change nothing"
        );

        // A placement cell out of range refuses AFTER the bounds pass
        // — the pre-flight must still leave bank and registry
        // untouched (the fuzz-caught minting leak).
        let before = serde_json::to_vec(&document).unwrap();
        let error = AddImage
            .apply_json(
                &mut document,
                json!({
                    "x": 0.0, "y": 0.0, "width": 8.0, "height": 8.0,
                    "page": 0, "layer": 9, "cell": 0, "name": null,
                }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
        let error = AddImage
            .apply_json(
                &mut document,
                json!({
                    "x": 0.0, "y": 0.0, "width": 8.0, "height": 8.0,
                    "page": 0, "layer": 0, "cell": 9, "name": null,
                }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::MissingTarget { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "refused placement cells mint nothing"
        );
    }

    #[test]
    fn via_copy_copies_the_selection_above_the_source_and_undoes() {
        let (mut document, image) = placed_image_document();
        let mut mask = SelectionMask::empty(4, 4);
        mask.set(0, 0, 128);
        document.selection = Some(mask);
        let before = serde_json::to_vec(&document).unwrap();

        let delta = AddImageViaCopy
            .apply_json(&mut document, json!({ "source": image }))
            .unwrap();
        let elements = &document.pages[0].layers.layers[0].cells[0].elements;
        assert_eq!(elements.len(), 4);
        let copy = elements[3];
        let doc::ObjectData::Image(new_image) = document.registry.get(copy).unwrap() else {
            panic!("the copy is an image")
        };
        assert_eq!((new_image.location.x, new_image.location.y), (0.0, 0.0));
        let doc::TileRef::Tile(copy_tile) = new_image.tiles[0] else {
            panic!("fresh bank tiles")
        };
        let pixels = document.tiles.get(copy_tile).unwrap().pixels();
        assert_eq!(
            &pixels[0..4],
            &[128, 0, 0, 128],
            "coverage 128 scales the premultiplied red"
        );
        assert_eq!(&pixels[4..8], &[0, 0, 0, 0], "outside the mask");
        assert_eq!(
            tile_pixel(&document, image),
            [255, 0, 0, 255],
            "the source bitmap is untouched"
        );

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "byte-exact undo"
        );
    }

    #[test]
    fn via_cut_clears_the_source_over_fresh_tiles_and_undoes() {
        let (mut document, image) = placed_image_document();
        let mut mask = SelectionMask::empty(4, 4);
        for y in 0..2 {
            for x in 0..2 {
                mask.set(x, y, 255);
            }
        }
        document.selection = Some(mask);
        let before = serde_json::to_vec(&document).unwrap();

        let delta = AddImageViaCut
            .apply_json(&mut document, json!({ "source": image }))
            .unwrap();
        // The source rebuild: fresh slot, cleared under the cut.
        let doc::ObjectData::Image(cut) = document.registry.get(image).unwrap() else {
            panic!("source stays an image")
        };
        let doc::TileRef::Tile(cut_tile) = cut.tiles[0] else {
            panic!("source was re-tiled over fresh slots")
        };
        assert_ne!(cut_tile, 0, "shared bank slots are never edited in place");
        let cut_pixels = document.tiles.get(cut_tile).unwrap().pixels();
        assert_eq!(&cut_pixels[0..4], &[0, 0, 0, 0]);
        assert_eq!(
            &cut_pixels[8..12],
            &[255, 0, 0, 255],
            "index (2,0) survives"
        );
        assert_eq!(
            &cut_pixels[(128 * 2) * 4..(128 * 2) * 4 + 4],
            &[255, 0, 0, 255]
        );
        assert_eq!(
            document.tiles.get(0).unwrap().pixels()[0..4],
            [255, 0, 0, 255],
            "the orphaned original tile keeps its bytes"
        );
        // The copy: the cut pixels moved.
        let elements = &document.pages[0].layers.layers[0].cells[0].elements;
        let doc::ObjectData::Image(new_image) = document.registry.get(elements[3]).unwrap() else {
            panic!("the copy is an image")
        };
        let doc::TileRef::Tile(new_tile) = new_image.tiles[0] else {
            panic!("fresh bank tiles")
        };
        let new_pixels = document.tiles.get(new_tile).unwrap().pixels();
        assert_eq!(&new_pixels[0..4], &[255, 0, 0, 255]);
        assert_eq!(&new_pixels[8..12], &[0, 0, 0, 0]);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "byte-exact undo restores the shared tile reference"
        );
    }

    #[test]
    fn via_copy_and_cut_refuse_without_a_spanning_selection() {
        let (mut document, image) = placed_image_document();
        let (mut no_mask, path) = crate::catalog::testdoc::sample_document();
        let _ = &mut no_mask;
        let stale = doc::ObjectId::from_parts(999, 0);
        let before = serde_json::to_vec(&document).unwrap();

        // No selection at all (a stale source refuses even earlier).
        for payload in [json!({ "source": image }), json!({ "source": stale })] {
            let error = AddImageViaCopy
                .apply_json(&mut document, payload.clone())
                .unwrap_err();
            assert!(matches!(
                error,
                EngineError::InvalidState { .. } | EngineError::MissingTarget { .. }
            ));
            let error = AddImageViaCut
                .apply_json(&mut document, payload)
                .unwrap_err();
            assert!(matches!(
                error,
                EngineError::InvalidState { .. } | EngineError::MissingTarget { .. }
            ));
        }
        // A plane sized for another image.
        document.selection = Some(SelectionMask::empty(8, 8));
        let error = AddImageViaCopy
            .apply_json(&mut document, json!({ "source": image }))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        let error = AddImageViaCut
            .apply_json(&mut document, json!({ "source": image }))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
        // Not a bitmap; not a live object.
        document.selection = Some(SelectionMask::empty(4, 4));
        for source in [path[0], stale] {
            for error in [
                AddImageViaCopy
                    .apply_json(&mut document, json!({ "source": source }))
                    .unwrap_err(),
                AddImageViaCut
                    .apply_json(&mut document, json!({ "source": source }))
                    .unwrap_err(),
            ] {
                assert!(
                    matches!(
                        error,
                        EngineError::InvalidState { .. } | EngineError::MissingTarget { .. }
                    ),
                    "non-image and stale sources refuse"
                );
            }
        }
        // The probes set selections (their own observable state); drop
        // them so the compare covers the refused commands only.
        document.selection = None;
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "refused copies and cuts change nothing"
        );
    }

    #[test]
    fn move_mask_translates_coverage_and_undo_is_exact() {
        let (mut document, image) = checker_document();
        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 2, "height": 2 }),
            )
            .unwrap();
        let before = document.selection.clone().unwrap();

        let delta = MoveSelectionMaskBy
            .apply_json(&mut document, json!({ "delta": { "x": 2, "y": 1 } }))
            .unwrap();
        let mask = document.selection.as_ref().unwrap();
        assert_eq!(mask.get(0, 0), Some(0), "the old corner vacated");
        assert_eq!(mask.get(2, 1), Some(255));
        assert_eq!(mask.get(3, 2), Some(255));
        assert_eq!(mask.get(3, 3), Some(0), "the shift clipped at the edge");

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.selection.as_ref().unwrap(), &before);
    }

    #[test]
    fn move_mask_gates() {
        let (mut document, image) = checker_document();
        // No plane yet: CS6 throws.
        let error = MoveSelectionMaskBy
            .apply_json(&mut document, json!({ "delta": { "x": 1, "y": 0 } }))
            .unwrap_err();
        assert!(error.to_string().contains("there is no pixel selection"));

        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 2, "height": 2 }),
            )
            .unwrap();
        let before = serde_json::to_vec(&document).unwrap();

        let error = MoveSelectionMaskBy
            .apply_json(&mut document, json!({ "delta": { "x": 0, "y": 0 } }))
            .unwrap_err();
        assert!(error.to_string().contains("zero delta"));

        let error = MoveSelectionMaskBy
            .apply_json(&mut document, json!({ "delta": { "x": 70000, "y": 0 } }))
            .unwrap_err();
        assert!(error.to_string().contains("65535"));

        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }

    #[test]
    fn adjust_expand_contract_round_trips_and_undo_is_exact() {
        let (mut document, image) = checker_document();
        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 1, "y": 1, "width": 2, "height": 2 }),
            )
            .unwrap();
        let before = document.selection.clone().unwrap();

        let delta = AdjustPixelSel
            .apply_json(&mut document, json!({ "what": "expand", "amount": 1 }))
            .unwrap();
        let mask = document.selection.as_ref().unwrap();
        // A 1px ring around the 2x2 block: the chamfer threshold is
        // the axial distance, so the diagonal corners of the 4x4 stay
        // out ((0,0) is 2 axial steps from (1,1)).
        assert_eq!(mask.get(0, 1), Some(255));
        assert_eq!(mask.get(3, 2), Some(255));
        assert_eq!(mask.get(0, 0), Some(0));

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document.selection.as_ref().unwrap(), &before);

        // Contract past the plane edge leaves the inner pixel only.
        AdjustPixelSel
            .apply_json(&mut document, json!({ "what": "contract", "amount": 1 }))
            .unwrap();
        let mask = document.selection.as_ref().unwrap();
        assert_eq!(
            mask.get(1, 1),
            Some(0),
            "1px from the unselected ring contracts away"
        );
        assert!(mask.alpha.iter().all(|&c| c == 0));
    }

    #[test]
    fn adjust_border_smooth_and_gates() {
        let (mut document, image) = checker_document();
        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 4, "height": 4 }),
            )
            .unwrap();
        let before = document.selection.clone().unwrap();

        // Border 1 of a full-plane selection: the whole plane is edge
        // (every pixel is 1 step from outside... no — outside is not a
        // boundary; there are no unselected pixels, so the border is
        // empty).
        AdjustPixelSel
            .apply_json(&mut document, json!({ "what": "border", "amount": 1 }))
            .unwrap();
        assert!(
            document
                .selection
                .as_ref()
                .unwrap()
                .alpha
                .iter()
                .all(|&c| c == 0)
        );

        // Smooth is closing(opening): on the full plane there are no
        // in-plane unselected pixels, so opening is the identity and
        // the smooth is too.
        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 4, "height": 4 }),
            )
            .unwrap();
        AdjustPixelSel
            .apply_json(&mut document, json!({ "what": "smooth", "amount": 2 }))
            .unwrap();
        assert!(
            document
                .selection
                .as_ref()
                .unwrap()
                .alpha
                .iter()
                .all(|&c| c == 255)
        );

        // A pinhole in the plane: closing fills it back.
        SelectRect
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 0, "y": 0, "width": 4, "height": 4 }),
            )
            .unwrap();
        let mut holed = document.selection.clone().unwrap();
        holed.set(1, 1, 0);
        document.selection = Some(holed);
        AdjustPixelSel
            .apply_json(&mut document, json!({ "what": "smooth", "amount": 1 }))
            .unwrap();
        assert_eq!(
            document.selection.as_ref().unwrap().get(1, 1),
            Some(255),
            "closing fills the pinhole"
        );

        // Gates: bad kind fails payload decode; amount 0 and
        // out-of-range refuse; refusals mutate nothing.
        assert!(
            AdjustPixelSel
                .apply_json(&mut document, json!({ "what": "feather", "amount": 1 }))
                .is_err()
        );
        for amount in [0, -1, 65536] {
            let error = AdjustPixelSel
                .apply_json(&mut document, json!({ "what": "expand", "amount": amount }))
                .unwrap_err();
            assert!(error.to_string().contains("1..=65535"));
        }
        document.selection = None;
        let error = AdjustPixelSel
            .apply_json(&mut document, json!({ "what": "expand", "amount": 1 }))
            .unwrap_err();
        assert!(error.to_string().contains("there is no pixel selection"));
        let _ = before;
    }
}
