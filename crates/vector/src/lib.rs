//! Vector depth for FireCraft: curve-aware path booleans (union,
//! intersect, punch, crop) over the `doc` path model, built on kurbo
//! curve primitives.
//!
//! Layer L2: depends on `doc` (L1) and `geom` (L0) only. No IO, no
//! threads — the crate is wasm-clean by construction.
//!
//! The algorithm (flattened-proxy intersection finding with true-curve
//! splitting and winding-rule region assembly) is decided and recorded
//! in `docs/decisions/0003-path-booleans.md`. Every numeric gate is a
//! named constant here so proofs stay tolerance-pinned.

mod boolean;
mod bridge;
mod flatten;
mod intersect;
mod split;

pub use boolean::{BooleanOp, boolean};
pub use bridge::{
    Seg, bezpath_to_contour, canonicalize_contour, contour_segments, contour_to_bezpath,
};
pub use flatten::{FLATTEN_TOLERANCE, flatten_cubic};

/// Pinned numeric gates for the boolean pipeline (pixels / normalized
/// parameters). Changing any of them moves every golden floor built on
/// top; do it deliberately and re-pin the proofs.
///
/// - [`FLATTEN_TOLERANCE`]: maximum deviation of the flattened proxy
///   used only to *find* segment intersections; output geometry keeps
///   the true curve pieces.
/// - [`SIDE_OFFSET`]: how far each side of a boundary piece is sampled
///   for winding classification. Must exceed [`FLATTEN_TOLERANCE`] (the
///   proxy's deviation from the true curve) so the sample lands on the
///   intended side of the *true* boundary; must stay below the feature
///   sizes the booleans claim to resolve — a sliver narrower than
///   2×SIDE_OFFSET is absorbed into the region on both sides of its
///   bounding edge and lost (proved by the sub-pixel wedge case in the
///   test suite; 0.1 resolves 0.2 px features).
/// - [`SNAP`]: endpoint quantization for loop stitching — two pieces
///   whose endpoints quantize to the same grid cell chain together.
pub const SIDE_OFFSET: f64 = 0.1;
/// Endpoint quantization grid for stitching, in pixels.
pub const SNAP: f64 = 1e-6;

/// Everything that can go wrong in a vector operation. No panics: the
/// registry turns these into command errors and the fuzz drivers count
/// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorError {
    /// A `BezPath` carried more than one subpath where a single contour
    /// was required.
    MultiSubpath,
    /// A `BezPath` had no drawing commands.
    EmptyPath,
    /// A `BezPath` started with something other than a move.
    NoCurrentPoint,
    /// Boundary pieces could not be closed into loops (numerical miss
    /// beyond [`SNAP`], or a classification inconsistency).
    OpenChain,
    /// A curve degenerated (zero-length chord under subdivision).
    DegenerateSegment,
    /// A `BezPath` carried a quadratic segment, which the `doc` path
    /// model (cubic-and-line spans) cannot represent.
    QuadraticSegment,
}

impl std::fmt::Display for VectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MultiSubpath => write!(f, "bezier path has more than one subpath"),
            Self::EmptyPath => write!(f, "bezier path is empty"),
            Self::NoCurrentPoint => write!(f, "bezier path does not start with a move"),
            Self::OpenChain => write!(f, "boolean boundary pieces did not close into loops"),
            Self::DegenerateSegment => write!(f, "curve degenerated under subdivision"),
            Self::QuadraticSegment => {
                write!(f, "quadratic segment cannot map to the doc path model")
            }
        }
    }
}

impl std::error::Error for VectorError {}

/// Quantizes a point onto the [`SNAP`] grid for stitching comparisons.
#[must_use]
pub(crate) fn snap_key(p: geom::Point) -> (i64, i64) {
    ((p.x / SNAP).round() as i64, (p.y / SNAP).round() as i64)
}

#[cfg(test)]
mod tests;
