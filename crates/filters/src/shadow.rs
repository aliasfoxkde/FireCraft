//! Shadow / glow composites (M14 task 3): drop shadow, inner shadow,
//! solid shadow, outer/inner glow — CS6's alpha-silhouette family,
//! built from proven pieces (silhouette, box blur, integer offset,
//! source-over) rather than new pixel math.
//!
//! All kernels read the source's **alpha** as the object silhouette and
//! never touch its color: a shadow is the silhouette blurred, offset,
//! colorized, and composited under (or, for inner effects, clipped
//! inside) the source. Premultiplied throughout (decision 0004), so
//! `out = src + shadow·(1 − src_a)` is exact integer math with house
//! rounding in the one division, and alpha scaling drags the color
//! channels along with it.
//!
//! Pinned substitutions (unobservable without a render oracle):
//! the silhouette blur is the proven box blur (CS6's shadow kernel is
//! unobserved); the angle convention is the standard desktop one —
//! `ShadowAngle` points where the **light** is, so the shadow offsets
//! by `(−d·cos θ, +d·sin θ)` in y-down pixel space (θ = 90°,
//! light-from-above, casts straight down); `EdgeThreshold`
//! (documented: "alpha threshold for detected edges" — Glow uses 1,
//! the rest 0) cuts silhouette alpha below `threshold/255`; packed
//! blend tokens (multiply, screen) are consumed upstream but applied
//! nowhere here — mid-stack the shadow can only composite against its
//! own source, and the page blend happens through the object's own
//! blend mode above this layer.

use crate::convolve::box_blur;
use crate::geometric::{angle_units, cos_fp, sin_fp};
use crate::{Surface, clamp255, div_half};

use color::Rgba8;

/// Drop shadow: the silhouette blurred by `blur`, offset `distance`
/// pixels opposite the light at `angle_degrees`, colorized, drawn
/// under the source. `knockout` thins the shadow wherever the source
/// itself is (semi-)opaque, so a translucent object does not tint its
/// own shadow.
#[must_use]
pub fn drop_shadow(
    surface: &Surface,
    color: Rgba8,
    angle_degrees: f64,
    distance: u32,
    blur: u32,
    knockout: bool,
) -> Surface {
    let (dx, dy) = shadow_offset(angle_degrees, distance);
    let mut shadow = colorize(&box_blur(&silhouette(surface, 0), blur), color);
    shadow = translate(&shadow, dx, dy);
    if knockout {
        shadow = scale_alpha_against(&shadow, surface, |src_a| 255 - i64::from(src_a));
    }
    over(surface, &shadow)
}

/// Inner shadow: the *inverted* silhouette blurred, offset by the same
/// light convention, clipped to the object (scaled by the source
/// alpha), drawn over the source — the interior edge opposite the
/// light darkens.
#[must_use]
pub fn inner_shadow(
    surface: &Surface,
    color: Rgba8,
    angle_degrees: f64,
    distance: u32,
    blur: u32,
) -> Surface {
    let (dx, dy) = shadow_offset(angle_degrees, distance);
    let inverted = silhouette_inverted(surface, 0);
    let shadow = colorize(&box_blur(&inverted, blur), color);
    let shadow = translate(&shadow, dx, dy);
    let clipped = scale_alpha_against(&shadow, surface, i64::from);
    over(&clipped, surface)
}

/// Solid shadow: the silhouette colorized flat — the object *is* the
/// shadow (Fireworks' Solid Shadow). `show_object` keeps the source on
/// top; without it only the flat silhouette remains.
#[must_use]
pub fn solid_shadow(surface: &Surface, color: Rgba8, show_object: bool) -> Surface {
    let solid = colorize(&silhouette(surface, 0), color);
    if show_object {
        over(surface, &solid)
    } else {
        solid
    }
}

/// Outer glow: a drop shadow with no offset and a hardenable
/// silhouette (`EdgeThreshold`; the dictionary records Glow at 1).
#[must_use]
pub fn outer_glow(surface: &Surface, color: Rgba8, blur: u32, edge_threshold: u8) -> Surface {
    let shadow = colorize(&box_blur(&silhouette(surface, edge_threshold), blur), color);
    over(surface, &shadow)
}

/// Inner glow: an inner shadow with no offset and the glow's
/// hardenable silhouette.
#[must_use]
pub fn inner_glow(surface: &Surface, color: Rgba8, blur: u32, edge_threshold: u8) -> Surface {
    let inverted = silhouette_inverted(surface, edge_threshold);
    let shadow = colorize(&box_blur(&inverted, blur), color);
    let clipped = scale_alpha_against(&shadow, surface, i64::from);
    over(&clipped, surface)
}

/// Offset in y-down pixel space for a light at `angle_degrees`:
/// `(−d·cos θ, +d·sin θ)` — θ=90° (light above) casts straight down,
/// θ=0° (light from the right) casts left.
fn shadow_offset(angle_degrees: f64, distance: u32) -> (i64, i64) {
    let angle = angle_units(angle_degrees);
    let d = i64::from(distance);
    (
        -div_half(d * cos_fp(angle), 1000),
        div_half(d * sin_fp(angle), 1000),
    )
}

/// The source's alpha as a black premultiplied surface; alpha below
/// the `edge_threshold` (0..255) is cut. Threshold 0 keeps the soft
/// alpha untouched.
fn silhouette(surface: &Surface, edge_threshold: u8) -> Surface {
    let threshold = i64::from(edge_threshold);
    let mut out = surface.clone();
    for at in (0..out.pixels.len()).step_by(4) {
        out.pixels[at] = 0;
        out.pixels[at + 1] = 0;
        out.pixels[at + 2] = 0;
        if i64::from(out.pixels[at + 3]) < threshold {
            out.pixels[at + 3] = 0;
        }
    }
    out
}

/// `255 − alpha` — the *outside* of the object as a black surface;
/// pixels below the edge threshold count as fully outside.
fn silhouette_inverted(surface: &Surface, edge_threshold: u8) -> Surface {
    let threshold = i64::from(edge_threshold);
    let mut out = surface.clone();
    for at in (0..out.pixels.len()).step_by(4) {
        out.pixels[at] = 0;
        out.pixels[at + 1] = 0;
        out.pixels[at + 2] = 0;
        let a = i64::from(out.pixels[at + 3]);
        out.pixels[at + 3] = if a < threshold {
            255
        } else {
            clamp255(255 - a)
        };
    }
    out
}

/// Colors a silhouette: premultiplied `color` in the RGB slots, the
/// surface's own alpha kept. The shadow color's own alpha is part of
/// the premultiplied cover.
fn colorize(silhouette: &Surface, color: Rgba8) -> Surface {
    let mut out = silhouette.clone();
    let (r, g, b, a) = (
        i64::from(color.r),
        i64::from(color.g),
        i64::from(color.b),
        i64::from(color.a),
    );
    for at in (0..out.pixels.len()).step_by(4) {
        let cover = div_half(i64::from(out.pixels[at + 3]) * a, 255);
        out.pixels[at] = clamp255(div_half(r * cover, 255));
        out.pixels[at + 1] = clamp255(div_half(g * cover, 255));
        out.pixels[at + 2] = clamp255(div_half(b * cover, 255));
        out.pixels[at + 3] = clamp255(cover);
    }
    out
}

/// Scales every channel of `subject` by `factor(src_a)/255` — the
/// premultiplied-safe clip (alpha scaling drags color along).
fn scale_alpha_against(subject: &Surface, source: &Surface, factor: impl Fn(u8) -> i64) -> Surface {
    let mut out = subject.clone();
    for at in (0..out.pixels.len()).step_by(4) {
        let keep = factor(source.pixels[at + 3]);
        for ch in 0..4 {
            out.pixels[at + ch] = clamp255(div_half(i64::from(out.pixels[at + ch]) * keep, 255));
        }
    }
    out
}

/// Integer translate; vacated pixels are transparent (a shadow may
/// leave the surface entirely on one side).
fn translate(surface: &Surface, dx: i64, dy: i64) -> Surface {
    let (w, h) = (surface.width(), surface.height());
    let (iw, ih) = (i64::from(w), i64::from(h));
    let mut out = surface.clone();
    out.pixels.fill(0);
    for y in 0..ih {
        let sy = y - dy;
        if sy < 0 || sy >= ih {
            continue;
        }
        for x in 0..iw {
            let sx = x - dx;
            if sx < 0 || sx >= iw {
                continue;
            }
            let src = surface.pixel_offset(sx as u32, sy as u32);
            let dst = out.pixel_offset(x as u32, y as u32);
            out.pixels[dst..dst + 4].copy_from_slice(&surface.pixels[src..src + 4]);
        }
    }
    out
}

/// Premultiplied source-over: `out = top + bottom·(1 − top_a)`.
fn over(top: &Surface, bottom: &Surface) -> Surface {
    let mut out = top.clone();
    for at in (0..out.pixels.len()).step_by(4) {
        let keep = 255 - i64::from(top.pixels[at + 3]);
        for ch in 0..4 {
            let under = i64::from(bottom.pixels[at + ch]);
            out.pixels[at + ch] =
                clamp255(i64::from(top.pixels[at + ch]) + div_half(under * keep, 255));
        }
    }
    out
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

    const RED: Rgba8 = Rgba8::new(255, 0, 0, 255);

    #[test]
    fn drop_shadow_offset_follows_the_light_convention() {
        // Light from above (90°), distance 2 → shadow shifts down 2.
        let out = drop_shadow(&square(), RED, 90.0, 2, 0, false);
        assert_eq!(px(&out, 4, 8), [255, 0, 0, 255], "shadow fills below");
        assert_eq!(px(&out, 0, 0)[3], 0, "no shadow above");
        assert_eq!(px(&out, 4, 4), [255, 255, 255, 255], "source stays on top");
        // Light from the right (0°) casts left.
        let left = drop_shadow(&square(), RED, 0.0, 2, 0, false);
        assert_eq!(px(&left, 0, 4), [255, 0, 0, 255]);
        assert_eq!(px(&left, 8, 4)[3], 0);
        // Light from the left (180°) casts right.
        let right = drop_shadow(&square(), RED, 180.0, 2, 0, false);
        assert_eq!(px(&right, 8, 4), [255, 0, 0, 255]);
        assert_eq!(px(&right, 0, 4)[3], 0);
    }

    #[test]
    fn drop_shadow_blurs_and_the_source_covers_the_overlap() {
        // Blur 1 spreads the offset shadow one ring in every direction.
        let out = drop_shadow(&square(), RED, 90.0, 3, 1, false);
        assert_eq!(
            px(&out, 4, 5),
            [255, 255, 255, 255],
            "source covers the shadow"
        );
        assert_eq!(
            px(&out, 4, 7),
            [255, 0, 0, 255],
            "blurred interior is full red"
        );
        // Side edge softens (the bottom edge is cut by the surface and
        // the box blur's edge clamp keeps that cut hard).
        let ring = px(&out, 1, 7);
        assert!(
            ring[3] > 0 && ring[3] < 255,
            "blur softens the side edge: {ring:?}"
        );
    }

    #[test]
    fn knockout_thins_the_shadow_under_a_translucent_source() {
        let mut ghost = Surface::new(9, 9).unwrap();
        for y in 2..=6u32 {
            for x in 2..=6u32 {
                ghost.set_straight(x, y, Rgba8::new(255, 255, 255, 128));
            }
        }
        let plain = drop_shadow(&ghost, RED, 0.0, 0, 0, false);
        let knocked = drop_shadow(&ghost, RED, 0.0, 0, 0, true);
        let inside_plain = px(&plain, 4, 4);
        let inside_knock = px(&knocked, 4, 4);
        assert!(
            inside_knock[3] < inside_plain[3],
            "knockout thins the shadow under the source: {inside_plain:?} vs {inside_knock:?}"
        );
        // Outside the silhouette knockout changes nothing.
        assert_eq!(px(&plain, 0, 0), px(&knocked, 0, 0));
    }

    #[test]
    fn inner_shadow_darkens_the_edge_opposite_the_light() {
        // Light from above → shadow collects along the TOP inside edge.
        let out = inner_shadow(&square(), RED, 90.0, 2, 0);
        assert_eq!(px(&out, 4, 2), [255, 0, 0, 255], "top edge red");
        assert_eq!(
            px(&out, 4, 6),
            [255, 255, 255, 255],
            "bottom edge untouched"
        );
        assert_eq!(px(&out, 0, 0)[3], 0, "nothing outside the object");
        // No offset but a blur: the outside bleeds in one ring.
        let even = inner_shadow(&square(), RED, 90.0, 0, 1);
        assert_eq!(
            px(&even, 4, 4),
            [255, 255, 255, 255],
            "interior too deep to reach"
        );
        let bled = px(&even, 4, 2);
        assert!(
            bled[0] > 170 && bled[3] == 255,
            "edge picks up shadow: {bled:?}"
        );
    }

    #[test]
    fn solid_shadow_replaces_or_tops_the_object() {
        let with_object = solid_shadow(&square(), RED, true);
        assert_eq!(
            px(&with_object, 4, 4),
            [255, 255, 255, 255],
            "source on top"
        );
        assert_eq!(px(&with_object, 1, 1)[3], 0, "no silhouette outside");
        let flat = solid_shadow(&square(), RED, false);
        assert_eq!(px(&flat, 4, 4), [255, 0, 0, 255], "flat red silhouette");
        assert_eq!(px(&flat, 4, 8)[3], 0, "no offset, no spill");
    }

    #[test]
    fn glows_ring_the_object_without_offset() {
        let out = outer_glow(&square(), RED, 1, 255);
        assert_eq!(px(&out, 4, 0)[3], 0, "blur 1 reaches one ring only");
        assert!(px(&out, 4, 1)[3] > 0, "one ring out glows");
        assert_eq!(px(&out, 4, 4), [255, 255, 255, 255], "object intact");
        // Edge threshold hardens: a half-covered pixel contributes to
        // the glow below the threshold and is cut above it.
        let mut soft = Surface::new(9, 9).unwrap();
        soft.set_straight(4, 4, Rgba8::new(255, 255, 255, 128));
        assert_eq!(
            outer_glow(&soft, RED, 0, 255),
            soft,
            "cut source adds no glow"
        );
        let kept = outer_glow(&soft, RED, 0, 0);
        assert!(
            kept.get(4, 4).unwrap()[3] > 128,
            "kept alpha glows under the source: {:?}",
            kept.get(4, 4).unwrap()
        );
        // Inner glow: the inside edge lights up (an 85-alpha red tint
        // over white reads as pink), the interior and the outside stay
        // clean.
        let in_glow = inner_glow(&square(), RED, 1, 255);
        let edge = px(&in_glow, 4, 2);
        assert_eq!(edge, [255, 170, 170, 255], "inside edge tinted: {edge:?}");
        assert_eq!(
            px(&in_glow, 4, 4),
            [255, 255, 255, 255],
            "interior keeps source"
        );
        assert_eq!(px(&in_glow, 0, 0)[3], 0);
    }

    #[test]
    fn shadow_color_alpha_scales_the_effect() {
        let half = Rgba8::new(255, 0, 0, 128);
        let out = drop_shadow(&square(), half, 90.0, 2, 0, false);
        assert_eq!(px(&out, 4, 8), [128, 0, 0, 128], "premultiplied half-red");
        let gone = drop_shadow(&square(), Rgba8::new(255, 0, 0, 0), 90.0, 2, 0, false);
        assert_eq!(px(&gone, 4, 8)[3], 0, "fully transparent shadow vanishes");
    }

    #[test]
    fn empty_input_is_a_fixed_point_and_results_are_deterministic() {
        let empty = Surface::new(9, 9).unwrap();
        assert_eq!(drop_shadow(&empty, RED, 45.0, 3, 2, false), empty);
        assert_eq!(inner_shadow(&empty, RED, 45.0, 3, 2), empty);
        assert_eq!(solid_shadow(&empty, RED, false), empty);
        assert_eq!(outer_glow(&empty, RED, 2, 255), empty);
        let s = square();
        assert_eq!(
            drop_shadow(&s, RED, 45.0, 3, 2, false),
            drop_shadow(&s, RED, 45.0, 3, 2, false)
        );
        assert_eq!(inner_glow(&s, RED, 2, 0), inner_glow(&s, RED, 2, 0));
    }
}
