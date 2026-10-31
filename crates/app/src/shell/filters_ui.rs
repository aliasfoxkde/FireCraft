//! The Live Filters preset table: CS6 family/preset names, `DCVs`
//! defaults, and the documented parameter metadata.

/// One quick-add preset of the Live Filters panel: the engine's
/// internal name — the registry id the Filters menu and this panel
/// share — the CS6 display label, and the `DCVs` defaults that give
/// each kernel its working shape (visible where a visible default
/// exists; `levels` ships its identity master so the rows are
/// editable from a known state, and the shadow family ships the CS6
/// defaults — the same four scalars its Inner Shadow preset
/// documents — because the kernel reads all four or renders nothing).
#[derive(Clone, Copy)]
pub(crate) struct FilterPreset {
    /// Engine internal name (`filter.add`'s `name`).
    pub name: &'static str,
    /// CS6 display label.
    pub label: &'static str,
    /// `DCVs` defaults: (key, decimal-text value).
    pub params: &'static [(&'static str, &'static str)],
    /// Curve knots for the `*_points` param — the engine's raw
    /// `GPLv` encoding; `value` must stay empty when these ride along.
    pub points: &'static [(f64, f64)],
}

/// The panel's quick-add registry, grouped the way the enumeration
/// table groups it (`docs/research/fw-code-notes.md`, M14 task 1).
/// CS6 builds its Filters menu at runtime from the same registry —
/// the static menu resource carries only Repeat Filter — which is
/// exactly the mirror `menus::FILTERS` casts.
pub(crate) static FILTER_FAMILIES: &[(&str, &[FilterPreset])] = &[
    (
        "Adjust Color",
        &[
            FilterPreset {
                name: "invert",
                label: "Invert",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "brightnessContrast",
                label: "Brightness/Contrast",
                params: &[("brightness_amount", "10"), ("contrast_amount", "10")],
                points: &[],
            },
            FilterPreset {
                name: "levels",
                label: "Levels",
                params: &[
                    ("source_low_rgb", "0"),
                    ("source_high_rgb", "255"),
                    ("gamma_rgb", "1"),
                    ("dest_low_rgb", "0"),
                    ("dest_high_rgb", "255"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "curves",
                label: "Curves",
                params: &[("rgb_points", "")],
                points: &[(0.0, 0.0), (128.0, 64.0), (255.0, 255.0)],
            },
            FilterPreset {
                name: "hueSaturation",
                label: "Hue/Saturation",
                params: &[
                    ("hue_amount", "15"),
                    ("saturation_amount", "10"),
                    ("lightness_amount", "0"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "autoLevels",
                label: "Auto Levels",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "colorFill",
                label: "Color Fill",
                params: &[("Color", "#336699"), ("Opacity", "100"), ("Blendmode", "0")],
                points: &[],
            },
            FilterPreset {
                name: "convertToAlpha",
                label: "Convert to Alpha",
                params: &[],
                points: &[],
            },
        ],
    ),
    (
        "Blur",
        &[
            FilterPreset {
                name: "blur",
                label: "Blur",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "blurMore",
                label: "Blur More",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "gaussianBlur",
                label: "Gaussian Blur",
                params: &[("gaussian_blur_radius", "2")],
                points: &[],
            },
            FilterPreset {
                name: "motionBlur",
                label: "Motion Blur",
                params: &[("motion_blur_angle", "0"), ("motion_blur_distance", "4")],
                points: &[],
            },
            FilterPreset {
                name: "radialBlur",
                label: "Radial Blur",
                params: &[("radial_blur_amount", "10"), ("radial_blur_quality", "3")],
                points: &[],
            },
            FilterPreset {
                name: "zoomBlur",
                label: "Zoom Blur",
                params: &[("zoom_blur_amount", "10"), ("zoom_blur_quality", "3")],
                points: &[],
            },
        ],
    ),
    (
        "Sharpen",
        &[
            FilterPreset {
                name: "sharpen",
                label: "Sharpen",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "sharpenMore",
                label: "Sharpen More",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "unsharpMask",
                label: "Unsharp Mask",
                params: &[
                    ("unsharp_mask_radius", "2"),
                    ("unsharp_mask_amount", "50"),
                    ("unsharp_mask_threshold", "0"),
                ],
                points: &[],
            },
        ],
    ),
    (
        "Stylize",
        &[
            FilterPreset {
                name: "addNoise",
                label: "Add Noise",
                params: &[("add_noise_amount", "10"), ("add_noise_use_color", "0")],
                points: &[],
            },
            FilterPreset {
                name: "findEdges",
                label: "Find Edges",
                params: &[],
                points: &[],
            },
            FilterPreset {
                name: "clouds",
                label: "Clouds",
                params: &[("sky_color", "#0000FF"), ("cloud_color", "#FFFFFF")],
                points: &[],
            },
            FilterPreset {
                name: "differenceClouds",
                label: "Difference Clouds",
                params: &[("sky_color", "#0000FF"), ("cloud_color", "#FFFFFF")],
                points: &[],
            },
        ],
    ),
    (
        "Shadow and Glow",
        &[
            FilterPreset {
                name: "dropShadow",
                label: "Drop Shadow",
                params: &[
                    ("ShadowColor", "#000000"),
                    ("ShadowAngle", "135"),
                    ("ShadowBlur", "2"),
                    ("ShadowDistance", "3"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "innerShadow",
                label: "Inner Shadow",
                params: &[
                    ("ShadowColor", "#000000"),
                    ("ShadowAngle", "135"),
                    ("ShadowBlur", "2"),
                    ("ShadowDistance", "3"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "solidShadow",
                label: "Solid Shadow",
                params: &[("ShadowColor", "#804000"), ("ShowObject", "1")],
                points: &[],
            },
            FilterPreset {
                name: "innerGlow",
                label: "Inner Glow",
                params: &[("ShadowColor", "#ffcc00"), ("GlowWidth", "3")],
                points: &[],
            },
            FilterPreset {
                name: "glow",
                label: "Glow",
                params: &[
                    ("BevelType", "4"),
                    ("GlowWidth", "5"),
                    ("LightAngle", "120"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#804000"),
                ],
                points: &[],
            },
        ],
    ),
    (
        "Bevel and Emboss",
        &[
            FilterPreset {
                name: "innerBevel",
                label: "Inner Bevel",
                params: &[
                    ("BevelType", "0"),
                    ("BevelWidth", "6"),
                    ("BevelContrast", "70"),
                    ("LightAngle", "120"),
                    ("LightDistance", "150"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#808000"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "outerBevel",
                label: "Outer Bevel",
                params: &[
                    ("BevelType", "1"),
                    ("BevelWidth", "6"),
                    ("BevelContrast", "70"),
                    ("LightAngle", "120"),
                    ("LightDistance", "150"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#808000"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "emboss",
                label: "Raised Emboss",
                params: &[
                    ("BevelType", "2"),
                    ("BevelWidth", "6"),
                    ("BevelContrast", "70"),
                    ("LightAngle", "120"),
                    ("LightDistance", "150"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#808000"),
                ],
                points: &[],
            },
            FilterPreset {
                name: "pillowEmboss",
                label: "Inset Emboss",
                params: &[
                    ("BevelType", "3"),
                    ("BevelWidth", "6"),
                    ("BevelContrast", "70"),
                    ("LightAngle", "120"),
                    ("LightDistance", "150"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#808000"),
                ],
                points: &[],
            },
        ],
    ),
];

/// Numeric edit-row metadata for applied effects: the documented
/// range (the enumeration table) plus the neutral value the row's
/// reset button restores. Keys absent here render as text rows; the
/// boolean keys render as checkboxes and `*_points` as knot lists.
pub(crate) struct FilterParamMeta {
    pub(super) key: &'static str,
    /// Row label.
    pub(super) label: &'static str,
    pub(super) min: f64,
    pub(super) max: f64,
    /// The reset value (the no-op where one exists).
    pub(super) default: f64,
}

pub(crate) const FILTER_PARAMS: &[FilterParamMeta] = &[
    FilterParamMeta {
        key: "brightness_amount",
        label: "Brightness",
        min: -100.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "contrast_amount",
        label: "Contrast",
        min: -100.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "source_low_rgb",
        label: "In black",
        min: 0.0,
        max: 255.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "source_high_rgb",
        label: "In white",
        min: 0.0,
        max: 255.0,
        default: 255.0,
    },
    FilterParamMeta {
        key: "gamma_rgb",
        label: "Gamma",
        min: 0.1,
        max: 10.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "dest_low_rgb",
        label: "Out black",
        min: 0.0,
        max: 255.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "dest_high_rgb",
        label: "Out white",
        min: 0.0,
        max: 255.0,
        default: 255.0,
    },
    // In colorize mode the hue is absolute 0..360 — the row clamps to
    // the shift range; flipping the mode is one setParam away.
    FilterParamMeta {
        key: "hue_amount",
        label: "Hue",
        min: -180.0,
        max: 180.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "saturation_amount",
        label: "Saturation",
        min: -100.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "lightness_amount",
        label: "Lightness",
        min: -100.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "gaussian_blur_radius",
        label: "Radius",
        min: 0.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "motion_blur_angle",
        label: "Angle",
        min: 0.0,
        max: 360.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "motion_blur_distance",
        label: "Distance",
        min: 1.0,
        max: 100.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "radial_blur_amount",
        label: "Amount",
        min: 1.0,
        max: 100.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "radial_blur_quality",
        label: "Quality",
        min: 1.0,
        max: 100.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "zoom_blur_amount",
        label: "Amount",
        min: 1.0,
        max: 100.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "zoom_blur_quality",
        label: "Quality",
        min: 1.0,
        max: 100.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "unsharp_mask_radius",
        label: "Radius",
        min: 0.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "unsharp_mask_amount",
        label: "Amount",
        min: 0.0,
        max: 200.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "unsharp_mask_threshold",
        label: "Threshold",
        min: 0.0,
        max: 255.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "add_noise_amount",
        label: "Amount",
        min: 0.0,
        max: 100.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "Opacity",
        label: "Opacity",
        min: 0.0,
        max: 100.0,
        default: 100.0,
    },
    FilterParamMeta {
        key: "ShadowAngle",
        label: "Angle",
        min: 0.0,
        max: 360.0,
        default: 135.0,
    },
    FilterParamMeta {
        key: "ShadowBlur",
        label: "Blur",
        min: 0.0,
        max: 250.0,
        default: 2.0,
    },
    FilterParamMeta {
        key: "ShadowDistance",
        label: "Distance",
        min: 0.0,
        max: 250.0,
        default: 3.0,
    },
    FilterParamMeta {
        key: "GlowWidth",
        label: "Width",
        min: 0.0,
        max: 35.0,
        default: 3.0,
    },
    FilterParamMeta {
        key: "GlowStartDistance",
        label: "Start",
        min: 0.0,
        max: 35.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "EdgeThreshold",
        label: "Edge threshold",
        min: 0.0,
        max: 1.0,
        default: 1.0,
    },
    FilterParamMeta {
        key: "BevelType",
        label: "Type",
        min: 0.0,
        max: 4.0,
        default: 0.0,
    },
    FilterParamMeta {
        key: "BevelWidth",
        label: "Width",
        min: 0.0,
        max: 35.0,
        default: 6.0,
    },
    FilterParamMeta {
        key: "BevelContrast",
        label: "Contrast",
        min: 0.0,
        max: 100.0,
        default: 70.0,
    },
    FilterParamMeta {
        key: "LightAngle",
        label: "Light angle",
        min: 0.0,
        max: 360.0,
        default: 120.0,
    },
    FilterParamMeta {
        key: "LightDistance",
        label: "Depth",
        min: 1.0,
        max: 200.0,
        default: 100.0,
    },
];

/// Checkbox keys (truthy = not "0"/"false", the compose read rule).
pub(crate) const FILTER_BOOL_KEYS: &[&str] = &["add_noise_use_color", "ShowObject"];

/// The preset for an internal name — the shared lookup behind the
/// panel's quick-add and the Filters menu's `Action::Filter`.
pub(crate) fn filter_preset(name: &str) -> Option<&'static FilterPreset> {
    FILTER_FAMILIES
        .iter()
        .flat_map(|(_, presets)| presets.iter())
        .find(|preset| preset.name == name)
}
