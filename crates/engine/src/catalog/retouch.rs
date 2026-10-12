//! Retouch tools (M7): blur, smudge-lite, rubber stamp (clone),
//! eraser, red-eye — the brush-shaped pixel tools.
//!
//! Every tool takes **explicit geometry**: brush center and radius
//! come in the payload, never from pointer state, so a command is the
//! whole stroke and replay reproduces it exactly. The brush footprint
//! is a circle in image pixel space with a deterministic soft edge —
//! full strength to half the radius, then a linear falloff to the rim,
//! all integer arithmetic rounded half-up. The active selection scales
//! the effect (a tool only paints where the mask has coverage); with
//! no selection the brush paints the whole footprint.

use doc::{Document, ObjectId};

use super::bitmap::{blend_pixel, masked_pixel_edit};
use crate::command::TypedCommand;
use crate::delta::Delta;
use crate::error::EngineError;

/// Largest brush radius in pixels.
const MAX_RADIUS: u32 = 512;

/// Registers the retouch commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(RetouchBlur)?;
    registry.register(RetouchSmudge)?;
    registry.register(RetouchClone)?;
    registry.register(RetouchEraser)?;
    registry.register(RetouchRedEye)
}

use crate::registry::CommandRegistry;

/// Validates a payload brush and folds in the shared limits.
struct Brush {
    center_x: f64,
    center_y: f64,
    radius: f64,
}

fn brush_of(name: &'static str, x: f64, y: f64, radius: u32) -> Result<Brush, EngineError> {
    if radius == 0 || radius > MAX_RADIUS {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("brush radius {radius} is not in 1..={MAX_RADIUS}"),
        });
    }
    Ok(Brush {
        center_x: x,
        center_y: y,
        radius: f64::from(radius),
    })
}

impl Brush {
    /// Coverage 0..=255 of the pixel at `px, py`: 255 inside half the
    /// radius, linear falloff to the rim, 0 outside — computed in f64
    /// from basic operations only, rounded half-up.
    fn coverage(&self, px: u32, py: u32) -> u8 {
        let dx = f64::from(px) + 0.5 - self.center_x;
        let dy = f64::from(py) + 0.5 - self.center_y;
        let distance = (dx * dx + dy * dy).sqrt();
        let inner = self.radius / 2.0;
        if distance <= inner {
            return 255;
        }
        if distance >= self.radius {
            return 0;
        }
        ((255.0 * (self.radius - distance) / inner) + 0.5) as u8
    }
}

/// Scales a raw brush coverage by a percentage strength (1..=100).
fn strength_of(name: &'static str, strength: u8) -> Result<u32, EngineError> {
    if strength == 0 {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "strength 0 would be a no-op".to_owned(),
        });
    }
    Ok(u32::from(strength))
}

/// The effect coverage for one pixel: brush × strength × selection.
#[inline]
fn effect(brush: u8, strength: u32, selection: u8) -> u8 {
    let scaled = (u32::from(brush) * strength + 50) / 100;
    ((scaled * u32::from(selection) + 127) / 255) as u8
}

/// One separable box average over a straight-alpha RGBA buffer —
/// horizontal pass then vertical pass, both edge-replicated, per
/// channel `u32` window sums with half-up rounding. The blur tool's
/// kernel (one pass: a soft brush, not a gaussian filter stack).
fn box_average(pixels: &[u8], width: usize, height: usize, half: usize) -> Vec<u8> {
    let rows = horizontal(pixels, width, height, half);
    let turned = transpose_interleaved(&rows, width, height);
    let columns = horizontal(&turned, height, width, half);
    transpose_interleaved(&columns, height, width)
}

/// One edge-replicated box pass along the rows of an interleaved RGBA
/// buffer (`w` pixels per row, 4 bytes per pixel).
fn horizontal(plane: &[u8], w: usize, h: usize, half: usize) -> Vec<u8> {
    let span = (half * 2 + 1) as u32;
    let mut out = vec![0u8; plane.len()];
    for y in 0..h {
        let row = &plane[y * w * 4..(y + 1) * w * 4];
        let out_row = &mut out[y * w * 4..(y + 1) * w * 4];
        let mut sums = [0u32; 4];
        // Window of column 0: (half+1) replicated copies of column 0
        // (indices −half..0 clamp to it) plus columns 1..=half.
        for (sum, pixel) in sums.iter_mut().zip(&row[0..4]) {
            *sum += u32::from(*pixel) * (half as u32 + 1);
        }
        for offset in 1..=half {
            for (sum, pixel) in sums.iter_mut().zip(&row[offset.min(w - 1) * 4..][..4]) {
                *sum += u32::from(*pixel);
            }
        }
        for x in 0..w {
            for (channel, sum) in sums.iter().enumerate() {
                out_row[x * 4 + channel] = ((sum + span / 2) / span) as u8;
            }
            if x + 1 < w {
                let add = (x + 1 + half).min(w - 1) * 4;
                let drop = x.saturating_sub(half) * 4;
                for channel in 0..4 {
                    sums[channel] += u32::from(row[add + channel]);
                    sums[channel] -= u32::from(row[drop + channel]);
                }
            }
        }
    }
    out
}

/// Transposes an interleaved RGBA buffer (width × height → height ×
/// width), keeping whole pixels together.
fn transpose_interleaved(plane: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0u8; plane.len()];
    for y in 0..height {
        for x in 0..width {
            let source = (y * width + x) * 4;
            let target = (x * height + y) * 4;
            out[target..target + 4].copy_from_slice(&plane[source..source + 4]);
        }
    }
    out
}

/// Samples a pixel with edge replication; out-of-range coordinates
/// clamp to the border.
fn sample(pixels: &[u8], width: u32, height: u32, x: i64, y: i64) -> [u8; 4] {
    let clamped_x = x.clamp(0, i64::from(width - 1));
    let clamped_y = y.clamp(0, i64::from(height - 1));
    let index = ((clamped_y * i64::from(width) + clamped_x) * 4) as usize;
    [
        pixels[index],
        pixels[index + 1],
        pixels[index + 2],
        pixels[index + 3],
    ]
}

/// Writes `value` into the pixel slot.
fn write(pixels: &mut [u8], index: usize, value: [u8; 4]) {
    pixels[index * 4..index * 4 + 4].copy_from_slice(&value);
}

/// `retouch.blur` — soften pixels under the brush toward their
/// neighborhood average, scaled by strength and mask coverage.
struct RetouchBlur;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RetouchBlurPayload {
    /// Target image.
    id: ObjectId,
    /// Brush center in image pixel coordinates.
    x: f64,
    y: f64,
    /// Brush radius in pixels (1..=512).
    radius: u32,
    /// Effect strength percent (1..=100).
    strength: u8,
}

impl TypedCommand for RetouchBlur {
    type Payload = RetouchBlurPayload;

    fn name(&self) -> &'static str {
        "retouch.blur"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "retouch.blur";
        let brush = brush_of(NAME, payload.x, payload.y, payload.radius)?;
        let strength = strength_of(NAME, payload.strength)?;
        let half = (payload.radius as usize / 2).max(1);
        masked_pixel_edit(document, NAME, payload.id, |pixels, size, mask| {
            let snapshot = pixels.to_vec();
            let blurred = box_average(&snapshot, size.0 as usize, size.1 as usize, half);
            for py in 0..size.1 {
                for px in 0..size.0 {
                    let index = py as usize * size.0 as usize + px as usize;
                    let eff = effect(brush.coverage(px, py), strength, mask.alpha[index]);
                    if eff == 0 {
                        continue;
                    }
                    let mut pixel: [u8; 4] =
                        pixels[index * 4..index * 4 + 4].try_into().map_err(|_| {
                            EngineError::InvalidState {
                                name: NAME.to_owned(),
                                detail: "pixel buffer misaligned".to_owned(),
                            }
                        })?;
                    let target: [u8; 4] =
                        blurred[index * 4..index * 4 + 4].try_into().map_err(|_| {
                            EngineError::InvalidState {
                                name: NAME.to_owned(),
                                detail: "pixel buffer misaligned".to_owned(),
                            }
                        })?;
                    blend_pixel(&mut pixel, target, eff);
                    write(pixels, index, pixel);
                }
            }
            Ok(())
        })
    }
}

/// `retouch.smudge` — smear pixels toward the color one step "behind"
/// the brush direction: each footprint pixel blends toward its neighbor
/// at `(−dx, −dy)`, so dragging left pulls color rightward. The lite
/// single-sample form; no drag-path state.
struct RetouchSmudge;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RetouchSmudgePayload {
    /// Target image.
    id: ObjectId,
    /// Brush center in image pixel coordinates.
    x: f64,
    y: f64,
    /// Brush radius in pixels (1..=512).
    radius: u32,
    /// Effect strength percent (1..=100).
    strength: u8,
    /// Smear direction in pixels; (0, 0) is refused.
    dx: i32,
    dy: i32,
}

impl TypedCommand for RetouchSmudge {
    type Payload = RetouchSmudgePayload;

    fn name(&self) -> &'static str {
        "retouch.smudge"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "retouch.smudge";
        let brush = brush_of(NAME, payload.x, payload.y, payload.radius)?;
        let strength = strength_of(NAME, payload.strength)?;
        if payload.dx == 0 && payload.dy == 0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "smudge direction (0, 0) would be a no-op".to_owned(),
            });
        }
        masked_pixel_edit(document, NAME, payload.id, |pixels, size, mask| {
            let snapshot = pixels.to_vec();
            for py in 0..size.1 {
                for px in 0..size.0 {
                    let index = py as usize * size.0 as usize + px as usize;
                    let eff = effect(brush.coverage(px, py), strength, mask.alpha[index]);
                    if eff == 0 {
                        continue;
                    }
                    let mut pixel: [u8; 4] =
                        pixels[index * 4..index * 4 + 4].try_into().map_err(|_| {
                            EngineError::InvalidState {
                                name: NAME.to_owned(),
                                detail: "pixel buffer misaligned".to_owned(),
                            }
                        })?;
                    let behind = sample(
                        &snapshot,
                        size.0,
                        size.1,
                        i64::from(px) - i64::from(payload.dx),
                        i64::from(py) - i64::from(payload.dy),
                    );
                    blend_pixel(&mut pixel, behind, eff);
                    write(pixels, index, pixel);
                }
            }
            Ok(())
        })
    }
}

/// `retouch.clone` — rubber stamp: copy pixels from the source offset
/// into the brush footprint. The offset is fixed by this stamp's
/// `source`/`target` points; the aligned behavior across strokes is the
/// caller holding that offset constant (the engine sees one stamp).
struct RetouchClone;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RetouchClonePayload {
    /// Target image.
    id: ObjectId,
    /// The pixel the sample is taken from.
    source_x: f64,
    source_y: f64,
    /// The pixel the stamp centers on.
    target_x: f64,
    target_y: f64,
    /// Brush radius in pixels (1..=512).
    radius: u32,
}

impl TypedCommand for RetouchClone {
    type Payload = RetouchClonePayload;

    fn name(&self) -> &'static str {
        "retouch.clone"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "retouch.clone";
        let brush = brush_of(NAME, payload.target_x, payload.target_y, payload.radius)?;
        // Stamp offset: what a footprint pixel adds to reach its source.
        let offset_x = (payload.source_x - payload.target_x).round() as i64;
        let offset_y = (payload.source_y - payload.target_y).round() as i64;
        masked_pixel_edit(document, NAME, payload.id, |pixels, size, mask| {
            let snapshot = pixels.to_vec();
            for py in 0..size.1 {
                for px in 0..size.0 {
                    let index = py as usize * size.0 as usize + px as usize;
                    let eff = effect(brush.coverage(px, py), 100, mask.alpha[index]);
                    if eff == 0 {
                        continue;
                    }
                    let mut pixel: [u8; 4] =
                        pixels[index * 4..index * 4 + 4].try_into().map_err(|_| {
                            EngineError::InvalidState {
                                name: NAME.to_owned(),
                                detail: "pixel buffer misaligned".to_owned(),
                            }
                        })?;
                    let source = sample(
                        &snapshot,
                        size.0,
                        size.1,
                        i64::from(px) + offset_x,
                        i64::from(py) + offset_y,
                    );
                    blend_pixel(&mut pixel, source, eff);
                    write(pixels, index, pixel);
                }
            }
            Ok(())
        })
    }
}

/// `retouch.eraser` — brush-shaped alpha reduction: scale each
/// footprint pixel's alpha down by strength × coverage.
struct RetouchEraser;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RetouchEraserPayload {
    /// Target image.
    id: ObjectId,
    /// Brush center in image pixel coordinates.
    x: f64,
    y: f64,
    /// Brush radius in pixels (1..=512).
    radius: u32,
    /// Effect strength percent (1..=100).
    strength: u8,
}

impl TypedCommand for RetouchEraser {
    type Payload = RetouchEraserPayload;

    fn name(&self) -> &'static str {
        "retouch.eraser"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "retouch.eraser";
        let brush = brush_of(NAME, payload.x, payload.y, payload.radius)?;
        let strength = strength_of(NAME, payload.strength)?;
        masked_pixel_edit(document, NAME, payload.id, |pixels, size, mask| {
            for py in 0..size.1 {
                for px in 0..size.0 {
                    let index = py as usize * size.0 as usize + px as usize;
                    let eff = effect(brush.coverage(px, py), strength, mask.alpha[index]);
                    if eff == 0 {
                        continue;
                    }
                    let mut pixel: [u8; 4] =
                        pixels[index * 4..index * 4 + 4].try_into().map_err(|_| {
                            EngineError::InvalidState {
                                name: NAME.to_owned(),
                                detail: "pixel buffer misaligned".to_owned(),
                            }
                        })?;
                    let alpha = u32::from(pixel[3]) * (255 - u32::from(eff)) + 127;
                    pixel[3] = (alpha / 255) as u8;
                    write(pixels, index, pixel);
                }
            }
            Ok(())
        })
    }
}

/// `retouch.redEye` — damp the red channel of red-dominant pixels in
/// the footprint: any pixel whose red exceeds `max(green, blue) + 32`
/// has its excess scaled down, leaving genuinely red objects (which
/// also have high green/blue) untouched.
struct RetouchRedEye;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct RetouchRedEyePayload {
    /// Target image.
    id: ObjectId,
    /// Brush center in image pixel coordinates.
    x: f64,
    y: f64,
    /// Brush radius in pixels (1..=512).
    radius: u32,
}

impl TypedCommand for RetouchRedEye {
    type Payload = RetouchRedEyePayload;

    fn name(&self) -> &'static str {
        "retouch.redEye"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "retouch.redEye";
        let brush = brush_of(NAME, payload.x, payload.y, payload.radius)?;
        masked_pixel_edit(document, NAME, payload.id, |pixels, size, mask| {
            for py in 0..size.1 {
                for px in 0..size.0 {
                    let index = py as usize * size.0 as usize + px as usize;
                    let eff = effect(brush.coverage(px, py), 100, mask.alpha[index]);
                    if eff == 0 {
                        continue;
                    }
                    let mut pixel: [u8; 4] =
                        pixels[index * 4..index * 4 + 4].try_into().map_err(|_| {
                            EngineError::InvalidState {
                                name: NAME.to_owned(),
                                detail: "pixel buffer misaligned".to_owned(),
                            }
                        })?;
                    let red = u16::from(pixel[0]);
                    let ceiling = u16::from(pixel[1].max(pixel[2])) + 32;
                    if red > ceiling {
                        let excess = u32::from(red - ceiling) * u32::from(255 - eff);
                        pixel[0] = (ceiling + ((excess + 127) / 255) as u16) as u8;
                    }
                    write(pixels, index, pixel);
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::bitmap::gather;
    use crate::command::Command;
    use doc::{Cell, ImageObject, Layer, LayerList, ObjectData, Page, PageInfo, TileRef};
    use serde_json::json;

    /// A 3x1 opaque strip of the given straight pixels, one tile.
    fn strip_document(pixels: [[u8; 4]; 3]) -> (Document, ObjectId) {
        let mut document = Document::default();
        let mut tile = raster::Tile::new();
        for (index, value) in pixels.iter().enumerate() {
            tile.pixels_mut()[index * 4..index * 4 + 4].copy_from_slice(value);
        }
        document.tiles.insert(0, tile);
        let image = document.registry.insert(ObjectData::Image(ImageObject {
            location: geom::Point::ZERO,
            offset: geom::Point::ZERO,
            pixels: (3, 1),
            tile_size: raster::TILE_SIDE as i64,
            tiles: vec![TileRef::Tile(0)],
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }));
        let mut layer = Layer {
            name: "L".to_owned(),
            display: true,
            ..Layer::default()
        };
        layer.cells.push(Cell {
            locked: false,
            visible: true,
            elements: vec![image],
        });
        let mut layers = LayerList::default();
        layers.layers.push(layer);
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
            name: Some("P".to_owned()),
            is_master: false,
            is_linked: false,
            page_id: None,
            last_export_dir: None,
            last_export_file: None,
            layers,
            grid: None,
            guides: Default::default(),
            export: None,
        });
        (document, image)
    }

    fn flat_pixel(document: &Document, image: ObjectId, x: u32) -> [u8; 4] {
        let object = match document.registry.get(image) {
            Some(ObjectData::Image(image)) => image.clone(),
            _ => panic!("not an image"),
        };
        let flat = gather(&object, &document.tiles, "test").unwrap();
        flat.pixels[x as usize * 4..x as usize * 4 + 4]
            .try_into()
            .unwrap()
    }

    /// The whole 3-pixel strip as straight pixels.
    fn strip(document: &Document, image: ObjectId) -> [[u8; 4]; 3] {
        [
            flat_pixel(document, image, 0),
            flat_pixel(document, image, 1),
            flat_pixel(document, image, 2),
        ]
    }

    /// A brush centered on the strip covering all three pixels.
    fn full_row(image: ObjectId, radius: u32) -> serde_json::Value {
        json!({ "id": image, "x": 1.5, "y": 0.5, "radius": radius, "strength": 100 })
    }

    #[test]
    fn blur_pulls_pixels_toward_the_neighborhood_average() {
        // Gray strip [0, 128, 255], one box pass (half = 1, edge
        // replicated): averages [43, 128, 213] — full coverage replaces.
        let (mut document, image) =
            strip_document([[0, 0, 0, 255], [128, 128, 128, 255], [255, 255, 255, 255]]);
        RetouchBlur
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 1.5, "y": 0.5, "radius": 2, "strength": 100 }),
            )
            .unwrap();
        assert_eq!(
            strip(&document, image),
            [
                [43, 43, 43, 255],
                [128, 128, 128, 255],
                [213, 213, 213, 255]
            ]
        );
    }

    #[test]
    fn smudge_pulls_color_from_behind_the_drag() {
        // dx = 1: each pixel blends toward its left neighbor (clamped).
        let (mut document, image) =
            strip_document([[0, 0, 0, 255], [128, 128, 128, 255], [255, 255, 255, 255]]);
        RetouchSmudge
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 1.5, "y": 0.5, "radius": 2, "strength": 100, "dx": 1, "dy": 0 }),
            )
            .unwrap();
        assert_eq!(
            strip(&document, image),
            [[0, 0, 0, 255], [0, 0, 0, 255], [128, 128, 128, 255]]
        );
    }

    #[test]
    fn clone_stamps_the_source_offset() {
        // Radius 1 covers only the pixel the target centers on; it
        // copies the source pixel (two to the left).
        let (mut document, image) =
            strip_document([[255, 0, 0, 255], [0, 255, 0, 255], [200, 200, 200, 255]]);
        RetouchClone
            .apply_json(
                &mut document,
                json!({ "id": image, "source_x": 0.5, "source_y": 0.5, "target_x": 2.5, "target_y": 0.5, "radius": 1 }),
            )
            .unwrap();
        assert_eq!(
            strip(&document, image),
            [[255, 0, 0, 255], [0, 255, 0, 255], [255, 0, 0, 255]]
        );
    }

    #[test]
    fn eraser_scales_alpha_by_strength() {
        // Full coverage, strength 50: eff = 128, alpha 255 -> 127.
        let (mut document, image) =
            strip_document([[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]]);
        RetouchEraser
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 1.5, "y": 0.5, "radius": 2, "strength": 50 }),
            )
            .unwrap();
        assert_eq!(
            strip(&document, image),
            [[255, 0, 0, 127], [0, 255, 0, 127], [0, 0, 255, 127],]
        );
    }

    #[test]
    fn red_eye_damps_only_red_dominant_pixels() {
        let (mut document, image) =
            strip_document([[255, 80, 60, 255], [80, 80, 90, 255], [255, 255, 255, 255]]);
        RetouchRedEye
            .apply_json(
                &mut document,
                json!({ "id": image, "x": 1.5, "y": 0.5, "radius": 2 }),
            )
            .unwrap();
        // Pupil: red clamps to max(g,b)+32 = 112. Gray/white untouched.
        assert_eq!(
            strip(&document, image),
            [[112, 80, 60, 255], [80, 80, 90, 255], [255, 255, 255, 255],]
        );
    }

    #[test]
    fn selection_clips_the_brush_and_undo_is_byte_exact() {
        let (mut document, image) =
            strip_document([[0, 0, 0, 255], [128, 128, 128, 255], [255, 255, 255, 255]]);
        let registry = single_registry();
        // Select only pixel 0.
        registry
            .execute(
                &mut document,
                "bitmap.selectRect",
                json!({ "id": image, "x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0 }),
            )
            .unwrap();
        let before = serde_json::to_vec(&document).unwrap();
        let stroke = json!({ "id": image, "x": 1.5, "y": 0.5, "radius": 2, "strength": 100 });
        let delta = registry
            .execute(&mut document, "retouch.blur", stroke.clone())
            .unwrap();
        let strip_now = strip(&document, image);
        assert_eq!(strip_now[1], [128, 128, 128, 255], "mask clips the brush");
        assert_eq!(strip_now[0], [43, 43, 43, 255]);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "undo restores the pre-stroke bytes exactly"
        );
    }

    fn single_registry() -> crate::registry::CommandRegistry {
        let mut registry = crate::registry::CommandRegistry::new();
        crate::catalog::install(&mut registry).unwrap();
        registry
    }

    #[test]
    fn refusals_leave_pixels_untouched() {
        let (mut document, image) =
            strip_document([[0, 0, 0, 255], [128, 128, 128, 255], [255, 255, 255, 255]]);
        let registry = single_registry();
        let before = serde_json::to_vec(&document).unwrap();
        for (name, payload) in [
            (
                "retouch.blur",
                json!({ "id": image, "x": 1.5, "y": 0.5, "radius": 0, "strength": 100 }),
            ),
            (
                "retouch.blur",
                json!({ "id": image, "x": 1.5, "y": 0.5, "radius": 513, "strength": 100 }),
            ),
            (
                "retouch.blur",
                json!({ "id": image, "x": 1.5, "y": 0.5, "radius": 2, "strength": 0 }),
            ),
            (
                "retouch.smudge",
                json!({ "id": image, "x": 1.5, "y": 0.5, "radius": 2, "strength": 100, "dx": 0, "dy": 0 }),
            ),
        ] {
            let error = registry
                .execute(&mut document, name, payload)
                .expect_err("refused");
            assert!(
                matches!(error, crate::error::EngineError::InvalidState { .. }),
                "{error}"
            );
        }
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            before,
            "refused strokes change nothing"
        );
    }

    #[test]
    fn every_tool_undoes_to_the_same_bytes() {
        let (mut document, image) =
            strip_document([[255, 80, 60, 255], [128, 128, 128, 255], [0, 0, 255, 255]]);
        let original = serde_json::to_vec(&document).unwrap();
        let registry = single_registry();
        for (name, payload) in [
            ("retouch.blur", full_row(image, 2)),
            ("retouch.smudge", {
                let mut value = full_row(image, 2);
                value["dx"] = json!(1);
                value["dy"] = json!(0);
                value
            }),
            (
                "retouch.clone",
                json!({ "id": image, "source_x": 0.5, "source_y": 0.5, "target_x": 1.5, "target_y": 0.5, "radius": 1 }),
            ),
            ("retouch.eraser", full_row(image, 2)),
            (
                "retouch.redEye",
                json!({ "id": image, "x": 1.5, "y": 0.5, "radius": 2 }),
            ),
        ] {
            let delta = registry
                .execute(&mut document, name, payload)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            delta.apply_inverse(&mut document).unwrap();
            assert_eq!(
                serde_json::to_vec(&document).unwrap(),
                original,
                "{name} undo is byte-exact"
            );
        }
    }
}
