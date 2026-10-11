//! Live-effect interpretation: `doc::Effect` stacks → `filters` kernels.
//!
//! Identity is the `EffectMoaID` GUID (the dictionary's `DCEv
//! "EffectMoaID"` table); stacks written without a GUID — synthetic
//! `filter.add` payloads — fall back to the internal name (`INMs`).
//! Effects the kernel catalog cannot interpret — shadows, glows,
//! bevels, per-channel levels, curves with raw grab points,
//! motion/radial/zoom blur — are skipped, never approximated: the
//! object renders unfiltered through them. That keeps composition
//! total and deterministic (the ADR 0002 oracle contract); widening
//! the catalog is strictly additive.
//!
//! Properties are the `EPSv` decimal strings. A required parameter
//! that is absent or unparseable skips its effect (counted in
//! [`Plan::skipped`]) — CS6 writes every parameter of an effect it
//! saved, so absence means a stack we would otherwise be guessing at.

use doc::{Effect, EffectValue};
use filters::Surface;

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
    /// Unsharp Mask; a non-zero threshold skips the effect (the kernel
    /// has no threshold step — recorded limitation).
    UnsharpMask {
        /// `unsharp_mask_radius`.
        radius: f64,
        /// `unsharp_mask_amount`, 0..=500.
        amount: u32,
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
            KernelOp::UnsharpMask { radius, amount } => {
                filters::unsharp_mask(&surface, *radius, *amount)
            }
            KernelOp::AddNoise {
                seed,
                amount,
                monochrome,
            } => filters::add_noise(&surface, *seed, *amount, *monochrome),
            KernelOp::FindEdges => filters::find_edges(&surface),
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
        _ => None,
    }
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
    // The kernel has no threshold step; a threshold other than zero
    // would silently change meaning, so the effect is skipped instead.
    if let Some(EffectValue::Text(threshold)) = prop(effect, "unsharp_mask_threshold") {
        if threshold.trim().parse::<f64>().is_ok_and(|t| t != 0.0) {
            return None;
        }
    }
    let radius = prop_f64(effect, "unsharp_mask_radius")?;
    let amount = prop_f64(effect, "unsharp_mask_amount")?;
    if radius <= 0.0 {
        return None;
    }
    Some(KernelOp::UnsharpMask {
        radius: radius.clamp(0.1, 250.0),
        amount: amount.clamp(0.0, 500.0) as u32,
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
    fn unsharp_with_threshold_is_uninterpretable() {
        let kept = guid_effect(
            "unsharpMask",
            GUID_UNSHARP_MASK,
            &[
                ("unsharp_mask_radius", "2"),
                ("unsharp_mask_amount", "80"),
                ("unsharp_mask_threshold", "0"),
            ],
        );
        let skipped = guid_effect(
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
            KernelOp::UnsharpMask { amount: 80, .. }
        ));
        let plan = plan(&[skipped]);
        assert!(plan.ops.is_empty());
        assert_eq!(plan.skipped, 1);
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
