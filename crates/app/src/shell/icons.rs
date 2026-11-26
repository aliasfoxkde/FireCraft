//! Icon rendering: Phosphor Icons (MIT licence) embedded at compile
//! time, recoloured to the requested ink and rasterized by
//! `egui`'s SVG loader at the exact on-screen size — crisp at
//! any DPI, and identical on desktop and wasm.

//! The icon *set* is third-party (Phosphor, MIT — see
//! `assets/icons/LICENSE-phosphor.txt` and the row in
//! `docs/ATTRIBUTION.md`); the tool->icon mapping is `FireCraft`'s own.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use egui::{Color32, Rect, Vec2};

use crate::editor::Tool;

/// The embedded Phosphor set, one entry per vendored file.
static ICONS: &[(&str, &[u8])] = &[
    (
        "auto-shape",
        include_bytes!("../../../../assets/icons-phosphor/auto-shape.svg"),
    ),
    (
        "autoshapecircle",
        include_bytes!("../../../../assets/icons-phosphor/autoshapecircle.svg"),
    ),
    (
        "autoshapethex",
        include_bytes!("../../../../assets/icons-phosphor/autoshapethex.svg"),
    ),
    (
        "blur-2",
        include_bytes!("../../../../assets/icons-phosphor/blur-2.svg"),
    ),
    (
        "blur",
        include_bytes!("../../../../assets/icons-phosphor/blur.svg"),
    ),
    (
        "brush",
        include_bytes!("../../../../assets/icons-phosphor/brush.svg"),
    ),
    (
        "burn-2",
        include_bytes!("../../../../assets/icons-phosphor/burn-2.svg"),
    ),
    (
        "burn",
        include_bytes!("../../../../assets/icons-phosphor/burn.svg"),
    ),
    (
        "crop",
        include_bytes!("../../../../assets/icons-phosphor/crop.svg"),
    ),
    (
        "default-stroke-fill",
        include_bytes!("../../../../assets/icons-phosphor/default-stroke-fill.svg"),
    ),
    (
        "distort",
        include_bytes!("../../../../assets/icons-phosphor/distort.svg"),
    ),
    (
        "dodge-2",
        include_bytes!("../../../../assets/icons-phosphor/dodge-2.svg"),
    ),
    (
        "dodge",
        include_bytes!("../../../../assets/icons-phosphor/dodge.svg"),
    ),
    (
        "ellipse",
        include_bytes!("../../../../assets/icons-phosphor/ellipse.svg"),
    ),
    (
        "eraser",
        include_bytes!("../../../../assets/icons-phosphor/eraser.svg"),
    ),
    (
        "eyedropper",
        include_bytes!("../../../../assets/icons-phosphor/eyedropper.svg"),
    ),
    (
        "fill",
        include_bytes!("../../../../assets/icons-phosphor/fill.svg"),
    ),
    (
        "fit-in-window",
        include_bytes!("../../../../assets/icons-phosphor/fit-in-window.svg"),
    ),
    (
        "fit",
        include_bytes!("../../../../assets/icons-phosphor/fit.svg"),
    ),
    (
        "gradient",
        include_bytes!("../../../../assets/icons-phosphor/gradient.svg"),
    ),
    (
        "hand",
        include_bytes!("../../../../assets/icons-phosphor/hand.svg"),
    ),
    (
        "hotspot",
        include_bytes!("../../../../assets/icons-phosphor/hotspot.svg"),
    ),
    (
        "knife-2",
        include_bytes!("../../../../assets/icons-phosphor/knife-2.svg"),
    ),
    (
        "knife",
        include_bytes!("../../../../assets/icons-phosphor/knife.svg"),
    ),
    (
        "lasso",
        include_bytes!("../../../../assets/icons-phosphor/lasso.svg"),
    ),
    (
        "line",
        include_bytes!("../../../../assets/icons-phosphor/line.svg"),
    ),
    (
        "marquee",
        include_bytes!("../../../../assets/icons-phosphor/marquee.svg"),
    ),
    (
        "no-stroke-fill",
        include_bytes!("../../../../assets/icons-phosphor/no-stroke-fill.svg"),
    ),
    (
        "paint-bucket",
        include_bytes!("../../../../assets/icons-phosphor/paint-bucket.svg"),
    ),
    (
        "pen-2",
        include_bytes!("../../../../assets/icons-phosphor/pen-2.svg"),
    ),
    (
        "pen",
        include_bytes!("../../../../assets/icons-phosphor/pen.svg"),
    ),
    (
        "pencil-2",
        include_bytes!("../../../../assets/icons-phosphor/pencil-2.svg"),
    ),
    (
        "pencil",
        include_bytes!("../../../../assets/icons-phosphor/pencil.svg"),
    ),
    (
        "picker",
        include_bytes!("../../../../assets/icons-phosphor/picker.svg"),
    ),
    (
        "pointer",
        include_bytes!("../../../../assets/icons-phosphor/pointer.svg"),
    ),
    (
        "polygon",
        include_bytes!("../../../../assets/icons-phosphor/polygon.svg"),
    ),
    (
        "rect",
        include_bytes!("../../../../assets/icons-phosphor/rect.svg"),
    ),
    (
        "red-eye-remove",
        include_bytes!("../../../../assets/icons-phosphor/red-eye-remove.svg"),
    ),
    (
        "red-eye",
        include_bytes!("../../../../assets/icons-phosphor/red-eye.svg"),
    ),
    (
        "redraw-path",
        include_bytes!("../../../../assets/icons-phosphor/redraw-path.svg"),
    ),
    (
        "replace-color-2",
        include_bytes!("../../../../assets/icons-phosphor/replace-color-2.svg"),
    ),
    (
        "replace-color",
        include_bytes!("../../../../assets/icons-phosphor/replace-color.svg"),
    ),
    (
        "rubber-stamp",
        include_bytes!("../../../../assets/icons-phosphor/rubber-stamp.svg"),
    ),
    (
        "scale",
        include_bytes!("../../../../assets/icons-phosphor/scale.svg"),
    ),
    (
        "select",
        include_bytes!("../../../../assets/icons-phosphor/select.svg"),
    ),
    (
        "sharpen",
        include_bytes!("../../../../assets/icons-phosphor/sharpen.svg"),
    ),
    (
        "skew",
        include_bytes!("../../../../assets/icons-phosphor/skew.svg"),
    ),
    (
        "slice",
        include_bytes!("../../../../assets/icons-phosphor/slice.svg"),
    ),
    (
        "smudge",
        include_bytes!("../../../../assets/icons-phosphor/smudge.svg"),
    ),
    (
        "stamp",
        include_bytes!("../../../../assets/icons-phosphor/stamp.svg"),
    ),
    (
        "subselect",
        include_bytes!("../../../../assets/icons-phosphor/subselect.svg"),
    ),
    (
        "swap-stroke-fill",
        include_bytes!("../../../../assets/icons-phosphor/swap-stroke-fill.svg"),
    ),
    (
        "toggle-screen-mode",
        include_bytes!("../../../../assets/icons-phosphor/toggle-screen-mode.svg"),
    ),
    (
        "type",
        include_bytes!("../../../../assets/icons-phosphor/type.svg"),
    ),
    (
        "vector-path",
        include_bytes!("../../../../assets/icons-phosphor/vector-path.svg"),
    ),
    (
        "wand-sparkles",
        include_bytes!("../../../../assets/icons-phosphor/wand-sparkles.svg"),
    ),
    (
        "zoom",
        include_bytes!("../../../../assets/icons-phosphor/zoom.svg"),
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
                let svg = String::from_utf8_lossy(bytes).replace("currentColor", "#ffffff");
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
        .or_else(|| white_icons().get("auto-shape"))
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

/// The Phosphor icon for a tool. The mapping is `FireCraft`'s own —
/// the set is Phosphor's, but no icon is Adobe's.
pub(crate) fn tool_icon(tool: Tool) -> &'static str {
    match tool {
        // Select
        Tool::Pointer => "select",
        Tool::Subselect => "subselect",
        Tool::Scale => "scale",
        Tool::Skew => "skew",
        Tool::Distort => "distort",
        Tool::Crop => "crop",
        // Bitmap: selection
        Tool::Marquee => "marquee",
        Tool::EllipticalMarquee => "lasso",
        Tool::Lasso => "lasso",
        Tool::PolygonalMarquee => "lasso",
        Tool::MagicWand => "wand-sparkles",
        Tool::SelectSimilar => "brush",
        // Bitmap: paint
        Tool::Paintbrush => "brush",
        Tool::Pencil => "pencil",
        Tool::Eraser => "eraser",
        // Bitmap: retouch
        Tool::Blur => "blur",
        Tool::Sharpen => "sharpen",
        Tool::Smudge => "smudge",
        Tool::Dodge => "dodge",
        Tool::Burn => "burn",
        Tool::RubberStamp => "stamp",
        Tool::RedEyeRemoval => "red-eye-remove",
        Tool::ReplaceColor => "replace-color",
        // Vector
        Tool::Line => "line",
        Tool::Pen => "pen",
        Tool::VectorPath => "vector-path",
        Tool::RedrawPath => "redraw-path",
        Tool::Rect => "rect",
        Tool::Ellipse => "ellipse",
        Tool::Text => "type",
        Tool::Freeform => "brush",
        Tool::ReshapeArea => "redraw-path",
        Tool::Knife => "knife",
        Tool::AutoShape(_) => "auto-shape",
        // Web
        Tool::Slice => "knife",
        Tool::PolygonSlice => "knife",
        Tool::Hotspot => "hotspot",
        Tool::CircleHotspot => "hotspot",
        Tool::PolygonHotspot => "hotspot",
        Tool::HideShowSlices => "brush",
        // Colors
        Tool::Eyedropper => "eyedropper",
        Tool::PaintBucket => "fill",
        Tool::Gradient => "gradient",
        Tool::DefaultStrokeFill => "line",
        Tool::SwapStrokeFill => "replace-color",
        Tool::NoStrokeFill => "line",
        // View
        Tool::ToggleScreenMode => "toggle-screen-mode",
        Tool::Hand => "hand",
        Tool::Zoom => "zoom",
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
            assert!(exists(name), "{tool:?} -> {name}");
        }
    }

    /// The Phosphor icons carry `currentColor` strokes, so recolouring
    /// to white is what makes the tint work.
    #[test]
    fn the_icons_are_recoloured_to_white() {
        let map = white_icons();
        assert_eq!(map.len(), ICONS.len());
        assert!(map.len() >= 50);
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
                    assert!(exists(name), "{:?} -> {name}", entry.tool);
                }
            }
        }
    }
}
