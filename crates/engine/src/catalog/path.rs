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
    registry.register(PathSetFillVector)?;
    registry.register(PathSetFillEdgeMode)?;
    registry.register(PathSetPatternFill)?;
    registry.register(PathCropSelection)?;
    registry.register(PathSplitPaths)?;
    registry.register(PathKnifePoint)?;
    registry.register(PathKnifePoints)
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
        pattern: None,
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
/// verb installs a solid fill; resolved against the documented model
/// in `docs/decisions/0018-setfillcolor-solid-collapse.md` — a Wine
/// oracle counterexample would surface as an encode diff).
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

/// The fill edge (`FEFi`/`FRDb`) a payload selects — CS6's
/// `setFillEdgeMode` strings.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum FillEdgeMode {
    /// `FRDb` set: no anti-aliasing on the fill boundary.
    Hard,
    /// `FEFi` 0, anti-aliased boundary.
    Antialias,
    /// `FEFi` = the feather amount, anti-aliased boundary.
    Feather,
}

/// `path.setFillEdgeMode` — CS6
/// `dom.setFillEdgeMode(edgemode, featherAmt)`: "Sets the edge type for
/// selected items with fills." Values map onto the two `FPLv` slots:
/// `hard` sets `FRDb` and clears `FEFi`; `antialias` clears both;
/// `feather` clears `FRDb` and stores the amount in `FEFi`. A path
/// without an `FPLv` subtree gains one (a solid definition with the
/// edge set — the panel does the same when you open Edge on a solid
/// fill).
struct PathSetFillEdgeMode;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathSetFillEdgeModePayload {
    /// Target object.
    id: doc::ObjectId,
    /// The edge treatment.
    mode: FillEdgeMode,
    /// Feather width in pixels; read only for `mode: "feather"`.
    #[serde(default)]
    feather: i64,
}

impl TypedCommand for PathSetFillEdgeMode {
    type Payload = PathSetFillEdgeModePayload;

    fn name(&self) -> &'static str {
        "path.setFillEdgeMode"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.setFillEdgeMode";
        if payload.mode == FillEdgeMode::Feather && payload.feather < 0 {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!("feather amount {} is negative", payload.feather),
            });
        }
        let before = access::object(document, NAME, payload.id)?.clone();
        let (hard_edge, feather) = match payload.mode {
            FillEdgeMode::Hard => (true, 0),
            FillEdgeMode::Antialias => (false, 0),
            FillEdgeMode::Feather => (false, payload.feather),
        };
        let applied = match document.registry.get_mut(payload.id) {
            Some(ObjectData::Path(path)) => {
                match &mut path.paint {
                    Some(paint) => match &mut paint.fill {
                        Some(fill) => {
                            fill.hard_edge = hard_edge;
                            fill.feather = feather;
                        }
                        None => {
                            paint.fill = Some(doc::Fill {
                                shape: doc::FillShape::Solid,
                                shape_extended: true,
                                texture_blend: 0,
                                feather,
                                stamping: None,
                                hard_edge,
                                gradient: None,
                                dither: None,
                                pattern: None,
                            });
                        }
                    },
                    // A path with no paint at all still carries a fill
                    // edge once the panel sets one; mirror setFill's
                    // materialization with the CS6 default paint.
                    None => {
                        path.paint = Some(doc::PaintStyle {
                            stroke_raw: 0,
                            fill_raw: 0xFF00_0000,
                            fill: Some(doc::Fill {
                                shape: doc::FillShape::Solid,
                                shape_extended: true,
                                texture_blend: 0,
                                feather,
                                stamping: None,
                                hard_edge,
                                gradient: None,
                                dither: None,
                                pattern: None,
                            }),
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

/// `path.cropSelection` — CS6 `dom.cropSelection(boundingRectangle)`:
/// "Crops the selection to the specified rectangle." Each selected
/// path is combined with the rectangle (the `Crop` boolean —
/// intersection with Fireworks' combine order), replaced in place,
/// `path.setPatternFill` — authors a pattern fill from an image and
/// installs it on a path. The CS6 Fill panel's pattern authoring has
/// no DOM verb (the panel writes `pathAttributes.fill` directly; the
/// parity matrix carries no authoring row), so this is the command
/// surface for that property write: the source image's raster becomes
/// the pattern tile at its native pixel extent, minted into the bank
/// with the flatten law (decision 0062).
struct PathSetPatternFill;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathSetPatternFillPayload {
    /// Target path.
    id: doc::ObjectId,
    /// Source image: its raster becomes the pattern tile.
    source: doc::ObjectId,
    /// Internal pattern name (`INMs`); defaults to the image's name.
    name: Option<String>,
}

impl TypedCommand for PathSetPatternFill {
    type Payload = PathSetPatternFillPayload;

    fn name(&self) -> &'static str {
        "path.setPatternFill"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.setPatternFill";
        // Preflights before the first mutation: the target must be a
        // path, the source an image whose extent `pattern_brush` would
        // accept at render — refusing at write, not at draw.
        let before = access::object(document, NAME, payload.id)?.clone();
        if !matches!(document.registry.get(payload.id), Some(ObjectData::Path(_))) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} is not a path", payload.id.index()),
            });
        }
        let Some(ObjectData::Image(image)) = document.registry.get(payload.source) else {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("object {} is not an image", payload.source.index()),
            });
        };
        if image.pixels.0 > 8192 || image.pixels.1 > 8192 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "pattern extent {}x{} is not a usable raster",
                    image.pixels.0, image.pixels.1
                ),
            });
        }
        let pixels = crate::bitmap::assemble_rgba(document, image, NAME)?;
        let (width, height) = image.pixels;
        let pattern_name = payload
            .name
            .clone()
            .or_else(|| image.name.clone())
            .unwrap_or_else(|| "Pattern".to_owned());
        let (tiles, minted) = crate::bitmap::retile_rgba(document, &pixels, width, height);
        let fill = doc::Fill {
            shape: doc::FillShape::Pattern,
            shape_extended: true,
            texture_blend: 0,
            feather: 0,
            stamping: None,
            hard_edge: false,
            gradient: None,
            dither: None,
            pattern: Some(doc::PatternTile {
                name: Some(pattern_name),
                pixels: (width, height),
                tile_size: i64::from(raster::TILE_SIDE as u32),
                tiles,
            }),
        };
        let applied = match document.registry.get_mut(payload.id) {
            Some(ObjectData::Path(path)) => {
                match &mut path.paint {
                    Some(paint) => paint.fill = Some(fill),
                    None => {
                        path.paint = Some(doc::PaintStyle {
                            stroke_raw: 0,
                            fill_raw: 0,
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
        // The bank capture rides LAST: undo restores the paint and
        // drops the minted slots in one step.
        Ok(Delta::Group(vec![
            Delta::Object {
                id: payload.id,
                inverse: ObjectInverse::Mutated {
                    object: Box::new(before),
                },
            },
            Delta::Bank { added: minted },
        ]))
    }
}

/// keeping its display fields; a path entirely outside the rectangle
/// leaves nothing behind and is removed, as `path.boolean` does.
/// Non-path members refuse the whole command: cropping bitmaps is a
/// pixel operation (the bake debt), and groups/text/slices have no
/// crop semantics on the wire.
struct PathCropSelection;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathCropSelectionPayload {
    /// The crop rectangle as `[x0, y0, x1, y1]`, world coordinates.
    rect: [f64; 4],
}

impl TypedCommand for PathCropSelection {
    type Payload = PathCropSelectionPayload;

    fn name(&self) -> &'static str {
        "path.cropSelection"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.cropSelection";
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
        if document.object_selection.is_empty() {
            return Ok(Delta::Noop);
        }
        let selection = document.object_selection.clone();
        let blade = doc::PathObject {
            contours: super::object::rounded_rect_contour(geom::Rect::new(x0, y0, x1, y1), 0.0),
            even_odd: false,
            primitive: None,
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
        };
        // Compute every crop before any mutation: one hostile member
        // refuses the whole command.
        let mut crops = Vec::new();
        for id in &selection {
            let Some(ObjectData::Path(path)) = document.registry.get(*id) else {
                return Err(EngineError::InvalidState {
                    name: NAME.to_owned(),
                    detail: format!(
                        "selected object {} is not a path; only paths crop",
                        id.index()
                    ),
                });
            };
            let cropped = vector::boolean(path, &blade, BooleanOp::Crop)
                .map_err(|error| vector_error(NAME, error))?;
            crops.push(cropped);
        }
        let mut deltas = Vec::new();
        // Pre-flight the removals: an unattached (or group-orphaned)
        // member must refuse before the first crop lands.
        for (id, cropped) in selection.iter().zip(&crops) {
            if cropped.contours.is_empty() {
                object::ensure_detachable(document, NAME, *id)?;
            }
        }
        for (id, cropped) in selection.iter().zip(crops) {
            let before = access::object(document, NAME, *id)?.clone();
            if cropped.contours.is_empty() {
                deltas.push(object::detach(document, NAME, *id)?);
                if document.registry.remove(*id).is_none() {
                    return Err(EngineError::MissingTarget {
                        name: NAME.to_owned(),
                        detail: format!("object {} vanished mid-crop", id.index()),
                    });
                }
                deltas.push(Delta::Object {
                    id: *id,
                    inverse: ObjectInverse::Removed {
                        object: Box::new(before),
                    },
                });
            } else {
                if let Some(ObjectData::Path(path)) = document.registry.get_mut(*id) {
                    path.contours = cropped.contours;
                    path.even_odd = cropped.even_odd;
                    path.primitive = None;
                }
                deltas.push(Delta::Object {
                    id: *id,
                    inverse: ObjectInverse::Mutated {
                        object: Box::new(before),
                    },
                });
            }
        }
        Ok(Delta::Group(deltas))
    }
}

/// The split contract shared by `path.splitPaths` and the knife verbs
/// (decision 0032): each changed path keeps its identity on the first
/// resulting contour; further contours become fresh paths attached
/// right after it in the same cell, inheriting the display fields.
/// The results take over the selection (CS6 leaves the pieces
/// selected); untouched selected paths stay selected where they were.
fn replace_with_pieces(
    document: &mut Document,
    name: &'static str,
    order: &[ObjectId],
    splits: &[(ObjectId, Vec<doc::Contour>)],
) -> Result<Delta, EngineError> {
    let mut deltas = vec![Delta::ObjectSelection {
        before: document.object_selection.clone(),
    }];
    let mut pieces_of: Vec<(ObjectId, Vec<ObjectId>)> = Vec::new();
    for (id, contours) in splits {
        let before = access::object(document, name, *id)?.clone();
        let template = match &before {
            ObjectData::Path(path) => path.clone(),
            _ => {
                return Err(EngineError::InvalidState {
                    name: name.to_owned(),
                    detail: format!("selected object {} is not a path", id.index()),
                });
            }
        };
        let Some(object::Owner::Cell {
            page,
            layer,
            cell,
            position,
        }) = object::find_owner(document, *id)
        else {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!(
                    "selected object {} must sit in a cell (not nested in groups)",
                    id.index()
                ),
            });
        };
        let mut piece_ids = Vec::new();
        for (offset, contour) in contours.iter().enumerate().skip(1) {
            let piece = doc::PathObject {
                contours: vec![contour.clone()],
                primitive: None,
                ..template.clone()
            };
            let piece_id = document.registry.insert(ObjectData::Path(piece));
            deltas.push(object::attach_cell(
                document,
                name,
                page,
                layer,
                cell,
                piece_id,
                Some(position + offset),
            )?);
            deltas.push(Delta::Object {
                id: piece_id,
                inverse: ObjectInverse::Created,
            });
            piece_ids.push(piece_id);
        }
        if let Some(ObjectData::Path(path)) = document.registry.get_mut(*id) {
            path.contours = vec![contours[0].clone()];
            path.primitive = None;
        }
        deltas.push(Delta::Object {
            id: *id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before),
            },
        });
        pieces_of.push((*id, piece_ids));
    }
    // Selection: split results replace their original's slot; paths the
    // command did not touch keep theirs.
    document.object_selection = order
        .iter()
        .flat_map(
            |id| match pieces_of.iter().find(|(split_id, _)| split_id == id) {
                Some((original, piece_ids)) => {
                    let mut pieces = vec![*original];
                    pieces.extend_from_slice(piece_ids);
                    pieces
                }
                None => vec![*id],
            },
        )
        .collect();
    Ok(Delta::Group(deltas))
}

/// Pre-flights the selection for a split-family command: every member
/// must be a cell-placed path. Returns the selection order, or an
/// empty vec for an empty selection.
fn split_selection(document: &Document, name: &'static str) -> Result<Vec<ObjectId>, EngineError> {
    if document.object_selection.is_empty() {
        return Ok(Vec::new());
    }
    for id in &document.object_selection {
        if !matches!(document.registry.get(*id), Some(ObjectData::Path(_))) {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!(
                    "selected object {} is not a path; only paths split",
                    id.index()
                ),
            });
        }
        if !matches!(
            object::find_owner(document, *id),
            Some(object::Owner::Cell { .. })
        ) {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!(
                    "selected object {} must sit in a cell (not nested in groups)",
                    id.index()
                ),
            });
        }
    }
    Ok(document.object_selection.clone())
}

fn point_of(name: &'static str, xy: [f64; 2]) -> Result<geom::Point, EngineError> {
    let [x, y] = xy;
    if !x.is_finite() || !y.is_finite() {
        return Err(EngineError::BadPayload {
            name: name.to_owned(),
            detail: "point is not finite".to_owned(),
        });
    }
    Ok(geom::Point::new(x, y))
}

/// `path.splitPaths` — CS6 `dom.splitPaths()`: "Splits the selected
/// paths. Compound paths are split into separate contours." Each split
/// path keeps its identity on the first contour; the rest become fresh
/// paths right after it. A single-contour path has nothing to split;
/// when nothing splits the command is a no-op (the dom returns nothing
/// either way).
struct PathSplitPaths;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathSplitPathsPayload {}

impl TypedCommand for PathSplitPaths {
    type Payload = PathSplitPathsPayload;

    fn name(&self) -> &'static str {
        "path.splitPaths"
    }

    fn apply(
        &self,
        document: &mut Document,
        _payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.splitPaths";
        let order = split_selection(document, NAME)?;
        if order.is_empty() {
            return Ok(Delta::Noop);
        }
        let splits: Vec<(ObjectId, Vec<doc::Contour>)> = order
            .iter()
            .filter_map(|id| match document.registry.get(*id) {
                Some(ObjectData::Path(path)) if path.contours.len() > 1 => {
                    Some((*id, path.contours.clone()))
                }
                _ => None,
            })
            .collect();
        if splits.is_empty() {
            return Ok(Delta::Noop);
        }
        replace_with_pieces(document, NAME, &order, &splits)
    }
}

/// `path.knifeElementsFromPoint` — CS6 `dom.knifeElementsFromPoint(
/// from, tolerance)`: "When the user clicks a single point while using
/// the Knife tool, this function cuts paths within the specified
/// tolerance." Selection-driven like the tool itself: only selected
/// paths cut. Nothing within tolerance is the dom's `false` — a no-op
/// here.
struct PathKnifePoint;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathKnifePointPayload {
    /// The clicked point.
    from: [f64; 2],
    /// Cut reach, >= 0.
    tolerance: f64,
}

impl TypedCommand for PathKnifePoint {
    type Payload = PathKnifePointPayload;

    fn name(&self) -> &'static str {
        "path.knifeElementsFromPoint"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.knifeElementsFromPoint";
        let from = point_of(NAME, payload.from)?;
        if !payload.tolerance.is_finite() || payload.tolerance < 0.0 {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!("tolerance {} is negative or not finite", payload.tolerance),
            });
        }
        let order = split_selection(document, NAME)?;
        if order.is_empty() {
            return Ok(Delta::Noop);
        }
        let splits: Vec<(ObjectId, Vec<doc::Contour>)> = order
            .iter()
            .filter_map(|id| match document.registry.get(*id) {
                Some(ObjectData::Path(path)) => {
                    vector::knife_contours(&path.contours, from, from, payload.tolerance)
                        .map(|contours| (*id, contours))
                }
                _ => None,
            })
            .collect();
        if splits.is_empty() {
            return Ok(Delta::Noop);
        }
        replace_with_pieces(document, NAME, &order, &splits)
    }
}

/// `path.knifeElementsFromPoints` — CS6 `dom.knifeElementsFromPoints(
/// from, to, tolerance)`: "When the user drags while using the Knife
/// tool, this function cuts additional items within the specified
/// tolerance." Every crossing of the from→to blade cuts; paths the
/// blade misses but that lie within tolerance are cut at the nearest
/// position.
struct PathKnifePoints;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct PathKnifePointsPayload {
    /// The drag start.
    from: [f64; 2],
    /// The drag end.
    to: [f64; 2],
    /// Cut reach for items the blade misses, >= 0.
    tolerance: f64,
}

impl TypedCommand for PathKnifePoints {
    type Payload = PathKnifePointsPayload;

    fn name(&self) -> &'static str {
        "path.knifeElementsFromPoints"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "path.knifeElementsFromPoints";
        let from = point_of(NAME, payload.from)?;
        let to = point_of(NAME, payload.to)?;
        if from == to {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: "the drag is zero-length; use the click verb".to_owned(),
            });
        }
        if !payload.tolerance.is_finite() || payload.tolerance < 0.0 {
            return Err(EngineError::BadPayload {
                name: NAME.to_owned(),
                detail: format!("tolerance {} is negative or not finite", payload.tolerance),
            });
        }
        let order = split_selection(document, NAME)?;
        if order.is_empty() {
            return Ok(Delta::Noop);
        }
        let splits: Vec<(ObjectId, Vec<doc::Contour>)> = order
            .iter()
            .filter_map(|id| match document.registry.get(*id) {
                Some(ObjectData::Path(path)) => {
                    vector::knife_contours(&path.contours, from, to, payload.tolerance)
                        .map(|contours| (*id, contours))
                }
                _ => None,
            })
            .collect();
        if splits.is_empty() {
            return Ok(Delta::Noop);
        }
        replace_with_pieces(document, NAME, &order, &splits)
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

    fn fill_of(document: &Document, id: ObjectId) -> &doc::Fill {
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => path
                .paint
                .as_ref()
                .and_then(|paint| paint.fill.as_ref())
                .expect("fill materialized"),
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn set_fill_edge_mode_maps_each_mode_and_inverts() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);

        // `antialias` materializes the fill definition (the square has
        // paint but no fill) and clears both edge slots.
        let deltas = [
            apply_by_name(
                &mut document,
                "path.setFillEdgeMode",
                json!({ "id": ids[0], "mode": "antialias" }),
            )
            .unwrap(),
            apply_by_name(
                &mut document,
                "path.setFillEdgeMode",
                json!({ "id": ids[0], "mode": "feather", "feather": 12 }),
            )
            .unwrap(),
            apply_by_name(
                &mut document,
                "path.setFillEdgeMode",
                json!({ "id": ids[0], "mode": "hard" }),
            )
            .unwrap(),
        ];
        let fill = fill_of(&document, ids[0]);
        assert!(fill.hard_edge, "hard sets FRDb");
        assert_eq!(fill.feather, 0, "hard clears FEFi");
        assert!(matches!(fill.shape, doc::FillShape::Solid));
        // The pre-existing paint survives the materialization.
        match document.registry.get(ids[0]).unwrap() {
            ObjectData::Path(path) => match path.paint.as_ref() {
                Some(paint) => {
                    assert_eq!(paint.stroke_raw, 0x11);
                    assert_eq!(paint.fill_raw, 0x22);
                }
                None => panic!("paint kept"),
            },
            other => panic!("wrong object kind: {other:?}"),
        }

        // Undo lands on the fill-less square (LIFO through the whole
        // sequence).
        for delta in deltas.iter().rev() {
            delta.apply_inverse(&mut document).unwrap();
        }
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn set_fill_edge_mode_creates_the_paint_when_absent() {
        let mut unpainted = square(0.0, 0.0, 8.0, 8.0);
        if let ObjectData::Path(path) = &mut unpainted {
            path.paint = None;
        }
        let (mut document, ids) = document_with(vec![unpainted]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.setFillEdgeMode",
            json!({ "id": ids[0], "mode": "feather", "feather": 4 }),
        )
        .unwrap();
        match document.registry.get(ids[0]).unwrap() {
            ObjectData::Path(path) => {
                let paint = path.paint.as_ref().expect("paint materialized");
                assert_eq!(paint.fill_raw, 0xFF00_0000, "CS6 default fill color");
                let fill = paint.fill.as_ref().expect("fill materialized");
                assert_eq!(fill.feather, 4);
                assert!(!fill.hard_edge);
            }
            other => panic!("wrong object kind: {other:?}"),
        }
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn set_fill_edge_mode_refusals_leave_the_document_alone() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0), image_object()]);
        let before = document_bytes(&document);
        let refusals = [
            (
                json!({ "id": ids[0], "mode": "feather", "feather": -1 }),
                "BadPayload",
                "negative feather",
            ),
            (
                json!({ "id": ids[0], "mode": "blurry" }),
                "BadPayload",
                "unknown mode",
            ),
            (
                json!({ "id": ids[1], "mode": "hard" }),
                "InvalidState",
                "not a path",
            ),
            (
                json!({ "id": ObjectId::from_parts(999, 0), "mode": "hard" }),
                "MissingTarget",
                "missing object",
            ),
        ];
        for (payload, kind, why) in refusals {
            let error = apply_by_name(&mut document, "path.setFillEdgeMode", payload).unwrap_err();
            let rendered = match &error {
                EngineError::BadPayload { .. } => "BadPayload",
                EngineError::InvalidState { .. } => "InvalidState",
                EngineError::MissingTarget { .. } => "MissingTarget",
                other => panic!("unexpected error for {why}: {other:?}"),
            };
            assert_eq!(rendered, kind, "{why}: {error}");
        }
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn crop_selection_clips_paths_and_inverts() {
        let (mut document, ids) = document_with(vec![
            square(0.0, 0.0, 8.0, 8.0),
            square(4.0, 4.0, 12.0, 12.0),
        ]);
        document.object_selection = vec![ids[0], ids[1]];
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.cropSelection",
            json!({ "rect": [2.0, 2.0, 6.0, 6.0] }),
        )
        .unwrap();
        let mut first = anchors(&document, ids[0]);
        first.sort_unstable();
        assert_eq!(first, vec![(2, 2), (2, 6), (6, 2), (6, 6)]);
        let mut second = anchors(&document, ids[1]);
        second.sort_unstable();
        assert_eq!(second, vec![(4, 4), (4, 6), (6, 4), (6, 6)]);
        // Display fields ride along; the geometry is derived.
        match document.registry.get(ids[0]).unwrap() {
            ObjectData::Path(path) => {
                assert_eq!(path.name.as_deref(), Some("square"));
                assert_eq!(path.paint.as_ref().unwrap().stroke_raw, 0x11);
                assert!(path.primitive.is_none());
            }
            other => panic!("wrong object kind: {other:?}"),
        }
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn crop_selection_removes_paths_outside_the_rect() {
        let (mut document, ids) = document_with(vec![
            square(0.0, 0.0, 8.0, 8.0),
            square(20.0, 20.0, 24.0, 24.0),
        ]);
        document.object_selection = vec![ids[0], ids[1]];
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.cropSelection",
            json!({ "rect": [1.0, 1.0, 5.0, 5.0] }),
        )
        .unwrap();
        assert!(
            document.registry.get(ids[1]).is_none(),
            "outside path removed"
        );
        let cell = &document.pages[0].layers.layers[0].cells[0];
        assert_eq!(cell.elements, vec![ids[0]]);
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn crop_selection_noop_and_refusals_leave_the_document_alone() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0), image_object()]);

        // Nothing selected: an explicit no-op.
        let delta = apply_by_name(
            &mut document,
            "path.cropSelection",
            json!({ "rect": [0.0, 0.0, 4.0, 4.0] }),
        )
        .unwrap();
        assert!(matches!(delta, Delta::Noop));

        document.object_selection = vec![ids[0], ids[1]];
        let before = document_bytes(&document);

        // Malformed rects are refused before the selection is read.
        for rect in [
            json!([0.0, 0.0, 0.0, 4.0]),
            json!([4.0, 4.0, 0.0, 0.0]),
            serde_json::Value::Null,
        ] {
            let error = apply_by_name(&mut document, "path.cropSelection", json!({ "rect": rect }))
                .unwrap_err();
            assert!(
                matches!(error, EngineError::BadPayload { .. }),
                "rect {rect}: {error}"
            );
        }

        // A non-path member refuses the whole command untouched.
        let error = apply_by_name(
            &mut document,
            "path.cropSelection",
            json!({ "rect": [1.0, 1.0, 6.0, 6.0] }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert_eq!(document_bytes(&document), before);
    }

    /// A named compound path: two disjoint squares in one object.
    fn compound() -> ObjectData {
        let ObjectData::Path(template) = square(0.0, 0.0, 10.0, 10.0) else {
            panic!("square builds a path");
        };
        ObjectData::Path(PathObject {
            contours: vec![
                square_contour(0.0, 0.0, 10.0, 10.0),
                square_contour(20.0, 0.0, 30.0, 10.0),
            ],
            ..template
        })
    }

    #[test]
    fn split_paths_separates_compound_and_undo_restores() {
        let (mut document, ids) = document_with(vec![compound()]);
        let id = ids[0];
        document.object_selection = vec![id];
        let before = document_bytes(&document);
        let delta = apply_by_name(&mut document, "path.splitPaths", json!({})).unwrap();

        // The original keeps the first contour; a fresh path carries the
        // second, attached right after it, inheriting the display fields.
        let piece = match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => {
                assert_eq!(
                    anchors(&document, id),
                    vec![(0, 0), (10, 0), (10, 10), (0, 10)]
                );
                assert!(path.primitive.is_none());
                *document.pages[0].layers.layers[0].cells[0]
                    .elements
                    .iter()
                    .find(|&&e| e != id)
                    .unwrap()
            }
            other => panic!("wrong object kind: {other:?}"),
        };
        match document.registry.get(piece).unwrap() {
            ObjectData::Path(path) => {
                assert_eq!(path.contours.len(), 1);
                assert_eq!(
                    anchors(&document, piece),
                    vec![(20, 0), (30, 0), (30, 10), (20, 10)]
                );
                assert_eq!(path.name.as_deref(), Some("square"));
                assert!(path.paint.is_some());
                assert!(path.primitive.is_none());
            }
            other => panic!("wrong object kind: {other:?}"),
        }
        let cell = &document.pages[0].layers.layers[0].cells[0];
        assert_eq!(cell.elements, vec![id, piece]);
        assert_eq!(document.object_selection, vec![id, piece]);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before, "undo restores both");
    }

    #[test]
    fn split_paths_noop_and_refusals() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0), image_object()]);

        // Empty selection and single-contour selection are no-ops.
        assert!(matches!(
            apply_by_name(&mut document, "path.splitPaths", json!({})).unwrap(),
            Delta::Noop
        ));
        document.object_selection = vec![ids[0]];
        assert!(matches!(
            apply_by_name(&mut document, "path.splitPaths", json!({})).unwrap(),
            Delta::Noop
        ));

        // A non-path member refuses the whole command untouched.
        document.object_selection = vec![ids[0], ids[1]];
        let before = document_bytes(&document);
        let error = apply_by_name(&mut document, "path.splitPaths", json!({})).unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert_eq!(document_bytes(&document), before);
    }

    #[test]
    fn knife_point_opens_square_and_undo_restores() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 10.0, 10.0)]);
        let id = ids[0];
        document.object_selection = vec![id];
        let before = document_bytes(&document);

        // A click within tolerance of the top edge cuts at the nearest
        // position and opens the ring; one piece, so the identity keeps
        // the geometry and no new object is minted.
        let delta = apply_by_name(
            &mut document,
            "path.knifeElementsFromPoint",
            json!({ "from": [5.0, -0.3], "tolerance": 1.0 }),
        )
        .unwrap();
        assert_eq!(
            document.pages[0].layers.layers[0].cells[0].elements,
            vec![id]
        );
        assert_eq!(
            anchors(&document, id),
            vec![(5, 0), (10, 0), (10, 10), (0, 10), (0, 0), (5, 0)]
        );
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => {
                assert!(!path.contours[0].closed, "the cut opens the ring");
                assert_eq!(path.name.as_deref(), Some("square"));
            }
            other => panic!("wrong object kind: {other:?}"),
        }
        assert_eq!(document.object_selection, vec![id]);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(document_bytes(&document), before, "undo restores the ring");

        // A click far from every selected path is the dom's `false`.
        document.object_selection = vec![id];
        let miss = apply_by_name(
            &mut document,
            "path.knifeElementsFromPoint",
            json!({ "from": [50.0, 50.0], "tolerance": 1.0 }),
        )
        .unwrap();
        assert!(matches!(miss, Delta::Noop));

        // Negative tolerance is refused before anything is read.
        let error = apply_by_name(
            &mut document,
            "path.knifeElementsFromPoint",
            json!({ "from": [5.0, -0.3], "tolerance": -1.0 }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }), "{error}");
    }

    #[test]
    fn knife_drag_splits_square_and_undo_restores() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 10.0, 10.0)]);
        let id = ids[0];
        document.object_selection = vec![id];
        let before = document_bytes(&document);

        // A vertical drag through the middle cuts the square into two
        // pieces: the identity keeps the left half, a fresh path carries
        // the right half.
        let delta = apply_by_name(
            &mut document,
            "path.knifeElementsFromPoints",
            json!({ "from": [-1.0, 5.0], "to": [11.0, 5.0], "tolerance": 0.0 }),
        )
        .unwrap();
        let piece = match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => {
                assert_eq!(path.contours.len(), 1);
                assert!(!path.contours[0].closed);
                *document.pages[0].layers.layers[0].cells[0]
                    .elements
                    .iter()
                    .find(|&&e| e != id)
                    .unwrap()
            }
            other => panic!("wrong object kind: {other:?}"),
        };
        assert!(matches!(
            document.registry.get(piece).unwrap(),
            ObjectData::Path(_)
        ));
        let cell = &document.pages[0].layers.layers[0].cells[0];
        assert_eq!(cell.elements, vec![id, piece]);
        assert_eq!(document.object_selection, vec![id, piece]);

        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            document_bytes(&document),
            before,
            "undo restores the square"
        );

        // A zero-length drag is refused (the click verb owns that case).
        document.object_selection = vec![id];
        let error = apply_by_name(
            &mut document,
            "path.knifeElementsFromPoints",
            json!({ "from": [5.0, 5.0], "to": [5.0, 5.0], "tolerance": 0.0 }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::BadPayload { .. }), "{error}");

        // A drag that misses everything is a no-op.
        let miss = apply_by_name(
            &mut document,
            "path.knifeElementsFromPoints",
            json!({ "from": [40.0, 40.0], "to": [60.0, 40.0], "tolerance": 0.0 }),
        )
        .unwrap();
        assert!(matches!(miss, Delta::Noop));
    }

    #[test]
    fn set_pattern_fill_mints_the_source_raster_and_installs() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0), image_object()]);
        let before = document_bytes(&document);
        let delta = apply_by_name(
            &mut document,
            "path.setPatternFill",
            json!({ "id": ids[0], "source": ids[1] }),
        )
        .unwrap();
        let ObjectData::Path(path) = document.registry.get(ids[0]).unwrap() else {
            panic!("square is a path");
        };
        let paint = path.paint.as_ref().unwrap();
        assert_eq!(paint.stroke_raw, 0x11, "the stroke channel is untouched");
        let fill = paint.fill.as_ref().unwrap();
        assert_eq!(fill.shape, doc::FillShape::Pattern);
        assert!(fill.shape_extended, "commands write the FSXi slot");
        let pattern = fill.pattern.as_ref().unwrap();
        assert_eq!(pattern.pixels, (4, 4));
        assert_eq!(pattern.tile_size, 128);
        assert_eq!(pattern.name.as_deref(), Some("Pattern"));
        let [doc::TileRef::Tile(tile_id)] = pattern.tiles.as_slice() else {
            panic!("4x4 tiles into exactly one 128 slot");
        };
        // The authored pattern carries the source raster verbatim
        // (assemble → retile is the flatten minting law).
        let ObjectData::Image(image) = document.registry.get(ids[1]).unwrap() else {
            panic!("inserted an image");
        };
        let source = crate::bitmap::assemble_rgba(&document, image, "test").unwrap();
        let tile = document.tiles.get(*tile_id).unwrap();
        // Bank tiles are 128-wide planes; the raster sits in the
        // top-left corner, row by row.
        for line in 0..4usize {
            let expected = &source[line * 16..line * 16 + 16];
            assert_eq!(
                &tile.pixels()[line * raster::TILE_SIDE * 4..line * raster::TILE_SIDE * 4 + 16],
                expected
            );
        }
        delta.apply_inverse(&mut document).unwrap();
        assert_eq!(
            document_bytes(&document),
            before,
            "undo restores the paint and drops the minted tile"
        );
    }

    #[test]
    fn set_pattern_fill_names_from_the_payload_then_the_image() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0), image_object()]);
        apply_by_name(
            &mut document,
            "path.setPatternFill",
            json!({ "id": ids[0], "source": ids[1], "name": "grid" }),
        )
        .unwrap();
        let ObjectData::Path(path) = document.registry.get(ids[0]).unwrap() else {
            panic!("square is a path");
        };
        let pattern = path.paint.as_ref().unwrap().fill.as_ref().unwrap();
        assert_eq!(
            pattern.pattern.as_ref().unwrap().name.as_deref(),
            Some("grid")
        );

        // Without a payload name the image's OBNs names the pattern.
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0), image_object()]);
        if let ObjectData::Image(image) = document.registry.get_mut(ids[1]).unwrap() {
            image.name = Some("src".to_owned());
        }
        apply_by_name(
            &mut document,
            "path.setPatternFill",
            json!({ "id": ids[0], "source": ids[1] }),
        )
        .unwrap();
        let ObjectData::Path(path) = document.registry.get(ids[0]).unwrap() else {
            panic!("square is a path");
        };
        let pattern = path.paint.as_ref().unwrap().fill.as_ref().unwrap();
        assert_eq!(
            pattern.pattern.as_ref().unwrap().name.as_deref(),
            Some("src")
        );
    }

    #[test]
    fn set_pattern_fill_refuses_and_mints_nothing() {
        // A non-image source refuses before the bank is touched.
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0)]);
        let before = document_bytes(&document);
        let error = apply_by_name(
            &mut document,
            "path.setPatternFill",
            json!({ "id": ids[0], "source": ids[0] }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert_eq!(document_bytes(&document), before);
        assert!(document.tiles.is_empty());

        // An over-extent source refuses at write, not at render. The
        // pixel extent is mutated after building the fixture, so the
        // extent preflight fires before the (now inconsistent) tile
        // grid is ever validated.
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 8.0, 8.0), image_object()]);
        if let ObjectData::Image(image) = document.registry.get_mut(ids[1]).unwrap() {
            image.pixels = (8193, 1);
        }
        let error = apply_by_name(
            &mut document,
            "path.setPatternFill",
            json!({ "id": ids[0], "source": ids[1] }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }), "{error}");
        assert!(document.tiles.is_empty());
    }

    #[test]
    fn an_authored_pattern_renders_the_source_pixels() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0, 2.0, 2.0), image_object()]);
        document.pages[0].info.canvas.width = 16.0;
        document.pages[0].info.canvas.height = 16.0;
        apply_by_name(
            &mut document,
            "path.setPatternFill",
            json!({ "id": ids[0], "source": ids[1] }),
        )
        .unwrap();
        let render = compose::render_subset(&document, 0, 0, &[ids[0]]).unwrap();
        let at = |x: usize, y: usize| -> [u8; 4] {
            let base = (y * usize::from(render.width) + x) * 4;
            render.pixels[base..base + 4].try_into().unwrap()
        };
        // The matte raster (opaque black) fills the path; the pattern
        // is anchored at the canvas origin unscaled, so 4×4 of it
        // covers the 2×2 square.
        assert_eq!(at(1, 1), [0, 0, 0, 255]);
        assert_eq!(at(8, 8), [0, 0, 0, 0], "outside the path stays empty");
    }
}
