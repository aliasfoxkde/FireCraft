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

use doc::{Document, ObjectData, ObjectId, SelectionMask};

use crate::access;
use crate::bitmap::{gather, scatter, touched_tiles};
use crate::command::TypedCommand;
use crate::delta::Delta;
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
    registry.register(SelectWand)?;
    registry.register(SelectFeather)?;
    registry.register(SelectNone)?;
    registry.register(Fill)?;
    registry.register(Erase)?;
    registry.register(MovePixels)
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
            "mode {:?} is not one of replace/add/subtract",
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
    document.selection = Some(fold(active.as_ref(), fresh, mode));
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
    /// Polygon vertices in image pixel coordinates.
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
fn masked_pixel_edit(
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
fn blend_pixel(pixel: &mut [u8; 4], target: [u8; 4], coverage: u8) {
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
}
