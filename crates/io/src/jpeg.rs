//! JPEG export via `jpeg-encoder` 0.6. The workspace MSRV is 1.95,
//! so 0.7 (which needs 1.87) is permitted — 0.6 is kept anyway so
//! exported bytes do not drift with an encoder bump.
//!
//! JPEG carries no alpha, so the plane is flattened over a matte
//! (default white — CS6's default) before encoding. Quality is the
//! standard 1..=100 independent-group scale; chroma subsampling is
//! 4:4:4 (sharp edges, default) or 4:2:0 (CS6's "smaller file"
//! posture). Dimension ceiling: 65535 per axis (the JPEG spec limit,
//! enforced by `u16` coordinates downstream).
//!
//! CS6's *selective* quality compresses the masked region of the
//! image at its own quality and the rest at the main one. A JPEG scan
//! carries one quantization table per component, so one scan cannot
//! hold two qualities; the encoder therefore encodes the image at each
//! of the two qualities, decodes both back, blends them per the mask,
//! and encodes the blend once more (decision 0122).

use doc::SelectionMask;
use jpeg_encoder::{ColorType, Encoder as JpegEncoder, SamplingFactor};

use crate::{ExportError, Matte, Plane};

/// The JPEG spec's dimension ceiling, per axis.
const LIMIT: u32 = 65535;

/// Chroma subsampling for YCbCr JPEG.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChromaSampling {
    /// Full-resolution chroma — sharpest on hard edges (default).
    #[default]
    S444,
    /// Half-resolution chroma — smaller files, softer color edges.
    S420,
}

/// The selective-JPEG settings: the quality the masked region of the
/// image is compressed at, and the mask that says which region that
/// is.
///
/// CS6's rule, from its reference manual ("Selectively compress areas
/// of a JPEG"): the *selected* area is compressed at the entered
/// "Selective quality" value and the rest of the image at the main
/// Quality slider's value — so a low value compresses the selected
/// area more than the rest of the image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectiveJpeg {
    /// The quality the masked region is compressed at (1..=100).
    pub quality: u8,
    /// The mask: one coverage byte per pixel (255 = masked, 0 = not).
    /// The plane spans the whole image, so a mask recorded at another
    /// size simply leaves the pixels it does not cover unmasked.
    pub mask: SelectionMask,
}

/// JPEG encoder options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JpegOptions {
    /// Quality 1..=100 (`jpeg-encoder` clamps internally; we refuse
    /// 0 up front so the scale stays CS6-shaped).
    pub quality: u8,
    /// Progressive scans instead of baseline.
    pub progressive: bool,
    /// Chroma subsampling.
    pub chroma: ChromaSampling,
    /// Matte the (alpha-carrying) plane is flattened over.
    pub matte: Matte,
    /// The selective-JPEG settings, when the export asks for them.
    pub selective: Option<SelectiveJpeg>,
}

impl Default for JpegOptions {
    fn default() -> Self {
        Self {
            quality: 90,
            progressive: false,
            chroma: ChromaSampling::default(),
            matte: Matte::default(),
            selective: None,
        }
    }
}

/// Encodes a plane as JPEG, flattening alpha over `options.matte`.
///
/// With `options.selective` set the masked region is compressed at the
/// selective quality and the rest at the main one.
///
/// # Errors
/// [`ExportError::TooLarge`] above 65535 per axis,
/// [`ExportError::Unsupported`] for quality 0,
/// [`ExportError::Encode`] when the encoder fails.
pub fn encode(plane: &Plane<'_>, options: &JpegOptions) -> Result<Vec<u8>, ExportError> {
    if options.quality == 0 {
        return Err(ExportError::Unsupported {
            format: "jpeg",
            reason: "quality must be 1..=100".to_owned(),
        });
    }
    if options
        .selective
        .as_ref()
        .is_some_and(|sel| sel.quality == 0)
    {
        return Err(ExportError::Unsupported {
            format: "jpeg",
            reason: "the selective quality must be 1..=100".to_owned(),
        });
    }
    if plane.width > LIMIT || plane.height > LIMIT {
        return Err(ExportError::TooLarge {
            format: "jpeg",
            width: plane.width,
            height: plane.height,
            limit: LIMIT,
        });
    }
    if let Some(selective) = &options.selective {
        return encode_selective(plane, options, selective);
    }
    let rgb = plane.flatten_rgb(options.matte);
    encode_rgb(&rgb, plane.width, plane.height, options, options.quality)
}

/// Encodes RGB bytes at a given quality.
fn encode_rgb(
    rgb: &[u8],
    width: u32,
    height: u32,
    options: &JpegOptions,
    quality: u8,
) -> Result<Vec<u8>, ExportError> {
    let mut out = Vec::new();
    let mut encoder = JpegEncoder::new(&mut out, quality);
    encoder.set_progressive(options.progressive);
    encoder.set_sampling_factor(match options.chroma {
        ChromaSampling::S444 => SamplingFactor::F_1_1,
        ChromaSampling::S420 => SamplingFactor::F_2_2,
    });
    encoder
        .encode(
            rgb,
            u16::try_from(width).unwrap_or(LIMIT as u16),
            u16::try_from(height).unwrap_or(LIMIT as u16),
            ColorType::Rgb,
        )
        .map_err(|error| ExportError::Encode {
            format: "jpeg",
            reason: error.to_string(),
        })?;
    Ok(out)
}

/// Encodes a plane with CS6's selective quality: the masked region at
/// the entered quality, the rest at the main one.
///
/// The two regions cannot share one scan, so the image is encoded
/// twice — once at each quality — decoded back, and the two decoded
/// versions are blended per the mask's coverage. The blend is encoded
/// once more at the better of the two qualities, so the region that
/// was compressed well is not degraded a second time by the final
/// pass. The well-preserved region therefore pays one extra
/// generation and the compressed region pays the second one; CS6's
/// own byte layout is not reproduced (decision 0122).
fn encode_selective(
    plane: &Plane<'_>,
    options: &JpegOptions,
    selective: &SelectiveJpeg,
) -> Result<Vec<u8>, ExportError> {
    let rgb = plane.flatten_rgb(options.matte);
    let selective_bytes = encode_rgb(&rgb, plane.width, plane.height, options, selective.quality)?;
    let main_bytes = encode_rgb(&rgb, plane.width, plane.height, options, options.quality)?;
    let selective_pixels = decode_rgb(&selective_bytes, plane.width, plane.height)?;
    let main_pixels = decode_rgb(&main_bytes, plane.width, plane.height)?;

    let mut blended = rgb.clone();
    for (index, pixel) in blended.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let coverage = u16::from(coverage_at(&selective.mask, plane.width, index));
        let base = index * 3;
        for channel in 0..3 {
            let main = u16::from(main_pixels[base + channel]);
            let masked = u16::from(selective_pixels[base + channel]);
            pixel[channel] = ((main * (255 - coverage) + masked * coverage) / 255) as u8;
        }
    }
    let final_quality = options.quality.max(selective.quality);
    encode_rgb(&blended, plane.width, plane.height, options, final_quality)
}

/// Decodes JPEG bytes back into RGB, for the selective blend.
///
/// # Errors
/// [`ExportError::Encode`] when the stream cannot be decoded or comes
/// back at another size.
fn decode_rgb(bytes: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ExportError> {
    let decoded = crate::raster::decode(bytes).map_err(|error| ExportError::Encode {
        format: "jpeg",
        reason: error.to_string(),
    })?;
    if decoded.width != width || decoded.height != height {
        return Err(ExportError::Encode {
            format: "jpeg",
            reason: format!(
                "the decoded image is {}x{}, expected {width}x{height}",
                decoded.width, decoded.height
            ),
        });
    }
    let mut rgb = Vec::with_capacity(decoded.rgba.len() / 4 * 3);
    for pixel in decoded.rgba.as_chunks::<4>().0 {
        rgb.push(pixel[0]);
        rgb.push(pixel[1]);
        rgb.push(pixel[2]);
    }
    Ok(rgb)
}

/// The mask's coverage of the pixel at `index` (row-major, RGB
/// triplets): 0..=255, or 0 when the mask does not span the pixel.
fn coverage_at(mask: &SelectionMask, width: u32, index: usize) -> u8 {
    if width == 0 {
        return 0;
    }
    let x = u32::try_from(index % usize::try_from(width).unwrap_or(1)).unwrap_or(0);
    let y = u32::try_from(index / usize::try_from(width).unwrap_or(1)).unwrap_or(0);
    mask.get(x, y).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    /// White 1×1 opaque plane.
    fn white_plane() -> Plane<'static> {
        Plane::new(1, 1, &[255u8, 255, 255, 255]).unwrap()
    }

    #[test]
    fn encode_rejects_quality_zero() {
        let plane = white_plane();
        let opts = JpegOptions {
            quality: 0,
            ..Default::default()
        };
        let err = encode(&plane, &opts).unwrap_err();
        assert!(
            matches!(err, ExportError::Unsupported { format: "jpeg", .. }
                       if format!("{err}").contains("quality"))
        );
    }

    #[test]
    fn encode_rejects_oversized_dimensions() {
        // 65536 wide × 1 high × 4 bytes/pixel = 262144 bytes.
        let width = 65536usize;
        let height = 1usize;
        let rgba = vec![0u8; width * height * 4];
        let plane = Plane::new(width as u32, height as u32, &rgba).unwrap();
        let err = encode(&plane, &JpegOptions::default()).unwrap_err();
        assert!(matches!(err, ExportError::TooLarge { format: "jpeg", .. }));
    }

    #[test]
    fn encode_s444_vs_s420_produce_different_sizes() {
        let plane = white_plane();
        let opts_444 = JpegOptions {
            chroma: ChromaSampling::S444,
            ..Default::default()
        };
        let opts_420 = JpegOptions {
            chroma: ChromaSampling::S420,
            ..Default::default()
        };
        let out_444 = encode(&plane, &opts_444).unwrap();
        let out_420 = encode(&plane, &opts_420).unwrap();
        // Same content but different subsampling → different byte counts.
        // Both should be valid JPEG (image crate accepts them).
        assert_ne!(out_444.len(), out_420.len());
    }

    #[test]
    fn jpeg_options_default_has_quality_90() {
        let opts = JpegOptions::default();
        assert_eq!(opts.quality, 90);
        assert_eq!(opts.chroma, ChromaSampling::S444);
        assert!(opts.selective.is_none());
    }
}

/// CS6's selective quality, as a test sees it.
#[cfg(test)]
mod selective {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::raster;

    /// The side of the image the mask covers.
    const SIDE: u32 = 64;

    /// An image of hard-edged blocks: the content a low JPEG quality
    /// loses and a high one keeps.
    fn blocks() -> Vec<u8> {
        let mut rgba = Vec::with_capacity((SIDE * SIDE * 4) as usize);
        let mut seed = 0x1234_5678u32;
        for y in 0..SIDE {
            for x in 0..SIDE {
                // A deterministic value per 8×8 block, so the image
                // has flat areas with hard edges between them.
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let level = if (x / 8 + y / 8) % 2 == 0 {
                    (seed >> 24) as u8
                } else {
                    255 - (seed >> 24) as u8
                };
                rgba.push(level);
                rgba.push(level / 2);
                rgba.push(255 - level);
                rgba.push(255);
            }
        }
        rgba
    }

    /// A mask that covers the left half of the image.
    fn left_half() -> SelectionMask {
        let mut mask = SelectionMask::empty(SIDE, SIDE);
        for y in 0..SIDE {
            for x in 0..SIDE / 2 {
                mask.set(x, y, 255);
            }
        }
        mask
    }

    /// The peak-signal-to-noise ratio in dB between the original and
    /// the decoded image, over the columns `x_from..x_to`.
    fn psnr(original: &[u8], decoded: &[u8], x_from: u32, x_to: u32) -> f64 {
        let mut squared = 0.0f64;
        let mut count = 0.0f64;
        for (index, (before, after)) in original
            .chunks_exact(4)
            .zip(decoded.chunks_exact(4))
            .enumerate()
        {
            let x = u32::try_from(index % SIDE as usize).unwrap_or(0);
            if x < x_from || x >= x_to {
                continue;
            }
            for channel in 0..3 {
                let difference = f64::from(i16::from(before[channel]) - i16::from(after[channel]));
                squared += difference * difference;
                count += 1.0;
            }
        }
        assert!(count > 0.0, "the region holds pixels");
        let mse = squared / count;
        if mse <= 0.0 {
            return 100.0;
        }
        10.0 * (255.0f64 * 255.0 / mse).log10()
    }

    #[test]
    fn the_masked_region_keeps_the_entered_quality() {
        let original = blocks();
        let plane = Plane::new(SIDE, SIDE, &original).unwrap();
        let mask = left_half();
        let low = encode(
            &plane,
            &JpegOptions {
                quality: 50,
                ..Default::default()
            },
        )
        .expect("the plain low-quality encode");
        let high = encode(
            &plane,
            &JpegOptions {
                quality: 90,
                ..Default::default()
            },
        )
        .expect("the plain high-quality encode");
        let selective = encode(
            &plane,
            &JpegOptions {
                quality: 50,
                selective: Some(SelectiveJpeg {
                    quality: 90,
                    mask: mask.clone(),
                }),
                ..Default::default()
            },
        )
        .expect("the selective encode");

        let low_image = raster::decode(&low).expect("decode");
        let high_image = raster::decode(&high).expect("decode");
        let selective_image = raster::decode(&selective).expect("decode");

        let inside = psnr(&original, &selective_image.rgba, 0, SIDE / 2);
        let outside = psnr(&original, &selective_image.rgba, SIDE / 2, SIDE);
        let inside_low = psnr(&original, &low_image.rgba, 0, SIDE / 2);
        let inside_high = psnr(&original, &high_image.rgba, 0, SIDE / 2);

        // The masked region carries the entered quality, not the main
        // one.
        assert!(
            inside > inside_low + 3.0,
            "the masked region is {inside} dB, the main quality alone gives {inside_low} dB"
        );
        // The masked region pays the composite's extra generation, so
        // it lands close to a plain encode at the entered quality.
        assert!(
            inside > inside_high - 4.0,
            "the masked region is {inside} dB, a plain encode at that quality gives \
             {inside_high} dB"
        );
        // The rest of the image carries the main quality, which is the
        // lower one here.
        assert!(
            outside < inside - 3.0,
            "the unmasked region is {outside} dB, the masked region is {inside} dB"
        );
    }

    #[test]
    fn a_lower_entered_quality_compresses_the_masked_region_more() {
        let original = blocks();
        let plane = Plane::new(SIDE, SIDE, &original).unwrap();
        let mask = left_half();
        let high = encode(
            &plane,
            &JpegOptions {
                quality: 90,
                ..Default::default()
            },
        )
        .expect("the plain high-quality encode");
        let low = encode(
            &plane,
            &JpegOptions {
                quality: 50,
                ..Default::default()
            },
        )
        .expect("the plain low-quality encode");
        let selective = encode(
            &plane,
            &JpegOptions {
                quality: 90,
                selective: Some(SelectiveJpeg {
                    quality: 50,
                    mask: mask.clone(),
                }),
                ..Default::default()
            },
        )
        .expect("the selective encode");

        let high_image = raster::decode(&high).expect("decode");
        let low_image = raster::decode(&low).expect("decode");
        let selective_image = raster::decode(&selective).expect("decode");

        let inside = psnr(&original, &selective_image.rgba, 0, SIDE / 2);
        let outside = psnr(&original, &selective_image.rgba, SIDE / 2, SIDE);
        let inside_low = psnr(&original, &low_image.rgba, 0, SIDE / 2);
        let inside_high = psnr(&original, &high_image.rgba, 0, SIDE / 2);

        // The masked region now carries the lower quality.
        assert!(
            inside < inside_high - 3.0,
            "the masked region is {inside} dB, the main quality alone gives {inside_high} dB"
        );
        // ... and it lands close to a plain encode at that quality.
        assert!(
            inside > inside_low - 4.0 && inside < inside_low + 4.0,
            "the masked region is {inside} dB, a plain encode at that quality gives \
             {inside_low} dB"
        );
        // The rest of the image keeps the main quality.
        assert!(
            outside > inside + 3.0,
            "the unmasked region is {outside} dB, the masked region is {inside} dB"
        );
    }

    #[test]
    fn a_mask_that_covers_nothing_leaves_the_main_quality_alone() {
        let original = blocks();
        let plane = Plane::new(SIDE, SIDE, &original).unwrap();
        let mask = SelectionMask::empty(SIDE, SIDE);
        let plain = encode(
            &plane,
            &JpegOptions {
                quality: 50,
                ..Default::default()
            },
        )
        .expect("the plain encode");
        let selective = encode(
            &plane,
            &JpegOptions {
                quality: 50,
                selective: Some(SelectiveJpeg {
                    quality: 90,
                    mask: mask.clone(),
                }),
                ..Default::default()
            },
        )
        .expect("the selective encode");

        let plain_image = raster::decode(&plain).expect("decode");
        let selective_image = raster::decode(&selective).expect("decode");
        let plain_psnr = psnr(&original, &plain_image.rgba, 0, SIDE);
        let selective_psnr = psnr(&original, &selective_image.rgba, 0, SIDE);
        // Nothing is masked, so the image is the main quality's, even
        // though the composite encodes it once more.
        assert!(
            selective_psnr > plain_psnr - 4.0,
            "the unmasked image is {selective_psnr} dB, the plain encode gives {plain_psnr} dB"
        );
    }
}
