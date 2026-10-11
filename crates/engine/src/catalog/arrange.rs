//! Selection arrangement (`object.align` / `object.distribute`):
//! bounding-box placement over a set of path objects, translated
//! through the vector crate's align/distribute math. Every moved
//! object captures its prior geometry, so the group delta restores the
//! whole arrangement.

use doc::{Document, ObjectData, ObjectId};
use geom::Rect;
use vector::{AlignMode, DistributeAxis};

use crate::access;
use crate::command::TypedCommand;
use crate::delta::{Delta, ObjectInverse};
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the arrangement commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(AlignObjects)?;
    registry.register(DistributeObjects)?;
    Ok(())
}

/// Which edge or center `object.align` pulls onto the selection box.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
enum AlignChoice {
    /// Left edges onto the union's left edge.
    #[serde(rename = "left")]
    Left,
    /// Horizontal centers onto the union's center.
    #[serde(rename = "centerX")]
    CenterX,
    /// Right edges onto the union's right edge.
    #[serde(rename = "right")]
    Right,
    /// Top edges onto the union's top edge.
    #[serde(rename = "top")]
    Top,
    /// Vertical centers onto the union's center.
    #[serde(rename = "centerY")]
    CenterY,
    /// Bottom edges onto the union's bottom edge.
    #[serde(rename = "bottom")]
    Bottom,
}

impl From<AlignChoice> for AlignMode {
    fn from(choice: AlignChoice) -> Self {
        match choice {
            AlignChoice::Left => AlignMode::Left,
            AlignChoice::CenterX => AlignMode::CenterX,
            AlignChoice::Right => AlignMode::Right,
            AlignChoice::Top => AlignMode::Top,
            AlignChoice::CenterY => AlignMode::CenterY,
            AlignChoice::Bottom => AlignMode::Bottom,
        }
    }
}

/// Which axis `object.distribute` spaces along.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum AxisChoice {
    /// Equal gaps between left/right edges.
    Horizontal,
    /// Equal gaps between top/bottom edges.
    Vertical,
}

impl From<AxisChoice> for DistributeAxis {
    fn from(choice: AxisChoice) -> Self {
        match choice {
            AxisChoice::Horizontal => DistributeAxis::Horizontal,
            AxisChoice::Vertical => DistributeAxis::Vertical,
        }
    }
}

/// `object.align` — pulls every selected path's box onto the
/// selection's union box (paths only until non-path bounds exist).
struct AlignObjects;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct AlignObjectsPayload {
    /// Selected objects; at least two, no duplicates.
    ids: Vec<ObjectId>,
    /// What to align to.
    mode: AlignChoice,
}

impl TypedCommand for AlignObjects {
    type Payload = AlignObjectsPayload;

    fn name(&self) -> &'static str {
        "object.align"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        arrange(
            document,
            self.name(),
            &payload.ids,
            2,
            vector::align,
            payload.mode.into(),
        )
    }
}

/// `object.distribute` — spaces the selected paths with equal gaps
/// along one axis (the panel's "space evenly"); the outermost objects
/// anchor.
struct DistributeObjects;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct DistributeObjectsPayload {
    /// Selected objects; at least three, no duplicates.
    ids: Vec<ObjectId>,
    /// Which axis to space along.
    axis: AxisChoice,
}

impl TypedCommand for DistributeObjects {
    type Payload = DistributeObjectsPayload;

    fn name(&self) -> &'static str {
        "object.distribute"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        arrange(
            document,
            self.name(),
            &payload.ids,
            3,
            vector::distribute,
            payload.axis.into(),
        )
    }
}

/// Shared arrange pipeline: capture every target, resolve boxes,
/// solve the translations, translate. `minimum` is 2 for align, 3 for
/// distribute; `solve` is the vector crate's per-box translation
/// solver.
fn arrange<T>(
    document: &mut Document,
    name: &'static str,
    ids: &[ObjectId],
    minimum: usize,
    solve: fn(&[Rect], T) -> Vec<geom::Vec2>,
    kind: T,
) -> Result<Delta, EngineError>
where
    T: Copy,
{
    if ids.len() < minimum {
        return Err(EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("need at least {minimum} objects, got {}", ids.len()),
        });
    }
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        if !seen.insert(*id) {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("object {} appears twice in the selection", id.index()),
            });
        }
    }

    // Capture + resolve before anything mutates.
    let mut targets = Vec::with_capacity(ids.len());
    for id in ids {
        let object = access::object(document, name, *id)?;
        let ObjectData::Path(path) = object else {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("object {} is not a path", id.index()),
            });
        };
        let rect = object_rect(path).ok_or_else(|| EngineError::InvalidState {
            name: name.to_owned(),
            detail: format!("object {} has empty geometry", id.index()),
        })?;
        if !is_finite_rect(rect) {
            return Err(EngineError::InvalidState {
                name: name.to_owned(),
                detail: format!("object {} has non-finite bounds", id.index()),
            });
        }
        targets.push((*id, object.clone(), rect));
    }

    let rects: Vec<Rect> = targets.iter().map(|(_, _, rect)| *rect).collect();
    let translations = solve(&rects, kind);
    if translations.iter().all(|t| *t == geom::Vec2::ZERO) {
        return Ok(Delta::Noop);
    }

    let mut deltas = Vec::with_capacity(targets.len());
    for ((id, before, _), translation) in targets.iter().zip(&translations) {
        let moved = vector::translate_path(
            match document.registry.get(*id) {
                Some(ObjectData::Path(path)) => path,
                _ => {
                    return Err(EngineError::MissingTarget {
                        name: name.to_owned(),
                        detail: format!(
                            "object {} (generation {}) vanished between capture and mutation",
                            id.index(),
                            id.generation()
                        ),
                    });
                }
            },
            *translation,
        );
        let applied = match document.registry.get_mut(*id) {
            Some(ObjectData::Path(path)) => {
                *path = moved;
                true
            }
            _ => false,
        };
        if !applied {
            return Err(EngineError::MissingTarget {
                name: name.to_owned(),
                detail: format!(
                    "object {} (generation {}) vanished between capture and mutation",
                    id.index(),
                    id.generation()
                ),
            });
        }
        deltas.push(Delta::Object {
            id: *id,
            inverse: ObjectInverse::Mutated {
                object: Box::new(before.clone()),
            },
        });
    }
    Ok(Delta::Group(deltas))
}

/// Union of a path object's contour boxes; `None` when every contour
/// is empty.
fn object_rect(path: &doc::PathObject) -> Option<Rect> {
    geom::bounds::union_all(
        path.contours
            .iter()
            .filter_map(|contour| geom::bounds::path_bounds(&vector::contour_to_bezpath(contour))),
    )
}

fn is_finite_rect(rect: Rect) -> bool {
    rect.x0.is_finite() && rect.x1.is_finite() && rect.y0.is_finite() && rect.y1.is_finite()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::error::EngineError;
    use crate::registry::CommandRegistry;
    use doc::{Cell, Contour, Layer, LayerList, Page, PageInfo, PathObject, PathPoint};
    use serde_json::json;

    fn point(x: f64, y: f64) -> PathPoint {
        PathPoint {
            anchor: geom::Point::new(x, y),
            primary: None,
            secondary: None,
            is_curve: false,
        }
    }

    /// A 10x10 square path whose top-left sits at (`x`, `y`).
    fn square(x: f64, y: f64) -> ObjectData {
        ObjectData::Path(PathObject {
            contours: vec![Contour {
                closed: true,
                point_count: 4,
                points: vec![
                    point(x, y),
                    point(x + 10.0, y),
                    point(x + 10.0, y + 10.0),
                    point(x, y + 10.0),
                ],
            }],
            ..PathObject::neutral(Vec::new())
        })
    }

    /// One page, one layer, one cell holding `objects` in order.
    fn document_with(objects: Vec<ObjectData>) -> (Document, Vec<ObjectId>) {
        let mut document = Document::default();
        let mut ids = Vec::new();
        for object in objects {
            ids.push(document.registry.insert(object));
        }
        let layer = Layer {
            name: "Layer 0".to_owned(),
            display: true,
            cells: vec![Cell {
                locked: false,
                visible: true,
                elements: ids.clone(),
            }],
            ..Layer::default()
        };
        document.pages.push(Page {
            info: PageInfo {
                version: None,
                revision: None,
                frame_count: 1,
                frame_count_duplicate: 1,
                request_id: 0,
                canvas: Default::default(),
                onion_before: 0,
                onion_after: 0,
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

    fn square_origin(document: &Document, id: ObjectId) -> (f64, f64) {
        match document.registry.get(id).unwrap() {
            ObjectData::Path(path) => {
                let anchor = &path.contours[0].points[0].anchor;
                (anchor.x, anchor.y)
            }
            other => panic!("wrong object kind: {other:?}"),
        }
    }

    #[test]
    fn align_left_pulls_both_onto_the_union() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0), square(20.0, 5.0)]);
        apply_by_name(
            &mut document,
            "object.align",
            json!({ "ids": [ids[0], ids[1]], "mode": "left" }),
        )
        .unwrap();
        assert_eq!(square_origin(&document, ids[1]), (0.0, 5.0));
        assert_eq!(square_origin(&document, ids[0]), (0.0, 0.0));
    }

    #[test]
    fn distribute_spaces_middles_and_keeps_anchors() {
        let (mut document, ids) =
            document_with(vec![square(0.0, 0.0), square(7.0, 0.0), square(30.0, 0.0)]);
        apply_by_name(
            &mut document,
            "object.distribute",
            json!({ "ids": [ids[0], ids[1], ids[2]], "axis": "horizontal" }),
        )
        .unwrap();
        assert_eq!(square_origin(&document, ids[0]), (0.0, 0.0));
        assert_eq!(square_origin(&document, ids[1]), (15.0, 0.0));
        assert_eq!(square_origin(&document, ids[2]), (30.0, 0.0));
    }

    #[test]
    fn an_already_arranged_selection_is_a_noop() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0), square(0.0, 20.0)]);
        let delta = apply_by_name(
            &mut document,
            "object.align",
            json!({ "ids": [ids[0], ids[1]], "mode": "left" }),
        )
        .unwrap();
        assert!(matches!(delta, Delta::Noop));
    }

    #[test]
    fn selection_errors_are_rejected_without_mutating() {
        let (mut document, ids) = document_with(vec![square(0.0, 0.0), square(20.0, 5.0)]);
        let before = serde_json::to_vec(&document).unwrap();

        // Too few for align.
        let error = apply_by_name(
            &mut document,
            "object.align",
            json!({ "ids": [ids[0]], "mode": "left" }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));

        // Duplicate id.
        let error = apply_by_name(
            &mut document,
            "object.align",
            json!({ "ids": [ids[0], ids[0]], "mode": "left" }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));

        // Too few for distribute.
        let error = apply_by_name(
            &mut document,
            "object.distribute",
            json!({ "ids": [ids[0], ids[1]], "axis": "horizontal" }),
        )
        .unwrap_err();
        assert!(matches!(error, EngineError::InvalidState { .. }));

        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
    }
}
