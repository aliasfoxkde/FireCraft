//! Icon rendering: Lucide SVGs (ISC licence) embedded at compile
//! time, recoloured to the requested ink and rasterized by
//! `egui_extras`' SVG loader at the exact on-screen size — crisp at
//! any DPI, and identical on desktop and wasm.
//!
//! The icon *set* is third-party (Lucide, ISC — see
//! `assets/icons/LICENSE-lucide.txt` and the row in
//! `docs/ATTRIBUTION.md`); the tool→icon mapping is `FireCraft`'s own.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use egui::{Color32, Rect, Vec2};

use crate::editor::Tool;

/// The embedded Lucide set, one entry per vendored file.
static ICONS: &[(&str, &[u8])] = &[
    (
        "arrow-left-right",
        include_bytes!("../../../../assets/icons/arrow-left-right.svg"),
    ),
    (
        "blend",
        include_bytes!("../../../../assets/icons/blend.svg"),
    ),
    (
        "brush",
        include_bytes!("../../../../assets/icons/brush.svg"),
    ),
    (
        "circle-dashed",
        include_bytes!("../../../../assets/icons/circle-dashed.svg"),
    ),
    (
        "circle-dot",
        include_bytes!("../../../../assets/icons/circle-dot.svg"),
    ),
    (
        "circle",
        include_bytes!("../../../../assets/icons/circle.svg"),
    ),
    ("copy", include_bytes!("../../../../assets/icons/copy.svg")),
    (
        "eraser",
        include_bytes!("../../../../assets/icons/eraser.svg"),
    ),
    (
        "eye-off",
        include_bytes!("../../../../assets/icons/eye-off.svg"),
    ),
    ("eye", include_bytes!("../../../../assets/icons/eye.svg")),
    (
        "grid-3x3",
        include_bytes!("../../../../assets/icons/grid-3x3.svg"),
    ),
    ("hand", include_bytes!("../../../../assets/icons/hand.svg")),
    (
        "image",
        include_bytes!("../../../../assets/icons/image.svg"),
    ),
    (
        "lasso-select",
        include_bytes!("../../../../assets/icons/lasso-select.svg"),
    ),
    (
        "lasso",
        include_bytes!("../../../../assets/icons/lasso.svg"),
    ),
    (
        "layers",
        include_bytes!("../../../../assets/icons/layers.svg"),
    ),
    (
        "lock-open",
        include_bytes!("../../../../assets/icons/lock-open.svg"),
    ),
    ("lock", include_bytes!("../../../../assets/icons/lock.svg")),
    (
        "minus",
        include_bytes!("../../../../assets/icons/minus.svg"),
    ),
    (
        "mouse-pointer-2",
        include_bytes!("../../../../assets/icons/mouse-pointer-2.svg"),
    ),
    ("move", include_bytes!("../../../../assets/icons/move.svg")),
    (
        "paint-bucket",
        include_bytes!("../../../../assets/icons/paint-bucket.svg"),
    ),
    (
        "pencil",
        include_bytes!("../../../../assets/icons/pencil.svg"),
    ),
    (
        "pentagon",
        include_bytes!("../../../../assets/icons/pentagon.svg"),
    ),
    (
        "pipette",
        include_bytes!("../../../../assets/icons/pipette.svg"),
    ),
    ("plus", include_bytes!("../../../../assets/icons/plus.svg")),
    (
        "pointer",
        include_bytes!("../../../../assets/icons/pointer.svg"),
    ),
    (
        "rectangle-horizontal",
        include_bytes!("../../../../assets/icons/rectangle-horizontal.svg"),
    ),
    (
        "rectangle-vertical",
        include_bytes!("../../../../assets/icons/rectangle-vertical.svg"),
    ),
    (
        "redo-2",
        include_bytes!("../../../../assets/icons/redo-2.svg"),
    ),
    (
        "ruler",
        include_bytes!("../../../../assets/icons/ruler.svg"),
    ),
    (
        "scissors",
        include_bytes!("../../../../assets/icons/scissors.svg"),
    ),
    (
        "search",
        include_bytes!("../../../../assets/icons/search.svg"),
    ),
    (
        "slice-knife",
        include_bytes!("../../../../assets/icons/slice-knife.svg"),
    ),
    (
        "sparkles",
        include_bytes!("../../../../assets/icons/sparkles.svg"),
    ),
    (
        "spline",
        include_bytes!("../../../../assets/icons/spline.svg"),
    ),
    (
        "square-dashed-mouse-pointer",
        include_bytes!("../../../../assets/icons/square-dashed-mouse-pointer.svg"),
    ),
    (
        "square-dashed",
        include_bytes!("../../../../assets/icons/square-dashed.svg"),
    ),
    (
        "square",
        include_bytes!("../../../../assets/icons/square.svg"),
    ),
    (
        "trash",
        include_bytes!("../../../../assets/icons/trash.svg"),
    ),
    ("type", include_bytes!("../../../../assets/icons/type.svg")),
    (
        "undo-2",
        include_bytes!("../../../../assets/icons/undo-2.svg"),
    ),
    (
        "wand-sparkles",
        include_bytes!("../../../../assets/icons/wand-sparkles.svg"),
    ),
    (
        "zoom-in",
        include_bytes!("../../../../assets/icons/zoom-in.svg"),
    ),
];

/// The icon bytes with `currentColor` swapped for white, so a single
/// white copy tints to any ink at paint time.
fn white_icons() -> &'static HashMap<&'static str, Arc<[u8]>> {
    static MAP: OnceLock<HashMap<&'static str, Arc<[u8]>>> = OnceLock::new();
    MAP.get_or_init(|| {
        ICONS
            .iter()
            .map(|(name, bytes)| {
                let svg = String::from_utf8_lossy(bytes)
                    .replace("currentColor", "#ffffff")
                    .replace("stroke-width=\"2\"", "stroke-width=\"1.75\"");
                (*name, Arc::from(svg.into_bytes().into_boxed_slice()))
            })
            .collect()
    })
}

/// Whether the named icon is in the embedded set.
#[cfg(test)]
fn exists(name: &str) -> bool {
    white_icons().contains_key(name)
}

/// The egui image for an icon, tinted. An unknown name falls back to
/// a square rather than painting nothing.
pub(crate) fn image(name: &str, size: f32, tint: Color32) -> egui::Image<'static> {
    let bytes = white_icons()
        .get(name)
        .or_else(|| white_icons().get("square"))
        .cloned()
        .unwrap_or_default();
    egui::Image::from_bytes(
        format!("bytes://icons/{name}.svg"),
        egui::load::Bytes::Shared(bytes),
    )
    .fit_to_exact_size(Vec2::splat(size))
    .tint(tint)
}

/// Paints an icon centred in `rect`.
pub(crate) fn paint(ui: &egui::Ui, rect: Rect, name: &str, size: f32, tint: Color32) {
    let r = Rect::from_center_size(rect.center(), Vec2::splat(size));
    image(name, size, tint).paint_at(ui, r);
}

/// Every tool variant, for the mapping test.
#[cfg(test)]
const TOOLS: &[Tool] = &[
    Tool::Pointer,
    Tool::Subselect,
    Tool::Rect,
    Tool::Ellipse,
    Tool::Line,
    Tool::Text,
    Tool::Zoom,
    Tool::Hand,
    Tool::Slice,
    Tool::Hotspot,
    Tool::AutoShape(crate::editor::AutoShape::Star),
    Tool::Paintbrush,
    Tool::Pencil,
    Tool::Eraser,
    Tool::PaintBucket,
    Tool::Gradient,
    Tool::Eyedropper,
    Tool::Marquee,
    Tool::EllipticalMarquee,
    Tool::Lasso,
    Tool::PolygonalMarquee,
    Tool::MagicWand,
];

/// The Lucide icon for a tool.
pub(crate) fn tool_icon(tool: Tool) -> &'static str {
    match tool {
        Tool::Pointer => "pointer",
        Tool::Subselect => "mouse-pointer-2",
        Tool::Rect => "square",
        Tool::Ellipse => "circle",
        Tool::Line => "minus",
        Tool::Text => "type",
        Tool::Zoom => "zoom-in",
        Tool::Hand => "hand",
        Tool::Slice => "slice-knife",
        Tool::Hotspot => "circle-dot",
        Tool::AutoShape(_) => "sparkles",
        Tool::Paintbrush => "brush",
        Tool::Pencil => "pencil",
        Tool::Eraser => "eraser",
        Tool::PaintBucket => "paint-bucket",
        Tool::Gradient => "blend",
        Tool::Eyedropper => "pipette",
        Tool::Marquee => "square-dashed",
        Tool::EllipticalMarquee => "circle-dashed",
        Tool::Lasso => "lasso",
        Tool::PolygonalMarquee => "pentagon",
        Tool::MagicWand => "wand-sparkles",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every tool maps to an icon that is actually embedded.
    #[test]
    fn every_tool_has_an_icon() {
        for tool in TOOLS {
            let name = tool_icon(*tool);
            assert!(exists(name), "{tool:?} → {name}");
        }
    }

    /// The Lucide files carry `currentColor` strokes, so recolouring
    /// to white is what makes the tint work.
    #[test]
    fn the_icons_are_recoloured_to_white() {
        let map = white_icons();
        assert!(map.len() >= 40);
        for (name, bytes) in map {
            let svg = String::from_utf8_lossy(bytes);
            assert!(!svg.contains("currentColor"), "{name} kept currentColor");
        }
    }
}
