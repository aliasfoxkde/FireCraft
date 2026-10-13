//! Golden-image gate: renders the corpus and pins the output.
//!
//! Three layers, per the M4 plan:
//!
//! - **Semantic anchors** — fixtures whose pixels are hand-computable
//!   must compose to exactly those values. This is the end-to-end
//!   oracle: fwpng parse → doc model → compose, checked against
//!   arithmetic rather than against itself.
//! - **Determinism pins** — every renderable fixture's output is
//!   sha256-pinned in a `<fixture>.render` sidecar; the gate renders
//!   twice (byte-identical required) and checks the pin. Re-pin after
//!   an intentional renderer change with
//!   `cargo run -p xtask -- golden --bless` (also run automatically by
//!   `corpus-gen`).
//! - **CS6 comparison** — when a real Fireworks CS6 export exists at
//!   `corpus/incoming/cs6/<stem>.png` (never committed), the render is
//!   composited over white and scored against it with
//!   `image_compare::rgb_hybrid_compare`. Report-only this milestone
//!   ([`CS6_SIMILARITY_FLOOR`] is recorded, not enforced) until exports
//!   land in `corpus/incoming/`.

use crate::corpus::manifest_entries;
use crate::util::{inflate, workspace_root};
use compose::Render;
use fwpng::chunk::parse_chunks;
use sha2::{Digest, Sha256};
use std::path::Path;

const FIXTURE_PREFIX: &str = "corpus/fixtures/";
const CS6_SIDECAR_DIR: &str = "corpus/incoming/cs6";

/// Recorded floor for CS6-export similarity (hybrid MSSIM + color).
/// Report-only until real exports land; raise to enforced with them.
pub const CS6_SIMILARITY_FLOOR: f64 = 0.95;

/// Gate entry point: anchors + determinism pins + CS6 report.
pub fn run() -> Result<(), String> {
    run_inner(false)
}

/// `xtask golden --bless`: re-render and rewrite every pin.
pub fn bless() -> Result<(), String> {
    run_inner(true)
}

fn run_inner(write_pins: bool) -> Result<(), String> {
    let mut rendered = 0usize;
    let mut blessed = 0usize;
    let mut compared = 0usize;
    let mut errors = Vec::new();
    for (_, rel) in manifest_entries()? {
        if !rel.starts_with(FIXTURE_PREFIX) {
            continue;
        }
        let bytes = match std::fs::read(workspace_root().join(&rel)) {
            Ok(b) => b,
            Err(err) => {
                errors.push(format!("{rel}: {err}"));
                continue;
            }
        };
        let render = match render_entry(&bytes) {
            Ok(None) => continue,
            Ok(Some(render)) => render,
            Err(err) => {
                errors.push(format!("{rel}: {err}"));
                continue;
            }
        };
        rendered += 1;
        let name = &rel[FIXTURE_PREFIX.len()..];

        if let Err(err) = semantic_anchor(name, &render) {
            errors.push(err);
        }

        // Determinism floor: a second render must be byte-identical.
        match render_entry(&bytes) {
            Ok(Some(second)) if second == render => {}
            Ok(Some(_)) => errors.push(format!("{name}: renders are not deterministic")),
            Ok(None) => errors.push(format!("{name}: second render found no pages")),
            Err(err) => errors.push(format!("{name}: second render failed: {err}")),
        }

        let pin = Path::new(&rel).with_file_name(format!("{name}.render"));
        let pin_path = workspace_root().join(&pin);
        if write_pins {
            let text = format!("{}\n", output_digest(&render));
            if let Err(err) = std::fs::write(&pin_path, text) {
                errors.push(format!("{}: write pin: {err}", pin.display()));
            } else {
                blessed += 1;
            }
        } else {
            match std::fs::read_to_string(&pin_path) {
                Ok(want) => {
                    let want = want.trim();
                    let got = output_digest(&render);
                    if got != want {
                        errors.push(format!(
                            "{name}: render sha256 {got} != pinned {want} \
                             (intentional change? `cargo run -p xtask -- golden --bless`)"
                        ));
                    }
                }
                Err(_) => errors.push(format!(
                    "{}: missing golden pin — run `cargo run -p xtask -- golden --bless`",
                    pin.display()
                )),
            }
        }

        compare_cs6(name, &render, &mut compared);
    }

    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    if write_pins {
        eprintln!("[xtask] golden: {rendered} render(s), {blessed} pin(s) blessed");
    } else {
        eprintln!(
            "[xtask] golden: {rendered} render(s) pinned and deterministic, \
             {compared} CS6 comparison(s) (report-only, floor {CS6_SIMILARITY_FLOOR})"
        );
    }
    Ok(())
}

/// Parses a corpus entry and composes its first page's first state.
/// `Ok(None)` when the file has nothing renderable (no `mkTS`, or no
/// pages — the non-compositor fixtures stay in the corpus for their own
/// gates).
pub fn render_entry(bytes: &[u8]) -> Result<Option<Render>, String> {
    let chunks = parse_chunks(bytes).map_err(|e| format!("parse_chunks: {e}"))?;
    let Some(chunk) = chunks.iter().find(|c| &c.ctype == b"mkTS") else {
        return Ok(None);
    };
    let inflated = inflate(&chunk.data)?;
    let nodes = fwpng::nodetree::parse(&inflated).map_err(|e| format!("mkTS parse: {e}"))?;
    let (document, _bag) = doc::fwmap::read(&nodes).map_err(|e| format!("doc read: {e}"))?;
    if document.pages.is_empty() {
        return Ok(None);
    }
    compose::render(&document, 0, 0)
        .map(Some)
        .map_err(|e| format!("compose: {e}"))
}

/// Pin digest: dimensions plus pixels (both matter — a size change with
/// identical colours must not pass silently).
fn output_digest(render: &Render) -> String {
    let mut hasher = Sha256::new();
    hasher.update(render.width.to_be_bytes());
    hasher.update(render.height.to_be_bytes());
    hasher.update(&render.pixels);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hand-computed pixel expectations per fixture. Only fixtures whose
/// output is derivable by arithmetic belong here; everything else is
/// covered by the determinism pins alone.
fn semantic_anchor(name: &str, render: &Render) -> Result<(), String> {
    match name {
        "composed.fw.png" => anchor_composed(name, render),
        "text.fw.png" => anchor_text(name, render),
        "textpath.fw.png" => anchor_textpath(name, render),
        "families.fw.png" => anchor_families(name, render),
        "gradient.fw.png" => anchor_gradient(name, render),
        _ => Ok(()),
    }
}

/// The per-family golden: twelve 20×16 rects on a 4×3 grid at 32px
/// pitch, each carrying exactly one live effect. Cells whose kernels
/// are interior-exact (point ops, the interior-fixed-point sharpens,
/// angle-0 motion blur at distance 4) check hand-computed pixels at
/// the cell center; the radial/zoom blur cells tap across the raster
/// and stay determinism-only. Straight colors here: interior pixels
/// of opaque fills compose exactly.
fn anchor_families(name: &str, render: &Render) -> Result<(), String> {
    let pixel = |x: u32, y: u32| -> [u8; 4] {
        let index = ((y * u32::from(render.width) + x) as usize) * 4;
        [
            render.pixels[index],
            render.pixels[index + 1],
            render.pixels[index + 2],
            render.pixels[index + 3],
        ]
    };
    // Cell centers: column c, row r → (c*32+16, r*32+16).
    // (row 0) hue+180 shifts opaque red to cyan; colorize REPLACES the
    // hue — red's L=.5/S=1 at hue 120 is pure green.
    // (row 0/1) levels: the red-channel override [0,128]→[0,255] lifts
    // 128→255, green/blue ride the identity master.
    // curves: knots (0,0)(128,64)(255,255) at x=128 → 64 (red),
    // x=64 → 32 (green), x=255 → 255 (blue).
    // autoLevels on one flat value: the histogram collapses to one
    // bin → identity. colorFill at 100% normal: the fill color.
    // convertToAlpha: white's luma 255 becomes alpha, color goes black.
    // sharpen/sharpenMore: a flat interior is a fixed point of both
    // stencils. motionBlur at angle 0 distance 4 taps pixels whose
    // neighborhood is the same fill → the fill itself.
    for (cell, want, label) in [
        (0usize, [0, 255, 255, 255], "hue shift 180 on red"),
        (1, [0, 255, 0, 255], "colorize red to hue 120"),
        (2, [255, 64, 255, 255], "per-channel levels lift"),
        (3, [64, 32, 255, 255], "curves knots"),
        (4, [255, 0, 0, 255], "auto levels on flat fill"),
        (5, [51, 102, 153, 255], "color fill 100%"),
        (6, [0, 0, 0, 255], "convert to alpha on white"),
        (7, [200, 100, 50, 255], "sharpen fixed point"),
        (8, [200, 100, 50, 255], "sharpen more fixed point"),
        (9, [255, 0, 0, 255], "motion blur angle 0"),
    ] {
        let (c, r) = ((cell % 4) as u32, (cell / 4) as u32);
        let got = pixel(c * 32 + 16, r * 32 + 16);
        if got != want {
            return Err(format!(
                "{name}: cell {cell} ({label}) center is {got:?}, hand-computed {want:?}"
            ));
        }
    }
    // The determinism-only cells must still be painted (an effect that
    // blanked its object would pin happily).
    for cell in [10usize, 11] {
        let (c, r) = ((cell % 4) as u32, (cell / 4) as u32);
        let got = pixel(c * 32 + 16, r * 32 + 16);
        if got[3] == 0 {
            return Err(format!(
                "{name}: cell {cell} center is transparent — the effect erased its object"
            ));
        }
    }
    Ok(())
}

/// The gradient golden. Three rects on white: a black→white linear
/// ramp along x (vector (4,14)→(28,14), length 24), a black→white
/// radial from (48,14) with radius 20, and a blue→yellow linear whose
/// coverage ramp goes transparent→opaque. Position-derived greys are
/// asserted as bounds wide enough to cover either rounding of the
/// exact product; ramp identity (grey channels equal), axis
/// invariance and the unpainted (transparent) canvas are exact. The
/// veil reads premultiplied: `Render` is premultiplied and compose
/// leaves the canvas colour to the host, so a veil pixel is its ramp
/// colour scaled by its own coverage.
fn anchor_gradient(name: &str, render: &Render) -> Result<(), String> {
    let pixel = |x: u32, y: u32| -> [u8; 4] {
        let index = ((y * u32::from(render.width) + x) as usize) * 4;
        [
            render.pixels[index],
            render.pixels[index + 1],
            render.pixels[index + 2],
            render.pixels[index + 3],
        ]
    };
    let grey = |got: [u8; 4], label: &str| -> Result<u8, String> {
        if got[0] != got[1] || got[1] != got[2] {
            return Err(format!("{name}: {label} is not grey: {got:?}"));
        }
        Ok(got[0])
    };
    // Linear rect (4,6)-(28,22), ramp 0→255 along the 24-unit vector:
    // the left edge samples t≈0.02, the right edge t≈0.98, the middle
    // t≈0.52 → 132.9.
    let left = grey(pixel(4, 14), "linear left edge")?;
    if left > 8 {
        return Err(format!("{name}: linear left edge is {left}, want ≤ 8"));
    }
    let right = grey(pixel(27, 14), "linear right edge")?;
    if right < 248 {
        return Err(format!("{name}: linear right edge is {right}, want ≥ 248"));
    }
    let mid = grey(pixel(16, 14), "linear middle")?;
    if !(130..=136).contains(&mid) {
        return Err(format!("{name}: linear middle is {mid}, want ≈ 133"));
    }
    // Axis invariance: a horizontal ramp is constant down a column.
    if grey(pixel(10, 8), "linear row 8")? != grey(pixel(10, 19), "linear row 19")? {
        return Err(format!("{name}: the linear ramp drifts across rows"));
    }
    // Radial rect (36,6)-(60,22): the start (48,14) samples ≈0.7/20;
    // the far corner ≈13.7/20 → 175.5.
    let center = grey(pixel(48, 14), "radial center")?;
    if center > 12 {
        return Err(format!("{name}: radial center is {center}, want ≤ 12"));
    }
    let corner = grey(pixel(59, 21), "radial corner")?;
    if !(172..=179).contains(&corner) {
        return Err(format!("{name}: radial corner is {corner}, want ≈ 176"));
    }
    // The veil (4,28)-(60,42): blue→yellow under a transparent→opaque
    // coverage ramp. `Render` is premultiplied and the canvas colour is
    // the host's to paint, so each pixel is the ramp colour scaled by
    // its own coverage. Left (5,35): t = 1.5/56 = 0.027 → straight
    // (7,7,248) at α7 → premultiplied (0,0,7) at 7. Mid (30,35):
    // t = 0.473 → (121,121,134) at α121 → premul ≈ (57,57,64) at 120.
    // Right (59,35): t = 0.991 → (253,253,2) at α253 → premul ≈ (250,
    // 250,2) at 252. Bounds cover either rounding of the products.
    let near = pixel(5, 35);
    if near != [0, 0, 6, 6] {
        return Err(format!(
            "{name}: veil left end is {near:?}, want premultiplied (0,0,6,6)"
        ));
    }
    let half = pixel(30, 35);
    if !(54..=59).contains(&half[0])
        || !(54..=59).contains(&half[1])
        || !(62..=67).contains(&half[2])
        || !(118..=122).contains(&half[3])
    {
        return Err(format!(
            "{name}: veil middle is {half:?}, want premul ≈ (57,57,64,120)"
        ));
    }
    let saturated = pixel(59, 35);
    if !(247..=252).contains(&saturated[0])
        || !(247..=252).contains(&saturated[1])
        || saturated[2] > 6
        || !(250..=253).contains(&saturated[3])
    {
        return Err(format!(
            "{name}: veil right end is {saturated:?}, want premul ≈ (250,250,2,252)"
        ));
    }
    // Coverage rises left→right along the veil.
    let (mut early, mut late) = (0u32, 0u32);
    for (sample, slot) in [(20u32, &mut early), (40, &mut late)] {
        let index = ((35 * u32::from(render.width) + sample) as usize) * 4;
        *slot = u32::from(render.pixels[index + 3]);
    }
    if early >= late {
        return Err(format!(
            "{name}: coverage did not rise left→right ({early} → {late})"
        ));
    }
    // Nothing painted above the rects: compose leaves the canvas
    // transparent (the canvas colour is the host's to paint).
    let bare = pixel(32, 2);
    if bare != [0, 0, 0, 0] {
        return Err(format!(
            "{name}: unpainted canvas pixel is {bare:?}, want transparent"
        ));
    }
    Ok(())
}

fn anchor_composed(name: &str, render: &Render) -> Result<(), String> {
    let pixel = |x: u32, y: u32| -> [u8; 4] {
        let index = ((y * u32::from(render.width) + x) as usize) * 4;
        [
            render.pixels[index],
            render.pixels[index + 1],
            render.pixels[index + 2],
            render.pixels[index + 3],
        ]
    };
    // Black 400‰ (α = 0.4) over opaque red: 255 × 0.6 = 153.
    let left = [153, 0, 0, 255];
    let right = [255, 0, 0, 255];
    for (x, y, want) in [
        (0, 0, left),
        (7, 0, left),
        (0, 15, left),
        (7, 15, left),
        (8, 0, right),
        (15, 0, right),
        (8, 15, right),
        (15, 15, right),
    ] {
        let got = pixel(x, y);
        if got != want {
            return Err(format!(
                "{name}: pixel ({x},{y}) is {got:?}, hand-computed {want:?}"
            ));
        }
    }
    Ok(())
}

/// The text golden: "FireCraft" set at 10px in frame (12,18,40,12),
/// filled ABGR 0xFF00_0000 (opaque black), nothing else on the page.
/// Every ink pixel must be premultiplied black (any channel value with
/// alpha means a broken fill decode), must fall inside the frame (±1px
/// antialiasing slack), and must exist — an empty render pins happily,
/// so emptiness is asserted, not assumed.
fn anchor_text(name: &str, render: &Render) -> Result<(), String> {
    const FRAME: (f64, f64, f64, f64) = (12.0, 18.0, 40.0, 12.0);
    let mut ink = 0usize;
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for y in 0..u32::from(render.height) {
        for x in 0..u32::from(render.width) {
            let index = ((y * u32::from(render.width) + x) as usize) * 4;
            let (r, g, b, a) = (
                render.pixels[index],
                render.pixels[index + 1],
                render.pixels[index + 2],
                render.pixels[index + 3],
            );
            if a == 0 {
                continue;
            }
            if r != 0 || g != 0 || b != 0 {
                return Err(format!(
                    "{name}: ink at ({x},{y}) is rgb({r},{g},{b}) — text fill must be black"
                ));
            }
            let (fx, fy, fw, fh) = FRAME;
            let inside = (x as f64) >= fx - 1.0
                && (x as f64) <= fx + fw + 1.0
                && (y as f64) >= fy - 1.0
                && (y as f64) <= fy + fh + 1.0;
            if !inside {
                return Err(format!(
                    "{name}: ink at ({x},{y}) falls outside the text frame {FRAME:?}"
                ));
            }
            ink += 1;
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    let Some((x0, y0, x1, y1)) = bounds else {
        return Err(format!("{name}: nothing rendered — text produced no ink"));
    };
    // Nine glyphs at 10px spread well past 40 dark pixels; a shaper or
    // lowering collapse shows up as a near-empty box.
    if ink < 40 || (x1 - x0) < 20 {
        return Err(format!(
            "{name}: ink collapsed to {ink} px in a {x0}..{x1} × {y0}..{y1} box"
        ));
    }
    Ok(())
}

/// The on-path golden: text must ride the arc. A linear-flow fallback
/// would still render black ink — inside the frame's ~12px band — so
/// the anchor demands a vertical span the rail's 24px rise makes
/// impossible without on-path lowering.
fn anchor_textpath(name: &str, render: &Render) -> Result<(), String> {
    let mut ink = 0usize;
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for y in 0..u32::from(render.height) {
        for x in 0..u32::from(render.width) {
            let index = ((y * u32::from(render.width) + x) as usize) * 4;
            let (r, g, b, a) = (
                render.pixels[index],
                render.pixels[index + 1],
                render.pixels[index + 2],
                render.pixels[index + 3],
            );
            if a == 0 {
                continue;
            }
            if r != 0 || g != 0 || b != 0 {
                return Err(format!(
                    "{name}: ink at ({x},{y}) is rgb({r},{g},{b}) — text fill must be black"
                ));
            }
            ink += 1;
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    let Some((x0, y0, x1, y1)) = bounds else {
        return Err(format!(
            "{name}: nothing rendered — on-path text produced no ink"
        ));
    };
    if ink < 40 || (x1 - x0) < 20 {
        return Err(format!(
            "{name}: ink collapsed to {ink} px in a {x0}..{x1} × {y0}..{y1} box"
        ));
    }
    // The rail climbs 24px over its run; glyphs rotate with it. A
    // linear fallback spans one 8px line instead.
    let span = u64::from(y1 - y0);
    if span < 20 {
        return Err(format!(
            "{name}: ink spans only {span}px vertically — text did not follow the arc"
        ));
    }
    Ok(())
}

/// Report-only similarity check against a real CS6 export, if one has
/// been dropped in for this fixture.
fn compare_cs6(name: &str, render: &Render, compared: &mut usize) {
    let stem = name.strip_suffix(".fw.png").unwrap_or(name);
    let sidecar = workspace_root()
        .join(CS6_SIDECAR_DIR)
        .join(format!("{stem}.png"));
    let Ok(expected) = image::open(&sidecar) else {
        return;
    };
    let got = over_white(render);
    let expected = expected.to_rgb8();
    if got.dimensions() != expected.dimensions() {
        eprintln!(
            "[xtask] golden: cs6 {stem}: SKIPPED (sidecar is {}x{}, render is {}x{})",
            expected.dimensions().0,
            expected.dimensions().1,
            got.dimensions().0,
            got.dimensions().1
        );
        return;
    }
    match image_compare::rgb_hybrid_compare(&got, &expected) {
        Ok(result) => {
            *compared += 1;
            let verdict = if result.score >= CS6_SIMILARITY_FLOOR {
                "meets"
            } else {
                "BELOW"
            };
            eprintln!(
                "[xtask] golden: cs6 {stem}: similarity {:.4} ({verdict} floor, report-only)",
                result.score
            );
        }
        Err(err) => eprintln!("[xtask] golden: cs6 {stem}: compare failed: {err}"),
    }
}

/// Composites premultiplied RGBA over opaque white for RGB comparison:
/// `out = fg_premul + white × (1 − α)` — no un-premultiplication needed.
fn over_white(render: &Render) -> image::RgbImage {
    let mut image = image::RgbImage::new(u32::from(render.width), u32::from(render.height));
    let width = u32::from(render.width);
    for (i, px) in render.pixels.chunks_exact(4).enumerate() {
        let alpha = f64::from(px[3]) / 255.0;
        let over = |fg_premul: u8, bg: u8| -> u8 {
            let value = f64::from(fg_premul) + f64::from(bg) * (1.0 - alpha);
            value.round().clamp(0.0, 255.0) as u8
        };
        let x = i as u32 % width;
        let y = i as u32 / width;
        image.put_pixel(
            x,
            y,
            image::Rgb([over(px[0], 255), over(px[1], 255), over(px[2], 255)]),
        );
    }
    image
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::over_white;
    use compose::Render;

    #[test]
    fn opaque_pixels_pass_through_over_white() {
        let render = Render {
            width: 2,
            height: 1,
            pixels: vec![255, 0, 0, 255, 0, 255, 0, 255],
        };
        let out = over_white(&render);
        assert_eq!(out.get_pixel(0, 0).0, [255, 0, 0]);
        assert_eq!(out.get_pixel(1, 0).0, [0, 255, 0]);
    }

    #[test]
    fn transparent_pixels_composite_to_white() {
        let render = Render {
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0, 0],
        };
        assert_eq!(over_white(&render).get_pixel(0, 0).0, [255, 255, 255]);
    }

    #[test]
    fn premultiplied_half_alpha_over_white_is_exact() {
        // Red 400‰ premultiplied: (102, 0, 0, 102). Over white:
        // r = 102 + 255 × 0.6 = 255 × 0.4 + 255 × 0.6 = 255; g/b = 0 +
        // 153 = 153. All terms integer-exact.
        let render = Render {
            width: 1,
            height: 1,
            pixels: vec![102, 0, 0, 102],
        };
        assert_eq!(over_white(&render).get_pixel(0, 0).0, [255, 153, 153]);
    }
}
