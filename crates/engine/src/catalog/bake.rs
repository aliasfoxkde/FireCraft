//! The filter bake family (decisions 0040/0042): CS6's
//! `filterSelection` / `filterSelectionByID` / `filterSelectionByName`
//! verbs — "applies the specified pixel filter to the selection as a
//! permanent action, not as a Live Effect" — plus the two parameter
//! forms `convolveSelection` (arbitrary integer kernel) and
//! `motionBlurSelection` (linear/radial/zoom with explicit samples).
//!
//! All five front doors resolve to the same core: interpret the filter
//! through compose's kernel planner (the same interpreter the Live
//! Effect stack and the flatten bakes use), run it over each selected
//! image's assembled surface, and re-tile. The active pixel selection
//! (`document.selection`) gates the result per pixel; CS6's scripts
//! split on `getPixelMask()` exactly this way. A non-image member
//! rasterizes first (decision 0059): CS6 converts the object to a
//! bitmap, then filters it — the slot mutates in place to a
//! canvas-spanning image, the flatten family's bake posture (decision
//! 0031). Slices refuse: they are web annotations, not rasterizable
//! content. Every target resolution, identity check, and tile assembly
//! happens before the first mutation (the 000557d ordering law).

use compose::Surface;
use doc::{Document, Effect, EffectProperty, EffectValue, ImageObject, ObjectData, SelectionMask};
use raster::TILE_SIDE;

use super::filter::{FilterParamPayload, build_effect};
use super::object::find_owner;
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
    registry.register(BakeSelectionByName)?;
    registry.register(ConvolveSelection)?;
    registry.register(MotionBlurSelection)
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

/// One selected object's resolved bake input. Images carry their
/// assembled surface and canvas location (the mask gate maps through
/// it); every other rasterizable kind carries its canvas-space render
/// (decision 0059's rasterize-first) — the conversion lands
/// canvas-spanning at the origin, the flatten family's bake posture.
enum BakeTarget {
    Image {
        assembled: Vec<u8>,
        location: geom::Point,
    },
    Convert {
        rendered: Vec<u8>,
        extent: (i64, i64),
    },
}

/// Walks up the ownership chain to the cell holding `id` — the page a
/// nested object renders on (the walker rule: an unplaced id carries
/// no page and refuses).
fn owning_page(
    document: &Document,
    name: &'static str,
    id: doc::ObjectId,
) -> Result<usize, EngineError> {
    let mut current = id;
    for _ in 0..64 {
        match find_owner(document, current) {
            None => {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: format!(
                        "object {} (generation {}) is not attached to any cell or group",
                        id.index(),
                        id.generation()
                    ),
                });
            }
            Some(super::object::Owner::Cell { page, .. }) => return Ok(page),
            Some(super::object::Owner::Group { id, .. }) => current = id,
        }
    }
    Err(EngineError::InvalidState {
        name: name.to_owned(),
        detail: "the ownership chain is cyclic".to_owned(),
    })
}

/// The shared bake: interpret the effect, then transform every image
/// in the object selection, gated by the active pixel mask.
fn bake_core(
    document: &mut Document,
    name: &'static str,
    plan: compose::Plan,
) -> Result<Delta, EngineError> {
    if document.object_selection.is_empty() {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "nothing is selected".to_owned(),
        });
    }
    // The plan must resolve completely — a skipped op or an empty op
    // list bakes nothing, and the bake refuses rather than applying
    // nothing (the no-passthrough floor law).
    if plan.skipped != 0 || plan.ops.is_empty() {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: "the filter does not resolve to a pixel kernel (unknown identity or missing parameters)"
                .to_owned(),
        });
    }
    // Resolve every target before any mutation (000557d): existence,
    // editability, and — for conversions (decision 0059) — the
    // read-only render of the object's own pixels.
    let mut targets = Vec::with_capacity(document.object_selection.len());
    for id in document.object_selection.clone() {
        let before = access::object(document, name, id)?;
        match &before {
            ObjectData::Image(image) => {
                let assembled = assemble_rgba(document, image, name)?;
                let location = image.location;
                targets.push((
                    id,
                    before.clone(),
                    BakeTarget::Image {
                        assembled,
                        location,
                    },
                ));
            }
            ObjectData::Slice(_) => {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: "slice objects are not rasterizable content".to_owned(),
                });
            }
            _ => {
                let page = owning_page(document, name, id)?;
                let rendered = compose::render_subset(document, page, 0, &[id]).map_err(|err| {
                    EngineError::InvalidState {
                        name: name.to_owned(),
                        detail: format!("object {} does not render: {err}", id.index()),
                    }
                })?;
                let extent = (i64::from(rendered.width), i64::from(rendered.height));
                targets.push((
                    id,
                    before.clone(),
                    BakeTarget::Convert {
                        rendered: rendered.pixels,
                        extent,
                    },
                ));
            }
        }
    }
    // Transform and re-tile each target, capturing per-object deltas in
    // mutation order; the pixel mask (canvas space) gates the result.
    let mask = document.selection.clone();
    let mut deltas = Vec::with_capacity(targets.len() + 1);
    let mut minted_all = Vec::new();
    for (id, before, target) in targets {
        let converting = matches!(target, BakeTarget::Convert { .. });
        let (width, height, assembled, location) = match target {
            BakeTarget::Image {
                assembled,
                location,
            } => {
                let ObjectData::Image(image) = &before else {
                    return Err(EngineError::InvalidState {
                        name: name.to_owned(),
                        detail: format!("object {} vanished mid-bake", id.index()),
                    });
                };
                (image.pixels.0, image.pixels.1, assembled, location)
            }
            BakeTarget::Convert { rendered, extent } => {
                (extent.0, extent.1, rendered, geom::Point::ZERO)
            }
        };
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
            blend_through_mask(&mut pixels, &assembled, width, height, location, mask);
        }
        let (tiles, minted) = retile_rgba(document, &pixels, width, height);
        minted_all.extend(minted);
        match document.registry.get_mut(id) {
            Some(ObjectData::Image(image)) => image.tiles = tiles,
            Some(slot) if converting => {
                *slot = ObjectData::Image(ImageObject {
                    location: geom::Point::ZERO,
                    offset: geom::Point::ZERO,
                    pixels: (width, height),
                    tile_size: i64::from(TILE_SIDE as u32),
                    tiles,
                    name: None,
                    // The render carries the old weight already — the
                    // flatten family's bake posture (decision 0031).
                    opacity: 1000,
                    blend: 0,
                    visible: true,
                    locked: false,
                    effects: Vec::new(),
                });
            }
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
    location: geom::Point,
    mask: &SelectionMask,
) {
    let left = location.x.round() as i64;
    let top = location.y.round() as i64;
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
        bake_core(document, NAME, compose::plan(std::slice::from_ref(&effect)))
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
        bake_core(document, NAME, compose::plan(std::slice::from_ref(&effect)))
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
        bake_core(document, NAME, compose::plan(std::slice::from_ref(&effect)))
    }
}

/// `filter.convolveSelection` — CS6
/// `dom.convolveSelection(kernelWidth, kernelHeight, kernelValues,
/// affectsAlpha)`: an arbitrary integer kernel baked through the same
/// core. Coefficients apply as-is (no normalization — the dom passes
/// raw values, so their scale is the caller's contract).
struct ConvolveSelection;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct ConvolveSelectionPayload {
    /// Kernel width in cells.
    kernel_width: u32,
    /// Kernel height in cells.
    kernel_height: u32,
    /// `kernel_width * kernel_height` row-major coefficients.
    kernel_values: Vec<i64>,
    /// Extend the pass to the alpha channel.
    #[serde(default)]
    affects_alpha: bool,
}

/// The largest kernel side accepted — a 31×31 kernel already samples a
/// 15-pixel radius on every axis, far past the UI's 7×7; anything
/// bigger is a payload bug, not a filter.
const KERNEL_SIDE_CAP: u32 = 31;

impl TypedCommand for ConvolveSelection {
    type Payload = ConvolveSelectionPayload;

    fn name(&self) -> &'static str {
        "filter.convolveSelection"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "filter.convolveSelection";
        if payload.kernel_width == 0
            || payload.kernel_height == 0
            || payload.kernel_width > KERNEL_SIDE_CAP
            || payload.kernel_height > KERNEL_SIDE_CAP
        {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "kernel {}x{} is outside 1..={KERNEL_SIDE_CAP} per side",
                    payload.kernel_width, payload.kernel_height
                ),
            });
        }
        let cells = usize::try_from(payload.kernel_width)
            .ok()
            .zip(usize::try_from(payload.kernel_height).ok())
            .and_then(|(w, h)| w.checked_mul(h));
        let Some(cells) = cells else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "kernel dimensions overflow usize".to_owned(),
            });
        };
        if payload.kernel_values.len() != cells {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "kernel_values holds {} entries for a {}x{} kernel (want {cells})",
                    payload.kernel_values.len(),
                    payload.kernel_width,
                    payload.kernel_height
                ),
            });
        }
        let plan = compose::Plan {
            ops: vec![compose::KernelOp::Convolve {
                width: payload.kernel_width,
                height: payload.kernel_height,
                values: payload.kernel_values.clone(),
                affects_alpha: payload.affects_alpha,
            }],
            skipped: 0,
        };
        bake_core(document, NAME, plan)
    }
}

/// `filter.motionBlurSelection` — CS6
/// `dom.motionBlurSelection(blurType, angle, distance, samples)`:
/// `blurType` is "linear", "radial", or "zoom"; `angle` (degrees) and
/// `distance` (pixels, 0..=400) drive the smear; `samples` is "the
/// number of times the original image is cloned" — the kernel's tap
/// count. Radial and zoom are center-based, so `angle` stands unused
/// there; `distance` reaches their kernels as sweep degrees / pinch
/// percent respectively (the recorded 0042 substitution).
struct MotionBlurSelection;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct MotionBlurSelectionPayload {
    /// "linear", "radial", or "zoom".
    blur_type: String,
    /// Smear direction, degrees (linear only).
    angle: f64,
    /// Smear length: pixels (linear), sweep degrees (radial), pinch
    /// percent (zoom); 0..=400, 0 = no blur.
    distance: u32,
    /// Kernel tap count ("clones of the original"); 0 or 1 = original
    /// only — an exact copy.
    samples: u32,
}

impl TypedCommand for MotionBlurSelection {
    type Payload = MotionBlurSelectionPayload;

    fn name(&self) -> &'static str {
        "filter.motionBlurSelection"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "filter.motionBlurSelection";
        if !payload.angle.is_finite() || !(0.0..=360.0).contains(&payload.angle) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("angle {} is outside 0..=360 degrees", payload.angle),
            });
        }
        if payload.distance > 400 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("distance {} is outside 0..=400", payload.distance),
            });
        }
        if payload.samples > 100 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("samples {} is outside 0..=100", payload.samples),
            });
        }
        // The kernels own the carrier-range clamps (radial sweep caps
        // at 360, zoom pinch at 100, quality at 1..=100); the engine
        // only gates the dom-documented ranges above.
        let op = match payload.blur_type.as_str() {
            "linear" => compose::KernelOp::MotionBlur {
                angle_degrees: payload.angle,
                distance: payload.distance,
                taps: payload.samples.max(1),
            },
            "radial" => compose::KernelOp::RadialBlur {
                amount: payload.distance,
                quality: payload.samples.max(1),
            },
            "zoom" => compose::KernelOp::ZoomBlur {
                amount: payload.distance,
                quality: payload.samples.max(1),
            },
            other => {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("unknown blur_type {other:?} (want linear/radial/zoom)"),
                });
            }
        };
        let plan = compose::Plan {
            ops: vec![op],
            skipped: 0,
        };
        bake_core(document, NAME, plan)
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

    /// A fully-opaque solid-color 8×8 image (the strongest convolve
    /// fixture: every sampled neighbor equals the pixel itself).
    fn solid_image(document: &mut Document, color: [u8; 4]) -> ObjectId {
        let (w, h) = (8i64, 8i64);
        let pixels = vec![color; (w * h) as usize].concat();
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

    #[test]
    fn convolve_identity_kernel_is_byte_equal_and_undoes() {
        let mut document = Document::default();
        let id = image_at(&mut document, 9);
        document.object_selection = vec![id];
        let before = pixel(&document, id, 5, 5);

        let delta = ConvolveSelection
            .apply_json(
                &mut document,
                json!({
                    "kernel_width": 3,
                    "kernel_height": 3,
                    "kernel_values": [0, 0, 0, 0, 1, 0, 0, 0, 0],
                }),
            )
            .expect("identity kernel applies");
        assert_eq!(pixel(&document, id, 5, 5), before, "center 1 keeps bytes");

        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(pixel(&document, id, 5, 5), before);
    }

    #[test]
    fn convolve_laplacian_zeros_a_solid() {
        let mut document = Document::default();
        let id = solid_image(&mut document, [200, 40, 30, 255]);
        document.object_selection = vec![id];

        // The classic edge kernel: a solid has no edges, so every
        // channel sums to -255 → clamped black. Edge-clamped sampling
        // reads the corner four times — same sum, same black.
        ConvolveSelection
            .apply_json(
                &mut document,
                json!({
                    "kernel_width": 3,
                    "kernel_height": 3,
                    "kernel_values": [0, 1, 0, 1, -4, 1, 0, 1, 0],
                }),
            )
            .expect("laplacian applies");
        for (x, y) in [(0usize, 0usize), (4, 4), (7, 7), (7, 3)] {
            assert_eq!(
                pixel(&document, id, x, y),
                [0, 0, 0, 255],
                "({x},{y}) solid has no edges"
            );
        }
    }

    #[test]
    fn convolve_affects_alpha_gate() {
        let mut document = Document::default();
        let id = solid_image(&mut document, [10, 20, 30, 100]);
        document.object_selection = vec![id];
        // A 1×1 ×2 kernel: doubles what it touches, clamped at 255.
        ConvolveSelection
            .apply_json(
                &mut document,
                json!({
                    "kernel_width": 1,
                    "kernel_height": 1,
                    "kernel_values": [2],
                }),
            )
            .expect("rgb pass");
        assert_eq!(
            pixel(&document, id, 2, 2),
            [20, 40, 60, 100],
            "alpha stands when affects_alpha is false"
        );

        ConvolveSelection
            .apply_json(
                &mut document,
                json!({
                    "kernel_width": 1,
                    "kernel_height": 1,
                    "kernel_values": [2],
                    "affects_alpha": true,
                }),
            )
            .expect("rgba pass");
        assert_eq!(
            pixel(&document, id, 2, 2),
            [40, 80, 120, 200],
            "alpha doubles too"
        );
    }

    #[test]
    fn motion_blur_samples_one_is_an_exact_copy() {
        let mut document = Document::default();
        let id = image_at(&mut document, 3);
        document.object_selection = vec![id];
        let before: Vec<[u8; 4]> = [(0usize, 0usize), (3, 5), (7, 7)]
            .into_iter()
            .map(|(x, y)| pixel(&document, id, x, y))
            .collect();

        // A single tap has no second sample to average, so the bake
        // writes the pixels back unchanged (the tile bank still
        // re-mints — pixels are the contract here).
        for samples in [0u32, 1] {
            MotionBlurSelection
                .apply_json(
                    &mut document,
                    json!({"blur_type": "linear", "angle": 90.0, "distance": 4, "samples": samples}),
                )
                .expect("single tap applies");
            for (slot, (x, y)) in before.iter().zip([(0usize, 0usize), (3, 5), (7, 7)]) {
                assert_eq!(pixel(&document, id, x, y), *slot, "samples {samples}");
            }
        }
    }

    #[test]
    fn motion_blur_linear_smears_and_undoes() {
        let mut document = Document::default();
        let id = image_at(&mut document, 0);
        document.object_selection = vec![id];
        let before = pixel(&document, id, 6, 2);

        let delta = MotionBlurSelection
            .apply_json(
                &mut document,
                json!({"blur_type": "linear", "angle": 0.0, "distance": 3, "samples": 4}),
            )
            .expect("linear blur applies");
        match &delta {
            Delta::Group(deltas) => assert_eq!(deltas.len(), 2, "one Mutated + one Bank"),
            other => panic!("expected a group, got {other:?}"),
        }
        assert_ne!(pixel(&document, id, 6, 2), before, "the smear moves pixels");
        delta.apply_inverse(&mut document).expect("undo");
        assert_eq!(pixel(&document, id, 6, 2), before);
    }

    #[test]
    fn motion_blur_radial_and_zoom_reach_their_kernels() {
        let mut document = Document::default();
        let id = image_at(&mut document, 77);
        document.object_selection = vec![id];
        let before = pixel(&document, id, 1, 6);

        MotionBlurSelection
            .apply_json(
                &mut document,
                json!({"blur_type": "radial", "angle": 0.0, "distance": 30, "samples": 8}),
            )
            .expect("radial applies");
        assert_ne!(pixel(&document, id, 1, 6), before, "spin moved pixels");

        MotionBlurSelection
            .apply_json(
                &mut document,
                json!({"blur_type": "zoom", "angle": 0.0, "distance": 25, "samples": 8}),
            )
            .expect("zoom applies");
        assert_ne!(pixel(&document, id, 1, 6), before, "zoom moved pixels");
    }

    #[test]
    fn convolve_and_motion_blur_refusals_mutate_nothing() {
        let mut document = Document::default();
        let id = image_at(&mut document, 1);
        document.object_selection = vec![id];
        let snapshot = serde_json::to_vec(&document).unwrap();

        let bad_convolve = [
            json!({"kernel_width": 0, "kernel_height": 3, "kernel_values": []}),
            json!({"kernel_width": 3, "kernel_height": 32, "kernel_values": []}),
            json!({"kernel_width": 3, "kernel_height": 3, "kernel_values": [0, 1]}),
        ];
        for payload in bad_convolve {
            let err = ConvolveSelection
                .apply_json(&mut document, payload)
                .expect_err("bad kernel refused");
            assert!(matches!(err, EngineError::InvalidState { .. }));
        }

        let bad_blur = [
            json!({"blur_type": "spiral", "angle": 0.0, "distance": 4, "samples": 2}),
            json!({"blur_type": "linear", "angle": -0.5, "distance": 4, "samples": 2}),
            json!({"blur_type": "linear", "angle": 361.0, "distance": 4, "samples": 2}),
            json!({"blur_type": "linear", "angle": 45.0, "distance": 401, "samples": 2}),
            json!({"blur_type": "linear", "angle": 45.0, "distance": 4, "samples": 101}),
        ];
        for payload in bad_blur {
            let err = MotionBlurSelection
                .apply_json(&mut document, payload)
                .expect_err("bad blur payload refused");
            assert!(matches!(err, EngineError::InvalidState { .. }));
        }

        assert_eq!(
            serde_json::to_vec(&document).unwrap(),
            snapshot,
            "every refusal left the document untouched"
        );
    }

    /// One page, one layer, one cell: `object` placed on a 24×24
    /// canvas and selected. The conversion tests need a real render
    /// target (the sample document's default canvas is zero-extent).
    fn render_document(object: ObjectData) -> (Document, doc::ObjectId) {
        use doc::{Canvas, Cell, Layer, Page, PageInfo};
        let mut document = Document::default();
        let id = document.registry.insert(object);
        let layer = Layer {
            name: "Layer 0".to_owned(),
            display: true,
            cells: vec![
                Cell {
                    locked: false,
                    visible: true,
                    elements: vec![id],
                },
                Cell {
                    locked: false,
                    visible: true,
                    elements: Vec::new(),
                },
            ],
            ..Layer::default()
        };
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: 2,
                frame_count_duplicate: 2,
                request_id: 0,
                canvas: Canvas {
                    width: 24.0,
                    height: 24.0,
                    ..Canvas::default()
                },
                onion_before: 0,
                onion_after: 0,
                frame_durations: Vec::new(),
            },
            name: Some("Page 0".to_owned()),
            is_master: false,
            is_linked: false,
            page_id: None,
            last_export_dir: None,
            last_export_file: None,
            layers: doc::LayerList {
                layers: vec![layer],
            },
            grid: None,
            export: None,
            guides: Default::default(),
        });
        document.object_selection = vec![id];
        (document, id)
    }

    /// An 8×9 square path at (4,4) with an opaque half-red fill —
    /// renders 81 opaque pixels inside the 24×24 canvas.
    fn filled_square() -> ObjectData {
        let mut path = match testdoc::path_object(1) {
            ObjectData::Path(path) => path,
            other => panic!("expected a path, got {other:?}"),
        };
        path.paint = Some(doc::PaintStyle {
            stroke_raw: 0,
            fill_raw: 0xFF_00_00_80, // ABGR: alpha 255, red 128
            fill: None,
        });
        // testdoc's square spans 0..9; shove it inside the canvas
        // instead of hugging the edge.
        for contour in &mut path.contours {
            for point in &mut contour.points {
                point.anchor.x += 4.0;
                point.anchor.y += 4.0;
            }
        }
        ObjectData::Path(path)
    }

    #[test]
    fn bake_rasterizes_a_path_then_filters_it() {
        let (mut document, id) = render_document(filled_square());
        let bank_before = document.tiles.len();

        let delta = BakeSelectionByID
            .apply_json(&mut document, json_onsel(GUID_INVERT, &[]))
            .expect("bake applies");

        // The slot converted in place: same id, image kind, spanning
        // the canvas at the origin (the flatten bake posture).
        let ObjectData::Image(image) = document.registry.get(id).expect("live") else {
            panic!("the path converted to an image");
        };
        assert_eq!(image.pixels, (24, 24));
        assert_eq!(image.location, geom::Point::ZERO);
        assert_eq!(image.opacity, 1000, "the render carries the weight");
        assert!(image.effects.is_empty());

        // Inside the square the fill inverted; outside it is blank.
        let assembled = crate::bitmap::assemble_rgba(&document, image, "test").expect("assembles");
        let at = |x: usize, y: usize| -> [u8; 4] {
            assembled[((y * 24 + x) * 4)..][..4]
                .try_into()
                .expect("four channels")
        };
        assert_eq!(at(6, 6), [127, 255, 255, 255], "fill inverted");
        assert_eq!(at(1, 1), [0, 0, 0, 0], "outside stays transparent");

        // Undo restores the path and retracts the minted tiles.
        delta.apply_inverse(&mut document).expect("undo");
        assert!(matches!(
            document.registry.get(id),
            Some(ObjectData::Path(_))
        ));
        assert_eq!(document.tiles.len(), bank_before, "tiles retracted");
    }

    #[test]
    fn bake_rasterizes_through_groups_and_refuses_slices() {
        use doc::GroupObject;
        // A group wrapping the filled square: the selection is the
        // group, so the conversion resolves the page through the
        // ownership walk and converts the group's slot.
        let (mut document, child) = render_document(filled_square());
        let group = document.registry.insert(ObjectData::Group(GroupObject {
            group_type: 0,
            name: None,
            children: vec![child],
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            mask_hidden: false,
            mask_linked: None,
            effects: Vec::new(),
        }));
        document.pages[0].layers.layers[0].cells[0].elements = vec![group];
        document.object_selection = vec![group];

        BakeSelectionByID
            .apply_json(&mut document, json_onsel(GUID_INVERT, &[]))
            .expect("group bakes");
        assert!(matches!(
            document.registry.get(group),
            Some(ObjectData::Image(_))
        ));

        // A slice refuses whole: not rasterizable content.
        let slice = document
            .registry
            .insert(ObjectData::Slice(doc::SliceObject {
                internal_name: None,
                frame: geom::Rect::new(0.0, 0.0, 4.0, 4.0),
                shape: 0,
                fill_raw: 0,
                name: None,
                slice_type: None,
                link: None,
                alt: None,
                label: None,
                td_text: None,
                symbol_number: None,
                contours: Vec::new(),
                background: None,
                behaviors: Vec::new(),
                custom_data: std::collections::BTreeMap::new(),
                export: None,
            }));
        document.pages[0].layers.layers[0].cells[1].elements = vec![slice];
        document.object_selection = vec![slice];
        let err = BakeSelectionByID
            .apply_json(&mut document, json_onsel(GUID_INVERT, &[]))
            .expect_err("slices refuse");
        assert!(matches!(err, EngineError::InvalidState { .. }));
        assert!(
            matches!(document.registry.get(slice), Some(ObjectData::Slice(_))),
            "the refusal mutated nothing"
        );
    }
}
