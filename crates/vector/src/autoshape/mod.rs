//! Auto Shape generators — the parametric shape library (Fireworks:
//! the Auto Shape *tools* — Star, Spiral, Pie, Doughnut, the rectangle
//! family, Smart Polygon, and friends).
//!
//! Port discipline (M6 scope line): each shape's behavior spec is the
//! CS6 tool's `.jsf` script (read in scratch from the local reference
//! install, never committed); the generator here is a fresh Rust
//! implementation of the *behavior* — parameter schema, geometry
//! formulas, handle placement — with the same defaults and clamps the
//! scripts apply, not a transcription of the script text.
//!
//! Contracts, pinned by the tests:
//!
//! - **Regenerate, don't bake.** A generator maps parameters to a
//!   neutral [`PathObject`]; it is total over its parameter space
//!   (out-of-range values clamp exactly as the scripts clamp — e.g. a
//!   star's corner count clamps to 3..=50) and deterministic
//!   (same params, same geometry bits, every run).
//! - **Coordinate space.** Generators consume and produce the same
//!   y-down document space the scripts operate in; angles are radians
//!   measured as in the scripts (`atan2` of control-point deltas), so
//!   ported defaults (−π/2 = "up") land where CS6 puts them.
//! - **Zero handles are absent handles.** A script corner stores its
//!   handles coincident with the anchor; our path model expresses that
//!   span as a line (`primary`/`secondary` = `None`,
//!   `is_curve = false`).
//! - **JS arithmetic quirks are pinned**, not "fixed": `Math.round`
//!   rounds half toward +∞ (`js_round` here); integer coercion
//!   truncates toward zero; handle-length clamps only cap the upper
//!   bound where the script does.
//!
//! Not yet settled here: persisting a shape's identity + parameters in
//! the document (CS6 stores smart-shape custom data; the mapping is
//! corpus-gated). Until then the parameter structs in this module are
//! the single source of truth, and the registry commands that mutate
//! shapes carry them in their payloads.

mod arrow;
mod arrow_line;
mod connector;
mod measure;
mod pie;
mod polygon;
mod rect;
mod spiral;
mod star;

pub use arrow::{ArrowParams, LShapeParams, arrow, lshape};
pub use arrow_line::{ArrowHead, ArrowLineParams, arrow_line};
pub use connector::{ConnectorParams, connector_line};
pub use measure::{MeasureParams, measure};
pub use pie::{PieParams, doughnut, pie};
pub use polygon::{PolygonParams, smart_polygon};
pub use rect::{CornerKind, RectParams, rectangle};
pub use spiral::{
    MAX_TIGHTNESS_DEG, MIN_TIGHTNESS_DEG, SpiralParams, spiral, tightness_from_control,
};
pub use star::{StarParams, star};

#[cfg(test)]
mod tests;

use doc::model::{Contour, PathObject, PathPoint};
use geom::Point;

/// `fw.ellipseBCPConst` — the Bézier handle constant that makes a
/// cubic approximate a quarter circle: `4/3·(√2−1)`.
pub const ELLIPSE_BCP_CONST: f64 = 4.0 / 3.0 * (std::f64::consts::SQRT_2 - 1.0);

/// `Math.round` — rounds half toward +∞ (JS semantics; Rust's `round`
/// rounds half away from zero, which diverges on negatives).
pub(crate) fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// A corner anchor: no handles, not a curve point.
pub(crate) fn corner(x: f64, y: f64) -> PathPoint {
    PathPoint {
        anchor: Point::new(x, y),
        primary: None,
        secondary: None,
        is_curve: false,
    }
}

/// A curve anchor with both handles (`pred` = in = `primary`,
/// `succ` = out = `secondary`).
pub(crate) fn curved(x: f64, y: f64, pred: (f64, f64), succ: (f64, f64)) -> PathPoint {
    PathPoint {
        anchor: Point::new(x, y),
        primary: Some(Point::new(pred.0, pred.1)),
        secondary: Some(Point::new(succ.0, succ.1)),
        is_curve: true,
    }
}

/// Builds a closed contour over `points`.
pub(crate) fn closed_contour(points: Vec<PathPoint>) -> Contour {
    let n = points.len() as i64;
    Contour {
        closed: true,
        point_count: n,
        points,
    }
}

/// Builds an open contour over `points`.
pub(crate) fn open_contour(points: Vec<PathPoint>) -> Contour {
    let n = points.len() as i64;
    Contour {
        closed: false,
        point_count: n,
        points,
    }
}

/// Wraps finished contours in a neutral path.
pub(crate) fn shape(contours: Vec<Contour>) -> PathObject {
    PathObject::neutral(contours)
}
