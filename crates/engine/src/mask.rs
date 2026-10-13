//! Selection-mask construction: marquee, lasso, wand, feather, and the
//! set operations, over [`doc::SelectionMask`] alpha planes.
//!
//! Everything here is deterministic integer/fixed arithmetic — the same
//! mask bytes for the same inputs on every platform, so golden pins and
//! journal replays can compare masks byte-exact. Pixel membership uses
//! the **center rule** throughout: pixel `(px, py)` is inside a shape
//! when `(px + 0.5, py + 0.5)` is.

use doc::SelectionMask;

/// How a new selection interacts with the active one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectMode {
    /// Replace the active selection.
    Replace,
    /// Union with the active selection (Fireworks shift-add).
    Add,
    /// Cut out of the active selection (Fireworks alt-subtract).
    Subtract,
    /// Keep only the overlap with the active selection.
    Intersect,
}

impl SelectMode {
    /// Parses the payload's `mode` string; `Replace` is the default
    /// when the field is absent.
    pub(crate) fn parse(value: Option<&str>) -> Option<Self> {
        match value {
            None | Some("replace") => Some(Self::Replace),
            Some("add") => Some(Self::Add),
            Some("subtract") => Some(Self::Subtract),
            Some("intersect") => Some(Self::Intersect),
            Some(_) => None,
        }
    }

    /// Folds `fresh` coverage into `active` under this mode.
    fn combine(self, active: u8, fresh: u8) -> u8 {
        match self {
            Self::Replace => fresh,
            Self::Add => active.max(fresh),
            // Soft-mask subtract: `a·(1−b)` with a saturating integer
            // form — hard masks (0/255) cut exactly, feathered edges
            // scale proportionally.
            Self::Subtract => ((u16::from(active) * (255 - u16::from(fresh))) / 255) as u8,
            // Soft-mask intersection: the product — hard masks keep
            // exactly the overlap, feathered edges multiply.
            Self::Intersect => ((u16::from(active) * u16::from(fresh)) / 255) as u8,
        }
    }
}

/// Stamps a filled rectangle (marquee) into `mask`: pixel centers
/// inside `[x, x+w) × [y, y+h)`.
pub(crate) fn stamp_rect(mask: &mut SelectionMask, x: f64, y: f64, width: f64, height: f64) {
    for py in 0..mask.height {
        let cy = f64::from(py) + 0.5;
        if cy < y || cy >= y + height {
            continue;
        }
        for px in 0..mask.width {
            let cx = f64::from(px) + 0.5;
            if cx >= x && cx < x + width {
                mask.set(px, py, 255);
            }
        }
    }
}

/// Stamps a filled ellipse (oval marquee) into `mask`: pixel centers
/// `((cx−cx₀)/rx)² + ((cy−cy₀)/ry)² ≤ 1` about the rectangle's center.
pub(crate) fn stamp_ellipse(mask: &mut SelectionMask, x: f64, y: f64, width: f64, height: f64) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let (center_x, center_y) = (x + width / 2.0, y + height / 2.0);
    let (radius_x, radius_y) = (width / 2.0, height / 2.0);
    for py in 0..mask.height {
        let dy = (f64::from(py) + 0.5 - center_y) / radius_y;
        for px in 0..mask.width {
            let dx = (f64::from(px) + 0.5 - center_x) / radius_x;
            if dx * dx + dy * dy <= 1.0 {
                mask.set(px, py, 255);
            }
        }
    }
}

/// Stamps a filled polygon (lasso) into `mask`: even-odd scanline fill;
/// a pixel is inside when its center's horizontal ray crosses the
/// polygon an odd number of times to its left. Degenerate polygons
/// (fewer than 3 points) select nothing.
pub(crate) fn stamp_polygon(mask: &mut SelectionMask, points: &[(f64, f64)]) {
    if points.len() < 3 {
        return;
    }
    for py in 0..mask.height {
        let ray = f64::from(py) + 0.5;
        let mut crossings = Vec::new();
        let mut previous = points[points.len() - 1];
        for &point in points {
            let (ax, ay) = previous;
            let (bx, by) = point;
            previous = point;
            // Half-open edge rule ([min, max) in y) so vertices on the
            // ray count exactly once.
            if (ay <= ray && by > ray) || (by <= ray && ay > ray) {
                let t = (ray - ay) / (by - ay);
                crossings.push(ax + t * (bx - ax));
            }
        }
        crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for pair in crossings.chunks_exact(2) {
            let (start, end) = (pair[0], pair[1]);
            // Centers strictly inside [start, end): px + 0.5 ∈ [start, end).
            let first = ((start - 0.5).ceil().max(0.0)) as u32;
            let past = ((end - 0.5).ceil().max(0.0)) as u32;
            for px in first..past.min(mask.width) {
                mask.set(px, py, 255);
            }
        }
    }
}

/// Which pixel comparisons the wand makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WandColor {
    /// Straight-alpha RGBA of the seed pixel.
    pub seed: [u8; 4],
    /// Per-channel tolerance `0..=255`: a channel matches when
    /// `|pixel − seed| ≤ tolerance`; a pixel matches when all four do.
    pub tolerance: u8,
}

impl WandColor {
    /// Whether `pixel` (straight-alpha RGBA) matches the seed.
    fn matches(self, pixel: [u8; 4]) -> bool {
        let tolerance = u16::from(self.tolerance);
        self.seed
            .iter()
            .zip(pixel)
            .all(|(seed, value)| u16::from(*seed).abs_diff(u16::from(value)) <= tolerance)
    }
}

/// Stamps a magic-wand selection into `mask` over a straight-alpha RGBA
/// image: `contiguous` flood-fills 4-connected from the seed; otherwise
/// every matching pixel in the image joins.
pub(crate) fn stamp_wand(
    mask: &mut SelectionMask,
    image: &[u8],
    x: u32,
    y: u32,
    target: WandColor,
    contiguous: bool,
) {
    if contiguous {
        let Some(start) = mask_index(mask, x, y) else {
            return;
        };
        let mut stack = vec![start];
        let mut visited = vec![false; mask.alpha.len()];
        visited[start] = true;
        while let Some(index) = stack.pop() {
            let pixel = [
                image[index * 4],
                image[index * 4 + 1],
                image[index * 4 + 2],
                image[index * 4 + 3],
            ];
            if !target.matches(pixel) {
                continue;
            }
            mask.alpha[index] = 255;
            let (px, py) = (
                index as u64 % u64::from(mask.width),
                index as u64 / u64::from(mask.width),
            );
            for (nx, ny) in [
                (px.wrapping_sub(1), py),
                (px + 1, py),
                (px, py.wrapping_sub(1)),
                (px, py + 1),
            ] {
                if let Some(neighbor) = mask_index(mask, nx as u32, ny as u32) {
                    if !visited[neighbor] {
                        visited[neighbor] = true;
                        stack.push(neighbor);
                    }
                }
            }
        }
        return;
    }
    for index in 0..mask.alpha.len() {
        let pixel = [
            image[index * 4],
            image[index * 4 + 1],
            image[index * 4 + 2],
            image[index * 4 + 3],
        ];
        if target.matches(pixel) {
            mask.alpha[index] = 255;
        }
    }
}

fn mask_index(mask: &SelectionMask, x: u32, y: u32) -> Option<usize> {
    if x >= mask.width || y >= mask.height {
        return None;
    }
    usize::try_from(u64::from(y) * u64::from(mask.width) + u64::from(x)).ok()
}

/// Feathers the active mask: three separable box-blur passes over the
/// coverage bytes (`3×box ≈ gaussian` for selection softening), each a
/// `u32` sliding sum rounded half-up — integer-exact, so the falloff is
/// pinnable. `radius` is in pixels, clamped to `1..=64` here; callers
/// refuse zero (that is "remove feather").
pub(crate) fn feather(mask: &mut SelectionMask, radius: u32) {
    let radius = radius.clamp(1, 64) as usize;
    let (width, height) = (mask.width as usize, mask.height as usize);
    for _ in 0..3 {
        let horizontal = blur_axis(&mask.alpha, width, height, radius);
        let vertical = blur_axis(
            &transpose(&horizontal, width, height),
            height,
            width,
            radius,
        );
        mask.alpha = transpose(&vertical, height, width);
    }
}

/// One box-blur pass along the rows of a `width × height` plane, with
/// edge-replicated windows: the window for column `x` is
/// `row[x−r ..= x+r]` clamped to the row, seeded once and slid.
fn blur_axis(plane: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
    let mut out = vec![0u8; plane.len()];
    let span = radius * 2 + 1;
    for y in 0..height {
        let row = &plane[y * width..(y + 1) * width];
        let out_row = &mut out[y * width..(y + 1) * width];
        // Window of column 0: r+1 replicated copies of row[0] (indices
        // −r..0) plus row[1..=r].
        let mut sum = u32::from(row[0]) * (radius as u32 + 1);
        for offset in 1..=radius {
            sum += u32::from(row[offset.min(width - 1)]);
        }
        for x in 0..width {
            out_row[x] = ((sum + (span as u32) / 2) / span as u32) as u8;
            let add = row[(x + radius + 1).min(width - 1)];
            let drop = row[x.saturating_sub(radius)];
            sum = sum + u32::from(add) - u32::from(drop);
        }
    }
    out
}

fn transpose(plane: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0u8; plane.len()];
    for y in 0..height {
        for x in 0..width {
            out[x * height + y] = plane[y * width + x];
        }
    }
    out
}

/// Applies a builder's fresh stamp through `mode`, storing the result
/// back as the document selection.
pub(crate) fn fold(
    active: Option<&SelectionMask>,
    mut fresh: SelectionMask,
    mode: SelectMode,
) -> SelectionMask {
    match (mode, active) {
        (SelectMode::Replace, _) | (_, None) => fresh,
        (_, Some(active)) => {
            // Planes for different images never mix: sizes must agree,
            // which commands enforce before reaching here.
            if active.width != fresh.width || active.height != fresh.height {
                return fresh;
            }
            for (slot, &active_byte) in fresh.alpha.iter_mut().zip(&active.alpha) {
                *slot = mode.combine(active_byte, *slot);
            }
            fresh
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn mask(width: u32, height: u32) -> SelectionMask {
        SelectionMask::empty(width, height)
    }

    fn coverage(mask: &SelectionMask) -> Vec<u8> {
        mask.alpha.clone()
    }

    #[test]
    fn rect_marquee_uses_pixel_centers() {
        // A rect from 0.5 to 3.5 selects pixels whose centers (0.5,
        // 1.5, 2.5) fall inside — columns 0..3 of a 4-wide plane.
        let mut m = mask(4, 2);
        stamp_rect(&mut m, 0.5, 0.0, 3.0, 2.0);
        assert_eq!(coverage(&m), vec![255, 255, 255, 0, 255, 255, 255, 0]);
    }

    #[test]
    fn ellipse_of_exact_diameter_selects_its_middle() {
        // 4×4 ellipse inscribed in the plane: only the four center
        // pixels sit at distance ≤ 1 from the middle.
        let mut m = mask(4, 4);
        stamp_ellipse(&mut m, 0.0, 0.0, 4.0, 4.0);
        assert_eq!(
            coverage(&m),
            vec![
                0, 255, 255, 0, // corners: 1.125 > 1
                255, 255, 255, 255, // (0,1): 0.625 ≤ 1
                255, 255, 255, 255, // mirror of the row above
                0, 255, 255, 0,
            ]
        );
    }

    #[test]
    fn polygon_lasso_fills_even_odd() {
        // Triangle (0,0)-(4,0)-(0,4) on a 4×4 plane: the hypotenuse's
        // crossing at each scanline pins exactly which centers fit.
        let mut m = mask(4, 4);
        stamp_polygon(&mut m, &[(0.0, 0.0), (4.0, 0.0), (0.0, 4.0)]);
        assert_eq!(
            coverage(&m),
            vec![
                255, 255, 255, 0, // crossing at x=3.5
                255, 255, 0, 0, // crossing at x=2.5
                255, 0, 0, 0, // crossing at x=1.5 — center 1.5 is out
                0, 0, 0, 0, // crossing at x=0.5 selects nothing
            ]
        );
        // Degenerate polygons select nothing.
        let mut m = mask(4, 4);
        stamp_polygon(&mut m, &[(0.0, 0.0), (4.0, 0.0)]);
        assert!(coverage(&m).iter().all(|&a| a == 0));
    }

    #[test]
    fn wand_flood_respects_tolerance_and_walls() {
        // Image: red square split from red pocket by a green wall.
        let (width, height) = (4u32, 1u32);
        let image = [
            255u8, 0, 0, 255, // red
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green wall
            250, 0, 0, 255, // near-red (within 5 of seed)
        ];
        let mut m = mask(width, height);
        stamp_wand(
            &mut m,
            &image,
            0,
            0,
            WandColor {
                seed: [255, 0, 0, 255],
                tolerance: 5,
            },
            true,
        );
        // Flood stops at the wall; the near-red pixel is unreachable.
        assert_eq!(coverage(&m), vec![255, 255, 0, 0]);

        let mut m = mask(width, height);
        stamp_wand(
            &mut m,
            &image,
            0,
            0,
            WandColor {
                seed: [255, 0, 0, 255],
                tolerance: 5,
            },
            false,
        );
        // Non-contiguous mode jumps the wall.
        assert_eq!(coverage(&m), vec![255, 255, 0, 255]);
    }

    #[test]
    fn wand_tolerance_is_per_channel_strict() {
        // Tolerance 5 on a blue seed: a pixel 6 away on ONE channel is
        // out even when every other channel matches exactly.
        let image = [0u8, 0, 255, 255, 0, 0, 249, 255, 6, 0, 255, 255];
        let target = WandColor {
            seed: [0, 0, 255, 255],
            tolerance: 5,
        };
        let slice: &[u8] = &image;
        let first: [u8; 4] = slice[0..4].try_into().unwrap();
        let second: [u8; 4] = slice[4..8].try_into().unwrap();
        let third: [u8; 4] = slice[8..12].try_into().unwrap();
        assert!(target.matches(first));
        assert!(!target.matches(second));
        assert!(!target.matches(third));
    }

    #[test]
    fn feather_falloff_table() {
        // One hard-selected column (width 5), feather radius 1, three
        // box passes (H,V)×3 — hand-computed: [0,85,85,85,0] →
        // [28,57,85,57,28] → [38,57,66,57,38], half-up rounding.
        let mut m = mask(5, 1);
        stamp_rect(&mut m, 2.0, 0.0, 1.0, 1.0);
        feather(&mut m, 1);
        assert_eq!(coverage(&m), vec![38, 57, 66, 57, 38]);
    }

    #[test]
    fn feather_preserves_full_selection_total_and_caps() {
        // A fully-selected plane stays fully selected (box blur of a
        // constant is the constant), whatever the radius.
        let mut m = mask(7, 4);
        for slot in m.alpha.iter_mut() {
            *slot = 255;
        }
        feather(&mut m, 64);
        assert!(coverage(&m).iter().all(|&a| a == 255));
    }

    #[test]
    fn modes_fold_replace_add_subtract() {
        let mut active = mask(4, 1);
        active.alpha.copy_from_slice(&[255, 255, 0, 0]);
        let mut fresh = mask(4, 1);
        fresh.alpha.copy_from_slice(&[0, 255, 255, 0]);

        assert_eq!(
            fold(Some(&active), fresh.clone(), SelectMode::Add).alpha,
            vec![255, 255, 255, 0]
        );
        assert_eq!(
            fold(Some(&active), fresh.clone(), SelectMode::Subtract).alpha,
            vec![255, 0, 0, 0]
        );
        assert_eq!(
            fold(Some(&active), fresh.clone(), SelectMode::Replace).alpha,
            fresh.alpha
        );
        // No active selection: any mode replaces.
        assert_eq!(
            fold(None, fresh.clone(), SelectMode::Subtract).alpha,
            fresh.alpha
        );
    }

    #[test]
    fn select_mode_parses_payload_strings() {
        assert_eq!(SelectMode::parse(None), Some(SelectMode::Replace));
        assert_eq!(SelectMode::parse(Some("add")), Some(SelectMode::Add));
        assert_eq!(
            SelectMode::parse(Some("subtract")),
            Some(SelectMode::Subtract)
        );
        assert_eq!(
            SelectMode::parse(Some("replace")),
            Some(SelectMode::Replace)
        );
        assert_eq!(SelectMode::parse(Some("xor")), None);
    }
}
