//! Connector Line tool (`Connector Line.jsf` behavior): an orthogonal
//! elbow from a start point to an end point, with independently
//! rounded corners.
//!
//! The route runs along the orientation axis: start run, a corner
//! arc, the center spine through the elbow control point, a second
//! corner arc, end run — six nodes in one open contour
//! (`RedrawSmartShape`). Corner `k` is a signed distance from the
//! spine along the run axis; its arc is a quarter circle of radius
//! `|corner|` with κ-scaled handles (zero radius stores absent
//! handles and collapses the arc pair onto the spine, the script's
//! sharp-corner insert state). The start/end runs sit on opposite
//! sides of the spine; the corner arcs may sit on either side of
//! their runs, which the signed values carry directly.

use super::{ELLIPSE_BCP_CONST, open_contour, shape};
use doc::model::{PathObject, PathPoint};
use geom::Point;

/// Connector Line parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectorParams {
    /// Start point (`cps[1]`, on the start run).
    pub start: Point,
    /// End point (`cps[2]`, on the end run).
    pub end: Point,
    /// Spine position (`cps[0]`).
    pub elbow: Point,
    /// Start corner: signed offset of the start arc from the spine
    /// (`corner1` = `c3 − c0` along the run axis).
    pub corner1: f64,
    /// End corner, likewise (`corner2` = `c4 − c0`).
    pub corner2: f64,
    /// Route orientation (`mode` parity): the spine runs vertically
    /// and the start/end runs horizontally when set.
    pub vertical: bool,
}

impl ConnectorParams {
    /// The tool's insert defaults: a 200-unit square route, spine
    /// halfway, sharp corners.
    #[must_use]
    pub fn defaults(elbow: Point) -> Self {
        Self {
            start: Point::new(elbow.x - 100.0, elbow.y - 100.0),
            end: Point::new(elbow.x + 100.0, elbow.y + 100.0),
            elbow,
            corner1: 0.0,
            corner2: 0.0,
            vertical: false,
        }
    }
}

/// A polyline vertex (handles coincident → absent in our model).
fn plain(x: f64, y: f64) -> PathPoint {
    PathPoint {
        anchor: Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    }
}

/// A node with one live handle at an absolute position; a
/// zero-radius corner collapses the handle onto the anchor, which is
/// the model's absent-handle case.
fn handled(x: f64, y: f64, hx: f64, hy: f64, succ: bool) -> PathPoint {
    if hx == x && hy == y {
        return plain(x, y);
    }
    let handle = Point::new(hx, hy);
    if succ {
        PathPoint {
            anchor: Point::new(x, y),
            primary: None,
            secondary: Some(handle),
            is_curve: true,
        }
    } else {
        PathPoint {
            anchor: Point::new(x, y),
            primary: Some(handle),
            secondary: None,
            is_curve: true,
        }
    }
}

/// Generates the six-node elbow contour (`RedrawSmartShape`, both
/// orientation branches, with the script's signed-corner values used
/// directly).
#[must_use]
pub fn connector_line(p: ConnectorParams) -> PathObject {
    let r1 = p.corner1.abs();
    let r2 = p.corner2.abs();
    let k = ELLIPSE_BCP_CONST;
    let mut nodes = Vec::with_capacity(6);
    nodes.push(plain(p.start.x, p.start.y));

    if p.vertical {
        // Spine vertical: corners displace along y; arcs turn onto it.
        let c3 = Point::new(p.start.x, p.elbow.y + p.corner1);
        // Start side (script nodes 1–2).
        let v1 = if p.start.y < p.elbow.y { r1 } else { -r1 };
        let h1 = if p.start.x < p.elbow.x { r1 } else { -r1 };
        nodes.push(handled(c3.x, c3.y, c3.x, c3.y + v1 * k, true));
        nodes.push(handled(
            c3.x + h1,
            p.elbow.y,
            c3.x + h1 - h1 * k,
            p.elbow.y,
            false,
        ));

        // End side (script nodes 3–4).
        let c4 = Point::new(p.end.x, p.elbow.y + p.corner2);
        let h2 = if p.end.x < p.elbow.x { r2 } else { -r2 };
        let v2 = if p.end.y > p.elbow.y { r2 } else { -r2 };
        nodes.push(handled(
            c4.x + h2,
            p.elbow.y,
            c4.x + h2 - h2 * k,
            p.elbow.y,
            true,
        ));
        nodes.push(handled(c4.x, c4.y, c4.x, c4.y - v2 * k, false));
    } else {
        // Spine horizontal: corners displace along x; arcs turn onto it.
        let c3 = Point::new(p.elbow.x + p.corner1, p.start.y);
        // Start side (script nodes 1–2).
        let h1 = if p.start.x < p.elbow.x { r1 } else { -r1 };
        let v1 = if p.start.y < p.elbow.y { r1 } else { -r1 };
        nodes.push(handled(c3.x, c3.y, c3.x + h1 * k, c3.y, true));
        nodes.push(handled(
            p.elbow.x,
            c3.y + v1,
            p.elbow.x,
            c3.y + v1 - v1 * k,
            false,
        ));

        // End side (script nodes 3–4).
        let c4 = Point::new(p.elbow.x + p.corner2, p.end.y);
        let v2 = if p.end.y > p.elbow.y { r2 } else { -r2 };
        let h2 = if p.end.x > p.elbow.x { r2 } else { -r2 };
        nodes.push(handled(
            p.elbow.x,
            c4.y - v2,
            p.elbow.x,
            c4.y - v2 + v2 * k,
            true,
        ));
        nodes.push(handled(c4.x, c4.y, c4.x - h2 * k, c4.y, false));
    }
    nodes.push(plain(p.end.x, p.end.y));
    shape(vec![open_contour(nodes)])
}
