//! Pixel formats and sRGB <-> linear-light conversion.

/// Channel storage depth of a raster surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChannelDepth {
    /// 8 bits per channel.
    Eight,
    /// 16 bits per channel.
    Sixteen,
    /// 32-bit float per channel (linear-light working space).
    Float32,
}

/// How alpha relates to the color channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlphaLayout {
    /// Color channels are unweighted (the interchange format).
    Straight,
    /// Color channels are scaled by alpha (the compositing format).
    Premultiplied,
}

/// A raster pixel format: depth plus alpha layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PixelFormat {
    /// Bits/representation per channel.
    pub depth: ChannelDepth,
    /// Alpha weighting.
    pub layout: AlphaLayout,
}

impl PixelFormat {
    /// 8-bit straight RGBA — the document interchange format.
    pub const RGBA8: Self = Self {
        depth: ChannelDepth::Eight,
        layout: AlphaLayout::Straight,
    };
    /// 8-bit premultiplied RGBA — the tile/compositing format.
    pub const RGBA8_PREMUL: Self = Self {
        depth: ChannelDepth::Eight,
        layout: AlphaLayout::Premultiplied,
    };
    /// 16-bit straight RGBA — high-fidelity interchange.
    pub const RGBA16: Self = Self {
        depth: ChannelDepth::Sixteen,
        layout: AlphaLayout::Straight,
    };
    /// 32-bit float premultiplied RGBA (linear) — the GPU/vello format.
    pub const RGBA32F_PREMUL: Self = Self {
        depth: ChannelDepth::Float32,
        layout: AlphaLayout::Premultiplied,
    };

    /// Bytes per pixel for this format.
    #[must_use]
    pub const fn bytes_per_pixel(self) -> usize {
        match self.depth {
            ChannelDepth::Eight => 4,
            ChannelDepth::Sixteen => 8,
            ChannelDepth::Float32 => 16,
        }
    }
}

/// sRGB transfer function (IEC 61966-2-1) encode: linear -> sRGB.
/// Input and output are 0.0..=1.0; out-of-range input saturates.
#[must_use]
pub fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0 {
        return 0.0;
    }
    if c >= 1.0 {
        return 1.0;
    }
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB transfer function decode: sRGB -> linear. Out-of-range input
/// saturates (negative values are legal output only via the linear
/// segment; this function never produces NaN).
#[must_use]
pub fn srgb_to_linear(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Fast path for 8-bit sRGB samples: exact table-free decode.
#[must_use]
pub fn srgb8_to_linear(c: u8) -> f32 {
    srgb_to_linear(f32::from(c) / 255.0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn endpoints_are_exact() {
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert_eq!(srgb_to_linear(1.0), 1.0);
        assert_eq!(linear_to_srgb(0.0), 0.0);
        assert_eq!(linear_to_srgb(1.0), 1.0);
    }

    #[test]
    fn midtone_matches_reference() {
        // sRGB 128/255 decodes to ~0.216 linear-light (standard table
        // value, e.g. Bruce Lindbloom's sRGB tables).
        let lin = srgb8_to_linear(128);
        assert!((lin - 0.2158).abs() < 1e-3, "got {lin}");
    }

    #[test]
    fn round_trip_within_eight_bit_tolerance() {
        for v in [0u8, 1, 13, 64, 128, 200, 254, 255] {
            let back = linear_to_srgb(srgb8_to_linear(v));
            let as_u8 = (back * 255.0).round() as i32;
            assert!(
                (as_u8 - i32::from(v)).abs() <= 1,
                "sRGB round trip drifted for {v}: got {as_u8}"
            );
        }
    }

    #[test]
    fn out_of_range_saturates_not_nan() {
        assert_eq!(srgb_to_linear(2.0), 1.0);
        assert_eq!(srgb_to_linear(-1.0), 0.0);
        assert_eq!(linear_to_srgb(2.0), 1.0);
        assert!(!srgb_to_linear(f32::INFINITY).is_nan());
        assert_eq!(srgb_to_linear(f32::INFINITY), 1.0);
    }

    #[test]
    fn format_sizes() {
        assert_eq!(PixelFormat::RGBA8.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::RGBA16.bytes_per_pixel(), 8);
        assert_eq!(PixelFormat::RGBA32F_PREMUL.bytes_per_pixel(), 16);
    }
}
