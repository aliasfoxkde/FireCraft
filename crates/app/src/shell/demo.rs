//! Real FireCraft Studio 1080p Showcase document builder.
//!
//! Constructs an editable 1920×1080 showcase through the real `engine` command
//! registry (vector cards, multi-layer FireCraft flame emblem, typography,
//! CTA button, and web slice), so every element on screen is genuinely
//! selectable, editable, undoable, and exportable to `.fw.png`.

use compose::abgr_pack;
use doc::ObjectId;

use crate::editor::Editor;

fn pack_rgb(r: u8, g: u8, b: u8) -> i64 {
    abgr_pack([r, g, b, 255])
}

fn latest_object_id(editor: &Editor) -> Option<ObjectId> {
    let page = editor.document().pages.get(editor.page)?;
    let layer = page.layers.layers.get(editor.active_layer)?;
    let cell = layer.cells.get(editor.frame)?;
    cell.objects.last().copied()
}

/// Build a fresh 1920×1080 FireCraft Studio Showcase `Editor`.
#[must_use]
pub fn build_showcase_editor() -> Editor {
    let mut editor = Editor::untitled();
    populate_showcase(&mut editor, 1920.0, 1080.0);
    editor
}

/// Populate `editor` with the FireCraft Studio showcase sized to `w × h`.
pub fn populate_showcase(editor: &mut Editor, w: f64, h: f64) {
    let page = editor.page;
    let layer = editor.active_layer;
    let cell = editor.frame;

    let _ = editor.apply(
        "page.setCanvasSize",
        serde_json::json!({ "page": page, "width": w, "height": h }),
    );
    let _ = editor.apply(
        "document.setResolution",
        serde_json::json!({ "resolution": 144.0, "unit": "inch" }),
    );
    let _ = editor.apply(
        "document.setCanvasColor",
        serde_json::json!({ "color_raw": pack_rgb(16, 18, 26) }),
    );

    let pad = (w * 0.035).clamp(24.0, 64.0);
    let hero_h = (h * 0.40).clamp(200.0, 420.0);

    // 1. Hero Brand Card
    add_rect(
        editor,
        pad,
        pad,
        w - pad * 2.0,
        hero_h,
        16.0,
        pack_rgb(30, 34, 48),
        pack_rgb(232, 163, 61),
        "Hero Brand Card",
    );

    // 2. Multi-layer FireCraft Flame Vector Emblem
    let fcx = pad + (w * 0.12).clamp(90.0, 210.0);
    let fcy = pad + hero_h * 0.52;
    let fs = (hero_h * 0.36).clamp(54.0, 140.0);

    add_poly(
        editor,
        &[
            [fcx, fcy - fs],
            [fcx + fs * 0.42, fcy - fs * 0.35],
            [fcx + fs * 0.68, fcy + fs * 0.22],
            [fcx + fs * 0.46, fcy + fs * 0.78],
            [fcx, fcy + fs * 0.92],
            [fcx - fs * 0.48, fcy + fs * 0.78],
            [fcx - fs * 0.66, fcy + fs * 0.18],
            [fcx - fs * 0.32, fcy - fs * 0.42],
        ],
        pack_rgb(230, 74, 38),
        pack_rgb(255, 190, 90),
        "FireCraft Flame Outer",
    );

    add_poly(
        editor,
        &[
            [fcx + fs * 0.04, fcy - fs * 0.56],
            [fcx + fs * 0.36, fcy + fs * 0.10],
            [fcx + fs * 0.28, fcy + fs * 0.64],
            [fcx, fcy + fs * 0.76],
            [fcx - fs * 0.32, fcy + fs * 0.58],
            [fcx - fs * 0.24, fcy + fs * 0.02],
        ],
        pack_rgb(245, 158, 42),
        0,
        "FireCraft Flame Mid",
    );

    add_poly(
        editor,
        &[
            [fcx, fcy - fs * 0.12],
            [fcx + fs * 0.18, fcy + fs * 0.34],
            [fcx, fcy + fs * 0.62],
            [fcx - fs * 0.18, fcy + fs * 0.34],
        ],
        pack_rgb(255, 232, 110),
        0,
        "FireCraft Flame Core",
    );

    // 3. Hero Title & Subtitle
    let text_x = fcx + fs * 1.05;
    let title_w = (w - pad - text_x - 32.0).max(200.0);
    add_text(
        editor,
        text_x,
        pad + hero_h * 0.16,
        title_w,
        84.0,
        "FireCraft Studio",
        (hero_h * 0.16).clamp(32.0, 60.0),
        true,
        pack_rgb(250, 208, 68),
        "Hero Brand Title",
    );
    add_text(
        editor,
        text_x,
        pad + hero_h * 0.42,
        title_w,
        44.0,
        "Open Source Vector, Bitmap & Web Graphics Editor · Rust + WASM",
        (hero_h * 0.062).clamp(16.0, 24.0),
        false,
        pack_rgb(198, 206, 224),
        "Hero Subtitle",
    );

    // 4. Primary CTA Button + Web Slice
    let cta_w = (w * 0.17).clamp(190.0, 310.0);
    let cta_h = (hero_h * 0.16).clamp(44.0, 64.0);
    let cta_x = text_x;
    let cta_y = pad + hero_h * 0.64;
    add_rect(
        editor,
        cta_x,
        cta_y,
        cta_w,
        cta_h,
        10.0,
        pack_rgb(232, 163, 61),
        pack_rgb(255, 224, 150),
        "Primary CTA Button",
    );
    add_text(
        editor,
        cta_x + 24.0,
        cta_y + cta_h * 0.28,
        cta_w - 36.0,
        cta_h * 0.6,
        "Explore Showcase",
        (cta_h * 0.36).clamp(15.0, 21.0),
        true,
        pack_rgb(24, 20, 14),
        "CTA Button Label",
    );

    // 5. Four Feature Cards
    let cards_top = pad + hero_h + (h * 0.04).clamp(24.0, 48.0);
    let cards_h = (h - pad - cards_top).max(220.0);
    let gap = (w * 0.02).clamp(16.0, 36.0);
    let card_w = ((w - pad * 2.0 - gap * 3.0) / 4.0).max(160.0);
    let features = [
        (
            "Vector + Bitmap",
            "Boolean paths, dynamic\nAuto Shapes & raster tools",
            (66, 153, 225),
        ),
        (
            "Live Filters",
            "Drop Shadow, Glow, Bevel\n& dockable studio panels",
            (237, 137, 54),
        ),
        (
            "Web & Slices",
            "PNG, WebP, SVG, HTML5\n2-Up & 4-Up optimization",
            (72, 187, 120),
        ),
        (
            "Pure Rust Engine",
            "257 commands, .fw.png\nCLI, scripting & WASM",
            (159, 122, 234),
        ),
    ];

    for (i, (heading, body, (r, g, b))) in features.iter().enumerate() {
        let x0 = pad + i as f64 * (card_w + gap);
        add_rect(
            editor,
            x0,
            cards_top,
            card_w,
            cards_h,
            12.0,
            pack_rgb(28, 32, 44),
            pack_rgb(*r, *g, *b),
            &format!("Feature Card: {heading}"),
        );
        add_text(
            editor,
            x0 + 24.0,
            cards_top + 28.0,
            card_w - 48.0,
            40.0,
            heading,
            (card_w * 0.068).clamp(18.0, 26.0),
            true,
            pack_rgb(*r, *g, *b),
            &format!("{heading} Title"),
        );
        add_text(
            editor,
            x0 + 24.0,
            cards_top + 88.0,
            card_w - 48.0,
            96.0,
            body,
            (card_w * 0.048).clamp(14.0, 19.0),
            false,
            pack_rgb(190, 198, 214),
            &format!("{heading} Body"),
        );
    }

    // 6. Web Slice over CTA Button
    let _ = editor.apply(
        "slice.create",
        serde_json::json!({
            "shape": "rect",
            "x": cta_x,
            "y": cta_y,
            "width": cta_w,
            "height": cta_h,
            "points": null,
            "page": page,
            "name": "CTA Web Slice",
            "link": "https://github.com/aliasfoxkde/FireCraft",
            "alt": "FireCraft Studio Showcase",
        }),
    );

    let _ = (layer, cell);
    editor.selection = None;
    editor.selection_set.clear();
}

#[allow(clippy::too_many_arguments)]
fn add_rect(
    editor: &mut Editor,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    roundness: f64,
    fill_raw: i64,
    stroke_raw: i64,
    name: &str,
) {
    let ok = editor
        .apply(
            "object.createPath",
            serde_json::json!({
                "kind": "rect",
                "x": x,
                "y": y,
                "width": width,
                "height": height,
                "page": editor.page,
                "layer": editor.active_layer,
                "cell": editor.frame,
                "name": name,
            }),
        )
        .is_ok();
    if !ok {
        return;
    }
    if let Some(id) = latest_object_id(editor) {
        if roundness > 0.0 {
            let _ = editor.apply(
                "primitive.setRectRoundness",
                serde_json::json!({ "id": id, "roundness": roundness, "mode": "pixels" }),
            );
        }
        let _ = editor.apply(
            "object.setFill",
            serde_json::json!({ "id": id, "color_raw": fill_raw }),
        );
        if stroke_raw != 0 {
            let _ = editor.apply(
                "object.setStroke",
                serde_json::json!({ "id": id, "color_raw": stroke_raw }),
            );
        }
    }
}

fn add_poly(editor: &mut Editor, points: &[[f64; 2]], fill_raw: i64, stroke_raw: i64, name: &str) {
    let ok = editor
        .apply(
            "object.createPolyline",
            serde_json::json!({
                "points": points,
                "closed": true,
                "page": editor.page,
                "layer": editor.active_layer,
                "cell": editor.frame,
                "name": name,
            }),
        )
        .is_ok();
    if !ok {
        return;
    }
    if let Some(id) = latest_object_id(editor) {
        let _ = editor.apply(
            "object.setFill",
            serde_json::json!({ "id": id, "color_raw": fill_raw }),
        );
        if stroke_raw != 0 {
            let _ = editor.apply(
                "object.setStroke",
                serde_json::json!({ "id": id, "color_raw": stroke_raw }),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn add_text(
    editor: &mut Editor,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    text: &str,
    size: f64,
    bold: bool,
    fill_raw: i64,
    name: &str,
) {
    let ok = editor
        .apply(
            "text.createText",
            serde_json::json!({
                "page": editor.page,
                "layer": editor.active_layer,
                "cell": editor.frame,
                "x": x,
                "y": y,
                "width": width,
                "height": height,
                "text": text,
                "name": name,
            }),
        )
        .is_ok();
    if !ok {
        return;
    }
    if let Some(id) = latest_object_id(editor) {
        let _ = editor.apply(
            "text.setAttrs",
            serde_json::json!({
                "id": id,
                "size": size,
                "bold": bold,
                "fill_raw": fill_raw,
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn showcase_creates_cards_flame_text_and_slice() {
        let editor = build_showcase_editor();
        let page = &editor.document().pages[0];
        assert!((page.info.canvas.width - 1920.0).abs() < 0.5);
        assert!((page.info.canvas.height - 1080.0).abs() < 0.5);
        let obj_count = editor.document().registry.len();
        assert!(
            obj_count >= 15,
            "expected rich showcase objects, got {obj_count}"
        );
    }
}
