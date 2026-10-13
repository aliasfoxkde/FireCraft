//! Live-effect interpretation: `doc::Effect` stacks → `filters` kernels.
//!
//! Identity is the `EffectMoaID` GUID (the dictionary's `DCEv
//! "EffectMoaID"` table); stacks written without a GUID — synthetic
//! `filter.add` payloads — fall back to the internal name (`INMs`).
//! Every native CS6 live effect in the M14 enumeration
//! (`docs/research/fw-code-notes.md`) is interpreted here; effects
//! outside it — the Photoshop Live Effects container, third-party
//! plugin GUIDs, Color Fill's exotic blend codes, shadow/glow/emboss
//! until the composite pass — are skipped, never approximated: the
//! object renders unfiltered through them. That keeps composition
//! total and deterministic (the ADR 0002 oracle contract); widening
//! the catalog is strictly additive.
//!
//! Properties are the `EPSv` decimal strings. A required parameter
//! that is absent or unparseable skips its effect (counted in
//! [`Plan::skipped`]) — CS6 writes every parameter of an effect it
//! saved, so absence means a stack we would otherwise be guessing at.

use color::BlendMode;
use doc::{Effect, EffectValue, NodeValue};
use filters::{CurvesKnots, Surface};

/// Brightness/Contrast (`EffectMoaID`, dictionary line 1497).
const GUID_BRIGHTNESS_CONTRAST: &str = "3439b08c-1921-11d3-9bde00e02910d580";
/// Hue/Saturation (dictionary line 1498).
const GUID_HUE_SATURATION: &str = "3439b08d-1922-11d3-9bde00e02910d580";
/// Gaussian Blur (dictionary line 1508).
const GUID_GAUSSIAN_BLUR: &str = "d04ef8c0-71b3-11d1-8c8200a024cdc039";
/// Levels (dictionary line 1510; the `_rgb` master controls only).
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
    /// Levels, five-control Photoshop form, master (`_rgb`) channels.
    Levels {
        /// `source_low_rgb`.
        in_black: u8,
        /// `source_high_rgb`.
        in_white: u8,
        /// `gamma_rgb` (1.0 takes the integer-exact kernel path).
        gamma: f64,
        /// `dest_low_rgb`.
        out_black: u8,
        /// `dest_high_rgb`.
        out_white: u8,
    },
    /// Invert — alpha passes through.
    Invert,
    /// Hue/Saturation: degrees, then −100..=100 percents (kernel takes
    /// fractions, so percents divide by 100 here).
    HueSaturation {
        /// `hue_amount`.
        hue: f64,
        /// `saturation_amount` / 100.
        saturation: f64,
        /// `lightness_amount` / 100.
        lightness: f64,
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
    /// Sharpen — fixed preset, no parameters.
    Sharpen,
    /// Sharpen More — fixed preset, no parameters.
    SharpenMore,
    /// Convert to Alpha — luma-to-opacity stencil, no parameters.
    ConvertToAlpha,
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
                in_black,
                in_white,
                gamma,
                out_black,
                out_white,
            } => filters::levels(
                &surface, *in_black, *in_white, *gamma, *out_black, *out_white,
            ),
            KernelOp::Invert => filters::invert(&surface),
            KernelOp::HueSaturation {
                hue,
                saturation,
                lightness,
            } => filters::hsl(&surface, *hue, *saturation, *lightness),
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
            } => filters::motion_blur(&surface, *angle_degrees, *distance),
            KernelOp::RadialBlur { amount, quality } => {
                filters::radial_blur_spin(&surface, *amount, *quality)
            }
            KernelOp::ZoomBlur { amount, quality } => {
                filters::zoom_blur(&surface, *amount, *quality)
            }
            KernelOp::Sharpen => filters::sharpen(&surface),
            KernelOp::SharpenMore => filters::sharpen_more(&surface),
            KernelOp::ConvertToAlpha => filters::convert_to_alpha(&surface),
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

fn motion_blur(effect: &Effect) -> Option<KernelOp> {
    let angle = prop_f64(effect, "motion_blur_angle")?;
    let distance = prop_f64(effect, "motion_blur_distance")?;
    if distance < 1.0 {
        return None;
    }
    Some(KernelOp::MotionBlur {
        angle_degrees: angle.clamp(0.0, 360.0),
        distance: distance.clamp(1.0, 100.0) as u32,
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
    Some(KernelOp::HueSaturation {
        hue: hue.clamp(-180.0, 180.0),
        saturation: saturation.clamp(-100.0, 100.0) / 100.0,
        lightness: lightness.clamp(-100.0, 100.0) / 100.0,
    })
}

fn levels(effect: &Effect) -> Option<KernelOp> {
    Some(KernelOp::Levels {
        in_black: prop_u8(effect, "source_low_rgb")?,
        in_white: prop_u8(effect, "source_high_rgb")?,
        gamma: prop_f64(effect, "gamma_rgb")?,
        out_black: prop_u8(effect, "dest_low_rgb")?,
        out_white: prop_u8(effect, "dest_high_rgb")?,
    })
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
    let use_color = match prop(effect, "add_noise_use_color") {
        Some(EffectValue::Text(text)) => {
            let text = text.trim().to_ascii_lowercase();
            text == "1" || text == "true"
        }
        Some(EffectValue::Raw(_)) | None => false,
    };
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
        ] {
            stack.push(guid_effect(name, guid, &props));
        }
        let plan = plan(&stack);
        assert_eq!(plan.ops.len(), 19, "every enumerated effect interprets");
        assert_eq!(plan.skipped, 0, "the native surface never rides through");
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
                distance: 10
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
}
