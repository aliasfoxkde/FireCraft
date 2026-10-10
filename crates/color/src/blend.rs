//! Alpha compositing. The full 27-mode Fireworks blend table lands in M1;
//! source-over ("Normal") comes first because every other mode composes
//! with it.

use crate::pixel::{Rgba8, premultiply};

/// Standard source-over compositing ("Normal" blend) in sRGB space.
///
/// Straight-alpha result; intermediate math runs in `u32` with the
/// destination weighted by `1 - src_alpha` and half-LSB rounding on the
/// final unpremultiply-style division.
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
    let s = premultiply(src);
    let d = premultiply(dst);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixel::Rgba8 as C;

    #[test]
    fn output_alpha_composes() {
        // 50% over 50% -> 75%.
        let out = normal(C::new(255, 0, 0, 128), C::new(0, 0, 255, 128));
        assert_eq!(out.a, 191);
    }

    #[test]
    fn degenerate_alphas() {
        let red = C::new(255, 0, 0, 255);
        let blue = C::new(0, 0, 255, 255);
        let clear = C::TRANSPARENT;
        assert_eq!(normal(clear, clear), clear);
        assert_eq!(normal(red, clear), red);
        // Both fully transparent: result transparent.
        assert_eq!(normal(clear, C::new(9, 9, 9, 0)), C::new(9, 9, 9, 0));
        assert_eq!(normal(blue, red), blue);
    }

    #[test]
    fn commutes_with_premultiply_model() {
        // normal(a, b) over normal(b, c) background equals a single blend
        // only for the associative case; here we just verify idempotence of
        // an opaque source repeated.
        let opaque = C::new(10, 20, 30, 255);
        let mid = normal(opaque, C::WHITE);
        assert_eq!(normal(opaque, mid), mid);
    }
}
