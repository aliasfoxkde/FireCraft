//! Fireworks' 27 blend modes as pure functions.
//!
//! Formulas follow W3C CSS Compositing and Blending Level 1
//! (<https://www.w3.org/TR/compositing-1/>) for the standard set, plus the
//! PDF 12 / Photoshop extended modes Fireworks exposes (Linear Burn,
//! Vivid/Linear/Pin Light, Hard Mix, Subtract, Divide, Darker/Lighter
//! Color). CS6-measured ground truth replaces hand-evaluated fixtures as
//! the corpus lands (M2+).
//!
//! All math is allocation-free. Per-pixel hot paths get integer/SIMD
//! specializations in the M4 compositor; these functions are the
//! correctness reference.

use crate::pixel::Rgba8;

/// One of the 27 blend modes, in Fireworks UI grouping order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// Normal.
    Normal,
    /// Dissolve (dithered transparency; position-seeded in the M4
    /// compositor).
    Dissolve,
    // -- darken group
    /// Darken.
    Darken,
    /// Multiply.
    Multiply,
    /// Color Burn.
    ColorBurn,
    /// Linear Burn.
    LinearBurn,
    /// Darker Color.
    DarkerColor,
    // -- lighten group
    /// Lighten.
    Lighten,
    /// Screen.
    Screen,
    /// Color Dodge.
    ColorDodge,
    /// Linear Dodge (Add).
    LinearDodge,
    /// Lighter Color.
    LighterColor,
    // -- overlay group
    /// Overlay.
    Overlay,
    /// Soft Light.
    SoftLight,
    /// Hard Light.
    HardLight,
    /// Vivid Light.
    VividLight,
    /// Linear Light.
    LinearLight,
    /// Pin Light.
    PinLight,
    /// Hard Mix.
    HardMix,
    // -- difference group
    /// Difference.
    Difference,
    /// Exclusion.
    Exclusion,
    /// Subtract.
    Subtract,
    /// Divide.
    Divide,
    // -- component group (non-separable)
    /// Hue.
    Hue,
    /// Saturation.
    Saturation,
    /// Color.
    Color,
    /// Luminosity.
    Luminosity,
}

impl BlendMode {
    /// Every mode, UI order.
    pub const ALL: [BlendMode; 27] = [
        BlendMode::Normal,
        BlendMode::Dissolve,
        BlendMode::Darken,
        BlendMode::Multiply,
        BlendMode::ColorBurn,
        BlendMode::LinearBurn,
        BlendMode::DarkerColor,
        BlendMode::Lighten,
        BlendMode::Screen,
        BlendMode::ColorDodge,
        BlendMode::LinearDodge,
        BlendMode::LighterColor,
        BlendMode::Overlay,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::VividLight,
        BlendMode::LinearLight,
        BlendMode::PinLight,
        BlendMode::HardMix,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Subtract,
        BlendMode::Divide,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
    ];

    /// CSS-style identifier (`"color-burn"`, `"linear-dodge"`, ...).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BlendMode::Normal => "normal",
            BlendMode::Dissolve => "dissolve",
            BlendMode::Darken => "darken",
            BlendMode::Multiply => "multiply",
            BlendMode::ColorBurn => "color-burn",
            BlendMode::LinearBurn => "linear-burn",
            BlendMode::DarkerColor => "darker-color",
            BlendMode::Lighten => "lighten",
            BlendMode::Screen => "screen",
            BlendMode::ColorDodge => "color-dodge",
            BlendMode::LinearDodge => "linear-dodge",
            BlendMode::LighterColor => "lighter-color",
            BlendMode::Overlay => "overlay",
            BlendMode::SoftLight => "soft-light",
            BlendMode::HardLight => "hard-light",
            BlendMode::VividLight => "vivid-light",
            BlendMode::LinearLight => "linear-light",
            BlendMode::PinLight => "pin-light",
            BlendMode::HardMix => "hard-mix",
            BlendMode::Difference => "difference",
            BlendMode::Exclusion => "exclusion",
            BlendMode::Subtract => "subtract",
            BlendMode::Divide => "divide",
            BlendMode::Hue => "hue",
            BlendMode::Saturation => "saturation",
            BlendMode::Color => "color",
            BlendMode::Luminosity => "luminosity",
        }
    }

    /// Parses a CSS-style identifier, case-insensitively, with the common
    /// `"add"` alias for Linear Dodge.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let folded = s.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|m| m.as_str() == folded)
            .or_else(|| {
                if folded == "add" {
                    Some(BlendMode::LinearDodge)
                } else {
                    None
                }
            })
    }

    /// Whether the mode is separable (applies per channel). Hue, Saturation,
    /// Color, and Luminosity are not; Darker/Lighter Color and Dissolve are
    /// pixel-wise rather than channel-wise.
    #[must_use]
    pub fn is_separable(self) -> bool {
        !matches!(
            self,
            BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity
        )
    }
}

impl std::fmt::Display for BlendMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Blends `src` over `dst` with the given mode (straight-alpha in, out).
#[must_use]
pub fn blend(mode: BlendMode, src: Rgba8, dst: Rgba8) -> Rgba8 {
    match mode {
        BlendMode::Normal => normal(src, dst),
        BlendMode::Dissolve => dissolve(src, dst),
        BlendMode::DarkerColor => pick_color(src, dst, false),
        BlendMode::LighterColor => pick_color(src, dst, true),
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => {
            let b = nonseparable(mode, src, dst);
            composite(src, dst, b)
        }
        separable => {
            let b = [
                separable_channel(
                    separable,
                    f64::from(src.r) / 255.0,
                    f64::from(dst.r) / 255.0,
                ),
                separable_channel(
                    separable,
                    f64::from(src.g) / 255.0,
                    f64::from(dst.g) / 255.0,
                ),
                separable_channel(
                    separable,
                    f64::from(src.b) / 255.0,
                    f64::from(dst.b) / 255.0,
                ),
            ];
            composite(src, dst, b)
        }
    }
}

/// Source-over ("Normal") compositing in sRGB space, u32 exact.
#[must_use]
pub fn normal(src: Rgba8, dst: Rgba8) -> Rgba8 {
    if src.is_opaque() {
        return src;
    }
    if src.is_transparent() {
        return dst;
    }
    let sa = u32::from(src.a);
    let da = u32::from(dst.a);
    let dst_weight = 255 - sa;
    let out_a = sa + da * dst_weight / 255;
    if out_a == 0 {
        return Rgba8::TRANSPARENT;
    }
    let s = crate::pixel::premultiply(src);
    let d = crate::pixel::premultiply(dst);
    // Precondition for the `as u8` below: premultiplied channels satisfy
    // cs + cd*w/255 <= sa + da*w/255 == out_a <= 255, so the quotient is
    // <= 255 even before rounding.
    let chan = |cs: u16, cd: u16| -> u8 {
        let num = (u32::from(cs) + u32::from(cd) * dst_weight / 255) * 255 + out_a / 2;
        (num / out_a) as u8
    };
    Rgba8::new(
        chan(s[0], d[0]),
        chan(s[1], d[1]),
        chan(s[2], d[2]),
        out_a as u8,
    )
}

/// Dissolve: keep opaque source pixels with probability `src.a / 255`.
///
/// The roll is a deterministic hash of the pixel — position-seeded dither
/// replaces this in the M4 compositor, where x/y coordinates exist.
#[must_use]
pub fn dissolve(src: Rgba8, dst: Rgba8) -> Rgba8 {
    let mut roll = src.to_u32();
    roll ^= roll >> 16;
    roll = roll.wrapping_mul(0x7feb_352d);
    roll ^= roll >> 15;
    roll = roll.wrapping_mul(0x846c_a68b);
    roll ^= roll >> 16;
    if roll & 0xFF < u32::from(src.a) {
        Rgba8::new(src.r, src.g, src.b, 255)
    } else {
        dst
    }
}

/// Darker/Lighter Color: whole-pixel selection by RGB sum.
fn pick_color(src: Rgba8, dst: Rgba8, lighter: bool) -> Rgba8 {
    let s = u32::from(src.r) + u32::from(src.g) + u32::from(src.b);
    let d = u32::from(dst.r) + u32::from(dst.g) + u32::from(dst.b);
    let take_src = if lighter { s >= d } else { s <= d };
    if take_src { src } else { dst }
}

/// Per-channel separable blend, inputs and output in 0.0..=1.0.
fn separable_channel(mode: BlendMode, cs: f64, cb: f64) -> f64 {
    match mode {
        BlendMode::Darken => cs.min(cb),
        BlendMode::Multiply => cs * cb,
        BlendMode::ColorBurn => color_burn(cb, cs),
        BlendMode::LinearBurn => (cb + cs - 1.0).max(0.0),
        BlendMode::Lighten => cs.max(cb),
        BlendMode::Screen => cb + cs - cb * cs,
        BlendMode::ColorDodge => color_dodge(cb, cs),
        BlendMode::LinearDodge => (cb + cs).min(1.0),
        BlendMode::Overlay => hard_light(cb, cs), // HardLight with operands swapped
        BlendMode::SoftLight => soft_light(cb, cs),
        BlendMode::HardLight => hard_light(cs, cb),
        BlendMode::VividLight => {
            if cs <= 0.5 {
                color_burn(cb, (2.0 * cs).min(1.0))
            } else {
                color_dodge(cb, (2.0 * (cs - 0.5)).min(1.0))
            }
        }
        BlendMode::LinearLight => {
            if cs <= 0.5 {
                (cb + 2.0 * cs - 1.0).max(0.0)
            } else {
                (cb + 2.0 * (cs - 0.5)).min(1.0)
            }
        }
        BlendMode::PinLight => {
            if cs <= 0.5 {
                cb.min(2.0 * cs)
            } else {
                cb.max(2.0 * (cs - 0.5))
            }
        }
        BlendMode::HardMix => {
            if cb + cs < 1.0 {
                0.0
            } else {
                1.0
            }
        }
        BlendMode::Difference => (cb - cs).abs(),
        BlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
        BlendMode::Subtract => (cb - cs).max(0.0),
        BlendMode::Divide => {
            if cs == 0.0 {
                1.0
            } else {
                (cb / cs).min(1.0)
            }
        }
        // Pixel-wise and non-separable modes never reach here.
        _ => cs,
    }
}

/// W3C Color Burn: `1 - min(1, (1-Cb)/Cs)`, with the Cs == 0 edge.
fn color_burn(cb: f64, cs: f64) -> f64 {
    if cb >= 1.0 {
        1.0
    } else if cs <= 0.0 {
        0.0
    } else {
        (1.0 - (1.0 - cb) / cs).max(0.0)
    }
}

/// W3C Color Dodge: `Cb / (1 - Cs)`, with the Cs == 1 edge.
fn color_dodge(cb: f64, cs: f64) -> f64 {
    if cb <= 0.0 {
        0.0
    } else if cs >= 1.0 {
        1.0
    } else {
        (cb / (1.0 - cs)).min(1.0)
    }
}

/// W3C Hard Light: Cs <= 1/2 -> 2·Cs·Cb, else Screen(2·Cs - 1).
fn hard_light(cs: f64, cb: f64) -> f64 {
    if cs <= 0.5 {
        2.0 * cs * cb
    } else {
        let screened = cb + (2.0 * cs - 1.0) - cb * (2.0 * cs - 1.0);
        screened.min(1.0)
    }
}

/// W3C Soft Light with the D(Cb) piecewise polynomial.
fn soft_light(cb: f64, cs: f64) -> f64 {
    let d = if cb <= 0.25 {
        ((16.0 * cb - 12.0) * cb + 4.0) * cb
    } else {
        cb.sqrt()
    };
    let v = if cs <= 0.5 {
        cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
    } else {
        cb + (2.0 * cs - 1.0) * (d - cb)
    };
    v.clamp(0.0, 1.0)
}

/// Non-separable component modes, all in 0.0..=1.0 linearized RGB tuples
/// (sRGB space values, W3C coefficients).
fn nonseparable(mode: BlendMode, src: Rgba8, dst: Rgba8) -> [f64; 3] {
    let cs = rgb(src);
    let cb = rgb(dst);
    match mode {
        BlendMode::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
        BlendMode::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
        BlendMode::Color => set_lum(cs, lum(cb)),
        BlendMode::Luminosity => set_lum(cb, lum(cs)),
        _ => cs,
    }
}

fn rgb(c: Rgba8) -> [f64; 3] {
    [
        f64::from(c.r) / 255.0,
        f64::from(c.g) / 255.0,
        f64::from(c.b) / 255.0,
    ]
}

fn lum(c: [f64; 3]) -> f64 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn clip_color(c: [f64; 3]) -> [f64; 3] {
    let l = lum(c);
    let n = l.min(c[0]).min(c[1]).min(c[2]);
    let x = l.max(c[0]).max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 {
        for ch in &mut out {
            *ch = l + (*ch - l) * l / (l - n);
        }
    }
    if x > 1.0 {
        for ch in &mut out {
            *ch = l + (*ch - l) * (1.0 - l) / (x - l);
        }
    }
    out
}

fn set_lum(c: [f64; 3], l: f64) -> [f64; 3] {
    let d = l - lum(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d])
}

fn set_sat(c: [f64; 3], s: f64) -> [f64; 3] {
    // Order the channels, then rescale so max - min == s.
    let mut order = [0usize, 1, 2];
    order.sort_by(|&a, &b| c[a].total_cmp(&c[b]));
    let (mi, mdi, ma) = (order[0], order[1], order[2]);
    let mut out = c;
    if c[ma] > c[mi] {
        out[mdi] = (c[mdi] - c[mi]) * s / (c[ma] - c[mi]);
        out[ma] = s;
    } else {
        out[mdi] = 0.0;
        out[ma] = 0.0;
    }
    out[mi] = 0.0;
    out
}

fn sat(c: [f64; 3]) -> f64 {
    let (n, x) = c
        .into_iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(n, x), v| {
            (n.min(v), x.max(v))
        });
    x - n
}

/// Composites a blended color B over `dst` with straight-alpha source,
/// per W3C: Co = (1-as)·ab·Cb + as·ab·B + as·(1-ab)·Cs.
fn composite(src: Rgba8, dst: Rgba8, b: [f64; 3]) -> Rgba8 {
    let as_ = f64::from(src.a) / 255.0;
    let ab = f64::from(dst.a) / 255.0;
    let out_a = as_ + ab * (1.0 - as_);
    if out_a <= 0.0 {
        return Rgba8::TRANSPARENT;
    }
    let cs = rgb(src);
    let cb = rgb(dst);
    let mut out = [0u8; 3];
    for i in 0..3 {
        let co = (1.0 - as_) * ab * cb[i] + as_ * ab * b[i] + as_ * (1.0 - ab) * cs[i];
        out[i] = ((co / out_a).clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    Rgba8::new(out[0], out[1], out[2], (out_a * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::pixel::Rgba8 as C;

    /// The standard opaque fixture: src = (204,102,0) = (0.8, 0.4, 0.0),
    /// dst = (51,153,255) = (0.2, 0.6, 1.0). Values below are the W3C/PDF
    /// formulas hand-evaluated at 255 scale (±1 for float rounding);
    /// CS6-measured ground truth lands with the M2 corpus.
    const SRC: C = C::new(204, 102, 0, 255);
    const DST: C = C::new(51, 153, 255, 255);

    fn assert_mode(mode: BlendMode, want: [u8; 3]) {
        let got = blend(mode, SRC, DST);
        let got = [got.r, got.g, got.b];
        for i in 0..3 {
            assert!(
                (i16::from(got[i]) - i16::from(want[i])).abs() <= 1,
                "{mode} channel {i}: got {got:?}, want {want:?}"
            );
        }
        assert_eq!(got_alpha(mode), 255, "{mode} must stay opaque");
    }

    fn got_alpha(mode: BlendMode) -> u8 {
        blend(mode, SRC, DST).a
    }

    #[test]
    fn darken_group() {
        assert_mode(BlendMode::Darken, [51, 102, 0]);
        assert_mode(BlendMode::Multiply, [41, 61, 0]);
        assert_mode(BlendMode::ColorBurn, [0, 0, 255]);
        assert_mode(BlendMode::LinearBurn, [0, 0, 0]);
        // Darker Color picks the whole pixel: src sum 306 < dst sum 459.
        assert_mode(BlendMode::DarkerColor, [204, 102, 0]);
    }

    #[test]
    fn lighten_group() {
        assert_mode(BlendMode::Lighten, [204, 153, 255]);
        assert_mode(BlendMode::Screen, [214, 194, 255]);
        assert_mode(BlendMode::ColorDodge, [255, 255, 255]);
        assert_mode(BlendMode::LinearDodge, [255, 255, 255]);
        // Lighter Color picks the whole pixel.
        assert_mode(BlendMode::LighterColor, [51, 153, 255]);
    }

    #[test]
    fn overlay_group() {
        // Overlay pivots on the backdrop: blue backdrop = 1.0 -> Screen
        // branch, so the blue channel saturates.
        assert_mode(BlendMode::Overlay, [82, 133, 255]);
        assert_mode(BlendMode::SoftLight, [89, 141, 255]);
        assert_mode(BlendMode::HardLight, [173, 122, 0]);
        assert_mode(BlendMode::VividLight, [128, 128, 255]);
        assert_mode(BlendMode::LinearLight, [204, 102, 0]);
        assert_mode(BlendMode::PinLight, [153, 153, 0]);
        assert_mode(BlendMode::HardMix, [255, 255, 255]);
    }

    #[test]
    fn difference_group() {
        assert_mode(BlendMode::Difference, [153, 51, 255]);
        assert_mode(BlendMode::Exclusion, [173, 133, 255]);
        assert_mode(BlendMode::Subtract, [0, 51, 255]);
        assert_mode(BlendMode::Divide, [64, 255, 255]);
    }

    #[test]
    fn component_group() {
        // Hue keeps the source hue (0.8, 0.4, 0) with the backdrop's
        // saturation (0.8) and luminance (0.524).
        assert_mode(BlendMode::Hue, [216, 114, 12]);
        // Degenerate backdrop (flat white: saturation 0) zeroes the hue.
        let got = blend(BlendMode::Hue, C::new(255, 0, 0, 255), C::WHITE);
        assert_eq!(got, C::WHITE);
        // Saturation(gray128, red): SetSat zeroes the flat gray, lum
        // restored -> the gray itself.
        let got = blend(
            BlendMode::Saturation,
            C::new(255, 0, 0, 255),
            C::new(128, 128, 128, 255),
        );
        assert!((i16::from(got.r) - 128).abs() <= 1);
        assert!((i16::from(got.g) - 128).abs() <= 1);
        assert!((i16::from(got.b) - 128).abs() <= 1);
        // Color(red, gray128): hue of red, luminance of the gray.
        let got = blend(
            BlendMode::Color,
            C::new(255, 0, 0, 255),
            C::new(128, 128, 128, 255),
        );
        assert_eq!(got.r, 255);
        assert!((i16::from(got.g) - 74).abs() <= 1);
        assert!((i16::from(got.b) - 74).abs() <= 1);
        // Luminosity(white, blue): white carries blue's luminance (0.11).
        let got = blend(BlendMode::Luminosity, C::new(0, 0, 255, 255), C::WHITE);
        assert!((i16::from(got.r) - 28).abs() <= 1);
        assert!((i16::from(got.g) - 28).abs() <= 1);
        assert!((i16::from(got.b) - 28).abs() <= 1);
    }

    #[test]
    fn normal_and_dissolve() {
        let red = C::new(255, 0, 0, 255);
        let half_red = C::new(255, 0, 0, 128);
        assert_eq!(blend(BlendMode::Normal, red, C::WHITE), red);
        assert_eq!(blend(BlendMode::Normal, C::TRANSPARENT, C::WHITE), C::WHITE);
        // Dissolve is deterministic and only ever yields opaque src or dst.
        let a = blend(BlendMode::Dissolve, half_red, C::WHITE);
        let b = blend(BlendMode::Dissolve, half_red, C::WHITE);
        assert_eq!(a, b, "dissolve must be deterministic");
        assert!(a == C::new(255, 0, 0, 255) || a == C::WHITE);
    }

    #[test]
    fn blend_composites_alpha() {
        // Multiply of white src over black dst zeroes the blend term, so
        // the result is dst-weighted only: 0 out (Normal would give 127).
        let got = blend(BlendMode::Multiply, C::new(255, 255, 255, 128), C::BLACK);
        assert_eq!((got.r, got.g, got.b), (0, 0, 0));
        assert_eq!(got.a, 255);

        // Multiply red@50% over white: blend term is red, dst term is
        // white -> (255, 127, 127).
        let got = blend(BlendMode::Multiply, C::new(255, 0, 0, 128), C::WHITE);
        assert_eq!(got.r, 255);
        assert!((i16::from(got.g) - 127).abs() <= 1);
        assert_eq!(got.a, 255);

        // 50% over 50% gives 75% alpha; the f64 compositor rounds the
        // exact 191.75 up to 192 (the u32 Normal fast path truncates to
        // 191 — unified in the M4 compositor).
        let got = blend(
            BlendMode::Multiply,
            C::new(255, 0, 0, 128),
            C::new(0, 0, 255, 128),
        );
        assert_eq!(got.a, 192);
    }

    #[test]
    fn mode_names_round_trip() {
        for mode in BlendMode::ALL {
            assert_eq!(BlendMode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(BlendMode::parse("ADD"), Some(BlendMode::LinearDodge));
        assert_eq!(BlendMode::parse("bogus"), None);
        assert_eq!(BlendMode::ALL.len(), 27);
    }

    #[test]
    fn edge_channels_do_not_panic() {
        // The divide-by-zero edges: black/black, white/white, 0/0.
        for mode in BlendMode::ALL {
            let _ = blend(mode, C::BLACK, C::BLACK);
            let _ = blend(mode, C::WHITE, C::WHITE);
            let _ = blend(mode, C::TRANSPARENT, C::TRANSPARENT);
            let _ = blend(mode, SRC, C::BLACK);
            let _ = blend(mode, C::BLACK, DST);
        }
    }
}
