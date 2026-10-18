//! JPEG export via `jpeg-encoder` 0.6 (0.7 needs Rust 1.87; the
//! workspace MSRV is 1.85).
//!
//! JPEG carries no alpha, so the plane is flattened over a matte
//! (default white — CS6's default) before encoding. Quality is the
//! standard 1..=100 independent-group scale; chroma subsampling is
//! 4:4:4 (sharp edges, default) or 4:2:0 (CS6's "smaller file"
//! posture). Dimension ceiling: 65535 per axis (the JPEG spec limit,
//! enforced by `u16` coordinates downstream).

use jpeg_encoder::{ColorType, Encoder as JpegEncoder, SamplingFactor};

use crate::{ExportError, Matte, Plane};

/// Chroma subsampling for YCbCr JPEG.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChromaSampling {
    /// Full-resolution chroma — sharpest on hard edges (default).
    #[default]
    S444,
    /// Half-resolution chroma — smaller files, softer color edges.
    S420,
}

/// JPEG encoder options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

impl Default for JpegOptions {
    fn default() -> Self {
        Self {
            quality: 90,
            progressive: false,
            chroma: ChromaSampling::default(),
            matte: Matte::default(),
        }
    }
}

/// Encodes a plane as JPEG, flattening alpha over `options.matte`.
///
/// # Errors
/// [`ExportError::TooLarge`] above 65535 per axis,
/// [`ExportError::Unsupported`] for quality 0,
/// [`ExportError::Encode`] when the encoder fails.
pub fn encode(plane: &Plane<'_>, options: &JpegOptions) -> Result<Vec<u8>, ExportError> {
    const LIMIT: u32 = 65535;
    if options.quality == 0 {
        return Err(ExportError::Unsupported {
            format: "jpeg",
            reason: "quality must be 1..=100".to_owned(),
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

    let rgb = plane.flatten_rgb(options.matte);
    let mut out = Vec::new();
    let mut encoder = JpegEncoder::new(&mut out, options.quality);
    encoder.set_progressive(options.progressive);
    encoder.set_sampling_factor(match options.chroma {
        ChromaSampling::S444 => SamplingFactor::F_1_1,
        ChromaSampling::S420 => SamplingFactor::F_2_2,
    });
    encoder
        .encode(
            &rgb,
            u16::try_from(plane.width).unwrap_or(LIMIT as u16),
            u16::try_from(plane.height).unwrap_or(LIMIT as u16),
            ColorType::Rgb,
        )
        .map_err(|error| ExportError::Encode {
            format: "jpeg",
            reason: error.to_string(),
        })?;
    Ok(out)
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
    }
}
