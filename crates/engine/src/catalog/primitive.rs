//! `PRIv` rectangle-primitive parameter commands (CS6's
//! `dom.setRectRoundness` / `dom.setRectSides`; decision 0027).
//!
//! A rectangle primitive keeps its untransformed source in
//! [`doc::PrimitiveSource`] (`RCTv` rect, `RDSf` radius, `MTXv`
//! transform) and re-derives its editable contours from it, so a
//! parameter change is "capture the path, rewrite the source, re-derive
//! the contours". The dictionary's `PRIv` children are exhaustive and
//! carry no roundness *mode* node — `setRectRoundnessMode` is UI
//! session state with no wire home and gets no command (decision 0027,
//! the Phase-4 rule: do not invent wire state).

use doc::{Document, ObjectData, ObjectId};

use super::object::rounded_rect_contour;
use crate::access;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the primitive commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(SetRectRoundness)?;
    registry.register(SetRectSides)
}

/// The `PathObject` behind a rectangle primitive, cloned out
/// (capture-before-mutate) with its `PRIv` source.
fn primitive_path(
    document: &Document,
    name: &'static str,
    id: ObjectId,
) -> Result<(doc::PathObject, doc::PrimitiveSource), EngineError> {
    let path = match access::object(document, name, id)? {
        ObjectData::Path(path) => path.clone(),
        _ => {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("object {} is not a vector path", id.index()),
            });
        }
    };
    let Some(primitive) = path.primitive.clone() else {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("path {} carries no PRIv primitive source", id.index()),
        });
    };
    Ok((path, primitive))
}

/// Writes `contours` and `primitive` back over the captured path,
/// returning the mutated-object delta.
fn store_path(
    document: &mut Document,
    id: ObjectId,
    before: doc::PathObject,
    contours: Vec<doc::Contour>,
    primitive: doc::PrimitiveSource,
) -> Delta {
    if let Some(ObjectData::Path(path)) = document.registry.get_mut(id) {
        path.contours = contours;
        path.primitive = Some(primitive);
    }
    Delta::Object {
        id,
        inverse: ObjectInverse::Mutated {
            object: Box::new(ObjectData::Path(before)),
        },
    }
}

/// The corner-roundness unit of `setRectRoundness` (CS6's `mode`
/// strings).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum RoundnessMode {
    /// `roundness` is 0..1 of the shorter side (1 = fully round).
    Percent,
    /// `roundness` is the corner radius itself.
    Pixels,
}

/// `primitive.setRectRoundness` — CS6
/// `dom.setRectRoundness(roundness, mode)`: sets the primitive's
/// radius and re-derives the contours. Percent values scale the
/// shorter side (1 = the stadium limit, `min(w, h) / 2`); pixel values
/// store as given and clamp at derivation time.
struct SetRectRoundness;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetRectRoundnessPayload {
    /// Target rectangle primitive.
    id: ObjectId,
    /// The roundness value, in `mode` units.
    roundness: f64,
    /// The unit `roundness` is expressed in.
    mode: RoundnessMode,
}

impl TypedCommand for SetRectRoundness {
    type Payload = SetRectRoundnessPayload;

    fn name(&self) -> &'static str {
        "primitive.setRectRoundness"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "primitive.setRectRoundness";
        if !payload.roundness.is_finite() {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!("roundness {} is not finite", payload.roundness),
            });
        }
        let (before, primitive) = primitive_path(document, NAME, payload.id)?;
        let side = primitive
            .rect
            .width()
            .abs()
            .min(primitive.rect.height().abs());
        let radius = match payload.mode {
            RoundnessMode::Percent => {
                if !(0.0..=1.0).contains(&payload.roundness) {
                    return Err(EngineError::BadPayload {
                        name: NAME.to_owned(),
                        detail: format!("percent roundness {} is outside 0..1", payload.roundness),
                    });
                }
                payload.roundness * side / 2.0
            }
            RoundnessMode::Pixels => {
                if payload.roundness < 0.0 {
                    return Err(EngineError::BadPayload {
                        name: NAME.to_owned(),
                        detail: format!("pixel roundness {} is negative", payload.roundness),
                    });
                }
                payload.roundness
            }
        };
        let short = side / 2.0;
        let contours = rounded_rect_contour(primitive.rect, radius.min(short));
        let mut primitive = primitive;
        primitive.radius = radius;
        Ok(store_path(
            document, payload.id, before, contours, primitive,
        ))
    }
}

/// `primitive.setRectSides` — CS6 `dom.setRectSides(newSides)`: the new
/// *untransformed* sides of the primitive (the live `MTXv` keeps
/// showing the transformed result). The stored radius carries over;
/// derivation clamps it to the new rect.
struct SetRectSides;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetRectSidesPayload {
    /// Target rectangle primitive.
    id: ObjectId,
    /// New untransformed rect as `[x0, y0, x1, y1]`.
    rect: [f64; 4],
}

impl TypedCommand for SetRectSides {
    type Payload = SetRectSidesPayload;

    fn name(&self) -> &'static str {
        "primitive.setRectSides"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "primitive.setRectSides";
        let [x0, y0, x1, y1] = payload.rect;
        if [x0, y0, x1, y1].iter().any(|value| !value.is_finite()) {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: "rect is not finite".to_owned(),
            });
        }
        if x1 <= x0 || y1 <= y0 {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!("rect ({x0}, {y0})..({x1}, {y1}) is empty"),
            });
        }
        let (before, mut primitive) = primitive_path(document, NAME, payload.id)?;
        primitive.rect = geom::Rect::new(x0, y0, x1, y1);
        let short = primitive
            .rect
            .width()
            .abs()
            .min(primitive.rect.height().abs())
            / 2.0;
        let contours = rounded_rect_contour(primitive.rect, primitive.radius.min(short));
        Ok(store_path(
            document, payload.id, before, contours, primitive,
        ))
    }
}

#[cfg(test)]
mod primitive_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::catalog::testdoc;
    use crate::command::Command;

    /// A document with one 100x50 rectangle primitive (sharp corners,
    /// identity transform) appended to page 0, layer 0, cell 0.
    fn rect_document() -> (Document, ObjectId) {
        let (mut document, _ids) = testdoc::sample_document();
        let rect = geom::Rect::new(10.0, 20.0, 110.0, 70.0);
        let id = document.registry.insert(ObjectData::Path(doc::PathObject {
            contours: rounded_rect_contour(rect, 0.0),
            primitive: Some(doc::PrimitiveSource {
                rect,
                radius: 0.0,
                transform: geom::Affine::IDENTITY,
                maintain_radius: false,
            }),
            even_odd: false,
            paint: None,
            fill_start: None,
            fill_end: None,
            fill_third: None,
            brush_placement: None,
            fill_on_top: None,
            locked: false,
            visible: true,
            opacity: 1000,
            blend: 0,
            name: None,
            effects: Vec::new(),
        }));
        document.pages[0].layers.layers[0].cells[0]
            .elements
            .push(id);
        (document, id)
    }

    /// The path behind `id`, unwrapped.
    fn path_of(document: &Document, id: ObjectId) -> doc::PathObject {
        match document.registry.get(id) {
            Some(ObjectData::Path(path)) => path.clone(),
            other => panic!("expected a path, got {other:?}"),
        }
    }

    #[test]
    fn percent_roundness_rederives_the_contours() {
        let (mut document, id) = rect_document();
        let delta = SetRectRoundness
            .apply_json(
                &mut document,
                serde_json::json!({"id": id, "roundness": 1.0, "mode": "percent"}),
            )
            .expect("roundness applies");
        let path = path_of(&document, id);
        let primitive = path.primitive.as_ref().expect("primitive stays");
        // 100% of the 50-unit shorter side is the stadium radius.
        assert_eq!(primitive.radius, 25.0);
        assert_eq!(path.contours.len(), 1);
        assert_eq!(path.contours[0].point_count, 8);
        delta.apply_inverse(&mut document).expect("undo");
        let path = path_of(&document, id);
        assert_eq!(path.primitive.as_ref().expect("radius").radius, 0.0);
        assert_eq!(path.contours[0].point_count, 4, "sharp corners return");
    }

    #[test]
    fn pixel_roundness_stores_and_clamps() {
        let (mut document, id) = rect_document();
        SetRectRoundness
            .apply_json(
                &mut document,
                serde_json::json!({"id": id, "roundness": 40.0, "mode": "pixels"}),
            )
            .expect("pixel roundness applies");
        let path = path_of(&document, id);
        // Stored as given (40 > the 25 stadium limit)...
        assert_eq!(path.primitive.as_ref().expect("radius").radius, 40.0);
        // ...while the contours clamp (anchors sit at the clamped 25,
        // not the stored 40).
        let top_left = &path.contours[0].points[0];
        let top_end = &path.contours[0].points[1];
        assert_eq!(top_left.anchor.x, 35.0, "x0 + clamped radius");
        assert_eq!(
            top_end.anchor.x - top_left.anchor.x,
            50.0,
            "the straight top edge is width - 2 * clamped radius"
        );

        // Refusals: percent out of range, negative pixels, and a
        // target without a PRIv source (a bare fixture path).
        for payload in [
            serde_json::json!({"id": id, "roundness": 1.5, "mode": "percent"}),
            serde_json::json!({"id": id, "roundness": -1.0, "mode": "pixels"}),
        ] {
            assert!(
                SetRectRoundness.apply_json(&mut document, payload).is_err(),
                "roundness must refuse"
            );
        }
        let (mut bare, ids) = testdoc::sample_document();
        assert!(
            SetRectRoundness
                .apply_json(
                    &mut bare,
                    serde_json::json!({"id": ids[0], "roundness": 0.5, "mode": "percent"})
                )
                .is_err(),
            "a path without PRIv refuses"
        );
    }

    #[test]
    fn set_rect_sides_rebases_the_primitive() {
        let (mut document, id) = rect_document();
        // Round it first so the carry-over is observable.
        SetRectRoundness
            .apply_json(
                &mut document,
                serde_json::json!({"id": id, "roundness": 0.5, "mode": "percent"}),
            )
            .expect("roundness applies");
        let delta = SetRectSides
            .apply_json(
                &mut document,
                serde_json::json!({"id": id, "rect": [0.0, 0.0, 200.0, 100.0]}),
            )
            .expect("sides apply");
        let primitive = path_of(&document, id).primitive.expect("primitive stays");
        assert_eq!(primitive.rect.x1, 200.0);
        // The 12.5-unit radius carries over (the rect only grew).
        assert_eq!(primitive.radius, 12.5);
        delta.apply_inverse(&mut document).expect("undo");
        let primitive = path_of(&document, id).primitive.expect("primitive stays");
        assert_eq!(primitive.rect.x1, 110.0);
        assert_eq!(primitive.radius, 12.5);

        // Refusals: non-finite and empty rects.
        for payload in [
            serde_json::json!({"id": id, "rect": [0.0, 0.0, 0.0, 100.0]}),
            serde_json::json!({"id": id, "rect": [0.0, 0.0, 50.0, 0.0]}),
        ] {
            assert!(
                SetRectSides.apply_json(&mut document, payload).is_err(),
                "sides must refuse"
            );
        }
    }
}
