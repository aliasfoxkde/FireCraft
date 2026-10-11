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
        _ => Ok(()),
    }
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
