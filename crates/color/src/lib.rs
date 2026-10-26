//! Color foundations for `FireCraft`: pixel formats, premultiplication,
//! sRGB/linear conversion, and the 27 blend modes.
//!
//! Layer L0: depends on nothing from this workspace.

pub mod blend;
pub mod convert;
pub mod pixel;

pub use blend::{BlendMode, blend};
pub use convert::{PixelFormat, linear_to_srgb, srgb_to_linear};
pub use pixel::Rgba8;

#[cfg(test)]
mod tests {
    use super::blend::normal;
    use super::pixel::Rgba8;

    #[test]
    fn normal_semantics() {
        let white = Rgba8::new(255, 255, 255, 255);
        let red = Rgba8::new(255, 0, 0, 255);
        let half_red = Rgba8::new(255, 0, 0, 128);
        let transparent = Rgba8::TRANSPARENT;

        // Opaque source replaces destination.
        assert_eq!(normal(red, white), red);
        // Transparent source leaves destination unchanged.
        assert_eq!(normal(transparent, white), white);
        // 50% red over white is a pink midtone on every channel.
        let out = normal(half_red, white);
        assert_eq!(out.r, 255);
        assert_eq!(out.g, 127);
        assert_eq!(out.b, 127);
        assert_eq!(out.a, 255);
    }
}
