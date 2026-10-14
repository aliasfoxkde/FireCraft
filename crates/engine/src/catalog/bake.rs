//! The filter bake family (decision 0040): CS6's
//! `filterSelection` / `filterSelectionByID` / `filterSelectionByName`
//! verbs — "applies the specified pixel filter to the selection as a
//! permanent action, not as a Live Effect".
//!
//! All three front doors resolve to the same core: build an `Effect`,
//! interpret it through compose's kernel planner (the same interpreter
//! the Live Effect stack and the flatten bakes use), run it over each
//! selected image's assembled surface, and re-tile. The active pixel
//! selection (`document.selection`) gates the result per pixel; CS6's
//! scripts split on `getPixelMask()` exactly this way. Non-image
//! members refuse whole-command — CS6 would rasterize them first,
//! which is the recorded path→image debt. Every target resolution,
//! identity check, and tile assembly happens before the first
//! mutation (the 000557d ordering law).

use compose::Surface;
use doc::{Document, Effect, EffectProperty, EffectValue, ImageObject, ObjectData, SelectionMask};

use super::filter::{FilterParamPayload, build_effect};
use crate::access;
use crate::bitmap::{assemble_rgba, retile_rgba};
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the bake commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(BakeSelection)?;
    registry.register(BakeSelectionByID)?;
    registry.register(BakeSelectionByName)
}

/// The bake payload's parameter list — the same encoding `filter.add`
/// speaks (text xor knot list).
pub(crate) type BakeParams = Vec<FilterParamPayload>;

/// Builds the bake effect for the shared core. `guid`, when given, is
/// written as the (normalized) `EffectMoaID` property; braces and case
/// are stripped to compose's canonical form.
fn guid_effect(
    name: &'static str,
    category: String,
    internal_name: String,
    guid: Option<&str>,
    params: &[FilterParamPayload],
) -> Result<Effect, EngineError> {
    let mut effect = build_effect(name, category, internal_name, params)?;
    if let Some(guid) = guid {
        let normalized: String = guid
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .to_ascii_lowercase();
        effect.properties.insert(
            0,
            EffectProperty {
                key: "EffectMoaID".to_owned(),
                value: EffectValue::Text(normalized),
            },
        );
    }
    Ok(effect)
}

/// Resolves `(category, name)` through compose's filter catalog. The
/// CS6 scripts author menu names with a dialog ellipsis
/// ("Hue/Saturation..."), so trailing dots are ignored on lookup.
fn catalog_guid(
    name: &'static str,
    category: &str,
    filter_name: &str,
) -> Result<&'static str, EngineError> {
    let trimmed = filter_name.trim_end_matches('.');
    for (cat, nm, guid) in compose::FILTER_CATALOG {
        if *cat == category && *nm == trimmed {
            return Ok(*guid);
        }
    }
    Err(EngineError::InvalidState {
        name: name.to_owned(),
        detail: format!("no pixel filter named {category}/{trimmed} is installed"),
    })
}

/// The shared bake: interpret the effect, then transform every image
/// in the object selection, gated by the active pixel mask.
fn bake_core(
    document: &mut Document,
    name: &'static str,
    effect: Effect,
) -> Result<Delta, EngineError> {
    if document.object_selection.is_empty() {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "nothing is selected".to_owned(),
        });
    }
    // Identity first: the plan must resolve the effect completely —
    // an unrecognized identity or a missing parameter skips, and the
    // bake refuses rather than applying nothing (the no-passthrough
    // floor law).
    let plan = compose::plan(std::slice::from_ref(&effect));
    if plan.skipped != 0 || plan.ops.is_empty() {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "the filter does not resolve to a pixel kernel (unknown identity or missing parameters)"
                .to_owned(),
        });
    }
    // Resolve every target before any mutation (000557d): existence,
    // image kind, and editability (assembly is read-only).
    let mut targets = Vec::with_capacity(document.object_selection.len());
    for id in document.object_selection.clone() {
        let before = access::object(document, name, id)?;
        match before {
            ObjectData::Image(image) => {
                let assembled = assemble_rgba(document, image, name)?;
                targets.push((id, before.clone(), assembled));
            }
            _ => {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: format!(
                        "object {} is not an image — CS6 would rasterize it first (recorded debt)",
                        id.index()
                    ),
                });
            }
        }
    }
    // Transform and re-tile each image, capturing per-object deltas in
    // mutation order; the pixel mask (canvas space) gates the result.
    let mask = document.selection.clone();
    let mut deltas = Vec::with_capacity(targets.len() + 1);
    let mut minted_all = Vec::new();
    for (id, before, assembled) in targets {
        let ObjectData::Image(image) = &before else {
            continue;
        };
        let (width, height) = image.pixels;
        let surface = Surface::from_premultiplied(
            usize::try_from(width).unwrap_or(0) as u32,
            usize::try_from(height).unwrap_or(0) as u32,
            assembled.clone(),
        )
        .map_err(|err| EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("image {} does not assemble: {err}", id.index()),
        })?;
        let filtered = compose::apply_stack(surface, &plan.ops);
        let mut pixels = filtered.pixels().to_vec();
        if let Some(mask) = &mask {
            blend_through_mask(&mut pixels, &assembled, width, height, image, mask);
        }
        let (tiles, minted) = retile_rgba(document, &pixels, width, height);
        minted_all.extend(minted);
        match document.registry.get_mut(id) {
            Some(ObjectData::Image(slot)) => slot.tiles = tiles,
            _ => {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: format!("image {} vanished mid-bake", id.index()),
                });
            }
        }
        deltas.push(Delta::Object {
            id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        });
    }
    deltas.push(Delta::Bank { added: minted_all });
    Ok(Delta::Group(deltas))
}

/// Blends `filtered` over `original` through the canvas-space pixel
/// mask, in image-local coordinates. Pixels outside the mask plane
/// stand unfiltered. The blend runs in the premultiplied domain the
/// tiles store: a weighted average of two premultiplied colors stays
/// premultiplied.
fn blend_through_mask(
    filtered: &mut [u8],
    original: &[u8],
    width: i64,
    height: i64,
    image: &ImageObject,
    mask: &SelectionMask,
) {
    let left = image.location.x.round() as i64;
    let top = image.location.y.round() as i64;
    for y in 0..height.max(0) {
        for x in 0..width.max(0) {
            let cx = left + x;
            let cy = top + y;
            if cx < 0 || cy < 0 {
                continue;
            }
            let (cx, cy) = (cx as u64, cy as u64);
            if cx >= u64::from(mask.width) || cy >= u64::from(mask.height) {
                continue;
            }
            let m = u16::from(mask.alpha[(cy as usize) * mask.width as usize + cx as usize]);
            if m == 255 {
                continue;
            }
            let index = ((y * width + x) * 4) as usize;
            for channel in 0..4 {
                let kept = u16::from(original[index + channel]);
                let baked = u16::from(filtered[index + channel]);
                filtered[index + channel] = ((kept * (255 - m) + baked * m + 127) / 255) as u8;
            }
        }
    }
}

/// `filter.selection` — CS6 `dom.filterSelection(LiveEffect)`: the
/// full effect form (internal name + parameters; a `EffectMoaID`
/// parameter rides along when authored).
struct BakeSelection;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct BakeSelectionPayload {
    /// Internal name (`INMs`), e.g. "invert".
    name: String,
    /// UI category (`CATs`); defaults to empty.
    #[serde(default)]
    category: String,
    /// `DCVs` parameters in list order.
    #[serde(default)]
    params: BakeParams,
}

impl TypedCommand for BakeSelection {
    type Payload = BakeSelectionPayload;

    fn name(&self) -> &'static str {
        "filter.selection"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "filter.selection";
        // The effect form carries the identity in its own properties —
        // the CS6 payloads pass `EffectMoaID` as a parameter, which
        // `build_effect` carries through verbatim; compose resolves it.
        let effect = guid_effect(
            NAME,
            payload.category.clone(),
            payload.name.clone(),
            None,
            &payload.params,
        )?;
        bake_core(document, NAME, effect)
    }
}

/// `filter.selectionByID` — CS6 `dom.filterSelectionByID(ID)`: the
/// filter's `EffectMoaID`.
struct BakeSelectionByID;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct BakeSelectionByIdPayload {
    /// The `EffectMoaID` GUID; braces and case are normalized.
    id: String,
    /// `DCVs` parameters in list order.
    #[serde(default)]
    params: BakeParams,
}

impl TypedCommand for BakeSelectionByID {
    type Payload = BakeSelectionByIdPayload;

    fn name(&self) -> &'static str {
        "filter.selectionByID"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "filter.selectionByID";
        let effect = guid_effect(
            NAME,
            String::new(),
            "filter".to_owned(),
            Some(&payload.id),
            &payload.params,
        )?;
        bake_core(document, NAME, effect)
    }
}

/// `filter.selectionByName` — CS6
/// `dom.filterSelectionByName(category, name)` (the manual's
/// `filterSelectonByName` usage-line typo rides this command).
struct BakeSelectionByName;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct BakeSelectionByNamePayload {
    /// Filter-menu category, e.g. "Adjust Color".
    category: String,
    /// Filter name, e.g. "Invert" (trailing dialog dots ignored).
    name: String,
    /// `DCVs` parameters in list order.
    #[serde(default)]
    params: BakeParams,
}

impl TypedCommand for BakeSelectionByName {
    type Payload = BakeSelectionByNamePayload;

    fn name(&self) -> &'static str {
        "filter.selectionByName"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "filter.selectionByName";
        let guid = catalog_guid(NAME, &payload.category, &payload.name)?;
        let effect = guid_effect(
            NAME,
            payload.category.clone(),
            payload.name.clone(),
            Some(guid),
            &payload.params,
        )?;
        bake_core(document, NAME, effect)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use serde_json::json;

    use super::*;
    use crate::catalog::testdoc;
    use crate::command::Command;
    use doc::ObjectId;

    const GUID_INVERT: &str = "{d2541291-70d6-11d1-8c8000a024cdc039}";

    /// Inserts an 8×8 fully-opaque image whose pixels count up from
    /// `seed` (tiles are already in the bank). Opaque keeps the
    /// premultiplied tile domain equal to straight, so kernel math is
    /// exact and invert is involutive.
    fn image_at(document: &mut Document, seed: u8) -> ObjectId {
        let (w, h) = (8i64, 8i64);
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        for (index, px) in pixels.chunks_exact_mut(4).enumerate() {
            // Quadratic, deliberately non-linear: a box mean preserves
            // affine gradients exactly, so a linear fixture would sit
            // still under Blur.
            px[0] = ((index as u32).pow(2) % 256) as u8;
            px[0] = px[0].wrapping_add(seed);
            px[1] = px[0].wrapping_add(17);
            px[2] = seed;
            px[3] = 255;
        }
        let (tiles, _minted) = crate::bitmap::retile_rgba(document, &pixels, w, h);
        document.registry.insert(ObjectData::Image(ImageObject {
            location: geom::Point::ZERO,
            offset: geom::Point::ZERO,
            pixels: (w, h),
            tile_size: i64::from(raster::TILE_SIDE as u32),
            tiles,
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        }))
    }

    /// Reads one pixel of an image straight out of its tiles.
    fn pixel(document: &Document, id: ObjectId, x: usize, y: usize) -> [u8; 4] {
        let ObjectData::Image(image) = document.registry.get(id).expect("image live") else {
            panic!("not an image");
        };
        crate::bitmap::assemble_rgba(document, image, "test").expect("assembles")
            [((y * 8 + x) * 4)..][..4]
            .try_into()
            .expect("four channels")
    }

    #[test]
    fn bake_inverts_selected_images_and_undoes() {
        let mut document = Document::default();
        let first = image_at(&mut document, 10);
        let second = image_at(&mut document, 200);
        document.object_selection = vec![first, second];
        let before_first = pixel(&document, first, 0, 0);
        let before_second = pixel(&document, second, 3, 4);

        let delta = BakeSelectionByID
            .apply_json(&mut document, json_onsel(GUID_INVERT, &[]))
            .expect("bake applies");
        match &delta {
            Delta::Group(deltas) => {
                assert_eq!(deltas.len(), 3, "two Mutated + one Bank");
            }
            other => panic!("expected a group, got {other:?}"),
        }
        // Pixel (0,0) of the quadratic fixture: 0²+10, +17, seed.
        assert_eq!(
            pixel(&document, first, 0, 0),
            [255 - 10, 255 - 27, 255 - 10, 255],
            "opaque pixels invert exactly"
        );
        assert_ne!(pixel(&document, second, 3, 4), before_second);

        // Invert is involutive: a second bake restores the pixels.
        BakeSelectionByID
            .apply_json(&mut document, json_onsel(GUID_INVERT, &[]))
            .expect("second bake");
        assert_eq!(pixel(&document, first, 0, 0), before_first);
        assert_eq!(pixel(&document, second, 3, 4), before_second);

        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(pixel(&document, first, 0, 0), before_first, "undo restores");
        assert_eq!(pixel(&document, second, 3, 4), before_second);
    }

    #[test]
    fn the_pixel_mask_gates_the_bake() {
        let mut document = Document::default();
        let id = image_at(&mut document, 0);
        document.object_selection = vec![id];
        let before = pixel(&document, id, 0, 0);
        let before_clear = pixel(&document, id, 7, 0);

        // Mask: left half active, right half clear.
        let mut alpha = vec![0u8; 64];
        for y in 0..8 {
            for x in 0..4 {
                alpha[y * 8 + x] = 255;
            }
        }
        document.selection = Some(SelectionMask {
            width: 8,
            height: 8,
            alpha,
        });

        BakeSelectionByID
            .apply_json(&mut document, json_onsel(GUID_INVERT, &[]))
            .expect("bake applies");
        assert_ne!(pixel(&document, id, 0, 0), before, "masked pixel changed");
        assert_eq!(
            pixel(&document, id, 7, 0),
            before_clear,
            "unmasked pixel stands"
        );
    }

    #[test]
    fn selection_by_name_resolves_the_catalog_and_refuses_unknowns() {
        let mut document = Document::default();
        let id = image_at(&mut document, 5);
        document.object_selection = vec![id];
        let before = pixel(&document, id, 2, 2);

        // "Blur..." — trailing dialog dots ignored (the CS6 scripts
        // author menu names with them).
        BakeSelectionByName
            .apply_json(
                &mut document,
                json!({"category": "Blur", "name": "Blur..."}),
            )
            .expect("blur resolves");
        assert_ne!(pixel(&document, id, 2, 2), before, "blur moved pixels");

        let after = serde_json::to_vec(&document).unwrap();
        let err = BakeSelectionByName
            .apply_json(
                &mut document,
                json!({"category": "Adjust Color", "name": "Bogus"}),
            )
            .expect_err("unknown name refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));
        let err = BakeSelectionByName
            .apply_json(&mut document, json!({"category": "Bogus", "name": "Blur"}))
            .expect_err("unknown category refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            after,
            "refusals mutate nothing"
        );
    }

    #[test]
    fn non_images_and_unresolvable_identities_refuse_untouched() {
        let (mut document, ids) = testdoc::sample_document();
        let image = image_at(&mut document, 42);

        // A path in the selection refuses the whole command.
        document.object_selection = vec![image, ids[0]];
        let snapshot = serde_json::to_vec(&document).unwrap();
        let err = BakeSelectionByID
            .apply_json(&mut document, json_onsel(GUID_INVERT, &[]))
            .expect_err("path refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            snapshot,
            "mixed selection refuses untouched"
        );

        // An identity no kernel speaks refuses.
        document.object_selection = vec![image];
        let snapshot = serde_json::to_vec(&document).unwrap();
        let err = BakeSelectionByID
            .apply_json(&mut document, json_onsel("bogus-guid", &[]))
            .expect_err("bogus guid refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));
        let err = BakeSelection
            .apply_json(&mut document, json!({"name": "noKernelByThisName"}))
            .expect_err("unknown internal name refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            snapshot,
            "identity refusals mutate nothing"
        );

        // So does an empty selection.
        document.object_selection.clear();
        let snapshot = serde_json::to_vec(&document).unwrap();
        let err = BakeSelectionByID
            .apply_json(&mut document, json_onsel(GUID_INVERT, &[]))
            .expect_err("empty selection refused");
        assert!(matches!(err, EngineError::InvalidState { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            snapshot,
            "the empty-selection refusal mutated nothing"
        );
    }

    #[test]
    fn the_effect_form_carries_the_identity_and_params() {
        let mut document = Document::default();
        let id = image_at(&mut document, 7);
        document.object_selection = vec![id];
        let before = pixel(&document, id, 1, 1);

        // The CS6 payload shape: EffectMoaID as a parameter plus
        // scalars (the manual's Hue/Saturation example).
        BakeSelection
            .apply_json(
                &mut document,
                json!({
                    "name": "Hue/Saturation",
                    "category": "Adjust Color",
                    "params": [
                        {"key": "EffectMoaID", "value": GUID_INVERT},
                        {"key": "hls_colorize", "value": "true"},
                        {"key": "hue_amount", "value": "30"},
                        {"key": "lightness_amount", "value": "0"},
                        {"key": "saturation_amount", "value": "20"}
                    ]
                }),
            )
            .expect("the CS6-shaped payload applies");
        assert_ne!(pixel(&document, id, 1, 1), before);

        // Without any interpretable identity, the same shape refuses.
        let after = serde_json::to_vec(&document).unwrap();
        let err = BakeSelection
            .apply_json(
                &mut document,
                json!({"name": "Hue/Saturation", "params": [{"key": "hue_amount", "value": "30"}]}),
            )
            .expect_err("identity-less effect refuses");
        assert!(matches!(err, EngineError::InvalidState { .. }));
        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            after,
            "the refused form mutated nothing"
        );
    }

    /// `filter.selectionByID` payload JSON with one id.
    fn json_onsel(guid: &str, params: &[(&str, &str)]) -> serde_json::Value {
        serde_json::json!({
            "id": guid,
            "params": params
                .iter()
                .map(|(key, value)| serde_json::json!({"key": key, "value": value}))
                .collect::<Vec<_>>()
        })
    }
}
