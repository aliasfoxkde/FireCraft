//! Pixel formats and premultiplication.

/// An 8-bit-per-channel sRGB color with straight (non-premultiplied) alpha.
///
/// This is the canonical interchange format between `FireCraft` crates; tiled
/// raster storage premultiplies internally for compositing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    /// Fully transparent black.
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    /// Opaque black.
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    /// Opaque white.
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    /// Constructs a color from channel values.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Packs as `0xRRGGBBAA` (the layout Fireworks documents use for colors
    /// in string contexts).
    #[must_use]
    pub const fn to_u32(self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }

    /// Unpacks a `0xRRGGBBAA` value.
    #[must_use]
    pub const fn from_u32(v: u32) -> Self {
        Self {
            r: (v >> 24) as u8,
            g: (v >> 16) as u8,
            b: (v >> 8) as u8,
            a: v as u8,
        }
    }

    /// Formats as the `#rrggbb(aa)` string form Fireworks uses for filter
    /// parameters (alpha part only when not fully opaque).
    #[must_use]
    pub fn to_css_string(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!(
                "#{:02x}{:02x}{:02x}({:02x})",
                self.r, self.g, self.b, self.a
            )
        }
    }

    /// Whether every pixel of this color is opaque.
    #[must_use]
    pub const fn is_opaque(self) -> bool {
        self.a == 255
    }

    /// Whether every pixel of this color is fully transparent.
    #[must_use]
    pub const fn is_transparent(self) -> bool {
        self.a == 0
    }
}

/// Premultiplies a straight-alpha color into 16-bit channels (still on the
/// 0..=255 scale; the extra headroom avoids overflow in blend math).
#[must_use]
pub fn premultiply(c: Rgba8) -> [u16; 4] {
    let a = u16::from(c.a);
    [
        u16::from(c.r) * a / 255,
        u16::from(c.g) * a / 255,
        u16::from(c.b) * a / 255,
        a,
    ]
}

/// Reverses [`premultiply`], with banker's-free rounding (add half the
/// divisor). Fully transparent premultiplied input maps to transparent
/// black, the canonical unambiguous form.
#[must_use]
pub fn unpremultiply(p: [u16; 4]) -> Rgba8 {
    let a = p[3];
    if a == 0 {
        return Rgba8::TRANSPARENT;
    }
    let chan = |v: u16| -> u8 {
        let v = u32::from(v) * 255 + u32::from(a) / 2;
        (v / u32::from(a)) as u8
    };
    Rgba8::new(chan(p[0]), chan(p[1]), chan(p[2]), a as u8)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn u32_pack_round_trip() {
        let c = Rgba8::new(0x12, 0x34, 0x56, 0x78);
        assert_eq!(c.to_u32(), 0x1234_5678);
        assert_eq!(Rgba8::from_u32(0x1234_5678), c);
    }

    #[test]
    fn premultiply_round_trip_within_tolerance() {
        for v in [0u8, 1, 7, 64, 128, 200, 254, 255] {
            let c = Rgba8::new(v, v.reverse_bits(), 255 - v, 137);
            let back = unpremultiply(premultiply(c));
            assert!((i16::from(back.r) - i16::from(c.r)).abs() <= 1);
            assert!((i16::from(back.g) - i16::from(c.g)).abs() <= 1);
            assert!((i16::from(back.b) - i16::from(c.b)).abs() <= 1);
            assert_eq!(back.a, c.a);
        }
    }

    #[test]
    fn transparent_premultiplied_is_unambiguous() {
        assert_eq!(unpremultiply([10, 20, 30, 0]), Rgba8::TRANSPARENT);
        assert_eq!(premultiply(Rgba8::TRANSPARENT), [0, 0, 0, 0]);
    }

    #[test]
    fn css_string_forms() {
        assert_eq!(Rgba8::new(255, 0, 0, 255).to_css_string(), "#ff0000");
        assert_eq!(Rgba8::new(0, 0, 255, 128).to_css_string(), "#0000ff(80)");
    }
}
