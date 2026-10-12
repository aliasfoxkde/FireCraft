//! The CSS-side workflows, from the `CssLayers.htt` behavioral spec:
//! CSS+Images (XHTML 1.0 Transitional, inline `@charset "utf-8"`
//! stylesheet, `#main` wrap, absolutely-positioned `div`s with
//! background images — the template's `csspos` absolute branch), CSS
//! Sprites (one sheet + per-slice `background-position` rules), and
//! the per-object CSS Properties text artifact.

use crate::WebExportOptions;
use crate::images::Piece;
use crate::layout::Grid;
use crate::naming;
use doc::EffectValue;

/// The stylesheet's fixed preamble, byte-for-byte the template's
/// constants (`pMargin` 1.1 included) — emitted after the `body`
/// rule, per the template's emission order.
const CSS_PREAMBLE: &str = "\
body * {
	font-size: 100%;
}
h1, h2, h3, h4, h5, h6 {
	font-weight: normal;
}
p {
	margin-bottom: 1.1em;
	margin-top: 0;
}
#main p.lastNode {
	margin-bottom: 0;
}
a:link img, a:visited img {
	border: none;
}
div.clearFloat {
	clear: both;
	font-size: 0;
	height: 0;
	line-height: 0px;
}
";

/// The canvas background as `#rrggbb` (raw is ABGR).
fn bg_hex(raw: i64) -> String {
    let raw = raw as u32;
    format!(
        "#{:02x}{:02x}{:02x}",
        raw & 0xFF,
        (raw >> 8) & 0xFF,
        (raw >> 16) & 0xFF
    )
}

/// The stylesheet for CSS+Images: `@charset`, the `body` rule, the
/// fixed preamble, `#main`, then one rule per piece in the template's
/// `csspos` absolute-branch order — left, position, top,
/// background-image, overflow, height, margin-bottom, width.
fn css_and_images_rules(
    document: &doc::Document,
    options: &WebExportOptions,
    grid: &Grid,
    pieces: &[Piece],
) -> String {
    let mut css = String::with_capacity(2048);
    css.push_str("@charset \"utf-8\";\n\n");
    if let Some(page) = document.pages.get(options.page) {
        css.push_str(&format!(
            "body {{\n\tbackground-color: {};\n\tfont-size: 62.5%;\n\tmargin: 0;\n\tpadding: 0;\n}}\n",
            bg_hex(page.info.canvas.background_raw)
        ));
    }
    css.push_str(CSS_PREAMBLE);
    let width = grid.x_cuts.last().copied().unwrap_or(0);
    css.push_str(&format!(
        "#main {{\n\tmargin: 0 auto;\n\twidth: {width}px;\n}}\n"
    ));
    for piece in pieces {
        if piece.object.is_none() {
            continue; // undefined-area pieces carry no div
        }
        let name = naming::stem_of(&piece.file);
        let src = format!("{}{}", options.images_dir, piece.file);
        let (x, y, w, h) = grid.cell_rect(piece.row, piece.col);
        css.push_str(&format!(
            "#{name} {{\n\tleft: {x}px;\n\tposition: absolute;\n\ttop: {y}px;\n\tbackground-image: url({src});\n\toverflow: hidden;\n\theight: {h}px;\n\tmargin-bottom: 0;\n\twidth: {w}px;\n}}\n"
        ));
    }
    css
}

/// Writes the CSS+Images document: XHTML 1.0 Transitional shell,
/// inline stylesheet (preamble + rules), IE conditional block, and
/// the `#main` div tree — one empty div per slice piece.
pub(crate) fn write_css_and_images(
    document: &doc::Document,
    options: &WebExportOptions,
    grid: &Grid,
    pieces: &[Piece],
) -> String {
    let mut out = String::with_capacity(4096);
    out.push_str("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\" \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">\n");
    out.push_str("<html xmlns=\"http://www.w3.org/1999/xhtml\">\n");
    out.push_str("<head>\n");
    out.push_str("\t<meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\" />\n");
    out.push_str(&format!("\t<title>{}</title>\n", options.title));
    out.push_str("\t<style type=\"text/css\" media=\"all\">\n");
    out.push_str(&css_and_images_rules(document, options, grid, pieces));
    out.push_str("\t</style>\n");
    out.push_str("\t<!--[if IE]>\n");
    out.push_str(
        "\t<style type=\"text/css\" media=\"all\">.borderitem {border-style: solid;}</style>\n",
    );
    out.push_str("\t<![endif]-->\n");
    out.push_str("</head>\n\n");
    out.push_str("<body>\n");
    out.push_str("<div id=\"main\">\n");
    for piece in pieces {
        let Some(_) = piece.object else {
            continue;
        };
        let name = naming::stem_of(&piece.file);
        out.push_str(&format!("<div id=\"{name}\"></div>\n"));
    }
    out.push_str("</div>\n");
    out.push_str("</body>\n");
    out.push_str("</html>\n");
    out
}

/// The sprite-sheet geometry: the sheet spans the union of the
/// pieces' rects. Returns `(x0, y0, width, height)`, or `None` when
/// no piece exists.
pub(crate) fn sheet_extent(grid: &Grid, pieces: &[Piece]) -> Option<(u32, u32, u32, u32)> {
    let mut extent: Option<(u32, u32, u32, u32)> = None;
    for piece in pieces {
        let (x, y, w, h) = grid.cell_rect(piece.row, piece.col);
        let x1 = x + w;
        let y1 = y + h;
        extent = Some(match extent {
            None => (x, y, x1, y1),
            Some((ex0, ey0, ex1, ey1)) => (ex0.min(x), ey0.min(y), ex1.max(x1), ey1.max(y1)),
        });
    }
    extent.map(|(x0, y0, x1, y1)| (x0, y0, x1 - x0, y1 - y0))
}

/// The sprite CSS: one rule per object piece with its
/// `background-position` inside the sheet (negative offsets, the
/// jQuery-facing form).
pub(crate) fn write_sprite_css(
    options: &WebExportOptions,
    grid: &Grid,
    pieces: &[Piece],
    sheet: &str,
    sheet_origin: (u32, u32),
) -> String {
    let mut css = String::with_capacity(1024);
    css.push_str(&format!(
        "@charset \"utf-8\";\n\n.sprite {{\n\tbackground-image: url({}{});\n\tbackground-repeat: no-repeat;\n\tdisplay: block;\n}}\n",
        options.images_dir, sheet
    ));
    for piece in pieces {
        if piece.object.is_none() {
            continue;
        }
        let name = naming::stem_of(&piece.file);
        let (x, y, w, h) = grid.cell_rect(piece.row, piece.col);
        let (ox, oy) = (
            x.saturating_sub(sheet_origin.0),
            y.saturating_sub(sheet_origin.1),
        );
        css.push_str(&format!(
            "#{name} {{\n\tbackground-position: -{ox}px -{oy}px;\n\twidth: {w}px;\n\theight: {h}px;\n}}\n"
        ));
    }
    css
}

/// The per-object CSS Properties text artifact: geometry, fill →
/// background-color, primitive radius, opacity, and the drop-shadow
/// effect's `box-shadow`, for every visible object on the exported
/// page's state, in paint order.
///
/// # Errors
/// [`WebExportError::PageOutOfBounds`] when `options.page` is outside
/// the document.
pub(crate) fn write_properties(
    document: &doc::Document,
    options: &WebExportOptions,
) -> Result<String, crate::WebExportError> {
    let Some(page) = document.pages.get(options.page) else {
        return Err(crate::WebExportError::PageOutOfBounds { page: options.page });
    };
    let mut out = String::with_capacity(1024);
    for layer in &page.layers.layers {
        if !layer.display {
            continue;
        }
        let Some(cell) = layer.cells.get(options.state) else {
            continue;
        };
        if !cell.visible {
            continue;
        }
        for id in &cell.elements {
            let Some(object) = document.registry.get(*id) else {
                continue;
            };
            if let Some(rule) = object_rule(id.index(), object) {
                out.push_str(&rule);
            }
        }
    }
    Ok(out)
}

/// One object's CSS rule, or `None` when the object carries nothing
/// the artifact renders.
fn object_rule(id: usize, object: &doc::ObjectData) -> Option<String> {
    let (bounds, fill, radius, effects, opacity, name, visible) = match object {
        doc::ObjectData::Path(path) => {
            let bounds = contour_bounds(&path.contours)?;
            (
                bounds,
                path.paint.as_ref().map(|paint| paint.fill_raw),
                path.primitive.as_ref().map(|primitive| primitive.radius),
                path.effects.as_slice(),
                path.opacity,
                path.name.as_deref(),
                path.visible,
            )
        }
        doc::ObjectData::Slice(slice) => (
            (
                slice.frame.x0,
                slice.frame.y0,
                slice.frame.x1 - slice.frame.x0,
                slice.frame.y1 - slice.frame.y0,
            ),
            Some(slice.fill_raw),
            None,
            &[] as &[doc::Effect],
            1000,
            slice.name.as_deref(),
            true,
        ),
        _ => return None,
    };
    if !visible {
        return None;
    }
    let selector = match name {
        Some(raw) if !naming::clean(raw).is_empty() => naming::clean(raw),
        _ => format!("object{id}"),
    };
    let (x0, y0, w, h) = bounds;
    let mut css = format!(
        "#{selector} {{\n\tleft: {:.0}px;\n\ttop: {:.0}px;\n\twidth: {:.0}px;\n\theight: {:.0}px;\n",
        x0, y0, w, h
    );
    if let Some(raw) = fill {
        css.push_str(&format!("\tbackground-color: {};\n", bg_hex(raw)));
    }
    if let Some(radius) = radius.filter(|radius| *radius > 0.0) {
        css.push_str(&format!("\tborder-radius: {radius:.0}px;\n"));
    }
    if opacity != 1000 {
        css.push_str(&format!("\topacity: {:.3};\n", opacity as f64 / 1000.0));
    }
    if let Some(shadow) = effects
        .iter()
        .find(|effect| effect.internal_name.eq_ignore_ascii_case("dropShadow"))
    {
        let mut blur = 0.0;
        let mut dx = 0.0;
        let mut dy = 0.0;
        for property in &shadow.properties {
            let EffectValue::Text(text) = &property.value else {
                continue;
            };
            let Ok(value) = text.parse::<f64>() else {
                continue;
            };
            match property.key.as_str() {
                "gaussian_blur_radius" => blur = value,
                "x_offset" => dx = value,
                "y_offset" => dy = value,
                _ => {}
            }
        }
        css.push_str(&format!(
            "\tbox-shadow: {dx:.0}px {dy:.0}px {blur:.0}px #000000;\n"
        ));
    }
    css.push_str("}\n");
    Some(css)
}

/// The axis-aligned bounds of the contour anchors.
fn contour_bounds(contours: &[doc::Contour]) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    for contour in contours {
        for point in &contour.points {
            let (x, y) = (point.anchor.x, point.anchor.y);
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    bounds.map(|(x0, y0, x1, y1)| (x0, y0, x1 - x0, y1 - y0))
}
