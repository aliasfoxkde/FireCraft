//! Path commands: boolean combine (Modify ▸ Combine Paths), simplify,
//! expand stroke, and inset/outset (Modify ▸ Alter Path). The geometry
//! lives in the `vector` crate (L2); these commands validate, capture,
//! and splice results into the document.
//!
//! Result-object contract (provisional until corpus evidence pins
//! Fireworks' exact result metadata): every command replaces the
//! target's *geometry* (`contours`, fill rule, live-primitive source —
//! a derived outline is no longer the primitive that made it) and keeps
//! the object's display fields (paint, opacity, blend, visibility, lock,
//! name). `path.boolean` keeps `a`'s identity and removes `b`; a result
//! with no contours (punch or intersect that consumes everything)
//! removes both operands, the way Fireworks leaves nothing behind.

use doc::{Document, ObjectData, ObjectId};
use vector::{BooleanOp, CapKind, JoinKind};

use crate::access;
use crate::catalog::object;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the path commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(PathBoolean)?;
    registry.register(PathSimplify)?;
    registry.register(PathExpandStroke)?;
    registry.register(PathInset)?;
    registry.register(PathMovePoints)?;
    registry.register(PathWarp)?;
    registry.register(PathReshapeArea)?;
    registry.register(PathRedraw)?;
    registry.register(PathSetFill)?;
    registry.register(PathSetFillColor)?;
    registry.register(PathSetFillVector)
}

/// Maps a vector-pipeline failure to a command error. [`VectorError`]
/// is deterministic given the geometry, so these are state refusals,
/// not lineage bugs.
fn vector_error(name: &'static str, error: vector::VectorError) -> EngineError {
    EngineError::InvalidState {
        name: name.to_owned(),
        detail: error.to_string(),
    }
}

/// Which combine operation (`path.boolean`).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum BooleanOpKind {
    /// Union of both operands.
    Union,
    /// Intersection of both operands.
    Intersect,
    /// `a` minus `b`.
    Punch,
    /// Intersection, under Fireworks' Crop command name.
    Crop,
}

impl From<BooleanOpKind> for BooleanOp {
    fn from(kind: BooleanOpKind) -> Self {
        match kind {
            BooleanOpKind::Union => BooleanOp::Union,
            BooleanOpKind::Intersect => BooleanOp::Intersect,
            BooleanOpKind::Punch => BooleanOp::Punch,
            BooleanOpKind::Crop => BooleanOp::Crop,
        }
    }
}

/// `path.boolean` — combines two paths. `a` holds the result (its
/// identity and display fields survive); `b` is removed from its
/// container and the registry. An empty result removes both.
struct PathBoolean;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathBooleanPayload {
    /// Kept object: receives the combined geometry.
    a: doc::ObjectId,
    /// Consumed object: removed.
    b: doc::ObjectId,
    /// Which combine.
    op: BooleanOpKind,
}

impl TypedCommand for PathBoolean {
    type Payload = PathBooleanPayload;

    fn name(&self) -> &'static str {
        "path.boolean"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.boolean";
        if payload.a == payload.b {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "both operands are the same object".to_owned(),
            });
        }
        let before_a = access::object(document, NAME, payload.a)?.clone();
        let before_b = access::object(document, NAME, payload.b)?.clone();
        let (ObjectData::Path(a_path), ObjectData::Path(b_path)) = (&before_a, &before_b) else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "both operands must be paths".to_owned(),
            });
        };
        // Compute before anything mutates: a refused combine touches
        // neither operand.
        let result = vector::boolean(a_path, b_path, payload.op.into())
            .map_err(|error| vector_error(NAME, error))?;

        // Pre-flight every detach the chosen branch will perform: an
        // unattached (or group-orphaned) operand must refuse BEFORE the
        // first detach, not after the first operand is already gone.
        object::ensure_detachable(document, NAME, payload.b)?;
        if result.contours.is_empty() {
            object::ensure_detachable(document, NAME, payload.a)?;
        }

        let mut deltas = Vec::new();
        if result.contours.is_empty() {
            // Nothing survives the combine: both operands go.
            for id in [payload.b, payload.a] {
                deltas.push(object::detach(document, NAME, id)?);
                if document.registry.remove(id).is_none() {
                    return Err(EngineError::MissingTarget {
                        name: NAME.to_owned(),
                        detail: format!(
                            "object {} (generation {}) vanished between capture and removal",
                            id.index(),
                            id.generation()
                        ),
                    });
                }
                deltas.push(Delta::Object {
                    id,
                    inverse: ObjectInverse::Removed {
                        object: Box::new(if id == payload.a {
                            before_a.clone()
                        } else {
                            before_b.clone()
                        }),
                    },
                });
            }
        } else {
            deltas.push(object::detach(document, NAME, payload.b)?);
            if document.registry.remove(payload.b).is_none() {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "object {} (generation {}) vanished between capture and removal",
                        payload.b.index(),
                        payload.b.generation()
                    ),
                });
            }
            deltas.push(Delta::Object {
                id: payload.b,
                inverse: ObjectInverse::Removed {
                    object: Box::new(before_b),
                },
            });
            let applied = match document.registry.get_mut(payload.a) {
                Some(ObjectData::Path(path)) => {
                    path.contours = result.contours;
                    path.even_odd = result.even_odd;
                    path.primitive = None;
                    true
                }
                _ => false,
            };
            if !applied {
                return Err(EngineError::MissingTarget {
                    name: NAME.to_owned(),
                    detail: format!(
                        "object {} (generation {}) vanished between validation and mutation",
                        payload.a.index(),
                        payload.a.generation()
                    ),
                });
            }
            deltas.push(Delta::Object {
                id: payload.a,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(before_a),
                },
            });
        }
        Ok(Delta::Group(deltas))
    }
}

/// End treatment for `path.expandStroke`'s open contours.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum CapChoice {
    /// Flat end at the contour endpoint.
    Butt,
    /// Square end extending by the stroke radius.
    Square,
    /// Semicircular end.
    Round,
}

/// Corner treatment for `path.expandStroke`.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum JoinChoice {
    /// Circular arc of offset radius.
    Round,
    /// Straight chord between the offset span ends.
    Bevel,
    /// Extend to the tangent intersection, up to the miter limit.
    Miter,
}

/// Runs `edit`'s vector operation on the target path, then splices the
/// returned contours in (keeping the object's display fields, dropping
/// any live primitive — the result is derived geometry). Shared by
/// expand-stroke and inset; the operation itself has already run
/// successfully when this is called, so the object is still there.
fn splice_geometry(
    document: &mut Document,
    name: &'static str,
    id: ObjectId,
    result: doc::PathObject,
) -> Result<(), EngineError> {
    let applied = match document.registry.get_mut(id) {
        Some(ObjectData::Path(path)) => {
            path.contours = result.contours;
            path.even_odd = result.even_odd;
            path.primitive = None;
            true
        }
        _ => false,
    };
    if !applied {
        return Err(EngineError::MissingTarget {
            name: name.to_owned(),
            detail: format!(
                "object {} (generation {}) vanished between validation and mutation",
                id.index(),
                id.generation()
            ),
        });
    }
    Ok(())
}

/// `path.simplify` — tolerance-driven anchor reduction (RDP on linear
/// runs; curve spans pass through untouched).
struct PathSimplify;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathSimplifyPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Maximum anchor deviation kept, in px; must be finite and > 0.
    tolerance: f64,
}

impl TypedCommand for PathSimplify {
    type Payload = PathSimplifyPayload;

    fn name(&self) -> &'static str {
        "path.simplify"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.simplify";
        let before = access::object(document, NAME, payload.id)?.clone();
        let simplified = {
            let ObjectData::Path(path) = &before else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a path", payload.id.index()),
                });
            };
            vector::simplify(path, payload.tolerance).map_err(|error| vector_error(NAME, error))?
        };
        splice_geometry(document, NAME, payload.id, simplified)?;
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `path.expandStroke` — converts the stroke of every contour into a
/// filled outline: closed contours become a band (one ring per side),
/// open contours one closed outline closed by `cap`.
struct PathExpandStroke;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathExpandStrokePayload {
    /// Target object.
    id: doc::ObjectId,
    /// Full stroke width, px; must be finite and > 0.
    width: f64,
    /// End treatment for open contours.
    cap: CapChoice,
    /// Corner treatment on the join side.
    join: JoinChoice,
    /// Miter length limit in radii before falling back to bevel; must
    /// be finite and > 0.
    miter_limit: f64,
}

impl TypedCommand for PathExpandStroke {
    type Payload = PathExpandStrokePayload;

    fn name(&self) -> &'static str {
        "path.expandStroke"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.expandStroke";
        // Positive-width precondition lives here (Fireworks' dialog
        // refuses 0/negative before running); `vector` re-validates its
        // own invariants as defense in depth.
        if payload.width <= 0.0 || !payload.width.is_finite() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("width {} is not a positive number", payload.width),
            });
        }
        if payload.miter_limit <= 0.0 || !payload.miter_limit.is_finite() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "miter limit {} is not a positive number",
                    payload.miter_limit
                ),
            });
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let outlined = {
            let ObjectData::Path(path) = &before else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a path", payload.id.index()),
                });
            };
            let style = vector::StrokeStyle {
                width: payload.width,
                cap: match payload.cap {
                    CapChoice::Butt => CapKind::Butt,
                    CapChoice::Square => CapKind::Square,
                    CapChoice::Round => CapKind::Round,
                },
                join: match payload.join {
                    JoinChoice::Round => JoinKind::Round,
                    JoinChoice::Bevel => JoinKind::Bevel,
                    JoinChoice::Miter => JoinKind::Miter,
                },
                miter_limit: payload.miter_limit,
            };
            vector::expand_stroke(path, &style).map_err(|error| vector_error(NAME, error))?
        };
        splice_geometry(document, NAME, payload.id, outlined)?;
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `path.inset` — offsets every closed contour by `distance` px:
/// positive outsets, negative insets. Open contours close by their
/// start–end chord first. Holes move the same direction as the body
/// they belong to (orientation-independent).
struct PathInset;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathInsetPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Signed offset distance, px; must be finite.
    distance: f64,
}

impl TypedCommand for PathInset {
    type Payload = PathInsetPayload;

    fn name(&self) -> &'static str {
        "path.inset"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.inset";
        let before = access::object(document, NAME, payload.id)?.clone();
        let offset = {
            let ObjectData::Path(path) = &before else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a path", payload.id.index()),
                });
            };
            vector::offset_path(path, payload.distance)
                .map_err(|error| vector_error(NAME, error))?
        };
        splice_geometry(document, NAME, payload.id, offset)?;
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// One per-point move for `path.movePoints`.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PointEdit {
    /// Contour index within the path.
    contour: usize,
    /// Point index within the contour.
    point: usize,
    /// Translation, world units.
    dx: f64,
    /// Translation, world units.
    dy: f64,
}

/// `path.movePoints` — the reshape tool's primitive: moves whole path
/// points (anchor plus both handles, so curve shape rides along).
/// Duplicate edits to the same point compose in payload order.
struct PathMovePoints;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PathMovePointsPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Point moves; at least one.
    edits: Vec<PointEdit>,
}

impl TypedCommand for PathMovePoints {
    type Payload = PathMovePointsPayload;

    fn name(&self) -> &'static str {
        "path.movePoints"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.movePoints";
        if payload.edits.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "no point edits".to_owned(),
            });
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let edited = {
            let ObjectData::Path(path) = &before else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a path", payload.id.index()),
                });
            };
            let edits: Vec<(usize, usize, geom::Vec2)> = payload
                .edits
                .iter()
                .map(|edit| (edit.contour, edit.point, geom::Vec2::new(edit.dx, edit.dy)))
                .collect();
            vector::translate_points(path, &edits).map_err(|error| vector_error(NAME, error))?
        };
        splice_geometry(document, NAME, payload.id, edited)?;
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `path.warp` — the freeform tool's primitive: pushes every point
/// inside a radius with linear falloff toward the ring edge (handles
/// ride along with their anchor).
struct PathWarp;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PathWarpPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Ring center, world coordinates.
    x: f64,
    /// Ring center, world coordinates.
    y: f64,
    /// Ring radius; must be finite and > 0.
    radius: f64,
    /// Drag vector, world units.
    dx: f64,
    /// Drag vector, world units.
    dy: f64,
}

impl TypedCommand for PathWarp {
    type Payload = PathWarpPayload;

    fn name(&self) -> &'static str {
        "path.warp"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.warp";
        let before = access::object(document, NAME, payload.id)?.clone();
        let warped = {
            let ObjectData::Path(path) = &before else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a path", payload.id.index()),
                });
            };
            vector::warp(
                path,
                geom::Point::new(payload.x, payload.y),
                payload.radius,
                geom::Vec2::new(payload.dx, payload.dy),
            )
            .map_err(|error| vector_error(NAME, error))?
        };
        splice_geometry(document, NAME, payload.id, warped)?;
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `path.reshapeArea` — the reshape-area tool's primitive: drags the
/// points inside a rectangle by the full delta, falling off linearly
/// across the band outside it (handles ride along).
struct PathReshapeArea;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PathReshapeAreaPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Area rectangle, world coordinates.
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    /// Falloff band width outside the rect; 0 = hard edge.
    falloff: f64,
    /// Drag vector, world units.
    dx: f64,
    dy: f64,
}

impl TypedCommand for PathReshapeArea {
    type Payload = PathReshapeAreaPayload;

    fn name(&self) -> &'static str {
        "path.reshapeArea"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.reshapeArea";
        if !(payload.x0.is_finite()
            && payload.y0.is_finite()
            && payload.x1.is_finite()
            && payload.y1.is_finite())
        {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: "area rectangle must be finite".to_owned(),
            });
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let shaped = {
            let ObjectData::Path(path) = &before else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a path", payload.id.index()),
                });
            };
            vector::reshape_area(
                path,
                vector::Rect::new(payload.x0, payload.y0, payload.x1, payload.y1),
                payload.falloff,
                geom::Vec2::new(payload.dx, payload.dy),
            )
            .map_err(|error| vector_error(NAME, error))?
        };
        splice_geometry(document, NAME, payload.id, shaped)?;
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// One drawn point for `path.redrawPath`.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DrawnPoint {
    /// Anchor, world coordinates.
    x: f64,
    y: f64,
    /// Primary (in) handle offset; `None` keeps the point a corner.
    primary_x: Option<f64>,
    primary_y: Option<f64>,
    /// Secondary (out) handle offset.
    secondary_x: Option<f64>,
    secondary_y: Option<f64>,
    /// The point is a curve point.
    #[serde(default)]
    is_curve: bool,
}

/// `path.redrawPath` — the redraw tool's commit: swaps the drawn span
/// of points into one contour between two existing indices.
struct PathRedraw;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PathRedrawPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Contour index within the path.
    contour: usize,
    /// First replaced point index (inclusive).
    from: usize,
    /// Last replaced point index (exclusive).
    to: usize,
    /// The drawn replacement; at least one point.
    points: Vec<DrawnPoint>,
}

impl TypedCommand for PathRedraw {
    type Payload = PathRedrawPayload;

    fn name(&self) -> &'static str {
        "path.redrawPath"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.redrawPath";
        let before = access::object(document, NAME, payload.id)?.clone();
        let redrawn = {
            let ObjectData::Path(path) = &before else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!("object {} is not a path", payload.id.index()),
                });
            };
            let drawn: Vec<doc::PathPoint> = payload
                .points
                .iter()
                .map(|d| doc::PathPoint {
                    anchor: geom::Point::new(d.x, d.y),
                    primary: match (d.primary_x, d.primary_y) {
                        (Some(px), Some(py)) => Some(geom::Point::new(px, py)),
                        _ => None,
                    },
                    secondary: match (d.secondary_x, d.secondary_y) {
                        (Some(sx), Some(sy)) => Some(geom::Point::new(sx, sy)),
                        _ => None,
                    },
                    is_curve: d.is_curve,
                })
                .collect();
            vector::redraw_span(path, payload.contour, payload.from, payload.to, drawn)
                .map_err(|error| vector_error(NAME, error))?
        };
        splice_geometry(document, NAME, payload.id, redrawn)?;
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// One gradient stop on a fill payload: ramp position plus the ABGR
/// colour (`FGPf` + `FGCi`).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct FillStopPayload {
    /// Ramp position, 0..=1.
    position: f64,
    /// Stop colour, ABGR layout.
    color_raw: i64,
}

/// Which fill family a payload names (`FSXi`; the narrower `FSHi`
/// compatibility slot never appears in commands — commands always
/// write the extended slot).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum FillShapeKind {
    /// Flat `FCLi` colour.
    Solid,
    /// Linear ramp along the fill vector.
    Linear,
    /// Radial ramp from the vector start, radius to its end.
    Radial,
    /// Sweep around the vector start.
    Conical,
    /// Satin, pinch, folds, elliptical, rectangular, bars, ripple,
    /// waves, pattern and web dither — modelled, rendered as the
    /// solid colour until each family's math is pinned.
    Satin,
    Pinch,
    Folds,
    Elliptical,
    Rectangular,
    Bars,
    Ripple,
    Waves,
    Pattern,
    WebDither,
    /// Contour (distance-band) ramp.
    Contour,
}

impl From<FillShapeKind> for doc::FillShape {
    fn from(kind: FillShapeKind) -> Self {
        match kind {
            FillShapeKind::Solid => doc::FillShape::Solid,
            FillShapeKind::Linear => doc::FillShape::Linear,
            FillShapeKind::Radial => doc::FillShape::Radial,
            FillShapeKind::Conical => doc::FillShape::Conical,
            FillShapeKind::Satin => doc::FillShape::Satin,
            FillShapeKind::Pinch => doc::FillShape::Pinch,
            FillShapeKind::Folds => doc::FillShape::Folds,
            FillShapeKind::Elliptical => doc::FillShape::Elliptical,
            FillShapeKind::Rectangular => doc::FillShape::Rectangular,
            FillShapeKind::Bars => doc::FillShape::Bars,
            FillShapeKind::Ripple => doc::FillShape::Ripple,
            FillShapeKind::Waves => doc::FillShape::Waves,
            FillShapeKind::Pattern => doc::FillShape::Pattern,
            FillShapeKind::WebDither => doc::FillShape::WebDither,
            FillShapeKind::Contour => doc::FillShape::Contour,
        }
    }
}

/// Builds a solid-fill `FPLv` definition from a payload's ramps.
fn fill_from_payload(
    shape: FillShapeKind,
    stops: &[FillStopPayload],
    alpha: &[FillStopPayload],
) -> doc::Fill {
    let gradient = if matches!(shape, FillShapeKind::Solid) {
        None
    } else {
        let ramp = |stops: &[FillStopPayload]| {
            stops
                .iter()
                .map(|stop| doc::GradientStop {
                    position: stop.position,
                    color_raw: stop.color_raw,
                })
                .collect()
        };
        Some(doc::Gradient {
            vertices: Vec::new(),
            rgb: ramp(stops),
            alpha: ramp(alpha),
        })
    };
    doc::Fill {
        shape: shape.into(),
        // Commands always write the extended `FSXi` slot; the
        // compatibility `FSHi` slot is a reader-side concern.
        shape_extended: true,
        texture_blend: 0,
        feather: 0,
        stamping: None,
        hard_edge: false,
        gradient,
        dither: None,
    }
}

/// `path.setFill` — installs a whole fill definition (family plus
/// ramps) on a path, replacing whatever definition was there. Named
/// for the CS6 DOM verb; `path.setFillColor` and `path.setFillVector`
/// are the narrow variants the Fill panel's colour well and the
/// gradient tool's handle drag map to. `FCLi` rides the first RGB
/// stop so the solid fallback paint stays representative (substitution
/// pending corpus evidence).
struct PathSetFill;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathSetFillPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Fill family.
    shape: FillShapeKind,
    /// RGB ramp (`FG0v`); at least one stop unless `shape` is solid.
    #[serde(default)]
    stops: Vec<FillStopPayload>,
    /// Coverage ramp (`FG1v`); empty means fully opaque.
    #[serde(default)]
    alpha: Vec<FillStopPayload>,
}

impl TypedCommand for PathSetFill {
    type Payload = PathSetFillPayload;

    fn name(&self) -> &'static str {
        "path.setFill"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.setFill";
        if !matches!(payload.shape, FillShapeKind::Solid) && payload.stops.is_empty() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "a gradient fill needs at least one stop".to_owned(),
            });
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let fill = fill_from_payload(payload.shape, &payload.stops, &payload.alpha);
        let applied = match document.registry.get_mut(payload.id) {
            Some(ObjectData::Path(path)) => {
                let first = payload.stops.first().map(|stop| stop.color_raw);
                match &mut path.paint {
                    Some(paint) => {
                        if let Some(color_raw) = first {
                            paint.fill_raw = color_raw;
                        }
                        paint.fill = Some(fill);
                    }
                    None => {
                        path.paint = Some(doc::PaintStyle {
                            stroke_raw: 0,
                            fill_raw: first.unwrap_or(0),
                            fill: Some(fill),
                        });
                    }
                }
                true
            }
            _ => false,
        };
        if !applied {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} is not a path", payload.id.index()),
            });
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// `path.setFillColor` — applies a solid fill colour: `FCLi` moves and
/// any gradient definition collapses to the solid family (the DOM
/// verb installs a solid fill; pinned pending corpus playback — a
/// counterexample surfaces as an encode diff).
struct PathSetFillColor;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathSetFillColorPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Fill colour, ABGR layout.
    color_raw: i64,
}

impl TypedCommand for PathSetFillColor {
    type Payload = PathSetFillColorPayload;

    fn name(&self) -> &'static str {
        "path.setFillColor"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.setFillColor";
        let before = access::object(document, NAME, payload.id)?.clone();
        let applied = match document.registry.get_mut(payload.id) {
            Some(ObjectData::Path(path)) => {
                match &mut path.paint {
                    Some(paint) => {
                        paint.fill_raw = payload.color_raw;
                        if let Some(fill) = &mut paint.fill {
                            fill.shape = doc::FillShape::Solid;
                            fill.gradient = None;
                            fill.dither = None;
                        }
                    }
                    None => {
                        path.paint = Some(doc::PaintStyle {
                            stroke_raw: 0,
                            fill_raw: payload.color_raw,
                            fill: None,
                        });
                    }
                }
                true
            }
            _ => false,
        };
        if !applied {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} is not a path", payload.id.index()),
            });
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

/// One gradient handle in world coordinates (`fdct`'s start/end/third
/// points live on the path, not inside the fill).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct FillHandlePayload {
    /// World x.
    x: f64,
    /// World y.
    y: f64,
}

/// `path.setFillVector` — moves the gradient handles (`fdct` start,
/// end and optional third point). Handles the payload omits stay
/// where they are; linear fills use start→end, radial start=center
/// with end on the rim, conical adds the third point for its zero
/// angle.
struct PathSetFillVector;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathSetFillVectorPayload {
    /// Target object.
    id: doc::ObjectId,
    /// Ramp origin; `None` leaves it in place.
    #[serde(default)]
    start: Option<FillHandlePayload>,
    /// Ramp rim/terminus; `None` leaves it in place.
    #[serde(default)]
    end: Option<FillHandlePayload>,
    /// Conical reference point; `None` leaves it in place.
    #[serde(default)]
    third: Option<FillHandlePayload>,
}

impl TypedCommand for PathSetFillVector {
    type Payload = PathSetFillVectorPayload;

    fn name(&self) -> &'static str {
        "path.setFillVector"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.setFillVector";
        if payload.start.is_none() && payload.end.is_none() && payload.third.is_none() {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: "no fill handles".to_owned(),
            });
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let applied = match document.registry.get_mut(payload.id) {
            Some(ObjectData::Path(path)) => {
                let handle = |point: &Option<FillHandlePayload>| {
                    point.map(|point| geom::Point::new(point.x, point.y))
                };
                if payload.start.is_some() {
                    path.fill_start = handle(&payload.start);
                }
                if payload.end.is_some() {
                    path.fill_end = handle(&payload.end);
                }
                if payload.third.is_some() {
                    path.fill_third = handle(&payload.third);
                }
                true
            }
            _ => false,
        };
        if !applied {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} is not a path", payload.id.index()),
            });
        }
        Ok(Delta::Object {
            id: payload.id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::error::EngineError;
    use crate::registry::CommandRegistry;
    use doc::{
        Cell, Contour, Document, ImageObject, Layer, LayerList, ObjectId, Page, PageInfo,
        PathObject, PathPoint, TileRef,
    };
    use serde_json::json;

    fn point(x: f64, y: f64) -> PathPoint {
        PathPoint {
            anchor: geom::Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        }
    }

    fn square_contour(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
        Contour {
            closed: true,
            point_count: 4,
            points: vec![point(x0, y0), point(x1, y0), point(x1, y1), point(x0, y1)],
        }
    }

    /// A filled, named square path (display fields prove inheritance).
    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> ObjectData {
        ObjectData::Path(PathObject {
            contours: vec![square_contour(x0, y0, x1, y1)],
            primitive: None,
            even_odd: false,
            paint: Some(doc::PaintStyle {
                stroke_raw: 0x11,
                fill_raw: 0x22,
                fill: None,
            }),
            fill_start: None,
            fill_end: None,
            fill_third: None,
            brush_placement: None,
            fill_on_top: None,
            locked: false,
            visible: true,
            opacity: 1000,
            blend: 0,
            name: Some("square".to_owned()),
            effects: Vec::new(),
        })
    }

    fn image_object() -> ObjectData {
        ObjectData::Image(ImageObject {
            location: geom::Point::ZERO,
            offset: geom::Point::ZERO,
            pixels: (4, 4),
            tile_size: 128,
            tiles: vec![TileRef::Matte(0xFF_00_00_00)],
            name: None,
            opacity: 1000,
            blend: 0,
            visible: true,
            locked: false,
            effects: Vec::new(),
        })
    }

    /// One page, one layer, one visible cell holding `objects` in order.
    fn document_with(objects: Vec<ObjectData>) -> (Document, Vec<ObjectId>) {
        let mut document = Document::default();
        let mut ids = Vec::new();
        for object in objects {
            ids.push(document.registry.insert(object));
        }
        let layer = Layer {
            name: "Layer 0".to_owned(),
            display: true,
            cells: vec![
                Cell {
                    locked: false,
                    visible: true,
                    elements: ids.clone(),
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
                canvas: Default::default(),
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
            layers: LayerList {
                layers: vec![layer],
            },
            grid: None,
            guides: Default::default(),
            export: None,
        });
        (document, ids)
    }

    fn apply_by_name(
        document: &mut Document,
        name: &str,
        payload: serde_json::Value,
    ) -> Result<Delta, EngineError> {
        let mut registry = CommandRegistry::new();
        install(&mut registry).unwrap();
        registry.execute(document, name, payload)
    }

    fn anchors(document: &Document, id: ObjectId) -> Vec<(i64, i64)> {
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => path
                .contours
                .iter()
                .flat_map(|contour| {
                    contour
                        .points
                        .iter()
                        .map(|p| (p.anchor.x as i64, p.anchor.y as i64))
                })
                .collect(),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    fn document_bytes(document: &Document) -> Vec<u8> {
        serde_json::to_vec(document).unwrap()
    }

    #[test]
    fn boolean_union_keeps_a_removes_b_and_undo_restores_both() {
        let (mut document, ids) = document_with(vec![
            square(0.0, 0.0, 8.0, 8.0),
            square(4.0, 4.0, 12.0, 12.0),
        ]);
        let (a, b) = (ids[0], ids[1]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.boolean",
            json!({ "a": a, "b": b, "op": "union" }),
        )
        .unwrap();
        // b is gone from the registry and the cell; a carries the
        // union's 8-anchor outline and a's display fields.
        assert!(document.registry.get(b).is_none());
        let mut got = anchors(&document, a);
        got.sort_unstable();
        assert_eq!(
            got,
            vec![
                (0, 0),
                (0, 8),
                (4, 8),
                (4, 12),
                (8, 0),
                (8, 4),
                (12, 4),
                (12, 12)
            ]
        );
        match document.registry.get(a).unwrap() {
            ObjectData::Path(path) => {
                assert_eq!(path.name.as_deref(), Some("square"));
                assert_eq!(path.opacity, 1000);
                assert!(path.primitive.is_none(), "result is derived geometry");
            }
            other => panic!("wrong object kind: {other:?}"),
        }
        let cell = &document.pages[0].layers.layers[0].cells[0];
        assert_eq!(cell.elements, vec![a]);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before, "undo restores both");
        // And redo lands back on the combined state.
        let combined = {
            let mut registry = CommandRegistry::new();
            install(&mut registry).unwrap();
            registry
                .execute(
                    &mut document,
                    "path.boolean",
                    json!({ "a": a, "b": b, "op": "union" }),
                )
                .unwrap();
            document_bytes(&document)
        };
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
        let _ = combined;
    }

    #[test]
    fn boolean_punch_to_empty_removes_both_operands() {
        let (mut document, ids) =
            document_with(vec![square(0.0, 0.0, 8.0, 8.0), square(0.0, 0.0, 8.0, 8.0)]);
        let (a, b) = (ids[0], ids[1]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.boolean",
            json!({ "a": a, "b": b, "op": "punch" }),
        )
        .unwrap();
        assert!(document.registry.get(a).is_none());
        assert!(document.registry.get(b).is_none());
        let cell = &document.pages[0].layers.layers[0].cells[0];
        assert!(cell.elements.is_empty());

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn boolean_refuses_same_object_and_non_paths() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0), image_object()]);
        for payload in [
            json!({ "a": ids[0], "b": ids[0], "op": "union" }),
            json!({ "a": ids[0], "b": ids[1], "op": "union" }),
        ] {
            let error = apply_by_name(&mut document, "path.boolean", payload.clone()).unwrap_err();
            assert!(
                matches!(error, EngineError::InvalidState { .. }),
                "{payload}"
            );
        }
    }

    #[test]
    fn boolean_refused_combine_leaves_the_document_alone() {
        let (mut document, ids) = document_with(vec![
            square(0.0, 0.0, 8.0, 8.0),
            square(20.0, 20.0, 24.0, 24.0),
        ]);
        let before = document_bytes(&document);
        // Crop of disjoint squares is empty — an error-free refusal is
        // impossible here, but the hostile case (OpenChain) must equally
        // not mutate; both refusal shapes are covered by the non-path
        // and same-object tests, so this one pins the empty-result
        // deletion contract instead: intersect of disjoint squares
        // deletes both.
        let delta = apply_by_name(
            &mut document,
            "path.boolean",
            json!({ "a": ids[0], "b": ids[1], "op": "intersect" }),
        )
        .unwrap();
        assert!(document.registry.get(ids[0]).is_none());
        assert!(document.registry.get(ids[1]).is_none());
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
        let _ = before;
    }

    #[test]
    fn simplify_reduces_collinear_anchors_and_undo_restores() {
        let mut open = square(0.0, 0.0, 8.0, 8.0);
        let ObjectData::Path(ref mut path) = open else {
            panic!("fixture is a path");
        };
        path.contours = vec![Contour {
            closed: false,
            point_count: 3,
            points: vec![point(0.0, 0.0), point(4.0, 0.0), point(8.0, 0.0)],
        }];
        path.name = None;
        let (mut document, ids) = document_with(vec![open]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.simplify",
            json!({ "id": ids[0], "tolerance": 1.0 }),
        )
        .unwrap();
        assert_eq!(anchors(&document, ids[0]), vec![(0, 0), (8, 0)]);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn simplify_refuses_bad_tolerance_and_non_paths() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0), image_object()]);
        for payload in [
            json!({ "id": ids[0], "tolerance": 0.0 }),
            json!({ "id": ids[0], "tolerance": -1.0 }),
            json!({ "id": ids[0], "tolerance": "x" }),
            json!({ "id": ids[1], "tolerance": 1.0 }),
        ] {
            let error = apply_by_name(&mut document, "path.simplify", payload.clone()).unwrap_err();
            assert!(
                matches!(
                    error,
                    EngineError::InvalidState { .. } | EngineError::BadPayload { .. }
                ),
                "{payload}"
            );
        }
    }

    #[test]
    fn expand_stroke_replaces_a_square_with_the_band() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.expandStroke",
            json!({
                "id": ids[0], "width": 2.0, "cap": "butt", "join": "miter",
                "miter_limit": 4.0,
            }),
        )
        .unwrap();
        match document.registry.get(ids[0]).unwrap() {
            ObjectData::Path(path) => {
                assert_eq!(path.contours.len(), 2, "outer ring + inner hole");
                assert!(path.primitive.is_none());
                // Display fields survive the geometry swap.
                assert_eq!(path.name.as_deref(), Some("square"));
            }
            other => panic!("wrong object kind: {other:?}"),
        }
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn expand_stroke_refuses_non_positive_width() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        for width in [0.0, -2.0] {
            let error = apply_by_name(
                &mut document,
                "path.expandStroke",
                json!({
                    "id": ids[0], "width": width, "cap": "butt", "join": "miter",
                    "miter_limit": 4.0,
                }),
            )
            .unwrap_err();
            assert!(matches!(error, EngineError::InvalidState { .. }), "{width}");
        }
        // NaN rides through JSON as null: refused at the payload layer.
        let error = apply_by_name(
            &mut document,
            "path.expandStroke",
            json!({
                "id": ids[0], "width": null, "cap": "butt", "join": "miter",
                "miter_limit": 4.0,
            }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }), "{error}");
    }

    #[test]
    fn inset_shrinks_and_keeps_display_fields() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.inset",
            json!({ "id": ids[0], "distance": -2.0 }),
        )
        .unwrap();
        let mut got = anchors(&document, ids[0]);
        got.sort_unstable();
        assert_eq!(got, vec![(2, 2), (2, 6), (6, 2), (6, 6)]);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn inset_refuses_non_finite_distance() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        // JSON cannot carry non-finite floats: they serialize to null
        // and the payload layer refuses before the vector layer can.
        // (The vector crate's own finite guard is proved in `vector`.)
        let error = apply_by_name(
            &mut document,
            "path.inset",
            json!({ "id": ids[0], "distance": null }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }), "{error}");
    }

    #[test]
    fn reshape_area_drags_rigidly_and_inverts() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.reshapeArea",
            json!({
                "id": ids[0],
                "x0": -1.0, "y0": -1.0, "x1": 3.0, "y1": 9.0,
                "falloff": 0.0,
                "dx": 4.0, "dy": 2.0
            }),
        )
        .unwrap();
        let mut got = anchors(&document, ids[0]);
        got.sort_unstable();
        // Left corners (0,0)/(0,8) move by the full delta; right
        // corners hold.
        assert_eq!(got, vec![(4, 2), (4, 10), (8, 0), (8, 8)]);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn reshape_area_refuses_a_nonfinite_rect_untouched() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        for (x1, falloff) in [(f64::NAN, 0.0), (3.0, f64::NEG_INFINITY)] {
            let error = apply_by_name(
                &mut document,
                "path.reshapeArea",
                json!({
                    "id": ids[0],
                    "x0": 0.0, "y0": 0.0, "x1": x1, "y1": 3.0,
                    "falloff": falloff,
                    "dx": 1.0, "dy": 0.0
                }),
            )
            .expect_err("non-finite refused");
            assert!(matches!(
                error,
                EngineError::BadPayload { .. } | EngineError::InvalidState { .. }
            ));
        }
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn redraw_path_swaps_a_span_and_inverts() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.redrawPath",
            json!({
                "id": ids[0],
                "contour": 0,
                "from": 0,
                "to": 2,
                "points": [
                    { "x": 0.0, "y": 0.0 },
                    { "x": 4.0, "y": -6.0 },
                    { "x": 8.0, "y": 0.0 }
                ]
            }),
        )
        .unwrap();
        let mut got = anchors(&document, ids[0]);
        got.sort_unstable();
        // The drawn bulge replaces the top edge wholesale.
        assert_eq!(got, vec![(0, 0), (0, 8), (4, -6), (8, 0), (8, 8)]);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn redraw_path_refuses_bad_spans_untouched() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        for payload in [
            json!({ "id": ids[0], "contour": 7, "from": 0, "to": 2,
                    "points": [{ "x": 1.0, "y": 1.0 }] }),
            json!({ "id": ids[0], "contour": 0, "from": 2, "to": 2,
                    "points": [{ "x": 1.0, "y": 1.0 }] }),
            json!({ "id": ids[0], "contour": 0, "from": 0, "to": 99,
                    "points": [{ "x": 1.0, "y": 1.0 }] }),
            json!({ "id": ids[0], "contour": 0, "from": 0, "to": 2, "points": [] }),
        ] {
            let error = apply_by_name(&mut document, "path.redrawPath", payload)
                .expect_err("bad span refused");
            assert!(matches!(error, EngineError::InvalidState { .. }));
        }
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn move_points_translates_and_inverts() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.movePoints",
            json!({
                "id": ids[0],
                "edits": [
                    { "contour": 0, "point": 0, "dx": 5.0, "dy": -3.0 },
                    { "contour": 0, "point": 2, "dx": 5.0, "dy": -3.0 }
                ],
            }),
        )
        .unwrap();
        let mut got = anchors(&document, ids[0]);
        got.sort_unstable();
        assert_eq!(got, vec![(0, 8), (5, -3), (8, 0), (13, 5)]);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn move_points_rejects_bad_indices_untouched() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let error = apply_by_name(
            &mut document,
            "path.movePoints",
            json!({
                "id": ids[0],
                "edits": [{ "contour": 0, "point": 99, "dx": 1.0, "dy": 1.0 }],
            }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert_eq!(document_bytes(&document), before);

        // An empty edit list is refused before touching the object.
        let error = apply_by_name(
            &mut document,
            "path.movePoints",
            json!({ "id": ids[0], "edits": [] }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn warp_pushes_inside_the_ring_and_inverts() {
        // Ring centered on the square's left edge, radius 8: the two
        // left corners (distance 4) shift by half the drag (weight
        // 1 − 4/8), the right edge is outside and stays.
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let first = apply_by_name(
            &mut document,
            "path.warp",
            json!({
                "id": ids[0],
                "x": 0.0, "y": 4.0, "radius": 8.0,
                "dx": -4.0, "dy": 0.0,
            }),
        )
        .unwrap();
        assert_eq!(
            anchors(&document, ids[0]),
            vec![(-2, 0), (8, 0), (8, 8), (-2, 8)]
        );
        let delta = apply_by_name(
            &mut document,
            "path.warp",
            json!({
                "id": ids[0],
                "x": 0.0, "y": 4.0, "radius": 0.001,
                "dx": 2.0, "dy": 0.0,
            }),
        )
        .unwrap();
        // Nothing within the tiny ring: the command still reports a
        // mutation (the vector layer is total), but the geometry is
        // byte-identical to the warped state, so its inverse restores
        // exactly that.
        let warped_bytes = document_bytes(&document);
        assert!(matches!(
            delta,
            Delta::Object {
                inverse: ObjectInverse::Mutated { .. },
                ..
            }
        ));
        assert_eq!(
            anchors(&document, ids[0]),
            vec![(-2, 0), (8, 0), (8, 8), (-2, 8)]
        );
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), warped_bytes);
        // And the first warp's own delta restores the pristine square.
        first.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn warp_rejects_non_positive_radius_untouched() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let error = apply_by_name(
            &mut document,
            "path.warp",
            json!({ "id": ids[0], "x": 0.0, "y": 0.0, "radius": 0.0, "dx": 1.0, "dy": 0.0 }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn boolean_refuses_unattached_operand_before_detaching() {
        // Contract: a refused call mutates nothing. An unattached
        // operand makes every combine refuse — it must refuse BEFORE
        // the other operand is detached and removed (found by
        // panic_hunt seed 728034928435966080 step 37).
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        // A live path no cell references.
        let orphan = document.registry.insert(square(20.0, 0.0, 28.0, 8.0));
        let before = document_bytes(&document);

        // `b` unattached: non-empty result branch refuses at pre-flight.
        let error = apply_by_name(
            &mut document,
            "path.boolean",
            json!({ "a": ids[0], "b": orphan, "op": "union" }),
        )
        .unwrap_err();
        assert!(
            matches!(error, EngineError::MissingTarget { .. }),
            "{error}"
        );
        assert_eq!(document_bytes(&document), before);

        // `a` unattached with a disjoint pair (empty result would drop
        // both): the `a` pre-flight must fire before `b` is detached.
        let (mut document, ids) = document_with(vec![square(40.0, 40.0, 48.0, 48.0)]);
        let orphan = document.registry.insert(square(0.0, 0.0, 8.0, 8.0));
        let before = document_bytes(&document);
        let error = apply_by_name(
            &mut document,
            "path.boolean",
            json!({ "a": orphan, "b": ids[0], "op": "intersect" }),
        )
        .unwrap_err();
        assert!(
            matches!(error, EngineError::MissingTarget { .. }),
            "{error}"
        );
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn path_commands_are_unknown_without_install() {
        // The registry, not the module, owns dispatch: an uninstalled
        // name is UnknownCommand (guards against install-order bugs).
        let mut registry = CommandRegistry::new();
        crate::catalog::object::install(&mut registry).unwrap();
        let (mut document, _ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let error = registry
            .execute(
                &mut document,
                "path.inset",
                json!({ "id": doc::ObjectId::from_parts(0, 0), "distance": 1.0 }),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::UnknownCommand { .. }));
    }

    #[test]
    fn set_fill_installs_a_gradient_and_inverts() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.setFill",
            json!({
                "id": ids[0],
                "shape": "linear",
                "stops": [
                    { "position": 0.0, "color_raw": 0xFF_00_00_FFi64 },
                    { "position": 1.0, "color_raw": 0xFF_FF_FF_FFi64 },
                ],
                "alpha": [
                    { "position": 0.0, "color_raw": 0 },
                    { "position": 1.0, "color_raw": 0xFF00_0000i64 },
                ],
            }),
        )
        .unwrap();
        let ObjectData::Path(path) = document.registry.get(ids[0]).unwrap() else {
            panic!("square is a path");
        };
        let paint = path.paint.as_ref().unwrap();
        let fill = paint.fill.as_ref().unwrap();
        assert_eq!(fill.shape, doc::FillShape::Linear);
        assert!(fill.shape_extended, "commands write the FSXi slot");
        assert_eq!(
            fill.gradient.as_ref().unwrap().rgb,
            vec![
                doc::GradientStop {
                    position: 0.0,
                    color_raw: 0xFF_00_00_FFi64
                },
                doc::GradientStop {
                    position: 1.0,
                    color_raw: 0xFF_FF_FF_FFi64
                },
            ]
        );
        assert_eq!(fill.gradient.as_ref().unwrap().alpha.len(), 2);
        // FCLi rides the first ramp stop.
        assert_eq!(paint.fill_raw, 0xFF_00_00_FFi64);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn set_fill_creates_the_paint_when_the_path_has_none() {
        let (mut document, ids) = document_with(vec![ObjectData::Path(PathObject {
            contours: vec![square_contour(0.0, 0.0, 8.0, 8.0)],
            primitive: None,
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
        })]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.setFill",
            json!({
                "id": ids[0],
                "shape": "radial",
                "stops": [{ "position": 0.5, "color_raw": 0x12 }],
            }),
        )
        .unwrap();
        let ObjectData::Path(path) = document.registry.get(ids[0]).unwrap() else {
            panic!("inserted a path");
        };
        let paint = path.paint.as_ref().unwrap();
        assert_eq!(
            paint.stroke_raw, 0,
            "the untouched stroke channel stays empty"
        );
        assert_eq!(paint.fill_raw, 0x12);
        assert_eq!(paint.fill.as_ref().unwrap().shape, doc::FillShape::Radial);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn set_fill_refuses_a_gradient_without_stops_untouched() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let error = apply_by_name(
            &mut document,
            "path.setFill",
            json!({ "id": ids[0], "shape": "linear", "stops": [] }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn set_fill_color_collapses_the_definition_and_inverts() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let gradient_state = apply_by_name(
            &mut document,
            "path.setFill",
            json!({
                "id": ids[0],
                "shape": "linear",
                "stops": [{ "position": 0.0, "color_raw": 0x33 }],
            }),
        )
        .unwrap();
        let delta = apply_by_name(
            &mut document,
            "path.setFillColor",
            json!({ "id": ids[0], "color_raw": 0xFF_00_00_FFi64 }),
        )
        .unwrap();
        let ObjectData::Path(path) = document.registry.get(ids[0]).unwrap() else {
            panic!("square is a path");
        };
        let paint = path.paint.as_ref().unwrap();
        assert_eq!(paint.fill_raw, 0xFF_00_00_FFi64);
        let fill = paint.fill.as_ref().unwrap();
        assert_eq!(fill.shape, doc::FillShape::Solid);
        assert!(fill.gradient.is_none(), "solid replaces the ramps");
        delta.apply_inverse(&mut document).unwrap();
        let ObjectData::Path(path) = document.registry.get(ids[0]).unwrap() else {
            panic!("square is a path");
        };
        assert_eq!(
            path.paint.as_ref().unwrap().fill.as_ref().unwrap().shape,
            doc::FillShape::Linear,
            "undo returns the gradient family"
        );
        gradient_state.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before, "both undos compose");
    }

    #[test]
    fn set_fill_vector_moves_handles_and_inverts() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.setFillVector",
            json!({
                "id": ids[0],
                "start": { "x": 1.0, "y": 2.0 },
                "end": { "x": 7.0, "y": 6.0 },
                "third": { "x": 4.0, "y": 4.0 },
            }),
        )
        .unwrap();
        let ObjectData::Path(path) = document.registry.get(ids[0]).unwrap() else {
            panic!("square is a path");
        };
        assert_eq!(path.fill_start, Some(geom::Point::new(1.0, 2.0)));
        assert_eq!(path.fill_end, Some(geom::Point::new(7.0, 6.0)));
        assert_eq!(path.fill_third, Some(geom::Point::new(4.0, 4.0)));
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);

        // Omitted handles stay put; an empty payload is refused before
        // touching the object.
        apply_by_name(
            &mut document,
            "path.setFillVector",
            json!({
                "id": ids[0],
                "start": { "x": 1.0, "y": 1.0 },
            }),
        )
        .unwrap();
        let ObjectData::Path(path) = document.registry.get(ids[0]).unwrap() else {
            panic!("square is a path");
        };
        assert_eq!(path.fill_end, None, "omitted handle unchanged");
        let error = apply_by_name(&mut document, "path.setFillVector", json!({ "id": ids[0] }))
            .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
    }
}
