//! `FireCraft` import/export codecs (L4).
//!
//! A pure data plane: byte buffers in, byte buffers out, no `std::fs`
//! and no shell concerns — the engine's platform-services traits and
//! the L5 shells own file access. Raster exporters speak straight
//! (un-premultiplied) RGBA8 planes; the caller composites with
//! `compose::render` + `compose::straight_alpha` first.
//!
//! Formats grow milestone by milestone (M8): PNG/JPEG/BMP/TIFF/WBMP
//! save first, then raster import, SVG, PSD, and animated GIF ↔
//! states. See `docs/planning/firecraft/M8-import-export.md`.

pub mod bmp;
pub mod expv;
pub mod gif;
pub mod jpeg;
pub mod png;
pub mod psd;
pub mod raster;
pub mod svg;
pub mod svgimport;
pub mod tiff;
pub mod wbmp;
pub mod webp;
#[cfg(feature = "lossy-webp")]
pub mod webp_lossy;

/// A borrowed straight-alpha RGBA8 image plane.
///
/// Every exporter takes one of these; `rgba` must hold exactly
/// `width * height * 4` bytes (validated on construction).
#[derive(Clone, Copy, Debug)]
pub struct Plane<'a> {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, straight alpha.
    pub rgba: &'a [u8],
}

impl<'a> Plane<'a> {
    /// Builds a plane, refusing mismatched dimensions.
    ///
    /// # Errors
    /// [`ExportError::PlaneSize`] when `rgba` does not hold
    /// `width * height * 4` bytes.
    pub fn new(width: u32, height: u32, rgba: &'a [u8]) -> Result<Self, ExportError> {
        let expected = usize::try_from(width)
            .ok()
            .and_then(|w| w.checked_mul(usize::try_from(height).unwrap_or(usize::MAX)))
            .and_then(|pixels| pixels.checked_mul(4));
        match expected {
            Some(len) if len == rgba.len() => Ok(Self {
                width,
                height,
                rgba,
            }),
            _ => Err(ExportError::PlaneSize {
                width,
                height,
                len: rgba.len(),
            }),
        }
    }

    /// The pixel at (`x`, `y`) as `[r, g, b, a]`.
    fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        let mut px = [0u8; 4];
        px.copy_from_slice(&self.rgba[i..i + 4]);
        px
    }

    /// Luma of the pixel at (`x`, `y`) flattened over `matte`, under
    /// the house integer policy `(77r + 151g + 28b + 127) / 255`
    /// (ITU-R 601 weights, half-up — same policy as the `filters`
    /// kernels).
    #[must_use]
    pub fn luma_at(&self, x: u32, y: u32, matte: Matte) -> u8 {
        let px = self.pixel(x, y);
        let r = flatten_channel(px[0], px[3], matte.red_channel());
        let g = flatten_channel(px[1], px[3], matte.green_channel());
        let b = flatten_channel(px[2], px[3], matte.blue_channel());
        u8::try_from((u32::from(r) * 77 + u32::from(g) * 151 + u32::from(b) * 28 + 127) / 255)
            .unwrap_or(255)
    }

    /// The plane flattened to opaque RGB over `matte` (for
    /// alpha-less formats).
    #[must_use]
    pub fn flatten_rgb(&self, matte: Matte) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(self.rgba.len() / 4 * 3);
        for px in self.rgba.chunks_exact(4) {
            rgb.push(flatten_channel(px[0], px[3], matte.red_channel()));
            rgb.push(flatten_channel(px[1], px[3], matte.green_channel()));
            rgb.push(flatten_channel(px[2], px[3], matte.blue_channel()));
        }
        rgb
    }
}

/// Backing color used when a format cannot carry alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Matte {
    /// Flatten over opaque white (CS6's default matte).
    #[default]
    White,
    /// Flatten over opaque black.
    Black,
    /// Flatten over an explicit color — the `EXPv` `FCLi` matte
    /// (red, green, blue).
    Rgb(u8, u8, u8),
}

impl Matte {
    fn red_channel(self) -> u8 {
        match self {
            Matte::White => 255,
            Matte::Black => 0,
            Matte::Rgb(r, _, _) => r,
        }
    }

    fn green_channel(self) -> u8 {
        match self {
            Matte::White => 255,
            Matte::Black => 0,
            Matte::Rgb(_, g, _) => g,
        }
    }

    fn blue_channel(self) -> u8 {
        match self {
            Matte::White => 255,
            Matte::Black => 0,
            Matte::Rgb(_, _, b) => b,
        }
    }
}

/// Blends one straight-alpha channel over a matte: `c*a + m*(1-a)`.
pub(crate) fn flatten_channel(channel: u8, alpha: u8, matte: u8) -> u8 {
    let a = u32::from(alpha);
    let c = u32::from(channel);
    let m = u32::from(matte);
    u8::try_from((c * a + m * (255 - a) + 127) / 255).unwrap_or(255)
}

/// Everything that can go wrong while exporting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportError {
    /// The plane's byte length does not match its dimensions.
    PlaneSize {
        /// Declared width.
        width: u32,
        /// Declared height.
        height: u32,
        /// Actual byte length.
        len: usize,
    },
    /// The format's dimension ceiling was exceeded.
    TooLarge {
        /// Format that refused the plane.
        format: &'static str,
        /// Plane width.
        width: u32,
        /// Plane height.
        height: u32,
        /// Per-axis ceiling of the format.
        limit: u32,
    },
    /// The format cannot represent the requested layout.
    Unsupported {
        /// Format that refused the request.
        format: &'static str,
        /// What cannot be represented.
        reason: String,
    },
    /// The encoder rejected the stream it was building.
    Encode {
        /// Format being written.
        format: &'static str,
        /// Encoder-reported failure.
        reason: String,
    },
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::PlaneSize { width, height, len } => write!(
                f,
                "plane {width}x{height} needs {} bytes, got {len}",
                u64::from(*width) * u64::from(*height) * 4
            ),
            ExportError::TooLarge {
                format,
                width,
                height,
                limit,
            } => write!(
                f,
                "{format} caps dimensions at {limit}: got {width}x{height}"
            ),
            ExportError::Unsupported { format, reason } => {
                write!(f, "{format} export unsupported: {reason}")
            }
            ExportError::Encode { format, reason } => write!(f, "{format} encode failed: {reason}"),
        }
    }
}

impl std::error::Error for ExportError {}

/// Everything that can go wrong while importing (grows with M8 task 2+).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    /// The stream ended before the format's header completed.
    Truncated {
        /// Format being read.
        format: &'static str,
        /// What the header still needed.
        needed: &'static str,
    },
    /// The stream is well-formed but outside the supported subset.
    Unsupported {
        /// Format being read.
        format: &'static str,
        /// What is unsupported and why.
        reason: String,
    },
    /// The stream is malformed.
    Decode {
        /// Format being read.
        format: &'static str,
        /// Where and why it is malformed.
        reason: String,
    },
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Truncated { format, needed } => {
                write!(f, "{format} stream truncated: {needed}")
            }
            ImportError::Unsupported { format, reason } => {
                write!(f, "{format} import unsupported: {reason}")
            }
            ImportError::Decode { format, reason } => write!(f, "{format} decode failed: {reason}"),
        }
    }
}

impl std::error::Error for ImportError {}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn error_display_spells_each_variant() {
        let truncated = ImportError::Truncated {
            format: "psd",
            needed: "more section bytes than the stream holds",
        };
        assert_eq!(
            truncated.to_string(),
            "psd stream truncated: more section bytes than the stream holds"
        );
        let unsupported = ImportError::Unsupported {
            format: "gif",
            reason: "zero axis".to_owned(),
        };
        assert_eq!(unsupported.to_string(), "gif import unsupported: zero axis");
        let decode = ImportError::Decode {
            format: "wbmp",
            reason: "bad header".to_owned(),
        };
        assert_eq!(decode.to_string(), "wbmp decode failed: bad header");
    }

    // ── Plane ────────────────────────────────────────────────────────────────

    #[test]
    fn plane_new_rejects_mismatched_bytes() {
        // 2×2 needs 16 bytes, not 8.
        let e = Plane::new(2, 2, &[0u8; 8]).unwrap_err();
        assert!(matches!(
            e,
            ExportError::PlaneSize {
                width: 2,
                height: 2,
                len: 8
            }
        ));
    }

    #[test]
    fn plane_new_accepts_exact_bytes() {
        let p = Plane::new(3, 2, &[0u8; 24]).unwrap();
        assert_eq!(p.width, 3);
        assert_eq!(p.height, 2);
    }

    #[test]
    fn plane_pixel_returns_correct_bytes() {
        let rgba = [0x11u8, 0x22, 0x33, 0x44];
        let p = Plane::new(1, 1, &rgba).unwrap();
        assert_eq!(p.pixel(0, 0), [0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn plane_luma_at_opaque_white() {
        // Opaque white → flatten_channel(255,255,255)=255 for all channels
        // → luma = (255*77 + 255*151 + 255*28 + 127)/255 = 255
        let rgba = [255u8, 255, 255, 255];
        let p = Plane::new(1, 1, &rgba).unwrap();
        assert_eq!(p.luma_at(0, 0, Matte::White), 255);
    }

    #[test]
    fn plane_luma_at_opaque_black() {
        // Opaque black → flatten_channel(0,255,0)=0 for all channels
        // → luma = (0*77 + 0*151 + 0*28 + 127)/255 = 0
        let rgba = [0u8, 0, 0, 255];
        let p = Plane::new(1, 1, &rgba).unwrap();
        assert_eq!(p.luma_at(0, 0, Matte::White), 0);
    }

    #[test]
    fn plane_luma_at_gray_is_deterministic() {
        // ITU-R 601: (77R + 151G + 28B + 128) / 255, half-up
        // R=G=B=128 → (128*256 + 128) / 255 = 329 * 128 / 255 = 129
        let rgba = [128u8, 128, 128, 255];
        let p = Plane::new(1, 1, &rgba).unwrap();
        assert_eq!(p.luma_at(0, 0, Matte::White), 129);
    }

    #[test]
    fn plane_flatten_rgb_strips_alpha() {
        let rgba = [255u8, 0, 0, 128]; // semi-transparent red
        let p = Plane::new(1, 1, &rgba).unwrap();
        let rgb = p.flatten_rgb(Matte::White);
        // flatten_channel(255,128,255) = (255*128 + 255*127)/255 ≈ 128
        assert_eq!(rgb.len(), 3);
        assert_eq!(rgb[0], 255); // red channel stays red
    }

    // ── Matte ────────────────────────────────────────────────────────────────

    #[test]
    fn matte_default_is_white() {
        assert_eq!(Matte::default(), Matte::White);
    }

    #[test]
    fn matte_rgb_extracts_channels() {
        let m = Matte::Rgb(10, 20, 30);
        assert_eq!(m.red_channel(), 10);
        assert_eq!(m.green_channel(), 20);
        assert_eq!(m.blue_channel(), 30);
    }

    // ── flatten_channel ───────────────────────────────────────────────────────

    #[test]
    fn flatten_channel_opaque_white_over_white() {
        // c=255, a=255, m=255 → (255*255 + 255*0 + 127)/255 = 255
        assert_eq!(super::flatten_channel(255, 255, 255), 255);
    }

    #[test]
    fn flatten_channel_opaque_black_over_white() {
        // c=0, a=255, m=255 → (0*255 + 255*0 + 127)/255 = 0
        assert_eq!(super::flatten_channel(0, 255, 255), 0);
    }

    #[test]
    fn flatten_channel_opaque_color_over_black() {
        // c=255, a=255, m=0 → (255*255 + 0*0 + 127)/255 = 255
        assert_eq!(super::flatten_channel(255, 255, 0), 255);
    }

    #[test]
    fn flatten_channel_50_percent_alpha() {
        // c=255, a=64 (25% alpha), m=255
        // → (255*64 + 255*191 + 127)/255 = 255
        let result = super::flatten_channel(255, 64, 255);
        assert_eq!(result, 255);
    }

    // ── ExportError Display ──────────────────────────────────────────────────

    #[test]
    fn export_error_plane_size_display() {
        let e = ExportError::PlaneSize {
            width: 2,
            height: 3,
            len: 16,
        };
        let s = format!("{e}");
        assert!(s.contains("2x3"));
        assert!(s.contains("16"));
    }

    #[test]
    fn export_error_too_large_display() {
        let e = ExportError::TooLarge {
            format: "bmp",
            width: 100,
            height: 100,
            limit: 50,
        };
        let s = format!("{e}");
        assert!(s.contains("bmp"));
        assert!(s.contains("100"));
        assert!(s.contains("50"));
    }

    #[test]
    fn export_error_encode_display() {
        let e = ExportError::Encode {
            format: "jpeg",
            reason: "oops".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("jpeg"));
        assert!(s.contains("oops"));
    }

    // ── ImportError Display ──────────────────────────────────────────────────

    #[test]
    fn import_error_truncated_display() {
        let e = ImportError::Truncated {
            format: "png",
            needed: "header",
        };
        let s = format!("{e}");
        assert!(s.contains("png"));
        assert!(s.contains("truncated"));
    }

    #[test]
    fn import_error_decode_display() {
        let e = ImportError::Decode {
            format: "gif",
            reason: "bad data".into(),
        };
        let s = format!("{e}");
        assert!(s.contains("gif"));
        assert!(s.contains("bad data"));
    }
}
