//! Raster import: sniff a raster stream by magic bytes, decode it to a
//! straight-alpha RGBA8 plane, and map it onto a fresh [`doc::Document`].
//!
//! Decoding is delegated to the `image` crate (JPEG via zune-jpeg,
//! GIF first frame, BMP, TIFF, WebP, PNG) with hard allocation limits;
//! WBMP is decoded by this crate's own [`crate::wbmp`] codec. The
//! document builder mirrors the engine's `page.add` skeleton (one
//! layer, one cell per state) and fills a 128×128 tile grid exactly
//! the way the `IMGv`/`TILv` model expects: premultiplied RGBA8
//! payloads, edge tiles zero-padded beyond the image bounds.

use std::io::Cursor;

use doc::{Cell, Document, Layer, LayerList, ObjectData, Page, PageInfo, TileRef};
use raster::{TILE_BYTES, TILE_SIDE, Tile};

use crate::{ImportError, Plane};

/// Defensive decode ceiling: no axis above 65535, no plane above
/// 400 MB of RGBA. The `image` crate enforces these before decoding.
const MAX_AXIS: u32 = 65535;
/// Allocation ceiling matching [`crate::wbmp`]'s pixel cap.
const MAX_ALLOC: usize = 400_000_000;

/// Where a decoded raster came from (recorded for diagnostics and
/// format-specific behaviors like GIF frame handling).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFormat {
    /// PNG signature.
    Png,
    /// JPEG `FF D8 FF`.
    Jpeg,
    /// GIF `GIF8`.
    Gif,
    /// BMP `BM`.
    Bmp,
    /// TIFF little- or big-endian.
    Tiff,
    /// WebP RIFF container.
    WebP,
    /// WBMP type 0 (decoded by [`crate::wbmp`]).
    Wbmp,
}

/// A decoded raster: straight-alpha RGBA8 pixels plus provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, straight alpha.
    pub rgba: Vec<u8>,
    /// Detected source format.
    pub source: SourceFormat,
}

impl DecodedImage {
    /// A validated [`Plane`] view over the pixels.
    ///
    /// # Errors
    /// [`ImportError::Decode`] if the stored dimensions and buffer
    /// disagree (a decode invariant; construction refuses mismatches).
    pub fn plane(&self) -> Result<Plane<'_>, ImportError> {
        Plane::new(self.width, self.height, &self.rgba).map_err(|error| ImportError::Decode {
            format: "raster",
            reason: error.to_string(),
        })
    }
}

/// Decodes any supported raster stream into straight RGBA8.
///
/// Animated GIFs decode to their first frame; per-state import is the
/// animated-GIF task (M8 task 7).
///
/// # Errors
/// [`ImportError::Unsupported`] for unknown signatures and
/// over-limit dimensions, [`ImportError::Decode`] for malformed
/// streams.
pub fn decode(input: &[u8]) -> Result<DecodedImage, ImportError> {
    let source = sniff(input)?;
    let name = match source {
        SourceFormat::Png => "png",
        SourceFormat::Jpeg => "jpeg",
        SourceFormat::Gif => "gif",
        SourceFormat::Bmp => "bmp",
        SourceFormat::Tiff => "tiff",
        SourceFormat::WebP => "webp",
        SourceFormat::Wbmp => "wbmp",
    };
    // Concrete decoders, because reader-level limits only reach the
    // PNG decoder in `image` 0.25 — every other format gets them via
    // `ImageDecoder::set_limits` (which refuses before allocating).
    let limits = || {
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_AXIS);
        limits.max_image_height = Some(MAX_AXIS);
        limits.max_alloc = Some(MAX_ALLOC as u64);
        limits
    };
    let limit_err = |error: image::ImageError| ImportError::Unsupported {
        format: name,
        reason: error.to_string(),
    };
    let decode_err = |error: image::ImageError| ImportError::Decode {
        format: name,
        reason: error.to_string(),
    };

    use image::ImageDecoder as _;
    let decoded = match source {
        SourceFormat::Png => {
            let mut decoder =
                image::codecs::png::PngDecoder::new(Cursor::new(input)).map_err(decode_err)?;
            decoder.set_limits(limits()).map_err(limit_err)?;
            image::DynamicImage::from_decoder(decoder).map_err(decode_err)?
        }
        SourceFormat::Jpeg => {
            let mut decoder =
                image::codecs::jpeg::JpegDecoder::new(Cursor::new(input)).map_err(decode_err)?;
            decoder.set_limits(limits()).map_err(limit_err)?;
            image::DynamicImage::from_decoder(decoder).map_err(decode_err)?
        }
        SourceFormat::Gif => {
            let mut decoder =
                image::codecs::gif::GifDecoder::new(Cursor::new(input)).map_err(decode_err)?;
            decoder.set_limits(limits()).map_err(limit_err)?;
            image::DynamicImage::from_decoder(decoder).map_err(decode_err)?
        }
        SourceFormat::Bmp => {
            let mut decoder =
                image::codecs::bmp::BmpDecoder::new(Cursor::new(input)).map_err(decode_err)?;
            decoder.set_limits(limits()).map_err(limit_err)?;
            image::DynamicImage::from_decoder(decoder).map_err(decode_err)?
        }
        SourceFormat::Tiff => {
            let mut decoder =
                image::codecs::tiff::TiffDecoder::new(Cursor::new(input)).map_err(decode_err)?;
            decoder.set_limits(limits()).map_err(limit_err)?;
            image::DynamicImage::from_decoder(decoder).map_err(decode_err)?
        }
        SourceFormat::WebP => {
            let mut decoder =
                image::codecs::webp::WebPDecoder::new(Cursor::new(input)).map_err(decode_err)?;
            decoder.set_limits(limits()).map_err(limit_err)?;
            image::DynamicImage::from_decoder(decoder).map_err(decode_err)?
        }
        SourceFormat::Wbmp => {
            let (width, height, rgba) = crate::wbmp::decode(input)?;
            let frame = image::RgbaImage::from_raw(width, height, rgba).ok_or_else(|| {
                ImportError::Decode {
                    format: "wbmp",
                    reason: "pixel buffer does not match the header".to_owned(),
                }
            })?;
            image::DynamicImage::ImageRgba8(frame)
        }
    };
    let rgba = decoded.to_rgba8();
    Ok(DecodedImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
        source,
    })
}

/// Decodes a raster and maps it onto a fresh single-page document:
/// canvas sized to the image, one layer, an `IMGv` object at the
/// origin spanning the pixels, and a full premultiplied tile bank.
///
/// # Errors
/// [`ImportError`] from [`decode`], or
/// [`ImportError::Unsupported`] when the tile grid would exceed
/// sanity (≥ 4096 tiles).
pub fn decode_to_document(input: &[u8], name: Option<String>) -> Result<Document, ImportError> {
    let decoded = decode(input)?;
    document_from_image(&decoded, name)
}

/// Maps an already-decoded image onto a fresh document.
///
/// # Errors
/// [`ImportError::Unsupported`] when the tile grid would exceed 4096
/// tiles.
pub fn document_from_image(
    image: &DecodedImage,
    name: Option<String>,
) -> Result<Document, ImportError> {
    let mut document = Document::default();
    document.info.frame_count = 1;
    document.info.canvas.width = f64::from(image.width);
    document.info.canvas.height = f64::from(image.height);
    let image_id = attach_image(&mut document, image.width, image.height, &image.rgba, name)?;
    let canvas = document.info.canvas.clone();
    build_single_object_page(&mut document, canvas, vec![image_id]);
    Ok(document)
}

/// Splits a straight-alpha RGBA plane into a premultiplied, zero-padded
/// tile bank and registers an `IMGv` object for it in `document`'s
/// registry. Shared by the raster import path and the SVG embedded-image
/// path. Tile ids are dense from 0, row-major.
///
/// # Errors
/// [`ImportError::Unsupported`] when the tile grid would exceed 4096
/// tiles or the plane length mismatches the dimensions.
pub(crate) fn attach_image(
    document: &mut Document,
    width: u32,
    height: u32,
    rgba: &[u8],
    name: Option<String>,
) -> Result<doc::ObjectId, ImportError> {
    let width = usize::try_from(width).unwrap_or(usize::MAX);
    let height = usize::try_from(height).unwrap_or(usize::MAX);
    let cols = width.div_ceil(TILE_SIDE);
    let rows = height.div_ceil(TILE_SIDE);
    let tile_count = cols
        .checked_mul(rows)
        .ok_or_else(|| ImportError::Unsupported {
            format: "raster",
            reason: "tile grid overflows".to_owned(),
        })?;
    if tile_count > 4096 {
        return Err(ImportError::Unsupported {
            format: "raster",
            reason: format!("{tile_count} tiles exceeds the 4096-tile ceiling"),
        });
    }
    if rgba.len() != width * height * 4 {
        return Err(ImportError::Decode {
            format: "raster",
            reason: format!(
                "image plane holds {} bytes, {width}x{height} needs {}",
                rgba.len(),
                width * height * 4
            ),
        });
    }

    // Tile references, row-major; payloads follow below. Ids are
    // global bank slots — local per-image numbering would make every
    // import clobber the first bank tile.
    let base = document.tiles.len();
    let last = base
        .checked_add(tile_count)
        .ok_or_else(|| ImportError::Unsupported {
            format: "raster",
            reason: "tile grid overflows".to_owned(),
        })?;
    if u64::try_from(last).unwrap_or(u64::MAX) > u64::from(u32::MAX) + 1 {
        return Err(ImportError::Unsupported {
            format: "raster",
            reason: format!("{last} bank slots exceed the u32 tile id space"),
        });
    }
    let mut tiles = Vec::with_capacity(tile_count);
    for index in 0..tile_count {
        let id = u32::try_from(base + index).unwrap_or(u32::MAX);
        tiles.push(TileRef::Tile(id));
    }

    let image_id = document
        .registry
        .insert(ObjectData::Image(doc::ImageObject {
            location: geom::Point::ZERO,
            offset: geom::Point::ZERO,
            pixels: (
                i64::try_from(width).unwrap_or(i64::MAX),
                i64::try_from(height).unwrap_or(i64::MAX),
            ),
            tile_size: TILE_SIDE as i64,
            tiles,
            name,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }));

    // Premultiplied tile payloads with zero padding past the image
    // edge (same policy as the engine's bitmap gather/scatter: tiles
    // are always full 128×128 planes).
    for row in 0..rows {
        for col in 0..cols {
            let id = u32::try_from(base + row * cols + col).unwrap_or(u32::MAX);
            let mut buffer = [0u8; TILE_BYTES];
            for py in 0..TILE_SIDE {
                let y = row * TILE_SIDE + py;
                if y >= height {
                    break;
                }
                for px in 0..TILE_SIDE {
                    let x = col * TILE_SIDE + px;
                    if x >= width {
                        break;
                    }
                    let src = (y * width + x) * 4;
                    let a = u32::from(rgba[src + 3]);
                    let dst = (py * TILE_SIDE + px) * 4;
                    for channel in 0..3 {
                        buffer[dst + channel] =
                            ((u32::from(rgba[src + channel]) * a + 127) / 255) as u8;
                    }
                    buffer[dst + 3] = rgba[src + 3];
                }
            }
            document.tiles.insert(id, Tile::from_rgba8(buffer));
        }
    }
    Ok(image_id)
}

/// A one-page, one-layer document skeleton whose single cell holds
/// `elements` in z-order (the raster import shape — also reused by the
/// SVG import and the PSD writer's single-object probe render).
pub fn build_single_object_page(
    document: &mut Document,
    canvas: doc::Canvas,
    elements: Vec<doc::ObjectId>,
) {
    let cell = Cell {
        locked: false,
        visible: true,
        elements,
    };
    let layer = Layer {
        name: "Layer 1".to_owned(),
        display: true,
        cells: vec![cell],
        ..Layer::default()
    };
    document.pages.push(Page {
        info: PageInfo {
            version: document.info.version,
            revision: document.info.revision,
            frame_count: 1,
            frame_count_duplicate: 1,
            request_id: 0,
            canvas,
            onion_before: 0,
            onion_after: 0,
        },
        name: None,
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
    });
}

/// Detects the format by magic bytes (never by file extension).
///
/// # Errors
/// [`ImportError::Unsupported`] when no known signature matches.
pub fn sniff(input: &[u8]) -> Result<SourceFormat, ImportError> {
    if input.len() < 4 {
        return Err(ImportError::Truncated {
            format: "raster",
            needed: "at least 4 signature bytes",
        });
    }
    if input.starts_with(b"\x89PNG") {
        return Ok(SourceFormat::Png);
    }
    if input.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Ok(SourceFormat::Jpeg);
    }
    if input.starts_with(b"GIF8") {
        return Ok(SourceFormat::Gif);
    }
    if input.starts_with(b"BM") {
        return Ok(SourceFormat::Bmp);
    }
    if input.starts_with(b"II") && input.get(2..4) == Some(&[0x2A, 0x00])
        || input.starts_with(b"MM") && input.get(2..4) == Some(&[0x00, 0x2A])
    {
        return Ok(SourceFormat::Tiff);
    }
    if input.starts_with(b"RIFF") && input.get(8..12) == Some(b"WEBP") {
        return Ok(SourceFormat::WebP);
    }
    // WBMP type 0: zero type octet, zero fixed-header octet.
    if input[0] == 0 && input[1] == 0 {
        return Ok(SourceFormat::Wbmp);
    }
    Err(ImportError::Unsupported {
        format: "raster",
        reason: format!(
            "unrecognized signature {:02x} {:02x} {:02x} {:02x}",
            input[0], input[1], input[2], input[3]
        ),
    })
}
