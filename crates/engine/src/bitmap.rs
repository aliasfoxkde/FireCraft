//! Bitmap working space: gather an image's tiles into one flat
//! straight-alpha RGBA buffer, edit, scatter back — the plumbing every
//! masked pixel op (fill, erase, move, the M7 retouch tools) shares.
//!
//! Tiles are stored **premultiplied** (`raster::Tile`); the flat buffer
//! is **straight** so masks and tools reason about real colors. The
//! conversions are the only place the layouts meet. Tile payloads the
//! host never loaded are a refusal, not silent transparency: editing
//! pixels we do not have would fabricate content. Only `TSZi` 128
//! grids are editable in M7 — the 256 variant is the grayscale mkBT
//! layout and stays an M8 import concern.

use doc::{Document, ImageObject, TileBank, TileRef};

use crate::error::EngineError;

/// Working-space cap: no image above 16 777 216 pixels (4096²) gathers
/// to a flat buffer — beyond that the tiled path (or tiling-aware tools)
/// is the answer, and the refusal names the limit.
pub(crate) const MAX_GATHER_PIXELS: u64 = 16_777_216;

/// One image's pixels, straight alpha RGBA8, row-major, image space.
#[derive(Debug)]
pub(crate) struct FlatImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Checks the editable-image preconditions shared by gather/scatter.
pub(crate) fn check_editable(
    image: &ImageObject,
    name: &'static str,
) -> Result<(u64, u64), EngineError> {
    if image.tile_size != raster::TILE_SIDE as i64 {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!(
                "tile size {} is not editable (only {}-pixel ARGB grids are)",
                image.tile_size,
                raster::TILE_SIDE
            ),
        });
    }
    let (Ok(width), Ok(height)) = (u64::try_from(image.pixels.0), u64::try_from(image.pixels.1))
    else {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("image pixels {:?} are not addressable", image.pixels),
        });
    };
    if width == 0 || height == 0 {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "image has no pixels to edit".to_owned(),
        });
    }
    if width * height > MAX_GATHER_PIXELS {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!(
                "image {width}x{height} exceeds the {MAX_GATHER_PIXELS}-pixel working space"
            ),
        });
    }
    Ok((width, height))
}

/// Gathers every `TIDi` tile of `image` from `bank` into one flat
/// straight-alpha buffer. Matte (`TMCi`) tiles contribute nothing —
/// they are flat file-side color, not editable pixels — but they still
/// occupy their grid slot in the `TILv` order. Missing payloads are
/// [`EngineError::InvalidState`].
pub(crate) fn gather(
    image: &ImageObject,
    bank: &TileBank,
    name: &'static str,
) -> Result<FlatImage, EngineError> {
    let (width, height) = check_editable(image, name)?;
    let mut pixels = vec![0u8; (width * height * 4).try_into().unwrap_or(usize::MAX)];
    for (slot, tile_ref) in image.tiles.iter().enumerate() {
        let TileRef::Tile(tile_id) = tile_ref else {
            continue;
        };
        let Some(tile) = bank.get(*tile_id) else {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("tile {tile_id} is referenced but not loaded"),
            });
        };
        copy_tile_into(&mut pixels, width, height, slot as u64, tile.pixels());
    }
    unpremultiply_all(&mut pixels);
    Ok(FlatImage {
        width: width as u32,
        height: height as u32,
        pixels,
    })
}

/// Copies one 128×128 premultiplied tile into the flat buffer at its
/// positional `TILv` slot (row-major grid), clipping to the bounds.
fn copy_tile_into(flat: &mut [u8], width: u64, height: u64, slot: u64, tile: &[u8]) {
    let side = raster::TILE_SIDE as u64;
    let cols = width.div_ceil(side);
    let (origin_x, origin_y) = ((slot % cols) * side, (slot / cols) * side);
    for ty in 0..side {
        let py = origin_y + ty;
        if py >= height {
            break;
        }
        for tx in 0..side {
            let px = origin_x + tx;
            let index = py * width + px;
            let Some(target) = (index * 4)
                .try_into()
                .ok()
                .and_then(|i: usize| flat.get_mut(i..i + 4))
            else {
                break;
            };
            let source = ((ty * side + tx) * 4) as usize;
            target.copy_from_slice(&tile[source..source + 4]);
        }
    }
}

/// Writes the flat buffer back into the bank, re-premultiplying and
/// copying only the tiles listed in `touched` (the delta's captures).
pub(crate) fn scatter(
    flat: &FlatImage,
    image: &ImageObject,
    bank: &mut TileBank,
    touched: &[u32],
    name: &'static str,
) -> Result<(), EngineError> {
    // The flat buffer's own dimensions are the working truth; the image
    // check still gates editability so scatter cannot write a foreign
    // grid.
    let _ = check_editable(image, name)?;
    let (width, height) = (u64::from(flat.width), u64::from(flat.height));
    let mut premultiplied = flat.pixels.clone();
    premultiply_all(&mut premultiplied);
    let side = raster::TILE_SIDE as u64;
    let cols = width.div_ceil(side);
    for tile_id in touched {
        let slot = image
            .tiles
            .iter()
            .position(|tile_ref| *tile_ref == TileRef::Tile(*tile_id))
            .ok_or_else(|| EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("tile {tile_id} is not in the image's TILv"),
            })? as u64;
        let (origin_x, origin_y) = ((slot % cols) * side, (slot / cols) * side);
        let mut buffer = [0u8; raster::TILE_BYTES];
        for ty in 0..side {
            let py = origin_y + ty;
            if py >= height {
                break;
            }
            for tx in 0..side {
                let px = origin_x + tx;
                if px >= width {
                    break;
                }
                let source = ((py * width + px) * 4) as usize;
                let target = (ty * side + tx) as usize * 4;
                buffer[target..target + 4].copy_from_slice(&premultiplied[source..source + 4]);
            }
        }
        if let Some(tile) = bank.get(*tile_id) {
            let mut tile = tile.clone();
            tile.pixels_mut().copy_from_slice(&buffer);
            bank.insert(*tile_id, tile);
        }
    }
    Ok(())
}

/// The `TIDi` ids of `image` whose tiles intersect the mask's set
/// pixels — the tiles a command must capture and rewrite.
pub(crate) fn touched_tiles(image: &ImageObject, mask: &doc::SelectionMask) -> Vec<u32> {
    let width = u64::from(mask.width);
    let side = raster::TILE_SIDE as u64;
    let cols = width.div_ceil(side);
    let mut touched = Vec::new();
    for (slot, tile_ref) in image.tiles.iter().enumerate() {
        let TileRef::Tile(tile_id) = tile_ref else {
            continue;
        };
        let (origin_x, origin_y) = ((slot as u64 % cols) * side, (slot as u64 / cols) * side);
        let y_end = (origin_y + side).min(u64::from(mask.height));
        let x_end = (origin_x + side).min(u64::from(mask.width));
        let any = (origin_y..y_end).any(|py| {
            (origin_x..x_end).any(|px| {
                usize::try_from(py * width + px)
                    .ok()
                    .and_then(|index| mask.alpha.get(index))
                    .is_some_and(|&coverage| coverage > 0)
            })
        });
        if any {
            touched.push(*tile_id);
        }
    }
    touched
}

/// Assembles an image's `TILv` grid into one premultiplied RGBA
/// buffer, row-major in image space. Unlike [`gather`] this is the
/// *tile-domain* view (no straight-alpha round-trip) and matte slots
/// bake to the renderer's premultiplied fill — the decision-0034 law
/// shared by `document.rotate` and the place-image family. A missing
/// referenced tile is a refusal, not silent transparency.
pub(crate) fn assemble_rgba(
    document: &Document,
    image: &ImageObject,
    name: &'static str,
) -> Result<Vec<u8>, EngineError> {
    let (width, height) = check_editable(image, name)?;
    let side = raster::TILE_SIDE as u64;
    let cols = width.div_ceil(side);
    let mut flat = vec![0u8; (width * height * 4) as usize];
    for (slot, tile_ref) in image.tiles.iter().enumerate() {
        let (base_x, base_y) = ((slot as u64 % cols) * side, (slot as u64 / cols) * side);
        let copy_w = (width - base_x).min(side) as usize;
        let copy_h = (height - base_y).min(side) as usize;
        let solid = match tile_ref {
            TileRef::Tile(tile_id) => {
                let Some(tile) = document.tiles.get(*tile_id) else {
                    return Err(EngineError::InvalidState {
                        name: name.to_owned(),
                        detail: format!("tile {tile_id} is referenced but not loaded"),
                    });
                };
                for line in 0..copy_h {
                    let source = line * raster::TILE_SIDE * 4;
                    let target = ((base_y + line as u64) * width + base_x) as usize * 4;
                    flat[target..target + copy_w * 4]
                        .copy_from_slice(&tile.pixels()[source..source + copy_w * 4]);
                }
                continue;
            }
            TileRef::Matte(raw) => Some(compose::matte_premultiplied(*raw)),
        };
        let Some(pixel) = solid else { continue };
        for line in 0..copy_h {
            let target = ((base_y + line as u64) * width + base_x) as usize * 4;
            for column in 0..copy_w {
                flat[target + column * 4..target + column * 4 + 4].copy_from_slice(&pixel);
            }
        }
    }
    Ok(flat)
}

/// Tiles a premultiplied RGBA buffer (row-major, `width` × `height`)
/// over fresh `mkBT` bank slots — the flatten minting law (`flatten::
/// next_tile_id`). Returns the `TILv` list and the minted ids in slot
/// order; the caller captures them in a [`crate::delta::Delta::Bank`].
pub(crate) fn retile_rgba(
    document: &mut Document,
    pixels: &[u8],
    width: i64,
    height: i64,
) -> (Vec<TileRef>, Vec<u32>) {
    let side = raster::TILE_SIDE as i64;
    let cols = (width / side + i64::from(width % side != 0)).max(1);
    let rows = (height / side + i64::from(height % side != 0)).max(1);
    let mut minted = Vec::new();
    let mut tiles = Vec::with_capacity((cols * rows) as usize);
    for row in 0..rows {
        for col in 0..cols {
            let mut buffer = [0u8; raster::TILE_BYTES];
            let (base_x, base_y) = (col * side, row * side);
            let copy_w = (width - base_x).clamp(0, side) as usize;
            let copy_h = (height - base_y).clamp(0, side) as usize;
            for line in 0..copy_h {
                let source = ((base_y + line as i64) * width + base_x) as usize * 4;
                let target = line * raster::TILE_SIDE * 4;
                buffer[target..target + copy_w * 4]
                    .copy_from_slice(&pixels[source..source + copy_w * 4]);
            }
            let id = crate::catalog::flatten::next_tile_id(document);
            document.tiles.insert(id, raster::Tile::from_rgba8(buffer));
            minted.push(id);
            tiles.push(TileRef::Tile(id));
        }
    }
    (tiles, minted)
}

fn unpremultiply_all(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = pixel[3];
        if alpha == 0 || alpha == 255 {
            continue;
        }
        // Straight = premultiplied · 255 / alpha, rounded half-up; the
        // dividend is ≤ 255·255 + 127, which fits u16 math per step via
        // u32 intermediates.
        for channel in &mut pixel[..3] {
            *channel =
                ((u32::from(*channel) * 255 + u32::from(alpha) / 2) / u32::from(alpha)) as u8;
        }
    }
}

fn premultiply_all(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = pixel[3];
        if alpha == 255 {
            continue;
        }
        for channel in &mut pixel[..3] {
            *channel = ((u16::from(*channel) * u16::from(alpha) + 127) / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn image_object(width: i64, height: i64, tile_size: i64, tiles: Vec<TileRef>) -> ImageObject {
        ImageObject {
            location: geom::Point::ZERO,
            offset: geom::Point::ZERO,
            pixels: (width, height),
            tile_size,
            tiles,
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }
    }

    fn solid_tile(rgba: [u8; 4]) -> raster::Tile {
        let mut buffer = [0u8; raster::TILE_BYTES];
        for pixel in buffer.chunks_exact_mut(4) {
            pixel.copy_from_slice(&rgba);
        }
        raster::Tile::from_rgba8(buffer)
    }

    #[test]
    fn gather_round_trips_one_full_tile() {
        let image = image_object(128, 128, 128, vec![TileRef::Tile(0)]);
        let mut bank = TileBank::new();
        bank.insert(0, solid_tile([200, 100, 50, 255]));

        let flat = gather(&image, &bank, "test").unwrap();
        assert_eq!((flat.width, flat.height), (128, 128));
        assert_eq!(&flat.pixels[0..4], &[200, 100, 50, 255]);
    }

    #[test]
    fn gather_unpremultiplies_partial_alpha() {
        // Premultiplied (128, 0, 0, 128) → straight (255, 0, 0, 128).
        let image = image_object(128, 128, 128, vec![TileRef::Tile(0)]);
        let mut bank = TileBank::new();
        bank.insert(0, solid_tile([128, 0, 0, 128]));
        let flat = gather(&image, &bank, "test").unwrap();
        assert_eq!(&flat.pixels[0..4], &[255, 0, 0, 128]);
    }

    #[test]
    fn matte_slots_hold_their_grid_position() {
        // TILv [Matte, Tile]: the Tile sits in the RIGHT column, so its
        // pixels land at x=128.
        let image = image_object(
            256,
            128,
            128,
            vec![TileRef::Matte(0xFF_FF_FF_FF), TileRef::Tile(0)],
        );
        let mut bank = TileBank::new();
        bank.insert(0, solid_tile([10, 20, 30, 255]));
        let flat = gather(&image, &bank, "test").unwrap();
        assert_eq!(&flat.pixels[0..4], &[0, 0, 0, 0], "matte slot stays empty");
        assert_eq!(&flat.pixels[128 * 4..128 * 4 + 4], &[10, 20, 30, 255]);
    }

    #[test]
    fn missing_tile_is_a_refusal() {
        let image = image_object(128, 128, 128, vec![TileRef::Tile(7)]);
        let bank = TileBank::new();
        let error = gather(&image, &bank, "test").unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));
    }

    #[test]
    fn non_128_grids_and_oversized_images_are_refused() {
        let mut bank = TileBank::new();
        bank.insert(0, solid_tile([0, 0, 0, 0]));
        for image in [
            image_object(256, 128, 256, vec![TileRef::Tile(0)]),
            image_object(8192, 8192, 128, vec![TileRef::Tile(0)]),
            image_object(0, 8, 128, vec![TileRef::Tile(0)]),
        ] {
            let error = gather(&image, &bank, "test").unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }));
        }
    }

    #[test]
    fn scatter_writes_only_touched_tiles_and_premultiplies() {
        let image = image_object(256, 128, 128, vec![TileRef::Tile(0), TileRef::Tile(1)]);
        let mut bank = TileBank::new();
        bank.insert(0, solid_tile([0, 0, 0, 0]));
        bank.insert(1, solid_tile([0, 0, 0, 0]));

        let mut flat = FlatImage {
            width: 256,
            height: 128,
            pixels: vec![0; 256 * 128 * 4],
        };
        // Half-alpha red in tile 1's first pixel.
        flat.pixels[128 * 4..128 * 4 + 4].copy_from_slice(&[255, 0, 0, 128]);

        scatter(&flat, &image, &mut bank, &[1], "test").unwrap();
        let untouched = bank.get(0).unwrap().pixels()[0..4].to_vec();
        assert_eq!(untouched, [0, 0, 0, 0], "untouched tile must not change");
        let written = bank.get(1).unwrap().pixels()[0..4].to_vec();
        assert_eq!(written, [128, 0, 0, 128], "scatter re-premultiplies");
    }

    #[test]
    fn touched_tiles_maps_the_grid() {
        // 256×256 image, four tiles, mask over the right half.
        let image = image_object(
            256,
            256,
            128,
            vec![
                TileRef::Tile(3),
                TileRef::Tile(1),
                TileRef::Tile(2),
                TileRef::Tile(0),
            ],
        );
        let mut mask = doc::SelectionMask::empty(256, 256);
        for py in 0..256 {
            for px in 128..256 {
                mask.set(px, py, 255);
            }
        }
        // TILv slots: 0 = (0,0), 1 = (1,0), 2 = (0,1), 3 = (1,1); the
        // mask covers columns 128.. → slots 1 and 3 → ids 1 and 0.
        assert_eq!(touched_tiles(&image, &mask), vec![1, 0]);
    }
}
