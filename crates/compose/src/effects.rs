//! Live-effect interpretation: `doc::Effect` stacks → `filters` kernels.
//!
//! Identity is the `EffectMoaID` GUID (the dictionary's `DCEv
//! "EffectMoaID"` table); stacks written without a GUID — synthetic
//! `filter.add` payloads — fall back to the internal name (`INMs`).
//! Every native CS6 live effect in the M14 enumeration
//! (`docs/research/fw-code-notes.md`) is interpreted here — the
//! shadow/glow family and the bevel/emboss family since the composite
//! passes, via the MX-era scalar props and the Photoshop Live Effects
//! container's packed strings. Effects still outside it — satin and
//! the fill kinds of the container, third-party plugin GUIDs, Color
//! Fill's exotic blend codes — are skipped, never approximated: the
//! object renders unfiltered through them. That keeps composition
//! total and deterministic (the ADR 0002 oracle contract); widening
//! the catalog is strictly additive.
//!
//! Properties are the `EPSv` decimal strings. A required parameter
//! that is absent or unparseable skips its effect (counted in
//! [`Plan::skipped`]) — CS6 writes every parameter of an effect it
//! saved, so absence means a stack we would otherwise be guessing at.

use color::{BlendMode, Rgba8};
use doc::{Effect, EffectValue, NodeValue};
use filters::{CurvesKnots, Surface};

/// Brightness/Contrast (`EffectMoaID`, dictionary line 1497).
const GUID_BRIGHTNESS_CONTRAST: &str = "3439b08c-1921-11d3-9bde00e02910d580";
/// Hue/Saturation (dictionary line 1498).
const GUID_HUE_SATURATION: &str = "3439b08d-1922-11d3-9bde00e02910d580";
/// Gaussian Blur (dictionary line 1508).
const GUID_GAUSSIAN_BLUR: &str = "d04ef8c0-71b3-11d1-8c8200a024cdc039";
/// Levels (dictionary line 1510; `*_rgb` master + per-channel sets).
const GUID_LEVELS: &str = "d04ef8c2-71b5-11d1-8c8200a024cdc039";
/// Invert (dictionary line 1512).
const GUID_INVERT: &str = "d2541291-70d6-11d1-8c8000a024cdc039";
/// Blur, fixed radius 1 (dictionary line 1559).
const GUID_BLUR: &str = "f1cfce41-718e-11d1-8c8200a024cdc039";
/// Blur More, fixed radius 2 (dictionary line 1560).
const GUID_BLUR_MORE: &str = "f1cfce42-718e-11d1-8c8200a024cdc039";
/// Unsharp Mask (dictionary line 1561).
const GUID_UNSHARP_MASK: &str = "f1cfce44-718e-11d1-8c8200a024cdc039";
/// Add Noise (dictionary line 1558).
const GUID_ADD_NOISE: &str = "e4c0f4bc-c0a3-4cb3-b3513822027e4d9f";
/// Find Edges (dictionary line 1562).
const GUID_FIND_EDGES: &str = "fc7093f1-f95c-11d0-8be200a024cdc039";
/// Curves (dictionary line 1499).
const GUID_CURVES: &str = "3439b08e-1923-11d3-9bde00e02910d580";
/// Auto Levels (dictionary line 1509).
const GUID_AUTO_LEVELS: &str = "d04ef8c1-71b4-11d1-8c8200a024cdc039";
/// Color Fill (dictionary line 1557).
const GUID_COLOR_FILL: &str = "dd54adc0-a279-11d3-b92a000502f3fdbe";
/// Motion Blur (dictionary line 1504).
const GUID_MOTION_BLUR: &str = "538016b6-ffd6-418f-a10c590e6ee841ae";
/// Radial Blur (dictionary line 1516).
const GUID_RADIAL_BLUR: &str = "d1c33142-2ad8-4215-a39fc5d934c8fc0c";
/// Zoom Blur (dictionary line 1506).
const GUID_ZOOM_BLUR: &str = "94dbd663-9b2e-44bf-8064ab6efdd2d327";
/// Sharpen (dictionary line 1494).
const GUID_SHARPEN: &str = "c20952b1-fc76-11d0-8be700a024cdc039";
/// Sharpen More (dictionary line 1493).
const GUID_SHARPEN_MORE: &str = "1f2f2591-9db7-11d1-8cad00a024cdc039";
/// Convert to Alpha (dictionary line 1495).
const GUID_CONVERT_TO_ALPHA: &str = "2932d5a2-ca48-11d1-8561000502701850";
/// Drop Shadow, MX-era scalar props (dictionary `EffectMoaID`).
const GUID_DROP_SHADOW: &str = "a7944db8-6ce2-11d1-8c76000502701850";
/// Inner Shadow, MX-era scalar props.
const GUID_INNER_SHADOW: &str = "5600f702-774c-11d3-baad0000861f4d01";
/// Inner Glow, MX-era scalar props.
const GUID_INNER_GLOW: &str = "2ba87123-8220-11d3-baad0000861f4d01";
/// Solid Shadow, MX-era scalar props.
const GUID_SOLID_SHADOW: &str = "b90c950e-64df-11d8-aaf2000a9582f7d4";
/// Bevel / Emboss, MX-era scalar props — one GUID, `BevelType` picks
/// the variant (dictionary line 1502).
const GUID_BEVEL: &str = "7fe61102-6ce2-11d1-8c76000502701850";
/// Clouds (dictionary line 1496).
const GUID_CLOUDS: &str = "ff84d00c-4494-11d8-a072000a9582f7d4";
/// Difference Clouds (dictionary line 1497).
const GUID_DIFFERENCE_CLOUDS: &str = "d2e7769c-6349-11d8-b0e9000a9595f34e";
/// Photoshop Live Effects Special Fill — the CS6 unified panel; each
/// kind (dropShadow, innerShadow, outerGlow, innerGlow, solidFill, …)
/// arrives as one `&`-packed string property.
const GUID_LIVE_EFFECTS: &str = "8eeadf50-9efe-11da-a7460800200c9a66";

/// Every `EffectMoaID` this crate interprets — the M14 floor's
/// authority. The xtask effect floor holds every GUID observed in the
/// corpus and in harvested files to this set: one outside it would
/// ride through unfiltered, and the passthrough floor is zero. Keep
/// in lockstep with the match arms above; the enumeration test fails
/// the build when a recognized GUID escapes this list.
pub const RECOGNIZED_GUIDS: &[&str] = &[
    GUID_BRIGHTNESS_CONTRAST,
    GUID_HUE_SATURATION,
    GUID_LEVELS,
    GUID_GAUSSIAN_BLUR,
    GUID_INVERT,
    GUID_BLUR,
    GUID_BLUR_MORE,
    GUID_UNSHARP_MASK,
    GUID_ADD_NOISE,
    GUID_FIND_EDGES,
    GUID_CURVES,
    GUID_AUTO_LEVELS,
    GUID_COLOR_FILL,
    GUID_MOTION_BLUR,
    GUID_RADIAL_BLUR,
    GUID_ZOOM_BLUR,
    GUID_SHARPEN,
    GUID_SHARPEN_MORE,
    GUID_CONVERT_TO_ALPHA,
    GUID_DROP_SHADOW,
    GUID_INNER_SHADOW,
    GUID_INNER_GLOW,
    GUID_SOLID_SHADOW,
    GUID_BEVEL,
    GUID_CLOUDS,
    GUID_DIFFERENCE_CLOUDS,
    GUID_LIVE_EFFECTS,
];

/// The pixel-filter catalog CS6's `filterSelectionByName(category,
/// name)` resolves against: Filter-menu categories and the dictionary's
/// `EffectMoaID` spellings. Shadows, bevels, glows and clouds are Live
/// Effect machinery — reachable through the effect forms but not part
/// of the bake-by-name menu. Every GUID here is in `RECOGNIZED_GUIDS`
/// (the enumeration test pins it).
pub const FILTER_CATALOG: &[(&str, &str, &str)] = &[
    ("Adjust Color", "Auto Levels", GUID_AUTO_LEVELS),
    (
        "Adjust Color",
        "Brightness/Contrast",
        GUID_BRIGHTNESS_CONTRAST,
    ),
    ("Adjust Color", "Color Fill", GUID_COLOR_FILL),
    ("Adjust Color", "Convert to Alpha", GUID_CONVERT_TO_ALPHA),
    ("Adjust Color", "Curves", GUID_CURVES),
    ("Adjust Color", "Hue/Saturation", GUID_HUE_SATURATION),
    ("Adjust Color", "Invert", GUID_INVERT),
    ("Adjust Color", "Levels", GUID_LEVELS),
    ("Blur", "Blur", GUID_BLUR),
    ("Blur", "Blur More", GUID_BLUR_MORE),
    ("Blur", "Gaussian Blur", GUID_GAUSSIAN_BLUR),
    ("Blur", "Motion Blur", GUID_MOTION_BLUR),
    ("Blur", "Radial Blur", GUID_RADIAL_BLUR),
    ("Blur", "Zoom Blur", GUID_ZOOM_BLUR),
    ("Noise", "Add Noise", GUID_ADD_NOISE),
    ("Sharpen", "Sharpen", GUID_SHARPEN),
    ("Sharpen", "Sharpen More", GUID_SHARPEN_MORE),
    ("Sharpen", "Unsharp Mask", GUID_UNSHARP_MASK),
    ("Other", "Find Edges", GUID_FIND_EDGES),
];

/// One interpretable kernel step, in stack order.
#[derive(Clone, Debug, PartialEq)]
pub enum KernelOp {
    /// Brightness/Contrast, both −100..=100 (clamped).
    BrightnessContrast {
        /// `brightness_amount`.
        brightness: i32,
        /// `contrast_amount`.
        contrast: i32,
    },
    /// Levels, five-control Photoshop form: the `*_rgb` master
    /// controls, plus the per-channel overrides
    /// (`source_low_red`/`green`/`blue` …) CS6 writes when a channel
    /// was edited individually. Each channel rides its override when
    /// present, the master otherwise.
    Levels {
        /// The `*_rgb` master controls.
        master: filters::LevelsControls,
        /// The `*_red` controls, when the file carries a full set.
        red: Option<filters::LevelsControls>,
        /// The `*_green` controls, when the file carries a full set.
        green: Option<filters::LevelsControls>,
        /// The `*_blue` controls, when the file carries a full set.
        blue: Option<filters::LevelsControls>,
    },
    /// Invert — alpha passes through.
    Invert,
    /// Hue/Saturation: degrees, then −100..=100 percents (kernel takes
    /// fractions, so percents divide by 100 here). Colorize mode
    /// (`hls_colorize`) REPLACES the pixel hue instead of shifting —
    /// `hue` is then absolute 0..360.
    HueSaturation {
        /// `hue_amount`.
        hue: f64,
        /// `saturation_amount` / 100.
        saturation: f64,
        /// `lightness_amount` / 100.
        lightness: f64,
        /// `hls_colorize`.
        colorize: bool,
    },
    /// Gaussian Blur, `gaussian_blur_radius` in pixels.
    GaussianBlur {
        /// Blur sigma.
        radius: f64,
    },
    /// Box blur at the CS6 fixed radii (Blur = 1, Blur More = 2).
    BoxBlur {
        /// Box radius.
        radius: u32,
    },
    /// Unsharp Mask with CS6's threshold step (channels whose delta
    /// is at or below the threshold stay untouched).
    UnsharpMask {
        /// `unsharp_mask_radius`.
        radius: f64,
        /// `unsharp_mask_amount`, 0..=500.
        amount: u32,
        /// `unsharp_mask_threshold` (absent = 0).
        threshold: u8,
    },
    /// Curves: knot lists per channel (`GPLv` grab points).
    Curves {
        /// The knots.
        knots: CurvesKnots,
    },
    /// Auto Levels — per-channel histogram stretch, no parameters.
    AutoLevels,
    /// Color Fill: solid color at opacity in a blend mode.
    ColorFill {
        /// `Color` (`#rrggbb`).
        fill: [u8; 3],
        /// `Opacity`, 0..=100.
        opacity: u32,
        /// `Blendmode`, the dictionary's numeric code mapped onto the
        /// kernel blend set.
        mode: BlendMode,
    },
    /// Motion Blur.
    MotionBlur {
        /// `motion_blur_angle`, degrees.
        angle_degrees: f64,
        /// `motion_blur_distance`, 1..=100.
        distance: u32,
        /// Tap count: `distance + 1` from the effect carrier; the
        /// dom verb's `samples` when it drives the bake.
        taps: u32,
    },
    /// Radial (spin) Blur.
    RadialBlur {
        /// `radial_blur_amount` (sweep degrees), 1..=100.
        amount: u32,
        /// `radial_blur_quality` (taps), 1..=100.
        quality: u32,
    },
    /// Zoom Blur.
    ZoomBlur {
        /// `zoom_blur_amount` (% pinch), 1..=100.
        amount: u32,
        /// `zoom_blur_quality` (taps), 1..=100.
        quality: u32,
    },
    /// Raw convolution — the `convolveSelection` dom verb's arbitrary
    /// integer kernel (`kernelValues`), edge-clamped, anchor at the
    /// kernel center. Coefficients are applied as-is: their scale is
    /// the caller's contract, exactly as the dom documents it.
    Convolve {
        /// Kernel width in cells.
        width: u32,
        /// Kernel height in cells.
        height: u32,
        /// `width * height` row-major coefficients.
        values: Vec<i64>,
        /// Extend the pass to the alpha channel.
        affects_alpha: bool,
    },
    /// Sharpen — fixed preset, no parameters.
    Sharpen,
    /// Sharpen More — fixed preset, no parameters.
    SharpenMore,
    /// Convert to Alpha — luma-to-opacity stencil, no parameters.
    ConvertToAlpha,
    /// Drop Shadow — silhouette blurred, offset opposite the light,
    /// drawn under the source.
    DropShadow {
        /// `ShadowColor` (`#rrggbb[aa]`).
        color: Rgba8,
        /// `ShadowAngle`, degrees (light direction).
        angle_degrees: f64,
        /// `ShadowDistance` in pixels.
        distance: u32,
        /// `ShadowBlur` radius.
        blur: u32,
        /// `ShadowType` = 1 (knockout).
        knockout: bool,
    },
    /// Inner Shadow — inverted silhouette blurred, offset, clipped to
    /// the object, drawn over it.
    InnerShadow {
        /// `ShadowColor`.
        color: Rgba8,
        /// `ShadowAngle`, degrees.
        angle_degrees: f64,
        /// `ShadowDistance` in pixels.
        distance: u32,
        /// `ShadowBlur` radius.
        blur: u32,
    },
    /// Solid Shadow — the silhouette colorized flat.
    SolidShadow {
        /// `ShadowColor`.
        color: Rgba8,
        /// `ShowObject` (source stays visible on top).
        show_object: bool,
    },
    /// Outer Glow — concentric silhouette halo.
    OuterGlow {
        /// `ShadowColor`.
        color: Rgba8,
        /// `GlowWidth` blur radius.
        blur: u32,
        /// `EdgeThreshold` scaled to 0..255 (Glow uses 1 → 255).
        edge_threshold: u8,
    },
    /// Inner Glow — concentric interior halo.
    InnerGlow {
        /// `ShadowColor`.
        color: Rgba8,
        /// `GlowWidth` blur radius.
        blur: u32,
        /// `EdgeThreshold` scaled to 0..255.
        edge_threshold: u8,
    },
    /// Bevel / Emboss — the height-field rim family. `start` only
    /// carries meaning for the Glow style; `face` only for the
    /// emboss styles.
    Bevel {
        /// `BevelType` 0..=4 mapped onto the kernel's styles.
        style: filters::BevelStyle,
        /// `LightAngle`, degrees (where the light is).
        angle_degrees: f64,
        /// `BevelWidth` (glow: `GlowWidth`) in pixels.
        width: u32,
        /// `BevelContrast`, 0..=100.
        contrast: u32,
        /// `LightDistance` depth percent, 39..=200 observed.
        depth: u32,
        /// `GlowStartDistance`, 0..=35 (glow only).
        start: u32,
        /// `HiliteColor`.
        hilite: Rgba8,
        /// `OuterBevelColor` (the dark rim).
        shadow: Rgba8,
        /// `EmbossFaceColor` (raised/inset only, when present).
        face: Option<Rgba8>,
    },
    /// Clouds — the opaque value-noise fill across the object's
    /// bounding box (kernel substitution pinned in `filters`).
    Clouds {
        /// Derived from the effect identity + parameters.
        seed: u64,
        /// `sky_color`.
        sky: Rgba8,
        /// `cloud_color`.
        cloud: Rgba8,
    },
    /// Difference Clouds — the same texture subtracted inside the
    /// object's own silhouette.
    DifferenceClouds {
        /// Derived from the effect identity + parameters.
        seed: u64,
        /// `sky_color`.
        sky: Rgba8,
        /// `cloud_color`.
        cloud: Rgba8,
    },
    /// Add Noise with a stack-deterministic seed (no clocks, no
    /// randomness — the seed derives from the effect's own bytes, so
    /// journal replay and re-renders are bit-stable).
    AddNoise {
        /// Derived from the effect identity + parameters.
        seed: u64,
        /// `add_noise_amount`, 0..=100.
        amount: u32,
        /// !`add_noise_use_color`.
        monochrome: bool,
    },
    /// Find Edges — no parameters.
    FindEdges,
}

/// An interpreted stack: the kernel ops in file order plus how many
/// effects were passed over.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// Interpretable steps in stack order (bottom → top).
    pub ops: Vec<KernelOp>,
    /// Effects skipped as uninterpretable (wrong identity, unknown
    /// name, missing parameter, hidden via `EffectIsVisible`).
    pub skipped: usize,
}

/// Interprets a live-effect stack into kernel ops.
pub fn plan(stack: &[Effect]) -> Plan {
    let mut ops = Vec::new();
    let mut skipped = 0usize;
    for effect in stack {
        if !is_visible(effect) {
            skipped += 1;
            continue;
        }
        match effect_id(effect) {
            Some(id) if id == GUID_BRIGHTNESS_CONTRAST => match brightness_contrast(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_HUE_SATURATION => match hue_saturation(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_LEVELS => match levels(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_GAUSSIAN_BLUR => match gaussian_blur(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_INVERT => ops.push(KernelOp::Invert),
            Some(id) if id == GUID_BLUR => ops.push(KernelOp::BoxBlur { radius: 1 }),
            Some(id) if id == GUID_BLUR_MORE => ops.push(KernelOp::BoxBlur { radius: 2 }),
            Some(id) if id == GUID_UNSHARP_MASK => match unsharp_mask(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_ADD_NOISE => match add_noise(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_FIND_EDGES => ops.push(KernelOp::FindEdges),
            Some(id) if id == GUID_CURVES => match curves(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_AUTO_LEVELS => ops.push(KernelOp::AutoLevels),
            Some(id) if id == GUID_COLOR_FILL => match color_fill(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_MOTION_BLUR => match motion_blur(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_RADIAL_BLUR => match radial_blur(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_ZOOM_BLUR => match zoom_blur(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_SHARPEN => ops.push(KernelOp::Sharpen),
            Some(id) if id == GUID_SHARPEN_MORE => ops.push(KernelOp::SharpenMore),
            Some(id) if id == GUID_CONVERT_TO_ALPHA => ops.push(KernelOp::ConvertToAlpha),
            Some(id) if id == GUID_DROP_SHADOW => match mx_drop_shadow(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_INNER_SHADOW => match mx_inner_shadow(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_INNER_GLOW => match mx_inner_glow(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_SOLID_SHADOW => match mx_solid_shadow(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_BEVEL => match mx_bevel(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_CLOUDS => match clouds(effect, false) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_DIFFERENCE_CLOUDS => match clouds(effect, true) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
            Some(id) if id == GUID_LIVE_EFFECTS => match live_effects(effect) {
                Some(parsed) => ops.extend(parsed),
                None => skipped += 1,
            },
            // A GUID we do not recognize is never guessed at — CS6
            // files carry effect identities beyond our kernel catalog.
            Some(_) => skipped += 1,
            // No GUID: fall back to the internal name so synthetic
            // stacks (and hand-written files) still compose.
            None => match by_name(effect) {
                Some(op) => ops.push(op),
                None => skipped += 1,
            },
        }
    }
    Plan { ops, skipped }
}

/// Applies a plan's ops to a surface, in order.
pub fn apply_stack(mut surface: Surface, ops: &[KernelOp]) -> Surface {
    for op in ops {
        surface = match op {
            KernelOp::BrightnessContrast {
                brightness,
                contrast,
            } => filters::brightness_contrast(&surface, *brightness, *contrast),
            KernelOp::Levels {
                master,
                red,
                green,
                blue,
            } => {
                // A channel rides its override when the full five-prop
                // group parsed, else the `*_rgb` master — the parse
                // guarantees no partial group reaches here.
                let pick = |over: &Option<filters::LevelsControls>| match over {
                    Some(c) => *c,
                    None => *master,
                };
                filters::levels_channels(&surface, pick(red), pick(green), pick(blue))
            }
            KernelOp::Invert => filters::invert(&surface),
            KernelOp::HueSaturation {
                hue,
                saturation,
                lightness,
                colorize,
            } => filters::hsl(&surface, *hue, *saturation, *lightness, *colorize),
            KernelOp::Clouds { seed, sky, cloud } => filters::clouds(&surface, *seed, *sky, *cloud),
            KernelOp::DifferenceClouds { seed, sky, cloud } => {
                filters::difference_clouds(&surface, *seed, *sky, *cloud)
            }
            KernelOp::GaussianBlur { radius } => filters::gaussian_blur(&surface, *radius),
            KernelOp::BoxBlur { radius } => filters::box_blur(&surface, *radius),
            KernelOp::UnsharpMask {
                radius,
                amount,
                threshold,
            } => filters::unsharp_mask_thresholded(&surface, *radius, *amount, *threshold),
            KernelOp::AddNoise {
                seed,
                amount,
                monochrome,
            } => filters::add_noise(&surface, *seed, *amount, *monochrome),
            KernelOp::FindEdges => filters::find_edges(&surface),
            KernelOp::Curves { knots } => filters::curves(&surface, knots),
            KernelOp::AutoLevels => filters::auto_levels(&surface),
            KernelOp::ColorFill {
                fill,
                opacity,
                mode,
            } => filters::solid_fill(
                &surface,
                color::Rgba8::new(fill[0], fill[1], fill[2], 255),
                *opacity,
                *mode,
            ),
            KernelOp::MotionBlur {
                angle_degrees,
                distance,
                taps,
            } => filters::motion_blur_taps(&surface, *angle_degrees, *distance, *taps),
            KernelOp::RadialBlur { amount, quality } => {
                filters::radial_blur_spin(&surface, *amount, *quality)
            }
            KernelOp::ZoomBlur { amount, quality } => {
                filters::zoom_blur(&surface, *amount, *quality)
            }
            KernelOp::Convolve {
                width,
                height,
                values,
                affects_alpha,
            } => filters::convolve(&surface, *width, *height, values, *affects_alpha),
            KernelOp::Sharpen => filters::sharpen(&surface),
            KernelOp::SharpenMore => filters::sharpen_more(&surface),
            KernelOp::ConvertToAlpha => filters::convert_to_alpha(&surface),
            KernelOp::DropShadow {
                color,
                angle_degrees,
                distance,
                blur,
                knockout,
            } => filters::drop_shadow(
                &surface,
                *color,
                *angle_degrees,
                *distance,
                *blur,
                *knockout,
            ),
            KernelOp::InnerShadow {
                color,
                angle_degrees,
                distance,
                blur,
            } => filters::inner_shadow(&surface, *color, *angle_degrees, *distance, *blur),
            KernelOp::SolidShadow { color, show_object } => {
                filters::solid_shadow(&surface, *color, *show_object)
            }
            KernelOp::OuterGlow {
                color,
                blur,
                edge_threshold,
            } => filters::outer_glow(&surface, *color, *blur, *edge_threshold),
            KernelOp::InnerGlow {
                color,
                blur,
                edge_threshold,
            } => filters::inner_glow(&surface, *color, *blur, *edge_threshold),
            KernelOp::Bevel {
                style,
                angle_degrees,
                width,
                contrast,
                depth,
                start,
                hilite,
                shadow,
                face,
            } => filters::bevel(
                &surface,
                *style,
                *angle_degrees,
                *width,
                *contrast,
                *depth,
                *start,
                *hilite,
                *shadow,
                *face,
            ),
        };
    }
    surface
}

/// `EffectIsVisible` — CS6 hides an effect without removing it from
/// the stack. Absent or truthy renders; a literal `"0"`/`"false"`
/// (any case) skips.
fn is_visible(effect: &Effect) -> bool {
    match prop(effect, "EffectIsVisible") {
        Some(EffectValue::Text(text)) => {
            let text = text.trim().to_ascii_lowercase();
            !(text == "0" || text == "false")
        }
        Some(EffectValue::Raw(_)) | None => true,
    }
}

/// Normalized `EffectMoaID`, when the effect carries one: braces and
/// case stripped (`{D2541291-...}` == `d2541291-...`).
fn effect_id(effect: &Effect) -> Option<String> {
    match prop(effect, "EffectMoaID") {
        Some(EffectValue::Text(guid)) => Some(
            guid.trim()
                .trim_start_matches('{')
                .trim_end_matches('}')
                .to_ascii_lowercase(),
        ),
        Some(EffectValue::Raw(_)) | None => None,
    }
}

/// Internal-name fallback, lowercased alphanumerics only
/// (`Gaussian Blur`, `gaussianBlur` and `gaussian_blur` all match).
fn by_name(effect: &Effect) -> Option<KernelOp> {
    let name: String = effect
        .internal_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    match name.as_str() {
        "invert" => Some(KernelOp::Invert),
        "gaussianblur" => gaussian_blur(effect),
        "blur" => Some(KernelOp::BoxBlur { radius: 1 }),
        "blurmore" => Some(KernelOp::BoxBlur { radius: 2 }),
        "unsharpmask" => unsharp_mask(effect),
        "addnoise" => add_noise(effect),
        "findedges" => Some(KernelOp::FindEdges),
        "brightnesscontrast" => brightness_contrast(effect),
        "huesaturation" => hue_saturation(effect),
        "levels" => levels(effect),
        "curves" => curves(effect),
        "autolevels" => Some(KernelOp::AutoLevels),
        "colorfill" => color_fill(effect),
        "motionblur" => motion_blur(effect),
        "radialblur" => radial_blur(effect),
        "zoomblur" => zoom_blur(effect),
        "sharpen" => Some(KernelOp::Sharpen),
        "sharpenmore" => Some(KernelOp::SharpenMore),
        "converttoalpha" => Some(KernelOp::ConvertToAlpha),
        "clouds" => clouds(effect, false),
        "differenceclouds" => clouds(effect, true),
        "dropshadow" => mx_drop_shadow(effect),
        "innershadow" => mx_inner_shadow(effect),
        "innerglow" => mx_inner_glow(effect),
        "solidshadow" => mx_solid_shadow(effect),
        "bevel" | "bevelemboss" | "innerbevel" | "outerbevel" | "raisedemboss" | "insetemboss"
        | "emboss" | "pillowemboss" | "glow" => mx_bevel(effect),
        _ => None,
    }
}

/// Curves knots: each `*_points` property carries a `GPLv` grab-point
/// list node; every `GPTv` child holds `XLCf`/`YLCf` floats. A
/// malformed list fails the whole effect (we would be misreading the
/// tree, not reading a curve). An effect with no knot lists at all is
/// skipped — it would change nothing.
fn curves(effect: &Effect) -> Option<KernelOp> {
    let knots = CurvesKnots {
        rgb: grab_points(effect, "rgb_points")?,
        red: grab_points(effect, "red_points")?,
        green: grab_points(effect, "green_points")?,
        blue: grab_points(effect, "blue_points")?,
    };
    if knots.is_empty() {
        return None;
    }
    Some(KernelOp::Curves { knots })
}

/// Reads one `GPLv` list; an absent property is an empty channel
/// (CS6 writes only the channels the user touched).
fn grab_points(effect: &Effect, key: &str) -> Option<Vec<(f64, f64)>> {
    let node = match prop(effect, key) {
        Some(EffectValue::Raw(node)) => node,
        Some(EffectValue::Text(_)) => return None,
        None => return Some(Vec::new()),
    };
    if node.fourcc != *b"GPLv" {
        return None;
    }
    let NodeValue::Children(points) = &node.value else {
        return None;
    };
    let mut out = Vec::with_capacity(points.len());
    for point in points {
        if point.fourcc != *b"GPTv" {
            return None;
        }
        let NodeValue::Children(coords) = &point.value else {
            return None;
        };
        let mut x = None;
        let mut y = None;
        for coord in coords {
            match (coord.fourcc, &coord.value) {
                (f, NodeValue::Float(v)) if f == *b"XLCf" => x = Some(*v),
                (f, NodeValue::Float(v)) if f == *b"YLCf" => y = Some(*v),
                _ => {}
            }
        }
        out.push((x?, y?));
    }
    Some(out)
}

fn color_fill(effect: &Effect) -> Option<KernelOp> {
    let hex = prop_text(effect, "Color")?;
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
    let fill = [channel(0)?, channel(2)?, channel(4)?];
    let opacity = prop_f64(effect, "Opacity")?;
    let code = prop_f64(effect, "Blendmode")?;
    Some(KernelOp::ColorFill {
        fill,
        opacity: opacity.clamp(0.0, 100.0) as u32,
        mode: blend_mode_code(code)?,
    })
}

/// The dictionary's `Blendmode` numeric codes mapped onto the kernel
/// blend set. The codes with no kernel counterpart (reflect, glow,
/// freeze, negation, xor, tint, …) return `None` — the effect skips
/// rather than guessing what CS6's private mode table means.
fn blend_mode_code(code: f64) -> Option<BlendMode> {
    match code as i64 {
        0 => Some(BlendMode::Normal),
        2 => Some(BlendMode::ColorBurn),
        3 => Some(BlendMode::Luminosity),
        4 => Some(BlendMode::HardLight),
        8 => Some(BlendMode::Lighten),
        9 => Some(BlendMode::Multiply),
        10 => Some(BlendMode::ColorDodge),
        18 => Some(BlendMode::SoftLight),
        21 => Some(BlendMode::Hue),
        22 => Some(BlendMode::Saturation),
        33 => Some(BlendMode::Exclusion),
        35 => Some(BlendMode::Difference),
        40 => Some(BlendMode::Overlay),
        _ => None,
    }
}

/// MX-era Drop Shadow: `ShadowColor` (`#rrggbb[aa]`), `ShadowAngle`
/// (degrees), `ShadowBlur`, `ShadowDistance`, `ShadowType`
/// (0 normal, 1 knockout). Angles normalize into the MX-era 0..360
/// domain — a file carrying −135 means 225 (the same light), not
/// "clamped to above".
fn mx_drop_shadow(effect: &Effect) -> Option<KernelOp> {
    let (color, angle, blur, distance) = mx_shadow_params(effect)?;
    Some(KernelOp::DropShadow {
        color,
        angle_degrees: angle.rem_euclid(360.0),
        distance: distance.round().clamp(0.0, 250.0) as u32,
        blur: blur.round().clamp(0.0, 250.0) as u32,
        knockout: prop_f64(effect, "ShadowType").unwrap_or(0.0) as i64 == 1,
    })
}

/// MX-era Inner Shadow: the same scalar props, no knockout switch
/// (the effect only exists inside the object).
fn mx_inner_shadow(effect: &Effect) -> Option<KernelOp> {
    let (color, angle, blur, distance) = mx_shadow_params(effect)?;
    Some(KernelOp::InnerShadow {
        color,
        angle_degrees: angle.rem_euclid(360.0),
        distance: distance.round().clamp(0.0, 250.0) as u32,
        blur: blur.round().clamp(0.0, 250.0) as u32,
    })
}

/// `ShadowColor` + `ShadowAngle` + `ShadowBlur` + `ShadowDistance`.
fn mx_shadow_params(effect: &Effect) -> Option<(Rgba8, f64, f64, f64)> {
    Some((
        prop_color(effect, "ShadowColor")?,
        prop_f64(effect, "ShadowAngle")?,
        prop_f64(effect, "ShadowBlur")?,
        prop_f64(effect, "ShadowDistance")?,
    ))
}

/// MX-era Solid Shadow: `ShadowColor` + `ShowObject` (absent = shown).
fn mx_solid_shadow(effect: &Effect) -> Option<KernelOp> {
    let color = prop_color(effect, "ShadowColor")?;
    let show_object = match prop_text(effect, "ShowObject") {
        Some(text) => {
            let text = text.trim().to_ascii_lowercase();
            !(text == "0" || text == "false")
        }
        None => true,
    };
    Some(KernelOp::SolidShadow { color, show_object })
}

/// MX-era Inner Glow: `ShadowColor`, `GlowWidth`, and `EdgeThreshold`
/// (the dictionary records Glow's own value at 1).
fn mx_inner_glow(effect: &Effect) -> Option<KernelOp> {
    let color = prop_color(effect, "ShadowColor")?;
    let width = prop_f64(effect, "GlowWidth")?;
    let threshold = prop_f64(effect, "EdgeThreshold").unwrap_or(1.0);
    Some(KernelOp::InnerGlow {
        color,
        blur: width.round().clamp(0.0, 250.0) as u32,
        edge_threshold: (threshold.clamp(0.0, 1.0) * 255.0).round() as u8,
    })
}

/// MX-era Bevel / Emboss: one GUID, `BevelType` picks the variant
/// (0 inner bevel, 1 outer bevel, 2 raised emboss, 3 inset emboss,
/// 4 the glow). The bevels read `BevelWidth`; the glow reads
/// `GlowWidth` + `GlowStartDistance`. `LightDistance` is a depth
/// percent (39..=200 observed) and defaults to 100; so does
/// `BevelContrast`. `MaskSoftness` and `AngleSoftness` are pinned
/// inert (the height-field blur already bounds the shading).
fn mx_bevel(effect: &Effect) -> Option<KernelOp> {
    let kind = prop_f64(effect, "BevelType")? as i64;
    let angle_degrees = prop_f64(effect, "LightAngle")?.rem_euclid(360.0);
    let hilite = prop_color(effect, "HiliteColor")?;
    let shadow = prop_color(effect, "OuterBevelColor")?;
    let contrast = prop_f64(effect, "BevelContrast")
        .unwrap_or(100.0)
        .round()
        .clamp(0.0, 100.0) as u32;
    let depth = prop_f64(effect, "LightDistance")
        .unwrap_or(100.0)
        .round()
        .clamp(1.0, 1000.0) as u32;
    let style = match kind {
        0 => filters::BevelStyle::InnerBevel,
        1 => filters::BevelStyle::OuterBevel,
        2 => filters::BevelStyle::RaisedEmboss,
        3 => filters::BevelStyle::InsetEmboss,
        4 => filters::BevelStyle::Glow,
        _ => return None,
    };
    let (width, start) = if style == filters::BevelStyle::Glow {
        let width = prop_f64(effect, "GlowWidth")?;
        let start = prop_f64(effect, "GlowStartDistance").unwrap_or(0.0);
        (
            width.round().clamp(0.0, 35.0) as u32,
            start.round().clamp(0.0, 35.0) as u32,
        )
    } else {
        let width = prop_f64(effect, "BevelWidth")?;
        (width.round().clamp(0.0, 35.0) as u32, 0)
    };
    Some(KernelOp::Bevel {
        style,
        angle_degrees,
        width,
        contrast,
        depth,
        start,
        hilite,
        shadow,
        face: prop_color(effect, "EmbossFaceColor"),
    })
}

/// The Photoshop Live Effects container: one effect whose properties
/// carry a `&`-packed string per enabled kind. All-or-nothing — a kind
/// we cannot interpret (satin, gradient/pattern fill, stroke) skips
/// the whole container rather than half-rendering the stack; a
/// malformed packed string does the same (a misread is not a shadow).
fn live_effects(effect: &Effect) -> Option<Vec<KernelOp>> {
    const UNINTERPRETED: &[&str] = &["satin", "gradientFill", "patternFill", "stroke"];
    for key in UNINTERPRETED {
        if prop(effect, key).is_some() {
            return None;
        }
    }
    let mut ops = Vec::new();
    if let Some(text) = prop_text(effect, "dropShadow") {
        ops.push(packed_drop_shadow(text)?);
    }
    if let Some(text) = prop_text(effect, "innerShadow") {
        ops.push(packed_inner_shadow(text)?);
    }
    if let Some(text) = prop_text(effect, "outerGlow") {
        ops.push(packed_outer_glow(text)?);
    }
    if let Some(text) = prop_text(effect, "innerGlow") {
        ops.push(packed_inner_glow(text)?);
    }
    if let Some(text) = prop_text(effect, "bevelEmboss") {
        ops.push(packed_bevel_emboss(text)?);
    }
    if let Some(text) = prop_text(effect, "solidFill") {
        ops.push(packed_solid_fill(text)?);
    }
    if ops.is_empty() { None } else { Some(ops) }
}

/// `dropShadow`/`innerShadow` packed fields (1-based dictionary
/// numbering over the `&` split): 2 opacity, 4 angle, 7 size,
/// 8 distance, 12 `RGBColor;r;g;b`. The dictionary labels field 8
/// "always `0`" but its own example carries `74.000000` there — the
/// positional read (Photoshop field order: distance after size) wins.
/// Opacity bakes into the shadow color's alpha; field 5 (spread) and
/// the packed blend token have no action at this layer and are pinned
/// inert (recorded in the shadow module docs).
fn packed_drop_shadow(text: &str) -> Option<KernelOp> {
    let (color, angle, size, distance) = packed_shadow_fields(text)?;
    Some(KernelOp::DropShadow {
        color,
        angle_degrees: angle.clamp(-180.0, 360.0),
        distance: distance.round().clamp(0.0, 250.0) as u32,
        blur: size.round().clamp(0.0, 250.0) as u32,
        knockout: false,
    })
}

/// `innerShadow` shares the dropShadow field shape.
fn packed_inner_shadow(text: &str) -> Option<KernelOp> {
    let (color, angle, size, distance) = packed_shadow_fields(text)?;
    Some(KernelOp::InnerShadow {
        color,
        angle_degrees: angle.clamp(-180.0, 360.0),
        distance: distance.round().clamp(0.0, 250.0) as u32,
        blur: size.round().clamp(0.0, 250.0) as u32,
    })
}

fn packed_shadow_fields(text: &str) -> Option<(Rgba8, f64, f64, f64)> {
    let fields: Vec<&str> = text.split('&').collect();
    let number = |at: usize| fields.get(at)?.trim().parse::<f64>().ok();
    let mut color = packed_color(fields.get(11)?)?;
    // Opacity (field 2) is the packed shadow's only alpha carrier —
    // the RGBColor triplet has none — so it fades the color here.
    let opacity = number(1)?.clamp(0.0, 100.0) / 100.0;
    color.a = (opacity * f64::from(color.a)).round() as u8;
    Some((color, number(3)?, number(6)?, number(7)?))
}

/// `outerGlow`/`innerGlow` packed fields: 3 blend token, 4 a
/// `:`-packed number list (1 opacity, 3 size), 6 `RGBColor;r;g;b`,
/// 9 technique token. Opacity bakes into the color's alpha; the
/// undocumented numbers and tokens are pinned inert; the edge
/// threshold stays soft (0).
fn packed_outer_glow(text: &str) -> Option<KernelOp> {
    let (color, size) = packed_glow_fields(text)?;
    Some(KernelOp::OuterGlow {
        color,
        blur: size.round().clamp(0.0, 250.0) as u32,
        edge_threshold: 0,
    })
}

/// `innerGlow` shares the outerGlow field shape.
fn packed_inner_glow(text: &str) -> Option<KernelOp> {
    let (color, size) = packed_glow_fields(text)?;
    Some(KernelOp::InnerGlow {
        color,
        blur: size.round().clamp(0.0, 250.0) as u32,
        edge_threshold: 0,
    })
}

/// Returns the opacity-faded color and the size.
fn packed_glow_fields(text: &str) -> Option<(Rgba8, f64)> {
    let fields: Vec<&str> = text.split('&').collect();
    let numbers: Vec<f64> = fields
        .get(3)?
        .split(':')
        .map(|part| part.trim().parse::<f64>())
        .collect::<Result<_, _>>()
        .ok()?;
    let [r, g, b] = packed_rgb(fields.get(5)?)?;
    let opacity = numbers.first()?.clamp(0.0, 100.0) / 100.0;
    Some((
        Rgba8::new(r, g, b, (opacity * 255.0).round() as u8),
        *numbers.get(2)?,
    ))
}

/// `bevelEmboss` packed fields (1-based dictionary numbering): 8
/// angle (−180..180), 10 depth (1..1000), 11 size (0..250), 14
/// highlight `RGBColor`, 16 shadow `RGBColor`, 18 style token. The
/// 75-twins (6, 7), the "always 30" field (9), soften (12), the
/// blend tokens (13, 15), the matte token (17), and stampIn/stampOut
/// (19) are pinned inert; the packed form carries no contrast
/// (100) and no face color.
fn packed_bevel_emboss(text: &str) -> Option<KernelOp> {
    let fields: Vec<&str> = text.split('&').collect();
    let number = |at: usize| fields.get(at)?.trim().parse::<f64>().ok();
    let style = match token(fields.get(17)?)?.as_str() {
        "innerbevel" => filters::BevelStyle::InnerBevel,
        "outerbevel" => filters::BevelStyle::OuterBevel,
        "emboss" => filters::BevelStyle::RaisedEmboss,
        "pillowemboss" => filters::BevelStyle::InsetEmboss,
        _ => return None, // strokeEmboss and anything unrecognized
    };
    Some(KernelOp::Bevel {
        style,
        angle_degrees: number(7)?.rem_euclid(360.0),
        width: number(10)?.round().clamp(0.0, 250.0) as u32,
        contrast: 100,
        depth: number(9)?.round().clamp(1.0, 1000.0) as u32,
        start: 0,
        hilite: packed_color(fields.get(13)?)?,
        shadow: packed_color(fields.get(15)?)?,
        face: None,
    })
}

/// A packed token, lowercased alphanumerics only.
fn token(text: &str) -> Option<String> {
    Some(
        text.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect(),
    )
}

/// `solidFill` (Photoshop Color Overlay): 2 opacity, 3 blend token,
/// 4 `RGBColor;r;g;b` — the one packed kind whose blend is
/// representable (it blends against the object itself).
fn packed_solid_fill(text: &str) -> Option<KernelOp> {
    let fields: Vec<&str> = text.split('&').collect();
    let opacity = fields.get(1)?.trim().parse::<f64>().ok()?;
    let mode = blend_name(fields.get(2)?)?;
    Some(KernelOp::ColorFill {
        fill: packed_rgb(fields.get(3)?)?,
        opacity: opacity.clamp(0.0, 100.0) as u32,
        mode,
    })
}

/// `RGBColor;<r>;<g>;<b>` with float channels.
fn packed_color(text: &str) -> Option<Rgba8> {
    let (r, g, b) = packed_rgb_channels(text)?;
    Some(Rgba8::new(r, g, b, 255))
}

/// The same shape as `[u8; 3]` for the ColorFill op.
fn packed_rgb(text: &str) -> Option<[u8; 3]> {
    let (r, g, b) = packed_rgb_channels(text)?;
    Some([r, g, b])
}

fn packed_rgb_channels(text: &str) -> Option<(u8, u8, u8)> {
    let rest = text.trim().strip_prefix("RGBColor;")?;
    let channel = |value: &str| {
        value
            .trim()
            .parse::<f64>()
            .ok()
            .map(|f| f.round().clamp(0.0, 255.0) as u8)
    };
    let mut parts = rest.split(';');
    Some((
        channel(parts.next()?)?,
        channel(parts.next()?)?,
        channel(parts.next()?)?,
    ))
}

/// Blend mode by packed-string name (lowercased alphanumerics, the
/// observed spellings being `normal`, `multiply`, `screen`).
fn blend_name(text: &str) -> Option<BlendMode> {
    let name: String = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    match name.as_str() {
        "normal" => Some(BlendMode::Normal),
        "multiply" => Some(BlendMode::Multiply),
        "screen" => Some(BlendMode::Screen),
        "overlay" => Some(BlendMode::Overlay),
        "darken" => Some(BlendMode::Darken),
        "lighten" => Some(BlendMode::Lighten),
        "hardlight" => Some(BlendMode::HardLight),
        "softlight" => Some(BlendMode::SoftLight),
        "colordodge" => Some(BlendMode::ColorDodge),
        "colorburn" => Some(BlendMode::ColorBurn),
        "difference" => Some(BlendMode::Difference),
        "exclusion" => Some(BlendMode::Exclusion),
        "hue" => Some(BlendMode::Hue),
        "saturation" => Some(BlendMode::Saturation),
        "color" => Some(BlendMode::Color),
        "luminosity" => Some(BlendMode::Luminosity),
        _ => None,
    }
}

/// `#rrggbb` or `#rrggbbaa` → [`Rgba8`] (6-digit form is opaque).
fn prop_color(effect: &Effect, key: &str) -> Option<Rgba8> {
    let hex = prop_text(effect, key)?;
    let hex = hex.trim().trim_start_matches('#');
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
    match hex.len() {
        6 => Some(Rgba8::new(channel(0)?, channel(2)?, channel(4)?, 255)),
        8 => Some(Rgba8::new(
            channel(0)?,
            channel(2)?,
            channel(4)?,
            channel(6)?,
        )),
        _ => None,
    }
}

fn motion_blur(effect: &Effect) -> Option<KernelOp> {
    let angle = prop_f64(effect, "motion_blur_angle")?;
    let distance = prop_f64(effect, "motion_blur_distance")?;
    if distance < 1.0 {
        return None;
    }
    Some(KernelOp::MotionBlur {
        angle_degrees: angle.clamp(0.0, 360.0),
        distance: distance.clamp(1.0, 100.0) as u32,
        taps: distance.clamp(1.0, 100.0) as u32 + 1,
    })
}

fn radial_blur(effect: &Effect) -> Option<KernelOp> {
    let amount = prop_f64(effect, "radial_blur_amount")?;
    let quality = prop_f64(effect, "radial_blur_quality")?;
    if amount < 1.0 || quality < 1.0 {
        return None;
    }
    Some(KernelOp::RadialBlur {
        amount: amount.clamp(1.0, 100.0) as u32,
        quality: quality.clamp(1.0, 100.0) as u32,
    })
}

fn zoom_blur(effect: &Effect) -> Option<KernelOp> {
    let amount = prop_f64(effect, "zoom_blur_amount")?;
    let quality = prop_f64(effect, "zoom_blur_quality")?;
    if amount < 1.0 || quality < 1.0 {
        return None;
    }
    Some(KernelOp::ZoomBlur {
        amount: amount.clamp(1.0, 100.0) as u32,
        quality: quality.clamp(1.0, 100.0) as u32,
    })
}

fn brightness_contrast(effect: &Effect) -> Option<KernelOp> {
    let brightness = prop_f64(effect, "brightness_amount")?;
    let contrast = prop_f64(effect, "contrast_amount")?;
    Some(KernelOp::BrightnessContrast {
        brightness: brightness.clamp(-100.0, 100.0) as i32,
        contrast: contrast.clamp(-100.0, 100.0) as i32,
    })
}

fn hue_saturation(effect: &Effect) -> Option<KernelOp> {
    let hue = prop_f64(effect, "hue_amount")?;
    let saturation = prop_f64(effect, "saturation_amount")?;
    let lightness = prop_f64(effect, "lightness_amount")?;
    // The dictionary's `hls_colorize`: truthy replaces the pixel hue
    // (`hue_amount` then absolute 0..360), falsy adjusts it.
    let colorize = prop_bool(effect, "hls_colorize");
    Some(KernelOp::HueSaturation {
        hue: if colorize {
            hue.clamp(0.0, 360.0)
        } else {
            hue.clamp(-180.0, 180.0)
        },
        saturation: saturation.clamp(-100.0, 100.0) / 100.0,
        lightness: lightness.clamp(-100.0, 100.0) / 100.0,
        colorize,
    })
}

/// One channel's Levels controls from the `*_rgb`-suffix family
/// (`suffix` ∈ "rgb" | "red" | "green" | "blue").
fn levels_controls(effect: &Effect, suffix: &str) -> Option<filters::LevelsControls> {
    Some(filters::LevelsControls {
        in_black: prop_u8(effect, &format!("source_low_{suffix}"))?,
        in_white: prop_u8(effect, &format!("source_high_{suffix}"))?,
        gamma: prop_f64(effect, &format!("gamma_{suffix}"))?,
        out_black: prop_u8(effect, &format!("dest_low_{suffix}"))?,
        out_white: prop_u8(effect, &format!("dest_high_{suffix}"))?,
    })
}

fn levels(effect: &Effect) -> Option<KernelOp> {
    // The master set is required (CS6 always writes it); a channel
    // override counts only when its whole five-prop group is present —
    // a partial group is the UI mid-edit state, not a spec.
    let master = levels_controls(effect, "rgb")?;
    let channel = |suffix: &str| levels_controls(effect, suffix);
    Some(KernelOp::Levels {
        master,
        red: channel("red"),
        green: channel("green"),
        blue: channel("blue"),
    })
}

fn clouds(effect: &Effect, difference: bool) -> Option<KernelOp> {
    let sky = prop_color(effect, "sky_color")?;
    let cloud = prop_color(effect, "cloud_color")?;
    let op = if difference {
        KernelOp::DifferenceClouds {
            seed: noise_seed(effect),
            sky,
            cloud,
        }
    } else {
        KernelOp::Clouds {
            seed: noise_seed(effect),
            sky,
            cloud,
        }
    };
    Some(op)
}

fn gaussian_blur(effect: &Effect) -> Option<KernelOp> {
    let radius = prop_f64(effect, "gaussian_blur_radius")?;
    if radius <= 0.0 {
        return None;
    }
    Some(KernelOp::GaussianBlur {
        radius: radius.clamp(0.1, 250.0),
    })
}

fn unsharp_mask(effect: &Effect) -> Option<KernelOp> {
    let radius = prop_f64(effect, "unsharp_mask_radius")?;
    let amount = prop_f64(effect, "unsharp_mask_amount")?;
    if radius <= 0.0 {
        return None;
    }
    // The threshold step is part of the kernel now (M14); an absent
    // property reads as CS6's default 0.
    let threshold = prop_f64(effect, "unsharp_mask_threshold")
        .unwrap_or(0.0)
        .clamp(0.0, 255.0) as u8;
    Some(KernelOp::UnsharpMask {
        radius: radius.clamp(0.1, 250.0),
        amount: amount.clamp(0.0, 500.0) as u32,
        threshold,
    })
}

fn add_noise(effect: &Effect) -> Option<KernelOp> {
    let amount = prop_f64(effect, "add_noise_amount")?;
    let use_color = prop_bool(effect, "add_noise_use_color");
    Some(KernelOp::AddNoise {
        seed: noise_seed(effect),
        amount: amount.clamp(0.0, 100.0) as u32,
        monochrome: !use_color,
    })
}

/// First property value under `key`.
fn prop<'a>(effect: &'a Effect, key: &str) -> Option<&'a EffectValue> {
    Some(&effect.properties.iter().find(|p| p.key == key)?.value)
}

/// Decimal-string property → f64 (`DCVs` carries decimals as text;
/// CS6 writes values like `5.000000`).
fn prop_f64(effect: &Effect, key: &str) -> Option<f64> {
    match prop(effect, key)? {
        EffectValue::Text(text) => text.trim().parse::<f64>().ok(),
        EffectValue::Raw(_) => None,
    }
}

fn prop_u8(effect: &Effect, key: &str) -> Option<u8> {
    prop_f64(effect, key).map(|value| value.round().clamp(0.0, 255.0) as u8)
}

/// Truthy-text flag (`"1"`/`"true"`, any case). Absent or raw reads
/// false — the CS6 scripts only ever write the flag when set.
fn prop_bool(effect: &Effect, key: &str) -> bool {
    match prop(effect, key) {
        Some(EffectValue::Text(text)) => {
            let text = text.trim().to_ascii_lowercase();
            text == "1" || text == "true"
        }
        Some(EffectValue::Raw(_)) | None => false,
    }
}

/// Text property value, for the non-decimal encodings (`Color` hex).
fn prop_text<'a>(effect: &'a Effect, key: &str) -> Option<&'a str> {
    match prop(effect, key)? {
        EffectValue::Text(text) => Some(text),
        EffectValue::Raw(_) => None,
    }
}

/// FNV-1a over the effect's own bytes: the internal name, then every
/// property key/value. Raw values contribute their node type only.
/// This is the noise seed — deterministic across runs, platforms and
/// journal replays without inventing a payload slot.
fn noise_seed(effect: &Effect) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let feed = |hash: &mut u64, bytes: &[u8]| {
        for byte in bytes {
            *hash = (*hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    feed(&mut hash, effect.internal_name.as_bytes());
    for property in &effect.properties {
        feed(&mut hash, property.key.as_bytes());
        match &property.value {
            EffectValue::Text(text) => feed(&mut hash, text.as_bytes()),
            EffectValue::Raw(node) => feed(&mut hash, &node.fourcc),
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use doc::{EffectProperty, Node, NodeValue};

    fn effect(name: &str, props: &[(&str, &str)]) -> Effect {
        Effect {
            category: String::new(),
            internal_name: name.to_owned(),
            properties: props
                .iter()
                .map(|(key, value)| EffectProperty {
                    key: (*key).to_owned(),
                    value: EffectValue::Text((*value).to_owned()),
                })
                .collect(),
        }
    }

    fn guid_effect(name: &str, guid: &str, props: &[(&str, &str)]) -> Effect {
        let mut props = props.to_vec();
        props.push(("EffectMoaID", guid));
        effect(name, &props)
    }

    #[test]
    fn guid_identity_beats_name() {
        // A GUIDed dropShadow named "invert" is still a dropShadow.
        let stack = [guid_effect(
            "invert",
            GUID_ADD_NOISE,
            &[("add_noise_amount", "20")],
        )];
        let plan = plan(&stack);
        assert_eq!(plan.ops.len(), 1);
        assert!(matches!(plan.ops[0], KernelOp::AddNoise { .. }));
        assert_eq!(plan.skipped, 0);
    }

    #[test]
    fn unknown_guid_skips_without_guessing() {
        let stack = [guid_effect(
            "dropShadow",
            "deadbeef-0000-11d1-8c8000a024cdc039",
            &[],
        )];
        let plan = plan(&stack);
        assert!(plan.ops.is_empty());
        assert_eq!(plan.skipped, 1);
    }

    #[test]
    fn name_fallback_matches_cs6_spellings() {
        for (name, expected) in [
            // The name fallback still honors required parameters.
            ("gaussianBlur", 0),
            ("Gaussian Blur", 0),
            ("gaussian_blur", 0),
            ("Blur More", 1),
            ("Find Edges", 2),
        ] {
            let effect = effect(name, &[("gaussian_blur_radius", "5")]);
            let plan = plan(std::slice::from_ref(&effect));
            assert_eq!(plan.ops.len(), 1, "{name}");
            match expected {
                0 => assert!(matches!(plan.ops[0], KernelOp::GaussianBlur { .. })),
                1 => assert!(matches!(plan.ops[0], KernelOp::BoxBlur { radius: 2 })),
                _ => assert!(matches!(plan.ops[0], KernelOp::FindEdges)),
            }
        }
    }

    #[test]
    fn hidden_effects_are_skipped() {
        let stack = [
            effect("invert", &[("EffectIsVisible", "false")]),
            effect("invert", &[("EffectIsVisible", "0")]),
            guid_effect("Invert", GUID_INVERT, &[("EffectIsVisible", "1")]),
        ];
        let plan = plan(&stack);
        assert_eq!(plan.ops.len(), 1);
        assert_eq!(plan.skipped, 2);
    }

    #[test]
    fn missing_required_params_skip_the_effect() {
        let stack = [
            effect("gaussianBlur", &[]),
            guid_effect("gb", GUID_GAUSSIAN_BLUR, &[]),
            guid_effect("lv", GUID_LEVELS, &[("source_low_rgb", "0")]),
            effect("brightnessContrast", &[("brightness_amount", "20")]),
        ];
        let plan = plan(&stack);
        assert!(plan.ops.is_empty());
        assert_eq!(plan.skipped, 4);
    }

    #[test]
    fn unsharp_threshold_rides_through_to_the_kernel() {
        let kept = guid_effect(
            "unsharpMask",
            GUID_UNSHARP_MASK,
            &[
                ("unsharp_mask_radius", "2"),
                ("unsharp_mask_amount", "80"),
                ("unsharp_mask_threshold", "0"),
            ],
        );
        let raised = guid_effect(
            "unsharpMask",
            GUID_UNSHARP_MASK,
            &[
                ("unsharp_mask_radius", "2"),
                ("unsharp_mask_amount", "80"),
                ("unsharp_mask_threshold", "10"),
            ],
        );
        assert!(matches!(
            plan(&[kept]).ops[0],
            KernelOp::UnsharpMask {
                amount: 80,
                threshold: 0,
                ..
            }
        ));
        // The threshold rides through to the kernel now (the M7-era
        // skip is retired); an absent property reads as the CS6
        // default 0.
        assert!(matches!(
            plan(&[raised]).ops[0],
            KernelOp::UnsharpMask { threshold: 10, .. }
        ));
        let bare = guid_effect(
            "unsharpMask",
            GUID_UNSHARP_MASK,
            &[("unsharp_mask_radius", "2"), ("unsharp_mask_amount", "80")],
        );
        assert!(matches!(
            plan(&[bare]).ops[0],
            KernelOp::UnsharpMask { threshold: 0, .. }
        ));
    }

    #[test]
    fn every_enumerated_native_guid_is_recognized() {
        // The M14 floor: each effect CS6 ships natively maps to an op
        // (with its required parameters present) — nothing rides
        // through unfiltered.
        let mut stack = Vec::new();
        let mut curves = guid_effect("curves", GUID_CURVES, &[]);
        curves.properties.push(EffectProperty {
            key: "rgb_points".to_owned(),
            value: EffectValue::Raw(grab_list(&[(0.0, 0.0), (255.0, 255.0)])),
        });
        stack.push(curves);
        for (name, guid, props) in [
            (
                "brightnessContrast",
                GUID_BRIGHTNESS_CONTRAST,
                vec![("brightness_amount", "10"), ("contrast_amount", "0")],
            ),
            (
                "hueSaturation",
                GUID_HUE_SATURATION,
                vec![
                    ("hue_amount", "0"),
                    ("saturation_amount", "0"),
                    ("lightness_amount", "0"),
                ],
            ),
            (
                "levels",
                GUID_LEVELS,
                vec![
                    ("source_low_rgb", "0"),
                    ("source_high_rgb", "255"),
                    ("gamma_rgb", "1"),
                    ("dest_low_rgb", "0"),
                    ("dest_high_rgb", "255"),
                ],
            ),
            (
                "gaussianBlur",
                GUID_GAUSSIAN_BLUR,
                vec![("gaussian_blur_radius", "4")],
            ),
            ("invert", GUID_INVERT, vec![]),
            ("blur", GUID_BLUR, vec![]),
            ("blurMore", GUID_BLUR_MORE, vec![]),
            (
                "unsharpMask",
                GUID_UNSHARP_MASK,
                vec![
                    ("unsharp_mask_radius", "2"),
                    ("unsharp_mask_amount", "50"),
                    ("unsharp_mask_threshold", "3"),
                ],
            ),
            (
                "addNoise",
                GUID_ADD_NOISE,
                vec![("add_noise_amount", "15"), ("add_noise_use_color", "0")],
            ),
            ("findEdges", GUID_FIND_EDGES, vec![]),
            ("autoLevels", GUID_AUTO_LEVELS, vec![]),
            (
                "colorFill",
                GUID_COLOR_FILL,
                vec![("Color", "#336699"), ("Opacity", "60"), ("Blendmode", "0")],
            ),
            (
                "motionBlur",
                GUID_MOTION_BLUR,
                vec![("motion_blur_angle", "45"), ("motion_blur_distance", "10")],
            ),
            (
                "radialBlur",
                GUID_RADIAL_BLUR,
                vec![("radial_blur_amount", "30"), ("radial_blur_quality", "5")],
            ),
            (
                "zoomBlur",
                GUID_ZOOM_BLUR,
                vec![("zoom_blur_amount", "25"), ("zoom_blur_quality", "5")],
            ),
            ("sharpen", GUID_SHARPEN, vec![]),
            ("sharpenMore", GUID_SHARPEN_MORE, vec![]),
            ("convertToAlpha", GUID_CONVERT_TO_ALPHA, vec![]),
            (
                "clouds",
                GUID_CLOUDS,
                vec![("sky_color", "#6699cc"), ("cloud_color", "#ffffff")],
            ),
            (
                "differenceClouds",
                GUID_DIFFERENCE_CLOUDS,
                vec![("sky_color", "#000000"), ("cloud_color", "#ffffff")],
            ),
            (
                "dropShadow",
                GUID_DROP_SHADOW,
                vec![
                    ("ShadowColor", "#00000080"),
                    ("ShadowAngle", "90"),
                    ("ShadowBlur", "4"),
                    ("ShadowDistance", "7"),
                    ("ShadowType", "0"),
                ],
            ),
            (
                "innerShadow",
                GUID_INNER_SHADOW,
                vec![
                    ("ShadowColor", "#000000"),
                    ("ShadowAngle", "135"),
                    ("ShadowBlur", "2"),
                    ("ShadowDistance", "3"),
                ],
            ),
            (
                "solidShadow",
                GUID_SOLID_SHADOW,
                vec![("ShadowColor", "#804000"), ("ShowObject", "true")],
            ),
            (
                "innerGlow",
                GUID_INNER_GLOW,
                vec![
                    ("ShadowColor", "#ffcc00"),
                    ("GlowWidth", "3"),
                    ("EdgeThreshold", "1"),
                ],
            ),
            (
                "innerBevel",
                GUID_BEVEL,
                vec![
                    ("BevelType", "0"),
                    ("BevelWidth", "6"),
                    ("BevelContrast", "70"),
                    ("LightAngle", "120"),
                    ("LightDistance", "150"),
                    ("HiliteColor", "#ffffcc"),
                    ("OuterBevelColor", "#808000"),
                ],
            ),
        ] {
            stack.push(guid_effect(name, guid, &props));
        }
        // The unified panel's outer glow exists only as a packed kind
        // inside the Live Effects container — one effect, four ops
        // (the container carries the dictionary's own example strings).
        let mut live = guid_effect("liveEffects", GUID_LIVE_EFFECTS, &[]);
        live.properties.push(EffectProperty {
            key: "dropShadow".to_owned(),
            value: EffectValue::Text(
                "0&75.000000&0&120.000000&0.000000&5.000000&5.000000&74.000000&0&1&multiply\
                 &RGBColor;0.000000;0.000000;0.000000\
                 &2$$0.000000;0.000000;1$255.000000;255.000000;1"
                    .to_owned(),
            ),
        });
        live.properties.push(EffectProperty {
            key: "outerGlow".to_owned(),
            value: EffectValue::Text(
                "0&0&screen&75.000000:0.000000:5.000000:0.000000:0.000000:50.000000\
                 &softMatte&RGBColor;255.000000;255.000000;189.996109&nogradient\
                 &2$$0.000000;0.000000;1$255.000000;255.000000;1&edgeGlow"
                    .to_owned(),
            ),
        });
        live.properties.push(EffectProperty {
            key: "bevelEmboss".to_owned(),
            value: EffectValue::Text(
                "0&0&0&0&0&75.000000&75.000000&120.000000&30.000000&100.000000&5.000000&0.000000\
                 &screen&RGBColor;255.000000;255.000000;255.000000\
                 &multiply&RGBColor;0.000000;0.000000;0.000000&softMatte&innerBevel&stampIn\
                 &2$$0.000000;0.000000;1$255.000000;255.000000;1&noshape&notexture"
                    .to_owned(),
            ),
        });
        live.properties.push(EffectProperty {
            key: "solidFill".to_owned(),
            value: EffectValue::Text(
                "0&50.000000&normal&RGBColor;51.000000;102.000000;153.000000".to_owned(),
            ),
        });
        stack.push(live);
        let plan = plan(&stack);
        assert_eq!(plan.ops.len(), 30, "every enumerated effect interprets");
        assert_eq!(plan.skipped, 0, "the native surface never rides through");
        // The floor's authority list stays in lockstep: every GUID
        // const published as recognized was exercised above (the
        // live-effects const through the container, the rest through
        // the per-GUID rows).
        let exercised: std::collections::HashSet<String> =
            stack.iter().filter_map(effect_id).collect();
        for guid in RECOGNIZED_GUIDS {
            assert!(
                exercised.contains(*guid),
                "{guid} is listed as recognized but not exercised"
            );
        }
    }

    /// Builds a `GPLv` node with a `GPTv` per point (the CS6 shape:
    /// `XLCf`/`YLCf` float coordinates per grab point).
    fn grab_list(points: &[(f64, f64)]) -> Node {
        Node {
            fourcc: *b"GPLv",
            value: NodeValue::Children(
                points
                    .iter()
                    .map(|(x, y)| Node {
                        fourcc: *b"GPTv",
                        value: NodeValue::Children(vec![
                            Node {
                                fourcc: *b"XLCf",
                                value: NodeValue::Float(*x),
                            },
                            Node {
                                fourcc: *b"YLCf",
                                value: NodeValue::Float(*y),
                            },
                        ]),
                    })
                    .collect(),
            ),
        }
    }

    #[test]
    fn curves_knots_parse_from_grab_point_lists() {
        let mut effect = guid_effect("curves", GUID_CURVES, &[]);
        effect.properties.push(EffectProperty {
            key: "rgb_points".to_owned(),
            value: EffectValue::Raw(grab_list(&[(0.0, 0.0), (128.0, 64.0), (255.0, 255.0)])),
        });
        effect.properties.push(EffectProperty {
            key: "red_points".to_owned(),
            value: EffectValue::Raw(grab_list(&[(0.0, 0.0), (255.0, 255.0)])),
        });
        let mut parsed = plan(std::slice::from_ref(&effect));
        assert_eq!(parsed.ops.len(), 1);
        let KernelOp::Curves { knots } = parsed.ops.remove(0) else {
            panic!("expected curves");
        };
        assert_eq!(knots.rgb, vec![(0.0, 0.0), (128.0, 64.0), (255.0, 255.0)]);
        assert_eq!(knots.red, vec![(0.0, 0.0), (255.0, 255.0)]);
        assert!(knots.green.is_empty() && knots.blue.is_empty());
        // No lists at all: nothing to apply, skipped.
        let bare = guid_effect("curves", GUID_CURVES, &[]);
        let bare_plan = plan(&[bare]);
        assert!(bare_plan.ops.is_empty());
        assert_eq!(bare_plan.skipped, 1);
        // A malformed list (wrong node type) skips the whole effect —
        // a misread tree is not a curve.
        let mut bad = guid_effect("curves", GUID_CURVES, &[]);
        bad.properties.push(EffectProperty {
            key: "rgb_points".to_owned(),
            value: EffectValue::Raw(Node {
                fourcc: *b"TXTv",
                value: NodeValue::Children(Vec::new()),
            }),
        });
        let bad_plan = plan(&[bad]);
        assert!(bad_plan.ops.is_empty());
        assert_eq!(bad_plan.skipped, 1);
    }

    #[test]
    fn color_fill_parses_hex_opacity_and_blend_codes() {
        let effect = guid_effect(
            "colorFill",
            GUID_COLOR_FILL,
            &[
                ("Color", "#336699"),
                ("Opacity", "60.000000"),
                ("Blendmode", "9"),
            ],
        );
        let parsed = plan(std::slice::from_ref(&effect));
        assert_eq!(
            parsed.ops,
            vec![KernelOp::ColorFill {
                fill: [0x33, 0x66, 0x99],
                opacity: 60,
                mode: BlendMode::Multiply,
            }]
        );
        // An exotic code with no kernel counterpart skips — never a
        // guessed mode.
        let exotic = guid_effect(
            "colorFill",
            GUID_COLOR_FILL,
            &[
                ("Color", "#000000"),
                ("Opacity", "100"),
                ("Blendmode", "13"),
            ],
        );
        let exotic_plan = plan(&[exotic]);
        assert!(exotic_plan.ops.is_empty());
        assert_eq!(exotic_plan.skipped, 1);
        // Missing any required property skips.
        let partial = guid_effect(
            "colorFill",
            GUID_COLOR_FILL,
            &[("Color", "#000000"), ("Opacity", "100")],
        );
        assert_eq!(plan(&[partial]).skipped, 1);
    }

    #[test]
    fn directional_blur_parameters_parse() {
        let motion = guid_effect(
            "motionBlur",
            GUID_MOTION_BLUR,
            &[
                ("motion_blur_angle", "45.000000"),
                ("motion_blur_distance", "10"),
            ],
        );
        assert_eq!(
            plan(&[motion]).ops,
            vec![KernelOp::MotionBlur {
                angle_degrees: 45.0,
                distance: 10,
                taps: 11
            }]
        );
        // Distance below the observed range refuses (CS6 never writes
        // it — a sub-1 distance is a misread, not a no-op blur).
        let tiny = guid_effect(
            "motionBlur",
            GUID_MOTION_BLUR,
            &[("motion_blur_angle", "45"), ("motion_blur_distance", "0")],
        );
        assert_eq!(plan(&[tiny]).skipped, 1);
        let spin = guid_effect(
            "radialBlur",
            GUID_RADIAL_BLUR,
            &[("radial_blur_amount", "30"), ("radial_blur_quality", "5")],
        );
        assert_eq!(
            plan(&[spin]).ops,
            vec![KernelOp::RadialBlur {
                amount: 30,
                quality: 5
            }]
        );
        let zoom = guid_effect(
            "zoomBlur",
            GUID_ZOOM_BLUR,
            &[("zoom_blur_amount", "25"), ("zoom_blur_quality", "5")],
        );
        assert_eq!(
            plan(&[zoom]).ops,
            vec![KernelOp::ZoomBlur {
                amount: 25,
                quality: 5
            }]
        );
    }

    #[test]
    fn cs6_decimal_strings_parse_and_clamp() {
        let stack = [guid_effect(
            "gaussianBlur",
            GUID_GAUSSIAN_BLUR,
            &[("gaussian_blur_radius", "5.000000")],
        )];
        assert_eq!(
            plan(&stack).ops,
            vec![KernelOp::GaussianBlur { radius: 5.0 }]
        );
        let over = [guid_effect(
            "gb",
            GUID_GAUSSIAN_BLUR,
            &[("gaussian_blur_radius", "99999")],
        )];
        assert_eq!(
            plan(&over).ops,
            vec![KernelOp::GaussianBlur { radius: 250.0 }]
        );
    }

    #[test]
    fn noise_seed_is_stable_and_param_sensitive() {
        let a = effect("addNoise", &[("add_noise_amount", "20")]);
        let b = effect("addNoise", &[("add_noise_amount", "21")]);
        let seed = |effect: &Effect| match plan(std::slice::from_ref(effect)).ops[0].clone() {
            KernelOp::AddNoise { seed, .. } => seed,
            _ => panic!("wrong op"),
        };
        assert_eq!(seed(&a), seed(&a), "same effect → same seed");
        assert_ne!(seed(&a), seed(&b), "parameters change the seed");
    }

    #[test]
    fn raw_values_never_panic_the_parser() {
        let raw = EffectProperty {
            key: "rgb_points".to_owned(),
            value: EffectValue::Raw(Node {
                fourcc: *b"GPLv",
                value: NodeValue::Children(Vec::new()),
            }),
        };
        let stack = [Effect {
            category: String::new(),
            internal_name: "curves".to_owned(),
            properties: vec![
                raw,
                EffectProperty {
                    key: "EffectMoaID".to_owned(),
                    value: EffectValue::Text(format!("{{{GUID_LEVELS}}}")),
                },
            ],
        }];
        // A levels identity whose parameters are raw nodes: skipped,
        // not crashed.
        let plan = plan(&stack);
        assert!(plan.ops.is_empty());
        assert_eq!(plan.skipped, 1);
    }

    #[test]
    fn stack_order_is_preserved() {
        let stack = [
            guid_effect("invert", GUID_INVERT, &[]),
            guid_effect(
                "brightnessContrast",
                GUID_BRIGHTNESS_CONTRAST,
                &[("brightness_amount", "10"), ("contrast_amount", "0")],
            ),
            guid_effect("blur", GUID_BLUR, &[]),
        ];
        let plan = plan(&stack);
        assert_eq!(plan.ops.len(), 3);
        assert!(matches!(plan.ops[0], KernelOp::Invert));
        assert!(matches!(
            plan.ops[1],
            KernelOp::BrightnessContrast {
                brightness: 10,
                contrast: 0
            }
        ));
        assert!(matches!(plan.ops[2], KernelOp::BoxBlur { radius: 1 }));
    }

    #[test]
    fn apply_stack_matches_direct_kernel_calls() {
        let width = 8u32;
        let height = 8u32;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        for (i, chunk) in pixels.chunks_exact_mut(4).enumerate() {
            chunk[0] = (i * 7 % 256) as u8;
            chunk[1] = (i * 13 % 256) as u8;
            chunk[2] = (i * 29 % 256) as u8;
            chunk[3] = 255;
        }
        let surface =
            Surface::from_premultiplied(width, height, pixels.clone()).expect("square size");
        let stack = [
            KernelOp::BrightnessContrast {
                brightness: 20,
                contrast: -10,
            },
            KernelOp::Invert,
        ];
        let via_stack = apply_stack(surface, &stack);
        let direct = filters::invert(&filters::brightness_contrast(
            &Surface::from_premultiplied(width, height, pixels).expect("square size"),
            20,
            -10,
        ));
        assert_eq!(via_stack.into_pixels(), direct.into_pixels());
    }

    #[test]
    fn apply_stack_drives_every_kernel() {
        // Every KernelOp variant passes through the stack once on a
        // tiny opaque surface and produces opaque, same-sized pixels —
        // the dispatch arms exist and no op panics or collapses alpha.
        let width = 6u32;
        let height = 6u32;
        let pixels = vec![128u8; (width * height * 4) as usize];
        let color = color::Rgba8::new(255, 0, 0, 255);
        let knots = filters::CurvesKnots {
            rgb: vec![(0.0, 0.0), (1.0, 1.0)],
            red: Vec::new(),
            green: Vec::new(),
            blue: Vec::new(),
        };
        let stack = vec![
            KernelOp::BrightnessContrast {
                brightness: 5,
                contrast: 5,
            },
            KernelOp::Levels {
                master: filters::LevelsControls {
                    in_black: 0,
                    in_white: 255,
                    gamma: 1.0,
                    out_black: 0,
                    out_white: 255,
                },
                red: Some(filters::LevelsControls {
                    in_black: 0,
                    in_white: 255,
                    gamma: 1.0,
                    out_black: 0,
                    out_white: 255,
                }),
                green: None,
                blue: None,
            },
            KernelOp::Invert,
            KernelOp::HueSaturation {
                hue: 10.0,
                saturation: 0.1,
                lightness: 0.0,
                colorize: false,
            },
            KernelOp::Clouds {
                seed: 1,
                sky: color,
                cloud: color,
            },
            KernelOp::DifferenceClouds {
                seed: 1,
                sky: color,
                cloud: color,
            },
            KernelOp::GaussianBlur { radius: 0.5 },
            KernelOp::BoxBlur { radius: 1 },
            KernelOp::UnsharpMask {
                radius: 1.0,
                amount: 50,
                threshold: 0,
            },
            KernelOp::Curves { knots },
            KernelOp::AutoLevels,
            KernelOp::ColorFill {
                fill: [10, 20, 30],
                opacity: 25,
                mode: BlendMode::Normal,
            },
            KernelOp::MotionBlur {
                angle_degrees: 0.0,
                distance: 1,
                taps: 2,
            },
            KernelOp::RadialBlur {
                amount: 10,
                quality: 4,
            },
            KernelOp::ZoomBlur {
                amount: 10,
                quality: 4,
            },
            KernelOp::Convolve {
                width: 3,
                height: 3,
                values: vec![0, 0, 0, 0, 1, 0, 0, 0, 0],
                affects_alpha: false,
            },
            KernelOp::Sharpen,
            KernelOp::SharpenMore,
            KernelOp::ConvertToAlpha,
            KernelOp::AddNoise {
                seed: 7,
                amount: 10,
                monochrome: false,
            },
            KernelOp::FindEdges,
            KernelOp::DropShadow {
                color,
                angle_degrees: 90.0,
                distance: 1,
                blur: 1,
                knockout: false,
            },
            KernelOp::InnerShadow {
                color,
                angle_degrees: 90.0,
                distance: 1,
                blur: 1,
            },
            KernelOp::SolidShadow {
                color,
                show_object: true,
            },
            KernelOp::OuterGlow {
                color,
                blur: 1,
                edge_threshold: 128,
            },
            KernelOp::InnerGlow {
                color,
                blur: 1,
                edge_threshold: 128,
            },
            KernelOp::Bevel {
                style: filters::BevelStyle::InnerBevel,
                angle_degrees: 120.0,
                width: 2,
                contrast: 50,
                depth: 100,
                start: 2,
                hilite: color,
                shadow: color,
                face: None,
            },
        ];
        let out = apply_stack(
            Surface::from_premultiplied(width, height, pixels).expect("square size"),
            &stack,
        );
        assert_eq!(out.into_pixels().len(), (width * height * 4) as usize);
    }

    #[test]
    fn mx_shadow_scalars_parse_from_dictionary_props() {
        // Drop Shadow: color alpha, angle, blur, distance; ShadowType 1
        // is the knockout variant.
        let shadow = guid_effect(
            "dropShadow",
            GUID_DROP_SHADOW,
            &[
                ("ShadowColor", "#ff000080"),
                ("ShadowAngle", "90"),
                ("ShadowBlur", "4"),
                ("ShadowDistance", "7"),
            ],
        );
        assert_eq!(
            plan(std::slice::from_ref(&shadow)).ops,
            vec![KernelOp::DropShadow {
                color: Rgba8::new(255, 0, 0, 128),
                angle_degrees: 90.0,
                distance: 7,
                blur: 4,
                knockout: false,
            }]
        );
        let mut knockout = shadow.clone();
        knockout.properties.push(EffectProperty {
            key: "ShadowType".to_owned(),
            value: EffectValue::Text("1".to_owned()),
        });
        assert!(matches!(
            plan(&[knockout]).ops[0],
            KernelOp::DropShadow { knockout: true, .. }
        ));
        // Inner shadow shares the scalar shape (no knockout).
        let inner = guid_effect(
            "innerShadow",
            GUID_INNER_SHADOW,
            &[
                ("ShadowColor", "#000000"),
                ("ShadowAngle", "-135"),
                ("ShadowBlur", "2"),
                ("ShadowDistance", "3"),
            ],
        );
        assert_eq!(
            plan(&[inner]).ops,
            vec![KernelOp::InnerShadow {
                color: Rgba8::new(0, 0, 0, 255),
                angle_degrees: 225.0,
                distance: 3,
                blur: 2,
            }]
        );
    }

    #[test]
    fn solid_shadow_show_object_toggles_replacement() {
        let shown = guid_effect(
            "solidShadow",
            GUID_SOLID_SHADOW,
            &[("ShadowColor", "#804000"), ("ShowObject", "true")],
        );
        let hidden = guid_effect(
            "solidShadow",
            GUID_SOLID_SHADOW,
            &[("ShadowColor", "#804000"), ("ShowObject", "0")],
        );
        let absent = guid_effect(
            "solidShadow",
            GUID_SOLID_SHADOW,
            &[("ShadowColor", "#804000")],
        );
        assert!(matches!(
            plan(std::slice::from_ref(&shown)).ops[0],
            KernelOp::SolidShadow {
                show_object: true,
                ..
            }
        ));
        assert!(matches!(
            plan(std::slice::from_ref(&hidden)).ops[0],
            KernelOp::SolidShadow {
                show_object: false,
                ..
            }
        ));
        // Absent means the object stays (the dictionary's default).
        assert_eq!(
            plan(std::slice::from_ref(&absent)).ops,
            plan(std::slice::from_ref(&shown)).ops
        );
    }

    #[test]
    fn inner_glow_threshold_scales_to_alpha_units() {
        let glow = guid_effect(
            "innerGlow",
            GUID_INNER_GLOW,
            &[
                ("ShadowColor", "#ffcc00"),
                ("GlowWidth", "3"),
                ("EdgeThreshold", "0.5"),
            ],
        );
        assert_eq!(
            plan(&[glow]).ops,
            vec![KernelOp::InnerGlow {
                color: Rgba8::new(255, 204, 0, 255),
                blur: 3,
                edge_threshold: 128,
            }]
        );
        // The threshold is optional (the dictionary records Glow at 1).
        let bare = guid_effect(
            "innerGlow",
            GUID_INNER_GLOW,
            &[("ShadowColor", "#ffcc00"), ("GlowWidth", "3")],
        );
        assert!(matches!(
            plan(&[bare]).ops[0],
            KernelOp::InnerGlow {
                edge_threshold: 255,
                ..
            }
        ));
    }

    #[test]
    fn mx_bevel_parses_per_type() {
        // Inner bevel (BevelType 0): width/contrast/angle/colors;
        // defaults fill the absent contrast and depth.
        let inner = guid_effect(
            "innerBevel",
            GUID_BEVEL,
            &[
                ("BevelType", "0"),
                ("BevelWidth", "6"),
                ("BevelContrast", "70"),
                ("LightAngle", "120"),
                ("LightDistance", "150"),
                ("HiliteColor", "#ffffcc"),
                ("OuterBevelColor", "#808000"),
            ],
        );
        assert_eq!(
            plan(std::slice::from_ref(&inner)).ops,
            vec![KernelOp::Bevel {
                style: filters::BevelStyle::InnerBevel,
                angle_degrees: 120.0,
                width: 6,
                contrast: 70,
                depth: 150,
                start: 0,
                hilite: Rgba8::new(255, 255, 204, 255),
                shadow: Rgba8::new(128, 128, 0, 255),
                face: None,
            }]
        );
        // Raised emboss (2) carries the face color; a negative angle
        // normalizes like the shadow family.
        let raised = guid_effect(
            "raisedEmboss",
            GUID_BEVEL,
            &[
                ("BevelType", "2"),
                ("BevelWidth", "4"),
                ("LightAngle", "-135"),
                ("HiliteColor", "#ffffff"),
                ("OuterBevelColor", "#000000"),
                ("EmbossFaceColor", "#4080c0ff"),
            ],
        );
        assert_eq!(
            plan(&[raised]).ops,
            vec![KernelOp::Bevel {
                style: filters::BevelStyle::RaisedEmboss,
                angle_degrees: 225.0,
                width: 4,
                contrast: 100,
                depth: 100,
                start: 0,
                hilite: Rgba8::new(255, 255, 255, 255),
                shadow: Rgba8::new(0, 0, 0, 255),
                face: Some(Rgba8::new(64, 128, 192, 255)),
            }]
        );
        // The glow variant reads GlowWidth + GlowStartDistance.
        let glow = guid_effect(
            "glow",
            GUID_BEVEL,
            &[
                ("BevelType", "4"),
                ("GlowWidth", "8"),
                ("GlowStartDistance", "3"),
                ("LightAngle", "90"),
                ("HiliteColor", "#ffffff"),
                ("OuterBevelColor", "#000000"),
            ],
        );
        assert!(matches!(
            plan(&[glow]).ops[0],
            KernelOp::Bevel {
                style: filters::BevelStyle::Glow,
                width: 8,
                start: 3,
                ..
            }
        ));
        // An unknown BevelType and a missing BevelWidth both skip.
        let unknown = guid_effect(
            "bevel",
            GUID_BEVEL,
            &[
                ("BevelType", "9"),
                ("BevelWidth", "4"),
                ("LightAngle", "0"),
                ("HiliteColor", "#ffffff"),
                ("OuterBevelColor", "#000000"),
            ],
        );
        let starved = guid_effect(
            "bevel",
            GUID_BEVEL,
            &[
                ("BevelType", "0"),
                ("LightAngle", "0"),
                ("HiliteColor", "#ffffff"),
                ("OuterBevelColor", "#000000"),
            ],
        );
        let planned = plan(&[unknown, starved]);
        assert!(planned.ops.is_empty());
        assert_eq!(planned.skipped, 2);
    }

    #[test]
    fn packed_container_parses_kinds_and_skips_whole_on_exotics() {
        // dropShadow + outerGlow + bevelEmboss + solidFill in one
        // container: four ops, the dictionary's own example strings.
        let mut live = guid_effect("liveEffects", GUID_LIVE_EFFECTS, &[]);
        let packed = [
            (
                "dropShadow",
                "0&75.000000&0&120.000000&0.000000&5.000000&5.000000&74.000000&0&1&multiply&RGBColor;0.000000;0.000000;0.000000&2$$0.000000;0.000000;1$255.000000;255.000000;1",
            ),
            (
                "outerGlow",
                "0&0&screen&75.000000:0.000000:5.000000:0.000000:0.000000:50.000000&softMatte&RGBColor;255.000000;255.000000;189.996109&nogradient&2$$0.000000;0.000000;1$255.000000;255.000000;1&edgeGlow",
            ),
            (
                "bevelEmboss",
                "0&0&0&0&0&75.000000&75.000000&120.000000&30.000000&100.000000&5.000000&0.000000&screen&RGBColor;255.000000;255.000000;255.000000&multiply&RGBColor;0.000000;0.000000;0.000000&softMatte&innerBevel&stampIn&2$$0.000000;0.000000;1$255.000000;255.000000;1&noshape&notexture",
            ),
            (
                "solidFill",
                "0&50.000000&normal&RGBColor;51.000000;102.000000;153.000000",
            ),
        ];
        for (key, text) in packed {
            live.properties.push(EffectProperty {
                key: key.to_owned(),
                value: EffectValue::Text(text.to_owned()),
            });
        }
        let ops = plan(&[live.clone()]).ops;
        assert_eq!(ops.len(), 4, "four packed kinds → four ops");
        // Packed opacity (75) fades the color; distance rides the
        // field the dictionary labels "always 0" but its example
        // carries 74 in.
        assert_eq!(
            ops[0],
            KernelOp::DropShadow {
                color: Rgba8::new(0, 0, 0, 191),
                angle_degrees: 120.0,
                distance: 74,
                blur: 5,
                knockout: false,
            }
        );
        assert_eq!(
            ops[1],
            KernelOp::OuterGlow {
                color: Rgba8::new(255, 255, 190, 191),
                blur: 5,
                edge_threshold: 0,
            }
        );
        // The packed bevel lands between the glow and the fill (the
        // parser reads kinds in the panel's own order, not property
        // order).
        assert_eq!(
            ops[2],
            KernelOp::Bevel {
                style: filters::BevelStyle::InnerBevel,
                angle_degrees: 120.0,
                width: 5,
                contrast: 100,
                depth: 100,
                start: 0,
                hilite: Rgba8::new(255, 255, 255, 255),
                shadow: Rgba8::new(0, 0, 0, 255),
                face: None,
            }
        );
        // solidFill is the one packed kind whose blend is
        // representable — it lands on the existing ColorFill op.
        assert_eq!(
            ops[3],
            KernelOp::ColorFill {
                fill: [51, 102, 153],
                opacity: 50,
                mode: BlendMode::Normal,
            }
        );
        // Any uninterpretable kind (satin, the fills, stroke, a style
        // token beyond the four we map) skips the WHOLE container —
        // never half a stack.
        for exotic in [
            ("satin", "0&0"),
            ("gradientFill", "0&0"),
            ("patternFill", "0&0"),
            ("stroke", "0&0"),
            (
                "bevelEmboss",
                "0&0&0&0&0&75.000000&75.000000&120.000000&30.000000&100.000000&5.000000&0.000000&screen&RGBColor;255.000000;255.000000;255.000000&multiply&RGBColor;0.000000;0.000000;0.000000&softMatte&strokeEmboss&stampIn&2$$0.000000;0.000000;1$255.000000;255.000000;1&noshape&notexture",
            ),
        ] {
            let mut guarded = live.clone();
            // An exotic REPLACES its same-key kind (the container
            // carries one packed string per kind).
            guarded.properties.retain(|p| p.key != exotic.0);
            guarded.properties.push(EffectProperty {
                key: exotic.0.to_owned(),
                value: EffectValue::Text(exotic.1.to_owned()),
            });
            let planned = plan(&[guarded]);
            assert!(
                planned.ops.is_empty(),
                "{} must skip the container",
                exotic.0
            );
            assert_eq!(planned.skipped, 1);
        }
        // A malformed packed string does the same (a misread is not a
        // shadow).
        let mut broken = guid_effect("liveEffects", GUID_LIVE_EFFECTS, &[]);
        broken.properties.push(EffectProperty {
            key: "innerGlow".to_owned(),
            value: EffectValue::Text("not&numbers".to_owned()),
        });
        let planned = plan(&[broken]);
        assert!(planned.ops.is_empty());
        assert_eq!(planned.skipped, 1);
    }

    #[test]
    fn prop_color_reads_six_and_eight_digit_hex() {
        let six = guid_effect(
            "solidShadow",
            GUID_SOLID_SHADOW,
            &[("ShadowColor", "#804000")],
        );
        assert!(matches!(
            plan(&[six]).ops[0],
            KernelOp::SolidShadow {
                color: Rgba8 {
                    r: 128,
                    g: 64,
                    b: 0,
                    a: 255
                },
                ..
            }
        ));
        let eight = guid_effect(
            "solidShadow",
            GUID_SOLID_SHADOW,
            &[("ShadowColor", "#10203040")],
        );
        assert!(matches!(
            plan(&[eight]).ops[0],
            KernelOp::SolidShadow {
                color: Rgba8 {
                    r: 16,
                    g: 32,
                    b: 48,
                    a: 64
                },
                ..
            }
        ));
    }

    /// The dictionary's full per-channel Levels prop set for one
    /// channel: `master` rides `*_rgb`, overrides ride `*_red` etc.
    fn levels_props(suffix: String, in_white: &str) -> Vec<(String, String)> {
        vec![
            (format!("source_low_{suffix}"), "0".to_owned()),
            (format!("source_high_{suffix}"), in_white.to_owned()),
            (format!("gamma_{suffix}"), "1".to_owned()),
            (format!("dest_low_{suffix}"), "0".to_owned()),
            (format!("dest_high_{suffix}"), "255".to_owned()),
        ]
    }

    #[test]
    fn levels_channel_overrides_replace_only_their_channel() {
        let mut props: Vec<EffectProperty> = levels_props("rgb".to_owned(), "255")
            .into_iter()
            .map(|(key, value)| EffectProperty {
                key,
                value: EffectValue::Text(value),
            })
            .collect();
        for (key, value) in levels_props("red".to_owned(), "128") {
            props.push(EffectProperty {
                key,
                value: EffectValue::Text(value),
            });
        }
        let parsed = Effect {
            category: String::new(),
            internal_name: "levels".to_owned(),
            properties: props,
        };
        let KernelOp::Levels {
            master,
            red,
            green,
            blue,
        } = plan(std::slice::from_ref(&parsed)).ops[0]
        else {
            panic!("expected levels");
        };
        assert_eq!(red.map(|c| c.in_white), Some(128), "red override rides");
        assert_eq!(green.map(|c| c.in_white), None, "green has no override");
        assert_eq!(blue.map(|c| c.in_white), None, "blue has no override");
        assert_eq!(master.in_white, 255);
    }

    #[test]
    fn levels_partial_channel_group_is_not_an_override() {
        // A UI mid-edit state (two of five props) must not silently
        // merge with the master — the channel falls back to it.
        let mut props: Vec<EffectProperty> = levels_props("rgb".to_owned(), "255")
            .into_iter()
            .map(|(key, value)| EffectProperty {
                key,
                value: EffectValue::Text(value),
            })
            .collect();
        for (key, value) in [("source_low_red", "10"), ("source_high_red", "200")] {
            props.push(EffectProperty {
                key: key.to_owned(),
                value: EffectValue::Text(value.to_owned()),
            });
        }
        let parsed = Effect {
            category: String::new(),
            internal_name: "levels".to_owned(),
            properties: props,
        };
        let KernelOp::Levels { red, .. } = plan(std::slice::from_ref(&parsed)).ops[0] else {
            panic!("expected levels");
        };
        assert_eq!(red, None, "partial group is not a spec");
    }

    #[test]
    fn hls_colorize_switches_shift_to_replace() {
        let shift = guid_effect(
            "hueSaturation",
            GUID_HUE_SATURATION,
            &[
                ("hue_amount", "180"),
                ("saturation_amount", "0"),
                ("lightness_amount", "0"),
            ],
        );
        assert!(matches!(
            plan(&[shift]).ops[0],
            KernelOp::HueSaturation {
                hue: 180.0,
                colorize: false,
                ..
            }
        ));
        let colorize = guid_effect(
            "hueSaturation",
            GUID_HUE_SATURATION,
            &[
                ("hue_amount", "120"),
                ("saturation_amount", "0"),
                ("lightness_amount", "0"),
                ("hls_colorize", "1"),
            ],
        );
        assert!(matches!(
            plan(&[colorize]).ops[0],
            KernelOp::HueSaturation {
                hue: 120.0,
                colorize: true,
                ..
            }
        ));
    }

    #[test]
    fn clouds_parse_seed_colors_and_difference_split() {
        let clouds_effect = guid_effect(
            "clouds",
            GUID_CLOUDS,
            &[("sky_color", "#6699cc"), ("cloud_color", "#ffffff")],
        );
        let plain = plan(std::slice::from_ref(&clouds_effect));
        assert_eq!(plain.ops.len(), 1, "no params missing, nothing skipped");
        assert!(matches!(plain.ops[0], KernelOp::Clouds { .. }));

        // Braced GUID spellings normalize to the same identity.
        let difference = Effect {
            category: String::new(),
            internal_name: "differenceClouds".to_owned(),
            properties: vec![
                EffectProperty {
                    key: "EffectMoaID".to_owned(),
                    value: EffectValue::Text(format!("{{{GUID_DIFFERENCE_CLOUDS}}}")),
                },
                EffectProperty {
                    key: "sky_color".to_owned(),
                    value: EffectValue::Text("#000000".to_owned()),
                },
                EffectProperty {
                    key: "cloud_color".to_owned(),
                    value: EffectValue::Text("#ffffff".to_owned()),
                },
            ],
        };
        assert!(matches!(
            plan(std::slice::from_ref(&difference)).ops[0],
            KernelOp::DifferenceClouds { .. }
        ));

        // Same params, different internal names → different seeds: the
        // seed feeds from the effect's own bytes, so the two renders
        // are distinct textures.
        let a = guid_effect(
            "clouds",
            GUID_CLOUDS,
            &[("sky_color", "#000000"), ("cloud_color", "#ffffff")],
        );
        let b = guid_effect(
            "differenceClouds",
            GUID_DIFFERENCE_CLOUDS,
            &[("sky_color", "#000000"), ("cloud_color", "#ffffff")],
        );
        let (KernelOp::Clouds { seed: sa, .. }, KernelOp::DifferenceClouds { seed: sb, .. }) =
            (plan(&[a]).ops[0].clone(), plan(&[b]).ops[0].clone())
        else {
            panic!("expected clouds pair");
        };
        assert_ne!(sa, sb, "seed derives from the effect bytes");
    }

    #[test]
    fn clouds_missing_color_param_skips() {
        let bare = guid_effect("clouds", GUID_CLOUDS, &[("sky_color", "#6699cc")]);
        let planned = plan(&[bare]);
        assert!(planned.ops.is_empty());
        assert_eq!(planned.skipped, 1);
    }

    #[test]
    fn clouds_guid_stack_applies_through_the_kernels() {
        // End-to-end through `apply_stack`: Clouds must fill the
        // object's opaque box with the pinned texture (deterministic,
        // sky↔cloud-bounded), Difference Clouds must keep the box.
        let fill = guid_effect(
            "clouds",
            GUID_CLOUDS,
            &[("sky_color", "#000000"), ("cloud_color", "#ffffff")],
        );
        let plan = plan(&[fill]);
        let base =
            filters::Surface::from_straight(8, 8, [255, 0, 0, 255].repeat(64).as_slice()).unwrap();
        let out = apply_stack(base, &plan.ops);
        let px = out.to_straight();
        assert_eq!(px.len(), 8 * 8 * 4);
        assert!(
            px.chunks_exact(4).all(|p| p[3] == 255),
            "clouds fills the box opaquely"
        );
        assert!(
            px.chunks_exact(4).all(|p| p[0] == p[1] && p[1] == p[2]),
            "gray sky↔cloud keeps the texture gray"
        );
    }

    #[test]
    fn the_filter_catalog_stays_inside_the_recognized_set() {
        for (category, name, guid) in FILTER_CATALOG {
            assert!(
                RECOGNIZED_GUIDS.contains(guid),
                "catalog entry {category}/{name} names {guid}, which no kernel interprets"
            );
        }
    }
}
