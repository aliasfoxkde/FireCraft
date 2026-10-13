//! Bevel / emboss composites (M14 task 3b): the gradient-lighting
//! family — inner/outer bevel, raised/inset emboss, and the MX
//! "Glow" variant (BevelType 4), all driven by one height field.
//!
//! The model: the silhouette alpha is a plateau (255 inside, 0
//! outside); `box_blur` by the bevel width rounds the rim into a
//! slope — that blurred alpha **is** the height field, and the
//! shading is its directional derivative against the light. A pixel
//! whose height rises toward the light faces it: hilite; the
//! opposite slope takes the shadow color. Every piece is integer
//! math on the proven blur (decision 0004), trig from the shared
//! fixed-point table, house rounding in the divisions.
//!
//! Angle convention matches the shadow family: `LightAngle` points
//! where the light is; in y-down pixel space the light vector is
//! `(cos θ, −sin θ)`, so θ=90° (light above) brightens the TOP rim
//! of a raised shape.
//!
//! Pinned substitutions (unobservable without a render oracle):
//! the slope profile is flat for every `SlopeType` (smooth/zigzag/
//! ring/ruffle remaps unobserved — recorded, revisit on oracle
//! evidence); `SlopeMultiplier` is pinned inert (observed range
//! 1..1); `AngleSoftness` is inert here (an extra softening blur on
//! the shading — the height blur already bounds the slope); the
//! packed bevel's opacity twin fields and its "always 30" field are
//! pinned inert; highlight/shadow blend tokens are consumed upstream
//! but applied nowhere (same recording as the shadow family);
//! `LightDistance` reads as a depth percent (÷100, observed 39..200)
//! scaling the shading response; the glow's `GlowStartDistance`
//! erodes the height field linearly over its 0..35 range.

use crate::convolve::box_blur;
use crate::geometric::{angle_units, cos_fp, sin_fp};
use crate::shadow::{over, scale_alpha_against, silhouette};
use crate::{Surface, clamp255, div_half};

use color::Rgba8;

/// Which rim the shading paints, per CS6's `BevelType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BevelStyle {
    /// `BevelType` 0 — shading inside the silhouette edge.
    InnerBevel,
    /// `BevelType` 1 — shading outside the silhouette edge.
    OuterBevel,
    /// `BevelType` 2 — the face takes `face` shaded by the light
    /// (classic raised emboss).
    RaisedEmboss,
    /// `BevelType` 3 — raised emboss lit from the opposite side.
    InsetEmboss,
    /// `BevelType` 4 — the MX Glow: outer shading with a start
    /// distance eroding the field.
    Glow,
}

/// The two rim colors and the shared geometry: `width` is the bevel
/// width in pixels (the height-field blur radius), `contrast_percent`
/// scales the shading response (0..=100), `depth_percent` the
/// `LightDistance` depth (39..=200 observed, ÷100 on use),
/// `start` the glow's start distance (0..=35, glow style only).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn bevel(
    surface: &Surface,
    style: BevelStyle,
    light_angle_degrees: f64,
    width: u32,
    contrast_percent: u32,
    depth_percent: u32,
    start: u32,
    hilite: Rgba8,
    shadow: Rgba8,
    face: Option<Rgba8>,
) -> Surface {
    let height = box_blur(&silhouette(surface, 0), width);
    let erode = if style == BevelStyle::Glow {
        div_half(i64::from(start.clamp(0, 35)) * 255, 35)
    } else {
        0
    };
    let contrast = i64::from(contrast_percent.clamp(0, 100));
    let depth = i64::from(depth_percent.clamp(0, 1000));
    let (dx, dy) = light_step(light_angle_degrees);
    // Inset emboss is the raised emboss lit from the opposite side.
    let (dx, dy) = if style == BevelStyle::InsetEmboss {
        (-dx, -dy)
    } else {
        (dx, dy)
    };
    let (w, h) = (surface.width(), surface.height());
    let mut shaded = surface.clone();
    shaded.pixels.fill(0);
    for y in 0..h {
        for x in 0..w {
            let at = shaded.pixel_offset(x, y);
            let slope = directional_slope(&height, x, y, dx, dy, erode);
            let t = (128 + div_half(slope * contrast * depth, 10_000)).clamp(0, 255);
            let [r, g, b, a] = rim_color(t, hilite, shadow, face, style);
            shaded.pixels[at] = r;
            shaded.pixels[at + 1] = g;
            shaded.pixels[at + 2] = b;
            shaded.pixels[at + 3] = a;
        }
    }
    match style {
        // Inner bevel and the embosses shade the object's OWN pixels
        // (a tint, not a layer over it); outer bevel and glow paint
        // the empty outside and draw under the source.
        BevelStyle::InnerBevel | BevelStyle::RaisedEmboss | BevelStyle::InsetEmboss => {
            tint(&shaded, surface)
        }
        BevelStyle::OuterBevel | BevelStyle::Glow => {
            let outside = scale_alpha_against(&shaded, surface, |a| 255 - i64::from(a));
            over(surface, &outside)
        }
    }
}

/// The inner shading: each pixel's color pulled toward the rim's
/// straight color by the rim's strength, clipped to the object's own
/// alpha (the shading can never outlive its surface). Alpha passes
/// through — a bevel shades, it does not change the shape's opacity.
/// The rim travels premultiplied-at-strength, so the straight-color
/// lerp `src·(1−s) + straight·s` simplifies to
/// `src + premult − src·s/255` — no division by the strength.
fn tint(shaded: &Surface, source: &Surface) -> Surface {
    let mut out = source.clone();
    for at in (0..out.pixels.len()).step_by(4) {
        let s = i64::from(shaded.pixels[at + 3]).min(i64::from(source.pixels[at + 3]));
        for ch in 0..3 {
            let src = i64::from(source.pixels[at + ch]);
            let cover = i64::from(shaded.pixels[at + ch]);
            out.pixels[at + ch] = clamp255(src + cover - div_half(src * s, 255));
        }
    }
    out
}

/// The light's unit step in whole pixels, y-down: `(cos θ, −sin θ)`
/// from the fixed-point table, each axis rounded to the nearest pixel
/// — θ=90° (light above) steps up, θ=45° steps diagonally.
fn light_step(angle_degrees: f64) -> (i64, i64) {
    let angle = angle_units(angle_degrees);
    (
        div_half(cos_fp(angle), 1000),
        -div_half(sin_fp(angle), 1000),
    )
}

/// The height change per pixel step *away* from the light (slope > 0
/// = the surface normal turns toward the light = it faces it): the
/// normal is `−∇h`, so the shading is the height drop along the
/// light vector. The glow's erosion shifts the whole field down
/// first, so its shading ring sits `start` units out.
fn directional_slope(height: &Surface, x: u32, y: u32, dx: i64, dy: i64, erode: i64) -> i64 {
    let sample = |px: i64, py: i64| -> i64 {
        let (w, h) = (i64::from(height.width()), i64::from(height.height()));
        let cx = px.clamp(0, w - 1) as u32;
        let cy = py.clamp(0, h - 1) as u32;
        let a = i64::from(height.pixels[height.pixel_offset(cx, cy) + 3]);
        (a - erode).clamp(0, 255)
    };
    let (iw, ih) = (i64::from(x), i64::from(y));
    let toward = sample(iw + dx, ih + dy);
    let away = sample(iw - dx, ih - dy);
    away - toward
}

/// Rim color at shading `t` (0 = full shadow slope, 128 = flat, 255 =
/// full hilite slope): a premultiplied lerp of the two rim colors.
/// The emboss styles replace the face instead — `face` shaded around
/// its own value (t=128 neutral), alpha kept at full inside the clip.
fn rim_color(
    t: i64,
    hilite: Rgba8,
    shadow: Rgba8,
    face: Option<Rgba8>,
    style: BevelStyle,
) -> [u8; 4] {
    // Emboss faces REPLACE the interior: the full-strength
    // premultiplied face, shaded around its own value (t=128
    // neutral). The alpha carries the replacement strength and the
    // tint clips it against the object.
    if let Some(face) =
        face.filter(|_| matches!(style, BevelStyle::RaisedEmboss | BevelStyle::InsetEmboss))
    {
        let scale = |c: i64| clamp255(div_half(c * t, 128));
        let a = i64::from(face.a);
        return [
            clamp255(div_half(i64::from(scale(i64::from(face.r))) * a, 255)),
            clamp255(div_half(i64::from(scale(i64::from(face.g))) * a, 255)),
            clamp255(div_half(i64::from(scale(i64::from(face.b))) * a, 255)),
            face.a,
        ];
    }
    // Bevel rims paint the hilite (t > 128) or shadow (t < 128) color
    // with strength ∝ |slope| — flat areas (t = 128) paint nothing,
    // so the source shows through untouched. Premultiplied output:
    // rgb rides the alpha.
    let strength = (2 * (t - 128).abs()).min(255);
    let rim = if t >= 128 { hilite } else { shadow };
    [
        clamp255(div_half(i64::from(rim.r) * strength, 255)),
        clamp255(div_half(i64::from(rim.g) * strength, 255)),
        clamp255(div_half(i64::from(rim.b) * strength, 255)),
        clamp255(div_half(i64::from(rim.a) * strength, 255)),
    ]
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// A 9×9 opaque white square at (2..=6, 2..=6) on transparent.
    fn square() -> Surface {
        let mut s = Surface::new(9, 9).unwrap();
        for y in 2..=6u32 {
            for x in 2..=6u32 {
                s.set_straight(x, y, Rgba8::new(255, 255, 255, 255));
            }
        }
        s
    }

    fn px(s: &Surface, x: u32, y: u32) -> [u8; 4] {
        s.get(x, y).unwrap()
    }

    const WHITE: Rgba8 = Rgba8::new(255, 255, 255, 255);
    const BLACK: Rgba8 = Rgba8::new(0, 0, 0, 255);
    const RED: Rgba8 = Rgba8::new(255, 0, 0, 255);

    #[test]
    fn light_from_above_brightens_the_top_rim() {
        // Inner bevel, light 90° (above): the top inside rim pulls
        // toward the hilite, the bottom rim toward the shadow color.
        // A red hilite over the white source shows the pull in the
        // green channel (white-on-white would composite invisible).
        let out = bevel(
            &square(),
            BevelStyle::InnerBevel,
            90.0,
            2,
            100,
            100,
            0,
            RED,
            BLACK,
            None,
        );
        let top = px(&out, 4, 2);
        let bottom = px(&out, 4, 6);
        assert!(top[0] == 255 && top[1] < 56, "top rim reddens: {top:?}");
        assert!(bottom[0] < 56, "bottom rim darkens: {bottom:?}");
        // The interior is far from any slope: neutral.
        assert_eq!(
            px(&out, 4, 4),
            [255, 255, 255, 255],
            "flat interior keeps the source"
        );
    }

    #[test]
    fn inset_emboss_is_the_raised_emboss_mirrored() {
        let face = Rgba8::new(128, 128, 128, 255);
        let raised = bevel(
            &square(),
            BevelStyle::RaisedEmboss,
            90.0,
            2,
            100,
            100,
            0,
            WHITE,
            BLACK,
            Some(face),
        );
        let inset = bevel(
            &square(),
            BevelStyle::InsetEmboss,
            90.0,
            2,
            100,
            100,
            0,
            WHITE,
            BLACK,
            Some(face),
        );
        // θ+180 on one equals the other — the rim shading flips sides.
        let raised_flipped = bevel(
            &square(),
            BevelStyle::RaisedEmboss,
            270.0,
            2,
            100,
            100,
            0,
            WHITE,
            BLACK,
            Some(face),
        );
        assert_eq!(
            inset, raised_flipped,
            "inset = raised with the light reversed"
        );
        // The raised face shades around the face gray: top rim
        // brighter than the face, bottom darker.
        let top = px(&raised, 4, 2);
        let bottom = px(&raised, 4, 6);
        assert!(top[0] > 128, "raised top rim brightens: {top:?}");
        assert!(bottom[0] < 128, "raised bottom rim darkens: {bottom:?}");
        assert_eq!(
            px(&raised, 4, 4),
            [128, 128, 128, 255],
            "flat interior keeps the face"
        );
    }

    #[test]
    fn outer_bevel_paints_only_outside_and_under() {
        let out = bevel(
            &square(),
            BevelStyle::OuterBevel,
            90.0,
            2,
            100,
            100,
            0,
            WHITE,
            BLACK,
            None,
        );
        // Inside stays the untouched source.
        assert_eq!(px(&out, 4, 4), [255, 255, 255, 255]);
        assert_eq!(px(&out, 4, 2), [255, 255, 255, 255]);
        // Outside: the ring just above the shape is lit, just below is
        // shadowed.
        assert!(px(&out, 4, 1)[0] > 0, "outside above: hilite");
        assert!(px(&out, 4, 7)[0] < 255, "outside below: shadow");
        // The corners sit at the edge of the blur's reach: faint, and
        // much weaker than the direct ring.
        assert!(
            px(&out, 0, 0)[3] < px(&out, 4, 1)[3],
            "corner shading fades"
        );
    }

    #[test]
    fn glow_erodes_the_field_by_the_start_distance() {
        let near = bevel(
            &square(),
            BevelStyle::Glow,
            90.0,
            2,
            100,
            100,
            0,
            WHITE,
            BLACK,
            None,
        );
        let far = bevel(
            &square(),
            BevelStyle::Glow,
            90.0,
            2,
            100,
            100,
            20,
            WHITE,
            BLACK,
            None,
        );
        // A start distance pushes the shading ring away from the
        // object: the near ring fades.
        let near_ring = px(&near, 4, 1);
        let far_ring = px(&far, 4, 1);
        assert!(
            far_ring[3] < near_ring[3],
            "start distance erodes the near ring: {near_ring:?} vs {far_ring:?}"
        );
        // And the object itself stays untouched on top.
        assert_eq!(px(&far, 4, 4), [255, 255, 255, 255]);
    }

    #[test]
    fn contrast_and_depth_scale_the_response() {
        // Stronger contrast pulls the top rim further toward the red
        // hilite: the source's green channel drops harder.
        let mild = bevel(
            &square(),
            BevelStyle::InnerBevel,
            90.0,
            3,
            30,
            100,
            0,
            RED,
            BLACK,
            None,
        );
        let harsh = bevel(
            &square(),
            BevelStyle::InnerBevel,
            90.0,
            3,
            100,
            100,
            0,
            RED,
            BLACK,
            None,
        );
        let mild_top = i64::from(px(&mild, 4, 2)[1]);
        let harsh_top = i64::from(px(&harsh, 4, 2)[1]);
        assert!(
            mild_top > harsh_top,
            "contrast deepens the hilite: {mild_top} vs {harsh_top}"
        );
        // Zero contrast flattens every slope to t=128: strength 0 —
        // nothing paints, inside or out.
        let flat = bevel(
            &square(),
            BevelStyle::InnerBevel,
            90.0,
            3,
            0,
            100,
            0,
            RED,
            BLACK,
            None,
        );
        assert_eq!(flat, square(), "no contrast → no shading anywhere");
    }

    #[test]
    fn empty_input_is_a_fixed_point_and_results_are_deterministic() {
        let empty = Surface::new(9, 9).unwrap();
        for style in [
            BevelStyle::InnerBevel,
            BevelStyle::OuterBevel,
            BevelStyle::RaisedEmboss,
            BevelStyle::InsetEmboss,
            BevelStyle::Glow,
        ] {
            assert_eq!(
                bevel(&empty, style, 45.0, 2, 100, 100, 0, WHITE, BLACK, None),
                empty,
                "{style:?} on empty"
            );
        }
        let s = square();
        assert_eq!(
            bevel(
                &s,
                BevelStyle::InnerBevel,
                45.0,
                2,
                80,
                120,
                0,
                WHITE,
                BLACK,
                None
            ),
            bevel(
                &s,
                BevelStyle::InnerBevel,
                45.0,
                2,
                80,
                120,
                0,
                WHITE,
                BLACK,
                None
            )
        );
    }
}
