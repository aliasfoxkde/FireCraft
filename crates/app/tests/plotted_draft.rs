//! T27-23: a plotted path belongs to the tool that started it.
//!
//! The Pen, the Polygon Slice and the Polygon Hotspot all place
//! their points with clicks, one click each, and the pending path
//! lives in the editor until it is committed. The review's Major
//! finding on the editor was that the pending path carried no tool:
//! the commit read whichever tool happened to be selected, so a Pen's
//! points could be folded into a Hotspot's outline — the user clicked
//! three times with the Pen, picked the Hotspot, and got a hotspot
//! that carried the Pen's geometry.
//!
//! The draft now records the tool that plotted it. Switching tools
//! converts the pending path explicitly: it is committed as the tool
//! that plotted it, so the two tools' geometry can never share an
//! object.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::{Editor, Tool};
use doc::ObjectData;
use geom::Point;

/// The anchors of every path in the document, contour by contour.
fn path_anchors(editor: &Editor) -> Vec<Vec<Vec<(f64, f64)>>> {
    editor
        .document()
        .registry
        .iter()
        .filter_map(|(_, data)| match data {
            ObjectData::Path(path) => Some(
                path.contours
                    .iter()
                    .map(|contour| {
                        contour
                            .points
                            .iter()
                            .map(|point| (point.anchor.x, point.anchor.y))
                            .collect()
                    })
                    .collect(),
            ),
            _ => None,
        })
        .collect()
}

/// The anchors of every hotspot in the document — a slice with no
/// slice type is a hotspot; a slice with one is a real slice.
fn hotspot_anchors(editor: &Editor) -> Vec<Vec<Vec<(f64, f64)>>> {
    editor
        .document()
        .registry
        .iter()
        .filter_map(|(_, data)| match data {
            ObjectData::Slice(slice) if slice.slice_type.is_none() => Some(
                slice
                    .contours
                    .iter()
                    .map(|contour| {
                        contour
                            .points
                            .iter()
                            .map(|point| (point.anchor.x, point.anchor.y))
                            .collect()
                    })
                    .collect(),
            ),
            _ => None,
        })
        .collect()
}

#[test]
fn the_pen_points_stay_a_polyline_when_the_hotspot_tool_takes_over() {
    let mut editor = Editor::untitled();
    let pen = [(10.0, 10.0), (60.0, 10.0), (60.0, 40.0)];
    editor.tool = Tool::Pen;
    for (x, y) in pen {
        editor
            .add_draft_point(Point::new(x, y))
            .expect("the point plots");
    }

    // The user picks the hotspot tool and keeps clicking.
    editor.tool = Tool::PolygonHotspot;
    let hotspot = [(120.0, 120.0), (160.0, 120.0), (160.0, 150.0)];
    for (x, y) in hotspot {
        editor
            .add_draft_point(Point::new(x, y))
            .expect("the point plots");
    }
    editor.commit_draft().expect("the hotspot replay applies");

    let paths = path_anchors(&editor);
    assert_eq!(paths.len(), 1, "the Pen's path is the only path");
    assert_eq!(paths[0].len(), 1, "it carries one contour");
    assert_eq!(
        paths[0][0],
        pen.to_vec(),
        "the polyline carries the Pen's points"
    );

    let hotspots = hotspot_anchors(&editor);
    assert_eq!(hotspots.len(), 1, "one hotspot was created");
    assert_eq!(hotspots[0].len(), 1, "it carries one outline");
    assert_eq!(
        hotspots[0][0],
        hotspot.to_vec(),
        "the hotspot carries its own points"
    );
}

#[test]
fn the_hotspot_points_stay_a_hotspot_when_the_pen_tool_takes_over() {
    let mut editor = Editor::untitled();
    let hotspot = [(10.0, 10.0), (60.0, 10.0), (60.0, 40.0)];
    editor.tool = Tool::PolygonHotspot;
    for (x, y) in hotspot {
        editor
            .add_draft_point(Point::new(x, y))
            .expect("the point plots");
    }

    // The user picks the Pen and keeps clicking.
    editor.tool = Tool::Pen;
    let pen = [(120.0, 120.0), (160.0, 120.0)];
    for (x, y) in pen {
        editor
            .add_draft_point(Point::new(x, y))
            .expect("the point plots");
    }
    editor.commit_draft().expect("the polyline applies");

    let hotspots = hotspot_anchors(&editor);
    assert_eq!(hotspots.len(), 1, "one hotspot was created");
    assert_eq!(hotspots[0].len(), 1, "it carries one outline");
    assert_eq!(
        hotspots[0][0],
        hotspot.to_vec(),
        "the hotspot keeps the points the hotspot plotted"
    );

    let paths = path_anchors(&editor);
    assert_eq!(paths.len(), 1, "the Pen's polyline is the only path");
    assert_eq!(paths[0].len(), 1, "it carries one contour");
    assert_eq!(
        paths[0][0],
        pen.to_vec(),
        "the polyline carries the Pen's points"
    );
}

#[test]
fn a_pen_point_too_few_to_commit_is_dropped_when_the_tool_changes() {
    let mut editor = Editor::untitled();
    editor.tool = Tool::Pen;
    editor
        .add_draft_point(Point::new(10.0, 10.0))
        .expect("the point plots");

    // One point is not a path, so the pending Pen point cannot be
    // committed — and it must not be carried into the hotspot's
    // outline either.
    editor.tool = Tool::PolygonHotspot;
    let hotspot = [(120.0, 120.0), (160.0, 120.0), (160.0, 150.0)];
    for (x, y) in hotspot {
        editor
            .add_draft_point(Point::new(x, y))
            .expect("the point plots");
    }
    editor.commit_draft().expect("the hotspot replay applies");

    assert!(
        path_anchors(&editor).is_empty(),
        "the lone Pen point never became a path"
    );
    let hotspots = hotspot_anchors(&editor);
    assert_eq!(hotspots.len(), 1, "one hotspot was created");
    assert_eq!(
        hotspots[0][0],
        hotspot.to_vec(),
        "the hotspot carries only its own points"
    );
}
