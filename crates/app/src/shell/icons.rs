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
        "aperture",
        include_bytes!("../../../../assets/icons/aperture.svg"),
    ),
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
        "circle",
        include_bytes!("../../../../assets/icons/circle.svg"),
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
        "contrast",
        include_bytes!("../../../../assets/icons/contrast.svg"),
    ),
    ("copy", include_bytes!("../../../../assets/icons/copy.svg")),
    ("crop", include_bytes!("../../../../assets/icons/crop.svg")),
    (
        "crosshair",
        include_bytes!("../../../../assets/icons/crosshair.svg"),
    ),
    (
        "droplet",
        include_bytes!("../../../../assets/icons/droplet.svg"),
    ),
    (
        "eraser",
        include_bytes!("../../../../assets/icons/eraser.svg"),
    ),
    ("eye", include_bytes!("../../../../assets/icons/eye.svg")),
    (
        "eye-off",
        include_bytes!("../../../../assets/icons/eye-off.svg"),
    ),
    (
        "feather",
        include_bytes!("../../../../assets/icons/feather.svg"),
    ),
    (
        "flame",
        include_bytes!("../../../../assets/icons/flame.svg"),
    ),
    (
        "git-commit-horizontal",
        include_bytes!("../../../../assets/icons/git-commit-horizontal.svg"),
    ),
    (
        "grid-3x3",
        include_bytes!("../../../../assets/icons/grid-3x3.svg"),
    ),
    ("hand", include_bytes!("../../../../assets/icons/hand.svg")),
    (
        "hexagon",
        include_bytes!("../../../../assets/icons/hexagon.svg"),
    ),
    (
        "image",
        include_bytes!("../../../../assets/icons/image.svg"),
    ),
    (
        "lasso",
        include_bytes!("../../../../assets/icons/lasso.svg"),
    ),
    (
        "lasso-select",
        include_bytes!("../../../../assets/icons/lasso-select.svg"),
    ),
    (
        "layers",
        include_bytes!("../../../../assets/icons/layers.svg"),
    ),
    ("lock", include_bytes!("../../../../assets/icons/lock.svg")),
    (
        "lock-open",
        include_bytes!("../../../../assets/icons/lock-open.svg"),
    ),
    (
        "magnet",
        include_bytes!("../../../../assets/icons/magnet.svg"),
    ),
    (
        "minus",
        include_bytes!("../../../../assets/icons/minus.svg"),
    ),
    (
        "monitor",
        include_bytes!("../../../../assets/icons/monitor.svg"),
    ),
    (
        "mouse-pointer-2",
        include_bytes!("../../../../assets/icons/mouse-pointer-2.svg"),
    ),
    ("move", include_bytes!("../../../../assets/icons/move.svg")),
    (
        "orbit",
        include_bytes!("../../../../assets/icons/orbit.svg"),
    ),
    (
        "paint-bucket",
        include_bytes!("../../../../assets/icons/paint-bucket.svg"),
    ),
    (
        "paintbrush",
        include_bytes!("../../../../assets/icons/paintbrush.svg"),
    ),
    (
        "palette",
        include_bytes!("../../../../assets/icons/palette.svg"),
    ),
    (
        "pen-tool",
        include_bytes!("../../../../assets/icons/pen-tool.svg"),
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
    (
        "pizza",
        include_bytes!("../../../../assets/icons/pizza.svg"),
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
        "refresh-cw",
        include_bytes!("../../../../assets/icons/refresh-cw.svg"),
    ),
    (
        "replace",
        include_bytes!("../../../../assets/icons/replace.svg"),
    ),
    (
        "route",
        include_bytes!("../../../../assets/icons/route.svg"),
    ),
    (
        "ruler",
        include_bytes!("../../../../assets/icons/ruler.svg"),
    ),
    (
        "scale",
        include_bytes!("../../../../assets/icons/scale.svg"),
    ),
    ("scan", include_bytes!("../../../../assets/icons/scan.svg")),
    (
        "scan-search",
        include_bytes!("../../../../assets/icons/scan-search.svg"),
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
        "shield",
        include_bytes!("../../../../assets/icons/shield.svg"),
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
        "square",
        include_bytes!("../../../../assets/icons/square.svg"),
    ),
    (
        "square-dashed",
        include_bytes!("../../../../assets/icons/square-dashed.svg"),
    ),
    (
        "square-dashed-mouse-pointer",
        include_bytes!("../../../../assets/icons/square-dashed-mouse-pointer.svg"),
    ),
    (
        "stamp",
        include_bytes!("../../../../assets/icons/stamp.svg"),
    ),
    ("sun", include_bytes!("../../../../assets/icons/sun.svg")),
    (
        "sun-dim",
        include_bytes!("../../../../assets/icons/sun-dim.svg"),
    ),
    (
        "trash",
        include_bytes!("../../../../assets/icons/trash.svg"),
    ),
    (
        "triangle",
        include_bytes!("../../../../assets/icons/triangle.svg"),
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
        "waypoints",
        include_bytes!("../../../../assets/icons/waypoints.svg"),
    ),
    (
        "wrench",
        include_bytes!("../../../../assets/icons/wrench.svg"),
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
    Tool::Scale,
    Tool::Skew,
    Tool::Distort,
    Tool::Crop,
    Tool::Marquee,
    Tool::EllipticalMarquee,
    Tool::Lasso,
    Tool::PolygonalMarquee,
    Tool::MagicWand,
    Tool::SelectSimilar,
    Tool::Paintbrush,
    Tool::Pencil,
    Tool::Eraser,
    Tool::Blur,
    Tool::Sharpen,
    Tool::Smudge,
    Tool::Dodge,
    Tool::Burn,
    Tool::RubberStamp,
    Tool::RedEyeRemoval,
    Tool::ReplaceColor,
    Tool::Line,
    Tool::Pen,
    Tool::VectorPath,
    Tool::RedrawPath,
    Tool::Rect,
    Tool::Ellipse,
    Tool::Text,
    Tool::Freeform,
    Tool::ReshapeArea,
    Tool::Knife,
    Tool::Slice,
    Tool::PolygonSlice,
    Tool::Hotspot,
    Tool::CircleHotspot,
    Tool::PolygonHotspot,
    Tool::HideShowSlices,
    Tool::Eyedropper,
    Tool::PaintBucket,
    Tool::Gradient,
    Tool::DefaultStrokeFill,
    Tool::SwapStrokeFill,
    Tool::NoStrokeFill,
    Tool::ToggleScreenMode,
    Tool::Hand,
    Tool::Zoom,
    Tool::AutoShape(crate::editor::AutoShape::Star),
];

/// The Lucide icon for a tool. The mapping is `FireCraft`'s own — the
/// set is Lucide's, but no icon is Adobe's.
pub(crate) fn tool_icon(tool: Tool) -> &'static str {
    match tool {
        // Select
        Tool::Pointer => "pointer",
        Tool::Subselect => "mouse-pointer-2",
        Tool::Scale => "scale",
        Tool::Skew => "triangle",
        Tool::Distort => "waypoints",
        Tool::Crop => "crop",
        // Bitmap: selection
        Tool::Marquee => "square-dashed",
        Tool::EllipticalMarquee => "circle-dashed",
        Tool::Lasso => "lasso",
        Tool::PolygonalMarquee => "lasso-select",
        Tool::MagicWand => "wand-sparkles",
        Tool::SelectSimilar => "scan-search",
        // Bitmap: paint
        Tool::Paintbrush => "paintbrush",
        Tool::Pencil => "pencil",
        Tool::Eraser => "eraser",
        // Bitmap: retouch
        Tool::Blur => "droplet",
        Tool::Sharpen => "aperture",
        Tool::Smudge => "feather",
        Tool::Dodge => "sun",
        Tool::Burn => "flame",
        Tool::RubberStamp => "stamp",
        Tool::RedEyeRemoval => "crosshair",
        Tool::ReplaceColor => "replace",
        // Vector
        Tool::Line => "minus",
        Tool::Pen => "pen-tool",
        Tool::VectorPath => "route",
        Tool::RedrawPath => "spline",
        Tool::Rect => "square",
        Tool::Ellipse => "circle",
        Tool::Text => "type",
        Tool::Freeform => "orbit",
        Tool::ReshapeArea => "refresh-cw",
        Tool::Knife => "slice-knife",
        Tool::AutoShape(_) => "sparkles",
        // Web
        Tool::Slice => "scissors",
        Tool::PolygonSlice => "pizza",
        Tool::Hotspot => "square-dashed-mouse-pointer",
        Tool::CircleHotspot => "circle-dot",
        Tool::PolygonHotspot => "hexagon",
        Tool::HideShowSlices => "eye",
        // Colors
        Tool::Eyedropper => "pipette",
        Tool::PaintBucket => "paint-bucket",
        Tool::Gradient => "blend",
        Tool::DefaultStrokeFill => "palette",
        Tool::SwapStrokeFill => "arrow-left-right",
        Tool::NoStrokeFill => "eye-off",
        // View
        Tool::ToggleScreenMode => "monitor",
        Tool::Hand => "hand",
        Tool::Zoom => "zoom-in",
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
        assert_eq!(map.len(), ICONS.len());
        assert!(map.len() >= 73);
        for (name, bytes) in map {
            let svg = String::from_utf8_lossy(bytes);
            assert!(!svg.contains("currentColor"), "{name} kept currentColor");
        }
    }

    /// Every tool the CS6 strip shows has an embedded icon — the
    /// strip is the surface the icons are for.
    #[test]
    fn every_strip_tool_has_an_embedded_icon() {
        for category in crate::toolstrip::CATEGORIES {
            for group in category.groups {
                for entry in group.tools {
                    let name = tool_icon(entry.tool);
                    assert!(exists(name), "{:?} → {name}", entry.tool);
                }
            }
        }
    }
}
